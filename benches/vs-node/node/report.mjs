// Turns the raw results of run.sh into the committed summary (benches/vs-node/results.json),
// the SVG charts (docs/vs-node/*.svg, light and dark) and the tables and charts between the
// `<!-- vs-node:NAME:start/end -->` markers of docs/VS_NODE.md. No dependencies.
//
// Usage: node node/report.mjs out/results.jsonl out/environment.json   (after run.sh)
//        node node/report.mjs                                          (re-render results.json)

import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';

const here = (path) => new URL(`../${path}`, import.meta.url);
const SUMMARY = here('results.json');
const DOC = here('../../docs/VS_NODE.md');
const CHARTS = here('../../docs/vs-node/');

// ---------------------------------------------------------------------------------------------
// Aggregation: one entry per configuration, the median run by its primary metric.

const PRIMARY = {
  decode: 'median_ns',
  flood: 'events_per_sec',
  memory: 'rss_kib',
  rest: 'requests_per_sec',
};

function aggregate(lines, environment) {
  const groups = new Map();
  for (const result of lines) {
    const key = [result.scenario, result.payload, result.guilds, result.library, result.variant, result.allocator].join('|');
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(result);
  }
  const scenarios = { decode: [], flood: [], memory: [], rest: [] };
  for (const runs of groups.values()) {
    const metric = PRIMARY[runs[0].scenario];
    const sorted = [...runs].sort((a, b) => a[metric] - b[metric]);
    const median = sorted[Math.floor((sorted.length - 1) / 2)];
    const { versions, ...rest } = median;
    scenarios[median.scenario].push({
      ...rest,
      runs: runs.length,
      range: [sorted[0][metric], sorted[sorted.length - 1][metric]],
    });
  }
  return { environment, scenarios };
}

// ---------------------------------------------------------------------------------------------
// Names and comparisons.

function name(r) {
  const mimalloc = r.allocator === 'mimalloc' ? ' + mimalloc' : '';
  switch (r.scenario) {
    case 'decode':
      if (r.library === 'djs-rs') return `djs-rs (djs-model)${mimalloc}`;
      if (r.library === 'discord.js') return `discord.js: JSON.parse + new ${r.payload === 'GUILD_CREATE' ? 'Guild' : 'Message'}()`;
      return 'Node.js: JSON.parse only';
    case 'flood':
      if (r.variant.startsWith('djs-gateway')) return 'djs-gateway Shard';
      if (r.library === '@discordjs/ws') return '@discordjs/ws WebSocketManager';
      if (r.library === 'discord.js') return r.variant.includes('cache off') ? 'discord.js Client, message cache off' : 'discord.js Client (default)';
      if (r.variant.includes('Sequential')) return `djs-rs Client, sequential handlers${mimalloc}`;
      return mimalloc ? 'djs-rs Client + mimalloc' : 'djs-rs Client (default)';
    case 'memory':
      return r.library === 'djs-rs' ? `djs-rs${mimalloc}` : 'discord.js Client (default caches)';
    default:
      return r.library === 'djs-rs' ? `djs-rest${mimalloc}` : '@discordjs/rest';
  }
}

const isRust = (r) => r.library === 'djs-rs';
const transport = (r) => r.variant.startsWith('djs-gateway') || r.library === '@discordjs/ws';

/** The Node.js configuration a result is compared with. */
function baselineOf(r, all) {
  switch (r.scenario) {
    case 'decode':
      return all.find((b) => b.payload === r.payload && b.library === 'discord.js');
    case 'flood':
      return all.find((b) => (transport(r) ? b.library === '@discordjs/ws' : b.name === 'discord.js Client (default)'));
    case 'memory':
      return all.find((b) => b.guilds === r.guilds && b.library === 'discord.js');
    default:
      return all.find((b) => b.library === '@discordjs/rest');
  }
}

// ---------------------------------------------------------------------------------------------
// Formatting.

