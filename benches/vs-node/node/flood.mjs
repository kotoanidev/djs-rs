// Scenario 2 (discord.js side): an end-to-end MESSAGE_CREATE flood through the mock gateway.
//
// Usage: node node/flood.mjs --api http://127.0.0.1:<port>/api --events 1000000 \
//          --layer client|ws [--message-cache default|off]
//
// `client` is a discord.js Client with one messageCreate listener; `ws` is a bare
// @discordjs/ws WebSocketManager (the layer the Client is built on) counting dispatches.
// The measured window runs from the first counted event to the last one.

import { REST } from '@discordjs/rest';
import { WebSocketManager, WebSocketShardEvents } from '@discordjs/ws';
import { Client, GatewayIntentBits, Options } from 'discord.js';
import { TOKEN, VERSIONS, cpuMs, emit, options, statusKib } from './common.mjs';

const args = options();
const target = Number(args.events ?? 1_000_000);
const layer = args.layer ?? 'client';
const messageCache = args['message-cache'] ?? 'default';
const intents = GatewayIntentBits.Guilds | GatewayIntentBits.GuildMessages | GatewayIntentBits.MessageContent;

let count = 0;
let opened;
let cpuBefore;

function hit(variant) {
  count += 1;
  if (count === 1) {
    opened = process.hrtime.bigint();
    cpuBefore = cpuMs();
  }
  if (count !== target) return;
  const seconds = Number(process.hrtime.bigint() - opened) / 1e9;
  const cpu = cpuMs() - cpuBefore;
  emit({
    scenario: 'flood',
    library: layer === 'client' ? 'discord.js' : '@discordjs/ws',
    variant,
    versions: VERSIONS,
    events: target,
    seconds,
    events_per_sec: (target - 1) / seconds,
    cpu_ms: cpu,
    cpu_us_per_event: (cpu * 1000) / (target - 1),
    peak_rss_kib: statusKib('VmHWM'),
  });
  // Exit without the clean disconnect; the mock server does not care.
  process.exit(0);
}

if (layer === 'client') {
  const client = new Client({
    intents,
    rest: { api: args.api },
    // discord.js keeps the last 200 messages of each channel by default.
    makeCache:
      messageCache === 'off'
        ? Options.cacheWithLimits({ ...Options.DefaultMakeCacheSettings, MessageManager: 0 })
        : Options.cacheWithLimits(Options.DefaultMakeCacheSettings),
  });
  const variant =
    messageCache === 'off' ? 'Client + messageCreate listener, message cache off' : 'Client + messageCreate listener';
  client.on('messageCreate', (message) => {
    if (message.content.length === 0) throw new Error('empty content');
    hit(variant);
  });
  await client.login(TOKEN);
} else {
  const rest = new REST({ api: args.api }).setToken(TOKEN);
  const manager = new WebSocketManager({ token: TOKEN, intents, rest });
  manager.on(WebSocketShardEvents.Dispatch, ({ data }) => {
    if (data.t === 'MESSAGE_CREATE') {
      if (data.d.content.length === 0) throw new Error('empty content');
      hit('WebSocketManager (parsed JSON dispatches)');
    }
  });
  await manager.connect();
}
