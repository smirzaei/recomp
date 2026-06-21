use std::path::{Path, PathBuf};

use sqlx::{
    Connection as _,
    migrate::{MigrateError as SqlxMigrateError, Migrator},
    postgres::PgConnection,
};
use thiserror::Error;

use crate::postgres::Config;

/// Error returned while running `SQLx` migrations.
#[derive(Debug, Error)]
pub enum MigrateError {
    /// The migration directory could not be resolved.
    #[error("Postgres migration directory resolution failed at {path}")]
    Resolve {
        path: PathBuf,
        #[source]
        source: SqlxMigrateError,
    },
    /// The migration connection could not be established.
    #[error("Postgres connection failed")]
    Connect(#[source] sqlx::Error),
    /// Migration execution failed.
    #[error("Postgres migration failed")]
    Migrate(#[source] SqlxMigrateError),
    /// Closing the migration connection failed.
    #[error("Postgres connection close failed")]
    Close(#[source] sqlx::Error),
    /// Migration execution and connection close both failed.
    #[error("Postgres migration failed; connection close also failed: {close}")]
    MigrateAndClose {
        #[source]
        migrate: SqlxMigrateError,
        close: sqlx::Error,
    },
}

/// Resolves and runs pending `SQLx` migrations from `directory`.
pub async fn run(directory: &Path, config: &Config) -> Result<(), MigrateError> {
    let migrator = Migrator::new(directory)
        .await
        .map_err(|source| MigrateError::Resolve {
            path: directory.to_path_buf(),
            source,
        })?;

    run_with_migrator(&migrator, config).await
}

/// Runs pending `SQLx` migrations using a caller-provided migrator.
pub async fn run_with_migrator(migrator: &Migrator, config: &Config) -> Result<(), MigrateError> {
    let options = config.into();
    let mut connection = PgConnection::connect_with(&options)
        .await
        .map_err(MigrateError::Connect)?;

    let migration_result = migrator.run(&mut connection).await;
    let close_result = connection.close().await;

    match (migration_result, close_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Ok(()), Err(error)) => Err(MigrateError::Close(error)),
        (Err(error), Ok(())) => Err(MigrateError::Migrate(error)),
        (Err(migrate), Err(close)) => Err(MigrateError::MigrateAndClose { migrate, close }),
    }
}
