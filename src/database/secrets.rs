use chrono::{DateTime, Days, Utc};
use serde::Deserialize;
use sqlx::prelude::FromRow;

use crate::database::{DbErr, DbExecutor, DbResult};

#[derive(Clone, FromRow)]
pub struct StoredSecret {
    pub arn: String,
    pub name: String,
    //
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub scheduled_delete_at: Option<DateTime<Utc>>,
    //
    pub version_id: String,
    #[sqlx(json)]
    pub version_stages: Vec<String>,
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
    #[sqlx(json)]
    pub version_stages: Vec<String>,
    //
    pub secret_string: Option<String>,
    pub secret_binary: Option<String>,
    //
    pub created_at: DateTime<Utc>,
    pub last_accessed_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Deserialize)]
pub struct StoredVersionTags {
    pub key: String,
    pub value: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

pub struct CreateSecret {
    pub arn: String,
    pub name: String,
    pub description: Option<String>,
}

/// Create a new "secret" with no versions
pub async fn create_secret(db: impl DbExecutor<'_>, create: CreateSecret) -> DbResult<()> {
    let created_at = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO "secrets" ("arn", "name", "description", "created_at") VALUES (?, ?, ?, ?)
    "#,
    )
    .bind(create.arn)
    .bind(create.name)
    .bind(create.description)
    .bind(created_at)
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
    let updated_at = Utc::now();

    sqlx::query(
        r#"UPDATE "secrets" SET "description" = ?, "updated_at" = ? WHERE "secrets"."arn" = ?"#,
    )
    .bind(description)
    .bind(updated_at)
    .bind(arn)
    .execute(db)
    .await?;

    Ok(())
}

