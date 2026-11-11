# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `djs-model`: `audit_log` module — `AuditLog`, `AuditLogEntry`, `AuditLogEvent`,
  `AuditLogEntryOptions` (D-071), `AuditLogChange` with `AuditLogValue` values that convert to
  any field type (D-070); `guild::PartialIntegration::application_id`.
- `djs-model`: the interaction model — `interaction::Interaction` (token redacted in `Debug`)
  with `InteractionData` by type (D-067): `CommandData` with `CommandDataOption` /
  `CommandOptionValue` (focused autocomplete values), `ComponentData`, `ModalSubmitData` with
  `SubmittedComponent`; `Resolved`, `InteractionMember`, `InteractionChannel`,
  `InteractionGuild`. Responses (D-068): `InteractionResponse`, `InteractionCallbackType`,
  `MessageCallbackData`, `DeferredCallbackData`, `AutocompleteCallbackData` /
  `AutocompleteChoices`, `ModalCallbackData`, and the callback response objects
  (`InteractionCallbackResponse`, `InteractionCallback`, `CallbackResource`,
  `ActivityInstanceResource`). Request types `poll::CreatePoll` (+ `Media::new`) and
  `message::{AttachmentRequest, AttachmentRequestId}`; `Message::resolved`.
- `djs-model`: `entitlement::{Entitlement, EntitlementType}` (with `Entitlement::is_active`).
- `djs-model`: `application_command` module — `ApplicationCommand`, `CommandType`, the
  `CommandOption` enum (D-069) with typed `Choice<T>`s, entry point handler types, command
  permissions (with the `@everyone` / all-channels constants); `interaction::InteractionContextType`.
- `djs-model`: `component` module — the `Component` enum (D-065) with every documented
  component (action rows, buttons as `Button::{Interactive, Link, Premium}` (D-066), the five
  select menus, text inputs, sections, text displays, thumbnails, media galleries, files,
  separators, containers, labels, file uploads, radio and checkbox groups, checkboxes),
  `UnfurledMediaItem` (+ flags, content scan metadata), constructors for sending;
  `Message::components`, `SnapshotMessage::components`, `Embed::components`.
- `djs-model`: the full message model — `message::Message` with every documented field
  (gateway extras `member`, `channel_type`; D-062: components and `resolved` follow later),
  `Mention` (D-060), `ReferencedMessage` (D-061), `Nonce`, `Attachment`, `Embed` and its parts,
  `Reaction`, `ChannelMention`, `AllowedMentions`, `MessageActivity`, `MessageCall`,
  `MessageSnapshot`, `RoleSubscriptionData`, `MessagePin`, `SharedClientTheme`, the
  `InteractionMetadata` enum (D-063); `poll` module; `interaction::{InteractionType,
  MessageInteraction, AuthorizingIntegrationOwners, GuildInstallOwner}`.
- `djs-model`: `guild_template::{GuildTemplate, SerializedSourceGuild, TemplateRole,
  TemplateChannel, TemplateOverwrite, TemplateForumTag}` with `u64` placeholder IDs (D-059).
- `djs-model`: `user::{Connection, ConnectionService, ConnectionVisibility,
  ApplicationRoleConnection}`, `guild::PartialIntegration` and
  `application_role_connection_metadata::{ApplicationRoleConnectionMetadata, MetadataType}`;
  dictionaries are `BTreeMap`s (D-058).
- `djs-model`: `application::{Application, PartialApplication, IntegrationType,
  IntegrationTypesConfig, IntegrationTypeConfig, InstallParams, EventWebhookStatus,
  WebhookEventType, ActivityInstance, ActivityLocation, ActivityLocationKind}`,
  `team::{Team, Member, MembershipState, MemberRole}`, `guild::PartialGuild` and
  `Invite::target_application`.
- `djs-model`: `stage_instance::{StageInstance, PrivacyLevel}`, `webhook::{Webhook,
  WebhookType, SourceGuild, SourceChannel}` (token redacted in `Debug`), `invite::{Invite,
  InviteWithMetadata, InviteType, InviteTargetType, InviteGuild, InviteChannel,
  InviteRecipient, InviteRole}`.
- `djs-model`: channel model (D-057) — `channel::Channel` enum (text, DM, voice, group DM,
  category, announcement, three thread kinds, stage, directory, forum, media, unknown) with
  per-kind structs and shared accessors, single-pass non-buffering deserialization; `Overwrite`
  (+ `OverwriteType`), `ThreadMetadata`, `ThreadMember` (+ flags), `ForumTag`,
  `DefaultReaction`, `FollowedChannel`, `VideoQualityMode`, `SortOrderType`, `ForumLayoutType`,
  `AutoArchiveDuration`.
- `djs-model`: `guild_scheduled_event` (events, privacy level, entity type, status, entity
  metadata, interested users, recurrence rules) and `auto_moderation` (rules, trigger types and
  metadata, keyword presets, event types, actions and their metadata).
