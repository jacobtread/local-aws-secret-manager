use std::{fmt::Display, str::FromStr};

use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::join;

use crate::{
    database::{
        DbPool,
        secrets::{
            count_secret_versions, count_secret_versions_allow_deprecated,
            get_secret_latest_version, get_secret_versions_page,
            get_secret_versions_page_allow_deprecated,
        },
    },
    handlers::{
        Handler,
        error::{
            AwsErrorResponse, InternalServiceError, InvalidRequestException,
            ResourceNotFoundException,
        },
    },
    utils::date::datetime_to_f64,
};

// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_ListSecretVersionIds.html
pub struct ListSecretVersionIdsHandler;

#[derive(Deserialize)]
pub struct ListSecretVersionIdsRequest {
    #[serde(rename = "IncludeDeprecated")]
    include_deprecated: Option<bool>,
    #[serde(rename = "MaxResults")]
    max_results: Option<i32>,
    #[serde(rename = "NextToken")]
    next_token: Option<String>,
    #[serde(rename = "SecretId")]
    secret_id: String,
}

#[derive(Serialize)]
pub struct ListSecretVersionIdsResponse {
    #[serde(rename = "ARN")]
    arn: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "NextToken")]
    next_token: Option<String>,
    #[serde(rename = "Versions")]
    versions: Vec<SecretVersionsListEntry>,
}

#[derive(Serialize)]
pub struct SecretVersionsListEntry {
    #[serde(rename = "CreatedDate")]
    created_date: f64,
    #[serde(rename = "KmsKeyIds")]
    kms_key_ids: Option<Vec<String>>,
    #[serde(rename = "LastAccessedDate")]
    last_accessed_date: Option<f64>,
    #[serde(rename = "VersionId")]
    version_id: String,
    #[serde(rename = "VersionStages")]
    version_stages: Vec<String>,
}

pub struct PaginationToken {
    /// Size of each page
    page_size: i64,
    /// Page index
    page_index: i64,
}

impl Display for PaginationToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.page_size, self.page_index)
    }
}

#[derive(Debug, Error)]
#[error("invalid pagination token")]
pub struct InvalidPaginationToken;

impl FromStr for PaginationToken {
    type Err = InvalidPaginationToken;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (page_size, page) = s.split_once(":").ok_or(InvalidPaginationToken)?;
        let page_size = page_size.parse().map_err(|_| InvalidPaginationToken)?;
        let page = page.parse().map_err(|_| InvalidPaginationToken)?;

        Ok(PaginationToken {
            page_size,
            page_index: page,
        })
    }
}

impl Handler for ListSecretVersionIdsHandler {
    type Request = ListSecretVersionIdsRequest;
    type Response = ListSecretVersionIdsResponse;

    async fn handle(db: &DbPool, request: Self::Request) -> Result<Self::Response, Response> {
        let secret_id = request.secret_id;

        // Whether to include secrets without stages
        let include_deprecated = request.include_deprecated.unwrap_or_default();

        // Maximum results to get (Default to 100)
        let max_results = request.max_results.unwrap_or(100);

        let mut pagination_token = request
            .next_token
            .map(|value| PaginationToken::from_str(&value))
            .transpose()
            // Invalid pagination token
            .map_err(|_| AwsErrorResponse(InvalidRequestException).into_response())?
            // Default pagination for the first page
            .unwrap_or(PaginationToken {
                page_size: max_results as i64,
                page_index: 0,
            });

        // Update the pagination page size to match the max results
        pagination_token.page_size = max_results as i64;

        let secret = match get_secret_latest_version(db, &secret_id).await {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(?error, %secret_id, "failed to get secret");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }
        };

        let secret = match secret {
            Some(value) => value,
            None => return Err(AwsErrorResponse(ResourceNotFoundException).into_response()),
        };

        let limit = pagination_token.page_size;
        let offset = match pagination_token
            .page_size
            .checked_mul(pagination_token.page_index)
        {
            Some(value) => value,
            None => {
                // Requested page exceeds the i64 bounds
                return Err(AwsErrorResponse(InvalidRequestException).into_response());
            }
        };

        let (versions, count) = if include_deprecated {
            join!(
                get_secret_versions_page_allow_deprecated(db, &secret.arn, limit, offset),
                count_secret_versions_allow_deprecated(db, &secret.arn),
            )
        } else {
            join!(
                get_secret_versions_page(db, &secret.arn, limit, offset),
                count_secret_versions(db, &secret.arn),
            )
        };

        let versions = match versions {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(?error, %secret_id, "failed to get versions");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }
        };

        let count = match count {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(?error, %secret_id, "failed to get versions count");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }
        };

        let has_next_page = offset.checked_add(limit).is_some_and(|size| count > size);

        let next_token = match (pagination_token.page_index.checked_add(1), has_next_page) {
            // Only provide a next token if the page is computable and we have enough entries to
            // fullfil the request
            (Some(next_page), true) => {
                //
                Some(PaginationToken {
                    page_size: pagination_token.page_size,
                    page_index: next_page,
                })
            }

            // No next page
            _ => None,
        };

        let next_token = next_token.map(|value| value.to_string());

        let versions = versions
            .into_iter()
            .map(|version| SecretVersionsListEntry {
                created_date: datetime_to_f64(version.created_at),
                kms_key_ids: None,
                last_accessed_date: version.last_accessed_at.map(datetime_to_f64),
                version_id: version.version_id,
                version_stages: version.version_stages,
            })
            .collect();

        Ok(ListSecretVersionIdsResponse {
            arn: secret.arn,
            name: secret.name,
            next_token,
            versions,
        })
    }
}
