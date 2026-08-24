# WASM Plugins

The in-process extension surface: community-authored tools compiled to WebAssembly, discovered from a plugin directory, and exposed through two MCP tools — `onto_plugin_list` and `onto_plugin_call`. The host is the pure-Rust wasmi interpreter, so the single-binary story holds: no system dependencies, no dynamic linking.

Enable it at build time:

```bash
cargo build --release --features plugins
```

Without the feature, both tools exist and return a clean "compiled without plugins" error.

## Trust and capability model

Plugins are sandboxed harder than any other extension tier:

- **No imports.** ABI v1 plugins may not import host functions at all — a module that declares imports fails instantiation. No filesystem, no network, no clocks.
- **No store access.** A plugin sees graph data only when the *caller* passes `sparql` to `onto_plugin_call`; the host runs the SELECT and injects the result rows into the plugin's input as `bindings`. The capability grant is explicit, per-call, and read-only.
- **Fuel metering.** Every invocation gets a bounded instruction budget; an infinite loop traps instead of hanging the server.
- **Fresh instance per call.** Plugins are stateless by construction; nothing persists between invocations.
- **Bounded output.** Returns are capped at 16 MB.

This makes a plugin the right home for *pure validation and transformation logic*: domain-specific lint rules, naming-convention checks, metric computations, report formatting. It is the wrong home for anything needing IO — that is a [companion server](companion-servers.md).

## Installing plugins

Drop `.wasm` files into either directory (or set `OPEN_ONTOLOGIES_PLUGIN_DIRS`, colon-separated, to override both):

```
~/.open-ontologies/plugins/
./plugins/
```

`onto_plugin_list` shows what was found, the tools each plugin declares, and any files that failed to load (a broken plugin never hides the working ones).

## ABI v1

A plugin is a wasm32 module exporting:

| Export | Signature | Contract |
| ------ | --------- | -------- |
| `memory` | linear memory | Host reads results from and writes inputs into it |
| `oo_abi_version` | `() -> i32` | Must return `1` |
| `oo_alloc` | `(len: i32) -> i32` | Return a pointer to `len` writable bytes for the host's input |
| `oo_describe` | `() -> i64` | Return `(ptr << 32) \| len` of a UTF-8 JSON manifest |
| `oo_call` | `(ptr: i32, len: i32) -> i64` | Handle one invocation; return packed ptr/len of UTF-8 JSON |

The manifest:

```json
{
  "name": "my-plugin",
  "version": "0.1.0",
  "tools": [{"name": "my_tool", "description": "When an agent should call this"}]
}
```

The input document `oo_call` receives:

```json
{
  "tool": "my_tool",
  "input": { "whatever the caller passed": true },
  "bindings": [ {"class": "<http://…>", "label": "…"} ]
}
```

`bindings` is present only when the caller supplied `sparql`; it is the SELECT's result rows keyed by variable name. Return any JSON value; by convention `{"ok": true, …}` on success and `{"error": "…"}` on failure.

## Writing a plugin in Rust

The reference implementation is [`examples/plugins/label-case-lint`](../examples/plugins/label-case-lint/) — a label-convention linter over injected bindings. Build and install:

```bash
rustup target add wasm32-unknown-unknown
cd examples/plugins/label-case-lint
cargo build --release --target wasm32-unknown-unknown
mkdir -p ~/.open-ontologies/plugins
cp target/wasm32-unknown-unknown/release/label_case_lint.wasm ~/.open-ontologies/plugins/
```

Then, in a session:

```text
onto_plugin_list
onto_plugin_call plugin=label-case-lint tool=lint_labels \
  sparql='SELECT ?class ?label WHERE { ?class a <http://www.w3.org/2002/07/owl#Class> ; <http://www.w3.org/2000/01/rdf-schema#label> ?label }'
```

Any language that compiles to freestanding wasm32 with exported functions works the same way — the test suite ([`tests/plugin_host_test.rs`](../tests/plugin_host_test.rs)) exercises the ABI with plugins hand-written in WAT.

## Design position

Plugins hold the project's MCP-native convention at the binary level: they are validation/scaffolding primitives, not agents. If your plugin wants to call an LLM, phone home, or orchestrate — flip the design (see [CLAUDE.md](../CLAUDE.md), "Architecture Convention"). The orchestrator judges; plugins compute.

ABI v1 is deliberately minimal. Planned, gated on real demand: host-function imports under named capabilities (e.g. a plugin declaring `sparql-select` gets a host import instead of caller-injected bindings), and dynamic MCP tool registration (`tools/list_changed`) so plugin tools appear as first-class `onto_*` entries.
