#!/usr/bin/env bash
# Runs every scenario of the djs-rs vs discord.js showcase and regenerates the numbers and
# charts of docs/VS_NODE.md (D-123). Linux only (/proc, taskset). Takes about five minutes.
#
#   benches/vs-node/run.sh             # everything, 3 runs per configuration
#   RUNS=1 benches/vs-node/run.sh      # quicker, noisier
#
# Clients run on CLIENT_CPUS and the mock servers on SERVER_CPUS, so they never compete for a
# core; the single-threaded decode loops run on DECODE_CPU. The defaults suit a 12-thread Intel
# hybrid CPU (performance cores 0-3, efficiency cores 4-11); adjust them for other machines.
set -euo pipefail
cd "$(dirname "$0")"

CLIENT_CPUS=${CLIENT_CPUS:-0-3}
SERVER_CPUS=${SERVER_CPUS:-4-11}
DECODE_CPU=${DECODE_CPU:-2}
RUNS=${RUNS:-3}
EVENTS=${EVENTS:-1000000}
REQUESTS=${REQUESTS:-300000}
CONCURRENCY=${CONCURRENCY:-64}
MEMORY_GUILDS=${MEMORY_GUILDS:-"0 100 500"}
export CLIENT_CPUS SERVER_CPUS DECODE_CPU RUNS

OUT=out
RESULTS=$OUT/results.jsonl
BIN=../../target/release/vs-node
BIN_MIMALLOC=../../target/release/vs-node-mimalloc

echo "== building (release) and installing the pinned npm packages"
cargo build --release --quiet -p djs-vs-node
npm ci --ignore-scripts --no-audit --no-fund --silent
mkdir -p "$OUT"
: > "$RESULTS"
: > "$OUT/mock.log"

node -e '
const os = require("os");
const fs = require("fs");
const [rustc, kernel] = process.argv.slice(1);
const version = (name) => JSON.parse(fs.readFileSync(`node_modules/${name}/package.json`)).version;
fs.writeFileSync("out/environment.json", JSON.stringify({
  date: new Date().toISOString().slice(0, 10),
  cpu: os.cpus()[0].model,
  threads: os.cpus().length,
  memory_gib: Math.round(os.totalmem() / 2 ** 30),
  kernel,
  rustc,
  node: process.versions.node,
  "discord.js": version("discord.js"),
  "@discordjs/ws": version("@discordjs/ws"),
  "@discordjs/rest": version("@discordjs/rest"),
  client_cpus: process.env.CLIENT_CPUS,
  server_cpus: process.env.SERVER_CPUS,
  decode_cpu: process.env.DECODE_CPU,
  runs: Number(process.env.RUNS),
}, null, 2) + "\n");
' "$(rustc --version)" "$(uname -sr)"

# CPU time (ms) used so far by process $1.
proc_cpu_ms() {
  local stat
  stat=$(< "/proc/$1/stat")
  stat=${stat##*) }
  read -r -a fields <<< "$stat"
  echo $(( (fields[11] + fields[12]) * 10 ))
}

# Starts a mock server in the background (arguments: its subcommand and options) and sets
# MOCK, API and GATEWAY.
start_mock() {
  : > "$OUT/mock.json"
  taskset -c "$SERVER_CPUS" "$BIN" "$@" > "$OUT/mock.json" 2>> "$OUT/mock.log" &
  MOCK=$!
  for _ in $(seq 100); do [ -s "$OUT/mock.json" ] && break; sleep 0.05; done
  API=$(sed -n 's/.*"api":"\([^"]*\)".*/\1/p' "$OUT/mock.json")
  GATEWAY=$(sed -n 's/.*"gateway":"\([^"]*\)".*/\1/p' "$OUT/mock.json")
}

stop_mock() {
  kill "$MOCK" 2> /dev/null || true
  wait "$MOCK" 2> /dev/null || true
}

# Runs one client on the client CPUs and records its result line, adding the CPU time the mock
# server spent meanwhile (to show the server was not the bottleneck).
client() {
  local before line
  before=$(proc_cpu_ms "$MOCK")
  line=$(taskset -c "$CLIENT_CPUS" "$@")
  echo "{\"server_cpu_ms\":$(( $(proc_cpu_ms "$MOCK") - before )),${line:1}" >> "$RESULTS"
  echo "   ${line:0:150}…"
}

echo "== 1/4 decode"
"$BIN" fixtures --dir "$OUT/fixtures"
taskset -c "$DECODE_CPU" "$BIN" decode --dir "$OUT/fixtures" >> "$RESULTS"
taskset -c "$DECODE_CPU" "$BIN_MIMALLOC" decode --dir "$OUT/fixtures" >> "$RESULTS"
taskset -c "$DECODE_CPU" node node/decode.mjs --dir "$OUT/fixtures" >> "$RESULTS"

echo "== 2/4 event flood ($EVENTS MESSAGE_CREATE per run)"
start_mock mock-gateway --guilds 1 --messages "$EVENTS"
for run in $(seq "$RUNS"); do
  echo " run $run/$RUNS"
  client "$BIN" flood --gateway "$GATEWAY" --events "$EVENTS"
  client "$BIN_MIMALLOC" flood --gateway "$GATEWAY" --events "$EVENTS"
  client "$BIN" flood --gateway "$GATEWAY" --events "$EVENTS" --mode sequential
  client "$BIN" shard-flood --gateway "$GATEWAY" --events "$EVENTS"
  client node node/flood.mjs --api "$API" --events "$EVENTS" --layer client
  client node node/flood.mjs --api "$API" --events "$EVENTS" --layer client --message-cache off
  client node node/flood.mjs --api "$API" --events "$EVENTS" --layer ws
done
stop_mock

echo "== 3/4 memory"
for guilds in $MEMORY_GUILDS; do
  start_mock mock-gateway --guilds "$guilds"
  for run in $(seq "$RUNS"); do
    echo " $guilds guilds, run $run/$RUNS"
    client "$BIN" memory --gateway "$GATEWAY" --guilds "$guilds"
    client "$BIN_MIMALLOC" memory --gateway "$GATEWAY" --guilds "$guilds"
    client node --expose-gc node/memory.mjs --api "$API"
  done
  stop_mock
done

echo "== 4/4 REST ($REQUESTS requests per run, $CONCURRENCY concurrent)"
start_mock mock-rest
for run in $(seq "$RUNS"); do
  echo " run $run/$RUNS"
  client "$BIN" rest --api "$API" --requests "$REQUESTS" --concurrency "$CONCURRENCY"
  client "$BIN_MIMALLOC" rest --api "$API" --requests "$REQUESTS" --concurrency "$CONCURRENCY"
  client node node/rest.mjs --api "$API" --requests "$REQUESTS" --concurrency "$CONCURRENCY"
done
stop_mock

echo "== report"
node node/report.mjs "$RESULTS" out/environment.json
