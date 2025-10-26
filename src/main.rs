use crate::{
    database::{DbPool, create_database},
    handlers::{
        HandlerRouter, create_secret::CreateSecretHandler, delete_secret::DeleteSecretHandler,
        describe_secret::DescribeSecretHandler, error::NotImplemented,
        get_secret_value::GetSecretValueHandler, list_secrets::ListSecretsHandler,
        put_secret_value::PutSecretValueHandler, update_secret::UpdateSecretHandler,
    },
    logging::init_logging,
};
use axum::{
    Extension, Router,
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::post,
};
use axum_server::tls_rustls::RustlsConfig;
use bytes::Bytes;
use std::{
    error::Error,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
};
use tower_http::trace::TraceLayer;

mod database;
mod handlers;
mod logging;

/// Default server address when not specified (HTTP)
const DEFAULT_SERVER_ADDRESS_HTTP: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 8080));

/// Default server address when not specified (HTTPS)
const DEFAULT_SERVER_ADDRESS_HTTPS: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 8443));

fn main() -> Result<(), Box<dyn Error>> {
    _ = dotenvy::dotenv();

    init_logging();

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(async move {
            if let Err(error) = server().await {
                tracing::error!(?error, message = %error, "error running server");
                return Err(error);
            }

            Ok(())
        })
}

async fn server() -> Result<(), Box<dyn Error>> {
    // Encryption key
    let encryption_key = std::env::var("SM_ENCRYPTION_KEY").inspect_err(|_| {
        tracing::error!("Must specify SM_ENCRYPTION_KEY environment variable");
    })?;

    // Path to the database file
    let database_path =
        std::env::var("SM_DATABASE_PATH").unwrap_or_else(|_| "secrets.db".to_string());

    // Connect to or create an encrypted database file
    let pool = create_database(encryption_key, database_path).await?;

    let handlers = HandlerRouter::default()
        .add_handler("secretsmanager.CreateSecret", CreateSecretHandler)
        .add_handler("secretsmanager.DeleteSecret", DeleteSecretHandler)
        .add_handler("secretsmanager.DescribeSecret", DescribeSecretHandler)
        .add_handler("secretsmanager.GetSecretValue", GetSecretValueHandler)
        .add_handler("secretsmanager.ListSecrets", ListSecretsHandler)
        .add_handler("secretsmanager.PutSecretValue", PutSecretValueHandler)
        .add_handler("secretsmanager.UpdateSecret", UpdateSecretHandler);

    // Setup router
    let app = router()
        .layer(Extension(pool))
        .layer(Extension(Arc::new(handlers)))
        .layer(TraceLayer::new_for_http());

    // Determine whether to use https
    let use_https = match std::env::var("SM_USE_HTTPS") {
        Ok(value) => value.parse::<bool>()?,
        // Default max file size in bytes (100MB)
        Err(_) => false,
    };

    // Determine the socket address to bind against
    let server_address = std::env::var("SERVER_ADDRESS")
        .ok()
        .and_then(|value| value.parse::<SocketAddr>().ok())
        .unwrap_or(if use_https {
            DEFAULT_SERVER_ADDRESS_HTTPS
        } else {
            DEFAULT_SERVER_ADDRESS_HTTP
        });

    // Development mode CORS access for local browser testing
    #[cfg(debug_assertions)]
    let app = app.layer(tower_http::cors::CorsLayer::very_permissive());

    // Log the startup message
    tracing::debug!("server started on {server_address}");

    let handle = axum_server::Handle::default();

    // Handle graceful shutdown on CTRL+C
    tokio::spawn({
        let handle = handle.clone();
        async move {
            _ = tokio::signal::ctrl_c().await;
            handle.graceful_shutdown(None);
        }
    });

    if use_https {
        // Determine whether to use https
        let certificate_path = match std::env::var("SM_HTTPS_CERTIFICATE_PATH") {
            Ok(value) => value,
            Err(_) => "sm.cert.pem".to_string(),
        };

        let private_key_path = match std::env::var("SM_HTTPS_PRIVATE_KEY_PATH") {
            Ok(value) => value,
            Err(_) => "sm.key.pem".to_string(),
        };

        if rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .is_err()
        {
            tracing::error!("failed install default crypto provider");
            return Err(std::io::Error::other("failed to install default crypto provider").into());
        }

        let config = match RustlsConfig::from_pem_file(certificate_path, private_key_path).await {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(?error, "failed to initialize https config");
                return Err(error.into());
            }
        };

        // Serve the app over HTTPS
        axum_server::bind_rustls(server_address, config)
            .handle(handle)
            .serve(app.into_make_service())
            .await?;
    } else {
        // Serve the app over HTTP
        axum_server::bind(server_address)
            .handle(handle)
            .serve(app.into_make_service())
            .await?;
    }

    Ok(())
}

pub fn router() -> Router {
    Router::new().route("/", post(handle_post))
}
async fn handle_post(
    Extension(db): Extension<DbPool>,
    Extension(handlers): Extension<Arc<HandlerRouter>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let target = headers
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        // TODO: Handle missing target
        .unwrap();

    let handler = handlers.get_handler(target);
    match handler {
        Some(value) => value.handle(&db, &body).await,
        None => NotImplemented.into_response(),
    }
}
