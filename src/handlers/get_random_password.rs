use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::{
    database::DbPool,
    handlers::{
        Handler,
        error::{AwsErrorResponse, NotImplemented},
    },
};

// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_GetRandomPassword.html
pub struct GetRandomPasswordHandler;

#[derive(Deserialize)]
pub struct GetRandomPasswordRequest {}

#[derive(Serialize)]
pub struct GetRandomPasswordResponse {}

impl Handler for GetRandomPasswordHandler {
    type Request = GetRandomPasswordRequest;
    type Response = GetRandomPasswordResponse;

    async fn handle(_db: &DbPool, _request: Self::Request) -> Result<Self::Response, Response> {
        Err(AwsErrorResponse(NotImplemented).into_response())
    }
}
