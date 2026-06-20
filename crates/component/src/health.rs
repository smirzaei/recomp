use std::error::Error;

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

/// Health reporting for a component or other runtime dependency.
///
/// Implement this trait alongside [`Component`](crate::Component) when callers
/// need to wait for readiness, supervise startup, or expose health probes.
pub trait HealthCheck {
    /// The reason the component is not healthy for a probe.
    type HealthError: Error + Send + Sync + 'static;

    /// Checks whether the component is healthy for the requested probe.
    fn is_healthy(&self, probe: HealthProbe) -> Result<(), Self::HealthError>;
}