/// Remove a secret
pub async fn delete_secret(db: impl DbExecutor<'_>, secret_arn: &str) -> DbResult<()> {
    sqlx::query(r#"DELETE FROM "secrets" WHERE "arn" = ?"#)
        .bind(secret_arn)
        .execute(db)
        .await?;

    Ok(())
}

/// Get the ARN's of all the secrets that are scheduled for deletion
///
/// Not used by the actual application, only used within tests to ensure
/// a deletion was properly scheduled
pub async fn get_scheduled_secret_deletions(db: impl DbExecutor<'_>) -> DbResult<Vec<(String,)>> {
    sqlx::query_as(r#"SELECT "arn" FROM "secrets" WHERE "scheduled_delete_at" IS NOT NULL"#)
        .fetch_all(db)
        .await
}

/// Delete all secrets that have past their "scheduled_delete_at" date
pub async fn delete_scheduled_secrets(db: impl DbExecutor<'_>) -> DbResult<()> {
    let now = Utc::now();

    sqlx::query(r#"DELETE FROM "secrets" WHERE "scheduled_delete_at" < ?"#)
        .bind(now)
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
    let deleted_at = Utc::now();
    let scheduled_deleted_at = deleted_at
        .checked_add_days(Days::new(days as u64))
        .ok_or_else(|| {
            DbErr::Encode(Box::new(std::io::Error::other(
                "failed to create a future timestamp",
            )))
        })?;

    let (date,): (DateTime<Utc>,) = sqlx::query_as(
        r#"
        UPDATE "secrets"
        SET
            "deleted_at" = ?,
            "scheduled_delete_at" = ?
        WHERE "arn" = ?
        RETURNING "scheduled_delete_at"
        "#,
    )
    .bind(deleted_at)
    .bind(scheduled_deleted_at)
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
        WHERE "arn" = ?
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
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO "secrets_tags" ("secret_arn", "key", "value", "created_at")
        VALUES (?, ?, ?, ?)
        ON CONFLICT("secret_arn", key)
        DO UPDATE SET
            "value" = "excluded"."value",
            "updated_at" = "excluded"."created_at"
        "#,
    )
    .bind(secret_arn)
    .bind(key)
    .bind(value)
    .bind(now)
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

pub struct CreateSecretVersion {
    pub secret_arn: String,
    pub version_id: String,
    //
    pub secret_string: Option<String>,
    pub secret_binary: Option<String>,
}

/// Creates a new version of a secret
pub async fn create_secret_version(
    db: impl DbExecutor<'_>,
    create: CreateSecretVersion,
) -> DbResult<()> {
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO "secrets_versions" ("secret_arn", "version_id", "secret_string", "secret_binary", "created_at")
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(create.secret_arn)
    .bind(create.version_id)
    .bind(create.secret_string)
    .bind(create.secret_binary)
    .bind(now)
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
    let now = Utc::now();

    sqlx::query(
        r#"
        UPDATE "secrets_versions"
        SET "last_accessed_at" = ?
        WHERE "secret_arn" = ? AND "version_id" = ?"#,
    )
    .bind(now)
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
    get_secret_by_version_stage(db, secret_id, "AWSCURRENT").await
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
            "secret_version"."secret_string",
            "secret_version"."secret_binary",
            "secret_version"."created_at" AS "version_created_at",
            "secret_version"."last_accessed_at" AS "version_last_accessed_at",
            COALESCE((
                SELECT json_group_array("version_stage"."value")
                FROM "secret_version_stages" "version_stage"
                WHERE "version_stage"."secret_arn" = "secret_version"."secret_arn"
                    AND "version_stage"."version_id" = "secret_version"."version_id"
            ), '[]') AS "version_stages",
            COALESCE((
                SELECT json_group_array(
                    json_object(
                        'key', "secret_tag"."key",
                        'value', "secret_tag"."value",
                        'created_at', "secret_tag"."created_at",
                        'updated_at', "secret_tag"."updated_at"
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

pub async fn add_secret_version_stage(
    db: impl DbExecutor<'_>,
    secret_arn: &str,
    version_id: &str,
    version_stage: &str,
) -> DbResult<()> {
    let created_at = Utc::now();
    sqlx::query(
        r#"
        INSERT INTO "secret_version_stages" ("secret_arn", "version_id", "value", "created_at")
        VALUES (?, ?, ?, ?)
    "#,
    )
    .bind(secret_arn)
    .bind(version_id)
    .bind(version_stage)
    .bind(created_at)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn remove_secret_version_stage(
    db: impl DbExecutor<'_>,
    secret_arn: &str,
    version_id: &str,
    version_stage: &str,
) -> DbResult<()> {
    sqlx::query(
        r#"
        DELETE FROM "secret_version_stages"
        WHERE "secret_arn" = ? AND "version_id" = ? AND "value" = ?
    "#,
    )
    .bind(secret_arn)
    .bind(version_id)
    .bind(version_stage)
    .execute(db)
    .await?;

    Ok(())
}

/// Remove a version stage label from any version in a secret
pub async fn remove_secret_version_stage_any(
    db: impl DbExecutor<'_>,
    secret_arn: &str,
    version_stage: &str,
) -> DbResult<()> {
    sqlx::query(
        r#"
        DELETE FROM "secret_version_stages"
        WHERE "secret_arn" = ? AND "value" = ?
    "#,
    )
    .bind(secret_arn)
    .bind(version_stage)
    .execute(db)
    .await?;

    Ok(())
}

/// Get a secret where the name OR arn matches the `secret_id` and there is a version
/// in `version_stage`
pub async fn get_secret_by_version_stage(
    db: impl DbExecutor<'_>,
    secret_id: &str,
    version_stage: &str,
) -> DbResult<Option<StoredSecret>> {
    sqlx::query_as(
        r#"
        SELECT
            "secret".*,
            "secret_version"."version_id",
            "secret_version"."secret_string",
            "secret_version"."secret_binary",
            "secret_version"."created_at" AS "version_created_at",
            "secret_version"."last_accessed_at" AS "version_last_accessed_at",
            COALESCE((
                SELECT json_group_array("version_stage"."value")
                FROM "secret_version_stages" "version_stage"
                WHERE "version_stage"."secret_arn" = "secret_version"."secret_arn"
                    AND "version_stage"."version_id" = "secret_version"."version_id"
            ), '[]') AS "version_stages",
            COALESCE((
                SELECT json_group_array(
                    json_object(
                        'key', "secret_tag"."key",
                        'value', "secret_tag"."value",
                        'created_at', "secret_tag"."created_at",
                        'updated_at', "secret_tag"."updated_at"
                    )
                )
                FROM "secrets_tags" "secret_tag"
                WHERE "secret_tag"."secret_arn" = "secret"."arn"
            ), '[]') AS "version_tags"
        FROM "secrets" "secret"
        JOIN "secrets_versions" "secret_version"
            ON "secret_version"."secret_arn" = "secret"."arn"
        JOIN "secret_version_stages" "version_stage"
            ON "version_stage"."secret_arn" = "secret_version"."secret_arn"
            AND "version_stage"."version_id" = "secret_version"."version_id"
            AND "version_stage"."value" = ?
        WHERE "secret"."name" = ? OR "secret"."arn" = ?
        ORDER BY "secret_version"."created_at" DESC
        LIMIT 1;
    "#,
    )
    .bind(version_stage)
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
    version_stage: &str,
) -> DbResult<Option<StoredSecret>> {
    sqlx::query_as(
        r#"
        SELECT
            "secret".*,
            "secret_version"."version_id",
            "secret_version"."secret_string",
            "secret_version"."secret_binary",
            "secret_version"."created_at" AS "version_created_at",
            "secret_version"."last_accessed_at" AS "version_last_accessed_at",
            COALESCE((
                SELECT json_group_array("version_stage"."value")
                FROM "secret_version_stages" "version_stage"
                WHERE "version_stage"."secret_arn" = "secret_version"."secret_arn"
                    AND "version_stage"."version_id" = "secret_version"."version_id"
            ), '[]') AS "version_stages",
            COALESCE((
                SELECT json_group_array(
                    json_object(
                        'key', "secret_tag"."key",
                        'value', "secret_tag"."value",
                        'created_at', "secret_tag"."created_at",
                        'updated_at', "secret_tag"."updated_at"
                    )
                )
                FROM "secrets_tags" "secret_tag"
                WHERE "secret_tag"."secret_arn" = "secret"."arn"
            ), '[]') AS "version_tags"
        FROM "secrets" "secret"
        JOIN ("secrets_versions" "secret_version" ON "secret_version"."secret_arn" = "secret"."arn")
            AND "secret_version"."version_id" = ?
        JOIN "secret_version_stages" "version_stage"
            ON "version_stage"."secret_arn" = "secret_version"."secret_arn"
            AND "version_stage"."version_id" = "secret_version"."version_id"
            AND "version_stage"."value" = ?
        WHERE "secret"."name" = ? OR "secret"."arn" = ?
        LIMIT 1;
    "#,
    )
    .bind(version_id)
    .bind(version_stage)
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
        SELECT
            "secret_version".*,
            COALESCE((
                SELECT json_group_array("version_stage"."value")
                FROM "secret_version_stages" "version_stage"
                WHERE "version_stage"."secret_arn" = "secret_version"."secret_arn"
                    AND "version_stage"."version_id" = "secret_version"."version_id"
            ), '[]') AS "version_stages"
        FROM "secrets_versions" "secret_version"
        WHERE "secret_version"."secret_arn" = ?
        ORDER BY "secret_version"."created_at" DESC
    "#,
    )
    .bind(secret_arn)
    .fetch_all(db)
    .await
}

/// Takes any secrets with over 100 versions and deletes any secrets that
/// are over 24h old until there is only 100 versions for each secret
pub async fn delete_excess_secret_versions(db: impl DbExecutor<'_>) -> DbResult<()> {
    let now = Utc::now();
    let cutoff = now.checked_sub_days(Days::new(1)).ok_or_else(|| {
        DbErr::Encode(Box::new(std::io::Error::other(
            "failed to create a future timestamp",
        )))
    })?;

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
              AND "created_at" < ?
        );
        "#,
    )
    .bind(cutoff)
    .execute(db)
    .await?;

    Ok(())
}