- `djs-model`: guild model — `guild::{Guild, Feature, Preview, Widget, WidgetSettings,
  WidgetChannel, WidgetMember, WidgetActivity, WidgetStyle, Integration, IntegrationType,
  IntegrationExpireBehavior, IntegrationAccount, IntegrationApplication, Ban, WelcomeScreen,
  WelcomeScreenChannel, Onboarding, OnboardingPrompt, PromptOption, OnboardingMode, PromptType,
  IncidentsData}` and the level enums; `oauth2::Scope`; `gateway::Status`. Docs examples are
  test fixtures (C-022).
- `djs-model`: `emoji::{Emoji, PartialEmoji}` (enum with a non-buffering deserializer, D-056)
  and `sticker::{Sticker, StickerType, StickerFormatType, StickerItem, StickerPack}`; absent
  `available` means `true` (D-055).
- `djs-model`: `guild::{Member, PartialMember, Role, RoleColors, RoleTags}` with
  `Member::display_name`/`is_timed_out`, `PartialMember::into_member`, `RoleColors::HOLOGRAPHIC`
  and "null means true" role tags.
- `djs-model`: complete user model — `user::{User, CurrentUser, AvatarDecorationData,
  Collectibles, Nameplate, NameplatePalette, PrimaryGuild, PremiumType}` (D-052, D-054),
  `util::Locale`, string-enum macro (C-018); `Ready::user` is now a `CurrentUser` and
  `ReadyApplication` carries `flags`.
- `djs-model`: all bitflags types (`u64`, integer serde, unknown bits kept; D-046/D-047):
  `message::{MessageFlags, AttachmentFlags, EmbedFlags, EmbedMediaFlags}`, `user::UserFlags`,
  `application::ApplicationFlags`, `channel::ChannelFlags`,
  `guild::{SystemChannelFlags, MemberFlags, RoleFlags}`, `invite::InviteFlags`,
  `sku::SkuFlags`, `gateway::ActivityFlags`; new modules `application`, `channel`, `invite`,
  `sku` (D-044).
- `djs-model`: `permissions::Permissions` (`u64` bitflags, all 52 documented flags, decimal-string
  serde that keeps unknown bits; D-040, D-045).
- Performance suite (D-041, D-042): divan micro-benchmarks for gateway decoding and value
  parsing (`djs-model/benches/decode.rs`), ignored extreme-load tests for `Shard`, `Rest` and
  `Client` (`tests/stress.rs` in each crate), results in `docs/PERFORMANCE.md`; CI compiles the
  benchmarks.
- Project foundation: decision log, roadmap, progress tracking, licensing (MIT OR Apache-2.0),
  contribution guidelines.
- Cargo workspace with nine placeholder crates (`djs-rs`, `djs-model`, `djs-rest`,
  `djs-gateway`, `djs-cache`, `djs-builders`, `djs-collection`, `djs-formatters`, `djs-util`).
- Workspace lints, rustfmt/clippy/cargo-deny configuration, GitHub Actions CI.
- `docs/ARCHITECTURE.md`.
- `djs-model` core (Phase 1): typed snowflake IDs `Id<Marker>` with 27 markers and aliases,
  `Timestamp` (dependency-free ISO 8601), compact `ImageHash`, `User`, `UnavailableGuild`,
  `Message` + `MessageType`, gateway `OpCode`, `CloseCode`, `Intents`, `ShardId`, payloads
  (`Hello`, `Ready`, `MessageCreate`, `Identify`, `Resume`, `Heartbeat`), `GatewayEvent` /
  `DispatchEvent` and the single-pass `GatewayEventDeserializer`.
- `djs-model`: `MessageReference` (+ type), `rest::{GatewayBot, SessionStartLimit, CreateMessage}`.
- `djs-rest` core (Phase 1): `Rest` client on hyper 1.x + rustls (ring, webpki-roots) with
  HTTP/2, `RestBuilder` (discord.js `RESTOptions` defaults), per-bucket sequential rate limiting
  with hash learning and state migration, global limiter + global 429 pauses, retries for
  5xx/transport/timeouts, `ApiError` parsing, typed `Route`s, `get_gateway_bot`,
  `create_message`, idle-bucket sweeping, token redaction.
- `djs-gateway` core (Phase 1): poll-driven `Shard` (connect/Hello timeouts, identify with
  per-shard 5 s limit, resume, heartbeats with jitter + zombie detection, reconnect backoff,
  close-code handling), `MessageSender`, `Config`/`ConfigBuilder`, `zstd-stream` compression
  (default feature), rustls with explicit ring provider.
- `djs-rs` core (Phase 1): `Client`/`ClientBuilder`, typed `Events` (`ClientReady`,
  `MessageCreate`), `client.on` with `Arc`-shared payloads, `HandlerMode` (concurrent by default,
  sequential optional), `client.events()` stream, `Context`, `MessageExt::reply`,
  `login`/`shutdown`, prelude; `examples/ping.rs`; end-to-end tests with mock REST + gateway.

### Changed
- `djs-model`: `Message`, `MessageCreate`, `GatewayEvent` and `DispatchEvent` are `PartialEq`
  but no longer `Eq`, because messages contain floats (D-064).
- `djs-model`: received model structs are `#[non_exhaustive]` (D-053).
- MSRV raised from 1.85 to 1.89 (required by maintained `tokio-websockets` and `simd-json`).
