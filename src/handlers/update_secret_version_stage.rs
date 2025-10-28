use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::{
    database::DbPool,
    handlers::{
        Handler,
        error::{AwsErrorResponse, NotImplemented},
    },
};

// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_UpdateSecretVersionStage.html
pub struct UpdateSecretVersionStageHandler;

#[allow(unused)]
#[derive(Deserialize)]
pub struct UpdateSecretVersionStageRequest {
    #[serde(rename = "MoveToVersionId")]
    move_to_version_id: Option<String>,
    #[serde(rename = "RemoveFromVersionId")]
    remove_from_version_id: Option<String>,
    #[serde(rename = "SecretId")]
    secret_id: String,
    #[serde(rename = "VersionStage")]
    version_stage: String,
}

#[derive(Serialize)]
pub struct UpdateSecretVersionStageResponse {}

impl Handler for UpdateSecretVersionStageHandler {
    type Request = UpdateSecretVersionStageRequest;
    type Response = UpdateSecretVersionStageResponse;

    async fn handle(_db: &DbPool, request: Self::Request) -> Result<Self::Response, Response> {
        let _secret_id = request.secret_id;
        Err(AwsErrorResponse(NotImplemented).into_response())
    }
}
