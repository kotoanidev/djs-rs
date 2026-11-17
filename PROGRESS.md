# Progress — resume point

> Updated at the end of every significant change. A fresh session must be able to continue
> from this file alone. Read `CLAUDE.md` and `docs/DECISIONS.md` first.

## Current phase
**Phase 1 — Vertical slice: ✅ done** (live test passed, benchmarks + extreme-load tests done,
D-041).
**Phase 2 — full `djs-model` coverage: ✅ done** (D-039; benchmarks + stress tests done).
**Phase 3 — full `djs-rest` coverage: ✅ done 2026-11-12** (every documented REST route, OAuth2,
CDN URLs; benchmarks + extreme-load tests in `docs/PERFORMANCE.md`).
**Phase 4 — full `djs-gateway` coverage: ✅ done 2026-11-12** (send limit, IDENTIFY queue +
sharding, presence, zlib-stream, JSON parsers measured; benchmarks + extreme-load tests and a
`perf`-profiled A/B in `docs/PERFORMANCE.md`). Left open on purpose: zero-copy parsing (no
measured need after D-150) and resharding while running (open question).
**Phase 5 — `djs-cache`: ✅ done 2026-11-12** (pluggable cache, every resource and event,
limits, sweeps, opt-in `Client` cache; benchmarks + extreme-load tests in
`docs/PERFORMANCE.md`, `docs/VS_NODE.md` re-measured with equal caches). Left open on purpose:
a lock-free backend (D-176), resolved roles/messages of interactions (open question).
**Phase 6 — `djs-builders`: ✅ done 2026-11-12** (validated builders for embeds, commands,
components incl. Components V2, and modals, with rules measured on the live API where Discord
allows it; benchmarks + extreme-load tests in `docs/PERFORMANCE.md`). Left open: removing
parts with the builders (open question), a container walked twice by `validate_components`.
**Phase 7 — small standalone crates: ✅ done 2026-11-12** (D-208–D-218; `djs-formatters` and
`djs-collection` built, benchmarked and stress-tested; `djs-util` stays empty until two crates
need a helper, D-218).
**Phase 8 — full `djs-rs` client: ✅ done 2026-11-13** (events with the cached old state,
structures with methods, managers, interactions and a router, collectors and gateway
requests, intents and partials guidance, example bots run on the test bot; benchmarks +
extreme-load tests in `docs/PERFORMANCE.md`).
**The serenity comparison: ✅ done 2026-11-13** (`docs/VS_SERENITY.md`, D-277–D-282, C-108,
C-109). The fixes found by the comparison (D-283) ✅: the rebuild time after an edit
(D-287–D-289, `docs/VS_SERENITY.md` re-measured) and the ergonomics gaps (D-290–D-295).
**Phase 9 — multi-process sharding: ✅ done 2026-11-14** (`djs_rs::sharding` behind the
`sharding` feature: processes of the same executable, IPC over `127.0.0.1`, the manager as
IDENTIFY queue, restarts, named commands and stats; benchmarks + extreme-load tests in
`docs/PERFORMANCE.md` "Phase 9"). **Phase 10 — `djs-voice` (D-017): in progress**, step 1
(connection, DAVE, Ogg/Opus, djs-rs's `voice` feature) done and live-tested 2026-11-16;
step 2 (the player) done and live-tested, its clicks fixed (C-127; a second listening test
later, the owner's choice); step 3 (receiving audio) done and live-tested (D-363–D-373, C-128; the playback and
tones rerun later, the owner's choice); step 4 (benchmarks and extreme-load tests) measured
2026-11-16 (`docs/PERFORMANCE.md`, "Step 4", C-129), its finding fixed (D-374, C-130).
**Phase 10 is done** (and live-tested end to end, 2026-11-16).
**The guide: in progress** (D-376–D-382, C-131): an mdBook in `book/` with its listings
compiled as examples of `book/code`, published with the API reference on GitHub Pages
(https://kotoanidev.github.io/djs-rs/, enabled 2026-11-16) by `.github/workflows/book.yml`.
The site went live 2026-11-16 (the Book workflow's first run). Written: introduction,
installation, the first bot, events and intents, slash commands. **Next: the "Components
and embeds" chapter** (embeds, buttons, select menus, modals, collectors), then the cache,
REST, sharding, voice (D-380), each with its example in `book/code/examples/`, built with `mdbook build book`
(mdBook 0.5.4) and its links checked against `target/doc`.

## Current plan (the owner's request of 2026-11-16, D-383–D-390), in this order
1. [x] Live voice test, 2026-11-16 (D-383): the bot recorded the owner (4.5 s of PCM, 272
   Opus packets, 3 lost), played it back, then step 2's tones (MP3 with a volume change,
   pause, resume and seek; PCM; Ogg at 0.5 then 1.0 and a seek): **the owner heard
   everything clean**, so the click fixes (C-127) and receiving hold live. The probe is
   deleted again.
2. [x] Coverage audit with `cargo-llvm-cov`, tests for the gaps (D-383, C-132): lines
   90.4 % → 93.5 %, functions 89.8 % → 93.0 %. New tests: every gateway dispatch into the
   client's events, the cache's views, every CDN path, the gateway config, OAuth2 URLs,
   JSON values, the managers' and structures' remaining routes, the collectors' limits and
   disposal, every channel kind's accessors, the model's custom deserializers, recurrence
   rules and builders. What stays low is ignored measurement code (codec, input, DAVE),
   macro output, and functions only doctests check (not counted on stable).
3. [x] Every benchmark and comparison measured again (D-384, D-385, 2026-11-16/17): bench-all
   (53 steps), A/Bs against `4085a57` and `0afab05` (no regression, C-135), divan pinned,
   `docs/PERFORMANCE.md` "Measured again for 0.1.0-alpha.1"; `VS_NODE.md`, `VS_SERENITY.md`
   (performance, builds, reliability) and the new `VS_TWILIGHT.md` re-run and their prose
   rewritten from the new numbers; the README's table covers all three. Logs in
   `.reference/bench/2026-11-16*`, raw results in `benches/vs-*/out/` (git-ignored).
   After the measurements the local toolchain went to stable 1.100 (CI's).
4. [ ] Documentation (D-386). **Rustdoc audit applied** (3472d56: 850 counterpart lines,
   names validated by `.reference/tools/rustdoc-audit/validate.py`), crate READMEs' status
   lines and the guide as homepage (a63f9b0), gate OK. Left: CHANGELOG/ROADMAP at release.
   Done before: the guide's chapters components and embeds, the cache,
   REST, sharding, voice and the migration chapter (examples rustfmt'ed, pass CI's tests;
   clippy fixed blind from CI's log, to run locally after bench-all); ARCHITECTURE and
   CONTRIBUTING rewritten. **Rustdoc audit prepared, not applied** (applying it rebuilds
   every crate, so after bench-all): scripts and curated maps in `.reference/tools/rustdoc-audit/` (git-ignored)
   (`audit.py`, `counterparts.py`, `model-map.txt`, `other-map.txt`, `rs-fn-map.txt`,
   `apply.py`); if lost, redo them: a `discord.js counterpart: …` line (C-134) for the 213
   model types, 75 other types and 61 djs-rs functions without one, then the remaining
   functions of model/cache/gateway/voice/formatters/collection/builders. Also left: the
   crate READMEs' stale "Status" lines (djs-rest "Phase 1", djs-cache "Phase 5 step 5", …),
   the workspace `homepage` → the guide (fixes CI 1.100's `redundant_homepage` warnings) and
   dropping the unused `djs-util` workspace dependency (both rebuild everything: after
   bench-all), and updating the local toolchain to 1.100 after every measurement.
5. [ ] The README (an index of everything, the comparisons' numbers) with the logo (D-390).
6. [ ] The release `0.1.0-alpha.1` (D-387, D-388, C-137). **Prepared and pushed:** versions,
   `publish` allowed for the ten crates, license files linked into each, the CHANGELOG
   section (dated 2026-11-17: re-date it if published later), README badge, the guide's
   installation from crates.io; names free on crates.io; `cargo package --workspace
   --exclude djs-vs-node --exclude djs-rs-guide` packs and builds all ten (largest
   djs-model, 315 KB); gate OK on Rust 1.100. **Waiting for the owner:** `cargo login`
   (the token never reaches Claude), the go-ahead, then `cargo publish --workspace
   --exclude djs-vs-node --exclude djs-rs-guide` (dependency order), the tag
   `v0.1.0-alpha.1` and the GitHub Release (the changelog's summary + a link to the full
   list).
7. [x] GitHub (D-389, C-134, D-391, D-392): issue forms, PR template, Dependabot, SECURITY.md;
   description, homepage (the guide), 12 topics, Discussions, private vulnerability
   reporting, labels; `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1, reports to the owner's
   `kontakt@kotoani.dev`); Dependabot's first 15 alerts dismissed with reasons (D-392).

## Blocked on the owner
- **Publishing 0.1.0-alpha.1** (plan item 6): the owner said "not yet" on 2026-11-17 (D-392);
  everything is prepared (C-137). When they say go: they run `cargo login` themselves, then
  `cargo publish --workspace --exclude djs-vs-node --exclude djs-rs-guide`, re-date the
  CHANGELOG section if the day changed, tag `v0.1.0-alpha.1`, push the tag, and create the
  GitHub Release (summary + link to the full changelog).
