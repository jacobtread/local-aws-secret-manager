use crate::{
    database::{
        DbPool,
        secrets::{
            CreateSecret, CreateSecretVersion, VersionStage, create_secret, create_secret_version,
            get_secret_by_version_id, put_secret_tag,
        },
    },
    handlers::{
        Handler, Tag,
        error::{
            AwsErrorResponse, InternalServiceError, InvalidRequestException,
            ResourceExistsException,
        },
    },
};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use std::ops::DerefMut;
use uuid::Uuid;

// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_CreateSecret.html
pub struct CreateSecretHandler;

#[derive(Deserialize)]
pub struct CreateSecretRequest {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Description")]
    description: Option<String>,
    #[serde(rename = "ClientRequestToken")]
    client_request_token: Option<String>,
    #[serde(rename = "SecretString")]
    secret_string: Option<String>,
    #[serde(rename = "SecretBinary")]
    secret_binary: Option<String>,
    #[serde(rename = "Tags")]
    tags: Option<Vec<Tag>>,
}

#[derive(Serialize)]
pub struct CreateSecretResponse {
    #[serde(rename = "ARN")]
    arn: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "VersionId")]
    version_id: String,
}

impl Handler for CreateSecretHandler {
    type Request = CreateSecretRequest;
    type Response = CreateSecretResponse;

    async fn handle(db: &DbPool, request: Self::Request) -> Result<Self::Response, Response> {
        let name = request.name;
        let arn = format!("arn:aws:secretsmanager:us-east-1:1:secret:{name}");

        let version_id = request
            .client_request_token
            // Generate a new version ID if none was provided
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let tags = request.tags.unwrap_or_default();

        if request.secret_string.is_none() && request.secret_binary.is_none() {
            return Err(AwsErrorResponse(InvalidRequestException).into_response());
        }

        let mut t = match db.begin().await {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(?error, %name, "failed to begin transaction");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }
        };

        // Create the secret
        if let Err(error) = create_secret(
            t.deref_mut(),
            CreateSecret {
                arn: arn.clone(),
                name: name.clone(),
                description: request.description,
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
                let secret = match get_secret_by_version_id(db, &arn, &version_id).await {
                    Ok(value) => value,
                    Err(error) => {
                        tracing::error!(?error, %name, "failed to determine existing version");
                        return Err(AwsErrorResponse(InternalServiceError).into_response());
                    }
                };

                // Another request already created this version
                if secret.is_some() {
                    return Ok(CreateSecretResponse {
                        arn,
                        name,
                        version_id,
                    });
                }

                // Tried to create a secret that already exists (And wasn't a matching version on the existing one)
                return Err(AwsErrorResponse(ResourceExistsException).into_response());
            }

            tracing::error!(?error, %name, "failed to create secret");
            return Err(AwsErrorResponse(InternalServiceError).into_response());
        }

        // Create the initial secret version
        if let Err(error) = create_secret_version(
            t.deref_mut(),
            CreateSecretVersion {
                secret_arn: arn.clone(),
                version_id: version_id.clone(),
                version_stage: VersionStage::Current,
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
                return Ok(CreateSecretResponse {
                    arn,
                    name,
                    version_id,
                });
            }

            // Rollback the transaction on failure
            if let Err(error) = t.rollback().await {
                tracing::error!(?error, "failed to rollback transaction");
            }

            tracing::error!(?error, %name, "failed to create secret version");
            return Err(AwsErrorResponse(InternalServiceError).into_response());
        }

        // Attach all the secrets
        for tag in tags {
            if let Err(error) = put_secret_tag(t.deref_mut(), &arn, &tag.key, &tag.value).await {
                // Rollback the transaction on failure
                if let Err(error) = t.rollback().await {
                    tracing::error!(?error, "failed to rollback transaction");
                }

                tracing::error!(?error, "failed to set secret tag");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }
        }

        if let Err(error) = t.commit().await {
            tracing::error!(?error, %name, "failed to commit transaction");
            return Err(AwsErrorResponse(InternalServiceError).into_response());
        }

        Ok(CreateSecretResponse {
            arn,
            name,
            version_id,
        })
    }
}
