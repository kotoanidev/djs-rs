# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Multi-process sharding, first step (D-284, D-285, D-296, D-297, D-300–D-303, C-112), behind
  the new `sharding` feature: `ClusterManager` starts the bot's executable again once per
  group of `shards_per_process` shards (1 by default, Discord's recommended count or a fixed
  one), gives each process its shards through `DJS_RS_CLUSTER`, accepts its connection over
  `127.0.0.1` (length-prefixed JSON frames, a secret token), reports `ClusterEvent`s and
  `clusters()`, and on `shutdown` asks every process to stop, killing those still running
  after `shutdown_timeout`. A process tells itself apart with `sharding::process()` and
  passes its `ClusterAssignment` to `ClientBuilder::cluster`; its client shuts down when the
  manager asks or goes away.
- The cluster manager is the IDENTIFY queue of every process (D-299, C-113): shards ask it
  for a turn, granted per rate-limit bucket every 5 seconds over all the processes. It calls
  Get Gateway Bot once (D-305), checks the session starts left for every shard
  (`ClusterError::SessionStartLimit`) and passes the gateway URL and `max_concurrency` to
  the processes, which no longer call Get Gateway Bot themselves.
- Failed cluster processes are restarted (D-298, D-306, C-114):
  `ClusterManagerBuilder::restart(Restart::{OnFailure, Always, Never})` chooses the exits
  (failures by default: a non-zero code or a signal), `restart_delay(first, max)` the
  delays (1 s doubling up to 60 s, back to 1 s after a process ran 60 s), and
  `ClusterEvent::Restarting { cluster, delay }` reports each restart.
- Commands between clusters instead of `broadcastEval` (D-286, D-307–D-309, C-115): a
  type implementing `ClusterCommand` (a name, serde input and output types) is handled in
  each cluster with `client.cluster()` → `ClusterClient::handle`, and sent by the manager
  or any cluster to every cluster (`broadcast`, one `ClusterReply` per cluster) or to one
  (`send`), through the manager, within its `command_timeout` (30 s).
- Built-in cluster values (D-310, C-116): `stats()` on `ClusterManager` and `ClusterClient`
  returns a `ClusterStats` per cluster: each shard's state and heartbeat latency, the
  cached guild and user counts, the client's uptime and the process's memory (Linux); the
  `Stats` command asks one cluster.
- Member permissions (D-292, D-294, D-295, C-111): `Permissions::guild_level`,
  `Permissions::in_channel` and `Permissions::during_timeout` compute them the way Discord
  does (owner, `@everyone`, roles, `ADMINISTRATOR`, channel overwrites, the implicit
  `VIEW_CHANNEL` / `SEND_MESSAGES` denials, timeouts); `Cache::member_permissions` and
  `Cache::member_permissions_in` run them on cached data (`None` when a piece is missing);
  `MemberExt::permissions` and `MemberExt::permissions_in` in `djs-rs`. 111 ns from the cache,
  37 ns for the computation alone, no allocation.
- `Context::shutdown` (D-293): a handler stops the client the way `Client::shutdown` does,
  without capturing a `Client` clone; the examples' `--cleanup` paths use it.
- Chainable setters on the message bodies (D-290, D-291, C-110): `CreateMessage`,
  `MessageCallbackData`, `EditMessage`, `EditWebhookMessage`, `ExecuteWebhook` and
  `CreateFollowupMessage` get `content`, `embed` / `embeds`, `component` / `components`,
  `allowed_mentions`, `no_mentions`, `flags`, `suppress_embeds`, `file` / `files`,
  `attachments`, `poll`, `tts` where the field exists, `ephemeral` on the interaction bodies,
  `suppress_notifications`, and setters for the fields only one body has (`reply_to`,
  `username`, `thread_id`, …). Every body converts from `&str`, `String` and `CompactString`
  (`EditMessage` had none, `EditWebhookMessage` only `&str`): an ephemeral reply is
  `MessageCallbackData::from("Done").ephemeral()`. The field types do not change.
- A comparison harness against serenity 0.12.5 with its default configuration
  (`benches/vs-serenity/`, its own workspace and `Cargo.lock`, D-277–D-282, C-108):
  performance runners (decode, gateway flood, memory, REST) reusing the `benches/vs-node`
  mocks, serenity reaching them through an HTTPS mock with a test CA; resources (`build.sh`
  builds two identical `!ping` bots for build times, binary sizes and dependencies);
  reliability (`reliability.sh` with a failing gateway — dropped connection, `RECONNECT`,
  `INVALID_SESSION`, a zombie connection, close codes 4000 and 4004 — and a failing API —
  route and global 429s, 502s). Every run happens in a network namespace without a route
  out. `benches/vs-node` gains `GET /gateway` in its mock REST server, the shared
  `metrics::Window`, `serenity_cache()` and `--cache serenity`.
- `docs/VS_SERENITY.md` (linked from the main README): the results of that comparison, with
  tables and SVG charts generated by `benches/vs-serenity/report.mjs` from the committed
  `benches/vs-serenity/results.json`, the method and the caveats, including where serenity
  does better (incremental rebuilds). The chart helpers of `benches/vs-node` move to a shared
  `node/charts.mjs`.
- Example bots in `djs-rs/examples/` (Phase 8 step 6, D-274–D-276, C-107):
  `slash_commands` (registration, the router, subcommands, autocomplete, a modal, ephemeral
  and deferred replies), `components` (a paginated menu, a quiz, a reaction poll with
  collectors), `moderation` (logs with the cached previous state, moderator-only slash
  commands, `fetch_all`) and `advanced` (automatic sharding, presence, the event stream,
  sequential handlers, `WebhookClient`, mimalloc), sharing a small `examples/common` module
  with `ping`; `.env.example` gains the optional `DISCORD_TEST_GUILD_ID` and
  `DISCORD_WEBHOOK_URL`.
