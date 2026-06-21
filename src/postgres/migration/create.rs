use std::{
    fs::{self, OpenOptions},
    io::{Error as IoError, ErrorKind, Write as _},
    num::TryFromIntError,
    path::{Path, PathBuf},
    time::{SystemTime, SystemTimeError, UNIX_EPOCH},
};

use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MigrationType {
    Up,
    Down,
}

impl MigrationType {
    const fn suffix(self) -> &'static str {
        match self {
            Self::Up => ".up.sql",
            Self::Down => ".down.sql",
        }
    }

    const fn file_content(self) -> &'static str {
        match self {
            Self::Up => "-- Add up migration script here\n",
            Self::Down => "-- Add down migration script here\n",
        }
    }
}

/// Files created for a reversible `SQLx` migration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatedMigration {
    up_path: PathBuf,
    down_path: PathBuf,
    version: i64,
}

impl CreatedMigration {
    /// Returns the timestamp version used in the migration filenames.
    #[must_use]
    pub const fn version(&self) -> i64 {
        self.version
    }

    /// Returns the created up migration path.
    #[must_use]
    pub fn up_path(&self) -> &Path {
        &self.up_path
    }

    /// Returns the created down migration path.
    #[must_use]
    pub fn down_path(&self) -> &Path {
        &self.down_path
    }
}

/// Error returned while creating migration files.
#[derive(Debug, Error)]
pub enum CreateError {
    /// The supplied description was empty after normalization.
    #[error("migration description is empty")]
    EmptyDescription,
    /// The supplied description contained a character that is not valid in the generated filename.
    #[error("migration description contains invalid character {character:?}")]
    InvalidDescriptionCharacter { character: char },
    /// The system clock was before the Unix epoch.
    #[error("system clock is before Unix epoch")]
    SystemTime(#[source] SystemTimeError),
    /// The Unix timestamp does not fit `SQLx`'s signed migration version.
    #[error("migration version {seconds} does not fit into SQLx's i64 migration version")]
    VersionOverflow {
        seconds: u64,
        source: TryFromIntError,
    },
    /// The target migration directory could not be created.
    #[error("migration directory creation failed at {path}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: IoError,
    },
    /// A target migration file already exists.
    #[error("migration file already exists at {path}")]
    FileExists { path: PathBuf },
    /// A target migration file could not be created.
    #[error("migration file creation failed at {path}")]
    CreateFile {
        path: PathBuf,
        #[source]
        source: IoError,
    },
    /// A target migration file could not be written.
    #[error("migration file write failed at {path}")]
    WriteFile {
        path: PathBuf,
        #[source]
        source: IoError,
    },
}

/// Creates a reversible `SQLx` migration file pair in `directory`.
pub fn create(directory: &Path, description: &str) -> Result<CreatedMigration, CreateError> {
    let version = current_version()?;
    let description = normalize_description(description)?;
    let up_path = migration_file_path(directory, version, description.as_str(), MigrationType::Up);
    let down_path = migration_file_path(
        directory,
        version,
        description.as_str(),
        MigrationType::Down,
    );

    fs::create_dir_all(directory).map_err(|source| CreateError::CreateDirectory {
        path: directory.to_path_buf(),
        source,
    })?;

    write_migration_file(&up_path, MigrationType::Up.file_content())?;
    write_migration_file(&down_path, MigrationType::Down.file_content())?;

    Ok(CreatedMigration {
        up_path,
        down_path,
        version,
    })
}

fn current_version() -> Result<i64, CreateError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(CreateError::SystemTime)?;

    let seconds = duration.as_secs();
    i64::try_from(seconds).map_err(|source| CreateError::VersionOverflow { seconds, source })
}

fn normalize_description(description: &str) -> Result<String, CreateError> {
    let description = description.trim();
    if description.is_empty() {
        return Err(CreateError::EmptyDescription);
    }

    let mut normalized = String::with_capacity(description.len());
    let mut previous_separator = false;
    for character in description.chars() {
        if character.is_ascii_alphanumeric() {
            normalized.push(character.to_ascii_lowercase());
            previous_separator = false;
            continue;
        }

        if character == '_' || character == '-' || character.is_ascii_whitespace() {
            if !previous_separator && !normalized.is_empty() {
                normalized.push('_');
                previous_separator = true;
            }
            continue;
        }

        return Err(CreateError::InvalidDescriptionCharacter { character });
    }

    if previous_separator {
        normalized.truncate(normalized.trim_end_matches('_').len());
    }

    if normalized.is_empty() {
        return Err(CreateError::EmptyDescription);
    }

    Ok(normalized)
}

fn migration_file_path(
    directory: &Path,
    version: i64,
    description: &str,
    migration_type: MigrationType,
) -> PathBuf {
    directory.join(format!(
        "{version}_{description}{}",
        migration_type.suffix()
    ))
}

