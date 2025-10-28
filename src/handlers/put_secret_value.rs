use std::ops::DerefMut;

use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    database::{
        DbPool,
        secrets::{
            CreateSecretVersion, add_secret_version_stage, create_secret_version,
            get_secret_by_version_id, get_secret_latest_version, remove_secret_version_stage_any,
        },
    },
    handlers::{
        Handler,
        error::{
            AwsErrorResponse, InternalServiceError, InvalidRequestException,
            ResourceExistsException, ResourceNotFoundException,
        },
    },
};

// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_PutSecretValue.html
pub struct PutSecretValueHandler;

#[derive(Deserialize)]
pub struct PutSecretValueRequest {
    #[serde(rename = "ClientRequestToken")]
    client_request_token: Option<String>,
    #[serde(rename = "SecretId")]
    secret_id: String,
    #[serde(rename = "SecretString")]
    secret_string: Option<String>,
    #[serde(rename = "SecretBinary")]
    secret_binary: Option<String>,
    #[serde(rename = "VersionStages")]
    version_stages: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct PutSecretValueResponse {
    #[serde(rename = "ARN")]
    arn: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "VersionId")]
    version_id: String,
    #[serde(rename = "VersionStages")]
    version_stages: Vec<String>,
}

impl Handler for PutSecretValueHandler {
    type Request = PutSecretValueRequest;
    type Response = PutSecretValueResponse;

    async fn handle(db: &DbPool, request: Self::Request) -> Result<Self::Response, Response> {
        let version_id = request
            .client_request_token
            // Generate a new version ID if none was provided
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let version_stages = match request.version_stages {
            Some(value) => {
                // When specifying version stages must specify at least one
                if value.is_empty() {
                    return Err(AwsErrorResponse(InvalidRequestException).into_response());
                }

                value
            }
            None => vec!["AWSCURRENT".to_string()],
        };

        // Must only specify one of the two
        if request.secret_string.is_some() && request.secret_binary.is_some() {
            return Err(AwsErrorResponse(InvalidRequestException).into_response());
        }

        // Must specify at least one
        if request.secret_string.is_none() && request.secret_binary.is_none() {
            return Err(AwsErrorResponse(InvalidRequestException).into_response());
        }

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

        let mut t = match db.begin().await {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(?error, name = %secret.name, "failed to begin transaction");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }
        };

        // Create the new secret version
        if let Err(error) = create_secret_version(
            t.deref_mut(),
            CreateSecretVersion {
                secret_arn: secret.arn.clone(),
                version_id: version_id.clone(),
                secret_string: request.secret_string.clone(),
                secret_binary: request.secret_binary.clone(),
            },
        )
        .await
        {
            if let Some(error) = error.as_database_error()
                && error.is_unique_violation()
            {
                // Must rollback the transaction before attempting to use the connection
                if let Err(error) = t.rollback().await {
                    tracing::error!(?error, "failed to rollback transaction");
                }

                // Check if the secret has been created
                let secret = match get_secret_by_version_id(db, &secret.arn, &version_id).await {
                    Ok(value) => value,
                    Err(error) => {
                        tracing::error!(?error, name = %secret.name,"failed to determine existing version");
                        return Err(AwsErrorResponse(InternalServiceError).into_response());
                    }
                };

                let secret = match secret {
                    Some(value) => value,
                    None => {
                        // Shouldn't be possible if we hit the unique violation
                        return Err(AwsErrorResponse(InternalServiceError).into_response());
                    }
                };

                // If the stored version data doesn't match this is an error that
                // the resource already exists
                if secret.secret_string.ne(&request.secret_string)
                    || secret.secret_binary.ne(&request.secret_binary)
                {
                    return Err(AwsErrorResponse(ResourceExistsException).into_response());
                }

                // Another request already created this version
                return Ok(PutSecretValueResponse {
                    arn: secret.arn,
                    name: secret.name,
                    version_id: secret.version_id,
                    version_stages: secret.version_stages,
                });
            }

            tracing::error!(?error, name = %secret.name, "failed to create secret version");
            return Err(AwsErrorResponse(InternalServiceError).into_response());
        }

        // Add the requested stages
        for version_stage in &version_stages {
            if let Err(error) =
                remove_secret_version_stage_any(t.deref_mut(), &secret.arn, version_stage).await
            {
                tracing::error!(?error, name = %secret.name, "failed to remove version stage from secret");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }

            // If we are re-assigning AWSCURRENT ensure that the previous secret is given AWSPREVIOUS
            if version_stage == "AWSCURRENT" {
                // Ensure nobody else has the AWSPREVIOUS stage
                if let Err(error) =
                    remove_secret_version_stage_any(t.deref_mut(), &secret.arn, "AWSPREVIOUS").await
                {
                    tracing::error!(?error, name = %secret.name, "failed to remove version stage from secret");
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
            }

            // Add the requested version stage
            if let Err(error) =
                add_secret_version_stage(t.deref_mut(), &secret.arn, &version_id, version_stage)
                    .await
            {
                tracing::error!(?error, name = %secret.name, "failed to add stage to secret");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }
        }

        if let Err(error) = t.commit().await {
            tracing::error!(?error, name = %secret.name,  "failed to commit transaction");
            return Err(AwsErrorResponse(InternalServiceError).into_response());
        }

        Ok(PutSecretValueResponse {
            arn: secret.arn,
            name: secret.name,
            version_id,
            version_stages,
        })
    }
}
