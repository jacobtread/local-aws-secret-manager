use crate::{
    database::{
        DbPool,
        secrets::{
            VersionStage, get_secret_by_version_id, get_secret_by_version_stage,
            get_secret_by_version_stage_and_id, get_secret_latest_version,
            update_secret_version_last_accessed,
        },
    },
    handlers::{
        Handler,
        error::{
            AwsErrorResponse, InternalServiceError, InvalidRequestException,
            ResourceNotFoundException,
        },
    },
};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_GetSecretValue.html
pub struct GetSecretValueHandler;

#[derive(Deserialize)]
pub struct GetSecretValueRequest {
    #[serde(rename = "SecretId")]
    secret_id: String,
    #[serde(rename = "VersionId")]
    version_id: Option<String>,
    #[serde(rename = "VersionStage")]
    version_stage: Option<String>,
}

#[derive(Serialize)]
pub struct GetSecretValueResponse {
    #[serde(rename = "ARN")]
    arn: String,
    #[serde(rename = "CreatedDate")]
    created_date: i64,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "SecretString")]
    secret_string: Option<String>,
    #[serde(rename = "SecretBinary")]
    secret_binary: Option<String>,
    #[serde(rename = "VersionId")]
    version_id: String,
    #[serde(rename = "VersionStages")]
    version_stages: Vec<VersionStage>,
}

impl Handler for GetSecretValueHandler {
    type Request = GetSecretValueRequest;
    type Response = GetSecretValueResponse;

    async fn handle(db: &DbPool, request: Self::Request) -> Result<Self::Response, Response> {
        let secret_id = request.secret_id;
        let version_id = request.version_id;
        let version_stage = match request
            .version_stage
            .map(VersionStage::try_from)
            .transpose()
        {
            Ok(value) => value,
            Err(_error) => {
                return Err(AwsErrorResponse(InvalidRequestException).into_response());
            }
        };

        let secret = match (&version_id, version_stage) {
            (None, None) => get_secret_latest_version(db, &secret_id).await,
            (Some(version_id), Some(version_stage)) => {
                get_secret_by_version_stage_and_id(db, &secret_id, version_id, version_stage).await
            }
            (Some(version_id), None) => get_secret_by_version_id(db, &secret_id, version_id).await,
            (None, Some(version_stage)) => {
                get_secret_by_version_stage(db, &secret_id, version_stage).await
            }
        };

        let secret = match secret {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(?error, %secret_id, ?version_id, ?version_stage, "failed to get secret value");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }
        };

        let secret = match secret {
            Some(value) => value,
            None => return Err(AwsErrorResponse(ResourceNotFoundException).into_response()),
        };

        // Secret is scheduled for deletion
        if secret.scheduled_delete_at.is_some() {
            return Err(AwsErrorResponse(InvalidRequestException).into_response());
        }

        if let Err(error) =
            update_secret_version_last_accessed(db, &secret.arn, &secret.version_id).await
        {
            tracing::error!(?error, name = %secret.name, "failed to update secret last accessed");
            return Err(AwsErrorResponse(InternalServiceError).into_response());
        }

        let created_at = if version_id.is_some() {
            secret.version_created_at
        } else {
            secret.created_at
        };

        Ok(GetSecretValueResponse {
            arn: secret.arn,
            created_date: created_at.timestamp(),
            name: secret.name,
            secret_string: secret.secret_string,
            secret_binary: secret.secret_binary,
            version_id: secret.version_id,
            version_stages: vec![secret.version_stage],
        })
    }
}