- Collectors in `djs-rs` (Phase 8 step 4a, D-263–D-269, C-103): `djs_rs::collector` with
  `Collector<T>` streams and `CollectorBuilder` (`time`, `idle`, `max`, `max_processed`,
  `filter`; `max_users`, `max_emojis`, `dispose`, `component_type` where discord.js has
  them), ending on deletions, `stop()` and shutdown (`EndReason`). Entry points:
  `message.reaction_collector` / `await_reactions` / `component_collector` /
  `await_component`, `channel.message_collector` / `await_messages` / …, the same on the
  `Context` by channel ID, and `interaction.component_collector` / `await_component` /
  `await_modal_submit`. Events reach them through an index by ID (D-264) and still reach the
  handlers and routes (D-266).
- Intents and partials guidance in `djs-rs` (Phase 8 step 5, D-271–D-273, C-106): every
  event's `Event::INTENTS`; a `tracing` warning at login (and for handlers or collectors
  added later) about events none of the client's intents covers;
  `Client::required_intents()`; a table from discord.js' partials to djs-rs in the `event`
  module documentation.
- Collector benchmarks and extreme-load tests (Phase 8 step 4c, C-105):
  `djs-rs/tests/stress_collectors.rs`; the collectors leave the event path unchanged until
  one wants the event (A/B within noise with 10 000 collectors alive), a busy collector
  costs ≈ 0.33 µs per message; results in `docs/PERFORMANCE.md`.
- Gateway requests in `djs-rs` (Phase 8 step 4b, D-270, C-104): `ctx.members(guild)
  .fetch_all()` / `.fetch_by_query(q)` / `.fetch_many(ids)` (`limit`, `presences`, `time`)
  gather the `GUILD_MEMBERS_CHUNK` answers of their nonce and cache the members;
  `ctx.guilds().fetch_soundboard_sounds(ids, time)`; `manager::GatewayRequestError`.
- Interaction benchmarks and extreme-load tests (Phase 8 step 3d, C-102):
  `djs-model/benches/interaction.rs` (getters 2–22 ns, no allocation; decoding a 1.9 KB
  interaction 3.8 µs), a gateway `INTERACTION_CREATE` flood, and
  `djs-rs/tests/stress_interactions.rs` (+ a mimalloc variant): the router costs ≈ 1 % over a
  `match` in an event handler; results in `docs/PERFORMANCE.md`.
- The interaction router in `djs-rs` (Phase 8 step 3c, D-251–D-255, C-101):
  `client.command("mod ban", …)` (full-path routes with fallback to the group and the
  command), `client.autocomplete(…)`, `client.component("vote", …)` and `client.modal(…)`
  (routed by the `custom_id` part before `:`, the rest as `args()`); handlers receive
  `CommandInteraction`, `AutocompleteInteraction`, `ComponentInteraction` or
  `ModalInteraction` (`djs_rs::router`), which dereference to the data and implement
  `InteractionExt`. `Events::InteractionCreate` still fires for every interaction.
- `InteractionExt` in `djs-rs` (Phase 8 step 3b, D-248, C-100), in the prelude: `reply`,
  `defer_reply`, `update`, `defer_update`, `show_modal`, `respond`, `launch_activity`,
  `fetch_reply`, `edit_reply`, `delete_reply`, `follow_up`, `fetch_follow_up`,
  `edit_follow_up`, `delete_follow_up` on `Interaction` and the `InteractionCreate` payload;
  the order Discord requires is documented on the trait.
- Interaction getters in `djs-model` (Phase 8 step 3a, D-249, D-250, C-099):
  `Interaction::{command, autocomplete, component, modal}`; `CommandData::{get,
  get_string, get_integer, get_number, get_boolean, get_user, get_member, get_role,
  get_channel, get_attachment, get_mentionable, subcommand, subcommand_group, leaf_options,
  focused_value, target_user, target_member, target_message}`; `ComponentData::{users,
  members, roles, channels, mentionables}`; `ModalSubmitData::{string_select_values,
  checkbox_group_values, radio_group_value, checkbox_value, selected_users,
  selected_members, selected_roles, selected_channels, selected_mentionables,
  uploaded_files}`; the `Mentionable` type.
- Manager benchmarks (Phase 8 step 2d, C-098): `djs-rs/benches/managers.rs` (a cache hit
  through a manager costs ≈ 1 ns over reading the cache and allocates nothing),
  `djs-cache` `store` benches and `djs-rs/tests/stress_managers.rs` (cache hits on every
  core, REST floods, 200 000 members stored from REST); results in `docs/PERFORMANCE.md`.
- `djs-rs` application commands, auto moderation and the audit log (Phase 8 step 2c batch 4,
  D-244–D-247, C-097): `ctx.application_id()` from `READY`; `ctx.commands()` and
  `ctx.guild_commands(guild)` (fetch, list, create, edit, delete, set; command permissions in
  a guild); `ctx.application_emojis()`; `ctx.auto_moderation_rules(guild)`;
  `ctx.guilds().audit_log(guild, …)`, `integrations` and `delete_integration`;
  `ctx.sticker(id)`, `ctx.sticker_packs()` and `ctx.sticker_pack(id)`. `GuildExt` gained
  `commands`, `auto_moderation_rules`, `audit_log` and `integrations`.
- `djs-rs` threads, scheduled events, stages and the soundboard (Phase 8 step 2c batch 3,
  D-242, D-243, C-096): `ctx.threads(parent)` (create, create in a forum, archived lists),
  `ctx.thread_members(thread)` (fetch, list, add, remove, join, leave),
  `ctx.guild_channels(guild).list_active_threads()`, `ctx.scheduled_events(guild)` (with
  user counts and interested users), `ctx.stage_instances(guild)` (by stage channel),
  `ctx.soundboard_sounds(guild)`, `ctx.default_soundboard_sounds()` and
  `send_soundboard_sound`; lists of threads come as `FetchedThreads`. `djs-cache`:
  `store_scheduled_event`, `store_stage_instance`, `store_soundboard_sound`.
