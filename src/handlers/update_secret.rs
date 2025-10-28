use crate::{
    database::{
        DbPool,
        secrets::{
            CreateSecretVersion, add_secret_version_stage, create_secret_version,
            get_secret_latest_version, remove_secret_version_stage,
            remove_secret_version_stage_any, update_secret_description,
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
use std::ops::DerefMut;
use uuid::Uuid;

// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_UpdateSecret.html
pub struct UpdateSecretHandler;

#[derive(Deserialize)]
pub struct UpdateSecretRequest {
    #[serde(rename = "ClientRequestToken")]
    client_request_token: Option<String>,
    #[serde(rename = "Description")]
    description: Option<String>,
    #[serde(rename = "SecretId")]
    secret_id: String,
    #[serde(rename = "SecretString")]
    secret_string: Option<String>,
    #[serde(rename = "SecretBinary")]
    secret_binary: Option<String>,
}

#[derive(Serialize)]
pub struct UpdateSecretResponse {
    #[serde(rename = "ARN")]
    arn: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "VersionId")]
    version_id: Option<String>,
}

impl Handler for UpdateSecretHandler {
    type Request = UpdateSecretRequest;
    type Response = UpdateSecretResponse;

    async fn handle(db: &DbPool, request: Self::Request) -> Result<Self::Response, Response> {
        let secret_id = request.secret_id;

        // Must only specify one of the two
        if request.secret_string.is_some() && request.secret_binary.is_some() {
            return Err(AwsErrorResponse(InvalidRequestException).into_response());
        }

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

        let mut t = match db.begin().await {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(?error, name = %secret.name, "failed to begin transaction");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }
        };

        if let Some(description) = request.description
            && let Err(error) =
                update_secret_description(t.deref_mut(), &secret.arn, &description).await
        {
            // Rollback the transaction on failure
            if let Err(error) = t.rollback().await {
                tracing::error!(?error, "failed to rollback transaction");
            }

            tracing::error!(?error, name = %secret.name, "failed to update secret version description");
            return Err(AwsErrorResponse(InternalServiceError).into_response());
        }

        let version_id = if request.secret_string.is_some() || request.secret_binary.is_some() {
            let version_id = request
                .client_request_token
                // Generate a new version ID if none was provided
                .unwrap_or_else(|| Uuid::new_v4().to_string());

            // Create a new current secret version
            if let Err(error) = create_secret_version(
                t.deref_mut(),
                CreateSecretVersion {
                    secret_arn: secret.arn.clone(),
                    version_id: version_id.clone(),
                    secret_string: request.secret_string,
                    secret_binary: request.secret_binary,
                },
            )
            .await
            {
                if let Some(error) = error.as_database_error()
                    && error.is_unique_violation()
                {
                    // Another request already created this version
                    return Ok(UpdateSecretResponse {
                        arn: secret.arn,
                        name: secret.name,
                        version_id: None,
                    });
                }

                // Rollback the transaction on failure
                if let Err(error) = t.rollback().await {
                    tracing::error!(?error, "failed to rollback transaction");
                }

                tracing::error!(?error, name = %secret.name, "failed to create secret version");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }

            // Remove AWSPREVIOUS from any other versions
            if let Err(error) =
                remove_secret_version_stage_any(t.deref_mut(), &secret.arn, "AWSPREVIOUS").await
            {
                tracing::error!(?error, name = %secret.name, "failed to deprecate old previous secret");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }

            // Add the AWSPREVIOUS stage to the old current
            if let Err(error) = add_secret_version_stage(
                t.deref_mut(),
                &secret.arn,
                &secret.version_id,
                "AWSPREVIOUS",
            )
            .await
            {
                tracing::error!(?error, name = %secret.name, "failed to add AWSPREVIOUS tag to secret");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }

            // Remove AWSCURRENT from the current version
            if let Err(error) = remove_secret_version_stage(
                t.deref_mut(),
                &secret.arn,
                &secret.version_id,
                "AWSCURRENT",
            )
            .await
            {
                tracing::error!(?error, name = %secret.name, "failed to remove AWSCURRENT from old version");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }

            // Add the AWSCURRENT stage to the new version
            if let Err(error) =
                add_secret_version_stage(t.deref_mut(), &secret.arn, &version_id, "AWSCURRENT")
                    .await
            {
                tracing::error!(?error, name = %secret.name, "failed to add AWSCURRENT tag to secret");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }

            Some(version_id)
        } else {
            // Nothing to update
            None
        };

        if let Err(error) = t.commit().await {
            tracing::error!(?error, name = %secret.name,  "failed to commit transaction");
            return Err(AwsErrorResponse(InternalServiceError).into_response());
        }

        Ok(UpdateSecretResponse {
            arn: secret.arn,
            name: secret.name,
            version_id,
        })
    }
}
