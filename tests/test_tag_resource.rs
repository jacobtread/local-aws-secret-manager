use crate::common::test_server;
use aws_sdk_secretsmanager::{
    error::SdkError,
    operation::tag_resource::TagResourceError,
    types::{Tag, error::ResourceNotFoundException},
};

mod common;

/// Tests that a secret tag can be created successfully
#[tokio::test]
async fn test_tag_resource_success() {
    let (client, _server) = test_server().await;

    let _create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    client
        .tag_resource()
        .secret_id("test")
        .tags(
            Tag::builder()
                .key("test-tag-2")
                .value("test-value-2")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let describe_response = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // Should have a matching tag
    assert_eq!(
        describe_response.tags(),
        &[
            Tag::builder().key("test-tag").value("test-value").build(),
            Tag::builder()
                .key("test-tag-2")
                .value("test-value-2")
                .build(),
        ]
    );
}

/// Tests that multiple tags can be added to a resource
#[tokio::test]
async fn test_tag_resource_multiple_success() {
    let (client, _server) = test_server().await;

    let _create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    client
        .tag_resource()
        .secret_id("test")
        .tags(
            Tag::builder()
                .key("test-tag-2")
                .value("test-value-2")
                .build(),
        )
        .tags(
            Tag::builder()
                .key("test-tag-3")
                .value("test-value-3")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let describe_response = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // Should have a matching tag
    assert_eq!(
        describe_response.tags(),
        &[
            Tag::builder().key("test-tag").value("test-value").build(),
            Tag::builder()
                .key("test-tag-2")
                .value("test-value-2")
                .build(),
            Tag::builder()
                .key("test-tag-3")
                .value("test-value-3")
                .build(),
        ]
    );
}

/// Tests that multiple tags can be added to a resource and overrides
/// can happen in the same operation
#[tokio::test]
async fn test_tag_resource_multiple_with_override_success() {
    let (client, _server) = test_server().await;

    let _create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    client
        .tag_resource()
        .secret_id("test")
        .tags(Tag::builder().key("test-tag").value("test-value-2").build())
        .tags(
            Tag::builder()
                .key("test-tag-2")
                .value("test-value-2")
                .build(),
        )
        .tags(
            Tag::builder()
                .key("test-tag-3")
                .value("test-value-3")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let describe_response = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // Should have a matching tag
    assert_eq!(
        describe_response.tags(),
        &[
            Tag::builder().key("test-tag").value("test-value-2").build(),
            Tag::builder()
                .key("test-tag-2")
                .value("test-value-2")
                .build(),
            Tag::builder()
                .key("test-tag-3")
                .value("test-value-3")
                .build(),
        ]
    );
}

/// Tests that when providing an existing tag key the previous
/// value will be overridden
#[tokio::test]
async fn test_tag_resource_override_success() {
    let (client, _server) = test_server().await;

    let _create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    client
        .tag_resource()
        .secret_id("test")
        .tags(Tag::builder().key("test-tag").value("test-value-2").build())
        .send()
        .await
        .unwrap();

    let describe_response = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // Should have a matching tag
    assert_eq!(
        describe_response.tags(),
        &[Tag::builder().key("test-tag").value("test-value-2").build(),]
    );
}

/// Tests that trying to tag an unknown resource will fail
#[tokio::test]
async fn test_tag_resource_unknown_error() {
    let (client, _server) = test_server().await;

    let tag_err = client
        .tag_resource()
        .secret_id("test")
        .tags(Tag::builder().key("test-tag").value("test-value-2").build())
        .send()
        .await
        .unwrap_err();

    let tag_err = match tag_err {
        SdkError::ServiceError(error) => error,
        error => panic!("expected SdkError::ServiceError got {error:?}"),
    };

    let _exception: ResourceNotFoundException = match tag_err.into_err() {
        TagResourceError::ResourceNotFoundException(error) => error,
        error => panic!("expected TagResourceError::ResourceNotFoundException got {error:?}"),
    };
}
