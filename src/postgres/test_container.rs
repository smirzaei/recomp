use testcontainers_modules::{
    postgres::Postgres,
    testcontainers::{
        ContainerAsync, ImageExt as _, TestcontainersError, runners::AsyncRunner as _,
    },
};
use url::Host;

use super::{Config, ConfigBuilderError};

const POSTGRES_IMAGE_TAG: &str = "18-alpine";
const POSTGRES_PORT: u16 = 5432;
const POSTGRES_DATABASE: &str = "postgres";
const POSTGRES_PASSWORD: &str = "postgres";
const POSTGRES_USERNAME: &str = "postgres";

/// Error returned while starting a Postgres test container.
#[derive(Debug, thiserror::Error)]
pub enum PostgresContainerStartError {
    #[error("Postgres test container failed to start")]
    Start(#[source] TestcontainersError),
    #[error("Postgres test container host lookup failed")]
    Host(#[source] TestcontainersError),
    #[error("Postgres test container port lookup failed")]
    Port(#[source] TestcontainersError),
}

/// Running Postgres test container.
pub struct PostgresContainer {
    _container: ContainerAsync<Postgres>,
    host: Host,
    port: u16,
}

impl PostgresContainer {
    /// Starts a Postgres test container.
    pub async fn start() -> Result<Self, PostgresContainerStartError> {
        let container = Postgres::default()
            .with_tag(POSTGRES_IMAGE_TAG)
            .start()
            .await
            .map_err(PostgresContainerStartError::Start)?;

        Self::from_container(container).await
    }

    /// Returns the mapped host for connecting from the test process.
    #[must_use]
    pub const fn host(&self) -> &Host {
        &self.host
    }

    /// Returns the mapped port for connecting from the test process.
    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// Builds a [`Config`] for this container.
    pub fn config(&self) -> Result<Config, ConfigBuilderError> {
        Config::builder()
            .with_host(self.host().to_string())
            .with_port(self.port())
            .with_username(POSTGRES_USERNAME)
            .with_password(POSTGRES_PASSWORD)
            .with_database(POSTGRES_DATABASE)
            .build()
    }

    async fn from_container(
        container: ContainerAsync<Postgres>,
    ) -> Result<Self, PostgresContainerStartError> {
        let host = container
            .get_host()
            .await
            .map_err(PostgresContainerStartError::Host)?;
        let port = container
            .get_host_port_ipv4(POSTGRES_PORT)
            .await
            .map_err(PostgresContainerStartError::Port)?;

        Ok(Self {
            _container: container,
            host,
            port,
        })
    }
}
