use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::{
    database::DbPool,
    handlers::{
        Handler,
        error::{AwsErrorResponse, NotImplemented},
    },
};

// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_BatchGetSecretValue.html
pub struct BatchGetSecretValueHandler;

#[derive(Deserialize)]
pub struct BatchGetSecretValueRequest {}

#[derive(Serialize)]
pub struct BatchGetSecretValueResponse {}

impl Handler for BatchGetSecretValueHandler {
    type Request = BatchGetSecretValueRequest;
    type Response = BatchGetSecretValueResponse;

    async fn handle(_db: &DbPool, _request: Self::Request) -> Result<Self::Response, Response> {
        Err(AwsErrorResponse(NotImplemented).into_response())
    }
}
