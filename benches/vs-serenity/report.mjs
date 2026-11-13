// Turns the raw results of run.sh, build.sh and reliability.sh into the committed summary
// (benches/vs-serenity/results.json), the SVG charts (docs/vs-serenity/*.svg, light and dark)
// and the tables and charts between the `<!-- vs-serenity:NAME:start/end -->` markers of
// docs/VS_SERENITY.md. No dependencies; the chart helpers are shared with benches/vs-node.
//
// Usage: node benches/vs-serenity/report.mjs out   (after the scripts: reads out/*.jsonl)
//        node benches/vs-serenity/report.mjs       (re-render results.json)

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { chartWriter, compact, duration, grouped, micros, mib, millis, replace, table, times } from '../vs-node/node/charts.mjs';

const here = (path) => new URL(path, import.meta.url);
const SUMMARY = here('results.json');
const DOC = here('../../docs/VS_SERENITY.md');
const CHARTS = here('../../docs/vs-serenity/');
const writeChart = chartWriter(CHARTS, 'vs-serenity', 'serenity');

// ---------------------------------------------------------------------------------------------
// Aggregation: one entry per configuration, the median run by its primary metric.

const PRIMARY = {
  decode: 'median_ns',
  flood: 'events_per_sec',
  memory: 'rss_kib',
  rest: 'requests_per_sec',
  build: 'clean_ms',
  dependencies: 'packages',
};

const median = (values) => {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor((sorted.length - 1) / 2)];
};

function byKey(lines, key) {
  const groups = new Map();
  for (const line of lines) {
    const k = key(line);
    if (!groups.has(k)) groups.set(k, []);
    groups.get(k).push(line);
  }
  return [...groups.values()];
}

/** Performance and build results: the median run by the primary metric, with the range. */
function aggregateRuns(lines) {
  const scenarios = { decode: [], flood: [], memory: [], rest: [], build: [], dependencies: [] };
  const key = (r) => [r.scenario, r.payload, r.guilds, r.library, r.variant, r.profile].join('|');
  for (const runs of byKey(lines, key)) {
    const metric = PRIMARY[runs[0].scenario];
    const sorted = [...runs].sort((a, b) => a[metric] - b[metric]);
    const chosen = sorted[Math.floor((sorted.length - 1) / 2)];
    const entry = { ...chosen, runs: runs.length, range: [sorted[0][metric], sorted.at(-1)[metric]] };
    if (chosen.scenario === 'build') {
      entry.rebuild_ms = median(runs.map((r) => r.rebuild_ms));
      entry.rebuild_range = [Math.min(...runs.map((r) => r.rebuild_ms)), Math.max(...runs.map((r) => r.rebuild_ms))];
    }
    scenarios[chosen.scenario].push(entry);
  }
  return scenarios;
}

/** What a client did after a gateway fault, from the mock's report. */
function recovery(run) {
  if (run.stop_outcome) return 'stopped';
  if (run.mock.identifies > 1) return 'identified';
  if (run.mock.resumed > 0) return 'resumed';
  return 'none';
}

