use std::str::FromStr;

use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::{
    database::{
        DbPool,
        secrets::{get_secret_latest_version, update_secret_version_last_accessed},
    },
    handlers::{
        APIErrorType, Filter, Handler, PaginationToken,
        error::{
            AwsError, AwsErrorResponse, InternalServiceError, InvalidRequestException,
            NotImplemented, ResourceNotFoundException,
        },
    },
    utils::date::datetime_to_f64,
};

// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_BatchGetSecretValue.html
pub struct BatchGetSecretValueHandler;

#[derive(Deserialize)]
pub struct BatchGetSecretValueRequest {
    #[serde(rename = "Filters")]
    filters: Option<Vec<Filter>>,

    #[serde(rename = "MaxResults")]
    max_results: Option<i32>,

    #[serde(rename = "NextToken")]
    next_token: Option<String>,

    #[serde(rename = "SecretIdList")]
    secret_id_list: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct BatchGetSecretValueResponse {
    #[serde(rename = "Errors")]
    errors: Vec<APIErrorType>,
    #[serde(rename = "NextToken")]
    next_token: Option<String>,
    #[serde(rename = "SecretValues")]
    secret_values: Vec<SecretValueEntry>,
}

#[derive(Serialize)]
struct SecretValueEntry {
    #[serde(rename = "ARN")]
    arn: String,
    #[serde(rename = "CreatedDate")]
    created_date: f64,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "SecretString")]
    secret_string: Option<String>,
    #[serde(rename = "SecretBinary")]
    secret_binary: Option<String>,
    #[serde(rename = "VersionId")]
    version_id: String,
    #[serde(rename = "VersionStages")]
    version_stages: Vec<String>,
}

impl Handler for BatchGetSecretValueHandler {
    type Request = BatchGetSecretValueRequest;
    type Response = BatchGetSecretValueResponse;

    async fn handle(db: &DbPool, request: Self::Request) -> Result<Self::Response, Response> {
        let mut errors: Vec<APIErrorType> = Vec::new();
        let mut secret_values: Vec<SecretValueEntry> = Vec::new();
        let mut next_token: Option<String> = None;

        match (request.filters, request.secret_id_list) {
            (Some(filters), None) => {
                let max_results = request.max_results.unwrap_or(20);

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
            }
            (None, Some(secret_id_list)) => {
                for secret_id in secret_id_list {
                    let secret = match get_secret_latest_version(db, &secret_id).await {
                        Ok(value) => value,
                        Err(error) => {
                            tracing::error!(?error, %secret_id, "failed to load secret");
                            return Err(AwsErrorResponse(InternalServiceError).into_response());
                        }
                    };

                    let secret = match secret {
                        Some(value) => value,
                        None => {
                            errors.push(APIErrorType {
                                error_code: Some(ResourceNotFoundException::TYPE.to_string()),
                                message: Some(ResourceNotFoundException::MESSAGE.to_string()),
                                secret_id: Some(secret_id),
                            });
                            continue;
                        }
                    };

                    if let Err(error) =
                        update_secret_version_last_accessed(db, &secret.arn, &secret.version_id)
                            .await
                    {
                        tracing::error!(?error, name = %secret.name, "failed to update secret last accessed");
                        return Err(AwsErrorResponse(InternalServiceError).into_response());
                    }

                    secret_values.push(SecretValueEntry {
                        arn: secret.arn,
                        created_date: datetime_to_f64(secret.created_at),
                        name: secret.name,
                        secret_string: secret.secret_string,
                        secret_binary: secret.secret_binary,
                        version_id: secret.version_id,
                        version_stages: secret.version_stages,
                    });
                }
            }

            // Must only specify one or the other and not both
            // and cannot pick neither
            (Some(_), Some(_)) | (None, None) => {
                return Err(AwsErrorResponse(InvalidRequestException).into_response());
            }
        }

        Ok(BatchGetSecretValueResponse {
            errors,
            next_token,
            secret_values,
        })
    }
}
