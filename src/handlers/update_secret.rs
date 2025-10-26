use std::ops::DerefMut;

use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    database::{
        DbPool,
        secrets::{
            CreateSecretVersion, VersionStage, create_secret_version, get_secret_latest_version,
            mark_secret_versions_previous, update_secret_description,
        },
    },
    handlers::{
        Handler,
        error::{AwsErrorResponse, ResourceNotFoundException},
    },
};

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
    secret_binary: Option<Vec<u8>>,
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

        let secret = get_secret_latest_version(db, &secret_id).await.unwrap();
        let secret = match secret {
            Some(value) => value,
            None => return Err(AwsErrorResponse(ResourceNotFoundException).into_response()),
        };

        let mut t = db.begin().await.unwrap();

        if let Some(description) = request.description {
            update_secret_description(t.deref_mut(), &secret.arn, &description)
                .await
                .unwrap();
        }

        let version_id = if request.secret_string.is_some() || request.secret_binary.is_some() {
            let version_id = request
                .client_request_token
                // Generate a new version ID if none was provided
                .unwrap_or_else(|| Uuid::new_v4().to_string());

            // Mark previous versions as non current
            mark_secret_versions_previous(t.deref_mut(), &secret.arn)
                .await
                .unwrap();

            // Create a new current secret version
            create_secret_version(
                t.deref_mut(),
                CreateSecretVersion {
                    secret_arn: secret.arn.clone(),
                    version_id: version_id.clone(),
                    version_stage: VersionStage::Current,
                    secret_string: request.secret_string,
                    secret_binary: request.secret_binary,
                },
            )
            .await
            .unwrap();

            Some(version_id)
        } else {
            None
        };

        t.commit().await.unwrap();

        Ok(UpdateSecretResponse {
            arn: secret.arn,
            name: secret.name,
            version_id,
        })
    }
}