function aggregateReliability(lines) {
  const gateway = [];
  const api = [];
  for (const runs of byKey(lines, (r) => `${r.scenario}|${r.fault}|${r.library}`)) {
    const first = runs[0];
    if (first.scenario === 'reliability') {
      const recovered = runs.map((r) => r.mock.recovered_ms).filter((v) => v !== null && v !== undefined);
      const modes = runs.map(recovery);
      gateway.push({
        fault: first.fault,
        library: first.library,
        runs: runs.length,
        expected: first.expected,
        after_fault: first.expected - first.mock.fault_at,
        lost: median(runs.map((r) => r.missing)),
        lost_range: [Math.min(...runs.map((r) => r.missing)), Math.max(...runs.map((r) => r.missing))],
        duplicates: Math.max(...runs.map((r) => r.duplicates)),
        recovered_ms: recovered.length ? median(recovered) : null,
        recovered_range: recovered.length ? [Math.min(...recovered), Math.max(...recovered)] : null,
        recovery: modes[0],
        consistent: modes.every((m) => m === modes[0]),
        replayed: median(runs.map((r) => r.mock.replayed)),
        connections_after_fault: Math.max(...runs.map((r) => r.mock.connections_after_fault)),
        stop_outcome: first.stop_outcome,
        heartbeat_ms: first.mock.heartbeat_ms,
      });
    } else {
      const waits = runs.flatMap((r) => r.mock.waited_ms);
      api.push({
        fault: first.fault,
        library: first.library,
        runs: runs.length,
        requests: first.requests,
        sent: median(runs.map((r) => r.sent)),
        failed: median(runs.map((r) => r.failed)),
        received: median(runs.map((r) => r.mock.received)),
        limited: median(runs.map((r) => r.mock.limited)),
        server_errors: median(runs.map((r) => r.mock.server_errors)),
        in_flight: Math.max(...runs.map((r) => r.mock.in_flight)),
        early: Math.max(...runs.map((r) => r.mock.early)),
        waited_ms: waits.length ? median(waits) : null,
        waited_range: waits.length ? [Math.min(...waits), Math.max(...waits)] : null,
        errors: [...new Set(runs.flatMap((r) => r.errors))],
      });
    }
  }
  return { gateway, api };
}

function readLines(file) {
  return existsSync(file) ? readFileSync(file, 'utf8').split('\n').filter(Boolean).map((line) => JSON.parse(line)) : [];
}

// ---------------------------------------------------------------------------------------------
// Names.

function name(r) {
  const serenity = r.library === 'serenity';
  switch (r.scenario) {
    case 'decode':
      return serenity ? 'serenity (GatewayEvent)' : 'djs-rs (djs-model)';
    case 'flood':
      if (serenity) return 'serenity Client (default cache)';
      return r.variant.includes("serenity's default cache") ? "djs-rs Client + serenity's cache" : 'djs-rs Client (default: no cache)';
    case 'memory':
      return serenity ? 'serenity Client (default cache)' : "djs-rs Client + serenity's cache";
    case 'rest':
      if (!serenity) return 'djs-rest (default)';
      return r.variant.includes('disabled') ? 'serenity Http, limiter off, plain HTTP (bound)' : 'serenity Http (default, HTTPS)';
    default:
      return serenity ? 'serenity' : 'djs-rs';
  }
}

const isRust = (r) => r.library === 'djs-rs';
const equalCache = (r) => r.name === "djs-rs Client + serenity's cache" || r.name === 'serenity Client (default cache)';
const serenityDefault = (list, extra = () => true) =>
  list.find((r) => r.library === 'serenity' && !(r.variant ?? '').includes('disabled') && extra(r));

// ---------------------------------------------------------------------------------------------
// Formatting.

const seconds = (ms) => `${(ms / 1000).toFixed(ms < 10000 ? 2 : 1)} s`;
const sizeMib = (bytes) => `${(bytes / 1048576).toFixed(1)} MiB`;
const ms = (value) => (value < 10 ? `${value.toFixed(1)} ms` : value < 1000 ? `${value.toFixed(0)} ms` : `${(value / 1000).toFixed(2)} s`);
/** Within 5 %, neither library is said to win. */
const even = (ratio) => Math.abs(ratio - 1) < 0.05;
/** A ratio of djs-rs to serenity as text: who wins and by how much. */
const verdict = (ratio, word) =>
  even(ratio) ? 'about the same' : ratio >= 1 ? `**${times(ratio)} ${word}**` : `serenity **${times(1 / ratio)} ${word}**`;
const range = (r, show) => (r.runs > 1 && r.range[0] !== r.range[1] ? ` (${show(r.range[0])}–${show(r.range[1])})` : '');

// ---------------------------------------------------------------------------------------------
// Sections of docs/VS_SERENITY.md.

