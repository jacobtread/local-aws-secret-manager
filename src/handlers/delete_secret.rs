use crate::{
    database::{
        DbPool,
        secrets::{delete_secret, get_secret_latest_version, schedule_delete_secret},
    },
    handlers::{
        Handler,
        error::{AwsErrorResponse, InternalServiceError, ResourceNotFoundException},
        models::SecretId,
    },
    utils::date::datetime_to_f64,
};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use garde::Validate;
use serde::{Deserialize, Serialize};

/// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_DeleteSecret.html
pub struct DeleteSecretHandler;

#[derive(Deserialize, Validate)]
pub struct DeleteSecretRequest {
    #[serde(rename = "ForceDeleteWithoutRecovery")]
    #[serde(default)]
    #[garde(skip)]
    force_delete_without_recovery: bool,

    #[serde(rename = "RecoveryWindowInDays")]
    #[serde(default = "default_recovery_window_days")]
    #[garde(range(min = 7, max = 30))]
    recovery_window_in_days: i32,

    #[serde(rename = "SecretId")]
    #[garde(dive)]
    secret_id: SecretId,
}

#[derive(Serialize)]
pub struct DeleteSecretResponse {
    #[serde(rename = "ARN")]
    arn: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "DeletionDate")]
    deletion_date: f64,
}

fn default_recovery_window_days() -> i32 {
    30
}

impl Handler for DeleteSecretHandler {
    type Request = DeleteSecretRequest;
    type Response = DeleteSecretResponse;

    #[tracing::instrument(skip_all, fields(secret_id = %request.secret_id))]
    async fn handle(db: &DbPool, request: Self::Request) -> Result<Self::Response, Response> {
        let DeleteSecretRequest {
            force_delete_without_recovery,
            recovery_window_in_days,
            secret_id,
        } = request;

        let SecretId(secret_id) = secret_id;

        let secret = match get_secret_latest_version(db, &secret_id).await {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(?error, "failed to get secret");
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
                deletion_date: datetime_to_f64(scheduled_deletion_date),
            });
        }

        let deletion_date = if force_delete_without_recovery {
            if let Err(error) = delete_secret(db, &secret.arn).await {
                tracing::error!(?error, "failed to delete secret");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }

            // Secret has been deleted
            Utc::now()
        } else {
            match schedule_delete_secret(db, &secret.arn, recovery_window_in_days).await {
                Ok(value) => value,
                Err(error) => {
                    tracing::error!(?error, "failed to mark secret for deletion");
                    return Err(AwsErrorResponse(InternalServiceError).into_response());
                }
            }
        };

        Ok(DeleteSecretResponse {
            arn: secret.arn,
            name: secret.name,
            deletion_date: datetime_to_f64(deletion_date),
        })
    }
}
