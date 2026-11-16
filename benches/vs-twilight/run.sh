#!/usr/bin/env bash
# Runs the djs-rs vs twilight comparison (D-385) and writes its raw results to
# out/results.jsonl. Linux only (/proc, taskset, unshare).
#
#   benches/vs-twilight/run.sh            # everything, 3 runs per configuration
#   RUNS=1 benches/vs-twilight/run.sh     # quicker, noisier
#
# It builds both harnesses, then runs itself again inside `unshare -rn`: a network namespace
# with only a loopback interface, so nothing can reach Discord (C-108). Clients run on
# CLIENT_CPUS and the mock servers on SERVER_CPUS; the single-threaded decode loops on
# DECODE_CPU. The defaults suit a 12-thread Intel hybrid CPU (performance cores 0-3), the
# same as benches/vs-serenity.
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
export CLIENT_CPUS SERVER_CPUS DECODE_CPU RUNS EVENTS REQUESTS CONCURRENCY MEMORY_GUILDS

OUT=out
RESULTS=$OUT/results.jsonl
NODE=../../target/release/vs-node
TW=target/release/djs-vs-twilight

if [ "${1:-}" != "--offline" ]; then
  echo "== building (release)"
  (cd ../.. && cargo build --release --quiet -p djs-vs-node)
  cargo build --release --quiet
  mkdir -p "$OUT"
  python3 - "$(rustc --version)" "$(uname -sr)" > "$OUT/environment.json" <<'EOF'
import json, os, re, sys, datetime
cpu = next(l.split(":", 1)[1].strip() for l in open("/proc/cpuinfo") if l.startswith("model name"))
memory = int(next(l.split()[1] for l in open("/proc/meminfo") if l.startswith("MemTotal"))) / 2**20
lock = open("Cargo.lock").read()
twilight = re.search(r'name = "twilight-gateway"\nversion = "([^"]+)"', lock).group(1)
try:
    profile = open("/sys/firmware/acpi/platform_profile").read().strip()
except OSError:
    profile = "unknown"
print(json.dumps({
    "date": datetime.date.today().isoformat(),
    "cpu": cpu,
    "threads": os.cpu_count(),
    "memory_gib": round(memory),
    "kernel": sys.argv[2],
    "rustc": sys.argv[1],
    "twilight": twilight,
    "platform_profile": profile,
    "client_cpus": os.environ["CLIENT_CPUS"],
    "server_cpus": os.environ["SERVER_CPUS"],
    "decode_cpu": os.environ["DECODE_CPU"],
    "runs": int(os.environ["RUNS"]),
}, indent=2))
EOF
  echo "== entering a network namespace without a route out"
  exec unshare -rn "$0" --offline
fi

ip link set lo up
if timeout 3 bash -c '</dev/tcp/1.1.1.1/443' 2> /dev/null; then
  echo "the network is reachable from the namespace; refusing to run" >&2
  exit 1
fi
: > "$RESULTS"
: > "$OUT/mock.log"

# CPU time (ms) used so far by process $1.
proc_cpu_ms() {
  local stat
  stat=$(< "/proc/$1/stat")
  stat=${stat##*) }
  read -r -a fields <<< "$stat"
  echo $(( (fields[11] + fields[12]) * 10 ))
}

# Reads field $2 of the JSON line in file $1.
field() {
  sed -n "s/.*\"$2\":\"\([^\"]*\)\".*/\1/p" "$1"
}

# Starts a mock server (its binary, subcommand and options) pinned to the server CPUs and
# sets MOCK, API and GATEWAY.
start_mock() {
  : > "$OUT/mock.json"
  taskset -c "$SERVER_CPUS" "$@" > "$OUT/mock.json" 2>> "$OUT/mock.log" &
  MOCK=$!
  for _ in $(seq 100); do [ -s "$OUT/mock.json" ] && break; sleep 0.05; done
  API=$(field "$OUT/mock.json" api)
  GATEWAY=$(field "$OUT/mock.json" gateway)
}

stop() {
  for pid in "$@"; do
    kill "$pid" 2> /dev/null || true
    wait "$pid" 2> /dev/null || true
  done
}

# Runs one client on the client CPUs and records its result line with the CPU time the mock
# server spent meanwhile (to show it was not the bottleneck).
client() {
  local before line after
  before=$(proc_cpu_ms "$MOCK")
  line=$(taskset -c "$CLIENT_CPUS" "$@")
  after=$(proc_cpu_ms "$MOCK")
  echo "{\"server_cpu_ms\":$(( after - before )),${line:1}" >> "$RESULTS"
  echo "   ${line:0:150}…"
}

echo "== 1/4 decode"
"$NODE" fixtures --dir "$OUT/fixtures"
taskset -c "$DECODE_CPU" "$NODE" decode --dir "$OUT/fixtures" >> "$RESULTS"
taskset -c "$DECODE_CPU" "$TW" decode --dir "$OUT/fixtures" >> "$RESULTS"

echo "== 2/4 event flood ($EVENTS MESSAGE_CREATE per run)"
start_mock "$NODE" mock-gateway --guilds 1 --messages "$EVENTS"
for run in $(seq "$RUNS"); do
  echo " run $run/$RUNS"
  client "$NODE" flood --gateway "$GATEWAY" --events "$EVENTS" --cache twilight
  client "$NODE" flood --gateway "$GATEWAY" --events "$EVENTS"
  client "$TW" flood --gateway "$GATEWAY" --events "$EVENTS"
  client "$TW" flood --gateway "$GATEWAY" --events "$EVENTS" --cache none
done
stop "$MOCK"

echo "== 3/4 memory"
for guilds in $MEMORY_GUILDS; do
  start_mock "$NODE" mock-gateway --guilds "$guilds"
  for run in $(seq "$RUNS"); do
    echo " $guilds guilds, run $run/$RUNS"
    client "$NODE" memory --gateway "$GATEWAY" --guilds "$guilds" --cache twilight
    client "$TW" memory --gateway "$GATEWAY" --guilds "$guilds"
  done
  stop "$MOCK"
done

echo "== 4/4 REST ($REQUESTS requests per run, $CONCURRENCY concurrent)"
start_mock "$NODE" mock-rest
for run in $(seq "$RUNS"); do
  echo " run $run/$RUNS"
  client "$NODE" rest --api "$API" --requests "$REQUESTS" --concurrency "$CONCURRENCY"
  client "$TW" rest --api "$API" --requests "$REQUESTS" --concurrency "$CONCURRENCY"
  client "$TW" rest --api "$API" --limiter off --requests "$REQUESTS" --concurrency "$CONCURRENCY"
done
stop "$MOCK"

echo "== done: $(wc -l < "$RESULTS") results in $RESULTS"