- `djs-rs` emojis, stickers, invites and webhooks (Phase 8 step 2c batch 2, D-237–D-241,
  C-095): `ctx.emojis(guild)` and `ctx.stickers(guild)` (cached, like roles),
  `ctx.invites()` (by code, plus create and list for channels and guilds) and
  `ctx.webhooks()` (by ID, plus create and list; sending and the webhook's messages with its
  token). `ChannelExt::create_webhook` / `webhooks` / `create_invite` / `invites`,
  `GuildExt::emojis` / `stickers` / `webhooks` / `invites` and the new `WebhookExt`
  (`send`, `edit`, `delete`; `WebhookError::MissingToken` for webhooks without a token).
  `WebhookClient` sends through a webhook URL without a bot (`from_url`, `send`, its
  messages, `edit`, `delete`). `djs-cache`: `Cache::store_emoji` and `store_sticker`.
- `djs-rs` manager writes (Phase 8 step 2c batch 1, D-234–D-236, C-094): channels edited,
  deleted and given permission overwrites for a role or a member (`OverwriteTarget`); a
  guild's channels listed, created and reordered through `ctx.guild_channels(guild)` /
  `guild.channels(&ctx)`; roles created, edited, deleted and reordered; guilds edited and
  left; members searched and pruned, the bot's own membership edited; reaction users listed
  and reactions removed. Deletions leave the cache to the gateway event, so the delete event
  still carries the previous state.
- `djs-rs` managers and structure methods (Phase 8 step 2b, D-229–D-233, C-093):
  `ctx.guilds()`, `channels()`, `users()`, `members(guild)`, `messages(channel)`,
  `roles(guild)` and `bans(guild)` in `djs_rs::manager`. `fetch(id)` reads the cache first
  and caches what Discord returns (`.force()` skips the cache, `.is_cached()` tells
  beforehand); lists are `Collection`s in Discord's order; members can be edited, timed out,
  renamed, given and stripped of roles, kicked and banned; messages sent, edited, deleted,
  bulk-deleted, reacted to, pinned, crossposted and turned into threads. The extension traits
  `MessageExt`, `ChannelExt`, `UserExt`, `MemberExt` and `GuildExt` (in the prelude) offer
  the same on the model types, the cached types and the event payloads.
- `djs-cache` REST write API (Phase 8 step 2a, D-230, D-233): `Cache::store_user`,
  `store_member`, `store_message`, `store_channel`, `store_role` and `store_guild` convert an
  object received from the REST API to its cached form, store it when the resource is
  enabled, and return the stored `Arc`.
- `djs-rs` event benchmarks (Phase 8 step 1b, C-092): `MESSAGE_UPDATE` floods with the
  previous state, a `Raw` handler and unwatched events in `tests/stress.rs`; an A/B against
  the start of Phase 8 in `docs/PERFORMANCE.md` (−2.4 % on the sequential flood).
- `djs-rs` events (Phase 8 step 1, D-219–D-228, C-091): `Events` covers every discord.js
  event and every other gateway dispatch, the shard lifecycle (`ShardReady`,
  `ShardReconnecting`, `ShardResume` with the replayed count, `Invalidated`, `ShardError`,
  `ShardDisconnect`) and `Raw`. Updates and removals carry the cached state before the event
  (`old`) next to Discord's data (`new`), dereferencing to `new`; `GuildCreate` /
  `GuildAvailable` / `GuildUnavailable` / `GuildDelete` are told apart with or without a
  cache; emoji and sticker lists are diffed into per-item events when cached. `ClientReady`
  fires once per login. Payloads are only built for events someone listens to.

### Changed
- Every REST method of `djs-rest` (`Rest::create_message`, …, 245 of them) returns
  `impl Future<Output = …> + Send` built inside the crate (D-287), instead of being an
  `async fn`: their bodies — sending, rate limiting, retries, hyper, serde for the model types
  — are compiled once in `djs-rest` rather than in every crate that awaits them. Calls do not
  change (`rest.create_message(id, &body).await`); a method's borrowed arguments now share
  one lifetime with `&self`. `Rest::request` keeps decoding the caller's `T` in the caller's
  crate but sends through the crate-built path. Each call costs one allocation (REST runner:
  the same throughput, +0.27 µs CPU per request); a `!ping` bot that replies rebuilds in
  6.4 s instead of 7.8 s after an edit.
- `Client::login` returns `impl Future<Output = Result<(), Error>> + Send` instead of being
  an `async fn`, and `Client::builder` / `ClientBuilder::build` reach the client's code
  through a table of functions (D-287): for the default cache backend that code — the event
  loop, the shards, the cache updates — is compiled once in `djs-rs`, not in the bot's crate;
  `ClientBuilder::cache_backend` switches to the generic table, so custom backends work as
  before. Calls do not change. A `!ping` bot rebuilds after an edit in 0.5 s (0.85 s when it
  replies) instead of 7.1 s (7.8 s), release profile.
- The managers, the structure methods and `Context` reach the cache through one
  `dyn` table of cache operations built where the client loop creates the context (D-289),
  and the gateway member and soundboard requests box their futures inside `djs-rs`: for the
  default backend none of that code is compiled in the bot's crate any more. No public API
  change. With the default release profile, a `!ping` bot that replies rebuilds after an edit
  in 0.77 s (10.8 s before D-287) and the moderation example in 2.1 s (12.7 s); event and
  REST throughput are unchanged (`docs/PERFORMANCE.md`, "Rebuild time after an edit").
- **Breaking:** the six maps of interaction `Resolved` data box their values
  (`BTreeMap<Id, Box<V>>`, D-256): an interaction takes 3.14 KB of heap instead of 8.72 KB
  at the same decoding time. Field access and method calls work through the box; the getters
  (`get_user`, `selected_roles`, …) still return plain references.
- The 16-shard reply flood of `djs-rs/tests/stress_interactions.rs` sends 50 000 interactions
  instead of 200 000 (D-258).
- `djs-rs` (breaking): `MessageExt::reply` returns the sent message as cached,
  `Arc<CachedMessage>`, instead of `Message` (C-093).
- `djs-rs`: `Dispatch` has one variant per event (replacing `Ready` / `MessageCreate` /
  `Resumed`), `Dispatch::name` returns discord.js' event name (`"messageCreate"`), and
  `ClientReady` fires once instead of at every shard's `READY` (`ShardReady` does that).
