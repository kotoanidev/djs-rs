# djs-rs vs serenity — benchmark harness

The harness behind `docs/VS_SERENITY.md` (decisions D-277–D-282). It measures djs-rs and
[serenity](https://github.com/serenity-rs/serenity) 0.12.5 (default features and settings,
D-280) on the same machine, with the same inputs and the same mock Discord servers as
[`benches/vs-node`](../vs-node/README.md). Linux only (`/proc`, `taskset`, `unshare`).

It is its own Cargo workspace with its own `Cargo.lock`, so the repository's quality gate
never builds serenity (D-278).

## How it fits together

| Part | Where |
|------|-------|
| Mock gateway and plain mock API, payloads, decode fixtures | `vs-node mock-gateway`, `vs-node mock-rest`, `vs-node fixtures` |
| djs-rs runners | `vs-node decode`, `vs-node flood --cache serenity`, `vs-node memory --cache serenity`, `vs-node rest` |
| HTTPS mock API for serenity (same responses, a CA made for the run) | `djs-vs-serenity tls-api` |
| serenity runners | `djs-vs-serenity decode`, `flood`, `memory`, `rest` |
| Shared measuring code | `djs_vs_node::metrics` (`Window`, `measure`, `/proc` readers) |

serenity's default rate limiter ignores `HttpBuilder::proxy`, so serenity keeps its
`https://discord.com` URLs and is given a `reqwest::Client` that resolves `discord.com` to the
HTTPS mock and trusts only the run's CA (D-282). Its REST numbers are also measured with the
limiter disabled over plain HTTP, its documented proxy setup, as a bound.

## Safety

Every run happens inside `unshare -rn`: a network namespace with only a loopback interface,
so nothing can reach Discord even by mistake (C-108). The runners also refuse any URL that is
not `127.0.0.1`, and serenity's client trusts no real certificate authority.
