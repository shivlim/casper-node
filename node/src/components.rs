//! Components
//!
//! Components are the building blocks of the whole application, wired together inside a reactor.
//! Each component has a unified interface, expressed by the `Component` trait.
pub(crate) mod block_executor;
pub(crate) mod block_proposer;
pub(crate) mod block_validator;
pub(crate) mod chainspec_loader;
#[cfg(test)]
pub(crate) mod collector;
pub(crate) mod consensus;
pub mod contract_runtime;
pub(crate) mod deploy_acceptor;
pub(crate) mod event_stream_server;
pub(crate) mod fetcher;
pub(crate) mod gossiper;
pub(crate) mod linear_chain;
pub(crate) mod linear_chain_fast_sync;
pub(crate) mod linear_chain_sync;
pub(crate) mod rest_server;
pub(crate) mod rpc_server;
// The `in_memory_network` is public for use in doctests.
#[cfg(test)]
pub mod in_memory_network;
pub(crate) mod metrics;
pub(crate) mod network;
pub(crate) mod networking_metrics;
pub(crate) mod small_network;
pub(crate) mod storage;

use once_cell::sync::Lazy;
use semver::Version;

use crate::{
    effect::{EffectBuilder, Effects},
    NodeRng,
};

// TODO - confirm if we want to use the protocol version for this.
pub(crate) static CLIENT_API_VERSION: Lazy<Version> = Lazy::new(|| Version::new(1, 0, 0));

/// Core Component.
///
/// Its inputs are `Event`s, allowing it to
/// perform work whenever an event is received, outputting `Effect`s each time it is called.
///
/// # Error and halting states
///
/// Components in general are expected to be able to handle every input (`Event`) in every state.
/// Invalid inputs are supposed to be discarded, and the machine is expected to recover from any
/// recoverable error states by itself.
///
/// If a fatal error occurs that is not recoverable, the reactor should be notified instead.
///
/// # Component events and reactor events
///
/// Each component has two events related to it: An associated `Event` and a reactor event (`REv`).
/// The `Event` type indicates what type of event a component accepts, these are typically event
/// types specific to the component.
///
/// Components place restrictions on reactor events (`REv`s), indicating what kind of effects they
/// need to be able to produce to operate.
pub trait Component<REv> {
    /// Event associated with `Component`.
    ///
    /// The event type that is handled by the component.
    type Event;

    /// Error emitted when constructing the component.
    type ConstructionError;

    /// Processes an event, outputting zero or more effects.
    ///
    /// This function must not ever perform any blocking or CPU intensive work, as it is expected
    /// to return very quickly.
    fn handle_event(
        &mut self,
        effect_builder: EffectBuilder<REv>,
        rng: &mut NodeRng,
        event: Self::Event,
    ) -> Effects<Self::Event>;
}
