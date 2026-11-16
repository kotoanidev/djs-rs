//! Shard configuration.

use core::{fmt, time::Duration};
use std::sync::Arc;

use djs_model::gateway::{
    Intents,
    payload::{IdentifyProperties, UpdatePresence},
};
use tokio_websockets::Connector;

use crate::{Compression, IdentifyQueue, LocalIdentifyQueue};

/// The default gateway URL, used until a URL from
/// [Get Gateway Bot](https://docs.discord.com/developers/events/gateway#get-gateway-bot) is
/// provided.
pub const DEFAULT_GATEWAY_URL: &str = "wss://gateway.discord.gg";

/// The configuration of a [`Shard`](crate::Shard).
///
/// Cheap to clone. Clones share the TLS configuration and the [IDENTIFY
/// queue](ConfigBuilder::identify_queue), so shards created from clones of one configuration
/// take turns identifying. Build one with [`Config::builder`].
///
/// The [`Debug`] implementation redacts the token.
///
/// discord.js counterpart: `WebSocketManagerOptions` (`@discordjs/ws`).
#[derive(Clone)]
pub struct Config {
    pub(crate) token: Box<str>,
    pub(crate) intents: Intents,
    pub(crate) gateway_url: Box<str>,
    pub(crate) compression: Compression,
    pub(crate) properties: IdentifyProperties,
    pub(crate) large_threshold: Option<u8>,
    pub(crate) connect_timeout: Duration,
    pub(crate) hello_timeout: Duration,
    pub(crate) tls: Arc<Connector>,
    pub(crate) identify_queue: Arc<dyn IdentifyQueue>,
    pub(crate) presence: Option<UpdatePresence>,
}

impl Config {
    /// Creates a configuration with default settings.
    ///
    /// Pass the raw token, without the `Bot ` prefix.
    ///
    /// discord.js counterpart: `new WebSocketManager(options)` (`@discordjs/ws`).
    #[must_use]
    pub fn new(token: impl Into<String>, intents: Intents) -> Self {
        Self::builder(token, intents).build()
    }

    /// Returns a builder to customise the configuration.
    ///
    /// discord.js counterpart: `WebSocketManagerOptions` (`@discordjs/ws`).
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_gateway::{Compression, Config};
    /// use djs_model::gateway::Intents;
    ///
    /// let config = Config::builder("my-token", Intents::GUILDS | Intents::GUILD_MESSAGES)
    ///     .gateway_url("wss://gateway.discord.gg")
    ///     .compression(Compression::None)
    ///     .large_threshold(250)
    ///     .build();
    /// assert_eq!(config.intents(), Intents::GUILDS | Intents::GUILD_MESSAGES);
    /// ```
    pub fn builder(token: impl Into<String>, intents: Intents) -> ConfigBuilder {
        ConfigBuilder {
            config: Self {
                token: token.into().into_boxed_str(),
                intents,
                gateway_url: DEFAULT_GATEWAY_URL.into(),
                compression: Compression::default(),
                properties: IdentifyProperties::new("djs-rs"),
                large_threshold: None,
                connect_timeout: Duration::from_secs(30),
                hello_timeout: Duration::from_secs(60),
                tls: Arc::new(crate::tls::connector()),
                identify_queue: Arc::new(LocalIdentifyQueue::default()),
                presence: None,
            },
        }
    }

    /// Returns the gateway intents.
    ///
    /// discord.js counterpart: the `intents` option of `WebSocketManagerOptions` (`@discordjs/ws`).
    #[must_use]
    pub const fn intents(&self) -> Intents {
        self.intents
    }

    /// Returns the gateway URL used for new sessions.
    ///
    /// discord.js counterpart: none (`@discordjs/ws` uses the URL of Get Gateway Bot).
    #[must_use]
    pub fn gateway_url(&self) -> &str {
        &self.gateway_url
    }

    /// Returns the transport compression.
    ///
    /// discord.js counterpart: the `compression` option of `WebSocketManagerOptions` (`@discordjs/ws`).
    #[must_use]
    pub const fn compression(&self) -> Compression {
        self.compression
    }

