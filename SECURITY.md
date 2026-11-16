# Security policy

## Supported versions

djs-rs is in alpha. Security fixes go into the newest release only; there are no backports
to older alphas.

| Version | Supported |
|---|---|
| newest `0.1.0-alpha.*` | yes |
| anything older | no |

## Reporting a vulnerability

**Please do not open a public issue.** Report it privately through GitHub:
[**Report a vulnerability**](https://github.com/kotoanidev/djs-rs/security/advisories/new)
(the repository's *Security* tab). Only the maintainers see the report.

Include what you can of:

- the crate and version (or commit) affected;
- what an attacker can do, and what they need (a malicious server, a crafted gateway or
  voice payload, a user's message, …);
- a proof of concept or the steps to reproduce it;
- a fix, if you have one in mind.

Never include a working bot token, yours or anyone's.

## What happens next

The maintainers aim to acknowledge a report within a week, then agree with the reporter on
a fix and a disclosure date. The fix is released first; the advisory is published with it,
crediting the reporter unless they ask not to be named.

## Scope

In scope: the crates of this repository: anything that lets data from Discord, from other
users or from the network crash a bot, run code, leak its token or secrets, or weaken a
protection the documentation promises (for example the encryption of voice calls).

Out of scope: vulnerabilities of Discord itself (report those to
[Discord](https://discord.com/security)), and of dependencies, unless djs-rs uses them
unsafely; problems that need an attacker who already controls the bot's machine or token.
