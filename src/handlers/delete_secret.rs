use crate::{
    database::{
        DbPool,
        secrets::{delete_secret, get_secret_latest_version, schedule_delete_secret},
    },
    handlers::{
        Handler,
        error::{AwsErrorResponse, InternalServiceError, ResourceNotFoundException},
    },
};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_DeleteSecret.html
pub struct DeleteSecretHandler;

#[derive(Deserialize)]
pub struct DeleteSecretRequest {
    #[serde(rename = "ForceDeleteWithoutRecovery")]
    force_delete_without_recovery: Option<bool>,
    #[serde(rename = "RecoveryWindowInDays")]
    recovery_window_in_days: Option<i32>,
    #[serde(rename = "SecretId")]
    secret_id: String,
}

#[derive(Serialize)]
pub struct DeleteSecretResponse {
    #[serde(rename = "ARN")]
    arn: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "DeletionDate")]
    deletion_date: i64,
}

impl Handler for DeleteSecretHandler {
    type Request = DeleteSecretRequest;
    type Response = DeleteSecretResponse;

    async fn handle(db: &DbPool, request: Self::Request) -> Result<Self::Response, Response> {
        let force_delete_without_recovery =
            request.force_delete_without_recovery.unwrap_or_default();
        let recovery_window_in_days = request.recovery_window_in_days.unwrap_or(30);
        let secret_id = request.secret_id;

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

        // Secret is already scheduled for deletion
        if let Some(scheduled_deletion_date) = secret.scheduled_delete_at {
            return Ok(DeleteSecretResponse {
                arn: secret.arn,
                name: secret.name,
                deletion_date: scheduled_deletion_date.timestamp(),
            });
        }

        let deletion_date = if force_delete_without_recovery {
            if let Err(error) = delete_secret(db, &secret.arn).await {
                tracing::error!(?error, %secret_id, "failed to delete secret");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }

            // Secret has been deleted
            Utc::now()
        } else {
            match schedule_delete_secret(db, &secret.arn, recovery_window_in_days).await {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("{error:?}");
                    tracing::error!(?error, %secret_id, "failed to mark secret for deletion");
                    return Err(AwsErrorResponse(InternalServiceError).into_response());
                }
            }
        };

        Ok(DeleteSecretResponse {
            arn: secret.arn,
            name: secret.name,
            deletion_date: deletion_date.timestamp(),
        })
    }
}
