use crate::{
    database::{
        DbPool,
        secrets::{
            CreateSecret, CreateSecretVersion, add_secret_version_stage, create_secret,
            create_secret_version, get_secret_by_version_id, put_secret_tag,
        },
    },
    handlers::{
        Handler,
        error::{
            AwsErrorResponse, InternalServiceError, InvalidRequestException,
            ResourceExistsException,
        },
        models::{ClientRequestToken, SecretBinary, SecretName, SecretString, Tag},
    },
};
use axum::response::{IntoResponse, Response};
use garde::Validate;
use rand::{Rng, distr::Alphanumeric};
use serde::{Deserialize, Serialize};
use std::ops::DerefMut;

// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_CreateSecret.html
pub struct CreateSecretHandler;

#[derive(Deserialize, Validate)]
pub struct CreateSecretRequest {
    #[serde(rename = "Name")]
    #[garde(dive)]
    name: SecretName,

    #[serde(rename = "Description")]
    #[garde(inner(length(max = 2048)))]
    description: Option<String>,

    #[serde(rename = "ClientRequestToken")]
    #[garde(dive)]
    client_request_token: Option<ClientRequestToken>,

    #[serde(rename = "SecretString")]
    #[garde(dive)]
    secret_string: Option<SecretString>,

    #[serde(rename = "SecretBinary")]
    #[garde(dive)]
    secret_binary: Option<SecretBinary>,

    #[serde(rename = "Tags")]
    #[garde(dive)]
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

/// Generate a new secret ARN
///
/// Uses the mock prefix arn:aws:secretsmanager:us-east-1:1:secret:
/// and provides a randomly generated suffix as is done by the
/// official implementation
fn create_secret_arn(name: &str) -> String {
    let random_suffix: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();

    format!("arn:aws:secretsmanager:us-east-1:1:secret:{name}-{random_suffix}")
}

impl Handler for CreateSecretHandler {
    type Request = CreateSecretRequest;
    type Response = CreateSecretResponse;

    #[tracing::instrument(skip_all, fields(name = %request.name))]
    async fn handle(db: &DbPool, request: Self::Request) -> Result<Self::Response, Response> {
        let SecretName(name) = request.name;
        let ClientRequestToken(version_id) = request.client_request_token.unwrap_or_default();

        let arn = create_secret_arn(&name);

        let tags = request.tags.unwrap_or_default();
        let secret_string = request.secret_string.map(SecretString::into_inner);
        let secret_binary = request.secret_binary.map(SecretBinary::into_inner);

        // Must only specify one of the two
        if secret_string.is_some() && secret_binary.is_some() {
            return Err(AwsErrorResponse(InvalidRequestException).into_response());
        }

        // Must specify at least one
        if secret_string.is_none() && secret_binary.is_none() {
            return Err(AwsErrorResponse(InvalidRequestException).into_response());
        }

        let mut t = db.begin().await.map_err(|error| {
            tracing::error!(?error, "failed to begin transaction");
            AwsErrorResponse(InternalServiceError).into_response()
        })?;

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
                let secret = match get_secret_by_version_id(db, &name, &version_id).await {
                    Ok(value) => value,
                    Err(error) => {
                        tracing::error!(?error, "failed to determine existing version");
                        return Err(AwsErrorResponse(InternalServiceError).into_response());
                    }
                };

                let secret = match secret {
                    Some(value) => value,
                    None => {
                        // This version we tried to store was not created so this is an already exists error
                        return Err(AwsErrorResponse(ResourceExistsException).into_response());
                    }
                };

                // If the stored version data doesn't match this is an error that
                // the resource already exists
                if secret.secret_string.ne(&secret_string)
                    || secret.secret_binary.ne(&secret_binary)
                {
                    return Err(AwsErrorResponse(ResourceExistsException).into_response());
                }

                // Request has already been fulfilled
                return Ok(CreateSecretResponse {
                    arn: secret.arn,
                    name,
                    version_id,
                });
            }

            tracing::error!(?error, "failed to create secret");
            return Err(AwsErrorResponse(InternalServiceError).into_response());
        }

        // Create the initial secret version
        if let Err(error) = create_secret_version(
            t.deref_mut(),
            CreateSecretVersion {
                secret_arn: arn.clone(),
                version_id: version_id.clone(),
                secret_string: secret_string.clone(),
                secret_binary: secret_binary.clone(),
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
                        tracing::error!(?error, "failed to determine existing version");
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
                if secret.secret_string.ne(&secret_string)
                    || secret.secret_binary.ne(&secret_binary)
                {
                    return Err(AwsErrorResponse(ResourceExistsException).into_response());
                }

                // Request has already been fulfilled
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

            tracing::error!(?error, "failed to create secret version");
            return Err(AwsErrorResponse(InternalServiceError).into_response());
        }

        // Add the AWSCURRENT stage to the new version
        if let Err(error) =
            add_secret_version_stage(t.deref_mut(), &arn, &version_id, "AWSCURRENT").await
        {
            if let Err(error) = t.rollback().await {
                tracing::error!(?error, "failed to rollback transaction");
            }

            tracing::error!(?error, "failed to add AWSPREVIOUS tag to secret");
            return Err(AwsErrorResponse(InternalServiceError).into_response());
        }

        // Attach all the secrets
        for tag in tags {
            if let Err(error) = put_secret_tag(t.deref_mut(), &arn, &tag.key, &tag.value).await {
                if let Err(error) = t.rollback().await {
                    tracing::error!(?error, "failed to rollback transaction");
                }

                tracing::error!(?error, "failed to set secret tag");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }
        }

        if let Err(error) = t.commit().await {
            tracing::error!(?error, "failed to commit transaction");
            return Err(AwsErrorResponse(InternalServiceError).into_response());
        }

        Ok(CreateSecretResponse {
            arn,
            name,
            version_id,
        })
    }
}