const grouped = (value, digits = 0) =>
  value.toLocaleString('en-US', { minimumFractionDigits: digits, maximumFractionDigits: digits });
const compact = (value) =>
  value >= 1e6 ? `${(value / 1e6).toFixed(2)} M` : value >= 1e4 ? `${(value / 1e3).toFixed(0)} k` : grouped(value);
const times = (value) => `${value >= 10 ? value.toFixed(0) : value.toFixed(1)}×`;
const mib = (kib) => `${(kib / 1024).toFixed(1)} MiB`;
const micros = (value) => `${value.toFixed(value < 10 ? 2 : 1)} µs`;
const millis = (us) => `${(us / 1000).toFixed(2)} ms`;
const duration = (ns) => (ns >= 1e6 ? `${(ns / 1e6).toFixed(2)} ms` : `${(ns / 1e3).toFixed(2)} µs`);

// ---------------------------------------------------------------------------------------------
// SVG bar charts (horizontal bars, one baseline, values at the bar ends).

const THEMES = {
  light: {
    surface: '#fcfcfb', primary: '#0b0b0b', secondary: '#52514e', muted: '#898781',
    grid: '#e1e0d9', axis: '#c3c2b7', rust: '#2a78d6', node: '#eb6834', border: 'rgba(11,11,11,0.10)',
  },
  dark: {
    surface: '#1a1a19', primary: '#f0efec', secondary: '#c3c2b7', muted: '#898781',
    grid: '#2c2c2a', axis: '#383835', rust: '#3987e5', node: '#d95926', border: 'rgba(255,255,255,0.10)',
  },
};
const FONT = 'system-ui, -apple-system, &quot;Segoe UI&quot;, Roboto, sans-serif';
const escape = (text) => String(text).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

function niceStep(max) {
  const raw = max / 4;
  const power = 10 ** Math.floor(Math.log10(raw));
  return [1, 2, 2.5, 5, 10].map((m) => m * power).find((step) => step >= raw);
}

/** A bar whose data end (right) has 4 px rounded corners and whose baseline end is square. */
function bar(x, y, width, height) {
  const r = Math.min(4, width / 2, height / 2);
  if (width <= 0) return '';
  return `M${x},${y}H${x + width - r}A${r},${r} 0 0 1 ${x + width},${y + r}V${y + height - r}A${r},${r} 0 0 1 ${x + width - r},${y + height}H${x}Z`;
}

/**
 * Renders a chart. `sections` is a list of `{ title?, rows: [{ label, value, rust, text }] }`;
 * `text` is printed at the bar end. `legend: false` hides the legend (single-series charts).
 */
