# Obsidian

Open Ontologies ships as an Obsidian plugin alongside the Rust binary, the Docker image and the PyPI package. The plugin runs **this engine** as a managed sidecar, so every tool is available inside Obsidian, reasoners included.

Repository: [fabio-rovai/obsidian-open-ontologies](https://github.com/fabio-rovai/obsidian-open-ontologies)

## What it does

Two planes of data, both first-class:

1. **Ontology files in the vault.** `.ttl`, `.owl`, `.rdf` and `.jsonld` files get validate-on-save with inline diagnostics, plus reason, classify, diff and lint commands. An ontology tree pane and a SPARQL console give you a workbench without leaving Obsidian.
2. **The vault itself as RDF.** Notes become individuals, `type:` frontmatter becomes `rdf:type`, Dataview-style `property:: [[Target]]` inline fields become object properties, plain wikilinks become a configurable predicate, and tags become SKOS concepts. Once compiled, the vault is reasoned over and SHACL-validated against shapes stored in the vault.

## Agent access over MCP

The sidecar is `open-ontologies serve-http`, so it is already an MCP server. The plugin binds it to a stable loopback port (default 27125) with a generated bearer token and offers a one-click config copy, which means Claude Code or Claude Desktop can query the reasoned vault graph directly.

This is deliberately not a file-access integration. `mcp-obsidian` with the Local REST API plugin already gives an agent read, write and search over vault files; the two compose. What this channel adds is reasoning: entailment, SHACL conformance, and SPARQL over a typed graph.

```json
{
  "mcpServers": {
    "open-ontologies": {
      "type": "http",
      "url": "http://127.0.0.1:27125/mcp",
      "headers": { "Authorization": "Bearer YOUR_TOKEN" }
    }
  }
}
```

Authentication is mandatory and not configurable off. The HTTP router applies permissive CORS, so an unauthenticated listener on a fixed, documented port would be reachable cross-origin from any page in the user's browser. The bearer layer wraps `/api` and `/mcp` (only `/health` sits outside it), so the token closes that hole.

## Starter vocabulary

Entailment over an untyped vault is vacuous: without types and typed relations, the graph is a bag of "links to" triples and OWL-RL derives nothing worth querying. The plugin therefore ships a starter ontology, installed by command, defining `Note`, `Person`, `Project`, `Task`, `Source`, `Idea` and `Topic`, with `partOf` declared `owl:TransitiveProperty` and `relatesTo` declared `owl:SymmetricProperty`, plus SHACL shapes. Those two property characteristics are what give the reasoner something to derive.

## Settings reference

| Setting | Meaning |
| --- | --- |
| Engine binary path | Empty auto-downloads the pinned release (checksum-verified). Set a path to reuse an existing install. |
| Vault mapping rules (YAML) | Overrides for `iriBase` (default `vault:`), `typeKey` (`type`), `iriKey` (`iri`), `defaultLinkPredicate` (`vault:linksTo`), `tagPredicate` (`vault:hasTag`), `skipKeys` (`aliases`, `cssclasses`, `tags`). |
| Auto-sync vault to graph | Recompiles the vault ten seconds after the last note change so an MCP client always queries current data. |
| MCP port | Stable loopback port for external MCP clients. Falls back to an ephemeral port if occupied. |
| Copy MCP client config | Copies a ready-to-paste JSON block with the URL and token. |
| Regenerate access token | Issues a new token and restarts the engine, invalidating previously copied configs. |

## Compatibility

The plugin pins a compatible engine range (`^1.1`) and blocks with an update prompt rather than running against a skewed version.
