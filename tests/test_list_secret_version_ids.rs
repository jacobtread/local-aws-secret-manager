use crate::common::test_server;
use aws_sdk_secretsmanager::types::Tag;

mod common;

/// Tests that the initial version ID for a created version is present
#[tokio::test]
async fn test_list_secret_version_ids_created() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let versions = client
        .list_secret_version_ids()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // ARN and name should match
    assert_eq!(versions.arn(), create_response.arn());
    assert_eq!(versions.name(), create_response.name());

    // Shouldn't have enough versions to paginate (Default max is 100)
    assert_eq!(versions.next_token(), None);

    let versions = versions.versions.unwrap();

    // Should have one version
    assert_eq!(versions.len(), 1);

    let first_version = versions.first().unwrap();
    assert_eq!(first_version.version_id(), create_response.version_id());
    assert_eq!(first_version.version_stages(), &["AWSCURRENT".to_string()]);
    assert!(first_version.created_date().unwrap().secs() > 0);
    assert_eq!(first_version.kms_key_ids, None);
    assert_eq!(first_version.last_accessed_date, None);
}

/// Tests that when one new version has been added when not requesting deprecated secrets
/// that both the initial and new version are present
#[tokio::test]
async fn test_list_secret_version_ids_one() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let version_1 = client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-updated")
        .send()
        .await
        .unwrap();

    let versions = client
        .list_secret_version_ids()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // ARN and name should match
    assert_eq!(versions.arn(), create_response.arn());
    assert_eq!(versions.name(), create_response.name());

    // Shouldn't have enough versions to paginate (Default max is 100)
    assert_eq!(versions.next_token(), None);

    let versions = versions.versions.unwrap();

    // Should have two versions
    assert_eq!(versions.len(), 2);

    let mut versions = versions.into_iter();

    let first_version = versions.next().unwrap();
    assert_eq!(first_version.version_id(), version_1.version_id());
    assert_eq!(first_version.version_stages(), &["AWSCURRENT".to_string()]);
    assert!(first_version.created_date().unwrap().secs() > 0);
    assert_eq!(first_version.kms_key_ids, None);
    assert_eq!(first_version.last_accessed_date, None);

    // Second version should be the initial created version and should be AWSPREVIOUS
    let second_version = versions.next().unwrap();
    assert_eq!(second_version.version_id(), create_response.version_id());
    assert_eq!(
        second_version.version_stages(),
        &["AWSPREVIOUS".to_string()]
    );
    assert!(second_version.created_date().unwrap().secs() > 0);
    assert_eq!(second_version.kms_key_ids, None);
    assert_eq!(second_version.last_accessed_date, None);
}

/// Tests that when two versions have been added when not requesting deprecated secrets
/// the third version will not be present
#[tokio::test]
async fn test_list_secret_version_ids_two() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let version_1 = client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-updated")
        .send()
        .await
        .unwrap();

    let version_2 = client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-updated")
        .send()
        .await
        .unwrap();

    let versions = client
        .list_secret_version_ids()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // ARN and name should match
    assert_eq!(versions.arn(), create_response.arn());
    assert_eq!(versions.name(), create_response.name());

    // Shouldn't have enough versions to paginate (Default max is 100)
    assert_eq!(versions.next_token(), None);

    let versions = versions.versions.unwrap();

    // Should have two versions
    assert_eq!(versions.len(), 2);

    let mut versions = versions.into_iter();

    let first_version = versions.next().unwrap();
    assert_eq!(first_version.version_id(), version_2.version_id());
    assert_eq!(first_version.version_stages(), &["AWSCURRENT".to_string()]);
    assert!(first_version.created_date().unwrap().secs() > 0);
    assert_eq!(first_version.kms_key_ids, None);
    assert_eq!(first_version.last_accessed_date, None);

    // Second version should be the initial created version and should be AWSPREVIOUS
    let second_version = versions.next().unwrap();
    assert_eq!(second_version.version_id(), version_1.version_id());
    assert_eq!(
        second_version.version_stages(),
        &["AWSPREVIOUS".to_string()]
    );
    assert!(second_version.created_date().unwrap().secs() > 0);
    assert_eq!(second_version.kms_key_ids, None);
    assert_eq!(second_version.last_accessed_date, None);
}

