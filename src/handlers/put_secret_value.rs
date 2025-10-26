use std::ops::DerefMut;

use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    database::{
        DbPool,
        secrets::{
            CreateSecretVersion, VersionStage, create_secret_version, get_secret_latest_version,
            mark_secret_versions_previous,
        },
    },
    handlers::{
        Handler,
        error::{AwsErrorResponse, ResourceNotFoundException},
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
    secret_binary: Option<Vec<u8>>,
    #[serde(rename = "VersionStages")]
    version_stages: Vec<String>,
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
    version_stages: Vec<VersionStage>,
}

impl Handler for PutSecretValueHandler {
    type Request = PutSecretValueRequest;
    type Response = PutSecretValueResponse;

    async fn handle(db: &DbPool, request: Self::Request) -> Result<Self::Response, Response> {
        let version_id = request
            .client_request_token
            // Generate a new version ID if none was provided
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let version_stages: Vec<VersionStage> = request
            .version_stages
            .into_iter()
            // TODO: Handle unsupported?
            .filter_map(|version| VersionStage::try_from(version).ok())
            .collect();

        let version_stage = version_stages
            .first()
            .copied()
            .unwrap_or(VersionStage::Current);

        if request.secret_string.is_none() && request.secret_binary.is_none() {
            todo!("missing secret error")
        }

        let secret_id = request.secret_id;

        let secret = get_secret_latest_version(db, &secret_id).await.unwrap();
        let secret = match secret {
            Some(value) => value,
            None => return Err(AwsErrorResponse(ResourceNotFoundException).into_response()),
        };

        let mut t = db.begin().await.unwrap();

        if matches!(version_stage, VersionStage::Current) {
            // Mark previous versions as non current
            mark_secret_versions_previous(t.deref_mut(), &secret.arn)
                .await
                .unwrap();
        }

        // Create the initial secret version
        create_secret_version(
            t.deref_mut(),
            CreateSecretVersion {
                secret_arn: secret.arn.clone(),
                version_id: version_id.clone(),
                version_stage,
                secret_string: request.secret_string,
                secret_binary: request.secret_binary,
            },
        )
        .await
        .unwrap();

        t.commit().await.unwrap();

        Ok(PutSecretValueResponse {
            arn: secret.arn,
            name: secret.name,
            version_id: secret.version_id,
            version_stages: vec![secret.version_stage],
        })
    }
}
