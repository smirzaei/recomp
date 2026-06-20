use std::{error::Error, future::Future};

use tokio_util::sync::CancellationToken;

/// A long-running application component.
///
/// Implement this trait for services that own a runtime lifecycle, such as a
/// server, worker, consumer, scheduler, or dependency connector. A component's
/// [`run`](Component::run) method should keep the component alive until the work
/// is complete, cancellation is requested, or a terminal error occurs.
///
/// Readiness and liveness are separate from this trait. Components that need to
/// expose health state should also implement [`HealthCheck`](crate::HealthCheck).
pub trait Component {
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
        &self,
        cancel: CancellationToken,
    ) -> impl Future<Output = Result<(), Self::RunError>> + Send;
}