function chart({ title, subtitle, sections, tick, legend = true, single = false }, themeName) {
  const t = THEMES[themeName];
  const width = 840;
  const pad = 24;
  const labelWidth = 300;
  const reserve = 230;
  const plotX = pad + labelWidth;
  const plotWidth = width - plotX - reserve - pad;
  const rowHeight = 32;
  const barHeight = 20;
  const sectionHeight = 30;
  const max = Math.max(...sections.flatMap((s) => s.rows.map((r) => r.value)));
  const step = niceStep(max);
  const top = Math.ceil(max / step) * step;
  const scale = (value) => (value / top) * plotWidth;

  let y = legend ? 100 : 76;
  const body = [];
  const plotTop = y;
  for (const section of sections) {
    if (section.title) {
      body.push(`<text x="${pad}" y="${y + 19}" font-size="12.5" font-weight="600" fill="${t.secondary}">${escape(section.title)}</text>`);
      y += sectionHeight;
    }
    for (const row of section.rows) {
      const barY = y + (rowHeight - barHeight) / 2;
      const color = single ? t.rust : row.rust ? t.rust : t.node;
      const end = plotX + scale(row.value);
      body.push(
        `<text x="${plotX - 12}" y="${y + rowHeight / 2 + 4.5}" font-size="13" text-anchor="end" fill="${t.primary}">${escape(row.label)}</text>`,
        `<path d="${bar(plotX, barY, scale(row.value), barHeight)}" fill="${color}"><title>${escape(`${row.label}: ${row.text}`)}</title></path>`,
        `<text x="${end + 8}" y="${y + rowHeight / 2 + 4.5}" font-size="12.5" fill="${t.primary}" style="font-variant-numeric: tabular-nums">${escape(row.text)}</text>`,
      );
      y += rowHeight;
    }
  }
  const plotBottom = y + 4;
  const grid = [];
  for (let value = 0; value <= top + step / 2; value += step) {
    const x = plotX + scale(value);
    grid.push(
      `<line x1="${x}" y1="${plotTop}" x2="${x}" y2="${plotBottom}" stroke="${value === 0 ? t.axis : t.grid}" stroke-width="1"/>`,
      `<text x="${x}" y="${plotBottom + 16}" font-size="11" text-anchor="middle" fill="${t.muted}" style="font-variant-numeric: tabular-nums">${escape(tick(value))}</text>`,
    );
  }
  const height = plotBottom + 32;
  const legendItems = legend
    ? [
        `<rect x="${pad}" y="68" width="12" height="12" rx="3" fill="${t.rust}"/>`,
        `<text x="${pad + 18}" y="78.5" font-size="12.5" fill="${t.secondary}">djs-rs (Rust)</text>`,
        `<rect x="${pad + 126}" y="68" width="12" height="12" rx="3" fill="${t.node}"/>`,
        `<text x="${pad + 144}" y="78.5" font-size="12.5" fill="${t.secondary}">Node.js (discord.js and its packages)</text>`,
      ]
    : [];
  const description = sections
    .flatMap((s) => s.rows.map((r) => `${s.title ? `${s.title}, ` : ''}${r.label}: ${r.text}`))
    .join('; ');
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" role="img" aria-labelledby="title desc" font-family="${FONT}">
<title id="title">${escape(title)}</title>
<desc id="desc">${escape(description)}</desc>
<rect x="0.5" y="0.5" width="${width - 1}" height="${height - 1}" rx="8" fill="${t.surface}" stroke="${t.border}"/>
<text x="${pad}" y="34" font-size="16" font-weight="600" fill="${t.primary}">${escape(title)}</text>
<text x="${pad}" y="55" font-size="12.5" fill="${t.secondary}">${escape(subtitle)}</text>
${legendItems.join('\n')}
${grid.join('\n')}
${body.join('\n')}
</svg>
`;
}

function writeChart(file, spec) {
  for (const theme of ['light', 'dark']) {
    writeFileSync(new URL(`${file}-${theme}.svg`, CHARTS), chart(spec, theme));
  }
  const alt = `${spec.title} — ${spec.subtitle}. The table below has the numbers.`;
  return `<picture>
  <source media="(prefers-color-scheme: dark)" srcset="vs-node/${file}-dark.svg">
  <img alt="${escape(alt)}" src="vs-node/${file}-light.svg">
</picture>`;
}

// ---------------------------------------------------------------------------------------------
// Sections of docs/VS_NODE.md.

function table(header, rows) {
  const line = (cells) => `| ${cells.join(' | ')} |`;
  return [line(header), line(header.map((_, i) => (i === 0 ? '---' : '---:'))), ...rows.map(line)].join('\n');
}

function decodeSection(all) {
  const parts = [];
  const rows = [];
  for (const payload of ['MESSAGE_CREATE', 'GUILD_CREATE']) {
    const results = all.filter((r) => r.payload === payload).sort((a, b) => a.median_ns - b.median_ns);
    const base = baselineOf(results[0], all);
    const perSecond = (r) => 1e9 / r.median_ns;
    const size = `${Math.round(results[0].bytes / 1024)} KiB`;
    const label = payload === 'GUILD_CREATE' ? `GUILD_CREATE, 1000 members (${size})` : `MESSAGE_CREATE (${size})`;
    parts.push(
      writeChart(`decode-${payload === 'GUILD_CREATE' ? 'guild' : 'message'}`, {
        title: `Decoding ${label}`,
        subtitle: 'payloads per second on one core, higher is better · ratios vs discord.js · median of 15 rounds',
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
      rows.push([
        `\`${payload}\``,
        r.name,
        duration(r.median_ns),
        grouped(perSecond(r)),
        r === base ? 'baseline' : `**${times(base.median_ns / r.median_ns)}**`,
      ]);
    }
  }
  return `${parts.join('\n\n')}\n\n${table(['Payload', 'Implementation', 'Time per payload', 'Payloads/s', 'vs discord.js'], rows)}`;
}