/** The performance and resource comparisons, each as (djs-rs, serenity, show, ratio, word). */
function headlines(s) {
  const rust = (list, extra = () => true) => list.find((r) => isRust(r) && extra(r));
  const message = (r) => r.payload === 'MESSAGE_CREATE';
  const guild = (r) => r.payload === 'GUILD_CREATE';
  const most = Math.max(...s.memory.map((r) => r.guilds));
  const big = (r) => r.guilds === most;
  const idle = (r) => r.guilds === 0;
  const release = (r) => r.profile === 'release';
  const items = [
    ['Decode `MESSAGE_CREATE`', rust(s.decode, message), serenityDefault(s.decode, message), (r) => duration(r.median_ns), (a, b) => b.median_ns / a.median_ns, 'faster'],
    ['Decode `GUILD_CREATE` (1000 members)', rust(s.decode, guild), serenityDefault(s.decode, guild), (r) => duration(r.median_ns), (a, b) => b.median_ns / a.median_ns, 'faster'],
    ['Event flood, equal caches (events/s)', rust(s.flood, equalCache), serenityDefault(s.flood), (r) => grouped(r.events_per_sec), (a, b) => a.events_per_sec / b.events_per_sec, 'more'],
    ['CPU per event during the flood', rust(s.flood, equalCache), serenityDefault(s.flood), (r) => micros(r.cpu_us_per_event), (a, b) => b.cpu_us_per_event / a.cpu_us_per_event, 'less'],
    ['Memory, idle bot (RSS, connected, no guilds)', rust(s.memory, idle), serenityDefault(s.memory, idle), (r) => mib(r.rss_kib), (a, b) => b.rss_kib / a.rss_kib, 'less'],
    [`Memory, ${grouped(most * 1000)} members (RSS)`, rust(s.memory, big), serenityDefault(s.memory, big), (r) => mib(r.rss_kib), (a, b) => b.rss_kib / a.rss_kib, 'less'],
    ['REST throughput (requests/s)', rust(s.rest), serenityDefault(s.rest), (r) => grouped(r.requests_per_sec), (a, b) => a.requests_per_sec / b.requests_per_sec, 'more'],
    ['REST p99 latency', rust(s.rest), serenityDefault(s.rest), (r) => millis(r.p99_us), (a, b) => b.p99_us / a.p99_us, 'lower'],
  ];
  if (s.dependencies.length) {
    items.push(
      ['Dependencies of a `!ping` bot', rust(s.dependencies), serenityDefault(s.dependencies), (r) => grouped(r.packages), (a, b) => b.packages / a.packages, 'fewer'],
      ['Clean release build', rust(s.build, release), serenityDefault(s.build, release), (r) => seconds(r.clean_ms), (a, b) => b.clean_ms / a.clean_ms, 'faster'],
      ['Release rebuild after editing `main.rs`', rust(s.build, release), serenityDefault(s.build, release), (r) => seconds(r.rebuild_ms), (a, b) => b.rebuild_ms / a.rebuild_ms, 'faster'],
      ['Release binary (stripped)', rust(s.build, release), serenityDefault(s.build, release), (r) => sizeMib(r.stripped_bytes), (a, b) => b.stripped_bytes / a.stripped_bytes, 'smaller'],
    );
  }
  return items.filter(([, a, b]) => a && b);
}

