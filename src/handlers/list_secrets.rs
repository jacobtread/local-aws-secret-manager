use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::{
    database::DbPool,
    handlers::{
        Handler,
        error::{AwsErrorResponse, NotImplemented},
    },
};

// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_ListSecrets.html
pub struct ListSecretsHandler;

#[derive(Deserialize)]
pub struct ListSecretsRequest {}

#[derive(Serialize)]
pub struct ListSecretsResponse {}

impl Handler for ListSecretsHandler {
    type Request = ListSecretsRequest;
    type Response = ListSecretsResponse;

    async fn handle(_db: &DbPool, _request: Self::Request) -> Result<Self::Response, Response> {
        Err(AwsErrorResponse(NotImplemented).into_response())
    }
}
