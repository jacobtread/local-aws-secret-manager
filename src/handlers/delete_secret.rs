use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::{
    database::DbPool,
    handlers::{
        Handler,
        error::{AwsErrorResponse, NotImplemented},
    },
};

// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_DeleteSecret.html
pub struct DeleteSecretHandler;

#[derive(Deserialize)]
pub struct DeleteSecretRequest {}

#[derive(Serialize)]
pub struct DeleteSecretResponse {}

impl Handler for DeleteSecretHandler {
    type Request = DeleteSecretRequest;
    type Response = DeleteSecretResponse;

    async fn handle(_db: &DbPool, _request: Self::Request) -> Result<Self::Response, Response> {
        Err(AwsErrorResponse(NotImplemented).into_response())
    }
}