- `djs-collection` benchmarks (`benches/collection.rs`) and extreme-load tests
  (`tests/stress.rs`: a model check of random operations, fairness of random picks,
  10 000 000 entries, adversarial removal patterns, all cores), results in
  `docs/PERFORMANCE.md` (Phase 7 step 2b, C-090). `random_n` is 3–4× faster (one seed per
  call and a wyrand generator) and `partition` 1.7× faster (`IndexMap::extract_if`).
- `djs-collection` (Phase 7 step 2, D-209, D-210, D-214–D-217, C-089): `Collection<K, V, S>`,
  an insertion-ordered map on `indexmap` hashing with foldhash, with the map basics
  (`remove` keeps the order, `swap_remove` is O(1)), discord.js' helpers (`first`, `last`,
  `at`, `find`, `random`, `random_n`, `ensure`, `sweep`, `sort_by`, `reverse`, `map`,
  `has_all`, `has_any`), consuming transformations (`filter`, `partition`, `map_values`,
  `flat_map`, `concat`, `union`, `intersection`, `difference`, `symmetric_difference`,
  `merge`, `combine_entries`), own iterator types and access to the `IndexMap`. Re-exported
  as `djs_rs::collection`.
- `djs-formatters` benchmarks (`benches/formatters.rs`) and extreme-load tests
  (`tests/stress.rs`: randomized invariants, adversarial megabytes, all cores), results in
  `docs/PERFORMANCE.md` (Phase 7 step 1b, C-088). Escaping plain text is 3.9× faster
  (2.4 GB/s), and a long run of backticks with code content skipped no longer takes
  quadratic time.
- `djs-formatters` (Phase 7 step 1, D-211–D-213, C-087): mentions (user, channel, role,
  linked role, slash command with subcommand or group), custom emoji, guild navigation,
  timestamps with every `TimestampStyle`, markdown (bold, italic, underline,
  strikethrough, spoiler, inline code, code blocks with a language, quotes, headings, subtext,
  nested lists, hyperlinks with titles, hidden embeds), channel / message / App Directory
  links, e-mail and phone links, faces, and `escape_markdown` with discord.js' options plus
  every single escaper. Formatters return `Display` values (no allocation until written)
  and take `djs-model` IDs; escaping is one pass returning `Cow<str>`, leaving links and
  mentions intact. Re-exported as `djs_rs::formatters`.
- `djs-builders` benchmarks (`benches/builders.rs`) and extreme-load tests
  (`tests/stress.rs`), results in `docs/PERFORMANCE.md` (Phase 6 step 4, C-086). Checking a
  message's components no longer allocates and finds repeated `custom_id`s and option values
  35–43 % faster; `validate_components` rejects a huge Components V2 list in milliseconds
  instead of minutes.
- `djs-builders` modal builders (Phase 6 step 3c, D-206, D-207, C-085):
  `ModalBuilder::new(custom_id, title)` with `.label(text, field)`, `.text(…)` and
  `.component(LabelBuilder…)`, `TextInputBuilder::{short, paragraph}`, `FileUploadBuilder`,
  `RadioGroupBuilder` / `CheckboxGroupBuilder` with `.option(label, value)` and option
  builders, and `CheckboxBuilder`. `build()` returns `ModalCallbackData` after checking the
  documented rules (title, 1-5 labels or texts, label texts, text input lengths, 0-10 files,
  2-10 radio and 1-10 checkbox options, enabled select menus, unique `custom_id`s).

- `djs-builders` Components V2 builders (Phase 6 step 3b, D-204, D-205, C-084):
  `ContainerBuilder`, `SectionBuilder::new(accessory)`, `TextDisplayBuilder`,
  `ThumbnailBuilder`, `MediaGalleryBuilder` with `MediaGalleryItemBuilder`, `FileBuilder` and
  `SeparatorBuilder`, with `.text("…")` shortcuts on containers and sections. The rules were
  measured on the live API: 1-40 components in a container, 1-3 texts in a section, 1-10
  gallery items, descriptions of 1-1024 code points, `attachment://` for files.
- `djs-builders` message component builders (Phase 6 step 3a, D-197–D-203, C-083):
  `ButtonBuilder::{primary, secondary, success, danger, link}` and `PremiumButtonBuilder`,
  `StringSelectMenuBuilder` with `StringSelectMenuOptionBuilder`, the user, role,
  mentionable and channel select menus, `ActionRowBuilder`, and
  `validate_components(&[Component], ComponentsVersion)` for a whole message. The rules follow
  what Discord accepted and rejected on the live API (code points, untrimmed `custom_id`s,
  label or emoji, `http`/`https`/`discord` links, unique `custom_id`s and option values,
  `min_values` ≤ `max_values` ≤ options, 40 components and 4000 characters for Components V2).
  `ValidationError` gained `TooFew`.
- `djs-builders` command builders (Phase 6 step 2, D-192–D-196, C-082):
  `SlashCommandBuilder`, `SlashCommandSubcommandBuilder`,
  `SlashCommandSubcommandGroupBuilder`, nine `SlashCommand…Option` types passed to one
  `option(…)`, and `ContextMenuCommandBuilder::{user, message}`, with translations, default
  member permissions, contexts, integration types and NSFW. `build()` checks Discord's rules
  as measured on the live API — the name pattern, descriptions in trimmed code points, unique
  names, required options first, no mixing of subcommands with options, `min_length` /
  `max_length`, the 8000-character total with the longest translations — and rejects
  `max_value` below `min_value` and choices with autocomplete. `ValidationError` gained
  `InvalidName` and `Invalid`; `djs-model` gained `new(name, description)` for the option
  types.
- `djs-builders` `EmbedBuilder` (Phase 6 step 1, D-178–D-191, C-081): consuming setters
  for every part, `EmbedAuthorBuilder` / `EmbedFooterBuilder` (or a plain text),
  `EmbedBuilder::from(embed)`, `build()` → `Result<Embed, ValidationError>` and
  `build_unchecked()`. `build()` checks the limits the way Discord applies them, as measured
  with a test bot: lengths in code points after trimming, URL schemes, the 6000-character
  total and that the embed has content; `embed_length` and `validate_embeds` check a whole
  message. `djs_builders` is re-exported as `djs_rs::builders`; `djs-model` gained
  constructors for `EmbedField`, `EmbedAuthor`, `EmbedFooter` and `EmbedImage`.
