use std::{fmt, num::NonZeroU32, time::Duration};

use thiserror::Error;

/// Default time to wait while acquiring a pool connection.
pub const DEFAULT_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);
/// Default time an idle pool connection may remain open.
pub const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_mins(5);
/// Default maximum age for a pool connection.
pub const DEFAULT_MAX_LIFETIME: Duration = Duration::from_mins(30);
/// Default maximum number of pool connections.
pub const DEFAULT_MAX_CONNECTIONS: NonZeroU32 = match NonZeroU32::new(10) {
    Some(value) => value,
    None => unreachable!(),
};
/// Default target minimum number of pool connections.
pub const DEFAULT_MIN_CONNECTIONS: u32 = 0;

/// Error returned when building Postgres configuration.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ConfigBuilderError {
    #[error(
        "Postgres min connections ({min_connections}) cannot exceed max connections ({max_connections})"
    )]
    MinConnectionsExceedsMaxConnections {
        min_connections: u32,
        max_connections: NonZeroU32,
    },
}

/// Postgres password wrapper that redacts its value from `Debug` output.
///
/// An empty password is valid and different from an omitted password. Use
/// [`ConfigBuilder::with_password`] with `""` when the empty string is the
/// intended authentication value.
#[derive(Clone, Eq, PartialEq)]
pub struct Password(String);

impl Password {
    #[must_use]
    pub fn new(password: impl Into<String>) -> Self {
        Self(password.into())
    }

    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

/// Builder for Postgres connection overrides and pool settings.
///
/// Connection fields are explicit overrides. If a connection field is omitted,
/// [`Config`] keeps it absent so the later `sqlx` mapping can leave `sqlx` and
/// Postgres defaults in control.
///
/// | Connection field | If omitted |
/// | --- | --- |
/// | Host | `sqlx` checks `PGHOSTADDR`, then `PGHOST`, then local socket directories, then `localhost`. |
/// | Port | `sqlx` checks `PGPORT`, then uses `5432`. |
/// | Username | `sqlx` checks `PGUSER`, then the operating system username, then `unknown`. |
/// | Password | `sqlx` may use `PGPASSWORD`, `PGPASSFILE`, or the default `.pgpass` file; password authentication falls back to an empty string. |
/// | Database | The Postgres startup protocol defaults to the username. |
/// | Application name | `sqlx` checks `PGAPPNAME` when using its defaults; otherwise no application name is sent. |
/// | Statement cache capacity | `sqlx` uses its own default, currently `100`. |
///
/// Pool fields have concrete defaults so a pool has bounded behavior even when
/// only connection defaults are used.
///
/// | Pool field | Meaning | Default |
/// | --- | --- | --- |
/// | Acquire timeout | Maximum time to wait for a pool connection before acquire fails. | [`DEFAULT_ACQUIRE_TIMEOUT`] |
/// | Idle timeout | Maximum time an idle connection may stay in the pool before it is closed. | [`DEFAULT_IDLE_TIMEOUT`] |
/// | Max lifetime | Maximum age for a connection before it is closed and replaced. | [`DEFAULT_MAX_LIFETIME`] |
/// | Max connections | Maximum number of open pool connections. | [`DEFAULT_MAX_CONNECTIONS`] |
/// | Min connections | Target minimum number of open pool connections. | [`DEFAULT_MIN_CONNECTIONS`] |
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConfigBuilder {
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<Password>,
    database: Option<String>,
    application_name: Option<String>,
    acquire_timeout: Option<Duration>,
    idle_timeout: Option<Duration>,
    max_lifetime: Option<Duration>,
    max_connections: Option<NonZeroU32>,
    min_connections: Option<u32>,
    statement_cache_capacity: Option<usize>,
}

impl ConfigBuilder {
    /// Creates a builder with no connection overrides and default pool values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Overrides the Postgres host.
    ///
    /// If omitted, `sqlx` checks `PGHOSTADDR`, then `PGHOST`, then local socket
    /// directories, then `localhost`.
    #[must_use]
    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    /// Overrides the Postgres port.
    ///
    /// If omitted, `sqlx` uses `PGPORT` or `5432`.
    #[must_use]
    pub const fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Overrides the Postgres username.
    ///
    /// If omitted, `sqlx` uses `PGUSER`, then the operating system username,
    /// then `unknown`.
    #[must_use]
    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Overrides the Postgres password.
    ///
    /// Passing `""` records an explicit empty password. Omitting this field is
    /// different: the `sqlx` mapping can still use `PGPASSWORD`, `PGPASSFILE`,
    /// or the default `.pgpass` file.
    #[must_use]
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(Password::new(password));
        self
    }