    /// Returns the presence shards using this configuration start with.
    ///
    /// discord.js counterpart: the `initialPresence` option of `WebSocketManagerOptions` (`@discordjs/ws`).
    #[must_use]
    pub const fn presence(&self) -> Option<&UpdatePresence> {
        self.presence.as_ref()
    }

    /// Returns the queue shards using this configuration wait in before identifying.
    ///
    /// discord.js counterpart: the `buildIdentifyThrottler` option of `WebSocketManagerOptions` (`@discordjs/ws`).
    #[must_use]
    pub fn identify_queue(&self) -> &Arc<dyn IdentifyQueue> {
        &self.identify_queue
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("token", &"<redacted>")
            .field("intents", &self.intents)
            .field("gateway_url", &self.gateway_url)
            .field("compression", &self.compression)
            .field("properties", &self.properties)
            .field("large_threshold", &self.large_threshold)
            .field("connect_timeout", &self.connect_timeout)
            .field("hello_timeout", &self.hello_timeout)
            .field("identify_queue", &self.identify_queue)
            .field("presence", &self.presence)
            .finish_non_exhaustive()
    }
}

/// Builds a [`Config`]. Created by [`Config::builder`].
///
/// discord.js counterpart: `WebSocketManagerOptions` (`@discordjs/ws`).
#[derive(Debug)]
#[must_use = "a builder does nothing until `build` is called"]
pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    /// Sets the gateway URL for new sessions (default [`DEFAULT_GATEWAY_URL`]).
    ///
    /// Use the URL returned by Get Gateway Bot. Resumes always use the `resume_gateway_url`
    /// from the `READY` event instead. `ws://` URLs are accepted (useful for tests and proxies).
    ///
    /// discord.js counterpart: none (`@discordjs/ws` uses the URL of Get Gateway Bot).
    pub fn gateway_url(mut self, url: impl Into<String>) -> Self {
        self.config.gateway_url = url.into().into_boxed_str();
        self
    }

    /// Sets the transport compression (default [`Compression::ZstdStream`] when the
    /// `zstd-stream` feature is enabled).
    ///
    /// discord.js counterpart: the `compression` option of `WebSocketManagerOptions` (`@discordjs/ws`).
    pub const fn compression(mut self, compression: Compression) -> Self {
        self.config.compression = compression;
        self
    }

    /// Sets the connection properties sent when identifying (default: library `djs-rs`).
    ///
    /// discord.js counterpart: the `identifyProperties` option of `WebSocketManagerOptions` (`@discordjs/ws`).
    pub fn identify_properties(mut self, properties: IdentifyProperties) -> Self {
        self.config.properties = properties;
        self
    }

    /// Sets the member count (50–250) above which Discord stops sending offline members of a
    /// guild (Discord's default is 50).
    ///
    /// discord.js counterpart: the `largeThreshold` option of `WebSocketManagerOptions` (`@discordjs/ws`).
    pub const fn large_threshold(mut self, threshold: u8) -> Self {
        self.config.large_threshold = Some(threshold);
        self
    }

    /// Sets how long establishing the WebSocket connection (TCP, TLS and upgrade) may take
    /// (default 30 s).
    ///
    /// discord.js counterpart: the `handshakeTimeout` option of `WebSocketManagerOptions` (`@discordjs/ws`).
    pub const fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.config.connect_timeout = timeout;
        self
    }

    /// Sets how long to wait for the Hello payload after connecting before reconnecting
    /// (default 60 s).
    ///
    /// discord.js counterpart: the `helloTimeout` option of `WebSocketManagerOptions` (`@discordjs/ws`).
    pub const fn hello_timeout(mut self, timeout: Duration) -> Self {
        self.config.hello_timeout = timeout;
        self
    }

    /// Sets the presence (status and activities) sent when identifying (default: none, the bot
    /// appears online without an activity).
    ///
    /// A shard uses it until [`MessageSender::update_presence`](crate::MessageSender::update_presence)
    /// replaces it; that presence is then also kept for later sessions.
    ///
    /// discord.js counterpart: the `presence` client option.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_gateway::Config;
    /// use djs_model::gateway::{Activity, Intents, Status, payload::UpdatePresence};
    ///
    /// let config = Config::builder("my-token", Intents::GUILDS)
    ///     .presence(UpdatePresence::new(Status::Online, [Activity::watching("you")]))
    ///     .build();
    /// assert!(config.presence().is_some());
    /// ```
    pub fn presence(mut self, presence: UpdatePresence) -> Self {
        self.config.presence = Some(presence);
        self
    }

    /// Sets the queue shards wait in before identifying (default: a new
    /// [`LocalIdentifyQueue`] with `max_concurrency` 1, shared by clones of the configuration).
    ///
    /// Pass a [`LocalIdentifyQueue`] created with the bot's `max_concurrency` to start several
    /// shards at once, or an own [`IdentifyQueue`] to coordinate several processes.
    ///
    /// discord.js counterpart: the `buildIdentifyThrottler` option of `WebSocketManagerOptions` (`@discordjs/ws`).
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    ///
    /// use djs_gateway::{Config, LocalIdentifyQueue};
    /// use djs_model::gateway::Intents;
    ///
    /// let config = Config::builder("my-token", Intents::GUILDS)
    ///     .identify_queue(Arc::new(LocalIdentifyQueue::new(16)))
    ///     .build();
    /// assert!(format!("{config:?}").contains("max_concurrency: 16"));
    /// ```
    pub fn identify_queue(mut self, queue: Arc<dyn IdentifyQueue>) -> Self {
        self.config.identify_queue = queue;
        self
    }

    /// Builds the configuration.
    ///
    /// discord.js counterpart: `new WebSocketManager(options)` (`@discordjs/ws`).
    #[must_use]
    pub fn build(self) -> Config {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;
    use std::sync::Arc;

    use djs_model::gateway::{
        Intents, Status,
        payload::{IdentifyProperties, UpdatePresence},
    };

    use super::Config;
    use crate::{Compression, LocalIdentifyQueue};

    #[test]
    fn defaults() {
        let config = Config::new("a-token", Intents::GUILDS);
        assert_eq!(config.intents(), Intents::GUILDS);
        assert_eq!(config.gateway_url(), super::DEFAULT_GATEWAY_URL);
        assert_eq!(config.compression(), Compression::default());
        assert!(config.presence().is_none());
        assert_eq!(config.connect_timeout, Duration::from_secs(30));
        assert_eq!(config.hello_timeout, Duration::from_secs(60));
        assert_eq!(config.large_threshold, None);
    }

    #[test]
    fn every_setting_is_kept_and_the_token_is_never_shown() {
        let queue = Arc::new(LocalIdentifyQueue::default());
        let config = Config::builder("a-secret-token", Intents::GUILD_MESSAGES)
            .gateway_url("wss://gateway.example.com")
            .compression(Compression::None)
            .identify_properties(IdentifyProperties::new("my-bot"))
            .large_threshold(100)
            .connect_timeout(Duration::from_secs(5))
            .hello_timeout(Duration::from_secs(7))
            .presence(UpdatePresence::new(Status::Idle, []))
            .identify_queue(queue)
            .build();
        assert_eq!(config.gateway_url(), "wss://gateway.example.com");
        assert_eq!(config.compression(), Compression::None);
        assert_eq!(config.large_threshold, Some(100));
        assert_eq!(config.connect_timeout, Duration::from_secs(5));
        assert_eq!(config.hello_timeout, Duration::from_secs(7));
        assert!(config.presence().is_some());
        let debug = format!("{config:?}");
        assert!(!debug.contains("a-secret-token"), "{debug}");
        assert!(
            debug.contains("<redacted>") && debug.contains("my-bot"),
            "{debug}"
        );
        let builder = format!("{:?}", Config::builder("a-secret-token", Intents::GUILDS));
        assert!(!builder.contains("a-secret-token"), "{builder}");
    }
}
