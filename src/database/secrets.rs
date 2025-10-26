use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

use crate::database::{DbExecutor, DbResult};

#[derive(Clone, FromRow)]
pub struct StoredSecret {
    pub arn: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub delete_at: Option<DateTime<Utc>>,
    pub scheduled_delete_at: Option<DateTime<Utc>>,
    //
    pub version_id: String,
    #[sqlx(try_from = "String")]
    pub version_stage: VersionStage,
    //
    pub description: Option<String>,
    pub secret_string: Option<String>,
    pub secret_binary: Option<String>,
    //
    pub version_created_at: DateTime<Utc>,
    pub version_last_accessed_at: Option<DateTime<Utc>>,
    //
    #[sqlx(json)]
    pub version_tags: Vec<StoredVersionTags>,
}

#[derive(Clone, FromRow)]
pub struct SecretVersion {
    pub secret_arn: String,
    //
    pub version_id: String,
    #[sqlx(try_from = "String")]
    pub version_stage: VersionStage,
    //
    pub secret_string: Option<String>,
    pub secret_binary: Option<String>,
    //
    pub created_at: DateTime<Utc>,
    pub last_accessed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, strum::EnumString, strum::Display, Deserialize, Serialize)]
pub enum VersionStage {
    #[serde(rename = "AWSCURRENT")]
    #[strum(serialize = "AWSCURRENT")]
    Current,
    #[serde(rename = "AWSPREVIOUS")]
    #[strum(serialize = "AWSPREVIOUS")]
    Previous,
}

impl TryFrom<String> for VersionStage {
    type Error = strum::ParseError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        VersionStage::from_str(&value)
    }
}

#[derive(Clone, Deserialize)]
pub struct StoredVersionTags {
    pub key: String,
    pub value: String,
    pub created_at: DateTime<Utc>,
}

pub struct CreateSecret {
    pub arn: String,
    pub name: String,
    pub description: Option<String>,
}

/// Create a new "secret" with no versions
pub async fn create_secret(db: impl DbExecutor<'_>, create: CreateSecret) -> DbResult<()> {
    sqlx::query(
        r#"
        INSERT INTO "secrets" ("arn", "name", "description", "created_at") VALUES (?, ?, ?, datetime('now'))
    "#,
    )
    .bind(create.arn)
    .bind(create.name)
    .bind(create.description)
    .execute(db)
    .await?;

    Ok(())
}