    /// Overrides the Postgres database name.
    ///
    /// If omitted, the Postgres startup protocol defaults the database to the
    /// username.
    #[must_use]
    pub fn with_database(mut self, database: impl Into<String>) -> Self {
        self.database = Some(database.into());
        self
    }

    /// Overrides the Postgres application name startup parameter.
    ///
    /// If omitted, this config records no application name override.
    #[must_use]
    pub fn with_application_name(mut self, application_name: impl Into<String>) -> Self {
        self.application_name = Some(application_name.into());
        self
    }

    /// Overrides the pool acquire timeout.
    ///
    /// Defaults to [`DEFAULT_ACQUIRE_TIMEOUT`].
    #[must_use]
    pub const fn with_acquire_timeout(mut self, acquire_timeout: Duration) -> Self {
        self.acquire_timeout = Some(acquire_timeout);
        self
    }

    /// Overrides the pool idle timeout.
    ///
    /// Defaults to [`DEFAULT_IDLE_TIMEOUT`].
    #[must_use]
    pub const fn with_idle_timeout(mut self, idle_timeout: Duration) -> Self {
        self.idle_timeout = Some(idle_timeout);
        self
    }

    /// Overrides the pool connection maximum lifetime.
    ///
    /// Defaults to [`DEFAULT_MAX_LIFETIME`].
    #[must_use]
    pub const fn with_max_lifetime(mut self, max_lifetime: Duration) -> Self {
        self.max_lifetime = Some(max_lifetime);
        self
    }

    /// Overrides the pool maximum connection count.
    ///
    /// Defaults to [`DEFAULT_MAX_CONNECTIONS`]. The value is non-zero so the
    /// pool cannot be configured with no capacity.
    #[must_use]
    pub const fn with_max_connections(mut self, max_connections: NonZeroU32) -> Self {
        self.max_connections = Some(max_connections);
        self
    }

    /// Overrides the pool target minimum connection count.
    ///
    /// Defaults to [`DEFAULT_MIN_CONNECTIONS`]. Building fails if this exceeds
    /// the configured maximum connection count.
    #[must_use]
    pub const fn with_min_connections(mut self, min_connections: u32) -> Self {
        self.min_connections = Some(min_connections);
        self
    }

    /// Overrides the per-connection prepared statement cache capacity.
    ///
    /// If omitted, `sqlx` uses its own default, currently `100`.
    #[must_use]
    pub const fn with_statement_cache_capacity(mut self, capacity: usize) -> Self {
        self.statement_cache_capacity = Some(capacity);
        self
    }

    /// Builds a config and validates pool size relationships.
    pub fn build(self) -> Result<Config, ConfigBuilderError> {
        let max_connections = self.max_connections.unwrap_or(DEFAULT_MAX_CONNECTIONS);
        let min_connections = self.min_connections.unwrap_or(DEFAULT_MIN_CONNECTIONS);

        if min_connections > max_connections.get() {
            return Err(ConfigBuilderError::MinConnectionsExceedsMaxConnections {
                min_connections,
                max_connections,
            });
        }

        Ok(Config {
            host: self.host,
            username: self.username,
            password: self.password,
            database: self.database,
            application_name: self.application_name,
            port: self.port,
            statement_cache_capacity: self.statement_cache_capacity,
            acquire_timeout: self.acquire_timeout.unwrap_or(DEFAULT_ACQUIRE_TIMEOUT),
            idle_timeout: self.idle_timeout.unwrap_or(DEFAULT_IDLE_TIMEOUT),
            max_lifetime: self.max_lifetime.unwrap_or(DEFAULT_MAX_LIFETIME),
            max_connections,
            min_connections,
        })
    }
}

