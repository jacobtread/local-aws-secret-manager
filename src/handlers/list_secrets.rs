use std::{collections::HashMap, str::FromStr};

use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use tokio::join;

use crate::{
    database::{
        DbPool,
        secrets::{
            SecretFilter, get_secrets_by_filter, get_secrets_by_filter_with_deprecated,
            get_secrets_count_by_filter, get_secrets_count_by_filter_with_deprecated,
        },
    },
    handlers::{
        Filter, Handler, PaginationToken, Tag,
        error::{AwsErrorResponse, InternalServiceError, InvalidRequestException},
    },
    utils::date::datetime_to_f64,
};

// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_ListSecrets.html
pub struct ListSecretsHandler;

#[derive(Deserialize)]
pub struct ListSecretsRequest {
    #[serde(rename = "Filters")]
    filters: Option<Vec<Filter>>,

    #[serde(rename = "IncludePlannedDeletion")]
    include_planned_deletion: Option<bool>,

    #[serde(rename = "MaxResults")]
    max_results: Option<i32>,

    #[serde(rename = "NextToken")]
    next_token: Option<String>,

    #[serde(rename = "SortOrder")]
    sort_order: Option<String>,
}

#[derive(Serialize)]
pub struct ListSecretsResponse {
    #[serde(rename = "NextToken")]
    next_token: Option<String>,
    #[serde(rename = "SecretList")]
    secret_list: Vec<SecretListEntry>,
}

#[derive(Serialize)]
pub struct SecretListEntry {
    #[serde(rename = "ARN")]
    arn: String,
    #[serde(rename = "CreatedDate")]
    created_date: f64,
    #[serde(rename = "DeletedDate")]
    deleted_date: Option<f64>,
    #[serde(rename = "Description")]
    description: Option<String>,
    #[serde(rename = "KmsKeyId")]
    kms_key_id: Option<String>,
    #[serde(rename = "LastAccessedDate")]
    last_accessed_date: Option<f64>,
    #[serde(rename = "LastChangedDate")]
    last_changed_date: Option<f64>,
    #[serde(rename = "LastRotatedDate")]
    last_rotated_date: Option<f64>,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "NextRotationDate")]
    next_rotation_date: Option<f64>,
    #[serde(rename = "OwningService")]
    owning_service: Option<String>,
    #[serde(rename = "PrimaryRegion")]
    primary_region: Option<String>,
    #[serde(rename = "RotationEnabled")]
    rotation_enabled: bool,
    #[serde(rename = "RotationLambdaARN")]
    rotation_lambda_arn: Option<String>,
    #[serde(rename = "RotationRules")]
    rotation_rules: Option<serde_json::Value>,
    #[serde(rename = "SecretVersionsToStages")]
    secret_versions_to_stages: HashMap<String, Vec<String>>,
    #[serde(rename = "Tags")]
    tags: Vec<Tag>,
}

impl Handler for ListSecretsHandler {
    type Request = ListSecretsRequest;
    type Response = ListSecretsResponse;

    async fn handle(db: &DbPool, request: Self::Request) -> Result<Self::Response, Response> {
        let mut filters = SecretFilter::default();
        let include_planned_deletion = request.include_planned_deletion.unwrap_or_default();
        let max_results = request.max_results.unwrap_or(100);
        let asc = request.sort_order.is_some_and(|value| value == "asc");

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

        if let Some(request_filters) = request.filters {
            for filter in request_filters {
                match filter.key.as_str() {
                    "description" => {
                        filters.description.extend(filter.values);
                    }
                    "name" => {
                        filters.name.extend(filter.values);
                    }
                    "tag-key" => {
                        filters.tag_key.extend(filter.values);
                    }
                    "tag-value" => {
                        filters.tag_value.extend(filter.values);
                    }
                    _ => {
                        filters.all.extend(filter.values);
                    }
                }
            }
        }

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

        let (secrets, count) = if include_planned_deletion {
            join!(
                get_secrets_by_filter_with_deprecated(db, &filters, limit, offset, asc),
                get_secrets_count_by_filter_with_deprecated(db, &filters),
            )
        } else {
            join!(
                get_secrets_by_filter(db, &filters, limit, offset, asc),
                get_secrets_count_by_filter(db, &filters),
            )
        };

        let secrets = match secrets {
            Ok(value) => value,
            Err(error) => {
                eprintln!("{error:?}");
                tracing::error!(?error, "failed to get secrets");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }
        };

        let count = match count {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(?error, "failed to get secrets count");
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

        let secret_list = secrets
            .into_iter()
            .map(|secret| {
                let versions = secret.versions;

                let most_recently_used = versions
                    .iter()
                    .filter_map(|version| version.last_accessed_at)
                    .max();

                let tags_updated_at = secret.version_tags.iter().filter_map(|tag| tag.updated_at);

                let last_changed_date = versions
                    .iter()
                    .map(|version| version.created_at)
                    .chain(secret.updated_at)
                    .chain(tags_updated_at)
                    .max();

                let secret_versions_to_stages = versions
                    .into_iter()
                    .map(|version| (version.version_id, version.version_stages))
                    .collect();

                SecretListEntry {
                    arn: secret.arn,
                    description: secret.description,
                    created_date: datetime_to_f64(secret.created_at),
                    deleted_date: secret.deleted_at.map(datetime_to_f64),
                    kms_key_id: None,
                    last_accessed_date: most_recently_used.map(datetime_to_f64),
                    last_changed_date: last_changed_date.map(datetime_to_f64),
                    last_rotated_date: None,
                    name: secret.name,
                    next_rotation_date: None,
                    owning_service: None,
                    primary_region: None,
                    rotation_enabled: false,
                    rotation_lambda_arn: None,
                    rotation_rules: None,
                    tags: secret
                        .version_tags
                        .into_iter()
                        .map(|tag| Tag {
                            key: tag.key,
                            value: tag.value,
                        })
                        .collect(),
                    secret_versions_to_stages,
                }
            })
            .collect();

        Ok(ListSecretsResponse {
            next_token,
            secret_list,
        })
    }
}
