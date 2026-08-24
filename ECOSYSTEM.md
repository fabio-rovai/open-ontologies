# Ecosystem

Open Ontologies is extensible on four surfaces, in increasing order of coupling:

| Surface | What it is | Contribute via |
| ------- | ---------- | -------------- |
| **Community packs** | Ontology data packs, installable through `onto_marketplace` at runtime | PR to [`community/registry.json`](community/registry.json) — see [community/README.md](community/README.md) |
| **Community skills** | Markdown workflow recipes that teach agents to chain `onto_*` tools | PR to [`skills/community/`](skills/community/) |
| **Companion servers** | Independent MCP servers composing with the core in one session | Follow the [companion contract](docs/companion-servers.md), then PR a row below |
| **WASM plugins** | Community `onto_*`-adjacent tools running in-process, sandboxed | See [docs/plugins.md](docs/plugins.md) |

## Companion servers

| Name | What it does | Contract surface |
| ---- | ------------ | ---------------- |
| [OpenCheir](https://github.com/fabio-rovai/opencheir) | Workflow governance — enforcer rules over ontology-engineering sessions (validate-after-save, version-before-push), automatic verdict logging | Lineage webhook consumer (`GOVERNANCE_WEBHOOK`) |

## Clients & surfaces

| Name | What it does |
| ---- | ------------ |
| [Studio](studio/) | Desktop app (Tauri) wrapping the engine — ontology tree, AI chat panel, property inspector, lineage viewer |
| [Obsidian plugin](https://github.com/fabio-rovai/obsidian-open-ontologies) | Brings the engine to Obsidian vaults — notes as a knowledge-graph surface |

## Community packs (highlights)

The live registry is [`community/registry.json`](community/registry.json); `onto_marketplace list` shows the current set with `"source": "community"`.

## Getting listed

- **Packs and skills**: merged PR = listed, no separate step.
- **Companion servers**: PR a row to the table above; criteria in [docs/companion-servers.md](docs/companion-servers.md#getting-listed).
