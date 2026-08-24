# Contributors

Open Ontologies exists thanks to everyone who has built, used, broken, and improved it.

## Maintainer

- **Fabio Rovai** ([@fabio-rovai](https://github.com/fabio-rovai)) — project lead

## Contributors

- **Jioh L. Jung** ([@ziozzang](https://github.com/ziozzang)) — production user and contributor of substantial backend and runtime features ([PR #11](https://github.com/fabio-rovai/open-ontologies/pull/11)):
  - DuckDB SQL backbone alongside Postgres
  - OpenAI-compatible embeddings provider
  - Compile cache + TTL eviction + tool exposure filter
  - `ontology_dirs` config + `onto_repo_list` / `onto_repo_load` tools
  - Operational limits surfaced as `[section]` config
  - Docs alignment + resolver regression tests
- **Jason Smith** ([@rustforrecess](https://github.com/rustforrecess)) — diagnosed and fixed a real bug in the drift detector ([PR #14](https://github.com/fabio-rovai/open-ontologies/pull/14)):
  - Identified that anonymous restriction classes (and any blank-node IRIs returned by SPARQL) get freshly minted IDs on every parse, producing ~40 phantom add/remove pairs plus a Cartesian product of confidence-scored "renames" on `detect(x, x)` for typical OWL ontologies (Pizza tutorial repro). Shipped a surgical filter on the `_:` prefix in `extract_vocabulary` that bought time for the proper successor (RDFC 1.0 canonicalisation via Oxigraph 0.5.8 — landed in [2e329ee](https://github.com/fabio-rovai/open-ontologies/commit/2e329ee)).
  - PR description quality (clear repro, minimal diff, full checklist of build/test/clippy/audit, CHANGELOG entry) is the model contributor experience.

- **Nicolas Geysse** ([@nicolas-geysse](https://github.com/nicolas-geysse)) — operational hardening of the HTTP transport, and the most rigorous issue reports this project has had ([PR #78](https://github.com/fabio-rovai/open-ontologies/pull/78), [PR #79](https://github.com/fabio-rovai/open-ontologies/pull/79)):
  - `serve-http` had a graceful-shutdown `CancellationToken` that nothing ever cancelled, so the shutdown future pended forever, the process could only be killed, and the unix socket was left on disk afterwards. Fixed with ctrl-c and `SIGTERM` handling, a second-signal escape hatch for a stalled shutdown, and socket unlink on both paths.
  - An unauthenticated `/health` liveness route, correctly registered after the bearer layer so orchestrators can probe without credentials while `/api` and `/mcp` stay behind auth, plus a README table documenting that the HTTP surface is **not** read-only.
  - Seven issues filed against named commits, each citing file and line and stating the failure mode rather than the code smell. Several found silent-corruption paths rather than crashes, which is the harder class to see.
- **Ladislav Gazo** ([@lgazo](https://github.com/lgazo)) — opt-in persistent triple store ([PR #81](https://github.com/fabio-rovai/open-ontologies/pull/81)):
  - A `[storage]` backend selector for the main graph: `memory` (default, unchanged) or `persistent`, opening a RocksDB-backed Oxigraph store at `<data_dir>/triplestore` so triples survive a restart. CLI flag, env override and config, with correct precedence, plus a drop-and-reopen round-trip test.
  - Handled the case that would otherwise have quietly not worked: the one-shot CLI subcommands read the same setting, so `load` then `query` share state. Kept sandbox stores in-memory so only the main graph is ever persistent, and documented Oxigraph's single-writer-per-directory constraint rather than leaving it to be discovered.
  - He wrote this on his fork in June and never opened a PR; it was found during a fork-network audit. If you are sitting on something similar, open the PR. We would rather review it than find it.

## How to be listed here

Open a PR. If it lands on `main` (in whole or in part), you'll be added with a short note describing your contribution. Bots and machine-generated commits are not credited as people.
