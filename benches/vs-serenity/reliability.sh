#!/usr/bin/env bash
# The reliability scenarios of the djs-rs vs serenity comparison (D-279): a failing gateway
# (dropped connection, RECONNECT, INVALID_SESSION, a zombie connection, close codes 4000 and
# 4004) and a failing API (route and global 429s, 502s). Writes out/reliability.jsonl: each
# client's result with the mock's report under "mock".
#
#   benches/vs-serenity/reliability.sh            # 3 runs of every scenario and library
#   RUNS=1 MESSAGES=4000 FAULT_AT=1000 benches/vs-serenity/reliability.sh
#
# Runs inside a network namespace without a route out, like run.sh (C-108).
set -euo pipefail
cd "$(dirname "$0")"

CLIENT_CPUS=${CLIENT_CPUS:-0-3}
SERVER_CPUS=${SERVER_CPUS:-4-11}
RUNS=${RUNS:-3}
MESSAGES=${MESSAGES:-10000}
FAULT_AT=${FAULT_AT:-2000}
# 40 requests stay under djs-rs's default proactive global limit of 50 per second (serenity
# has none), so the times measure the fault's handling only.
REQUESTS=${REQUESTS:-40}
CONCURRENCY=${CONCURRENCY:-4}
GATEWAY_FAULTS=${GATEWAY_FAULTS:-"drop reconnect invalid zombie close4000 close4004"}
REST_FAULTS=${REST_FAULTS:-"route429 global429 server5xx"}
export CLIENT_CPUS SERVER_CPUS RUNS MESSAGES FAULT_AT REQUESTS CONCURRENCY GATEWAY_FAULTS REST_FAULTS

OUT=out
RESULTS=$OUT/reliability.jsonl
SER=target/release/djs-vs-serenity

if [ "${1:-}" != "--offline" ]; then
  echo "== building (release)"
  cargo build --release --quiet
  mkdir -p "$OUT"
  echo "== entering a network namespace without a route out"
  exec unshare -rn "$0" --offline
fi
ip link set lo up
if timeout 3 bash -c '</dev/tcp/1.1.1.1/443' 2> /dev/null; then
  echo "the network is reachable from the namespace; refusing to run" >&2
  exit 1
fi
: > "$RESULTS"
: > "$OUT/fault.log"

field() {
  sed -n "s/.*\"$2\":\"\([^\"]*\)\".*/\1/p" "$1"
}

# Starts a server pinned to the server CPUs, waits for its JSON line in $OUT/$1.json and adds
# its pid to SERVERS.
start() {
  local name=$1
  shift
  : > "$OUT/$name.json"
  taskset -c "$SERVER_CPUS" "$@" > "$OUT/$name.json" 2>> "$OUT/fault.log" &
  SERVERS+=($!)
  for _ in $(seq 100); do [ -s "$OUT/$name.json" ] && return; sleep 0.05; done
  echo "$name did not start" >&2
  exit 1
}

stop_servers() {
  for pid in "${SERVERS[@]}"; do
    kill "$pid" 2> /dev/null || true
    wait "$pid" 2> /dev/null || true
  done
  SERVERS=()
}

# Records the client's line ($1) with the mock's report ($2).
record() {
  python3 -c 'import json, sys
line = json.loads(sys.argv[1])
line["mock"] = json.load(open(sys.argv[2]))
print(json.dumps(line))' "$1" "$2" | tee -a "$RESULTS" | cut -c1-200
}

SERVERS=()
echo "== 1/2 gateway faults ($MESSAGES messages, one per ms, the fault at message $FAULT_AT)"
for fault in $GATEWAY_FAULTS; do
  # A client that loses messages waits for the whole timeout: the production and 5 s more.
  timeout_s=$((MESSAGES / 1000 + 5))
  [ "$fault" = close4004 ] && timeout_s=8
  for run in $(seq "$RUNS"); do
    for library in djs-rs serenity; do
      echo " $fault, $library, run $run/$RUNS"
      start gateway "$SER" fault-gateway --scenario "$fault" --messages "$MESSAGES" \
        --fault-at "$FAULT_AT" --report "$OUT/fault-report.json"
      gateway=$(field "$OUT/gateway.json" gateway)
      if [ "$library" = djs-rs ]; then
        connect=(--gateway "$gateway")
      else
        start tls "$SER" tls-api --dir "$OUT/tls" --gateway "$gateway"
        connect=(--tls "$(field "$OUT/tls.json" tls_api)" --ca "$(field "$OUT/tls.json" ca)")
      fi
      line=$(taskset -c "$CLIENT_CPUS" "$SER" reliability --library "$library" --scenario "$fault" \
        --messages "$MESSAGES" --timeout-s "$timeout_s" "${connect[@]}")
      sleep 0.5
      record "$line" "$OUT/fault-report.json"
      stop_servers
    done
  done
done

echo "== 2/2 API faults ($REQUESTS requests, $CONCURRENCY channels)"
for fault in $REST_FAULTS; do
  for run in $(seq "$RUNS"); do
    for library in djs-rs serenity; do
      echo " $fault, $library, run $run/$RUNS"
      start api "$SER" fault-rest --scenario "$fault" --dir "$OUT/tls" --report "$OUT/fault-report.json"
      if [ "$library" = djs-rs ]; then
        connect=(--api "$(field "$OUT/api.json" api)")
      else
        connect=(--tls "$(field "$OUT/api.json" tls_api)" --ca "$(field "$OUT/api.json" ca)")
      fi
      line=$(taskset -c "$CLIENT_CPUS" timeout 120 "$SER" rest-faults --library "$library" \
        --scenario "$fault" --requests "$REQUESTS" --concurrency "$CONCURRENCY" "${connect[@]}")
      record "$line" "$OUT/fault-report.json"
      stop_servers
    done
  done
done

echo "== done: $(wc -l < "$RESULTS") results in $RESULTS"