- `docs/VS_NODE.md` re-measured with equal caches (D-174): djs-rs with
  `CacheConfig::discord_js()` handles 1.9× the events of discord.js and needs 2.5–4.6× less
  memory for the same cached guilds (391–413 vs 843–1,019 B per member); the memory section
  uses the real cache instead of the `HashMap` stand-in, and its claim that a cache could
  only need the same or less is corrected. The flood harness no longer calls the real
  Discord API (a fixed shard skips Get Gateway Bot).
- `djs-cache` applies `TYPING_START` (its member), `INTERACTION_CREATE` (its user and member,
  the resolved users and members) and `THREAD_MEMBERS_UPDATE` (the thread's member count,
  the added members) like discord.js (D-173, C-079).
- `djs-cache` benchmarks (`benches/cache.rs`: every hot event, reads, sweeps, foldhash vs
  SipHash) and extreme-load tests (`tests/stress.rs`: 500 000 members, concurrent readers
  and writers, bounded message churn, sweeping under load, guild churn, type sizes); results
  in `docs/PERFORMANCE.md` (Phase 5 step 6). Updates of entries a guild already has now take
  its index entry shared instead of exclusively, so writers of one guild no longer queue
  behind each other (C-078).
- djs-rs `Client` cache (Phase 5 step 5, D-170–D-172): `ClientBuilder::cache(CacheConfig)`
  (no cache by default), `Client::cache()` / `Context::cache()` → `Option<&Cache<B>>`, every
  event applied to the cache before its handlers run, the configured sweeps run while logged
  in, and `ClientBuilder::cache_backend` for a custom storage backend
  (`Client<B = DashMapBackend>`, `Context<B>`). `djs_cache` is re-exported as
  `djs_rs::cache`.
- `djs-cache` sweeping (Phase 5 step 4, D-167–D-169): a `Cache::sweep_*` method with a
  filter for every resource, the presets `sweep_messages_older_than` and
  `sweep_archived_threads`, and `SweepConfig` / `Sweep` in `CacheConfig` (default: threads
  archived over 4 h, hourly, like discord.js) for the djs-rs `Client` to run.
- `djs-cache` messages, presences and the remaining guild resources (Phase 5 step 3,
  D-164–D-166): `CachedMessage` (per channel, `cache.messages(channel)`, author shared with
  users; reactions kept up to date by the reaction events), `CachedPresence`
  (`cache.presences(guild)`), voice states, threads, scheduled events, stage instances and
  soundboard sounds, each with its events and previous state.
- `djs_model::message::Reaction::new(emoji)`, a reaction with zero counts.
- `djs-cache` resources (Phase 5 step 2, D-160–D-163): guilds (`CachedGuild`), channels,
  roles, emojis, stickers, members (`CachedMember`, sharing its `Arc<User>`), users and the
  current user, kept up to date by `Cache::update(&payload)` for `READY`, `USER_UPDATE`, the
  guild, channel, pins, role, emoji, sticker and member events (each returning the previous
  state, typed per event through `UpdateCache`) and by `Cache::update_dispatch`. Read through
  views: `cache.guilds().get(id)`, `cache.channels().in_guild(id)`,
  `cache.members(guild).values()`, ….
- `djs-cache` core (Phase 5, D-151–D-159): `CacheConfig` with one `Limit` per `Resource`
  (`Disabled`, `Max(n)`, `Unlimited`; minimal default of guilds, channels and roles;
  `CacheConfig::discord_js()` preset), the pluggable `Store` / `Backend` traits with the
  default `DashMapBackend` (`dashmap` + `foldhash`), and `Cache<B = DashMapBackend>`. Cached
  resources follow in the next steps.
- `zlib-stream` transport compression (feature `zlib-stream` in djs-gateway and djs-rs,
  D-140): `Compression::ZlibStream`, decoded with `flate2` + the pure-Rust `zlib-rs`,
  including payloads split over several WebSocket messages. Stress-tested against zstd
  (`docs/PERFORMANCE.md`, Phase 4).
- Presence (D-148, D-149): `ConfigBuilder::presence` (sent in every IDENTIFY),
  `MessageSender::update_presence` (also kept for later sessions), `ClientBuilder::presence`,
  `Client::set_presence` (every shard), and `Activity::{playing, streaming, listening,
  watching, competing, custom}`.
- Sharding (D-141, D-144–D-147): `djs_gateway::IdentifyQueue` (trait) and
  `LocalIdentifyQueue` (one IDENTIFY per `max_concurrency` bucket every 5 s), shared through
  `ConfigBuilder::identify_queue`; `djs_gateway::create_shards(range, total, config)`. The
  djs-rs `Client` now runs one task per shard, with Discord's recommended count by default
  (`ClientBuilder::shards(Shards::Auto | Count | Range)`), and `login` fails with
  `Error::SessionStartLimit` when too few session starts are left.
- The gateway send limit (D-142): a shard sends at most 115 queued commands in any 61 s
  (Discord allows 120 per 60 s and disconnects clients that send more); the rest wait in the
  queue, in order, and heartbeats, identify and resume keep the remaining room. A new
  connection starts with a fresh limit.
- Phase 3 benchmarks and extreme-load tests (D-041): `djs-model/benches/encode.rs` (request
  bodies, base64 images and sounds, target users CSV), `djs-rest/benches/request.rs` (routes,
  requests, CDN and authorization URLs) and multipart upload stress tests; results in
  `docs/PERFORMANCE.md`.
- `djs_rest::cdn` (D-137–D-139): `Cdn` builds the URLs of every CDN endpoint (avatars,
  banners, icons, splashes, emojis, stickers, avatar decorations, app assets, role icons,
  event covers, server tag badges, group DM icons, soundboard sounds), with `ImageOptions`
  (`ImageFormat`, `ImageSize`, `force_static`); animated images default to animated WebP.
  `Rest::cdn()`, `RestBuilder::{cdn, media_proxy}`, `default_avatar_index` and
  `Rest::guild_widget_image_url` (D-116).
