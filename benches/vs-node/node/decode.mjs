// Scenario 1 (discord.js side): decoding gateway payloads into the library's structures.
//
// Usage: node node/decode.mjs --dir out/fixtures

import { readFileSync } from 'node:fs';
import { Client, Guild, Message } from 'discord.js';
import { VERSIONS, emit, measure, options } from './common.mjs';

const { dir } = options();
const messageJson = readFileSync(`${dir}/message_create.json`, 'utf8');
const guildJson = readFileSync(`${dir}/guild_create.json`, 'utf8');

const client = new Client({ intents: [] });

// GUILD_CREATE: the structure the GUILD_CREATE handler builds (`client.guilds._add` is
// `new Guild` plus a cache insert). A Guild also fills the client-wide user and channel caches;
// they are cleared after each call so that every call builds those objects from scratch, as
// the first GUILD_CREATE of a guild does, instead of patching existing ones.
function guildCreate() {
  const guild = new Guild(client, JSON.parse(guildJson).d);
  client.users.cache.clear();
  client.channels.cache.clear();
  return guild;
}

// MESSAGE_CREATE: the structure the MESSAGE_CREATE handler builds, with the message's guild and
// channel cached (as in a running bot), so the member is resolved too. The message itself is
// not inserted into the channel's message cache.
function messageCreate() {
  return new Message(client, JSON.parse(messageJson).d);
}

const results = [];
const run = (payload, json, variant, library, f) => {
  const timing = measure(f);
  results.push({
    scenario: 'decode',
    payload,
    bytes: Buffer.byteLength(json),
    library,
    variant,
    versions: VERSIONS,
    ...timing,
  });
};

// Sanity checks: the structures are complete.
const guild = guildCreate();
if (guild.members.cache.size !== 1000) throw new Error('the guild should have 1000 members');
client.guilds._add(JSON.parse(guildJson).d);
const message = messageCreate();
if (!message.member || !message.channel) throw new Error('the message should resolve its member and channel');

run('MESSAGE_CREATE', messageJson, 'JSON.parse + new Message()', 'discord.js', messageCreate);
run('MESSAGE_CREATE', messageJson, 'JSON.parse only (no structures)', 'Node.js', () => JSON.parse(messageJson));
client.guilds.cache.clear();
client.users.cache.clear();
client.channels.cache.clear();
run('GUILD_CREATE', guildJson, 'JSON.parse + new Guild()', 'discord.js', guildCreate);
run('GUILD_CREATE', guildJson, 'JSON.parse only (no structures)', 'Node.js', () => JSON.parse(guildJson));
for (const result of results) emit(result);
