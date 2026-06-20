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

mod component;
mod health;

pub use component::Component;
pub use health::{HealthCheck, HealthProbe};