fn write_migration_file(path: &Path, content: &str) -> Result<(), CreateError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| create_file_error(path, source))?;

    file.write_all(content.as_bytes())
        .map_err(|source| CreateError::WriteFile {
            path: path.to_path_buf(),
            source,
        })
}

fn create_file_error(path: &Path, source: IoError) -> CreateError {
    if source.kind() == ErrorKind::AlreadyExists {
        return CreateError::FileExists {
            path: path.to_path_buf(),
        };
    }

    CreateError::CreateFile {
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use std::{env, process};

    use super::*;

    #[test]
    fn normalize_description_accepts_common_name_shapes() {
        let normalized = match normalize_description("  Create users-table__v2  ") {
            Ok(normalized) => normalized,
            Err(error) => panic!("common migration name must normalize: {error}"),
        };

        assert_eq!(
            normalized, "create_users_table_v2",
            "normalization must produce SQLx-safe migration filenames"
        );
    }

    #[test]
    fn normalize_description_rejects_empty_name() {
        let error = match normalize_description(" - _ ") {
            Ok(normalized) => panic!("empty name must be rejected, got: {normalized}"),
            Err(error) => error,
        };

        assert!(
            matches!(error, CreateError::EmptyDescription),
            "empty normalized names must return EmptyDescription"
        );
    }

    #[test]
    fn normalize_description_rejects_path_separators() {
        let error = match normalize_description("foo/bar") {
            Ok(normalized) => panic!("path separator must be rejected, got: {normalized}"),
            Err(error) => error,
        };

        assert!(
            matches!(
                error,
                CreateError::InvalidDescriptionCharacter { character: '/' }
            ),
            "path separators must not be accepted in generated filenames"
        );
    }

    #[test]
    fn migration_file_path_uses_reversible_suffixes() {
        let directory = Path::new("db/migrations/core");

        assert_eq!(
            migration_file_path(directory, 42, "create_users", MigrationType::Up),
            directory.join("42_create_users.up.sql"),
            "up migration path must use the SQLx reversible suffix"
        );
        assert_eq!(
            migration_file_path(directory, 42, "create_users", MigrationType::Down),
            directory.join("42_create_users.down.sql"),
            "down migration path must use the SQLx reversible suffix"
        );
    }

    #[test]
    fn create_creates_reversible_migration_files() {
        let directory = test_directory("create_creates_reversible_migration_files");

        let created = match create(&directory, "Create Users") {
            Ok(created) => created,
            Err(error) => panic!("migration files must be created: {error}"),
        };

        assert!(
            created.version() > 0,
            "created migration version must be a positive Unix timestamp"
        );
        assert_eq!(
            created.up_path().parent(),
            Some(directory.as_path()),
            "up migration must be created in the requested directory"
        );
        assert_eq!(
            created.down_path().parent(),
            Some(directory.as_path()),
            "down migration must be created in the requested directory"
        );
        assert_eq!(
            read_file(created.up_path()),
            MigrationType::Up.file_content(),
            "up migration must contain the SQLx up template"
        );
        assert_eq!(
            read_file(created.down_path()),
            MigrationType::Down.file_content(),
            "down migration must contain the SQLx down template"
        );

        remove_test_directory(&directory);
    }

    #[test]
    fn write_migration_file_rejects_existing_file() {
        let directory = test_directory("write_migration_file_rejects_existing_file");
        create_test_directory(&directory);
        let path = directory.join("42_create_users.up.sql");
        match write_migration_file(&path, MigrationType::Up.file_content()) {
            Ok(()) => {}
            Err(error) => panic!("initial migration file must be created: {error}"),
        }

        let error = match write_migration_file(&path, MigrationType::Up.file_content()) {
            Ok(()) => panic!("duplicate migration file must be rejected"),
            Err(error) => error,
        };

        assert!(
            matches!(error, CreateError::FileExists { .. }),
            "duplicate migration creation must not overwrite existing files"
        );

        remove_test_directory(&directory);
    }

    fn test_directory(name: &str) -> PathBuf {
        let directory =
            env::temp_dir().join(format!("recomp-migration-create-{name}-{}", process::id()));
        remove_test_directory(&directory);

        directory
    }

    fn create_test_directory(directory: &Path) {
        match fs::create_dir_all(directory) {
            Ok(()) => {}
            Err(error) => panic!("test migration directory must be created: {error}"),
        }
    }

    fn read_file(path: &Path) -> String {
        match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) => panic!("migration file must be readable: {error}"),
        }
    }

    fn remove_test_directory(directory: &Path) {
        match fs::remove_dir_all(directory) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => panic!("test directory cleanup failed: {error}"),
        }
    }
}
