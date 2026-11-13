#!/usr/bin/env bash
# Resources of the same `!ping` bot built on djs-rs and on serenity (D-279): clean release and
# debug build times, the rebuild after a change of `main.rs`, the binary sizes and the number
# of dependencies. Writes out/build.jsonl. The bots are only built, never run.
#
#   benches/vs-serenity/build.sh             # 2 runs of each build, alternated
#   BUILD_RUNS=1 benches/vs-serenity/build.sh
#
# Builds use every CPU (cargo's default), as on a developer's machine, `--offline` and inside a
# network namespace without a route out (C-108); the dependencies must be fetched before
# (`cargo fetch` in bots/djs-rs and bots/serenity).
set -euo pipefail
cd "$(dirname "$0")"

BUILD_RUNS=${BUILD_RUNS:-2}
export BUILD_RUNS
OUT=out
RESULTS=$OUT/build.jsonl

if [ "${1:-}" != "--offline" ]; then
  mkdir -p "$OUT"
  exec unshare -rn "$0" --offline
fi
ip link set lo up
if timeout 3 bash -c '</dev/tcp/1.1.1.1/443' 2> /dev/null; then
  echo "the network is reachable from the namespace; refusing to run" >&2
  exit 1
fi
: > "$RESULTS"

# Times `cargo build` ($2: release or debug) of bot $1 from a fresh target directory, then the
# rebuild after touching main.rs, and records one line.
build() {
  local bot=$1 profile=$2 target=$OUT/target-$1 flags=() started clean rebuild
  [ "$profile" = release ] && flags=(--release)
  rm -rf "$target"
  started=$(date +%s%N)
  (cd "bots/$bot" && CARGO_TARGET_DIR="../../$target" cargo build --offline --quiet "${flags[@]}")
  clean=$(( ($(date +%s%N) - started) / 1000000 ))
  touch "bots/$bot/src/main.rs"
  started=$(date +%s%N)
  (cd "bots/$bot" && CARGO_TARGET_DIR="../../$target" cargo build --offline --quiet "${flags[@]}")
  rebuild=$(( ($(date +%s%N) - started) / 1000000 ))
  local binary=$target/$profile/ping-$bot size stripped
  size=$(stat -c %s "$binary")
  strip -o "$target/stripped" "$binary"
  stripped=$(stat -c %s "$target/stripped")
  echo "{\"scenario\":\"build\",\"library\":\"$bot\",\"profile\":\"$profile\",\"clean_ms\":$clean,\"rebuild_ms\":$rebuild,\"binary_bytes\":$size,\"stripped_bytes\":$stripped,\"jobs\":$(nproc)}" | tee -a "$RESULTS"
}

# The normal (not build or dev) dependencies of bot $1 on this platform, without the bot.
dependencies() {
  local count
  count=$(cd "bots/$1" && cargo tree --offline -e normal --prefix none \
    | sed -E 's/ \((\*|proc-macro)\)//g' | sort -u | grep -vc "^ping-$1 ")
  echo "{\"scenario\":\"dependencies\",\"library\":\"$1\",\"packages\":$count}" | tee -a "$RESULTS"
}

dependencies djs-rs
dependencies serenity
for run in $(seq "$BUILD_RUNS"); do
  echo " run $run/$BUILD_RUNS"
  for profile in release debug; do
    build djs-rs "$profile"
    build serenity "$profile"
  done
done
rm -rf "$OUT"/target-*
