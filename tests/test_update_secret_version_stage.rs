use aws_sdk_secretsmanager::types::Tag;

use crate::common::test_server;

mod common;

/// Tests that the AWSCURRENT stage can be removed from the current secret
#[tokio::test]
async fn test_update_secret_version_stage_remove_current_stage() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    client
        .update_secret_version_stage()
        .secret_id("test")
        .version_stage("AWSCURRENT")
        .remove_from_version_id(create_response.version_id().unwrap())
        .send()
        .await
        .unwrap();

    let describe_response = client
        .get_secret_value()
        .secret_id("test")
        .version_id(create_response.version_id().unwrap())
        .send()
        .await
        .unwrap();

    assert!(describe_response.version_stages().is_empty());
}

/// Tests that the AWSPREVIOUS stage can be removed from the previous secret
#[tokio::test]
async fn test_update_secret_version_stage_remove_previous_stage() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let _version_2 = client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-2")
        .send()
        .await
        .unwrap();

    client
        .update_secret_version_stage()
        .secret_id("test")
        .version_stage("AWSPREVIOUS")
        .remove_from_version_id(create_response.version_id().unwrap())
        .send()
        .await
        .unwrap();

    let describe_response = client
        .get_secret_value()
        .secret_id("test")
        .version_id(create_response.version_id().unwrap())
        .send()
        .await
        .unwrap();

    assert!(describe_response.version_stages().is_empty());
}

/// Tests moving the AWSCURRENT stage from the current version to another version
/// also causes the AWSPREVIOUS to be moved to the other secret
#[tokio::test]
async fn test_update_secret_version_stage_swap_current_stage() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let version_2 = client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-2")
        .send()
        .await
        .unwrap();

    client
        .update_secret_version_stage()
        .secret_id("test")
        .version_stage("AWSCURRENT")
        .remove_from_version_id(version_2.version_id().unwrap())
        .move_to_version_id(create_response.version_id().unwrap())
        .send()
        .await
        .unwrap();

    let describe_response_1 = client
        .get_secret_value()
        .secret_id("test")
        .version_id(create_response.version_id().unwrap())
        .send()
        .await
        .unwrap();

    assert_eq!(
        describe_response_1.version_stages(),
        &["AWSCURRENT".to_string()]
    );

    let describe_response_2 = client
        .get_secret_value()
        .secret_id("test")
        .version_id(version_2.version_id().unwrap())
        .send()
        .await
        .unwrap();

    assert_eq!(
        describe_response_2.version_stages(),
        &["AWSPREVIOUS".to_string()]
    );
}

/// Tests moving the AWSPREVIOUS stage from the current version to another version
#[tokio::test]
async fn test_update_secret_version_stage_swap_previous_stage() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let version_2 = client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-2")
        .send()
        .await
        .unwrap();

    let version_3 = client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-3")
        .send()
        .await
        .unwrap();

    client
        .update_secret_version_stage()
        .secret_id("test")
        .version_stage("AWSPREVIOUS")
        .remove_from_version_id(version_2.version_id().unwrap())
        .move_to_version_id(create_response.version_id().unwrap())
        .send()
        .await
        .unwrap();

    // Initial version should not have the AWSPREVIOUS stage
    let describe_response_1 = client
        .get_secret_value()
        .secret_id("test")
        .version_id(create_response.version_id().unwrap())
        .send()
        .await
        .unwrap();

    assert_eq!(
        describe_response_1.version_stages(),
        &["AWSPREVIOUS".to_string()]
    );

    // Second version should not have any versions stages left
    let describe_response_2 = client
        .get_secret_value()
        .secret_id("test")
        .version_id(version_2.version_id().unwrap())
        .send()
        .await
        .unwrap();

    assert!(describe_response_2.version_stages().is_empty());

    // Latest version should still be AWSCURRENT
    let describe_response_3 = client
        .get_secret_value()
        .secret_id("test")
        .version_id(version_3.version_id().unwrap())
        .send()
        .await
        .unwrap();

    assert_eq!(
        describe_response_3.version_stages(),
        &["AWSCURRENT".to_string()]
    );
}

/// Tests adding custom stages to the current version
#[tokio::test]
async fn test_update_secret_version_stage_add_custom_current_stage() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    client
        .update_secret_version_stage()
        .secret_id("test")
        .version_stage("CUSTOM")
        .move_to_version_id(create_response.version_id().unwrap())
        .send()
        .await
        .unwrap();

    let describe_response_1 = client
        .get_secret_value()
        .secret_id("test")
        .version_id(create_response.version_id().unwrap())
        .send()
        .await
        .unwrap();

    assert_eq!(
        describe_response_1.version_stages(),
        &["AWSCURRENT".to_string(), "CUSTOM".to_string()]
    );
}

