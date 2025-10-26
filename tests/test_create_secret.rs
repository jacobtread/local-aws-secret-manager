use aws_sdk_secretsmanager::types::Tag;

use crate::common::test_server;

mod common;

/// Tests that a string secret can be created successfully
#[tokio::test]
async fn test_create_secret_string_success() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    // Server should reply with a version_id for the created version
    assert!(create_response.version_id().is_some());

    // Name should match
    assert_eq!(create_response.name(), Some("test"));

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

    // Retrieved version should match created
    assert_eq!(get_response.version_id(), create_response.version_id());

    // Created secret should be in the AWSCURRENT version stage
    assert_eq!(
        get_response
            .version_stages()
            .first()
            .map(|value| value.as_ref()),
        Some("AWSCURRENT")
    );

    let describe_response = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // Should have a matching tag
    assert_eq!(
        describe_response.tags(),
        &[Tag::builder().key("test-tag").value("test-value").build()]
    );
}
