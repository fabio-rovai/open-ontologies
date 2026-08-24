# Example Plugins

Reference implementations of the [WASM plugin ABI](../../docs/plugins.md).

| Plugin | What it does |
| ------ | ------------ |
| [`label-case-lint`](label-case-lint/) | Lints class labels from injected SPARQL bindings: empty labels, stray whitespace, lowercase-initial class labels |

Each is an independent Cargo project (not part of the main build). Build with:

```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
```

and copy the resulting `.wasm` from `target/wasm32-unknown-unknown/release/` into `~/.open-ontologies/plugins/`.