/// Tests adding custom stages to the current version
#[tokio::test]
async fn test_update_secret_version_stage_add_custom_multiple_current_stage() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    client
        .update_secret_version_stage()
        .secret_id("test")
        .version_stage("CUSTOM")
        .move_to_version_id(create_response.version_id().unwrap())
        .send()
        .await
        .unwrap();

    client
        .update_secret_version_stage()
        .secret_id("test")
        .version_stage("CUSTOM_2")
        .move_to_version_id(create_response.version_id().unwrap())
        .send()
        .await
        .unwrap();

    let describe_response_1 = client
        .get_secret_value()
        .secret_id("test")
        .version_id(create_response.version_id().unwrap())
        .send()
        .await
        .unwrap();

    assert_eq!(
        describe_response_1.version_stages(),
        &[
            "AWSCURRENT".to_string(),
            "CUSTOM".to_string(),
            "CUSTOM_2".to_string()
        ]
    );
}

/// Tests adding custom stages to the non current version
#[tokio::test]
async fn test_update_secret_version_stage_add_custom_non_current_stage() {
    let (client, _server) = test_server().await;

    let _create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let version_2 = client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-2")
        .send()
        .await
        .unwrap();

    client
        .update_secret_version_stage()
        .secret_id("test")
        .version_stage("CUSTOM")
        .move_to_version_id(version_2.version_id().unwrap())
        .send()
        .await
        .unwrap();

    let describe_response_1 = client
        .get_secret_value()
        .secret_id("test")
        .version_id(version_2.version_id().unwrap())
        .send()
        .await
        .unwrap();

    assert_eq!(
        describe_response_1.version_stages(),
        &["AWSCURRENT".to_string(), "CUSTOM".to_string()]
    );
}

/// Tests adding custom stages to the non current version
#[tokio::test]
async fn test_update_secret_version_stage_add_custom_multiple_non_current_stage() {
    let (client, _server) = test_server().await;

    let _create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let version_2 = client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-2")
        .send()
        .await
        .unwrap();

    client
        .update_secret_version_stage()
        .secret_id("test")
        .version_stage("CUSTOM")
        .move_to_version_id(version_2.version_id().unwrap())
        .send()
        .await
        .unwrap();

    client
        .update_secret_version_stage()
        .secret_id("test")
        .version_stage("CUSTOM_2")
        .move_to_version_id(version_2.version_id().unwrap())
        .send()
        .await
        .unwrap();

    let describe_response_1 = client
        .get_secret_value()
        .secret_id("test")
        .version_id(version_2.version_id().unwrap())
        .send()
        .await
        .unwrap();

    assert_eq!(
        describe_response_1.version_stages(),
        &[
            "AWSCURRENT".to_string(),
            "CUSTOM".to_string(),
            "CUSTOM_2".to_string()
        ]
    );
}

/// Tests adding custom stages to the non current version that currently has no stages
#[tokio::test]
async fn test_update_secret_version_stage_add_custom_non_current_empty_stage() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let _version_2 = client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-2")
        .send()
        .await
        .unwrap();

    let _version_3 = client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-2")
        .send()
        .await
        .unwrap();

    client
        .update_secret_version_stage()
        .secret_id("test")
        .version_stage("CUSTOM")
        .move_to_version_id(create_response.version_id().unwrap())
        .send()
        .await
        .unwrap();

    let describe_response_1 = client
        .get_secret_value()
        .secret_id("test")
        .version_id(create_response.version_id().unwrap())
        .send()
        .await
        .unwrap();

    assert_eq!(
        describe_response_1.version_stages(),
        &["CUSTOM".to_string()]
    );
}

/// Tests adding custom stages to the non current version that currently has no stages
#[tokio::test]
async fn test_update_secret_version_stage_add_custom_multiple_non_current_empty_stage() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let _version_2 = client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-2")
        .send()
        .await
        .unwrap();

    let _version_3 = client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-2")
        .send()
        .await
        .unwrap();

    client
        .update_secret_version_stage()
        .secret_id("test")
        .version_stage("CUSTOM")
        .move_to_version_id(create_response.version_id().unwrap())
        .send()
        .await
        .unwrap();

    client
        .update_secret_version_stage()
        .secret_id("test")
        .version_stage("CUSTOM_2")
        .move_to_version_id(create_response.version_id().unwrap())
        .send()
        .await
        .unwrap();

    let describe_response_1 = client
        .get_secret_value()
        .secret_id("test")
        .version_id(create_response.version_id().unwrap())
        .send()
        .await
        .unwrap();

    assert_eq!(
        describe_response_1.version_stages(),
        &["CUSTOM".to_string(), "CUSTOM_2".to_string()]
    );
}

/// Tests that moving a version stage to another version while its already attached
/// to a version with specifying to remove it from that version should return
/// an error
#[tokio::test]
async fn test_update_secret_version_stage_move_without_remove_error() {}
