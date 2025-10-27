use aws_sdk_secretsmanager::{primitives::Blob, types::Tag};
use loker::database::secrets::VersionStage;

use crate::common::test_server;

mod common;

/// Tests that a string secret can be updated to a new value
#[tokio::test]
async fn test_put_secret_value_string_secret_success() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let get_response = client
        .get_secret_value()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // Created ARN should match
    assert_eq!(get_response.arn(), create_response.arn());

    // Retrieved value should match created
    assert_eq!(get_response.secret_string(), Some("test"));

    let put_response = client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-updated")
        .send()
        .await
        .unwrap();

    // ARN should match
    assert_eq!(put_response.arn(), create_response.arn());

    // Name should match
    assert_eq!(put_response.name(), create_response.name());

    // Version number should have changed
    assert_ne!(put_response.version_id(), create_response.version_id());

    // When no stage is present the stage matched should be
    assert_eq!(
        put_response.version_stages(),
        &[VersionStage::Current.to_string()]
    );

    let get_response = client
        .get_secret_value()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // ARN should still match
    assert_eq!(get_response.arn(), create_response.arn());

    // Retrieved value should match created
    assert_eq!(get_response.secret_string(), Some("test-updated"));

    // Version number should have changed
    assert_eq!(get_response.version_id(), put_response.version_id());

    // Should be in the current stage
    assert_eq!(
        get_response.version_stages(),
        &[VersionStage::Current.to_string()]
    );
}

/// Tests that a binary secret can be updated to a new value
#[tokio::test]
async fn test_put_secret_value_binary_secret_success() {
    let (client, _server) = test_server().await;

    let binary_secret = Blob::new(b"TEST");

    let create_response = client
        .create_secret()
        .name("test")
        .secret_binary(binary_secret.clone())
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let get_response = client
        .get_secret_value()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // Created ARN should match
    assert_eq!(get_response.arn(), create_response.arn());

    // Retrieved value should match created
    assert_eq!(get_response.secret_binary(), Some(&binary_secret));

    let binary_secret = Blob::new(b"TEST2");

    let put_response = client
        .put_secret_value()
        .secret_id("test")
        .secret_binary(binary_secret.clone())
        .send()
        .await
        .unwrap();

    // ARN should match
    assert_eq!(put_response.arn(), create_response.arn());

    // Name should match
    assert_eq!(put_response.name(), create_response.name());

    // Version number should have changed
    assert_ne!(put_response.version_id(), create_response.version_id());

    // When no stage is present the stage matched should be
    assert_eq!(
        put_response.version_stages(),
        &[VersionStage::Current.to_string()]
    );

    let get_response = client
        .get_secret_value()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // ARN should still match
    assert_eq!(get_response.arn(), create_response.arn());

    // Retrieved value should match created
    assert_eq!(get_response.secret_binary(), Some(&binary_secret));

    // Version number should have changed
    assert_eq!(get_response.version_id(), put_response.version_id());

    // Should be in the current stage
    assert_eq!(
        get_response.version_stages(),
        &[VersionStage::Current.to_string()]
    );
}
