use std::path::Path;

use sqlx::{Sqlite, SqlitePool, Transaction, sqlite::SqlitePoolOptions};

pub use sqlx::SqliteExecutor as DbExecutor;
use thiserror::Error;
use tokio::fs::File;

use crate::database::migrations::{apply_migrations, setup_migrations};

pub mod migrations;
pub mod secrets;

/// Type of the database connection pool
pub type DbPool = SqlitePool;

/// Short type alias for a database error
pub type DbErr = sqlx::Error;

/// Type alias for a result where the error is a [DbErr]
pub type DbResult<T> = Result<T, DbErr>;

/// Type of a database transaction
pub type DbTransaction<'c> = Transaction<'c, Sqlite>;

#[derive(Debug, Error)]
pub enum CreateDatabaseError {
    #[error("failed to create database file")]
    CreateFile(std::io::Error),

    #[error(transparent)]
    Db(#[from] DbErr),
}

pub async fn create_database(key: String, raw_path: String) -> Result<DbPool, CreateDatabaseError> {
    let path = Path::new(&raw_path);
    if !path.exists() {
        let _file = File::create(&path)
            .await
            .map_err(CreateDatabaseError::CreateFile)?;
    }

    let pool = SqlitePoolOptions::new()
        .after_connect(move |connection, _metadata| {
            let key = key.clone();
            // Ensure connection is provided the database key
            Box::pin(async move {
                sqlx::query(&format!("PRAGMA key = '{key}';"))
                    .execute(connection)
                    .await?;

                Ok(())
            })
        })
        .connect(&format!("sqlite:{raw_path}"))
        .await?;

    initialize_database(&pool).await?;

    Ok(pool)
}

pub async fn initialize_database(db: &DbPool) -> DbResult<()> {
    let mut t = db.begin().await?;

    setup_migrations(&mut t).await?;
    apply_migrations(&mut t).await?;

    t.commit().await?;

    Ok(())
}
