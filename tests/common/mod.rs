use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_sdk_secretsmanager::config::{Credentials, SharedCredentialsProvider};

/// Create an AWS sdk config for use in tests
pub fn test_sdk_config() -> SdkConfig {
    // 1. Provide dummy credentials
    let credentials = Credentials::new("test_access_key", "test_secret_key", None, None, "test");

    SdkConfig::builder()
        .behavior_version(BehaviorVersion::v2025_08_07())
        .region(Region::from_static("us-east-1"))
        .endpoint_url("http://localhost:8080")
        .credentials_provider(SharedCredentialsProvider::new(credentials))
        .build()
}
