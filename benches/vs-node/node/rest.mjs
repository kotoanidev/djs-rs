// Scenario 4 (discord.js side): REST throughput — concurrent Create Message calls against the
// mock API.
//
// Usage: node node/rest.mjs --api http://127.0.0.1:<port>/api --requests 200000 --concurrency 64
//
// Each worker posts to its own channel, so every request goes through a different rate-limit
// bucket. The proactive global limit (50/s by default in both libraries) is lifted so the
// client overhead is what is measured. A warm-up round (a tenth of the requests, which also
// opens the connections) runs first.

import { REST } from '@discordjs/rest';
import { Routes } from 'discord.js';
import { TOKEN, VERSIONS, cpuMs, emit, options, percentile, statusKib } from './common.mjs';

const args = options();
const requests = Number(args.requests ?? 200_000);
const concurrency = Number(args.concurrency ?? 64);
const rest = new REST({ api: args.api, globalRequestsPerSecond: Number.MAX_SAFE_INTEGER }).setToken(TOKEN);

/** The ID of channel `worker` of guild 0, as in `src/payloads.rs`. */
const channelId = (worker) => (1_200_000_000_000_000_000n + BigInt(worker)).toString();

async function round(count) {
  let issued = 0;
  const latencies = [];
  await Promise.all(
    Array.from({ length: concurrency }, async (_, worker) => {
      const route = Routes.channelMessages(channelId(worker));
      while (issued++ < count) {
        const started = process.hrtime.bigint();
        const message = await rest.post(route, { body: { content: 'benchmark' } });
        if (!message.id) throw new Error('the mock API should return a message');
        latencies.push(Number(process.hrtime.bigint() - started) / 1000);
      }
    }),
  );
  return latencies;
}

await round(Math.floor(requests / 10));

const cpuBefore = cpuMs();
const started = process.hrtime.bigint();
const latencies = await round(requests);
const seconds = Number(process.hrtime.bigint() - started) / 1e9;
const cpu = cpuMs() - cpuBefore;
latencies.sort((a, b) => a - b);
emit({
  scenario: 'rest',
  library: '@discordjs/rest',
  variant: 'REST#post (Routes.channelMessages)',
  versions: VERSIONS,
  requests,
  concurrency,
  seconds,
  requests_per_sec: requests / seconds,
  p50_us: Math.round(percentile(latencies, 50)),
  p99_us: Math.round(percentile(latencies, 99)),
  cpu_ms: cpu,
  cpu_us_per_request: (cpu * 1000) / requests,
  peak_rss_kib: statusKib('VmHWM'),
});
process.exit(0);
