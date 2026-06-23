//! Traits for long-running application components.
//!
//! A component owns a runtime task such as a web server, queue consumer,
//! scheduler, database connector, or other process that usually runs for the
//! lifetime of an application. Components run until they complete, receive a
//! cancellation signal, or hit a terminal error.
//!
//! Health reporting is intentionally separate from the component lifecycle. A
//! component can implement health-check behavior alongside [`Component`] when
//! callers need to wait for readiness or expose health probes.

use std::{error::Error, future::Future, sync::Arc};

use tokio_util::sync::CancellationToken;

mod health;

pub use health::{HealthCheck, HealthProbe, WaitUntilHealthyError};

/// A long-running application component.
///
/// Implement this trait for services that own a runtime lifecycle, such as a
/// server, worker, consumer, scheduler, or dependency connector. A component's
/// [`run`](Component::run) method should keep the component alive until the work
/// is complete, cancellation is requested, or a terminal error occurs.
///
/// Readiness and liveness are separate from this trait. Components that need to
/// expose health state should also implement [`HealthCheck`].
pub trait Component: Send + Sync + 'static {
    /// The terminal error returned by [`run`](Component::run).
    type RunError: Error + Send + Sync + 'static;

    /// Returns the name of the component.
    fn name(&self) -> &str;

    /// Runs the component until completion, cancellation, or a terminal error.
    ///
    /// The cancellation token is the cooperative shutdown signal for the
    /// component and any child work it owns. Returning `Ok(())` means the
    /// component stopped without a terminal error; returning `Err` means the
    /// component cannot continue without external intervention.
    fn run(
        self: Arc<Self>,
        cancel: CancellationToken,
    ) -> impl Future<Output = Result<(), Self::RunError>> + Send;
}
