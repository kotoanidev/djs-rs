# djs-rs vs discord.js — benchmark harness

The harness behind [`docs/VS_NODE.md`](../../docs/VS_NODE.md) (decision D-123). It measures
djs-rs and discord.js on the same machine with the same inputs and the same mock Discord
servers. Linux only: metrics are read from `/proc` and processes are pinned with `taskset`.

```bash
benches/vs-node/run.sh     # runs everything and rewrites docs/VS_NODE.md
```

## Layout

| Path | What it is |
|------|------------|
| `run.sh` | Orchestrator: builds, runs every scenario, calls `report.mjs`. Settings are environment variables at its top. |
| `src/` | The Rust crate `djs-vs-node` (`publish = false`): payload generator, mock servers and the djs-rs runners. |
| `src/bin/vs-node.rs`, `src/bin/vs-node-mimalloc.rs` | The same subcommands with the system allocator and with mimalloc. |
| `node/` | The discord.js runners (`decode`, `flood`, `memory`, `rest`), shared helpers and `report.mjs`. |
| `package.json`, `package-lock.json` | The pinned npm packages (`npm ci`; `node_modules/` is git-ignored). |
| `results.json` | The committed summary of the last run, from which `report.mjs` renders the page. |
| `out/` | Raw results, fixtures and mock-server logs of the last run (git-ignored). |

## Scenarios

| Scenario | Rust runner | Node.js runner | Mock |
|----------|-------------|----------------|------|
| Decoding | `vs-node decode` | `node/decode.mjs` | — (fixtures from `vs-node fixtures`) |
| Event flood | `vs-node flood [--cache discord-js] [--mode sequential]`, `vs-node shard-flood` | `node/flood.mjs --layer client\|ws` | `vs-node mock-gateway --messages N` |
| Memory | `vs-node memory` | `node --expose-gc node/memory.mjs` | `vs-node mock-gateway --guilds N` |
| REST | `vs-node rest` | `node/rest.mjs` | `vs-node mock-rest` |

Every runner prints one JSON line. The mock servers print their URLs as JSON on start-up
(`{"api": …, "gateway": …}`) and run until killed.

## Fairness rules

- Inputs come from one generator; both sides read the same fixture files or connect to the
  same mock server.
- Clients run pinned to the same CPUs, mock servers on other CPUs.
- Default configurations first; every variant is labelled.
- No table or chart is typed by hand: `report.mjs` generates them from the results. The
  prose of `docs/VS_NODE.md` quotes the committed run and must be checked after a re-run.
