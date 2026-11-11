// Scenario 3 (discord.js side): the memory taken by the guilds a bot receives at startup.
//
// Usage: node --expose-gc node/memory.mjs --api http://127.0.0.1:<port>/api --guilds 100
//
// A discord.js Client with the default caches logs in to the mock gateway, which sends
// `--guilds` GUILD_CREATEs. Once the client is ready (every guild arrived), the garbage
// collector runs and the resident memory is read.

import { Client, GatewayIntentBits } from 'discord.js';
import { TOKEN, VERSIONS, emit, options, statusKib } from './common.mjs';

if (typeof globalThis.gc !== 'function') throw new Error('run with node --expose-gc');

const args = options();
const client = new Client({
  intents: GatewayIntentBits.Guilds | GatewayIntentBits.GuildMembers,
  rest: { api: args.api },
});

let connectedRss = 0;
client.on('raw', (packet) => {
  if (packet.t === 'READY') {
    globalThis.gc();
    connectedRss = statusKib('VmRSS');
  }
});

client.on('clientReady', () => {
  globalThis.gc();
  const members = client.guilds.cache.reduce((sum, guild) => sum + guild.members.cache.size, 0);
  emit({
    scenario: 'memory',
    library: 'discord.js',
    variant: 'Client with the default caches',
    versions: VERSIONS,
    guilds: client.guilds.cache.size,
    members,
    connected_rss_kib: connectedRss,
    rss_kib: statusKib('VmRSS'),
    heap_used_kib: Math.round(process.memoryUsage().heapUsed / 1024),
    peak_rss_kib: statusKib('VmHWM'),
  });
  process.exit(0);
});

await client.login(TOKEN);
