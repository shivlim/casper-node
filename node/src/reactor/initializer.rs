//! Reactor used to initialize a node.

use std::fmt::{self, Display, Formatter};

use datasize::DataSize;
use derive_more::From;
use prometheus::Registry;
use serde::Serialize;
use thiserror::Error;

#[cfg(test)]
use crate::types::Chainspec;
use crate::{
    components::{
        chainspec_loader::{self, ChainspecLoader},
        contract_runtime::{self, ContractRuntime},
        network::NetworkIdentity,
        small_network::{SmallNetworkIdentity, SmallNetworkIdentityError},
        storage::{self, Storage},
        Component,
    },
    effect::{
        announcements::ChainspecLoaderAnnouncement,
        requests::{ContractRuntimeRequest, NetworkRequest, StorageRequest},
        EffectBuilder, Effects,
    },
    protocol::Message,
    reactor::{self, validator, EventQueueHandle},
    types::{chainspec, NodeId},
    utils::WithDir,
    NodeRng,
};

/// Top-level event for the reactor.
#[derive(Debug, From, Serialize)]
#[must_use]
pub enum Event {
    /// Chainspec handler event.
    #[from]
    Chainspec(chainspec_loader::Event),

    /// Storage event.
    #[from]
    Storage(#[serde(skip_serializing)] storage::Event),

    /// Contract runtime event.
    #[from]
    ContractRuntime(contract_runtime::Event),
}

impl From<StorageRequest> for Event {
    fn from(request: StorageRequest) -> Self {
        Event::Storage(storage::Event::StorageRequest(request))
    }
}

impl From<ContractRuntimeRequest> for Event {
    fn from(request: ContractRuntimeRequest) -> Self {
        Event::ContractRuntime(contract_runtime::Event::Request(request))
    }
}

impl From<NetworkRequest<NodeId, Message>> for Event {
    fn from(_request: NetworkRequest<NodeId, Message>) -> Self {
        unreachable!("no network traffic happens during initialization")
    }
}

impl From<ChainspecLoaderAnnouncement> for Event {
    fn from(_announcement: ChainspecLoaderAnnouncement) -> Self {
        unreachable!("no chainspec announcements happen during initialization")
    }
}

impl Display for Event {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Event::Chainspec(event) => write!(formatter, "chainspec: {}", event),
            Event::Storage(event) => write!(formatter, "storage: {}", event),
            Event::ContractRuntime(event) => write!(formatter, "contract runtime: {}", event),
        }
    }
}

/// Error type returned by the initializer reactor.
#[derive(Debug, Error)]
pub enum Error {
    /// `Config` error.
    #[error("config error: {0}")]
    ConfigError(String),

    /// Metrics-related error
    #[error("prometheus (metrics) error: {0}")]
    Metrics(#[from] prometheus::Error),

    /// `ChainspecHandler` component error.
    #[error("chainspec error: {0}")]
    Chainspec(#[from] chainspec::Error),

    /// `Storage` component error.
    #[error("storage error: {0}")]
    Storage(#[from] storage::Error),

    /// `ContractRuntime` component error.
    #[error("contract runtime config error: {0}")]
    ContractRuntime(#[from] contract_runtime::ConfigError),

    /// An error that occurred when creating a `SmallNetworkIdentity`.
    #[error(transparent)]
    SmallNetworkIdentityError(#[from] SmallNetworkIdentityError),
}

/// Initializer node reactor.
#[derive(DataSize, Debug)]
pub struct Reactor {
    pub(super) config: WithDir<validator::Config>,
    pub(super) chainspec_loader: ChainspecLoader,
    pub(super) storage: Storage,
    pub(super) contract_runtime: ContractRuntime,
    pub(super) small_network_identity: SmallNetworkIdentity,
    #[data_size(skip)]
    pub(super) network_identity: NetworkIdentity,
}

impl Reactor {
    /// Returns whether the initialization process completed successfully or not.
    pub fn stopped_successfully(&self) -> bool {
        self.chainspec_loader.stopped_successfully()
    }

    #[cfg(test)]
    pub(crate) fn new_with_chainspec(
        config: <Self as reactor::Reactor>::Config,
        registry: &Registry,
        event_queue: EventQueueHandle<Event>,
        chainspec: Chainspec,
    ) -> Result<(Self, Effects<Event>), Error> {
        let effect_builder = EffectBuilder::new(event_queue);
        let (chainspec_loader, chainspec_effects) =
            ChainspecLoader::new_with_chainspec(chainspec, effect_builder);
        Self::new_with_chainspec_loader(config, registry, chainspec_loader, chainspec_effects)
    }

    fn new_with_chainspec_loader(
        config: <Self as reactor::Reactor>::Config,
        registry: &Registry,
        chainspec_loader: ChainspecLoader,
        chainspec_effects: Effects<chainspec_loader::Event>,
    ) -> Result<(Self, Effects<Event>), Error> {
        let storage_config = config.map_ref(|cfg| cfg.storage.clone());
        let storage = Storage::new(&storage_config)?;

        let contract_runtime =
            ContractRuntime::new(storage_config, &config.value().contract_runtime, registry)?;

        let effects = reactor::wrap_effects(Event::Chainspec, chainspec_effects);

        let small_network_identity = SmallNetworkIdentity::new()?;

        let network_identity = NetworkIdentity::new();

        let reactor = Reactor {
            config,
            chainspec_loader,
            storage,
            contract_runtime,
            small_network_identity,
            network_identity,
        };
        Ok((reactor, effects))
    }
}

impl reactor::Reactor for Reactor {
    type Event = Event;
    type Config = WithDir<validator::Config>;
    type Error = Error;

    fn new(
        config: Self::Config,
        registry: &Registry,
        event_queue: EventQueueHandle<Self::Event>,
        _rng: &mut NodeRng,
    ) -> Result<(Self, Effects<Self::Event>), Error> {
        let effect_builder = EffectBuilder::new(event_queue);

        // Construct the `ChainspecLoader` first so we fail fast if the chainspec is invalid.
        let (chainspec_loader, chainspec_effects) =
            ChainspecLoader::new(config.dir(), effect_builder)?;
        Self::new_with_chainspec_loader(config, registry, chainspec_loader, chainspec_effects)
    }

    fn dispatch_event(
        &mut self,
        effect_builder: EffectBuilder<Self::Event>,
        rng: &mut NodeRng,
        event: Event,
    ) -> Effects<Self::Event> {
        match event {
            Event::Chainspec(event) => reactor::wrap_effects(
                Event::Chainspec,
                self.chainspec_loader
                    .handle_event(effect_builder, rng, event),
            ),
            Event::Storage(event) => reactor::wrap_effects(
                Event::Storage,
                self.storage.handle_event(effect_builder, rng, event),
            ),
            Event::ContractRuntime(event) => reactor::wrap_effects(
                Event::ContractRuntime,
                self.contract_runtime
                    .handle_event(effect_builder, rng, event),
            ),
        }
    }

    fn is_stopped(&mut self) -> bool {
        self.chainspec_loader.is_stopped()
    }
}
