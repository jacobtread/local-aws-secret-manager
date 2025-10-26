use crate::{
    database::{
        DbPool,
        secrets::{get_secret_latest_version, put_secret_tag},
    },
    handlers::{
        Handler, Tag,
        error::{AwsErrorResponse, InternalServiceError, ResourceNotFoundException},
    },
};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use std::ops::DerefMut;

/// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_TagResource.html
pub struct TagResourceHandler;

#[derive(Deserialize)]
pub struct TagResourceRequest {
    #[serde(rename = "SecretId")]
    secret_id: String,
    #[serde(rename = "Tags")]
    tags: Vec<Tag>,
}

#[derive(Serialize)]
pub struct TagResourceResponse {}

impl Handler for TagResourceHandler {
    type Request = TagResourceRequest;
    type Response = TagResourceResponse;

    async fn handle(
        db: &DbPool,
        request: Self::Request,
    ) -> Result<Self::Response, axum::response::Response> {
        let secret_id = request.secret_id;
        let tags = request.tags;

        let secret = match get_secret_latest_version(db, &secret_id).await {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(?error, %secret_id, "failed to get secret");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }
        };

        let secret = match secret {
            Some(value) => value,
            None => return Err(AwsErrorResponse(ResourceNotFoundException).into_response()),
        };

        let mut t = match db.begin().await {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(?error, name = %secret.name, "failed to begin transaction");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }
        };

        // Attach all the secrets
        for tag in tags {
            if let Err(error) =
                put_secret_tag(t.deref_mut(), &secret.arn, &tag.key, &tag.value).await
            {
                tracing::error!(?error, "failed to set secret tag");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }
        }

        if let Err(error) = t.commit().await {
            tracing::error!(?error, name = %secret.name,  "failed to commit transaction");
            return Err(AwsErrorResponse(InternalServiceError).into_response());
        }

        Ok(TagResourceResponse {})
    }
}
