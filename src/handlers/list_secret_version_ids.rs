use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::{
    database::DbPool,
    handlers::{
        Handler,
        error::{AwsErrorResponse, NotImplemented},
    },
};

// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_ListSecretVersionIds.html
pub struct ListSecretVersionIdsHandler;

#[derive(Deserialize)]
pub struct ListSecretVersionIdsRequest {}

#[derive(Serialize)]
pub struct ListSecretVersionIdsResponse {}

impl Handler for ListSecretVersionIdsHandler {
    type Request = ListSecretVersionIdsRequest;
    type Response = ListSecretVersionIdsResponse;

    async fn handle(_db: &DbPool, _request: Self::Request) -> Result<Self::Response, Response> {
        Err(AwsErrorResponse(NotImplemented).into_response())
    }
}
