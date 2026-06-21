use std::{
    sync::{
        OnceLock,
        atomic::{AtomicU8, Ordering},
    },
    time::Duration,
};

use sqlx::Executor as _;
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use thiserror::Error;
use tokio_util::sync::CancellationToken;

use crate::component::{Component, HealthCheck, HealthProbe};

use super::Config;

const HEALTH_PROBE_INTERVAL: Duration = Duration::from_secs(10);

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HealthState {
    Uninitialized = 0,
    Starting = 1,
    Healthy = 2,
    Unhealthy = 3,
    Stopped = 4,
}

impl From<HealthState> for u8 {
    fn from(state: HealthState) -> Self {
        match state {
            HealthState::Uninitialized => 0,
            HealthState::Starting => 1,
            HealthState::Healthy => 2,
            HealthState::Unhealthy => 3,
            HealthState::Stopped => 4,
        }
    }
}

impl TryFrom<u8> for HealthState {
    type Error = InvalidHealthState;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Uninitialized),
            1 => Ok(Self::Starting),
            2 => Ok(Self::Healthy),
            3 => Ok(Self::Unhealthy),
            4 => Ok(Self::Stopped),
            value => Err(InvalidHealthState { value }),
        }
    }
}

#[derive(Debug, Error)]
#[error("invalid Postgres health state value {value}")]
struct InvalidHealthState {
    value: u8,
}

#[derive(Debug)]
enum InitializeOutcome {
    Initialized,
    Cancelled,
}

/// Long-running Postgres pool component.
///
/// The component connects a [`PgPool`], keeps it open until cancellation, and
/// reports healthy only after a live `SELECT 1` probe succeeds.
pub struct Postgres {
    name: String,
    config: Config,
    pool: OnceLock<PgPool>,
    health: AtomicU8,
}

impl Postgres {
    /// Creates a Postgres component without connecting to the database.
    #[must_use]
    pub fn new(name: impl Into<String>, config: Config) -> Self {
        Self {
            name: name.into(),
            config,
            pool: OnceLock::new(),
            health: AtomicU8::new(HealthState::Uninitialized.into()),
        }
    }

    /// Returns the connected pool.
    pub fn pool(&self) -> Result<&PgPool, PostgresAccessError> {
        self.pool.get().ok_or(PostgresAccessError::NotConnected)
    }

    async fn initialize(
        &self,
        cancel: &CancellationToken,
    ) -> Result<InitializeOutcome, PostgresRunError> {
        if cancel.is_cancelled() {
            return Ok(InitializeOutcome::Cancelled);
        }

        if self.pool.get().is_some() {
            return Err(PostgresRunError::AlreadyInitialized);
        }

        let options = PgConnectOptions::from(&self.config);
        let pool_options = PgPoolOptions::from(&self.config);
        let pool = tokio::select! {
            biased;

            () = cancel.cancelled() => return Ok(InitializeOutcome::Cancelled),
            result = pool_options.connect_with(options) => result.map_err(PostgresRunError::Connect)?,
        };

        if let Err(pool) = self.pool.set(pool.clone()) {
            pool.close().await;

            return Err(PostgresRunError::AlreadyInitialized);
        }

        Ok(InitializeOutcome::Initialized)
    }

    async fn run_health_probe(
        &self,
        cancel: CancellationToken,
        interval: Duration,
    ) -> Result<(), PostgresAccessError> {
        loop {
            let pool = self.pool()?;
            let probe = tokio::select! {
                biased;

                () = cancel.cancelled() => return Ok(()),
                result = pool.execute("SELECT 1") => result,
            };

            if probe.is_ok() {
                self.set_health(HealthState::Healthy);
            } else {
                self.set_health(HealthState::Unhealthy);
            }

            tokio::select! {
                biased;

                () = cancel.cancelled() => return Ok(()),
                () = tokio::time::sleep(interval) => {}
            }
        }
    }

    fn set_health(&self, state: HealthState) {
        self.health.store(state.into(), Ordering::Release);
    }

    fn health_state(&self) -> Result<HealthState, InvalidHealthState> {
        HealthState::try_from(self.health.load(Ordering::Acquire))
    }
}

/// Error returned when accessing the Postgres pool before it is connected.
#[derive(Debug, Error)]
pub enum PostgresAccessError {
    #[error("Postgres pool is not connected")]
    NotConnected,
}

/// Error returned by [`Postgres::run`](Component::run).
#[derive(Debug, Error)]
pub enum PostgresRunError {
    #[error("Postgres component is already initialized")]
    AlreadyInitialized,
    #[error("Postgres pool connection failed")]
    Connect(#[source] sqlx::Error),
    #[error(transparent)]
    PoolAccess(#[from] PostgresAccessError),
}

impl Component for Postgres {
    type RunError = PostgresRunError;

    fn name(&self) -> &str {
        self.name.as_str()
    }

    async fn run(&self, cancel: CancellationToken) -> Result<(), Self::RunError> {
        if self
            .health
            .compare_exchange(
                HealthState::Uninitialized.into(),
                HealthState::Starting.into(),
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_err()
        {
            return Err(PostgresRunError::AlreadyInitialized);
        }

        match self.initialize(&cancel).await {
            Ok(InitializeOutcome::Initialized) => {
                let health_probe_result = tokio::select! {
                    biased;

                    () = cancel.cancelled() => Ok(()),
                    result = self.run_health_probe(cancel.clone(), HEALTH_PROBE_INTERVAL) => result,
                };
                health_probe_result?;

                self.set_health(HealthState::Stopped);
                self.pool()?.close().await;
                Ok(())
            }
            Ok(InitializeOutcome::Cancelled) => {
                self.set_health(HealthState::Stopped);
                Ok(())
            }
            Err(error @ PostgresRunError::AlreadyInitialized) => Err(error),
            Err(error) => {
                self.set_health(HealthState::Unhealthy);
                Err(error)
            }
        }
    }
}

/// Error returned by [`Postgres`] health checks.
#[derive(Debug, Error)]
pub enum PostgresHealthError {
    #[error("Postgres pool is unhealthy")]
    Unhealthy,
}

impl HealthCheck for Postgres {
    type HealthError = PostgresHealthError;

    fn is_healthy(&self, _probe: HealthProbe) -> Result<(), Self::HealthError> {
        match self.health_state() {
            Ok(HealthState::Healthy) => Ok(()),
            _ => Err(PostgresHealthError::Unhealthy),
        }
    }
}
