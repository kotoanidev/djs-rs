// The chart and table helpers shared by the report generators of benches/vs-node
// (node/report.mjs → docs/VS_NODE.md) and benches/vs-serenity (report.mjs →
// docs/VS_SERENITY.md): number formatting, horizontal SVG bar charts in a light and a dark
// theme, Markdown tables, and replacing the generated parts of a page. No dependencies.

import { writeFileSync } from 'node:fs';

// ---------------------------------------------------------------------------------------------
// Formatting.

export const grouped = (value, digits = 0) =>
  value.toLocaleString('en-US', { minimumFractionDigits: digits, maximumFractionDigits: digits });
export const compact = (value) =>
  value >= 1e6 ? `${(value / 1e6).toFixed(2)} M` : value >= 1e4 ? `${(value / 1e3).toFixed(0)} k` : grouped(value);
export const times = (value) => `${value >= 10 ? value.toFixed(0) : value.toFixed(1)}×`;
export const mib = (kib) => `${(kib / 1024).toFixed(1)} MiB`;
export const micros = (value) => `${value.toFixed(value < 10 ? 2 : 1)} µs`;
export const millis = (us) => `${(us / 1000).toFixed(2)} ms`;
export const duration = (ns) => (ns >= 1e6 ? `${(ns / 1e6).toFixed(2)} ms` : `${(ns / 1e3).toFixed(2)} µs`);

// ---------------------------------------------------------------------------------------------
// SVG bar charts (horizontal bars, one baseline, values at the bar ends).

export const THEMES = {
  light: {
    surface: '#fcfcfb', primary: '#0b0b0b', secondary: '#52514e', muted: '#898781',
    grid: '#e1e0d9', axis: '#c3c2b7', rust: '#2a78d6', node: '#eb6834', border: 'rgba(11,11,11,0.10)',
  },
  dark: {
    surface: '#1a1a19', primary: '#f0efec', secondary: '#c3c2b7', muted: '#898781',
    grid: '#2c2c2a', axis: '#383835', rust: '#3987e5', node: '#d95926', border: 'rgba(255,255,255,0.10)',
  },
};
export const FONT = 'system-ui, -apple-system, &quot;Segoe UI&quot;, Roboto, sans-serif';
export const escape = (text) => String(text).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

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
 * `text` is printed at the bar end. `legend: false` hides the legend (single-series charts);
 * `other` names the second series in the legend.
 */
export function chart({ title, subtitle, sections, tick, legend = true, single = false }, themeName, other = 'Node.js (discord.js and its packages)') {
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
        `<text x="${pad + 144}" y="78.5" font-size="12.5" fill="${t.secondary}">${escape(other)}</text>`,
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

/**
 * Returns `writeChart(file, spec)`, which writes the light and dark SVGs of a chart to
 * `directory` (a file URL ending in `/`) and returns the `<picture>` element for a page in which
 * they are at `prefix/`. `other` names the second series in the legend.
 */
export function chartWriter(directory, prefix, other) {
  return (file, spec) => {
    for (const theme of ['light', 'dark']) {
      writeFileSync(new URL(`${file}-${theme}.svg`, directory), chart(spec, theme, other));
    }
    const alt = `${spec.title} — ${spec.subtitle}. The table below has the numbers.`;
    return `<picture>
  <source media="(prefers-color-scheme: dark)" srcset="${prefix}/${file}-dark.svg">
  <img alt="${escape(alt)}" src="${prefix}/${file}-light.svg">
</picture>`;
  };
}

// ---------------------------------------------------------------------------------------------
// Markdown.

export function table(header, rows) {
  const line = (cells) => `| ${cells.join(' | ')} |`;
  return [line(header), line(header.map((_, i) => (i === 0 ? '---' : '---:'))), ...rows.map(line)].join('\n');
}

/** Replaces what is between `<!-- PREFIX:MARKER:start -->` and `<!-- PREFIX:MARKER:end -->`. */
export function replace(doc, prefix, marker, content) {
  const start = `<!-- ${prefix}:${marker}:start -->`;
  const end = `<!-- ${prefix}:${marker}:end -->`;
  const from = doc.indexOf(start);
  const to = doc.indexOf(end);
  if (from < 0 || to < 0) throw new Error(`the page has no ${prefix}:${marker} markers`);
  return `${doc.slice(0, from + start.length)}\n${content}\n${doc.slice(to)}`;
}
