use std::{error::Error, fmt, future::Future, time::Duration};

use tokio_util::sync::CancellationToken;

/// A caller-defined reason for checking component health.
///
/// The variants are modeled after common Kubernetes probe categories as a
/// convenience, but the crate does not assign fixed semantics to them. Each
/// component decides which probes it recognizes and what healthy means for that
/// probe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HealthProbe {
    /// A startup-oriented probe.
    Startup,
    /// A readiness-oriented probe.
    Readiness,
    /// A liveness-oriented probe.
    Liveness,
    /// A probe that does not fit the other categories.
    Other,
}

/// An error returned while waiting for a health check to become healthy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WaitUntilHealthyError {
    /// The wait was cancelled before the health check became healthy.
    Cancelled,
}

impl fmt::Display for WaitUntilHealthyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => f.write_str("health wait cancelled"),
        }
    }
}

impl Error for WaitUntilHealthyError {}

/// Health reporting for a component or other runtime dependency.
///
/// Implement this trait alongside [`Component`](super::Component) when callers
/// need to wait for readiness, supervise startup, or expose health probes.
pub trait HealthCheck {
    /// The reason the component is not healthy for a probe.
    type HealthError: Error + Send + Sync + 'static;

    /// Checks whether the component is healthy for the requested probe.
    fn is_healthy(&self, probe: HealthProbe) -> Result<(), Self::HealthError>;

    /// Waits until [`HealthProbe::Other`] is healthy or cancellation is requested.
    ///
    /// Health errors are treated as "not yet healthy" and retried after the
    /// provided interval. Call [`is_healthy`](HealthCheck::is_healthy) directly
    /// when the caller needs the current health error.
    fn wait_until_healthy(
        &self,
        cancel: CancellationToken,
        interval: Duration,
    ) -> impl Future<Output = Result<(), WaitUntilHealthyError>> + Send + '_
    where
        Self: Sync,
    {
        async move {
            loop {
                if self.is_healthy(HealthProbe::Other).is_ok() {
                    return Ok(());
                }

                tokio::select! {
                    () = cancel.cancelled() => return Err(WaitUntilHealthyError::Cancelled),
                    () = tokio::time::sleep(interval) => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fmt, time::Duration};

    use super::{HealthCheck, HealthProbe, WaitUntilHealthyError};
    use tokio_util::sync::CancellationToken;

    struct AlwaysHealthy;

    impl HealthCheck for AlwaysHealthy {
        type HealthError = Unhealthy;

        fn is_healthy(&self, _probe: HealthProbe) -> Result<(), Self::HealthError> {
            Ok(())
        }
    }

    struct NeverHealthy;

    impl HealthCheck for NeverHealthy {
        type HealthError = Unhealthy;

        fn is_healthy(&self, _probe: HealthProbe) -> Result<(), Self::HealthError> {
            Err(Unhealthy)
        }
    }

    #[derive(Debug)]
    struct Unhealthy;

    impl fmt::Display for Unhealthy {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("unhealthy")
        }
    }

    impl std::error::Error for Unhealthy {}

    #[test]
    fn wait_until_healthy_returns_when_healthy() {
        let runtime = runtime();
        let health = AlwaysHealthy;
        let cancel = CancellationToken::new();

        let result = runtime.block_on(health.wait_until_healthy(cancel, Duration::from_secs(1)));

        assert!(
            result.is_ok(),
            "healthy component should not wait for cancellation"
        );
    }

    #[test]
    fn wait_until_healthy_returns_when_cancelled() {
        let runtime = runtime();
        let health = NeverHealthy;
        let cancel = CancellationToken::new();
        cancel.cancel();

        let result = runtime.block_on(health.wait_until_healthy(cancel, Duration::from_secs(1)));

        assert_eq!(
            result,
            Err(WaitUntilHealthyError::Cancelled),
            "cancelled wait should return the cancellation error"
        );
    }

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("test runtime should build")
    }
}
