// Turns the raw results of run.sh and build.sh into the committed summary
// (benches/vs-twilight/results.json), the SVG charts (docs/vs-twilight/*.svg, light and dark)
// and the tables and charts between the `<!-- vs-twilight:NAME:start/end -->` markers of
// docs/VS_TWILIGHT.md (D-385). No dependencies; the chart helpers are shared with
// benches/vs-node.
//
// Usage: node benches/vs-twilight/report.mjs out   (after the scripts: reads out/*.jsonl)
//        node benches/vs-twilight/report.mjs       (re-render results.json)

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { chartWriter, compact, duration, grouped, micros, mib, millis, replace, table, times } from '../vs-node/node/charts.mjs';

const here = (path) => new URL(path, import.meta.url);
const SUMMARY = here('results.json');
const DOC = here('../../docs/VS_TWILIGHT.md');
const CHARTS = here('../../docs/vs-twilight/');
const writeChart = chartWriter(CHARTS, 'vs-twilight', 'twilight');

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

function aggregateRuns(lines) {
  const scenarios = { decode: [], flood: [], memory: [], rest: [], build: [], dependencies: [] };
  const groups = new Map();
  for (const line of lines) {
    const key = [line.scenario, line.payload, line.guilds, line.library, line.variant, line.profile].join('|');
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(line);
  }
  for (const runs of groups.values()) {
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

function readLines(file) {
  return existsSync(file) ? readFileSync(file, 'utf8').split('\n').filter(Boolean).map((line) => JSON.parse(line)) : [];
}

// ---------------------------------------------------------------------------------------------
// Names.

const isRust = (r) => r.library === 'djs-rs';
const cached = (r) => (isRust(r) ? (r.variant ?? '').includes("twilight's default cache") : (r.variant ?? '').includes('default cache'));

function name(r) {
  const twilight = r.library === 'twilight';
  switch (r.scenario) {
    case 'decode':
      return twilight ? 'twilight (twilight-model GatewayEvent)' : 'djs-rs (djs-model)';
    case 'flood':
      if (twilight) return cached(r) ? 'twilight Shard + default cache' : 'twilight Shard, no cache';
      return cached(r) ? "djs-rs Client + twilight's cache" : 'djs-rs Client (default: no cache)';
    case 'memory':
      return twilight ? 'twilight Shard + default cache' : "djs-rs Client + twilight's cache";
    case 'rest':
      if (!twilight) return 'djs-rest, global limit lifted';
      return r.variant.includes('disabled') ? 'twilight-http, limiter off (bound)' : 'twilight-http, global limit lifted';
    default:
      return twilight ? 'twilight' : 'djs-rs';
  }
}

/** twilight's default configuration (its default cache, its rate limiter). */
const twilightDefault = (list, extra = () => true) =>
  list.find((r) => r.library === 'twilight' && !(r.variant ?? '').includes('disabled') && !(r.variant ?? '').includes('no cache') && extra(r));
const rust = (list, extra = () => true) => list.find((r) => isRust(r) && extra(r));

// ---------------------------------------------------------------------------------------------
// Formatting.

const seconds = (ms) => `${(ms / 1000).toFixed(ms < 10000 ? 2 : 1)} s`;
const sizeMib = (bytes) => `${(bytes / 1048576).toFixed(1)} MiB`;
/** Within 5 %, neither library is said to win. */
const even = (ratio) => Math.abs(ratio - 1) < 0.05;
/** A ratio of djs-rs to twilight as text: who wins and by how much. */
const verdict = (ratio, word) =>
  even(ratio) ? 'about the same' : ratio >= 1 ? `**${times(ratio)} ${word}**` : `twilight **${times(1 / ratio)} ${word}**`;
const range = (r, show) => (r.runs > 1 && r.range[0] !== r.range[1] ? ` (${show(r.range[0])}–${show(r.range[1])})` : '');

// ---------------------------------------------------------------------------------------------
// Sections of docs/VS_TWILIGHT.md.

/** The comparisons, each as (label, djs-rs, twilight, show, ratio, word). */
function headlines(s) {
  const message = (r) => r.payload === 'MESSAGE_CREATE';
  const guild = (r) => r.payload === 'GUILD_CREATE';
  const most = Math.max(...s.memory.map((r) => r.guilds));
  const big = (r) => r.guilds === most;
  const idle = (r) => r.guilds === 0;
  const release = (r) => r.profile === 'release';
  const twilightNoCache = s.flood.find((r) => r.library === 'twilight' && !cached(r));
  const items = [
    ['Decode `MESSAGE_CREATE`', rust(s.decode, message), twilightDefault(s.decode, message), (r) => duration(r.median_ns), (a, b) => b.median_ns / a.median_ns, 'faster'],
    ['Decode `GUILD_CREATE` (1000 members)', rust(s.decode, guild), twilightDefault(s.decode, guild), (r) => duration(r.median_ns), (a, b) => b.median_ns / a.median_ns, 'faster'],
    ['Event flood, equal caches (events/s)', rust(s.flood, cached), twilightDefault(s.flood), (r) => grouped(r.events_per_sec), (a, b) => a.events_per_sec / b.events_per_sec, 'more'],
    ['Event flood, no cache (events/s)', rust(s.flood, (r) => !cached(r)), twilightNoCache, (r) => grouped(r.events_per_sec), (a, b) => a.events_per_sec / b.events_per_sec, 'more'],
    ['CPU per event, equal caches', rust(s.flood, cached), twilightDefault(s.flood), (r) => micros(r.cpu_us_per_event), (a, b) => b.cpu_us_per_event / a.cpu_us_per_event, 'less'],
    ['Memory, idle bot (RSS, connected, no guilds)', rust(s.memory, idle), twilightDefault(s.memory, idle), (r) => mib(r.rss_kib), (a, b) => b.rss_kib / a.rss_kib, 'less'],
    [`Memory, ${grouped(most * 1000)} members (RSS)`, rust(s.memory, big), twilightDefault(s.memory, big), (r) => mib(r.rss_kib), (a, b) => b.rss_kib / a.rss_kib, 'less'],
    ['REST throughput (requests/s)', rust(s.rest), twilightDefault(s.rest), (r) => grouped(r.requests_per_sec), (a, b) => a.requests_per_sec / b.requests_per_sec, 'more'],
    ['REST p99 latency', rust(s.rest), twilightDefault(s.rest), (r) => millis(r.p99_us), (a, b) => b.p99_us / a.p99_us, 'lower'],
  ];
  if (s.dependencies.length) {
    items.push(
      ['Dependencies of a `!ping` bot', rust(s.dependencies), twilightDefault(s.dependencies), (r) => grouped(r.packages), (a, b) => b.packages / a.packages, 'fewer'],
      ['Clean release build', rust(s.build, release), twilightDefault(s.build, release), (r) => seconds(r.clean_ms), (a, b) => b.clean_ms / a.clean_ms, 'faster'],
      ['Release rebuild after editing `main.rs`', rust(s.build, release), twilightDefault(s.build, release), (r) => seconds(r.rebuild_ms), (a, b) => b.rebuild_ms / a.rebuild_ms, 'faster'],
      ['Release binary (stripped)', rust(s.build, release), twilightDefault(s.build, release), (r) => sizeMib(r.stripped_bytes), (a, b) => b.stripped_bytes / a.stripped_bytes, 'smaller'],
    );
  }
  return items.filter(([, a, b]) => a && b);
}

function summarySection(s) {
  const items = headlines(s);
  const picture = writeChart('summary', {
    title: 'djs-rs vs twilight, scenario by scenario',
    subtitle: 'how many times better the winner is: djs-rs in blue, twilight in orange · default configurations, equal caches',
    tick: (v) => `${v}×`,
    legend: false,
    sections: [
      {
        rows: items
          .map(([label, a, b, , ratio, word]) => {
            const r = ratio(a, b);
            const text = even(r) ? 'about the same' : r >= 1 ? `${times(r)} ${word}` : `twilight ${times(1 / r)} ${word}`;
            return { label: label.replace(/`/g, ''), value: r >= 1 ? r : 1 / r, rust: r >= 1, text };
          })
          .sort((x, y) => (y.rust - x.rust) || (y.value - x.value)),
      },
    ],
  });
  const rows = items.map(([label, a, b, show, ratio, word]) => [label, show(a), show(b), verdict(ratio(a, b), word)]);
  return `${picture}\n\n${table(['Scenario', 'djs-rs', 'twilight', 'Result'], rows)}`;
}

function decodeSection(all) {
  const parts = [];
  const rows = [];
  for (const payload of ['MESSAGE_CREATE', 'GUILD_CREATE']) {
    const results = all.filter((r) => r.payload === payload).sort((a, b) => a.median_ns - b.median_ns);
    const base = twilightDefault(results);
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
  return `${parts.join('\n\n')}\n\n${table(['Payload', 'Implementation', 'Time per payload', 'Payloads/s', 'vs twilight'], rows)}`;
}

function floodSection(all) {
  const results = [...all].sort((a, b) => b.events_per_sec - a.events_per_sec);
  // Each row is compared with twilight in the same cache configuration.
  const base = (r) => all.find((x) => x.library === 'twilight' && cached(x) === cached(r));
  const throughput = writeChart('flood-throughput', {
    title: 'MESSAGE_CREATE flood through a mock gateway',
    subtitle: 'events per second reaching a handler, higher is better · 1 connection, 4 client CPUs',
    tick: (v) => compact(v),
    sections: [true, false].map((withCache) => ({
      title: withCache ? "with twilight's default cache on both sides" : 'without a cache',
      rows: results.filter((r) => cached(r) === withCache).map((r) => ({ label: r.name, value: r.events_per_sec, rust: isRust(r), text: `${compact(r.events_per_sec)}/s` })),
    })),
  });
  const cpu = writeChart('flood-cpu', {
    title: 'CPU time per event during the flood',
    subtitle: 'microseconds of CPU (all threads) per event, lower is better',
    tick: (v) => `${v} µs`,
    sections: [true, false].map((withCache) => ({
      title: withCache ? 'equal caches' : 'no cache',
      rows: results
        .filter((r) => cached(r) === withCache)
        .sort((a, b) => a.cpu_us_per_event - b.cpu_us_per_event)
        .map((r) => ({ label: r.name, value: r.cpu_us_per_event, rust: isRust(r), text: micros(r.cpu_us_per_event) })),
    })),
  });
  const rows = results.map((r) => {
    const b = base(r);
    return [
      r.name,
      grouped(r.events_per_sec) + range(r, (v) => grouped(v)),
      r === b ? 'baseline' : verdict(r.events_per_sec / b.events_per_sec, 'more'),
      micros(r.cpu_us_per_event),
      r === b ? 'baseline' : verdict(b.cpu_us_per_event / r.cpu_us_per_event, 'less'),
      mib(r.peak_rss_kib),
    ];
  });
  return `${throughput}\n\n${cpu}\n\n${table(['Implementation', 'Events/s (range)', 'vs twilight, same cache', 'CPU per event', 'vs twilight', 'Peak RSS'], rows)}`;
}

function memorySection(all) {
  const counts = [...new Set(all.map((r) => r.guilds))].sort((a, b) => a - b);
  const loaded = counts.filter((g) => g > 0);
  const label = (g) => `${grouped(g)} guilds · ${grouped(g * 1000)} members`;
  const order = (r) => (isRust(r) ? 0 : 1);
  const base = (g) => twilightDefault(all, (r) => r.guilds === g);
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
  return `${picture}\n\n${table(['Guilds', 'Implementation', 'RSS', 'vs twilight', 'RSS added per member'], rows)}`;
}

function restSection(all) {
  const order = (r) => (isRust(r) ? 0 : r.variant.includes('disabled') ? 2 : 1);
  const results = [...all].sort((a, b) => order(a) - order(b));
  const base = twilightDefault(all);
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
  return `${throughput}\n\n${latency}\n\n${table(['Implementation', 'Requests/s', 'vs twilight default', 'p50', 'p99', 'CPU per request', 'Peak RSS'], rows)}`;
}

function buildSection(builds, dependencies) {
  if (!builds.length) return '_Not measured yet: run `benches/vs-twilight/build.sh`._';
  const order = (r) => (isRust(r) ? 0 : 1);
  const profiles = ['release', 'debug'];
  const of = (profile) => builds.filter((r) => r.profile === profile).sort((a, b) => order(a) - order(b));
  const timesChart = writeChart('build-time', {
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
  return `${timesChart}\n\n${size}\n\n${table(['Profile', 'Bot', 'Clean build (range)', 'Rebuild (range)', 'Binary', 'Stripped'], rows)}\n\n${table(['Bot', 'Dependencies (normal, this platform)'], deps)}`;
}

function environmentSection(e) {
  return table(
    ['', ''],
    [
      ['Date', e.date],
      ['CPU', `${e.cpu} (${e.threads} threads), ${e.memory_gib} GiB RAM, platform profile \`${e.platform_profile}\``],
      ['OS', e.kernel],
      ['Rust', `\`${e.rustc}\`, release profile (\`lto = "fat"\`, \`codegen-units = 1\`) for the harness; the bots' own defaults for the builds`],
      ['twilight', `${e.twilight}, the gateway without default features (no \`zstd\`: the mock sends plain JSON), everything else default`],
      ['CPU pinning', `clients on CPUs ${e.client_cpus}, mock servers on CPUs ${e.server_cpus}, decode loops on CPU ${e.decode_cpu}; builds use every CPU`],
      ['Runs', `${e.runs} per configuration (the median run is shown; decode: median of 15 rounds; builds: the faster of ${e.build_runs ?? 2})`],
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
  summary = { environment, scenarios: runs };
  writeFileSync(SUMMARY, `${JSON.stringify(summary, null, 2)}\n`);
} else {
  summary = JSON.parse(readFileSync(SUMMARY, 'utf8'));
}
for (const list of Object.values(summary.scenarios)) for (const r of list) r.name = name(r);

mkdirSync(CHARTS, { recursive: true });
const s = summary.scenarios;
let doc = readFileSync(DOC, 'utf8');
doc = replace(doc, 'vs-twilight', 'summary', summarySection(s));
doc = replace(doc, 'vs-twilight', 'decode', decodeSection(s.decode));
doc = replace(doc, 'vs-twilight', 'flood', floodSection(s.flood));
doc = replace(doc, 'vs-twilight', 'memory', memorySection(s.memory));
doc = replace(doc, 'vs-twilight', 'rest', restSection(s.rest));
doc = replace(doc, 'vs-twilight', 'build', buildSection(s.build, s.dependencies));
doc = replace(doc, 'vs-twilight', 'environment', environmentSection(summary.environment));
writeFileSync(DOC, doc);
console.log('wrote benches/vs-twilight/results.json, docs/vs-twilight/*.svg and docs/VS_TWILIGHT.md');