function summarySection(s) {
  const items = headlines(s);
  const picture = writeChart('summary', {
    title: 'djs-rs vs serenity, scenario by scenario',
    subtitle: 'how many times better the winner is: djs-rs in blue, serenity in orange · equal caches, default configurations',
    tick: (v) => `${v}×`,
    legend: false,
    sections: [
      {
        rows: items
          .map(([label, a, b, , ratio, word]) => {
            const r = ratio(a, b);
            const text = even(r) ? 'about the same' : r >= 1 ? `${times(r)} ${word}` : `serenity ${times(1 / r)} ${word}`;
            return { label: label.replace(/`/g, ''), value: r >= 1 ? r : 1 / r, rust: r >= 1, text };
          })
          .sort((x, y) => (y.rust - x.rust) || (y.value - x.value)),
      },
    ],
  });
  const rows = items.map(([label, a, b, show, ratio, word]) => [label, show(a), show(b), verdict(ratio(a, b), word)]);
  return `${picture}\n\n${table(['Scenario', 'djs-rs', 'serenity', 'Result'], rows)}`;
}

const FAULTS = {
  drop: 'Connection dropped (no close frame)',
  reconnect: 'Opcode 7 `RECONNECT`',
  invalid: 'Opcode 9 `INVALID_SESSION`, not resumable',
  zombie: 'Zombie connection (no heartbeat ACK)',
  close4000: 'Close code 4000 (resumable)',
  close4004: 'Close code 4004 (authentication failed)',
  route429: 'Route 429, `retry_after` 0.5 s',
  global429: 'Global 429, `retry_after` 0.5 s',
  server5xx: '502 on the first attempt at every 4th message',
};

/** One line for a client's behaviour after a gateway fault. */
function gatewayOutcome(r) {
  const lost = r.lost === 0 ? 'nothing lost' : `${grouped(r.lost)} events lost`;
  const dup = r.duplicates ? `, ${grouped(r.duplicates)} duplicated` : '';
  const recovered = r.recovered_ms === null ? '' : ` after ${ms(r.recovered_ms)}`;
  switch (r.recovery) {
    case 'stopped':
      return r.connections_after_fault === 0 ? 'stopped with an error, did not reconnect' : 'stopped with an error after reconnecting';
    case 'resumed':
      return `resumed${recovered}, ${lost}${dup}`;
    case 'identified':
      return `new session${recovered}, ${lost}${dup}`;
    default:
      return `did not recover, ${lost}`;
  }
}

function apiOutcome(r) {
  const sent = `${r.sent}/${r.requests} sent`;
  if (r.fault === 'server5xx') {
    const retried = r.received - r.requests;
    return r.failed === 0 ? `${sent}, ${retried} retried` : `${sent}, ${r.failed} failed (no retry)`;
  }
  const early = r.early === 0 ? 'none early' : `${r.early} early`;
  return `${sent}, waited ${ms(r.waited_ms)}, ${early}`;
}

function pair(list, fault) {
  return ['djs-rs', 'serenity'].map((library) => list.find((r) => r.fault === fault && r.library === library));
}

function reliabilitySummarySection(rel) {
  const rows = [];
  for (const fault of Object.keys(FAULTS)) {
    const list = rel.gateway.some((r) => r.fault === fault) ? rel.gateway : rel.api;
    const [a, b] = pair(list, fault);
    if (!a || !b) continue;
    const show = list === rel.gateway ? gatewayOutcome : apiOutcome;
    rows.push([FAULTS[fault], show(a), show(b)]);
  }
  return table(['Fault', 'djs-rs', 'serenity'], rows);
}

function decodeSection(all) {
  const parts = [];
  const rows = [];
  for (const payload of ['MESSAGE_CREATE', 'GUILD_CREATE']) {
    const results = all.filter((r) => r.payload === payload).sort((a, b) => a.median_ns - b.median_ns);
    const base = serenityDefault(results);
    const perSecond = (r) => 1e9 / r.median_ns;
    const size = `${Math.round(results[0].bytes / 1024)} KiB`;
    const label = payload === 'GUILD_CREATE' ? `GUILD_CREATE, 1000 members (${size})` : `MESSAGE_CREATE (${size})`;
    parts.push(
      writeChart(`decode-${payload === 'GUILD_CREATE' ? 'guild' : 'message'}`, {
        title: `Decoding ${label}`,
        subtitle: 'payloads per second on one core, higher is better · median of 15 rounds',
        tick: (v) => compact(v),
        sections: [
          {
            rows: results.map((r) => ({
              label: r.name,
              value: perSecond(r),
              rust: isRust(r),
              text: `${compact(perSecond(r))}/s${r === base ? '' : ` · ${times(base.median_ns / r.median_ns)} faster`}`,
            })),
          },
        ],
      }),
    );
    for (const r of results) {
      rows.push([`\`${payload}\``, r.name, duration(r.median_ns), grouped(perSecond(r)), r === base ? 'baseline' : verdict(base.median_ns / r.median_ns, 'faster')]);
    }
  }
  return `${parts.join('\n\n')}\n\n${table(['Payload', 'Implementation', 'Time per payload', 'Payloads/s', 'vs serenity'], rows)}`;
}

function floodSection(all) {
  const results = [...all].sort((a, b) => b.events_per_sec - a.events_per_sec);
  const base = serenityDefault(all);
  const throughput = writeChart('flood-throughput', {
    title: 'MESSAGE_CREATE flood through a mock gateway',
    subtitle: 'events per second reaching a handler, higher is better · 1 connection, 4 client CPUs',
    tick: (v) => compact(v),
    sections: [{ rows: results.map((r) => ({ label: r.name, value: r.events_per_sec, rust: isRust(r), text: `${compact(r.events_per_sec)}/s` })) }],
  });
  const cpu = writeChart('flood-cpu', {
    title: 'CPU time per event during the flood',
    subtitle: 'microseconds of CPU (all threads) per event, lower is better',
    tick: (v) => `${v} µs`,
    sections: [
      {
        rows: [...results]
          .sort((a, b) => a.cpu_us_per_event - b.cpu_us_per_event)
          .map((r) => ({ label: r.name, value: r.cpu_us_per_event, rust: isRust(r), text: micros(r.cpu_us_per_event) })),
      },
    ],
  });
  const rows = results.map((r) => [
    r.name,
    grouped(r.events_per_sec) + range(r, (v) => grouped(v)),
    r === base ? 'baseline' : verdict(r.events_per_sec / base.events_per_sec, 'more'),
    micros(r.cpu_us_per_event),
    r === base ? 'baseline' : verdict(base.cpu_us_per_event / r.cpu_us_per_event, 'less'),
    mib(r.peak_rss_kib),
  ]);
  return `${throughput}\n\n${cpu}\n\n${table(['Implementation', 'Events/s (range)', 'vs serenity', 'CPU per event', 'vs serenity', 'Peak RSS'], rows)}`;
}

function memorySection(all) {
  const counts = [...new Set(all.map((r) => r.guilds))].sort((a, b) => a - b);
  const loaded = counts.filter((g) => g > 0);
  const label = (g) => `${grouped(g)} guilds · ${grouped(g * 1000)} members`;
  const order = (r) => (isRust(r) ? 0 : 1);
  const base = (g) => serenityDefault(all, (r) => r.guilds === g);
  const picture = writeChart('memory', {
    title: 'Memory after receiving the guilds at startup',
    subtitle: 'resident memory (RSS) of the whole process, lower is better · equal caches',
    tick: (v) => `${v} MiB`,
    sections: loaded.map((g) => ({
      title: label(g),
      rows: all
        .filter((r) => r.guilds === g)
        .sort((a, b) => order(a) - order(b))
        .map((r) => ({
          label: r.name,
          value: r.rss_kib / 1024,
          rust: isRust(r),
          text: `${mib(r.rss_kib)}${r === base(g) ? '' : ` · ${times(base(g).rss_kib / r.rss_kib)} less`}`,
        })),
    })),
  });
  const rows = [];
  for (const g of counts) {
    for (const r of all.filter((x) => x.guilds === g).sort((a, b) => order(a) - order(b))) {
      const added = r.rss_kib - r.connected_rss_kib;
      rows.push([
        g === 0 ? 'connected, no guilds' : label(g),
        r.name,
        mib(r.rss_kib),
        r === base(g) ? 'baseline' : verdict(base(g).rss_kib / r.rss_kib, 'less'),
        g === 0 ? '—' : `${grouped((added * 1024) / r.members)} B`,
      ]);
    }
  }
  return `${picture}\n\n${table(['Guilds', 'Implementation', 'RSS', 'vs serenity', 'RSS added per member'], rows)}`;
}

function restSection(all) {
  const order = (r) => (isRust(r) ? 0 : r.variant.includes('disabled') ? 2 : 1);
  const results = [...all].sort((a, b) => order(a) - order(b));
  const base = serenityDefault(all);
  const throughput = writeChart('rest-throughput', {
    title: 'REST: concurrent Create Message calls against a mock API',
    subtitle: `requests per second, higher is better · ${results[0].concurrency} in flight, 4 client CPUs`,
    tick: (v) => compact(v),
    sections: [
      {
        rows: results.map((r) => ({
          label: r.name,
          value: r.requests_per_sec,
          rust: isRust(r),
          text: `${compact(r.requests_per_sec)}/s${isRust(r) ? ` · ${times(r.requests_per_sec / base.requests_per_sec)}` : ''}`,
        })),
      },
    ],
  });
  const latency = writeChart('rest-latency', {
    title: 'REST: latency of each call under that load',
    subtitle: 'milliseconds from the call to the decoded response, lower is better',
    tick: (v) => `${v} ms`,
    sections: [
      ['p50 (median)', 'p50_us'],
      ['p99', 'p99_us'],
    ].map(([title, key]) => ({
      title,
      rows: results.map((r) => ({ label: r.name, value: r[key] / 1000, rust: isRust(r), text: millis(r[key]) })),
    })),
  });
  const rows = results.map((r) => [
    r.name,
    grouped(r.requests_per_sec),
    r === base ? 'baseline' : isRust(r) ? verdict(r.requests_per_sec / base.requests_per_sec, 'more') : `${times(r.requests_per_sec / base.requests_per_sec)} the default`,
    millis(r.p50_us),
    millis(r.p99_us),
    micros(r.cpu_us_per_request),
    mib(r.peak_rss_kib),
  ]);
  return `${throughput}\n\n${latency}\n\n${table(['Implementation', 'Requests/s', 'vs serenity default', 'p50', 'p99', 'CPU per request', 'Peak RSS'], rows)}`;
}

function buildSection(builds, dependencies) {
  const order = (r) => (isRust(r) ? 0 : 1);
  const profiles = ['release', 'debug'];
  const of = (profile) => builds.filter((r) => r.profile === profile).sort((a, b) => order(a) - order(b));
  const times_ = writeChart('build-time', {
    title: 'Building the same !ping bot',
    subtitle: `seconds, lower is better · ${builds[0].jobs} parallel jobs · clean build, and the rebuild after editing main.rs`,
    tick: (v) => `${v} s`,
    sections: profiles.flatMap((profile) => [
      { title: `${profile}: clean build`, rows: of(profile).map((r) => ({ label: r.name, value: r.clean_ms / 1000, rust: isRust(r), text: seconds(r.clean_ms) })) },
      { title: `${profile}: rebuild after editing main.rs`, rows: of(profile).map((r) => ({ label: r.name, value: r.rebuild_ms / 1000, rust: isRust(r), text: seconds(r.rebuild_ms) })) },
    ]),
  });
  const size = writeChart('binary-size', {
    title: 'Size of the !ping bot',
    subtitle: 'MiB of the executable, lower is better · stripped of symbols and debug info',
    tick: (v) => `${v} MiB`,
    sections: profiles.map((profile) => ({
      title: profile,
      rows: of(profile).map((r) => ({ label: r.name, value: r.stripped_bytes / 1048576, rust: isRust(r), text: sizeMib(r.stripped_bytes) })),
    })),
  });
  const rows = [];
  for (const profile of profiles) {
    for (const r of of(profile)) {
      const showRange = (lo, hi, show) => (lo === hi ? '' : ` (${show(lo)}–${show(hi)})`);
      rows.push([
        profile,
        r.name,
        seconds(r.clean_ms) + showRange(r.range[0], r.range[1], seconds),
        seconds(r.rebuild_ms) + showRange(r.rebuild_range[0], r.rebuild_range[1], seconds),
        sizeMib(r.binary_bytes),
        sizeMib(r.stripped_bytes),
      ]);
    }
  }
  const deps = dependencies.sort((a, b) => order(a) - order(b)).map((r) => [r.name, grouped(r.packages)]);
  return `${times_}\n\n${size}\n\n${table(['Profile', 'Bot', 'Clean build (range)', 'Rebuild (range)', 'Binary', 'Stripped'], rows)}\n\n${table(['Bot', 'Dependencies (normal, this platform)'], deps)}`;
}

function gatewaySection(gateway) {
  const faults = Object.keys(FAULTS).filter((f) => gateway.some((r) => r.fault === f));
  const label = (fault) => FAULTS[fault].replace(/`/g, '');
  const recoverable = faults.filter((f) => pair(gateway, f).every((r) => r && r.recovered_ms !== null));
  const outage = writeChart('gateway-recovery', {
    title: 'Time without events after a gateway fault',
    subtitle: "from the fault to the first READY or RESUMED, lower is better · the mock's 1 s heartbeat interval",
    tick: (v) => `${v} s`,
    sections: recoverable.map((fault) => ({
      title: label(fault),
      rows: pair(gateway, fault).map((r) => ({ label: name({ library: r.library }), value: r.recovered_ms / 1000, rust: isRust(r), text: ms(r.recovered_ms) })),
    })),
  });
  const lossy = faults.filter((f) => f !== 'close4004' && pair(gateway, f).some((r) => r && r.lost > 0));
  const lost = writeChart('gateway-lost', {
    title: 'Events lost to a gateway fault',
    subtitle: `of the ${grouped(gateway[0].after_fault)} events Discord produced after the fault (one per ms), lower is better`,
    tick: (v) => grouped(v),
    sections: lossy.map((fault) => ({
      title: label(fault),
      rows: pair(gateway, fault).map((r) => ({ label: name({ library: r.library }), value: r.lost, rust: isRust(r), text: `${grouped(r.lost)} lost` })),
    })),
  });
  const rows = [];
  for (const fault of faults) {
    for (const r of pair(gateway, fault).filter(Boolean)) {
      const consistency = r.consistent ? '' : ' (not in every run)';
      rows.push([
        FAULTS[fault],
        name({ library: r.library }),
        `${r.recovery}${consistency}`,
        r.recovered_ms === null ? '—' : ms(r.recovered_ms) + (r.recovered_range[0] === r.recovered_range[1] ? '' : ` (${ms(r.recovered_range[0])}–${ms(r.recovered_range[1])})`),
        grouped(r.lost),
        grouped(r.duplicates),
        grouped(r.replayed),
      ]);
    }
  }
  return `${outage}\n\n${lost}\n\n${table(['Fault', 'Library', 'Recovery', 'Without events (range)', 'Lost', 'Duplicated', 'Replayed by RESUME'], rows)}`;
}

function apiSection(api) {
  const faults = Object.keys(FAULTS).filter((f) => api.some((r) => r.fault === f));
  const limited = faults.filter((f) => f !== 'server5xx');
  const waits = writeChart('api-wait', {
    title: 'Waiting out a 429 that asks for 0.5 s',
    subtitle: 'from the 429 to the next request the client sends, closer to 0.5 s is better (none was early)',
    tick: (v) => `${v} s`,
    sections: limited.map((fault) => ({
      title: FAULTS[fault].replace(/`/g, ''),
      rows: pair(api, fault).map((r) => ({ label: name({ library: r.library }), value: r.waited_ms / 1000, rust: isRust(r), text: ms(r.waited_ms) })),
    })),
  });
  const rows = [];
  for (const fault of faults) {
    for (const r of pair(api, fault).filter(Boolean)) {
      rows.push([
        FAULTS[fault],
        name({ library: r.library }),
        `${r.sent}/${r.requests}`,
        grouped(r.received),
        r.waited_ms === null ? '—' : ms(r.waited_ms),
        `${r.in_flight} / ${r.early}`,
        r.errors.length ? `\`${r.errors[0].replace(/`/g, "'")}\`` : '—',
      ]);
    }
  }
  return `${waits}\n\n${table(['Fault', 'Library', 'Sent', 'Requests received', 'Wait after the 429', 'In flight / early', 'Error reported'], rows)}`;
}