/// Updates the description of a secret
pub async fn update_secret_description(
    db: impl DbExecutor<'_>,
    arn: &str,
    description: &str,
) -> DbResult<()> {
    sqlx::query(r#"UPDATE "secrets" SET "description" = ?, "updated_at" = datetime('now') WHERE "secrets"."secret_arn" = ?"#)
        .bind(arn)
        .bind(description)
        .execute(db)
        .await?;

    Ok(())
}

/// Remove a secret
pub async fn delete_secret(db: impl DbExecutor<'_>, secret_arn: &str) -> DbResult<()> {
    sqlx::query(r#"DELETE FROM "secrets" WHERE "secret_arn" = ?"#)
        .bind(secret_arn)
        .execute(db)
        .await?;

    Ok(())
}

/// Delete all secrets that have past their "scheduled_delete_at" date
pub async fn delete_scheduled_secrets(db: impl DbExecutor<'_>) -> DbResult<()> {
    sqlx::query(r#"DELETE FROM "secrets" WHERE "scheduled_delete_at" < datetime('now')"#)
        .execute(db)
        .await?;

    Ok(())
}

/// Mark a secret for deletion, sets the scheduled deletion date for `days` days
/// into the future
pub async fn schedule_delete_secret(
    db: impl DbExecutor<'_>,
    secret_arn: &str,
    days: i32,
) -> DbResult<DateTime<Utc>> {
    let (date,): (DateTime<Utc>,) = sqlx::query_as(
        r#"
        UPDATE "secrets"
        SET
            "deleted_at" = datetime('now'),
            "scheduled_delete_at" = datetime('now', '+' || ? || ' days')
        WHERE "secret_arn" = ?
        RETURNING "scheduled_delete_at"
        "#,
    )
    .bind(days)
    .bind(secret_arn)
    .fetch_one(db)
    .await?;

    Ok(date)
}

/// Cancel a secrets deletion
pub async fn cancel_delete_secret(db: impl DbExecutor<'_>, secret_arn: &str) -> DbResult<()> {
    sqlx::query(
        r#"
        UPDATE "secrets"
        SET
            "deleted_at" = NULL,
            "scheduled_delete_at" = NULL
        WHERE "secret_arn" = ?
        RETURNING "scheduled_delete_at"
        "#,
    )
    .bind(secret_arn)
    .execute(db)
    .await?;

    Ok(())
}

/// Set a tag on a secret
pub async fn put_secret_tag(
    db: impl DbExecutor<'_>,
    secret_arn: &str,
    key: &str,
    value: &str,
) -> DbResult<()> {
    sqlx::query(
        r#"
        INSERT INTO "secrets_tags" ("secret_arn", "key", "value", "created_at")
        VALUES (?, ?, ?, datetime('now'))
        ON CONFLICT("secret_arn", key)
        DO UPDATE SET
            "value" = "excluded"."value",
            "updated_at" = datetime('now')
        "#,
    )
    .bind(secret_arn)
    .bind(key)
    .bind(value)
    .execute(db)
    .await?;

    Ok(())
}

/// Remove a tag from a secret
pub async fn remove_secret_tag(
    db: impl DbExecutor<'_>,
    secret_arn: &str,
    key: &str,
) -> DbResult<()> {
    sqlx::query(r#"DELETE FROM "secrets_tags" WHERE "secret_arn" = ? AND "key" = ?"#)
        .bind(secret_arn)
        .bind(key)
        .execute(db)
        .await?;

    Ok(())
}

/// Updates all versions of a secret by ARN ensuring they are marked as the [VersionStage::Previous]
/// version in preparation for a new secret being inserted
pub async fn mark_secret_versions_previous(db: impl DbExecutor<'_>, arn: &str) -> DbResult<()> {
    sqlx::query(
        r#"
        UPDATE "secrets_versions"
        SET "version_stage" = ?
        WHERE "secret_arn" = ?
    "#,
    )
    .bind(VersionStage::Previous.to_string())
    .bind(arn)
    .execute(db)
    .await?;

    Ok(())
}

pub struct CreateSecretVersion {
    pub secret_arn: String,
    pub version_id: String,
    pub version_stage: VersionStage,
    //
    pub secret_string: Option<String>,
    pub secret_binary: Option<String>,
}

/// Creates a new version of a secret
pub async fn create_secret_version(
    db: impl DbExecutor<'_>,
    create: CreateSecretVersion,
) -> DbResult<()> {
    sqlx::query(
        r#"
        INSERT INTO "secrets_versions" ("secret_arn", "version_id", "version_stage", "secret_string", "secret_binary", "created_at")
        VALUES (?, ?, ?, ?, ?, datetime('now'))
        "#,
    )
    .bind(create.secret_arn)
    .bind(create.version_id)
    .bind(create.version_stage.to_string())
    .bind(create.secret_string)
    .bind(create.secret_binary)
    .execute(db)
    .await?;

    Ok(())
}

/// Updates the last access date of a secret version
pub async fn update_secret_version_last_accessed(
    db: impl DbExecutor<'_>,
    secret_arn: &str,
    version_id: &str,
) -> DbResult<()> {
    sqlx::query(
        r#"
        UPDATE "secrets_versions"
        SET "last_accessed_at" = datetime('now')
        WHERE "secret_arn" = ? AND "version_id" = ?"#,
    )
    .bind(secret_arn)
    .bind(version_id)
    .execute(db)
    .await?;

    Ok(())
}

/// Get the current version of a secret where the name OR arn matches the `secret_id`
pub async fn get_secret_latest_version(
    db: impl DbExecutor<'_>,
    secret_id: &str,
) -> DbResult<Option<StoredSecret>> {
    get_secret_by_version_stage(db, secret_id, VersionStage::Current).await
}

/// Get a secret where the name OR arn matches the `secret_id` and there is a version
/// with the version ID of `version_id`
pub async fn get_secret_by_version_id(
    db: impl DbExecutor<'_>,
    secret_id: &str,
    version_id: &str,
) -> DbResult<Option<StoredSecret>> {
    sqlx::query_as(
        r#"
        SELECT
            "secret".*,
            "secret_version"."version_id",
            "secret_version"."version_stage",
            "secret_version"."secret_string",
            "secret_version"."secret_binary",
            "secret_version"."created_at" AS "version_created_at",
            "secret_version"."last_accessed_at" AS "version_last_accessed_at",
            COALESCE((
                SELECT json_group_array(
                    json_object(
                        'key', "secret_tag"."key",
                        'value', "secret_tag"."value",
                        'created_at', strftime('%Y-%m-%dT%H:%M:%SZ', "secret_tag"."created_at")
                    )
                )
                FROM "secrets_tags" "secret_tag"
                WHERE "secret_tag"."secret_arn" = "secret"."arn"
            ), '[]') AS "version_tags"
        FROM "secrets" "secret"
        JOIN "secrets_versions" "secret_version"
            ON "secret_version"."secret_arn" = "secret"."arn"
            AND "secret_version"."version_id" = ?
        WHERE "secret"."name" = ? OR "secret"."arn" = ?
        LIMIT 1;
    "#,
    )
    .bind(version_id)
    .bind(secret_id)
    .bind(secret_id)
    .fetch_optional(db)
    .await
}

/// Get a secret where the name OR arn matches the `secret_id` and there is a version
/// in `version_stage`
pub async fn get_secret_by_version_stage(
    db: impl DbExecutor<'_>,
    secret_id: &str,
    version_stage: VersionStage,
) -> DbResult<Option<StoredSecret>> {
    sqlx::query_as(
        r#"
        SELECT
            "secret".*,
            "secret_version"."version_id",
            "secret_version"."version_stage",
            "secret_version"."secret_string",
            "secret_version"."secret_binary",
            "secret_version"."created_at" AS "version_created_at",
            "secret_version"."last_accessed_at" AS "version_last_accessed_at",
            COALESCE((
                SELECT json_group_array(
                    json_object(
                        'key', "secret_tag"."key",
                        'value', "secret_tag"."value",
                        'created_at', strftime('%Y-%m-%dT%H:%M:%SZ', "secret_tag"."created_at")
                    )
                )
                FROM "secrets_tags" "secret_tag"
                WHERE "secret_tag"."secret_arn" = "secret"."arn"
            ), '[]') AS "version_tags"
        FROM "secrets" "secret"
        JOIN "secrets_versions" "secret_version"
            ON "secret_version"."secret_arn" = "secret"."arn"
            AND "secret_version"."version_stage" = ?
        WHERE "secret"."name" = ? OR "secret"."arn" = ?
        LIMIT 1;
    "#,
    )
    .bind(version_stage.to_string())
    .bind(secret_id)
    .bind(secret_id)
    .fetch_optional(db)
    .await
}

/// Get a secret where the name OR arn matches the `secret_id` and there is a version
/// in `version_stage` with the version ID `version_id`
pub async fn get_secret_by_version_stage_and_id(
    db: impl DbExecutor<'_>,
    secret_id: &str,
    version_id: &str,
    version_stage: VersionStage,
) -> DbResult<Option<StoredSecret>> {
    sqlx::query_as(
        r#"
        SELECT
            "secret".*,
            "secret_version"."version_id",
            "secret_version"."version_stage",
            "secret_version"."secret_string",
            "secret_version"."secret_binary",
            "secret_version"."created_at" AS "version_created_at",
            "secret_version"."last_accessed_at" AS "version_last_accessed_at",
            COALESCE((
                SELECT json_group_array(
                    json_object(
                        'key', "secret_tag"."key",
                        'value', "secret_tag"."value",
                        'created_at', strftime('%Y-%m-%dT%H:%M:%SZ', "secret_tag"."created_at")
                    )
                )
                FROM "secrets_tags" "secret_tag"
                WHERE "secret_tag"."secret_arn" = "secret"."arn"
            ), '[]') AS "version_tags"
        FROM "secrets" "secret"
        JOIN ("secrets_versions" "secret_version" ON "secret_version"."secret_arn" = "secret"."arn")
            AND "secret_version"."version_id" = ?
            AND "secret_version"."version_stage" = ?
        WHERE "secret"."name" = ? OR "secret"."arn" = ?
        LIMIT 1;
    "#,
    )
    .bind(version_id)
    .bind(version_stage.to_string())
    .bind(secret_id)
    .bind(secret_id)
    .fetch_optional(db)
    .await
}

/// Get all versions of a secret
pub async fn get_secret_versions(
    db: impl DbExecutor<'_>,
    secret_arn: &str,
) -> DbResult<Vec<SecretVersion>> {
    sqlx::query_as(
        r#"
        SELECT "secret_version".*
        FROM ""secrets_versions" "secret_version"
        WHERE "secret_version"."secret_arn" = "secret"."arn"
    "#,
    )
    .bind(secret_arn)
    .fetch_all(db)
    .await
}

/// Takes any secrets with over 100 versions and deletes any secrets that
/// are over 24h old until there is only 100 versions for each secret
pub async fn delete_excess_secret_versions(db: impl DbExecutor<'_>) -> DbResult<()> {
    sqlx::query(
        r#"
        WITH "ranked_versions" AS (
            SELECT
                "secret_version".*,
                ROW_NUMBER() OVER (
                    PARTITION BY "secret_version"."secret_arn"
                    ORDER BY "secret_version"."created_at" DESC
                ) AS "row_number"
            FROM "secret_versions" "secret_version"
        )
        DELETE FROM "secret_versions" "secret_version"
        WHERE ("secret_arn", "version_id") IN (
            SELECT "secret_arn", "version_id"
            FROM "ranked_versions"
            WHERE "row_number" > 100
              AND datetime("created_at", '+1 day') < datetime('now')
        );
        "#,
    )
    .execute(db)
    .await?;

    Ok(())
}