- Application identity profile endpoints (game stats): update (returns the profile on the
  first write, D-135) and get a profile, list identities by user or external ID, delete an
  identity. Bodies `UpdateApplicationIdentityProfile`, `DeleteApplicationIdentity`, query
  `GetApplicationIdentitiesByExternalId`.
- `get_gateway` with the new `djs_model::rest::Gateway` (D-136).
- `scripts/route_coverage.py` lists documented REST routes without a `Route`.
- Voice endpoints: `list_voice_regions`, `get_current_user_voice_state`,
  `get_user_voice_state`, `modify_current_user_voice_state` and `modify_user_voice_state`
  (bodies `ModifyCurrentUserVoiceState`, `ModifyUserVoiceState`).
- Lobby endpoints (16 methods, Discord Social SDK): create, create-or-join, get, modify and
  delete lobbies; add, bulk-update, remove members and leave; link and unlink a channel;
  channel invites for oneself or a member; send and read messages and set their moderation
  metadata. Bodies `CreateLobby`, `CreateOrJoinLobby` (secret redacted in `Debug`),
  `ModifyLobby`, `LobbyMemberData`, `AddLobbyMember`, `BulkLobbyMember` (D-134),
  `SendLobbyMessage`, query `GetLobbyMessages`, alias `LobbyMetadata`.
- `OAuth2` (D-131–D-133): `get_current_bot_application_information`,
  `get_current_authorization_information`, and the token endpoints `exchange_code`,
  `refresh_access_token`, `client_credentials_grant` and `revoke_token`, which take
  `djs_rest::oauth2::OAuth2Credentials` (secret redacted in `Debug`), send
  `application/x-www-form-urlencoded` bodies and never the client's token. The
  `djs_rest::oauth2::AuthorizeUrl` builder makes bot invite and authorization code URLs.
  `djs_model::oauth2::{AccessTokenResponse, CurrentAuthorizationInformation}` (tokens
  redacted in `Debug`).
- Application role connection metadata endpoints:
  `get_application_role_connection_metadata_records` and
  `update_application_role_connection_metadata_records`;
  `ApplicationRoleConnectionMetadata::new`.
- Application endpoints: `get_current_application`, `edit_current_application` (body
  `EditCurrentApplication`) and `get_application_activity_instance`;
  `InstallParams::new`.
- Monetization endpoints (9 methods): `list_skus`; `list_entitlements` (with the
  `ListEntitlements` query), `get_entitlement`, `consume_entitlement`,
  `create_test_entitlement` (`CreateTestEntitlement::guild` / `::user`, D-130) and
  `delete_test_entitlement`; `list_sku_subscriptions` (with the `ListSkuSubscriptions`
  query) and `get_sku_subscription`. New `entitlement::EntitlementOwnerType`.