function environmentSection(e, rel) {
  const heartbeat = rel.gateway[0]?.heartbeat_ms;
  return table(
    ['', ''],
    [
      ['Date', e.date],
      ['CPU', `${e.cpu} (${e.threads} threads), ${e.memory_gib} GiB RAM`],
      ['OS', e.kernel],
      ['Rust', `\`${e.rustc}\`, release profile (\`lto = "fat"\`, \`codegen-units = 1\`) for the harness; the bots' own defaults for the builds`],
      ['serenity', `${e.serenity}, default features and configuration`],
      ['CPU pinning', `clients on CPUs ${e.client_cpus}, mock servers on CPUs ${e.server_cpus}, decode loops on CPU ${e.decode_cpu}; builds use every CPU`],
      ['Runs', `${e.runs} per configuration (the median run is shown; decode: median of 15 rounds; builds: the faster of ${e.build_runs ?? 2})`],
      ['Fault mocks', `heartbeat interval ${heartbeat} ms, one message per ms, 429s ask for 0.5 s`],
      ['Network', 'none: every run inside a network namespace with only a loopback interface'],
    ],
  );
}

// ---------------------------------------------------------------------------------------------

const [outDir] = process.argv.slice(2);
let summary;
if (outDir) {
  const at = (file) => resolve(outDir, file);
  const environment = JSON.parse(readFileSync(at('environment.json'), 'utf8'));
  const runs = aggregateRuns([...readLines(at('results.jsonl')), ...readLines(at('build.jsonl'))]);
  summary = { environment, scenarios: runs, reliability: aggregateReliability(readLines(at('reliability.jsonl'))) };
  writeFileSync(SUMMARY, `${JSON.stringify(summary, null, 2)}\n`);
} else {
  summary = JSON.parse(readFileSync(SUMMARY, 'utf8'));
}
for (const list of Object.values(summary.scenarios)) for (const r of list) r.name = name(r);

mkdirSync(CHARTS, { recursive: true });
const s = summary.scenarios;
const rel = summary.reliability;
let doc = readFileSync(DOC, 'utf8');
doc = replace(doc, 'vs-serenity', 'summary', summarySection(s));
doc = replace(doc, 'vs-serenity', 'reliability-summary', reliabilitySummarySection(rel));
doc = replace(doc, 'vs-serenity', 'decode', decodeSection(s.decode));
doc = replace(doc, 'vs-serenity', 'flood', floodSection(s.flood));
doc = replace(doc, 'vs-serenity', 'memory', memorySection(s.memory));
doc = replace(doc, 'vs-serenity', 'rest', restSection(s.rest));
doc = replace(doc, 'vs-serenity', 'build', buildSection(s.build, s.dependencies));
doc = replace(doc, 'vs-serenity', 'gateway', gatewaySection(rel.gateway));
doc = replace(doc, 'vs-serenity', 'api', apiSection(rel.api));
doc = replace(doc, 'vs-serenity', 'environment', environmentSection(summary.environment, rel));
writeFileSync(DOC, doc);
console.log('wrote benches/vs-serenity/results.json, docs/vs-serenity/*.svg and docs/VS_SERENITY.md');
