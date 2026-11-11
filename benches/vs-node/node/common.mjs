// Shared helpers of the discord.js side of the showcase (docs/VS_NODE.md, D-123).
//
// Process metrics and the timing loop are implemented exactly like `src/metrics.rs`, so both
// libraries are measured the same way.

import { readFileSync } from 'node:fs';

/** The token every client uses. It is not a real token and authenticates nothing. */
export const TOKEN = 'not-a-real-token-only-for-benchmarking-0123456789';

/** Clock ticks per second of /proc/<pid>/stat (USER_HZ, 100 on mainstream Linux). */
const TICKS_PER_SECOND = 100;

/** CPU time (user + system, all threads) the process has used so far, in milliseconds. */
export function cpuMs() {
  const stat = readFileSync('/proc/self/stat', 'utf8');
  // The command name (field 2) is in parentheses and may contain spaces; field 3 follows.
  const fields = stat.slice(stat.lastIndexOf(')') + 2).split(' ');
  // `fields` starts at field 3, so utime (14) and stime (15) are entries 11 and 12.
  return ((Number(fields[11]) + Number(fields[12])) * 1000) / TICKS_PER_SECOND;
}

/** A kB value of /proc/self/status, such as VmRSS (resident now) or VmHWM (peak). */
export function statusKib(key) {
  const status = readFileSync('/proc/self/status', 'utf8');
  const line = status.split('\n').find((entry) => entry.startsWith(`${key}:`));
  if (!line) throw new Error(`${key} is not in /proc/self/status`);
  return Number.parseInt(line.slice(key.length + 1).trim(), 10);
}

/** Parses `--key value` arguments. */
export function options() {
  const args = process.argv.slice(2);
  const parsed = {};
  for (let i = 0; i < args.length; i += 2) {
    if (!args[i].startsWith('--') || args[i + 1] === undefined) {
      throw new Error(`expected --option value, got ${args[i]}`);
    }
    parsed[args[i].slice(2)] = args[i + 1];
  }
  return parsed;
}

/** Prints one result line. */
export function emit(result) {
  console.log(JSON.stringify(result));
}

/** The versions of the measured packages, included in every result. */
export const VERSIONS = (() => {
  const version = (name) =>
    JSON.parse(readFileSync(new URL(`../node_modules/${name}/package.json`, import.meta.url), 'utf8')).version;
  return {
    node: process.versions.node,
    'discord.js': version('discord.js'),
    '@discordjs/ws': version('@discordjs/ws'),
    '@discordjs/rest': version('@discordjs/rest'),
  };
})();

const WARM_UP_MS = 1000;
const ROUND_MS = 200;
const ROUNDS = 15;

/**
 * Times `f`: warms up for a second, picks an iteration count that takes about 200 ms, then
 * runs 15 rounds of it and returns the median and minimum time per call in nanoseconds.
 */
export function measure(f) {
  const now = () => Number(process.hrtime.bigint());
  const started = now();
  let calls = 0;
  while (now() - started < WARM_UP_MS * 1e6) {
    f();
    calls += 1;
  }
  const perCall = (WARM_UP_MS * 1e6) / calls;
  const iterations = Math.max(1, Math.floor((ROUND_MS * 1e6) / perCall));
  const rounds = [];
  for (let round = 0; round < ROUNDS; round += 1) {
    const roundStarted = now();
    for (let i = 0; i < iterations; i += 1) f();
    rounds.push((now() - roundStarted) / iterations);
  }
  rounds.sort((a, b) => a - b);
  return { median_ns: rounds[Math.floor(ROUNDS / 2)], min_ns: rounds[0] };
}

/** The `p`-th percentile (0–100) of sorted values (nearest rank), like `metrics::percentile`. */
export function percentile(sorted, p) {
  if (sorted.length === 0) return 0;
  const index = Math.min(Math.max(Math.ceil((p / 100) * sorted.length), 1), sorted.length) - 1;
  return sorted[index];
}
