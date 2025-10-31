use crate::{
    database::{
        DbPool,
        secrets::{get_secret_latest_version, remove_secret_tag},
    },
    handlers::{
        Handler,
        error::{AwsErrorResponse, InternalServiceError, ResourceNotFoundException},
        models::SecretId,
    },
};
use axum::response::IntoResponse;
use garde::Validate;
use serde::{Deserialize, Serialize};
use std::ops::DerefMut;

/// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_UntagResource.html
pub struct UntagResourceHandler;

#[derive(Deserialize, Validate)]
pub struct UntagResourceRequest {
    #[serde(rename = "SecretId")]
    #[garde(dive)]
    secret_id: SecretId,

    #[serde(rename = "TagKeys")]
    #[garde(inner(length(min = 1, max = 128)))]
    tag_keys: Vec<String>,
}

#[derive(Serialize)]
pub struct UntagResourceResponse {}

impl Handler for UntagResourceHandler {
    type Request = UntagResourceRequest;
    type Response = UntagResourceResponse;

    #[tracing::instrument(skip_all, fields(secret_id = %request.secret_id))]
    async fn handle(
        db: &DbPool,
        request: Self::Request,
    ) -> Result<Self::Response, axum::response::Response> {
        let SecretId(secret_id) = request.secret_id;
        let tag_keys = request.tag_keys;

        let secret = match get_secret_latest_version(db, &secret_id).await {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(?error, "failed to get secret");
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
                tracing::error!(?error, "failed to begin transaction");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }
        };

        // Attach all the secrets
        for key in tag_keys {
            if let Err(error) = remove_secret_tag(t.deref_mut(), &secret.arn, &key).await {
                // Rollback the transaction on failure
                if let Err(error) = t.rollback().await {
                    tracing::error!(?error, "failed to rollback transaction");
                }

                tracing::error!(?error, "failed to remove secret tag");
                return Err(AwsErrorResponse(InternalServiceError).into_response());
            }
        }

        if let Err(error) = t.commit().await {
            tracing::error!(?error, "failed to commit transaction");
            return Err(AwsErrorResponse(InternalServiceError).into_response());
        }

        Ok(UntagResourceResponse {})
    }
}