function floodSection(all) {
  const results = [...all].sort((a, b) => b.events_per_sec - a.events_per_sec);
  const vs = (r) => {
    const base = baselineOf(r, all);
    return r === base ? 'baseline' : `**${times(r.events_per_sec / base.events_per_sec)}**`;
  };
  const vsCpu = (r) => {
    const base = baselineOf(r, all);
    return r === base ? 'baseline' : `**${times(base.cpu_us_per_event / r.cpu_us_per_event)}** less`;
  };
  const sections = [
    { title: 'Full client (events reach a handler / listener)', rows: results.filter((r) => !transport(r)) },
    { title: 'Gateway layer only (djs-gateway vs @discordjs/ws)', rows: results.filter(transport) },
  ];
  const throughput = writeChart('flood-throughput', {
    title: 'MESSAGE_CREATE flood through a mock gateway',
    subtitle: 'events per second, higher is better · 1 connection, 4 client CPUs · median of runs',
    tick: (v) => compact(v),
    sections: sections.map((s) => ({
      title: s.title,
      rows: s.rows.map((r) => ({ label: r.name, value: r.events_per_sec, rust: isRust(r), text: `${compact(r.events_per_sec)}/s` })),
    })),
  });
  const cpu = writeChart('flood-cpu', {
    title: 'CPU time per event during the flood',
    subtitle: 'microseconds of CPU (all threads) per event, lower is better',
    tick: (v) => `${v} µs`,
    sections: sections.map((s) => ({
      title: s.title,
      rows: [...s.rows]
        .sort((a, b) => a.cpu_us_per_event - b.cpu_us_per_event)
        .map((r) => ({ label: r.name, value: r.cpu_us_per_event, rust: isRust(r), text: micros(r.cpu_us_per_event) })),
    })),
  });
  const rows = sections.flatMap((s) =>
    s.rows.map((r) => [
      transport(r) ? 'gateway layer' : 'full client',
      r.name,
      grouped(r.events_per_sec),
      vs(r),
      micros(r.cpu_us_per_event),
      vsCpu(r),
      mib(r.peak_rss_kib),
    ]),
  );
  return `${throughput}\n\n${cpu}\n\n${table(['Layer', 'Implementation', 'Events/s', 'vs Node.js', 'CPU per event', 'vs Node.js', 'Peak RSS'], rows)}`;
}

function memorySection(all) {
  const counts = [...new Set(all.map((r) => r.guilds))].sort((a, b) => a - b);
  const loaded = counts.filter((g) => g > 0);
  const label = (g) => `${grouped(g)} guilds · ${grouped(g * 1000)} members`;
  const order = (r) => (r.library === 'djs-rs' ? (r.allocator === 'system' ? 0 : 1) : 2);
  const picture = writeChart('memory', {
    title: 'Memory after receiving the guilds at startup',
    subtitle: 'resident memory (RSS) of the whole process, lower is better · Node.js after a full GC',
    tick: (v) => `${v} MiB`,
    sections: loaded.map((g) => ({
      title: label(g),
      rows: all
        .filter((r) => r.guilds === g)
        .sort((a, b) => order(a) - order(b))
        .map((r) => {
          const base = baselineOf(r, all);
          return {
            label: r.name,
            value: r.rss_kib / 1024,
            rust: isRust(r),
            text: `${mib(r.rss_kib)}${r === base ? '' : ` · ${times(base.rss_kib / r.rss_kib)} less`}`,
          };
        }),
    })),
  });
  const rows = [];
  for (const g of counts) {
    for (const r of all.filter((x) => x.guilds === g).sort((a, b) => order(a) - order(b))) {
      const base = baselineOf(r, all);
      const added = r.rss_kib - r.connected_rss_kib;
      rows.push([
        g === 0 ? 'connected, no guilds' : label(g),
        r.name,
        mib(r.rss_kib),
        r === base ? 'baseline' : `**${times(base.rss_kib / r.rss_kib)}** less`,
        g === 0 ? '—' : `${grouped((added * 1024) / r.members)} B`,
        r.heap_used_kib === undefined ? '—' : mib(r.heap_used_kib),
      ]);
    }
  }
  return `${picture}\n\n${table(['Guilds', 'Implementation', 'RSS', 'vs discord.js', 'RSS added per member', 'JS heap used'], rows)}`;
}

