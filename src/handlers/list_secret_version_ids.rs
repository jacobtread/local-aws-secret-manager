use axum::response::{IntoResponse, Response};
use garde::Validate;
use serde::{Deserialize, Serialize};
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
        Handler, PaginationToken, SecretId,
        error::{
            AwsErrorResponse, InternalServiceError, InvalidRequestException,
            ResourceNotFoundException,
        },
    },
    utils::date::datetime_to_f64,
};

// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_ListSecretVersionIds.html
pub struct ListSecretVersionIdsHandler;

#[derive(Deserialize, Validate)]
pub struct ListSecretVersionIdsRequest {
    #[serde(rename = "IncludeDeprecated")]
    #[serde(default)]
    #[garde(skip)]
    include_deprecated: bool,

    #[serde(rename = "MaxResults")]
    #[serde(default = "default_max_results")]
    #[garde(range(min = 1, max = 100))]
    max_results: i32,

    #[serde(rename = "NextToken")]
    #[serde(default = "default_next_token")]
    #[garde(dive)]
    next_token: PaginationToken,

    #[serde(rename = "SecretId")]
    #[garde(dive)]
    secret_id: SecretId,
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

fn default_max_results() -> i32 {
    100
}

fn default_next_token() -> PaginationToken {
    PaginationToken {
        page_size: 100,
        page_index: 0,
    }
}

impl Handler for ListSecretVersionIdsHandler {
    type Request = ListSecretVersionIdsRequest;
    type Response = ListSecretVersionIdsResponse;

    async fn handle(db: &DbPool, request: Self::Request) -> Result<Self::Response, Response> {
        let SecretId(secret_id) = request.secret_id;
        let include_deprecated = request.include_deprecated;
        let max_results = request.max_results;

        let mut pagination_token = request.next_token;

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
