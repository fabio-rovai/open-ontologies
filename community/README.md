# Community Packs

The marketplace has two tiers:

- **Curated catalogue** — 33 W3C/ISO/industry standard ontologies, compiled into the binary and vetted by maintainers.
- **Community packs** (this directory) — an open-submission registry. Anyone can add an ontology pack by PR. The server fetches `registry.json` at runtime, so a merged pack becomes installable by every user immediately — no release needed.

A pack is **data, not code**: a manifest pointing at an ontology file the server fetches over HTTPS and loads into the triple store. Nothing in a pack executes.

## Installing community packs

```bash
onto_marketplace list                 # curated + community, each entry tagged with its source
onto_marketplace install pizza        # curated IDs are checked first, then community
```

The registry is resolved in priority order:

1. `OPEN_ONTOLOGIES_COMMUNITY_REGISTRY` — a URL or local file path (use this to pin, mirror, or run your own registry)
2. `./community/registry.json` if the server runs from a source checkout
3. The canonical registry on GitHub: `https://raw.githubusercontent.com/fabio-rovai/open-ontologies/main/community/registry.json`

If the registry cannot be fetched, `onto_marketplace` degrades gracefully to the curated catalogue and reports the error.

## Submitting a pack

Open a PR adding one object to the `packs` array in [`registry.json`](registry.json):

```json
{
  "id": "my-pack",
  "name": "Human-Readable Name",
  "description": "What it is and when a Claude session would install it.",
  "domain": "one-word-domain",
  "url": "https://example.org/my-ontology.ttl",
  "format": "turtle",
  "maintainer": "@your-github-handle",
  "homepage": "https://example.org",
  "license": "CC-BY-4.0"
}
```

| Field | Required | Rules |
| ----- | -------- | ----- |
| `id` | yes | Lowercase kebab-case (`[a-z0-9-]`), unique, must not collide with a curated ID (curated always wins) |
| `name` | yes | Human-readable name |
| `description` | yes | One or two sentences; say when an agent should install it |
| `domain` | yes | Short domain tag used for filtering (e.g. `teaching`, `maritime`, `legal`) |
| `url` | yes | Direct HTTPS link to the raw ontology file — stable, no auth, no HTML landing page |
| `format` | yes | One of `turtle`, `rdfxml`, `ntriples`, `nquads`, `trig` |
| `maintainer` | no | GitHub handle of the submitter |
| `homepage` | no | Project or documentation page |
| `license` | no (strongly encouraged) | SPDX identifier of the ontology's licence |

## Acceptance criteria

A PR is merged when:

1. `cargo test marketplace` passes — the shipped registry is validated in CI (`shipped_community_registry_is_valid`), so a malformed manifest fails the build.
2. The URL serves the raw ontology file and `onto_validate` + `onto_load` succeed on it.
3. The ontology is openly licensed (or the licence is clearly stated and permits redistribution by fetch).
4. The description honestly says what the pack is. Teaching examples, niche domain vocabularies, and drafts are all welcome — that is what the community tier is for.

Maintainers may remove packs whose URLs go dead. Pin a specific version in the URL (a tagged raw GitHub link, a w3id.org PURL) rather than a moving branch where possible.