- Poll endpoints: `get_answer_voters` (with the `GetAnswerVoters` query; returns the users
  unwrapped from `{"users": [...]}`) and `end_poll`. Routes accept `u32` path parameters
  (the poll's `answer_id`).
- Auto Moderation endpoints (5 methods): list, get, create, modify and delete rules. Bodies
  `CreateAutoModerationRule` (one constructor per trigger type, D-128) and
  `ModifyAutoModerationRule`; `auto_moderation::Action` gets one constructor per action type
  (D-129).
- `get_guild_audit_log` with the `GetGuildAuditLog` query (user, action type, `before` /
  `after`, limit).
- Guild template endpoints (6 methods): get a template by code, list a guild's templates,
  create, sync, modify and delete; bodies `CreateGuildTemplate` and `ModifyGuildTemplate`.
- Invite endpoints (9 methods): `get_invite` (with counts and a scheduled event),
  `delete_invite`, and the target users of restricted invites — `get_target_users` parses
  Discord's CSV into user IDs (D-126), add and remove one user, bulk add and delete
  (`BulkTargetUsers`), `update_target_users` uploads a CSV file built with
  `FileUpload::target_users` (D-127), and `get_target_users_job_status`
  (`InviteTargetUsersJobStatus` in `djs_model::invite`).
- Stage instance endpoints (4 methods): create, get, modify and delete the open stage of a
  stage channel; bodies `CreateStageInstance` and `ModifyStageInstance`.
- Guild scheduled event endpoints (6 methods): list (with user counts), create, get, modify
  (including starting, ending and canceling through `status`), delete, and the paginated
  subscribers. `CreateGuildScheduledEvent` has a constructor per place — `stage`, `voice`,
  `external` (D-125); recurrence rules are sent as `RecurrenceRuleData` with constructors for
  the shapes Discord accepts (D-124). Queries `ListScheduledEventsForGuild`,
  `GetGuildScheduledEvent`, `GetGuildScheduledEventUsers`; constructors
  `EntityMetadata::new` and `RecurrenceNWeekday::new`.
- Soundboard endpoints (7 methods): `send_soundboard_sound` in a voice channel, the default
  sounds, and list (unwrapped from `{"items": [...]}`), get, create, modify and delete guild
  sounds. Bodies `SendSoundboardSound`, `CreateGuildSoundboardSound` (a `SoundData` sound,
  D-122) and `ModifyGuildSoundboardSound`.
- Sticker endpoints (8 methods): get any sticker, list and get Discord's sticker packs
  (`list_sticker_packs` unwraps `{"sticker_packs": [...]}`), and list, get, create, modify and
  delete guild stickers. `create_guild_sticker` sends `CreateGuildSticker` as
  `multipart/form-data` fields with the file, as Discord requires; `ModifyGuildSticker` is
  JSON.
- Emoji endpoints (10 methods): list, get, create, modify and delete guild emojis (with audit
  log reasons) and application emojis; `list_application_emojis` unwraps Discord's
  `{"items": [...]}` into a `Vec<Emoji>`. Bodies `CreateGuildEmoji`, `ModifyGuildEmoji`,
  `CreateApplicationEmoji` and `ModifyApplicationEmoji` in `djs_model::rest`.
- `docs/VS_NODE.md`: a reproducible benchmark showcase against discord.js 14 on Node.js
  (D-123) — decoding, an event flood through a mock gateway, the memory of cached guilds and
  REST throughput — with tables and light/dark SVG charts generated from the measured
  results. The harness (`benches/vs-node/`: the `djs-vs-node` workspace crate, Node scripts
  with pinned npm packages, `run.sh`) re-runs everything and regenerates the page.
- `djs-model`: `util::SoundData` (D-122) for sounds sent as data URIs (MP3/Ogg detected from
  the file signature), sharing `ImageData`'s streaming base64 encoder.
- Webhook endpoints (14 methods): create, list (channel, guild), get, modify and delete
  webhooks (also with the webhook's token), `execute_webhook` and `execute_webhook_and_wait`
  (D-119; bodies `CreateWebhook`, `ModifyWebhook`, `ExecuteWebhook` with files, threads and
  forum posts), get/edit/delete webhook messages. Slack/GitHub-compatible execution is not
  modelled (D-120).
- User endpoints (12 methods): current user (get, modify), get user, current user guilds
  (`CurrentUserGuild`, with `shard` for large bot sharding), leave guild, create DM / group
  DM, and the `OAuth2` (bearer token) endpoints: current user guild member, connections and
  application role connections.
- Guild endpoints (45 methods): get/preview/modify guild, guild channels (create, positions,
  active threads), members (get, list, search, permissions, add with an `OAuth2` token,
  modify, modify current member, roles, kick), bans (incl. bulk ban), roles (incl.
  permissions, member counts, positions), prune, voice regions, invites, integrations,
  widget settings and widget, vanity URL, welcome screen, onboarding (`OnboardingPromptData`
  with generated prompt IDs, D-118) and incident actions. Deprecated Modify Current User Nick
  and request fields are left out; the widget image will be a URL (D-116).
- `djs-model`: constructors `Overwrite::{new, role, member}`, `DefaultReaction::{custom,
  unicode}`, `WelcomeScreenChannel::new` (these types could not be built outside the crate).
- Channel and thread endpoints (28 methods): get, modify (`modify_channel`,
  `modify_thread`, `modify_group_dm` with `ModifyGuildChannel`, `ModifyThread`,
  `ModifyGroupDm`, D-109) and delete channels; computed member/role permissions (returned as
  `Permissions`, D-110); voice channel status; permission overwrites; invites (with
  `target_user_ids` or a CSV `target_users_file`, D-111); following announcement channels;
  typing; group DM recipients; starting threads from a message, without one, or as a forum
  post (`ForumThreadMessage`, D-112, returning `ForumThread` with the starter message,
  D-115); thread members; archived thread lists (`ArchivedThreads`). New request types
  include `ForumTagData` (D-114), `EditChannelPermissions`, `CreateChannelInvite` and the
  thread queries.
- `djs-model`: `util::ImageData` (D-113) for images sent in request bodies, serialized as a
  `data:` URI by a streaming base64 encoder; the media type is detected from the file
  signature (PNG, JPEG, GIF, WebP, AVIF) or given.
- Application command endpoints: get/create/edit/delete and bulk overwrite of global and
  guild commands, and their permissions (get all in a guild, get and edit one), with the
  bodies `CreateApplicationCommand` (`chat_input`/`user`/`message` constructors),
  `EditApplicationCommand`, `EditApplicationCommandPermissions` and the query
  `GetApplicationCommands`.
- `djs-rest`: `RestBuilder::bearer_token` for clients that act with a user's OAuth2 access
  token (D-108), e.g. to edit command permissions.
- Interaction endpoints: `create_interaction_response` and
  `create_interaction_response_with_response` (D-106), get/edit/delete of the original
  response, and create/get/edit/delete of follow-up messages, with the bodies
  `CreateFollowupMessage` (D-107) and `EditWebhookMessage`.
- Message endpoints (D-102): get/create/edit/delete/crosspost messages, bulk delete, pins
  (get, pin, unpin) and reactions (create, delete own/user/all/all for emoji, get) as `Rest`
  methods, with `djs_rest::ReactionEmoji` (D-103) and the bodies/queries `EditMessage`,
  `GetChannelMessages`, `GetReactions`, `BulkDeleteMessages`, `GetChannelPins`,
  `ChannelPins`; `CreateMessage` covers every documented field. Search Guild Messages
  (`SearchGuildMessages`, `SearchFilter`, `MessageSearchResults`) with
  `ErrorKind::NotYetIndexed` while the guild is not indexed (D-104).
- `djs-model`: `util::Patch<T>` (`Keep`/`Clear`/`Set`) for request fields that can be left
  unchanged, cleared or set (D-083, D-101).
- `native-roots` feature of `djs-rest`, `djs-gateway` and `djs-rs` (D-088, D-100): also trust
  the operating system's root certificates, in addition to the bundled Mozilla roots.
- API errors (D-097..D-099): `djs_model::rest::ErrorCode` with every documented JSON error
  code, and `djs_rest::FieldError` — `ApiError::field_errors` lists the form-body errors
  (`embeds[0].title[BASE_TYPE_REQUIRED]: …`), which the error's `Display` shows like
  discord.js.
- File uploads (D-087, D-093..D-096): `djs_model::rest::FileUpload` (data as `bytes::Bytes`),
  `files` fields on `CreateMessage` and `MessageCallbackData` whose `attachments` entries are
  generated automatically, `InteractionResponse::files`, and `djs_rest::Request::files`, which
  sends `multipart/form-data` without copying the file data (content type guessed from the
  file name when not given).
- `djs-rest`: `Route<'a>` can borrow tokens (D-090); interaction routes
  `CreateInteractionResponse`, `{Get,Edit,Delete}OriginalInteractionResponse`,
  `{Create,Get,Edit,Delete}FollowupMessage`, and `DeleteMessage` (messages older than two
  weeks get their own rate-limit bucket). Interaction endpoints skip the global rate limit
  and callbacks are never queued behind one another; tokens are percent-encoded in paths,
  redacted in `Debug` and never logged.
- `djs-rest`: `Request::reason` sets the audit log reason (`X-Audit-Log-Reason`, sent
  percent-encoded, D-086).
- `djs-rest`: invalid-request counter `Rest::invalid_requests()` with warnings and an opt-in
  fail-fast limit `RestBuilder::invalid_request_limit` (`ErrorKind::InvalidRequestLimit`,
  D-085).
- `djs-rs`: README section on the global allocator (mimalloc for very large bots, D-080).
- `djs-model`: `webhook_event` module (D-076) — `WebhookEventPayload` (`PayloadType::{Ping,
  Event}`), `WebhookEvent` with `WebhookEventData` by type, `ApplicationAuthorized`,
  `ApplicationDeauthorized`, `LobbyMessageDelete`, `GameDirectMessage` (D-079); event times
  without an offset are read as UTC.
- `djs-model`: every gateway dispatch event is routed to a typed `DispatchEvent` variant with
  its own payload type in `gateway::payload` (D-073), e.g. `GuildCreate` (available or
  unavailable, `AvailableGuild`), `ThreadCreate`, `GuildMemberAdd`, `PresenceUpdate`,
  `VoiceServerUpdate` (token redacted in `Debug`), `TypingStart`, `ChannelInfo`;
  `DispatchEvent` implements `Serialize`. Presences: `gateway::{Presence, PresenceUser (D-074),
  ClientStatus, Activity, ActivityType, ActivityTimestamps, ActivityEmoji, ActivityParty,
  ActivityAssets, ActivitySecrets (redacted in `Debug`), StatusDisplayType}`; Unix times are
  `Timestamp`s (D-075). Gateway commands `RequestGuildMembers`, `RequestSoundboardSounds`,
  `RequestChannelInfo`, `UpdateVoiceState`, `UpdatePresence`; `message::ReactionType`,
  `guild::UnavailableGuild::new`.
- `djs-model`: `application_identity_profile::{ApplicationIdentity, ApplicationIdentityProfile,
  ProfileData, PrimaryProfileData, ProfileMedia, DynamicField, DynamicFieldValue,
  DynamicFieldType}` (D-072); `util::{JsonValue, JsonValueError}` for arbitrary JSON.
- `djs-model`: `soundboard::SoundboardSound`, `lobby::{Lobby, LobbyMember, LobbyMemberFlags,
  LobbyMessage, LobbyMessageMember, LobbyInvite}`.
- `djs-model`: `sku::{Sku, SkuType}`, `subscription::{Subscription, SubscriptionStatus}`,
  `voice::{VoiceState, VoiceRegion}`.
- `djs-model`: `audit_log` module — `AuditLog`, `AuditLogEntry`, `AuditLogEvent`,
  `AuditLogEntryOptions` (D-071), `AuditLogChange` with `util::JsonValue` values that convert
  to any field type (D-070, D-072); `guild::PartialIntegration::application_id`.
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
- `djs-builders`: an action row holding a text input is now rejected — text inputs belong in
  a modal's label (D-207); the deprecated format stays possible with `build_unchecked()`.
- **Breaking (`djs-cache`, D-177):** users are cached as the new compact `CachedUser`
  (`cache.users()`, `CachedMember::user`, `CachedMessage::author`), and `CachedMember` keeps
  its rarely set fields behind one box: `avatar`, `banner`, `premium_since`,
  `communication_disabled_until`, `avatar_decoration_data` and `collectibles` are methods
  now, and `roles` is a `Box<[RoleId]>`. Memory per cached member: 546 → 418 B (discord.js'
  defaults).
- A shard's own 5-second identify spacing is now done by its configuration's IDENTIFY queue.
- `ClientBuilder::gateway_url` no longer skips Get Gateway Bot on its own: it is skipped only
  together with a fixed shard count (`Shards::Count`/`Shards::Range`).
- `djs-rest`: `Request`'s `Debug` output shows only the size of the body and the names of
  form parts, as bodies can hold secrets (access tokens, client secrets; C-011). It used to
  print the serialized JSON body.
- `djs-model`: `GuildScheduledEventUser::member` is a `PartialMember`: Discord sends the
  subscriber's member data without its user, which is the `user` field (combine them with
  `PartialMember::into_member`). As a `Member` it could not be decoded.
- `djs-rest`: routes authenticated by a webhook or interaction token in their path are sent
  without the client's `Authorization` header (D-121), so webhooks work with a client built
  without a token.
- `djs-model`: `EditWebhookMessage` gained `thread_id` and `with_components`, sent in the
  query string (D-119), also by the interaction edit methods.
- `djs-model`: `InviteWithMetadata`'s metadata moved into `InviteMetadata`, now optional
  (`metadata: Option<InviteMetadata>`, D-117).
- `djs-rest`: `RestBuilder`'s `Debug` output redacts the token (it printed it before).
- `djs-model`: `CreateMessage` is `#[non_exhaustive]` and moved to `rest::message` (still
  re-exported as `djs_model::rest::CreateMessage`).
- `djs-rest`: `ApiError::code` is an `ErrorCode` instead of a `u32`; `ApiError` is
  `#[non_exhaustive]` and gained `field_errors`.
- `djs-model`: `CreateMessage` is no longer `Eq` (its files can carry an `f64` duration) and
  gained `attachments` and `files`; `MessageCallbackData` gained `files`.
- `djs-rest`: `Request` is `Request<'a>` and `Request::route` returns `Route<'a>` (D-090).
- `djs-rest`: the global rate limiter is strict (a sliding log, D-089): never more than the
  limit in any sliding second.
- `djs-model`: `WebhookEventType` moved from `application` to `webhook_event`;
  `lobby::LobbyMessage::application_id` is an `Option`, and `LobbyMessage` gained `timestamp`
  and `edited_timestamp` (D-078).
- `djs-model`: `Identify` gained `presence` and `capabilities` (`GatewayCapabilities`) fields.
- `djs-model`: `Message`, `MessageCreate`, `GatewayEvent` and `DispatchEvent` are `PartialEq`
  but no longer `Eq`, because messages contain floats (D-064).
- `djs-model`: received model structs are `#[non_exhaustive]` (D-053).
- MSRV raised from 1.85 to 1.89 (required by maintained `tokio-websockets` and `simd-json`).
