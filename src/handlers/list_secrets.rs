use axum::response::{IntoResponse, Response};
use garde::Validate;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::join;

use crate::{
    database::{
        DbPool,
        secrets::{get_secrets_by_filter, get_secrets_count_by_filter},
    },
    handlers::{
        Filter, Handler, PaginationToken, Tag,
        error::{AwsErrorResponse, InternalServiceError, InvalidRequestException},
    },
    utils::date::datetime_to_f64,
};

// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_ListSecrets.html
pub struct ListSecretsHandler;

#[derive(Deserialize, Validate)]
pub struct ListSecretsRequest {
    #[serde(rename = "Filters")]
    #[serde(default)]
    #[garde(dive)]
    filters: Vec<Filter>,

    #[serde(rename = "IncludePlannedDeletion")]
    #[serde(default)]
    #[garde(skip)]
    include_planned_deletion: bool,

    #[serde(rename = "MaxResults")]
    #[serde(default = "default_max_results")]
    #[garde(range(min = 1, max = 100))]
    max_results: i32,

    #[serde(rename = "NextToken")]
    #[serde(default = "default_next_token")]
    #[garde(dive)]
    next_token: PaginationToken,

    #[serde(rename = "SortOrder")]
    #[serde(default = "default_sort_order")]
    #[garde(custom(is_valid_sort_order))]
    sort_order: String,
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

fn default_sort_order() -> String {
    "desc".to_string()
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

const VALID_SORT_ORDER: [&str; 2] = ["asc", "desc"];

/// Checks if the provided value is a valid sort order
fn is_valid_sort_order(value: &str, _context: &()) -> garde::Result {
    if !VALID_SORT_ORDER.contains(&value) {
        let expected = VALID_SORT_ORDER.iter().join(", ");
        return Err(garde::Error::new(format!(
            "unknown sort order expected one of: {expected}"
        )));
    }

    Ok(())
}

impl Handler for ListSecretsHandler {
    type Request = ListSecretsRequest;
    type Response = ListSecretsResponse;

    #[tracing::instrument(skip_all)]
    async fn handle(db: &DbPool, request: Self::Request) -> Result<Self::Response, Response> {
        let include_planned_deletion = request.include_planned_deletion;
        let max_results = request.max_results;
        let filters = request.filters;
        let asc = request.sort_order == "asc";

        let mut pagination_token = request.next_token;

        // Update the pagination page size to match the max results
        pagination_token.page_size = max_results as i64;

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

        let (secrets, count) = join!(
            get_secrets_by_filter(db, &filters, include_planned_deletion, limit, offset, asc),
            get_secrets_count_by_filter(db, &filters, include_planned_deletion),
        );

        let secrets = match secrets {
            Ok(value) => value,
            Err(error) => {
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
