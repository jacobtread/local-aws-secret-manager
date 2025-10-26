use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::{
    database::DbPool,
    handlers::{Handler, error::NotImplemented},
};

// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_DescribeSecret.html
pub struct DescribeSecretHandler;

#[derive(Deserialize)]
pub struct DescribeSecretRequest {}

#[derive(Serialize)]
pub struct DescribeSecretResponse {}

impl Handler for DescribeSecretHandler {
    type Request = DescribeSecretRequest;
    type Response = DescribeSecretResponse;

    async fn handle(_db: &DbPool, _request: Self::Request) -> Result<Self::Response, Response> {
        Err(NotImplemented.into_response())
    }
}