/// Postgres connection overrides and pool configuration.
///
/// Connection values are optional because `sqlx` and Postgres already define
/// useful defaults. Pool values are concrete because this crate chooses bounded
/// pool behavior by default.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<Password>,
    database: Option<String>,
    application_name: Option<String>,
    statement_cache_capacity: Option<usize>,
    acquire_timeout: Duration,
    idle_timeout: Duration,
    max_lifetime: Duration,
    max_connections: NonZeroU32,
    min_connections: u32,
}

impl Config {
    /// Creates a builder with no connection overrides and default pool values.
    #[must_use]
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }

    /// Returns the explicit host override, if one was configured.
    ///
    /// `None` means the `sqlx` mapping should leave the host unset so `sqlx`
    /// can check `PGHOSTADDR`, then `PGHOST`, then local socket directories,
    /// then `localhost`.
    #[must_use]
    pub const fn host(&self) -> Option<&str> {
        match &self.host {
            Some(host) => Some(host.as_str()),
            None => None,
        }
    }

    /// Returns the explicit port override, if one was configured.
    #[must_use]
    pub const fn port(&self) -> Option<u16> {
        self.port
    }

    /// Returns the explicit username override, if one was configured.
    #[must_use]
    pub const fn username(&self) -> Option<&str> {
        match &self.username {
            Some(username) => Some(username.as_str()),
            None => None,
        }
    }

    /// Returns the explicit password override, if one was configured.
    #[must_use]
    pub const fn password(&self) -> Option<&Password> {
        match &self.password {
            Some(password) => Some(password),
            None => None,
        }
    }

    /// Returns the explicit database override, if one was configured.
    #[must_use]
    pub const fn database(&self) -> Option<&str> {
        match &self.database {
            Some(database) => Some(database.as_str()),
            None => None,
        }
    }

    /// Returns the explicit application name override, if one was configured.
    #[must_use]
    pub const fn application_name(&self) -> Option<&str> {
        match &self.application_name {
            Some(application_name) => Some(application_name.as_str()),
            None => None,
        }
    }

    /// Returns the pool acquire timeout.
    #[must_use]
    pub const fn acquire_timeout(&self) -> Duration {
        self.acquire_timeout
    }

    /// Returns the pool idle timeout.
    #[must_use]
    pub const fn idle_timeout(&self) -> Duration {
        self.idle_timeout
    }

    /// Returns the pool connection maximum lifetime.
    #[must_use]
    pub const fn max_lifetime(&self) -> Duration {
        self.max_lifetime
    }

    /// Returns the pool maximum connection count.
    #[must_use]
    pub const fn max_connections(&self) -> NonZeroU32 {
        self.max_connections
    }

    /// Returns the pool target minimum connection count.
    #[must_use]
    pub const fn min_connections(&self) -> u32 {
        self.min_connections
    }

    /// Returns the explicit per-connection prepared statement cache capacity,
    /// if one was configured.
    #[must_use]
    pub const fn statement_cache_capacity(&self) -> Option<usize> {
        self.statement_cache_capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_without_connection_overrides_uses_no_fake_connection_values() {
        let config = build_config(test_builder());

        assert_eq!(
            config.host(),
            None,
            "config must not invent a Postgres host"
        );
        assert_eq!(
            config.port(),
            None,
            "config must not invent a Postgres port"
        );
        assert_eq!(
            config.username(),
            None,
            "config must not invent a Postgres username"
        );
        assert_eq!(
            config.password().map(Password::as_str),
            None,
            "config must not invent a Postgres password"
        );
        assert_eq!(
            config.database(),
            None,
            "config must not invent a Postgres database"
        );
        assert_eq!(
            config.application_name(),
            None,
            "config must not invent an application name"
        );
        assert_eq!(
            config.statement_cache_capacity(),
            None,
            "config must not invent a statement cache capacity"
        );
    }

    #[test]
    fn build_uses_default_pool_values() {
        let config = build_config(test_builder());

        assert_eq!(
            config.acquire_timeout(),
            DEFAULT_ACQUIRE_TIMEOUT,
            "config must default the acquire timeout"
        );
        assert_eq!(
            config.idle_timeout(),
            DEFAULT_IDLE_TIMEOUT,
            "config must default the idle timeout"
        );
        assert_eq!(
            config.max_lifetime(),
            DEFAULT_MAX_LIFETIME,
            "config must default the max lifetime"
        );
        assert_eq!(
            config.max_connections(),
            DEFAULT_MAX_CONNECTIONS,
            "config must default the max connections"
        );
        assert_eq!(
            config.min_connections(),
            DEFAULT_MIN_CONNECTIONS,
            "config must default the min connections"
        );
    }

    #[test]
    fn build_uses_overridden_connection_values() {
        let host = "localhost";
        let port = 15432;
        let username = "recomp";
        let password = "";
        let database = "recomp_development";
        let application_name = "recomp";
        let config = build_config(
            test_builder()
                .with_host(host)
                .with_port(port)
                .with_username(username)
                .with_password(password)
                .with_database(database)
                .with_application_name(application_name),
        );

        assert_eq!(
            config.host(),
            Some(host),
            "config must preserve the supplied Postgres host"
        );
        assert_eq!(
            config.port(),
            Some(port),
            "config must preserve the supplied Postgres port"
        );
        assert_eq!(
            config.username(),
            Some(username),
            "config must preserve the supplied Postgres username"
        );
        assert_eq!(
            config.password().map(Password::as_str),
            Some(password),
            "config must preserve an explicitly supplied empty Postgres password"
        );
        assert_eq!(
            config.database(),
            Some(database),
            "config must preserve the supplied Postgres database"
        );
        assert_eq!(
            config.application_name(),
            Some(application_name),
            "config must preserve the supplied application name"
        );
    }

    #[test]
    fn build_uses_overridden_pool_values() {
        let acquire_timeout = Duration::from_secs(2);
        let idle_timeout = Duration::from_secs(3);
        let max_lifetime = Duration::from_secs(4);
        let max_connections = non_zero_u32(20);
        let min_connections = 5;
        let statement_cache_capacity = 256;
        let config = build_config(
            test_builder()
                .with_acquire_timeout(acquire_timeout)
                .with_idle_timeout(idle_timeout)
                .with_max_lifetime(max_lifetime)
                .with_max_connections(max_connections)
                .with_min_connections(min_connections)
                .with_statement_cache_capacity(statement_cache_capacity),
        );

        assert_eq!(
            config.acquire_timeout(),
            acquire_timeout,
            "config must preserve the supplied acquire timeout"
        );
        assert_eq!(
            config.idle_timeout(),
            idle_timeout,
            "config must preserve the supplied idle timeout"
        );
        assert_eq!(
            config.max_lifetime(),
            max_lifetime,
            "config must preserve the supplied max lifetime"
        );
        assert_eq!(
            config.max_connections(),
            max_connections,
            "config must preserve the supplied max connections"
        );
        assert_eq!(
            config.min_connections(),
            min_connections,
            "config must preserve the supplied min connections"
        );
        assert_eq!(
            config.statement_cache_capacity(),
            Some(statement_cache_capacity),
            "config must preserve the supplied statement cache capacity"
        );
    }

    #[test]
    fn debug_redacts_password() {
        let config = build_config(test_builder().with_password("secret-password"));
        let debug = format!("{config:?}");

        assert!(
            debug.contains("<redacted>"),
            "debug output must show that the password was redacted: {debug}"
        );
        assert!(
            !debug.contains("secret-password"),
            "debug output must not contain the Postgres password: {debug}"
        );
    }

    #[test]
    fn build_rejects_min_connections_over_max_connections() {
        let max_connections = non_zero_u32(2);
        let min_connections = 3;
        let error = match test_builder()
            .with_max_connections(max_connections)
            .with_min_connections(min_connections)
            .build()
        {
            Ok(config) => panic!(
                "min connections above max connections must be rejected, got config: {config:?}"
            ),
            Err(error) => error,
        };

        assert_eq!(
            error,
            ConfigBuilderError::MinConnectionsExceedsMaxConnections {
                min_connections,
                max_connections,
            },
            "builder must report the invalid pool size relationship"
        );
    }

    fn test_builder() -> ConfigBuilder {
        Config::builder()
    }

    fn build_config(builder: ConfigBuilder) -> Config {
        match builder.build() {
            Ok(config) => config,
            Err(error) => panic!("test config must build: {error}"),
        }
    }

    const fn non_zero_u32(value: u32) -> NonZeroU32 {
        match NonZeroU32::new(value) {
            Some(value) => value,
            None => panic!("test value must be non-zero"),
        }
    }
}
