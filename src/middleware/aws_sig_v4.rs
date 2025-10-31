use crate::{
    handlers::error::{
        AwsErrorResponse, IncompleteSignature, InvalidClientTokenId, InvalidRequestException,
        MissingAuthenticationToken, SignatureDoesNotMatch,
    },
    utils::aws_sig_v4::{aws_sig_v4, create_canonical_request, parse_auth_header},
};
use axum::{
    body::Body,
    http::{Request, header::AUTHORIZATION},
    response::{IntoResponse, Response},
};
use futures::future::BoxFuture;
use http_body_util::BodyExt;
use std::{mem::swap, sync::Arc};
use tower::{Layer, Service};

/// Credential for the [AwsSigV4AuthLayer] to allow access to
pub struct AwsCredential {
    access_key_id: String,
    access_key_secret: String,
}

impl AwsCredential {
    pub fn new(access_key_id: String, access_key_secret: String) -> Self {
        Self {
            access_key_id,
            access_key_secret,
        }
    }
}

/// Middleware provider layer
#[derive(Clone)]
pub struct AwsSigV4AuthLayer {
    credentials: Arc<AwsCredential>,
}

impl AwsSigV4AuthLayer {
    /// Create a new AWS SigV4 layer using the provided credentials
    pub fn new(credentials: AwsCredential) -> Self {
        Self {
            credentials: Arc::new(credentials),
        }
    }
}

impl<S> Layer<S> for AwsSigV4AuthLayer {
    type Service = AwsSigV4AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AwsSigV4AuthMiddleware {
            inner,
            credentials: self.credentials.clone(),
        }
    }
}

/// Middleware structure
#[derive(Clone)]
pub struct AwsSigV4AuthMiddleware<S> {
    inner: S,
    credentials: Arc<AwsCredential>,
}

impl<S> Service<Request<Body>> for AwsSigV4AuthMiddleware<S>
where
    S: Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        let credential = self.credentials.clone();

        // Swap to ensure we get the service that was ready and not the cloned one
        swap(&mut inner, &mut self.inner);

        Box::pin(async move {
            let (parts, body) = req.into_parts();

            let authorization = match parts.headers.get(AUTHORIZATION) {
                Some(value) => match value.to_str() {
                    Ok(value) => value,
                    // Invalid auth header
                    Err(_) => {
                        return Ok(AwsErrorResponse(InvalidRequestException).into_response());
                    }
                },
                None => {
                    // Unauthorized missing header
                    return Ok(AwsErrorResponse(MissingAuthenticationToken).into_response());
                }
            };

            let amz_date = match parts.headers.get("x-amz-date") {
                Some(value) => match value.to_str() {
                    Ok(value) => value,
                    // Invalid date header
                    Err(_) => {
                        return Ok(AwsErrorResponse(InvalidRequestException).into_response());
                    }
                },
                None => {
                    // Missing date header
                    return Ok(AwsErrorResponse(InvalidRequestException).into_response());
                }
            };

            let auth = match parse_auth_header(authorization) {
                Ok(value) => value,
                Err(_) => {
                    return Ok(AwsErrorResponse(IncompleteSignature).into_response());
                }
            };

            let mut credentials_parts = auth.credential.split('/');
            let access_key_id = match credentials_parts.next() {
                Some(value) => value,
                None => {
                    return Ok(AwsErrorResponse(IncompleteSignature).into_response());
                }
            };

            let date_yyyymmdd = match credentials_parts.next() {
                Some(value) => value,
                None => {
                    return Ok(AwsErrorResponse(IncompleteSignature).into_response());
                }
            };

            let region = match credentials_parts.next() {
                Some(value) => value,
                None => {
                    return Ok(AwsErrorResponse(IncompleteSignature).into_response());
                }
            };

            let service = match credentials_parts.next() {
                Some(value) => value,
                None => {
                    return Ok(AwsErrorResponse(IncompleteSignature).into_response());
                }
            };

            // Missing the aws4_request portion of the credential
            if credentials_parts
                .next()
                .is_none_or(|value| value != "aws4_request")
            {
                return Ok(AwsErrorResponse(IncompleteSignature).into_response());
            }

            if access_key_id != credential.access_key_id {
                // Invalid access key
                return Ok(AwsErrorResponse(InvalidClientTokenId).into_response());
            }

            let body = match body.collect().await {
                Ok(value) => value.to_bytes(),
                Err(_) => {
                    // Failed to ready body
                    return Ok(AwsErrorResponse(InvalidRequestException).into_response());
                }
            };

            let canonical_request = create_canonical_request(&auth.signed_headers, &parts, &body);
            let signature = aws_sig_v4(
                date_yyyymmdd,
                amz_date,
                region,
                service,
                &canonical_request,
                &credential.access_key_secret,
            );

            if signature != auth.signature {
                // Verify failure, bad signature
                return Ok(AwsErrorResponse(SignatureDoesNotMatch).into_response());
            }

            // Re-create the body since we consumed the previous one
            let body = Body::from(body);

            let request = Request::from_parts(parts, body);

            inner.call(request).await
        })
    }
}