/// Tests that when two versions have been added when  requesting deprecated secrets
/// the third version will be present
#[tokio::test]
async fn test_list_secret_version_ids_two_deprecated() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let version_1 = client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-updated")
        .send()
        .await
        .unwrap();

    let version_2 = client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-updated")
        .send()
        .await
        .unwrap();

    let versions = client
        .list_secret_version_ids()
        .secret_id("test")
        .include_deprecated(true)
        .send()
        .await
        .unwrap();

    // ARN and name should match
    assert_eq!(versions.arn(), create_response.arn());
    assert_eq!(versions.name(), create_response.name());

    // Shouldn't have enough versions to paginate (Default max is 100)
    assert_eq!(versions.next_token(), None);

    let versions = versions.versions.unwrap();

    // Should have two versions
    assert_eq!(versions.len(), 3);

    let mut versions = versions.into_iter();

    let first_version = versions.next().unwrap();
    assert_eq!(first_version.version_id(), version_2.version_id());
    assert_eq!(first_version.version_stages(), &["AWSCURRENT".to_string()]);
    assert!(first_version.created_date().unwrap().secs() > 0);
    assert_eq!(first_version.kms_key_ids, None);
    assert_eq!(first_version.last_accessed_date, None);

    // Second version should be the initial created version and should be AWSPREVIOUS
    let second_version = versions.next().unwrap();
    assert_eq!(second_version.version_id(), version_1.version_id());
    assert_eq!(
        second_version.version_stages(),
        &["AWSPREVIOUS".to_string()]
    );
    assert!(second_version.created_date().unwrap().secs() > 0);
    assert_eq!(second_version.kms_key_ids, None);
    assert_eq!(second_version.last_accessed_date, None);

    // Second version should be the initial created version and should be deprecated
    let third_version = versions.next().unwrap();
    assert_eq!(third_version.version_id(), create_response.version_id());
    assert!(third_version.version_stages().is_empty());
    assert!(third_version.created_date().unwrap().secs() > 0);
    assert_eq!(third_version.kms_key_ids, None);
    assert_eq!(third_version.last_accessed_date, None);
}

/// Tests that the default pagination works
#[tokio::test]
async fn test_list_secret_version_ids_test_default_pagination() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let mut created_versions: Vec<String> = Vec::new();
    created_versions.push(create_response.version_id().unwrap().to_string());

    for _i in 0..199 {
        let version = client
            .put_secret_value()
            .secret_id("test")
            .secret_string("test-updated")
            .send()
            .await
            .unwrap();
        created_versions.push(version.version_id().unwrap().to_string());
    }

    created_versions.reverse();

    let versions = client
        .list_secret_version_ids()
        .secret_id("test")
        .include_deprecated(true)
        .send()
        .await
        .unwrap();

    for i in 0..100 {
        let version = versions.versions().get(i).unwrap();
        let created_version = created_versions.get(i).unwrap();

        // Make sure the versions match
        assert_eq!(version.version_id(), Some(created_version.as_str()));
    }

    // Next page should be "100:1" (100 items, page index 1)
    assert_eq!(versions.next_token(), Some("100:1"));

    // Load the next page
    let versions = client
        .list_secret_version_ids()
        .secret_id("test")
        .include_deprecated(true)
        .next_token(versions.next_token().unwrap())
        .send()
        .await
        .unwrap();

    for i in 0..100 {
        let version = versions.versions().get(i).unwrap();
        let created_version = created_versions.get(i + 100).unwrap();

        // Make sure the versions match
        assert_eq!(version.version_id(), Some(created_version.as_str()));
    }

    // Should have nothing more
    assert_eq!(versions.next_token(), None);
}

/// Tests that the pagination works with a custom max results size
#[tokio::test]
async fn test_list_secret_version_ids_test_custom_pagination() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let page_size = 50;
    let pages = 3;
    let items_to_make = (page_size * pages) - 1;

    let mut created_versions: Vec<String> = Vec::new();
    created_versions.push(create_response.version_id().unwrap().to_string());

    for _i in 0..items_to_make {
        let version = client
            .put_secret_value()
            .secret_id("test")
            .secret_string("test-updated")
            .send()
            .await
            .unwrap();
        created_versions.push(version.version_id().unwrap().to_string());
    }

    created_versions.reverse();

    let mut next_token: Option<String> = None;

    for page in 0..pages {
        let versions = client
            .list_secret_version_ids()
            .secret_id("test")
            .include_deprecated(true)
            .max_results(page_size as i32)
            .set_next_token(next_token)
            .send()
            .await
            .unwrap();

        for i in 0..page_size {
            let version = versions.versions().get(i).unwrap();
            let created_version = created_versions.get(i + (page_size * page)).unwrap();

            // Make sure the versions match
            assert_eq!(version.version_id(), Some(created_version.as_str()));
        }

        if page < pages - 1 {
            assert_eq!(
                versions.next_token(),
                Some(format!("{}:{}", page_size, page + 1).as_str())
            );

            next_token = versions.next_token().map(|value| value.to_string());
        } else {
            // Should have nothing more
            assert_eq!(versions.next_token(), None);
            next_token = None;
        }
    }
}
