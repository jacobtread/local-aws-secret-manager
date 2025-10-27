use axum::{Extension, Router, routing::post_service};
use loker::{
    database::{DbPool, initialize_database},
    handlers::{self},
    middleware::aws_sig_v4::{AwsCredential, AwsSigV4AuthLayer},
};
use sqlx::sqlite::SqlitePoolOptions;

use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_sdk_secretsmanager::config::{Credentials, SharedCredentialsProvider};
use tokio::task::AbortHandle;

const TEST_ACCESS_KEY_ID: &str = "test";
const TEST_ACCESS_KEY_SECRET: &str = "test";

/// Create an AWS sdk config for use in tests
#[allow(dead_code)]
pub fn test_sdk_config(endpoint_url: &str) -> SdkConfig {
    let credentials = Credentials::new(
        TEST_ACCESS_KEY_ID,
        TEST_ACCESS_KEY_SECRET,
        None,
        None,
        "test",
    );

    SdkConfig::builder()
        .behavior_version(BehaviorVersion::v2025_08_07())
        .region(Region::from_static("us-east-1"))
        .endpoint_url(endpoint_url)
        .credentials_provider(SharedCredentialsProvider::new(credentials))
        .build()
}

#[allow(dead_code)]
pub struct TestServer {
    sdk_config: SdkConfig,
    pub db: DbPool,
    handle: AbortHandle,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

#[allow(dead_code)]
async fn memory_database() -> DbPool {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .unwrap();
    initialize_database(&pool).await.unwrap();
    pool
}

#[allow(dead_code)]
pub async fn test_server() -> (aws_sdk_secretsmanager::Client, TestServer) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let db = memory_database().await;
    let server_address = listener.local_addr().unwrap();
    let harness_db = db.clone();

    let abort_handle = tokio::spawn(async move {
        let handlers = handlers::create_handlers();
        let handlers_service = handlers.into_service();
        let app = Router::new()
            .route_service("/", post_service(handlers_service))
            .layer(AwsSigV4AuthLayer::new(AwsCredential {
                access_key_id: TEST_ACCESS_KEY_ID.to_string(),
                access_key_secret: TEST_ACCESS_KEY_SECRET.to_string(),
            }))
            .layer(Extension(db.clone()));

        axum::serve(listener, app).await.unwrap();
    })
    .abort_handle();

    let sdk_config = test_sdk_config(&format!("http://{server_address}/"));
    let client = aws_sdk_secretsmanager::Client::new(&sdk_config);

    (
        client,
        TestServer {
            sdk_config,
            handle: abort_handle,
            db: harness_db,
        },
    )
}