function restSection(all) {
  const order = (r) => (r.library === 'djs-rs' ? (r.allocator === 'system' ? 0 : 1) : 2);
  const results = [...all].sort((a, b) => order(a) - order(b));
  const base = baselineOf(results[0], all);
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
          text: `${compact(r.requests_per_sec)}/s${r === base ? '' : ` · ${times(r.requests_per_sec / base.requests_per_sec)}`}`,
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
    r === base ? 'baseline' : `**${times(r.requests_per_sec / base.requests_per_sec)}**`,
    millis(r.p50_us),
    millis(r.p99_us),
    micros(r.cpu_us_per_request),
    mib(r.peak_rss_kib),
  ]);
  return `${throughput}\n\n${latency}\n\n${table(['Implementation', 'Requests/s', 'vs Node.js', 'p50', 'p99', 'CPU per request', 'Peak RSS'], rows)}`;
}

function summarySection(s) {
  const find = (list, predicate) => list.find(predicate);
  const rust = (list, extra = () => true) => find(list, (r) => isRust(r) && r.allocator === 'system' && extra(r));
  const node = (list, extra = () => true) => find(list, (r) => !isRust(r) && extra(r));
  const message = (r) => r.payload === 'MESSAGE_CREATE';
  const guild = (r) => r.payload === 'GUILD_CREATE';
  const decodeNode = (p) => (r) => p(r) && r.library === 'discord.js';
  const clientDefault = (r) => r.name === 'djs-rs Client (default)' || r.name === 'discord.js Client (default)';
  const most = Math.max(...s.memory.map((r) => r.guilds));
  const big = (r) => r.guilds === most;
  const idle = (r) => r.guilds === 0;
  const items = [
    ['Decode `MESSAGE_CREATE`', rust(s.decode, message), node(s.decode, decodeNode(message)), (r) => duration(r.median_ns), (a, b) => b.median_ns / a.median_ns, 'faster'],
    ['Decode `GUILD_CREATE` (1000 members)', rust(s.decode, guild), node(s.decode, decodeNode(guild)), (r) => duration(r.median_ns), (a, b) => b.median_ns / a.median_ns, 'faster'],
    ['Event flood, default clients (events/s)', rust(s.flood, clientDefault), node(s.flood, clientDefault), (r) => grouped(r.events_per_sec), (a, b) => a.events_per_sec / b.events_per_sec, 'more'],
    ['Event flood, gateway layer (events/s)', rust(s.flood, transport), node(s.flood, transport), (r) => grouped(r.events_per_sec), (a, b) => a.events_per_sec / b.events_per_sec, 'more'],
    ['Memory, idle bot (RSS, connected, no guilds)', rust(s.memory, idle), node(s.memory, idle), (r) => mib(r.rss_kib), (a, b) => b.rss_kib / a.rss_kib, 'less'],
    [`Memory, ${grouped(most * 1000)} members (RSS)`, rust(s.memory, big), node(s.memory, big), (r) => mib(r.rss_kib), (a, b) => b.rss_kib / a.rss_kib, 'less'],
    ['REST throughput (requests/s)', rust(s.rest), node(s.rest), (r) => grouped(r.requests_per_sec), (a, b) => a.requests_per_sec / b.requests_per_sec, 'more'],
    ['REST p99 latency', rust(s.rest), node(s.rest), (r) => millis(r.p99_us), (a, b) => b.p99_us / a.p99_us, 'lower'],
  ];
  const picture = writeChart('summary', {
    title: 'How many times better djs-rs is than discord.js',
    subtitle: 'ratio per scenario (default configurations, system allocator), higher is better',
    tick: (v) => `${v}×`,
    legend: false,
    single: true,
    sections: [
      {
        rows: items
          .map(([label, a, b, , ratio, word]) => ({ label: label.replace(/`/g, ''), value: ratio(a, b), text: `${times(ratio(a, b))} ${word}` }))
          .sort((x, y) => y.value - x.value),
      },
    ],
  });
  const rows = items.map(([label, a, b, show, ratio, word]) => [label, show(a), show(b), `**${times(ratio(a, b))} ${word}**`]);
  return `${picture}\n\n${table(['Scenario', 'djs-rs', 'discord.js', 'djs-rs advantage'], rows)}`;
}

function environmentSection(e) {
  return table(
    ['', ''],
    [
      ['Date', e.date],
      ['CPU', `${e.cpu} (${e.threads} threads), ${e.memory_gib} GiB RAM`],
      ['OS', e.kernel],
      ['Rust', `\`${e.rustc}\`, release profile (\`lto = "fat"\`, \`codegen-units = 1\`)`],
      ['Node.js', `v${e.node}, discord.js ${e['discord.js']}, @discordjs/ws ${e['@discordjs/ws']}, @discordjs/rest ${e['@discordjs/rest']}`],
      ['CPU pinning', `clients on CPUs ${e.client_cpus}, mock servers on CPUs ${e.server_cpus}, decode loops on CPU ${e.decode_cpu}`],
      ['Runs', `${e.runs} per configuration (the median run is shown; decode: median of 15 rounds)`],
    ],
  );
}

function replace(doc, marker, content) {
  const start = `<!-- vs-node:${marker}:start -->`;
  const end = `<!-- vs-node:${marker}:end -->`;
  const from = doc.indexOf(start);
  const to = doc.indexOf(end);
  if (from < 0 || to < 0) throw new Error(`docs/VS_NODE.md has no ${marker} markers`);
  return `${doc.slice(0, from + start.length)}\n${content}\n${doc.slice(to)}`;
}

// ---------------------------------------------------------------------------------------------

const [resultsFile, environmentFile] = process.argv.slice(2);
let summary;
if (resultsFile) {
  const lines = readFileSync(resultsFile, 'utf8').split('\n').filter(Boolean).map((line) => JSON.parse(line));
  summary = aggregate(lines, JSON.parse(readFileSync(environmentFile, 'utf8')));
  writeFileSync(SUMMARY, `${JSON.stringify(summary, null, 2)}\n`);
} else {
  summary = JSON.parse(readFileSync(SUMMARY, 'utf8'));
}
for (const list of Object.values(summary.scenarios)) for (const r of list) r.name = name(r);

mkdirSync(CHARTS, { recursive: true });
let doc = readFileSync(DOC, 'utf8');
doc = replace(doc, 'summary', summarySection(summary.scenarios));
doc = replace(doc, 'decode', decodeSection(summary.scenarios.decode));
doc = replace(doc, 'flood', floodSection(summary.scenarios.flood));
doc = replace(doc, 'memory', memorySection(summary.scenarios.memory));
doc = replace(doc, 'rest', restSection(summary.scenarios.rest));
doc = replace(doc, 'environment', environmentSection(summary.environment));
writeFileSync(DOC, doc);
console.log('wrote benches/vs-node/results.json, docs/vs-node/*.svg and docs/VS_NODE.md');
