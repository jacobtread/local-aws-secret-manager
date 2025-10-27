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

#[derive(Deserialize)]
pub struct UpdateSecretVersionStageRequest {}

#[derive(Serialize)]
pub struct UpdateSecretVersionStageResponse {}

impl Handler for UpdateSecretVersionStageHandler {
    type Request = UpdateSecretVersionStageRequest;
    type Response = UpdateSecretVersionStageResponse;

    async fn handle(_db: &DbPool, _request: Self::Request) -> Result<Self::Response, Response> {
        Err(AwsErrorResponse(NotImplemented).into_response())
    }
}
