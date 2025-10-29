use crate::common::test_server;
use aws_sdk_secretsmanager::types::{Filter, Tag};

mod common;

#[tokio::test]
async fn test_list_secrets_all() {
    let (client, _server) = test_server().await;

    let create_response_1 = client
        .create_secret()
        .name("test-1")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let create_response_2 = client
        .create_secret()
        .name("test-2")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let create_response_3 = client
        .create_secret()
        .name("test-3")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let list_1 = client
        .list_secrets()
        .sort_order(aws_sdk_secretsmanager::types::SortOrderType::Asc)
        .send()
        .await
        .unwrap();

    let secret_list = list_1.secret_list();
    assert_eq!(secret_list.len(), 3);

    let mut secret_list = secret_list.iter();

    let first_secret = secret_list.next().unwrap();
    assert_eq!(first_secret.arn(), create_response_1.arn());
    assert_eq!(first_secret.name(), create_response_1.name());

    let second_secret = secret_list.next().unwrap();
    assert_eq!(second_secret.arn(), create_response_2.arn());
    assert_eq!(second_secret.name(), create_response_2.name());

    let third_secret = secret_list.next().unwrap();
    assert_eq!(third_secret.arn(), create_response_3.arn());
    assert_eq!(third_secret.name(), create_response_3.name());
}

#[tokio::test]
async fn test_list_secrets_asc() {
    let (client, _server) = test_server().await;

    let create_response_1 = client
        .create_secret()
        .name("test-1")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let create_response_2 = client
        .create_secret()
        .name("test-2")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let create_response_3 = client
        .create_secret()
        .name("test-3")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let _create_response_4 = client
        .create_secret()
        .name("name-that-one-match")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let list_1 = client
        .list_secrets()
        .filters(
            Filter::builder()
                .key(aws_sdk_secretsmanager::types::FilterNameStringType::Name)
                .values("test-")
                .build(),
        )
        .sort_order(aws_sdk_secretsmanager::types::SortOrderType::Asc)
        .send()
        .await
        .unwrap();

    let secret_list = list_1.secret_list();
    assert_eq!(secret_list.len(), 3);

    let mut secret_list = secret_list.iter();

    let first_secret = secret_list.next().unwrap();
    assert_eq!(first_secret.arn(), create_response_1.arn());
    assert_eq!(first_secret.name(), create_response_1.name());

    let second_secret = secret_list.next().unwrap();
    assert_eq!(second_secret.arn(), create_response_2.arn());
    assert_eq!(second_secret.name(), create_response_2.name());

    let third_secret = secret_list.next().unwrap();
    assert_eq!(third_secret.arn(), create_response_3.arn());
    assert_eq!(third_secret.name(), create_response_3.name());
}

/// Tests that secrets can be found by prefix match for the description
#[tokio::test]
async fn test_list_secrets_description_filter() {}

/// Tests that no matches will be found when theres no matching description
#[tokio::test]
async fn test_list_secrets_description_filter_no_matches() {}

/// Tests that secrets can be found by prefix match for the name
#[tokio::test]
async fn test_list_secrets_name_filter() {}

/// Tests that no matches will be found when theres no matching name
#[tokio::test]
async fn test_list_secrets_name_filter_no_matches() {}

/// Tests that secrets can be found by prefix match for an associated tag key
#[tokio::test]
async fn test_list_secrets_tag_key_filter() {}

/// Tests that no matches will be found when theres no matching tag key
#[tokio::test]
async fn test_list_secrets_tag_key_filter_no_matches() {}

/// Tests that secrets can be found by prefix match for an associated tag value
#[tokio::test]
async fn test_list_secrets_tag_value_filter() {}

/// Tests that no matches will be found when theres no matching tag value
#[tokio::test]
async fn test_list_secrets_tag_value_filter_no_matches() {}

/// Tests that matches for all an be found
#[tokio::test]
async fn test_list_secrets_all_value_filter() {}

/// Tests that bad all matches will have no results
#[tokio::test]
async fn test_list_secrets_all_value_filter_no_matches() {}

/// Tests that combining a name and tag filter together will yield more specific results
#[tokio::test]
async fn test_list_secrets_name_and_tag_filter() {}

/// Tests that combining a name and tag filter together will yield no results
/// when theres no secrets that meet the criteria
#[tokio::test]
async fn test_list_secrets_name_and_tag_filter_no_matches() {}

/// Tests that requesting a list before creating any secrets will return an empty list
#[tokio::test]
async fn test_list_secrets_empty_list() {}

/// Tests that requesting a list with no filters will provide all secrets that are
/// not pending deletion in DESC order
#[tokio::test]
async fn test_list_secrets_no_filters_list() {}

/// Tests that requesting a list with no filters with the asc sort order
/// will provide all secrets that are not pending deletion in ASC order
#[tokio::test]
async fn test_list_secrets_no_filters_asc_list() {}

/// Tests that requesting a list with no filters will provide all secrets
/// including those pending deletion in DESC order
#[tokio::test]
async fn test_list_secrets_no_filters_include_pending_deletion_list() {}

/// Tests that requesting a list with no filters with the asc sort order
/// will provide all secrets including those pending deletion in ASC order
#[tokio::test]
async fn test_list_secrets_no_filters_include_pending_deletion_asc_list() {}

/// Tests that the default pagination size works correctly
#[tokio::test]
async fn test_list_secrets_default_pagination() {}

/// Tests that the custom pagination size works correctly
#[tokio::test]
async fn test_list_secrets_custom_pagination() {}

/// Tests that requesting an invalid pagination token should error
#[tokio::test]
async fn test_list_secrets_invalid_pagination_erro() {}
