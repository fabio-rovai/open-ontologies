# Open Ontologies for Obsidian — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An Obsidian community plugin that runs the full Open Ontologies engine (all 70+ tools, reasoners included) as a managed native sidecar, with an ontology-file workbench and a vault→RDF mapper.

**Architecture:** TypeScript plugin in a new sibling repo `obsidian-open-ontologies`. An `EngineManager` auto-downloads the platform release binary (SHA-256 verified against the published `SHASUMS.txt`), spawns `open-ontologies serve-http` on a free loopback port, and health-checks it via `GET /health`. An `EngineClient` speaks MCP streamable-HTTP JSON-RPC to `POST /mcp`. Pure-logic modules (IRI minting, mapping rules, vault→RDF mapper, asset resolution) are Obsidian-free and unit-tested; UI panes are thin `ItemView` shells over tested render helpers.

**Tech Stack:** TypeScript, esbuild, vitest, js-yaml, Obsidian plugin API, Node `child_process`/`net`/`crypto` (desktop-only).

## Global Constraints

- Plugin repo: `/Users/fabio/projects/obsidian-open-ontologies` (new git repo). Plan/spec live in the engine repo.
- Plugin id `open-ontologies`, name "Open Ontologies", `isDesktopOnly: true`, `minAppVersion: "1.5.0"`, initial version `0.1.0`.
- Engine consumed **as released, zero Rust changes**. Pinned `ENGINE_VERSION = "1.1.1"`; compatibility rule `major === 1 && minor >= 1`.
- Engine release facts (verified 2026-08-14): tags are `v`-prefixed (`v1.1.1`); assets are `open-ontologies-aarch64-apple-darwin`, `open-ontologies-x86_64-apple-darwin`, `open-ontologies-x86_64-unknown-linux-gnu`, `open-ontologies-x86_64-pc-windows-msvc.exe`, plus `SHASUMS.txt` (sha256sum format, two-space separator).
- Sidecar CLI: `open-ontologies serve-http --host 127.0.0.1 --port <p> --token <t>`. Defaults to 127.0.0.1 already; we pass it explicitly anyway. MCP endpoint `http://127.0.0.1:<p>/mcp`; liveness `GET /health` → `{"status":"ok","version":"1.1.1"}` (outside the bearer layer, so health checks need no token).
- **Auth is mandatory.** The engine applies `CorsLayer::permissive()` to the router (verified in `src/main.rs`), so a fixed, documented port without a token would be reachable cross-origin from any page in the user's browser. The plugin generates a 32-byte hex token on first run and never spawns the engine without one. No setting disables it.
- Stable port default **27125** (beside Local REST API's 27124), so an external MCP client can be configured once. Falls back to an ephemeral port with a notice if occupied.
- MCP client config emitted by settings (verified against current Claude Code docs; same shape for Claude Desktop's `claude_desktop_config.json`): `{"mcpServers":{"open-ontologies":{"type":"http","url":"http://127.0.0.1:27125/mcp","headers":{"Authorization":"Bearer <token>"}}}}`.
- All engine tools return a JSON **string** in `result.content[0].text` of a `tools/call` response. Tool names verified: `onto_validate`, `onto_load`, `onto_query`, `onto_reason`, `onto_reason_incremental`, `onto_classify_el`, `onto_shacl`, `onto_shacl_check`, `onto_diff`, `onto_lint`, `onto_apply`, `onto_save`, `onto_stats`, `onto_pack`, `onto_unpack`.
- Spec deviation (agreed rationale): typed links use the Dataview inline-field convention `property:: [[Target]]` (community standard, machine-parseable) rather than the literal `[[property::Target]]` syntax, which is not a valid Obsidian link.
- No em dashes in any user-facing copy (README, settings text, notices).
- Every commit in the plugin repo ends with the standard `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>` trailer.

## File Structure (plugin repo)

```
obsidian-open-ontologies/
├── manifest.json            # store manifest
├── versions.json            # plugin-version → minAppVersion map
├── package.json / tsconfig.json / esbuild.config.mjs / vitest.config.ts
├── styles.css
├── src/
│   ├── constants.ts         # ENGINE_VERSION, repo, view types
│   ├── main.ts              # plugin entry: lifecycle, commands, view registration
│   ├── settings.ts          # settings interface + tab
│   ├── engine/
│   │   ├── assets.ts        # platform→asset name, SHASUMS parse, version compat (pure)
│   │   ├── download.ts      # fetch binary + sha256 verify + chmod
│   │   ├── client.ts        # EngineClient: MCP streamable-HTTP
│   │   └── manager.ts       # EngineManager: resolve/spawn/health/restart/stop
│   ├── mapper/
│   │   ├── iri.ts           # IRI minting + N-Triples literal escaping (pure)
│   │   ├── rules.ts         # MappingRules + YAML parse (pure)
│   │   └── mapper.ts        # NoteInput → N-Triples (pure)
│   ├── sparql.ts            # tolerant SPARQL-result parsing (pure)
│   ├── inferred.ts          # entailed-minus-asserted diff (pure)
│   ├── starter/             # bundled vault ontology + SHACL shapes
│   └── views/
│       ├── validation.ts    # validation results pane
│       ├── tree.ts          # ontology tree pane
│       └── console.ts       # SPARQL console pane
├── tests/                   # vitest; e2e.test.ts gated by OO_E2E=1
├── test-vault/              # manual-QA vault (notes + shapes.ttl + sample.ttl)
└── .github/workflows/       # ci.yml (build+test+e2e), release.yml (tag → assets)
```

---

### Task 1: Scaffold the plugin repo and build toolchain

**Files:**
- Create: `/Users/fabio/projects/obsidian-open-ontologies/` — `package.json`, `tsconfig.json`, `esbuild.config.mjs`, `vitest.config.ts`, `manifest.json`, `versions.json`, `styles.css`, `src/main.ts`, `src/constants.ts`, `.gitignore`, `tests/smoke.test.ts`

**Interfaces:**
- Produces: a building, testing repo; `src/constants.ts` exports `ENGINE_VERSION`, `ENGINE_REPO`, `VIEW_VALIDATION`, `VIEW_TREE`, `VIEW_CONSOLE` used by every later task.

- [ ] **Step 1: Create repo and package.json**

```bash
mkdir -p /Users/fabio/projects/obsidian-open-ontologies/{src/engine,src/mapper,src/views,tests,test-vault}
cd /Users/fabio/projects/obsidian-open-ontologies && git init -b main
```

`package.json`:

```json
{
  "name": "obsidian-open-ontologies",
  "version": "0.1.0",
  "description": "Full Open Ontologies engine in Obsidian",
  "main": "main.js",
  "scripts": {
    "build": "node esbuild.config.mjs production",
    "dev": "node esbuild.config.mjs",
    "test": "vitest run"
  },
  "devDependencies": {
    "@types/js-yaml": "^4.0.9",
    "@types/node": "^20.11.0",
    "builtin-modules": "^3.3.0",
    "esbuild": "^0.21.0",
    "obsidian": "^1.5.7",
    "typescript": "^5.4.0",
    "vitest": "^2.0.0"
  },
  "dependencies": {
    "js-yaml": "^4.1.0"
  }
}
```

`tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "lib": ["ES2022", "DOM"],
    "strict": true,
    "noEmit": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "types": ["node"]
  },
  "include": ["src/**/*.ts", "tests/**/*.ts"]
}
```

`esbuild.config.mjs`:

```js
import esbuild from "esbuild";
import builtins from "builtin-modules";

const prod = process.argv[2] === "production";
const ctx = await esbuild.context({
  entryPoints: ["src/main.ts"],
  bundle: true,
  external: ["obsidian", "electron", ...builtins, ...builtins.map((m) => `node:${m}`)],
  format: "cjs",
  target: "es2022",
  platform: "node",
  sourcemap: prod ? false : "inline",
  outfile: "main.js",
});
if (prod) { await ctx.rebuild(); process.exit(0); } else { await ctx.watch(); }
```

`vitest.config.ts`:

```ts
import { defineConfig } from "vitest/config";
export default defineConfig({
  test: { include: ["tests/**/*.test.ts"], environment: "node" },
  resolve: { alias: { obsidian: new URL("./tests/obsidian-stub.ts", import.meta.url).pathname } },
});
```

`tests/obsidian-stub.ts` (lets view modules import "obsidian" under vitest):

```ts
export class ItemView { containerEl: any = { children: [null, { empty() {}, createEl() { return { createEl() { return this; }, addEventListener() {}, empty() {} }; } }] }; constructor(public leaf: any) {} }
export class Plugin {}
export class PluginSettingTab { constructor(public app: any, public plugin: any) {} }
export class Setting { constructor(public el: any) {} setName() { return this; } setDesc() { return this; } addText() { return this; } addTextArea() { return this; } addButton() { return this; } }
export class Notice { constructor(public msg: string) {} }
export class TFile {}
export class FuzzySuggestModal { constructor(public app: any) {} }
export class WorkspaceLeaf {}
```

`manifest.json`:

```json
{
  "id": "open-ontologies",
  "name": "Open Ontologies",
  "version": "0.1.0",
  "minAppVersion": "1.5.0",
  "description": "Validate, reason over, SHACL-check and SPARQL-query ontology files and your vault as RDF, powered by the full Open Ontologies engine.",
  "author": "Fabio Rovai",
  "authorUrl": "https://github.com/fabio-rovai",
  "isDesktopOnly": true
}
```

`versions.json`:

```json
{ "0.1.0": "1.5.0" }
```

`.gitignore`:

```
node_modules/
main.js
*.map
```

`styles.css`:

```css
.oo-validation-item { padding: 4px 8px; border-left: 3px solid var(--text-muted); margin: 4px 0; cursor: pointer; }
.oo-sev-violation, .oo-sev-error { border-left-color: var(--text-error); }
.oo-sev-warning { border-left-color: var(--text-warning); }
.oo-results-table { width: 100%; border-collapse: collapse; }
.oo-results-table td, .oo-results-table th { border: 1px solid var(--background-modifier-border); padding: 2px 6px; font-size: var(--font-ui-small); }
.oo-console-input { width: 100%; min-height: 120px; font-family: var(--font-monospace); }
```

`src/constants.ts`:

```ts
export const ENGINE_VERSION = "1.1.1";
export const ENGINE_REPO = "fabio-rovai/open-ontologies";
export const VIEW_VALIDATION = "oo-validation";
export const VIEW_TREE = "oo-tree";
export const VIEW_CONSOLE = "oo-console";
export const ONTOLOGY_EXTENSIONS = ["ttl", "owl", "rdf", "jsonld"];
```

`src/main.ts` (minimal, grows in Task 9):

```ts
import { Plugin } from "obsidian";

export default class OpenOntologiesPlugin extends Plugin {
  async onload() {
    console.log("Open Ontologies plugin loaded");
  }
  async onunload() {}
}
```

`tests/smoke.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { ENGINE_VERSION } from "../src/constants";

describe("scaffold", () => {
  it("pins the engine version", () => {
    expect(ENGINE_VERSION).toMatch(/^\d+\.\d+\.\d+$/);
  });
});
```

- [ ] **Step 2: Install, build, test**

Run: `cd /Users/fabio/projects/obsidian-open-ontologies && npm install && npm run build && npm test`
Expected: `main.js` created; 1 test passes.

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "chore: scaffold Obsidian plugin (esbuild + vitest + manifest)"
```

---

### Task 2: IRI minting and N-Triples escaping (`src/mapper/iri.ts`)

**Files:**
- Create: `src/mapper/iri.ts`
- Test: `tests/iri.test.ts`

**Interfaces:**
- Produces: `mintNoteIri(vaultPath: string, base: string): string` (strips `.md`, percent-encodes each path segment); `mintTermIri(name: string, base: string): string` (for predicates/classes/tags; strips surrounding `[[ ]]` and `.md`); `escapeLiteral(s: string): string` (N-Triples string escaping: `\` `"` newline, tab, CR).

- [ ] **Step 1: Write the failing test**

```ts
import { describe, it, expect } from "vitest";
import { mintNoteIri, mintTermIri, escapeLiteral } from "../src/mapper/iri";

describe("iri", () => {
  it("mints note IRIs from vault paths", () => {
    expect(mintNoteIri("People/Ada Lovelace.md", "vault:")).toBe("vault:People/Ada%20Lovelace");
    expect(mintNoteIri("Simple.md", "vault:")).toBe("vault:Simple");
  });
  it("mints term IRIs, unwrapping wikilinks", () => {
    expect(mintTermIri("[[Person]]", "vault:")).toBe("vault:Person");
    expect(mintTermIri("knows", "vault:")).toBe("vault:knows");
    expect(mintTermIri("[[People/Ada Lovelace]]", "vault:")).toBe("vault:People/Ada%20Lovelace");
  });
  it("escapes N-Triples literals", () => {
    expect(escapeLiteral('say "hi"\nnow')).toBe('say \\"hi\\"\\nnow');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- tests/iri.test.ts`
Expected: FAIL, cannot resolve `../src/mapper/iri`.

- [ ] **Step 3: Implement**

```ts
function encodeSegments(path: string): string {
  return path.split("/").map((s) => encodeURIComponent(s)).join("/");
}

export function mintNoteIri(vaultPath: string, base: string): string {
  const stripped = vaultPath.replace(/\.md$/, "");
  return base + encodeSegments(stripped);
}

export function mintTermIri(name: string, base: string): string {
  let n = name.trim();
  const m = n.match(/^\[\[(.+?)(\|.*)?\]\]$/);
  if (m) n = m[1];
  n = n.replace(/\.md$/, "");
  return base + encodeSegments(n);
}

export function escapeLiteral(s: string): string {
  return s
    .replace(/\\/g, "\\\\")
    .replace(/"/g, '\\"')
    .replace(/\n/g, "\\n")
    .replace(/\r/g, "\\r")
    .replace(/\t/g, "\\t");
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- tests/iri.test.ts` — Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/mapper/iri.ts tests/iri.test.ts && git commit -m "feat(mapper): IRI minting and N-Triples escaping"
```

---

### Task 3: Mapping rules with YAML overrides (`src/mapper/rules.ts`)

**Files:**
- Create: `src/mapper/rules.ts`
- Test: `tests/rules.test.ts`

**Interfaces:**
- Produces:

```ts
export interface MappingRules {
  iriBase: string;              // "vault:"
  typeKey: string;              // "type"
  iriKey: string;               // "iri"
  defaultLinkPredicate: string; // "vault:linksTo"
  tagPredicate: string;         // "vault:hasTag"
  skipKeys: string[];           // ["aliases", "cssclasses", "tags"]
}
export const DEFAULT_RULES: MappingRules;
export function parseRules(yamlText: string): MappingRules; // merge partial YAML onto defaults; invalid YAML → defaults
```

- [ ] **Step 1: Write the failing test**

```ts
import { describe, it, expect } from "vitest";
import { parseRules, DEFAULT_RULES } from "../src/mapper/rules";

describe("rules", () => {
  it("returns defaults for empty input", () => {
    expect(parseRules("")).toEqual(DEFAULT_RULES);
  });
  it("merges partial overrides", () => {
    const r = parseRules("iriBase: 'https://kb.example.org/'\ntypeKey: is_a");
    expect(r.iriBase).toBe("https://kb.example.org/");
    expect(r.typeKey).toBe("is_a");
    expect(r.tagPredicate).toBe(DEFAULT_RULES.tagPredicate);
  });
  it("falls back to defaults on invalid YAML", () => {
    expect(parseRules(": : :")).toEqual(DEFAULT_RULES);
  });
});
```

- [ ] **Step 2: Run to verify it fails** — `npm test -- tests/rules.test.ts` → FAIL (module missing).

- [ ] **Step 3: Implement**

```ts
import { load } from "js-yaml";

export interface MappingRules {
  iriBase: string;
  typeKey: string;
  iriKey: string;
  defaultLinkPredicate: string;
  tagPredicate: string;
  skipKeys: string[];
}

export const DEFAULT_RULES: MappingRules = {
  iriBase: "vault:",
  typeKey: "type",
  iriKey: "iri",
  defaultLinkPredicate: "vault:linksTo",
  tagPredicate: "vault:hasTag",
  skipKeys: ["aliases", "cssclasses", "tags"],
};

export function parseRules(yamlText: string): MappingRules {
  if (!yamlText.trim()) return { ...DEFAULT_RULES };
  try {
    const parsed = load(yamlText);
    if (typeof parsed !== "object" || parsed === null) return { ...DEFAULT_RULES };
    return { ...DEFAULT_RULES, ...(parsed as Partial<MappingRules>) };
  } catch {
    return { ...DEFAULT_RULES };
  }
}
```

- [ ] **Step 4: Run to verify it passes** — `npm test -- tests/rules.test.ts` → PASS.

- [ ] **Step 5: Commit**

```bash
git add src/mapper/rules.ts tests/rules.test.ts && git commit -m "feat(mapper): mapping rules with YAML overrides"
```

---

### Task 4: Vault→RDF mapper (`src/mapper/mapper.ts`)

**Files:**
- Create: `src/mapper/mapper.ts`
- Test: `tests/mapper.test.ts`

**Interfaces:**
- Consumes: `mintNoteIri`, `mintTermIri`, `escapeLiteral` (Task 2); `MappingRules` (Task 3).
- Produces:

```ts
export interface NoteInput {
  path: string;                                    // vault-relative
  frontmatter: Record<string, unknown>;
  links: { target: string }[];                     // resolved vault paths of plain wikilinks
  inlineFields: { key: string; target: string }[]; // "prop:: [[Target]]" lines, target resolved
  tags: string[];                                  // no leading '#'
}
export function mapNote(note: NoteInput, rules: MappingRules): string[]; // N-Triples lines
export function mapVault(notes: NoteInput[], rules: MappingRules): string; // deduped, joined
export function extractInlineFields(body: string): { key: string; target: string }[];
```

Mapping semantics: subject = frontmatter `iri:` if present else `mintNoteIri(path)`. Always emit `rdfs:label` = basename without extension. `typeKey` → `rdf:type` with `mintTermIri(value)`. Other frontmatter: skipKeys skipped; wikilink string values → object property `mintTermIri(key)`; numbers → `xsd:integer`/`xsd:double`; booleans → `xsd:boolean`; `YYYY-MM-DD` strings → `xsd:date`; everything else → plain string literal. Arrays map element-wise. Inline fields → `<s> <base+key> <mint(target)>`. Plain links → `defaultLinkPredicate`. Tags → `<s> <tagPredicate> <base+tags/tag>` plus `<base+tags/tag> rdf:type skos:Concept`.

- [ ] **Step 1: Write the failing test**

```ts
import { describe, it, expect } from "vitest";
import { mapNote, mapVault, extractInlineFields, NoteInput } from "../src/mapper/mapper";
import { DEFAULT_RULES } from "../src/mapper/rules";

const RDF_TYPE = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";

const ada: NoteInput = {
  path: "People/Ada Lovelace.md",
  frontmatter: { type: "[[Person]]", born: 1815, active: true, updated: "2026-08-14", note: "pioneer" },
  links: [{ target: "Charles Babbage.md" }],
  inlineFields: [{ key: "collaboratedWith", target: "Charles Babbage.md" }],
  tags: ["mathematician"],
};

describe("mapper", () => {
  it("maps a note to N-Triples", () => {
    const t = mapNote(ada, DEFAULT_RULES);
    const s = "<vault:People/Ada%20Lovelace>";
    expect(t).toContain(`${s} ${RDF_TYPE} <vault:Person> .`);
    expect(t).toContain(`${s} <http://www.w3.org/2000/01/rdf-schema#label> "Ada Lovelace" .`);
    expect(t).toContain(`${s} <vault:born> "1815"^^<http://www.w3.org/2001/XMLSchema#integer> .`);
    expect(t).toContain(`${s} <vault:active> "true"^^<http://www.w3.org/2001/XMLSchema#boolean> .`);
    expect(t).toContain(`${s} <vault:updated> "2026-08-14"^^<http://www.w3.org/2001/XMLSchema#date> .`);
    expect(t).toContain(`${s} <vault:note> "pioneer" .`);
    expect(t).toContain(`${s} <vault:linksTo> <vault:Charles%20Babbage> .`);
    expect(t).toContain(`${s} <vault:collaboratedWith> <vault:Charles%20Babbage> .`);
    expect(t).toContain(`${s} <vault:hasTag> <vault:tags/mathematician> .`);
    expect(t).toContain(`<vault:tags/mathematician> ${RDF_TYPE} <http://www.w3.org/2004/02/skos/core#Concept> .`);
  });
  it("honours an explicit iri and dedupes across the vault", () => {
    const n: NoteInput = { path: "X.md", frontmatter: { iri: "https://ex.org/x" }, links: [], inlineFields: [], tags: ["t"] };
    const out = mapVault([n, n], DEFAULT_RULES);
    expect(out).toContain("<https://ex.org/x>");
    const lines = out.trim().split("\n");
    expect(new Set(lines).size).toBe(lines.length);
  });
  it("extracts Dataview-style inline fields", () => {
    const fields = extractInlineFields("intro\ncollaboratedWith:: [[Charles Babbage]]\nplain [[Link]] text\n");
    expect(fields).toEqual([{ key: "collaboratedWith", target: "Charles Babbage" }]);
  });
});
```

- [ ] **Step 2: Run to verify it fails** — `npm test -- tests/mapper.test.ts` → FAIL.

- [ ] **Step 3: Implement**

```ts
import { mintNoteIri, mintTermIri, escapeLiteral } from "./iri";
import { MappingRules } from "./rules";

export interface NoteInput {
  path: string;
  frontmatter: Record<string, unknown>;
  links: { target: string }[];
  inlineFields: { key: string; target: string }[];
  tags: string[];
}

const RDF_TYPE = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
const RDFS_LABEL = "<http://www.w3.org/2000/01/rdf-schema#label>";
const SKOS_CONCEPT = "<http://www.w3.org/2004/02/skos/core#Concept>";
const XSD = "http://www.w3.org/2001/XMLSchema#";

function iriRef(iri: string): string {
  return `<${iri}>`;
}

function isWikilink(v: string): boolean {
  return /^\[\[.+\]\]$/.test(v.trim());
}

function literalFor(v: unknown): string | null {
  if (typeof v === "number") {
    const t = Number.isInteger(v) ? "integer" : "double";
    return `"${v}"^^<${XSD}${t}>`;
  }
  if (typeof v === "boolean") return `"${v}"^^<${XSD}boolean>`;
  if (typeof v === "string") {
    if (/^\d{4}-\d{2}-\d{2}$/.test(v)) return `"${v}"^^<${XSD}date>`;
    return `"${escapeLiteral(v)}"`;
  }
  return null;
}

export function mapNote(note: NoteInput, rules: MappingRules): string[] {
  const out: string[] = [];
  const fmIri = note.frontmatter[rules.iriKey];
  const s = iriRef(typeof fmIri === "string" && fmIri ? fmIri : mintNoteIri(note.path, rules.iriBase));
  const basename = note.path.replace(/\.md$/, "").split("/").pop() ?? note.path;
  out.push(`${s} ${RDFS_LABEL} "${escapeLiteral(basename)}" .`);

  for (const [key, raw] of Object.entries(note.frontmatter)) {
    if (key === rules.iriKey || rules.skipKeys.includes(key)) continue;
    const values = Array.isArray(raw) ? raw : [raw];
    for (const v of values) {
      if (key === rules.typeKey && typeof v === "string") {
        out.push(`${s} ${RDF_TYPE} ${iriRef(mintTermIri(v, rules.iriBase))} .`);
      } else if (typeof v === "string" && isWikilink(v)) {
        out.push(`${s} ${iriRef(mintTermIri(key, rules.iriBase))} ${iriRef(mintTermIri(v, rules.iriBase))} .`);
      } else {
        const lit = literalFor(v);
        if (lit) out.push(`${s} ${iriRef(mintTermIri(key, rules.iriBase))} ${lit} .`);
      }
    }
  }
  for (const f of note.inlineFields) {
    out.push(`${s} ${iriRef(mintTermIri(f.key, rules.iriBase))} ${iriRef(mintTermIri(f.target, rules.iriBase))} .`);
  }
  for (const l of note.links) {
    out.push(`${s} ${iriRef(rules.defaultLinkPredicate)} ${iriRef(mintTermIri(l.target, rules.iriBase))} .`);
  }
  for (const tag of note.tags) {
    const tagIri = iriRef(mintTermIri(`tags/${tag}`, rules.iriBase));
    out.push(`${s} ${iriRef(rules.tagPredicate)} ${tagIri} .`);
    out.push(`${tagIri} ${RDF_TYPE} ${SKOS_CONCEPT} .`);
  }
  return out;
}

export function mapVault(notes: NoteInput[], rules: MappingRules): string {
  const all = new Set<string>();
  for (const n of notes) for (const t of mapNote(n, rules)) all.add(t);
  return [...all].join("\n") + "\n";
}

const INLINE_FIELD = /^([A-Za-z][\w-]*)::\s*\[\[([^\]|]+)(\|[^\]]*)?\]\]\s*$/;

export function extractInlineFields(body: string): { key: string; target: string }[] {
  const out: { key: string; target: string }[] = [];
  for (const line of body.split("\n")) {
    const m = line.match(INLINE_FIELD);
    if (m) out.push({ key: m[1], target: m[2].trim() });
  }
  return out;
}
```

- [ ] **Step 4: Run to verify it passes** — `npm test -- tests/mapper.test.ts` → PASS.

- [ ] **Step 5: Commit**

```bash
git add src/mapper/mapper.ts tests/mapper.test.ts && git commit -m "feat(mapper): vault to N-Triples compilation"
```

---

### Task 5: EngineClient — MCP streamable HTTP (`src/engine/client.ts`)

**Files:**
- Create: `src/engine/client.ts`
- Test: `tests/client.test.ts`

**Interfaces:**
- Produces:

```ts
export interface HealthInfo { status: string; version: string }
export class EngineClient {
  constructor(baseUrl: string);              // "http://127.0.0.1:PORT"
  health(): Promise<HealthInfo>;             // GET /health
  initialize(): Promise<void>;               // MCP initialize + notifications/initialized; captures mcp-session-id
  call(tool: string, args?: Record<string, unknown>): Promise<string>;   // tools/call → content[0].text; throws on isError
  callJson<T = unknown>(tool: string, args?: Record<string, unknown>): Promise<T>; // JSON.parse of call()
}
```

Protocol: JSON-RPC 2.0 POSTs to `/mcp` with `Accept: application/json, text/event-stream`. Response body is either JSON or an SSE stream; for SSE, concatenate `data:` lines, parse each, return the message whose `id` matches. `initialize` params: `{ protocolVersion: "2025-03-26", capabilities: {}, clientInfo: { name: "obsidian-open-ontologies", version: "0.1.0" } }`. Echo the `mcp-session-id` response header on every subsequent request. After initialize, POST `notifications/initialized` (no id).

- [ ] **Step 1: Write the failing test** (spin a real `node:http` server as the mock engine)

```ts
import { describe, it, expect, beforeAll, afterAll } from "vitest";
import http from "node:http";
import { EngineClient } from "../src/engine/client";

let server: http.Server;
let base: string;
const seen: any[] = [];

beforeAll(async () => {
  server = http.createServer((req, res) => {
    if (req.method === "GET" && req.url === "/health") {
      res.setHeader("content-type", "application/json");
      res.end(JSON.stringify({ status: "ok", version: "1.1.1" }));
      return;
    }
    let body = "";
    req.on("data", (c) => (body += c));
    req.on("end", () => {
      const msg = JSON.parse(body);
      seen.push({ msg, session: req.headers["mcp-session-id"] });
      if (msg.method === "initialize") {
        res.setHeader("mcp-session-id", "sess-1");
        res.setHeader("content-type", "application/json");
        res.end(JSON.stringify({ jsonrpc: "2.0", id: msg.id, result: { protocolVersion: "2025-03-26", capabilities: {}, serverInfo: { name: "oo", version: "1.1.1" } } }));
      } else if (msg.method === "notifications/initialized") {
        res.statusCode = 202; res.end();
      } else if (msg.method === "tools/call") {
        // answer as SSE to exercise the stream path
        res.setHeader("content-type", "text/event-stream");
        res.end(`event: message\ndata: ${JSON.stringify({ jsonrpc: "2.0", id: msg.id, result: { content: [{ type: "text", text: '{"ok":true}' }] } })}\n\n`);
      }
    });
  });
  await new Promise<void>((r) => server.listen(0, "127.0.0.1", r));
  const addr = server.address() as any;
  base = `http://127.0.0.1:${addr.port}`;
});
afterAll(() => server.close());

describe("EngineClient", () => {
  it("initializes, keeps the session id, and calls tools over SSE", async () => {
    const c = new EngineClient(base);
    expect((await c.health()).version).toBe("1.1.1");
    await c.initialize();
    const out = await c.callJson<{ ok: boolean }>("onto_stats");
    expect(out.ok).toBe(true);
    const toolCall = seen.find((s) => s.msg.method === "tools/call");
    expect(toolCall.session).toBe("sess-1");
  });
});
```

- [ ] **Step 2: Run to verify it fails** — `npm test -- tests/client.test.ts` → FAIL.

- [ ] **Step 3: Implement**

```ts
export interface HealthInfo { status: string; version: string }

interface JsonRpcResponse { jsonrpc: "2.0"; id?: number; result?: any; error?: { code: number; message: string } }

export class EngineClient {
  private nextId = 1;
  private sessionId?: string;

  constructor(
    public baseUrl: string,
    private token?: string,
  ) {}

  async health(): Promise<HealthInfo> {
    const res = await fetch(`${this.baseUrl}/health`);
    if (!res.ok) throw new Error(`health check failed: HTTP ${res.status}`);
    return (await res.json()) as HealthInfo;
  }

  private async post(payload: object): Promise<JsonRpcResponse | null> {
    const headers: Record<string, string> = {
      "content-type": "application/json",
      accept: "application/json, text/event-stream",
    };
    if (this.sessionId) headers["mcp-session-id"] = this.sessionId;
    if (this.token) headers["authorization"] = `Bearer ${this.token}`;
    const res = await fetch(`${this.baseUrl}/mcp`, { method: "POST", headers, body: JSON.stringify(payload) });
    const sid = res.headers.get("mcp-session-id");
    if (sid) this.sessionId = sid;
    if (res.status === 202) return null;
    if (!res.ok) throw new Error(`engine HTTP ${res.status}: ${await res.text()}`);
    const ct = res.headers.get("content-type") ?? "";
    const body = await res.text();
    const id = (payload as any).id;
    if (ct.includes("text/event-stream")) {
      for (const line of body.split("\n")) {
        if (!line.startsWith("data:")) continue;
        const msg = JSON.parse(line.slice(5).trim()) as JsonRpcResponse;
        if (msg.id === id) return msg;
      }
      throw new Error("no matching response in SSE stream");
    }
    return JSON.parse(body) as JsonRpcResponse;
  }

  async initialize(): Promise<void> {
    const resp = await this.post({
      jsonrpc: "2.0",
      id: this.nextId++,
      method: "initialize",
      params: {
        protocolVersion: "2025-03-26",
        capabilities: {},
        clientInfo: { name: "obsidian-open-ontologies", version: "0.1.0" },
      },
    });
    if (!resp || resp.error) throw new Error(`initialize failed: ${resp?.error?.message ?? "no response"}`);
    await this.post({ jsonrpc: "2.0", method: "notifications/initialized" });
  }

  async call(tool: string, args: Record<string, unknown> = {}): Promise<string> {
    const resp = await this.post({
      jsonrpc: "2.0",
      id: this.nextId++,
      method: "tools/call",
      params: { name: tool, arguments: args },
    });
    if (!resp) throw new Error(`no response for ${tool}`);
    if (resp.error) throw new Error(`${tool}: ${resp.error.message}`);
    const text = resp.result?.content?.[0]?.text ?? "";
    if (resp.result?.isError) throw new Error(`${tool}: ${text}`);
    return text;
  }

  async callJson<T = unknown>(tool: string, args: Record<string, unknown> = {}): Promise<T> {
    return JSON.parse(await this.call(tool, args)) as T;
  }
}
```

- [ ] **Step 4: Run to verify it passes** — `npm test -- tests/client.test.ts` → PASS.

- [ ] **Step 5: Commit**

```bash
git add src/engine/client.ts tests/client.test.ts && git commit -m "feat(engine): MCP streamable-HTTP client"
```

---

### Task 6: Release asset resolution and checksum parsing (`src/engine/assets.ts`)

**Files:**
- Create: `src/engine/assets.ts`
- Test: `tests/assets.test.ts`

**Interfaces:**
- Produces: `assetName(platform: string, arch: string): string` (throws on unsupported combos); `parseShasums(text: string): Map<string, string>` (asset name → hex digest); `releaseUrl(version: string, file: string): string`; `isCompatible(version: string): boolean` (`major === 1 && minor >= 1`).

- [ ] **Step 1: Write the failing test**

```ts
import { describe, it, expect } from "vitest";
import { assetName, parseShasums, releaseUrl, isCompatible } from "../src/engine/assets";

describe("assets", () => {
  it("maps platform/arch to release asset names", () => {
    expect(assetName("darwin", "arm64")).toBe("open-ontologies-aarch64-apple-darwin");
    expect(assetName("darwin", "x64")).toBe("open-ontologies-x86_64-apple-darwin");
    expect(assetName("linux", "x64")).toBe("open-ontologies-x86_64-unknown-linux-gnu");
    expect(assetName("win32", "x64")).toBe("open-ontologies-x86_64-pc-windows-msvc.exe");
    expect(() => assetName("linux", "arm64")).toThrow(/unsupported/i);
  });
  it("parses SHASUMS.txt (sha256sum two-space format)", () => {
    const m = parseShasums("abc123  open-ontologies-aarch64-apple-darwin\ndef456  open-ontologies-x86_64-unknown-linux-gnu\n");
    expect(m.get("open-ontologies-aarch64-apple-darwin")).toBe("abc123");
    expect(m.size).toBe(2);
  });
  it("builds release URLs with the v-prefixed tag", () => {
    expect(releaseUrl("1.1.1", "SHASUMS.txt")).toBe(
      "https://github.com/fabio-rovai/open-ontologies/releases/download/v1.1.1/SHASUMS.txt"
    );
  });
  it("checks version compatibility (^1.1)", () => {
    expect(isCompatible("1.1.1")).toBe(true);
    expect(isCompatible("1.4.0")).toBe(true);
    expect(isCompatible("1.0.9")).toBe(false);
    expect(isCompatible("2.0.0")).toBe(false);
  });
});
```

- [ ] **Step 2: Run to verify it fails** — `npm test -- tests/assets.test.ts` → FAIL.

- [ ] **Step 3: Implement**

```ts
import { ENGINE_REPO } from "../constants";

const MAP: Record<string, string> = {
  "darwin/arm64": "open-ontologies-aarch64-apple-darwin",
  "darwin/x64": "open-ontologies-x86_64-apple-darwin",
  "linux/x64": "open-ontologies-x86_64-unknown-linux-gnu",
  "win32/x64": "open-ontologies-x86_64-pc-windows-msvc.exe",
};

export function assetName(platform: string, arch: string): string {
  const name = MAP[`${platform}/${arch}`];
  if (!name) throw new Error(`Unsupported platform: ${platform}/${arch}. Set a manual engine binary path in settings.`);
  return name;
}

export function parseShasums(text: string): Map<string, string> {
  const out = new Map<string, string>();
  for (const line of text.split("\n")) {
    const m = line.trim().match(/^([0-9a-f]{6,64})\s+\*?(.+)$/);
    if (m) out.set(m[2].trim(), m[1]);
  }
  return out;
}

export function releaseUrl(version: string, file: string): string {
  return `https://github.com/${ENGINE_REPO}/releases/download/v${version}/${file}`;
}

export function isCompatible(version: string): boolean {
  const m = version.match(/^(\d+)\.(\d+)\./);
  if (!m) return false;
  return Number(m[1]) === 1 && Number(m[2]) >= 1;
}
```

- [ ] **Step 4: Run to verify it passes** — `npm test -- tests/assets.test.ts` → PASS.

- [ ] **Step 5: Commit**

```bash
git add src/engine/assets.ts tests/assets.test.ts && git commit -m "feat(engine): release asset resolution, checksums, version gate"
```

---

### Task 7: Binary download + EngineManager (`src/engine/download.ts`, `src/engine/manager.ts`)

**Files:**
- Create: `src/engine/download.ts`, `src/engine/manager.ts`
- Test: `tests/download.test.ts` (checksum verification against a local HTTP server; no real GitHub calls)

**Interfaces:**
- Consumes: `assetName`, `parseShasums`, `releaseUrl`, `isCompatible` (Task 6); `EngineClient` (Task 5).
- Produces:

```ts
// download.ts
export async function downloadEngine(destDir: string, log: (s: string) => void, urlBase?: string): Promise<string>;
// urlBase overrides the GitHub release URL prefix (tests point it at a local server); returns absolute binary path

// manager.ts
export interface ManagerOptions { binDir: string; explicitPath?: string; log: (line: string) => void; }
export class EngineManager {
  constructor(opts: ManagerOptions);
  client: EngineClient | null;
  start(): Promise<EngineClient>;   // resolve → spawn → health poll (30 × 500ms) → version gate → initialize
  stop(): Promise<void>;            // kill child, disable restarts
  restart(): Promise<EngineClient>;
  recentLog(): string[];            // last 500 stderr lines
}
```

- [ ] **Step 1: Write the failing download test**

```ts
import { describe, it, expect, beforeAll, afterAll } from "vitest";
import http from "node:http";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import crypto from "node:crypto";
import { downloadEngine } from "../src/engine/download";
import { assetName } from "../src/engine/assets";

const FAKE = Buffer.from("#!/bin/sh\necho fake-engine\n");
let server: http.Server;
let urlBase: string;

beforeAll(async () => {
  const asset = assetName(process.platform, process.arch);
  const digest = crypto.createHash("sha256").update(FAKE).digest("hex");
  server = http.createServer((req, res) => {
    if (req.url!.endsWith("/SHASUMS.txt")) res.end(`${digest}  ${asset}\n`);
    else if (req.url!.endsWith(`/${asset}`)) res.end(FAKE);
    else { res.statusCode = 404; res.end(); }
  });
  await new Promise<void>((r) => server.listen(0, "127.0.0.1", r));
  urlBase = `http://127.0.0.1:${(server.address() as any).port}`;
});
afterAll(() => server.close());

describe("downloadEngine", () => {
  it("downloads, verifies sha256, chmods, returns the path", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "oo-dl-"));
    const bin = await downloadEngine(dir, () => {}, urlBase);
    expect(fs.readFileSync(bin)).toEqual(FAKE);
    if (process.platform !== "win32") expect(fs.statSync(bin).mode & 0o111).toBeTruthy();
  });
  it("rejects a corrupted download", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "oo-dl-bad-"));
    const badServer = http.createServer((req, res) => {
      if (req.url!.endsWith("/SHASUMS.txt")) res.end(`${"0".repeat(64)}  ${assetName(process.platform, process.arch)}\n`);
      else res.end(FAKE);
    });
    await new Promise<void>((r) => badServer.listen(0, "127.0.0.1", r));
    const badBase = `http://127.0.0.1:${(badServer.address() as any).port}`;
    await expect(downloadEngine(dir, () => {}, badBase)).rejects.toThrow(/checksum/i);
    badServer.close();
  });
});
```

- [ ] **Step 2: Run to verify it fails** — `npm test -- tests/download.test.ts` → FAIL.

- [ ] **Step 3: Implement download.ts**

```ts
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { ENGINE_VERSION } from "../constants";
import { assetName, parseShasums, releaseUrl } from "./assets";

export async function downloadEngine(
  destDir: string,
  log: (s: string) => void,
  urlBase?: string
): Promise<string> {
  const asset = assetName(process.platform, process.arch);
  const url = (f: string) => (urlBase ? `${urlBase}/${f}` : releaseUrl(ENGINE_VERSION, f));

  log(`Downloading checksums for engine v${ENGINE_VERSION}...`);
  const shaRes = await fetch(url("SHASUMS.txt"));
  if (!shaRes.ok) throw new Error(`SHASUMS.txt download failed: HTTP ${shaRes.status}`);
  const expected = parseShasums(await shaRes.text()).get(asset);
  if (!expected) throw new Error(`No checksum published for ${asset}`);

  log(`Downloading ${asset}...`);
  const binRes = await fetch(url(asset));
  if (!binRes.ok) throw new Error(`Engine download failed: HTTP ${binRes.status}`);
  const buf = Buffer.from(await binRes.arrayBuffer());

  const actual = crypto.createHash("sha256").update(buf).digest("hex");
  if (actual !== expected) throw new Error(`Checksum mismatch for ${asset}: expected ${expected}, got ${actual}`);

  fs.mkdirSync(destDir, { recursive: true });
  const dest = path.join(destDir, `open-ontologies-${ENGINE_VERSION}${process.platform === "win32" ? ".exe" : ""}`);
  fs.writeFileSync(dest, buf);
  if (process.platform !== "win32") fs.chmodSync(dest, 0o755);
  log(`Engine installed at ${dest}`);
  return dest;
}
```

- [ ] **Step 4: Run to verify it passes** — `npm test -- tests/download.test.ts` → PASS.

- [ ] **Step 5: Implement manager.ts** (spawn/restart logic; exercised for real by the e2e test in Task 13 and manual QA — the pure pieces are already tested)

```ts
import { spawn, ChildProcess } from "node:child_process";
import net from "node:net";
import fs from "node:fs";
import path from "node:path";
import { ENGINE_VERSION } from "../constants";
import { EngineClient } from "./client";
import { isCompatible } from "./assets";
import { downloadEngine } from "./download";

export interface ManagerOptions {
  binDir: string;
  explicitPath?: string;
  preferredPort: number;
  token: string;
  log: (line: string) => void;
}

const BACKOFF_MS = [1000, 5000, 15000];

export class EngineManager {
  client: EngineClient | null = null;
  port: number | null = null;
  private child: ChildProcess | null = null;
  private stopping = false;
  private restarts = 0;
  private logLines: string[] = [];

  constructor(private opts: ManagerOptions) {}

  recentLog(): string[] {
    return [...this.logLines];
  }

  private log(line: string) {
    this.logLines.push(line);
    if (this.logLines.length > 500) this.logLines.shift();
    this.opts.log(line);
  }

  private async resolveBinary(): Promise<string> {
    if (this.opts.explicitPath) {
      if (!fs.existsSync(this.opts.explicitPath)) throw new Error(`Configured engine path not found: ${this.opts.explicitPath}`);
      return this.opts.explicitPath;
    }
    const cached = path.join(this.opts.binDir, `open-ontologies-${ENGINE_VERSION}${process.platform === "win32" ? ".exe" : ""}`);
    if (fs.existsSync(cached)) return cached;
    return downloadEngine(this.opts.binDir, (s) => this.log(s));
  }

  private probePort(candidate: number): Promise<number | null> {
    return new Promise((resolve) => {
      const srv = net.createServer();
      srv.once("error", () => resolve(null));
      srv.listen(candidate, "127.0.0.1", () => {
        const port = (srv.address() as net.AddressInfo).port;
        srv.close(() => resolve(port));
      });
    });
  }

  private async resolvePort(): Promise<number> {
    const preferred = await this.probePort(this.opts.preferredPort);
    if (preferred !== null) return preferred;
    this.log(
      `Port ${this.opts.preferredPort} is in use; falling back to a random port. External MCP clients configured against ${this.opts.preferredPort} will not reach this engine until the port is free.`,
    );
    const ephemeral = await this.probePort(0);
    if (ephemeral === null) throw new Error("Could not bind any loopback port");
    return ephemeral;
  }

  async start(): Promise<EngineClient> {
    this.stopping = false;
    const binary = await this.resolveBinary();
    const port = await this.resolvePort();
    this.port = port;
    this.log(`Starting engine: ${binary} serve-http --host 127.0.0.1 --port ${port} --token <redacted>`);
    this.child = spawn(
      binary,
      ["serve-http", "--host", "127.0.0.1", "--port", String(port), "--token", this.opts.token],
      { stdio: ["ignore", "ignore", "pipe"] },
    );
    this.child.stderr!.on("data", (c: Buffer) => c.toString().split("\n").filter(Boolean).forEach((l) => this.log(l)));
    this.child.on("exit", (code) => {
      this.client = null;
      if (this.stopping) return;
      this.log(`Engine exited with code ${code}`);
      if (this.restarts < BACKOFF_MS.length) {
        const delay = BACKOFF_MS[this.restarts++];
        this.log(`Restarting in ${delay / 1000}s (attempt ${this.restarts}/${BACKOFF_MS.length})`);
        setTimeout(() => { this.start().catch((e) => this.log(`Restart failed: ${e.message}`)); }, delay);
      } else {
        this.log("Engine crashed repeatedly. Use the restart command after checking the log.");
      }
    });

    const client = new EngineClient(`http://127.0.0.1:${port}`, this.opts.token);
    let health = null;
    for (let i = 0; i < 30; i++) {
      try { health = await client.health(); break; } catch { await new Promise((r) => setTimeout(r, 500)); }
    }
    if (!health) { await this.stop(); throw new Error("Engine did not become healthy within 15s. Check the engine log in settings."); }
    if (!isCompatible(health.version)) {
      await this.stop();
      throw new Error(`Engine v${health.version} is not compatible with this plugin (requires ^1.1). Update the engine or clear the manual path.`);
    }
    await client.initialize();
    this.restarts = 0;
    this.client = client;
    this.log(`Engine v${health.version} ready on port ${port}`);
    return client;
  }

  async stop(): Promise<void> {
    this.stopping = true;
    if (this.child && !this.child.killed) this.child.kill();
    this.child = null;
    this.client = null;
  }

  async restart(): Promise<EngineClient> {
    await this.stop();
    this.restarts = 0;
    return this.start();
  }
}
```

- [ ] **Step 6: Full test run and commit**

Run: `npm test` — Expected: all suites PASS.

```bash
git add src/engine/download.ts src/engine/manager.ts tests/download.test.ts
git commit -m "feat(engine): verified binary download and sidecar lifecycle manager"
```

---

### Task 8: SPARQL result parsing (`src/sparql.ts`)

**Files:**
- Create: `src/sparql.ts`
- Test: `tests/sparql.test.ts`

**Interfaces:**
- Produces: `parseBindings(text: string): Record<string, string>[] | null` — accepts W3C SPARQL-JSON (`{head, results:{bindings:[{v:{value}}]}}`), a bare array of flat objects, or returns `null` when the text is not parseable as either (caller shows raw text). Used by the tree view (Task 11) and console (Task 12).

- [ ] **Step 1: Write the failing test**

```ts
import { describe, it, expect } from "vitest";
import { parseBindings } from "../src/sparql";

describe("parseBindings", () => {
  it("parses W3C SPARQL-JSON", () => {
    const rows = parseBindings(JSON.stringify({ head: { vars: ["cls"] }, results: { bindings: [{ cls: { type: "uri", value: "vault:Person" } }] } }));
    expect(rows).toEqual([{ cls: "vault:Person" }]);
  });
  it("parses a bare array of flat objects", () => {
    expect(parseBindings('[{"cls":"vault:Person"}]')).toEqual([{ cls: "vault:Person" }]);
  });
  it("returns null for non-tabular text", () => {
    expect(parseBindings("not json")).toBeNull();
    expect(parseBindings('{"error":"boom"}')).toBeNull();
  });
});
```

- [ ] **Step 2: Run to verify it fails** — `npm test -- tests/sparql.test.ts` → FAIL.

- [ ] **Step 3: Implement**

```ts
export function parseBindings(text: string): Record<string, string>[] | null {
  let parsed: unknown;
  try { parsed = JSON.parse(text); } catch { return null; }
  if (Array.isArray(parsed)) {
    return parsed.map((row) => {
      const out: Record<string, string> = {};
      for (const [k, v] of Object.entries(row as Record<string, unknown>)) out[k] = typeof v === "object" && v !== null && "value" in (v as any) ? String((v as any).value) : String(v);
      return out;
    });
  }
  const bindings = (parsed as any)?.results?.bindings;
  if (Array.isArray(bindings)) {
    return bindings.map((b: Record<string, { value: string }>) => {
      const out: Record<string, string> = {};
      for (const [k, v] of Object.entries(b)) out[k] = v.value;
      return out;
    });
  }
  return null;
}
```

- [ ] **Step 4: Run to verify it passes** — `npm test -- tests/sparql.test.ts` → PASS.

- [ ] **Step 5: Commit**

```bash
git add src/sparql.ts tests/sparql.test.ts && git commit -m "feat: tolerant SPARQL result parsing"
```

---

### Task 9: Settings, plugin wiring, and commands (`src/settings.ts`, `src/main.ts`)

**Files:**
- Modify: `src/main.ts`
- Create: `src/settings.ts`

**Interfaces:**
- Consumes: `EngineManager` (Task 7), `parseRules` (Task 3), `mapVault`/`extractInlineFields` (Task 4), constants (Task 1).
- Produces: `OpenOntologiesSettings { enginePath: string; mappingYaml: string; sparqlHistory: string[] }`; `plugin.manager: EngineManager`; `plugin.rules(): MappingRules`; `plugin.syncVault(): Promise<string>` (returns engine response text) used by views; command ids `oo-sync-vault`, `oo-validate-file`, `oo-reason`, `oo-classify-el`, `oo-lint-file`, `oo-shacl-vault`, `oo-restart-engine`, `oo-open-console`, `oo-open-tree`, `oo-open-validation`.

- [ ] **Step 1: Implement settings.ts**

```ts
import { App, Notice, PluginSettingTab, Setting } from "obsidian";
import { randomBytes } from "node:crypto";
import type OpenOntologiesPlugin from "./main";

export interface OpenOntologiesSettings {
  enginePath: string;      // empty = auto-download
  mappingYaml: string;     // overrides for MappingRules
  sparqlHistory: string[];
  mcpPort: number;         // stable port for external MCP clients
  mcpToken: string;        // generated on first run; never empty at runtime
  autoSync: boolean;       // debounced vault -> graph re-sync on markdown change
}

export const DEFAULT_SETTINGS: OpenOntologiesSettings = {
  enginePath: "",
  mappingYaml: "",
  sparqlHistory: [],
  mcpPort: 27125,
  mcpToken: "",
  autoSync: true,
};

export function mcpClientConfig(port: number, token: string): string {
  return JSON.stringify(
    {
      mcpServers: {
        "open-ontologies": {
          type: "http",
          url: `http://127.0.0.1:${port}/mcp`,
          headers: { Authorization: `Bearer ${token}` },
        },
      },
    },
    null,
    2,
  );
}

export class OpenOntologiesSettingTab extends PluginSettingTab {
  constructor(app: App, private plugin: OpenOntologiesPlugin) { super(app, plugin); }

  display(): void {
    const { containerEl } = this;
    containerEl.empty();

    new Setting(containerEl)
      .setName("Engine binary path")
      .setDesc("Leave empty to auto-download the pinned engine release. Set a path to reuse an existing install.")
      .addText((t) => t.setValue(this.plugin.settings.enginePath).onChange(async (v) => {
        this.plugin.settings.enginePath = v.trim();
        await this.plugin.saveSettings();
      }));

    new Setting(containerEl)
      .setName("Vault mapping rules (YAML)")
      .setDesc("Overrides for iriBase, typeKey, iriKey, defaultLinkPredicate, tagPredicate, skipKeys.")
      .addTextArea((t) => t.setValue(this.plugin.settings.mappingYaml).onChange(async (v) => {
        this.plugin.settings.mappingYaml = v;
        await this.plugin.saveSettings();
      }));

    new Setting(containerEl)
      .setName("Auto-sync vault to graph")
      .setDesc("Re-compile the vault into the knowledge graph 10 seconds after the last note change, so an MCP client always queries current data.")
      .addToggle((t) => t.setValue(this.plugin.settings.autoSync).onChange(async (v) => {
        this.plugin.settings.autoSync = v;
        await this.plugin.saveSettings();
      }));

    containerEl.createEl("h3", { text: "Connect an AI agent" });
    containerEl.createEl("p", {
      text: "The engine is an MCP server. Point Claude Code or Claude Desktop at it and your agent can query, reason over and validate the vault graph. Access requires the token below, so keep it private.",
    });

    new Setting(containerEl)
      .setName("MCP port")
      .setDesc("Stable loopback port for external MCP clients. Changing it restarts the engine and invalidates any config you already copied.")
      .addText((t) => t.setValue(String(this.plugin.settings.mcpPort)).onChange(async (v) => {
        const n = Number(v);
        if (!Number.isInteger(n) || n < 1024 || n > 65535) return;
        this.plugin.settings.mcpPort = n;
        await this.plugin.saveSettings();
      }));

    new Setting(containerEl)
      .setName("Copy MCP client config")
      .setDesc("Copies a ready-to-paste JSON block containing the URL and access token.")
      .addButton((b) => b.setButtonText("Copy").onClick(async () => {
        await navigator.clipboard.writeText(
          mcpClientConfig(this.plugin.settings.mcpPort, this.plugin.settings.mcpToken),
        );
        new Notice("MCP client config copied. It contains your access token.");
      }));

    new Setting(containerEl)
      .setName("Regenerate access token")
      .setDesc("Issues a new token and restarts the engine. Any MCP client using the old token stops working until you copy the config again.")
      .addButton((b) => b.setButtonText("Regenerate").setWarning().onClick(async () => {
        this.plugin.settings.mcpToken = randomBytes(32).toString("hex");
        await this.plugin.saveSettings();
        await this.plugin.restartEngine();
        this.display();
      }));

    new Setting(containerEl)
      .setName("Restart engine")
      .addButton((b) => b.setButtonText("Restart").onClick(() => this.plugin.restartEngine()));

    containerEl.createEl("h3", { text: "Engine log" });
    const pre = containerEl.createEl("pre", { text: this.plugin.manager?.recentLog().slice(-100).join("\n") ?? "(engine not started)" });
    pre.style.maxHeight = "240px";
    pre.style.overflow = "auto";
  }
}
```

- [ ] **Step 2: Rewrite main.ts with engine lifecycle + commands**

```ts
import { Notice, Plugin, TFile, FuzzySuggestModal, FileSystemAdapter, normalizePath } from "obsidian";
import { randomBytes } from "node:crypto";
import { EngineManager } from "./engine/manager";
import { EngineClient } from "./engine/client";
import { DEFAULT_SETTINGS, OpenOntologiesSettings, OpenOntologiesSettingTab } from "./settings";
import { parseRules, MappingRules } from "./mapper/rules";
import { mapVault, extractInlineFields, NoteInput } from "./mapper/mapper";
import { VIEW_VALIDATION, VIEW_TREE, VIEW_CONSOLE, ONTOLOGY_EXTENSIONS } from "./constants";
import path from "node:path";

class TtlFileModal extends FuzzySuggestModal<TFile> {
  constructor(app: any, private files: TFile[], private onPick: (f: TFile) => void) { super(app); }
  getItems(): TFile[] { return this.files; }
  getItemText(f: TFile): string { return f.path; }
  onChooseItem(f: TFile): void { this.onPick(f); }
}

export default class OpenOntologiesPlugin extends Plugin {
  settings!: OpenOntologiesSettings;
  manager!: EngineManager;

  rules(): MappingRules { return parseRules(this.settings.mappingYaml); }

  async client(): Promise<EngineClient> {
    if (this.manager.client) return this.manager.client;
    return this.manager.start();
  }

  fullPath(vaultPath: string): string {
    const adapter = this.app.vault.adapter;
    if (adapter instanceof FileSystemAdapter) return adapter.getFullPath(normalizePath(vaultPath));
    throw new Error("Filesystem vault required");
  }

  ontologyFiles(): TFile[] {
    return this.app.vault.getFiles().filter((f) => ONTOLOGY_EXTENSIONS.includes(f.extension));
  }

  async buildNoteInputs(): Promise<NoteInput[]> {
    const notes: NoteInput[] = [];
    for (const f of this.app.vault.getMarkdownFiles()) {
      const cache = this.app.metadataCache.getFileCache(f);
      const body = await this.app.vault.cachedRead(f);
      const inlineFields = extractInlineFields(body).map((fld) => {
        const dest = this.app.metadataCache.getFirstLinkpathDest(fld.target, f.path);
        return { key: fld.key, target: dest ? dest.path : `${fld.target}.md` };
      });
      const inlineTargets = new Set(inlineFields.map((x) => x.target));
      const links = (cache?.links ?? [])
        .map((l) => this.app.metadataCache.getFirstLinkpathDest(l.link, f.path))
        .filter((d): d is TFile => !!d)
        .map((d) => ({ target: d.path }))
        .filter((l) => !inlineTargets.has(l.target));
      const tags = (cache?.tags ?? []).map((t) => t.tag.replace(/^#/, ""));
      const fmTags = cache?.frontmatter?.tags;
      if (Array.isArray(fmTags)) tags.push(...fmTags.map(String));
      notes.push({ path: f.path, frontmatter: { ...(cache?.frontmatter ?? {}) }, links, inlineFields, tags: [...new Set(tags)] });
    }
    return notes;
  }

  async syncVault(): Promise<string> {
    const c = await this.client();
    const nt = mapVault(await this.buildNoteInputs(), this.rules());
    const res = await c.call("onto_load", { input: nt, inline: true, name: "vault" });
    new Notice("Vault synced to knowledge graph");
    return res;
  }

  async restartEngine(): Promise<void> {
    try { await this.manager.restart(); new Notice("Engine restarted"); }
    catch (e: any) { new Notice(`Engine restart failed: ${e.message}`, 8000); }
  }

  async onload() {
    this.settings = Object.assign({}, DEFAULT_SETTINGS, await this.loadData());
    if (!this.settings.mcpToken) {
      this.settings.mcpToken = randomBytes(32).toString("hex");
      await this.saveSettings();
    }
    const binDir = path.join(this.fullPath(this.app.vault.configDir + "/plugins/open-ontologies"), "bin");
    this.manager = new EngineManager({
      binDir,
      explicitPath: this.settings.enginePath || undefined,
      preferredPort: this.settings.mcpPort,
      token: this.settings.mcpToken,
      log: (l) => console.log(`[open-ontologies] ${l}`),
    });

    // Auto-sync: keep the graph an MCP client queries current.
    let syncTimer: number | null = null;
    const scheduleSync = () => {
      if (!this.settings.autoSync) return;
      if (syncTimer) window.clearTimeout(syncTimer);
      syncTimer = window.setTimeout(() => {
        this.syncVault().catch((e) => console.error("[open-ontologies] auto-sync failed", e));
      }, 10000);
    };
    for (const ev of ["create", "modify", "delete", "rename"] as const) {
      this.registerEvent(
        (this.app.vault as any).on(ev, (f: any) => {
          if (f?.extension === "md") scheduleSync();
        }),
      );
    }
    this.addSettingTab(new OpenOntologiesSettingTab(this.app, this));

    // Views are registered in Tasks 10-12; commands that depend on them activate the leaves.
    this.addCommand({ id: "oo-sync-vault", name: "Sync vault to knowledge graph", callback: () => this.syncVault().catch((e) => new Notice(e.message, 8000)) });
    this.addCommand({
      id: "oo-validate-file", name: "Validate current ontology file",
      checkCallback: (checking) => {
        const f = this.app.workspace.getActiveFile();
        if (!f || !ONTOLOGY_EXTENSIONS.includes(f.extension)) return false;
        if (!checking) this.client().then((c) => c.call("onto_validate", { input: this.fullPath(f.path) })).then((r) => new Notice(r.slice(0, 300), 8000)).catch((e) => new Notice(e.message, 8000));
        return true;
      },
    });
    this.addCommand({ id: "oo-reason", name: "Reason over loaded graph (OWL-RL)", callback: () => this.client().then((c) => c.call("onto_reason", { profile: "owl-rl" })).then((r) => new Notice(r.slice(0, 300), 8000)).catch((e) => new Notice(e.message, 8000)) });
    this.addCommand({ id: "oo-classify-el", name: "Classify loaded graph (OWL-EL)", callback: () => this.client().then((c) => c.call("onto_classify_el", {})).then((r) => new Notice(r.slice(0, 300), 8000)).catch((e) => new Notice(e.message, 8000)) });
    this.addCommand({
      id: "oo-lint-file", name: "Lint current ontology file",
      checkCallback: (checking) => {
        const f = this.app.workspace.getActiveFile();
        if (!f || !ONTOLOGY_EXTENSIONS.includes(f.extension)) return false;
        if (!checking) this.client().then((c) => c.call("onto_lint", { input: this.fullPath(f.path) })).then((r) => new Notice(r.slice(0, 300), 8000)).catch((e) => new Notice(e.message, 8000));
        return true;
      },
    });
    this.addCommand({
      id: "oo-shacl-vault", name: "SHACL-validate vault graph against shapes file",
      callback: () => new TtlFileModal(this.app, this.ontologyFiles(), (f) =>
        this.client().then((c) => c.call("onto_shacl", { shapes: this.fullPath(f.path) })).then((r) => new Notice(r.slice(0, 300), 8000)).catch((e) => new Notice(e.message, 8000))
      ).open(),
    });
    this.addCommand({
      id: "oo-diff-file", name: "Diff current ontology file against another",
      checkCallback: (checking) => {
        const f = this.app.workspace.getActiveFile();
        if (!f || !ONTOLOGY_EXTENSIONS.includes(f.extension)) return false;
        if (!checking) new TtlFileModal(this.app, this.ontologyFiles().filter((o) => o.path !== f.path), (other) =>
          this.client().then((c) => c.call("onto_diff", { old_path: this.fullPath(other.path), new_path: this.fullPath(f.path) })).then((r) => new Notice(r.slice(0, 300), 8000)).catch((e) => new Notice(e.message, 8000))
        ).open();
        return true;
      },
    });
    this.addCommand({
      id: "oo-pack", name: "Pack loaded graph to verified artifact",
      callback: () => this.client().then((c) => c.call("onto_pack", { path: this.fullPath("graph.oopack") })).then((r) => new Notice(r.slice(0, 300), 8000)).catch((e) => new Notice(e.message, 8000)),
    });
    this.addCommand({
      id: "oo-unpack", name: "Unpack verified artifact into store",
      callback: () => this.client().then((c) => c.call("onto_unpack", { path: this.fullPath("graph.oopack") })).then((r) => new Notice(r.slice(0, 300), 8000)).catch((e) => new Notice(e.message, 8000)),
    });
    this.addCommand({ id: "oo-restart-engine", name: "Restart engine", callback: () => this.restartEngine() });

    // Start the engine in the background so first command is fast; failure surfaces as a notice.
    this.manager.start().catch((e) => new Notice(`Open Ontologies engine failed to start: ${e.message}`, 10000));
  }

  async onunload() {
    await this.manager.stop();
  }

  async saveSettings() {
    await this.saveData(this.settings);
  }
}
```

- [ ] **Step 3: Build and run unit tests**

Run: `npm run build && npm test` — Expected: build clean, all tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/main.ts src/settings.ts && git commit -m "feat: engine lifecycle, settings tab, core commands"
```

---

### Task 10: Validation results pane (`src/views/validation.ts`)

**Files:**
- Create: `src/views/validation.ts`
- Modify: `src/main.ts` (register view + route command results into it + validate-on-save)
- Test: `tests/validation.test.ts` (pure `toValidationItems`)

**Interfaces:**
- Produces: `interface ValidationItem { severity: string; message: string; file?: string }`; `toValidationItems(toolName: string, jsonText: string): ValidationItem[]` (tolerant: engine `{error}` → one error item; SHACL report arrays under `violations`/`results`/`issues` → items; unrecognised JSON → single info item with raw text); `ValidationView extends ItemView` with `setResults(items: ValidationItem[]): void`; clicking an item with `file` opens it via `openLinkText`. `main.ts` gains `showValidation(items)` and replaces the `Notice`-only handling in `oo-validate-file`, `oo-lint-file`, `oo-shacl-vault` with panel output; registers a debounced (1s) vault `modify` handler that validates changed ontology files into the panel.

- [ ] **Step 1: Write the failing test for `toValidationItems`**

```ts
import { describe, it, expect } from "vitest";
import { toValidationItems } from "../src/views/validation";

describe("toValidationItems", () => {
  it("maps engine errors", () => {
    const items = toValidationItems("onto_validate", '{"error":"parse failed at line 3"}');
    expect(items).toEqual([{ severity: "error", message: "parse failed at line 3" }]);
  });
  it("maps SHACL-style result arrays", () => {
    const items = toValidationItems("onto_shacl", JSON.stringify({ conforms: false, violations: [{ severity: "Violation", message: "missing name", focus_node: "vault:X" }] }));
    expect(items[0].severity.toLowerCase()).toContain("violation");
    expect(items[0].message).toContain("missing name");
  });
  it("wraps unknown JSON as info", () => {
    const items = toValidationItems("onto_lint", '{"ok":true,"triples":42}');
    expect(items).toHaveLength(1);
    expect(items[0].severity).toBe("info");
  });
});
```

- [ ] **Step 2: Run to verify it fails** — `npm test -- tests/validation.test.ts` → FAIL.

- [ ] **Step 3: Implement**

```ts
import { ItemView, WorkspaceLeaf } from "obsidian";
import { VIEW_VALIDATION } from "../constants";

export interface ValidationItem { severity: string; message: string; file?: string }

export function toValidationItems(toolName: string, jsonText: string): ValidationItem[] {
  let parsed: any;
  try { parsed = JSON.parse(jsonText); } catch { return [{ severity: "info", message: jsonText.slice(0, 2000) }]; }
  if (parsed && typeof parsed === "object" && typeof parsed.error === "string") {
    return [{ severity: "error", message: parsed.error }];
  }
  const arr = parsed?.violations ?? parsed?.results ?? parsed?.issues;
  if (Array.isArray(arr) && arr.length) {
    return arr.map((r: any) => ({
      severity: String(r.severity ?? r.level ?? "warning"),
      message: [r.message ?? r.text ?? JSON.stringify(r), r.focus_node ?? r.focusNode ?? ""].filter(Boolean).join(" @ "),
      file: typeof r.file === "string" ? r.file : undefined,
    }));
  }
  return [{ severity: "info", message: `${toolName}: ${jsonText.slice(0, 2000)}` }];
}

export class ValidationView extends ItemView {
  private items: ValidationItem[] = [];
  constructor(leaf: WorkspaceLeaf, private openFile: (path: string) => void) { super(leaf); }
  getViewType() { return VIEW_VALIDATION; }
  getDisplayText() { return "Ontology validation"; }
  getIcon() { return "shield-check"; }

  setResults(items: ValidationItem[]) {
    this.items = items;
    this.render();
  }

  private render() {
    const el = this.containerEl.children[1] as HTMLElement;
    el.empty();
    el.createEl("h4", { text: `Validation results (${this.items.length})` });
    for (const item of this.items) {
      const row = el.createEl("div", { cls: `oo-validation-item oo-sev-${item.severity.toLowerCase()}` });
      row.createEl("strong", { text: item.severity });
      row.createEl("div", { text: item.message });
      if (item.file) row.addEventListener("click", () => this.openFile(item.file!));
    }
  }

  async onOpen() { this.render(); }
}
```

- [ ] **Step 4: Wire into main.ts**

In `onload()` add, before the commands:

```ts
this.registerView(VIEW_VALIDATION, (leaf) => new ValidationView(leaf, (p) => this.app.workspace.openLinkText(p, "", false)));
this.addCommand({ id: "oo-open-validation", name: "Open validation panel", callback: () => this.activateView(VIEW_VALIDATION) });

let debounce: number | null = null;
this.registerEvent(this.app.vault.on("modify", (f) => {
  if (!(f instanceof TFile) || !ONTOLOGY_EXTENSIONS.includes(f.extension)) return;
  if (debounce) window.clearTimeout(debounce);
  debounce = window.setTimeout(() => {
    this.client()
      .then((c) => c.call("onto_validate", { input: this.fullPath(f.path) }))
      .then((r) => this.showValidation(toValidationItems("onto_validate", r).map((i) => ({ ...i, file: i.file ?? f.path }))))
      .catch((e) => new Notice(e.message, 6000));
  }, 1000);
}));
```

And add the helper methods on the plugin class:

```ts
async activateView(type: string): Promise<void> {
  const existing = this.app.workspace.getLeavesOfType(type);
  const leaf = existing[0] ?? this.app.workspace.getRightLeaf(false)!;
  await leaf.setViewState({ type, active: true });
  this.app.workspace.revealLeaf(leaf);
}

async showValidation(items: ValidationItem[]): Promise<void> {
  await this.activateView(VIEW_VALIDATION);
  const view = this.app.workspace.getLeavesOfType(VIEW_VALIDATION)[0]?.view;
  if (view instanceof ValidationView) view.setResults(items);
}
```

Then change the `oo-validate-file`, `oo-lint-file`, and `oo-shacl-vault` command bodies from `new Notice(r.slice(0, 300), 8000)` to `this.showValidation(toValidationItems("<tool>", r))` (keeping the catch clauses), and add the imports `import { ValidationView, toValidationItems, ValidationItem } from "./views/validation";`.

- [ ] **Step 5: Run tests and build** — `npm test && npm run build` → PASS, clean build.

- [ ] **Step 6: Commit**

```bash
git add src/views/validation.ts src/main.ts tests/validation.test.ts
git commit -m "feat(views): validation panel with deep links and validate-on-save"
```

---

### Task 11: Ontology tree pane (`src/views/tree.ts`)

**Files:**
- Create: `src/views/tree.ts`
- Modify: `src/main.ts` (register view + `oo-open-tree` command)
- Test: `tests/tree.test.ts` (pure `buildForest`)

**Interfaces:**
- Consumes: `parseBindings` (Task 8), `EngineClient.callJson`/`call` (Task 5).
- Produces: `interface TreeNode { iri: string; label: string; children: TreeNode[] }`; `buildForest(rows: { cls: string; parent?: string }[]): TreeNode[]` (roots = classes with no parent in the set; cycles broken by first-seen wins); `TreeView extends ItemView` with a refresh button that runs the class query and renders nested `<details>` elements; clicking a node whose IRI starts with the rules' `iriBase` decodes it to a vault path and opens the note.

Class query (run via `onto_query`):

```sparql
PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT ?cls ?parent WHERE {
  ?cls a owl:Class .
  OPTIONAL { ?cls rdfs:subClassOf ?parent . FILTER(isIRI(?parent)) }
}
```

- [ ] **Step 1: Write the failing test for `buildForest`**

```ts
import { describe, it, expect } from "vitest";
import { buildForest } from "../src/views/tree";

describe("buildForest", () => {
  it("nests children under parents and keeps orphans as roots", () => {
    const forest = buildForest([
      { cls: "v:Agent" },
      { cls: "v:Person", parent: "v:Agent" },
      { cls: "v:Org", parent: "v:Agent" },
      { cls: "v:Loose" },
    ]);
    expect(forest.map((n) => n.iri).sort()).toEqual(["v:Agent", "v:Loose"]);
    const agent = forest.find((n) => n.iri === "v:Agent")!;
    expect(agent.children.map((c) => c.iri).sort()).toEqual(["v:Org", "v:Person"]);
  });
  it("does not loop on cycles", () => {
    const forest = buildForest([{ cls: "v:A", parent: "v:B" }, { cls: "v:B", parent: "v:A" }]);
    expect(forest.length).toBeGreaterThan(0);
  });
});
```

- [ ] **Step 2: Run to verify it fails** — `npm test -- tests/tree.test.ts` → FAIL.

- [ ] **Step 3: Implement**

```ts
import { ItemView, WorkspaceLeaf } from "obsidian";
import { VIEW_TREE } from "../constants";
import { EngineClient } from "../engine/client";
import { parseBindings } from "../sparql";

export interface TreeNode { iri: string; label: string; children: TreeNode[] }

function labelOf(iri: string): string {
  const tail = iri.split(/[/#:]/).pop() ?? iri;
  try { return decodeURIComponent(tail); } catch { return tail; }
}

export function buildForest(rows: { cls: string; parent?: string }[]): TreeNode[] {
  const nodes = new Map<string, TreeNode>();
  const childOf = new Map<string, string>();
  for (const r of rows) {
    if (!nodes.has(r.cls)) nodes.set(r.cls, { iri: r.cls, label: labelOf(r.cls), children: [] });
    if (r.parent && !childOf.has(r.cls)) childOf.set(r.cls, r.parent);
  }
  const roots: TreeNode[] = [];
  for (const [iri, node] of nodes) {
    const parent = childOf.get(iri);
    if (parent && nodes.has(parent) && parent !== iri && !isAncestor(nodes, childOf, parent, iri)) {
      nodes.get(parent)!.children.push(node);
    } else {
      roots.push(node);
    }
  }
  return roots;
}

function isAncestor(nodes: Map<string, TreeNode>, childOf: Map<string, string>, candidate: string, of: string): boolean {
  let cur: string | undefined = candidate;
  const seen = new Set<string>();
  while (cur && !seen.has(cur)) {
    if (cur === of) return true;
    seen.add(cur);
    cur = childOf.get(cur);
  }
  return false;
}

const CLASS_QUERY = `PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT ?cls ?parent WHERE { ?cls a owl:Class . OPTIONAL { ?cls rdfs:subClassOf ?parent . FILTER(isIRI(?parent)) } }`;

export class TreeView extends ItemView {
  constructor(
    leaf: WorkspaceLeaf,
    private getClient: () => Promise<EngineClient>,
    private iriBase: () => string,
    private openNote: (vaultPath: string) => void
  ) { super(leaf); }

  getViewType() { return VIEW_TREE; }
  getDisplayText() { return "Ontology tree"; }
  getIcon() { return "network"; }

  async refresh() {
    const el = this.containerEl.children[1] as HTMLElement;
    el.empty();
    const btn = el.createEl("button", { text: "Refresh" });
    btn.addEventListener("click", () => this.refresh());
    try {
      const c = await this.getClient();
      const text = await c.call("onto_query", { query: CLASS_QUERY });
      const rows = parseBindings(text);
      if (!rows) { el.createEl("pre", { text }); return; }
      const forest = buildForest(rows as { cls: string; parent?: string }[]);
      const root = el.createEl("div");
      const renderNode = (node: TreeNode, parent: HTMLElement) => {
        const d = parent.createEl("details", { attr: { open: "" } });
        const s = d.createEl("summary", { text: node.label });
        const base = this.iriBase();
        if (node.iri.startsWith(base)) {
          s.addEventListener("click", (ev) => { ev.preventDefault(); this.openNote(decodeURIComponent(node.iri.slice(base.length)) + ".md"); });
        }
        for (const ch of node.children) renderNode(ch, d);
      };
      for (const n of forest) renderNode(n, root);
      if (!forest.length) el.createEl("p", { text: "No owl:Class found. Load an ontology or sync the vault first." });
    } catch (e: any) {
      el.createEl("p", { text: `Tree failed: ${e.message}` });
    }
  }

  async onOpen() { await this.refresh(); }
}
```

- [ ] **Step 4: Wire into main.ts** — in `onload()`:

```ts
this.registerView(VIEW_TREE, (leaf) => new TreeView(
  leaf,
  () => this.client(),
  () => this.rules().iriBase,
  (p) => this.app.workspace.openLinkText(p, "", false)
));
this.addCommand({ id: "oo-open-tree", name: "Open ontology tree", callback: () => this.activateView(VIEW_TREE) });
```

with import `import { TreeView } from "./views/tree";`.

- [ ] **Step 5: Run tests and build** — `npm test && npm run build` → PASS.

- [ ] **Step 6: Commit**

```bash
git add src/views/tree.ts src/main.ts tests/tree.test.ts && git commit -m "feat(views): ontology tree pane with note deep links"
```

---

### Task 12: SPARQL console pane (`src/views/console.ts`)

**Files:**
- Create: `src/views/console.ts`
- Modify: `src/main.ts` (register view + `oo-open-console` command; history persisted in settings)

**Interfaces:**
- Consumes: `parseBindings` (Task 8), `EngineClient` (Task 5), `sparqlHistory` from settings (Task 9).
- Produces: `ConsoleView extends ItemView` — textarea (class `oo-console-input`), Run button + Ctrl/Cmd-Enter, results rendered by exported pure helper `renderRows(el: HTMLElement, rows: Record<string, string>[]): void` (table with class `oo-results-table`); non-tabular results shown in a `<pre>`. History: last 50 queries, newest first, deduped, shown in a `<select>` that fills the textarea.

- [ ] **Step 1: Implement** (render helper is trivially DOM-bound; covered by e2e + manual QA, no unit test)

```ts
import { ItemView, WorkspaceLeaf } from "obsidian";
import { VIEW_CONSOLE } from "../constants";
import { EngineClient } from "../engine/client";
import { parseBindings } from "../sparql";

export function renderRows(el: HTMLElement, rows: Record<string, string>[]): void {
  if (!rows.length) { el.createEl("p", { text: "No results." }); return; }
  const cols = Object.keys(rows[0]);
  const table = el.createEl("table", { cls: "oo-results-table" });
  const head = table.createEl("tr");
  for (const c of cols) head.createEl("th", { text: c });
  for (const row of rows) {
    const tr = table.createEl("tr");
    for (const c of cols) tr.createEl("td", { text: row[c] ?? "" });
  }
}

export class ConsoleView extends ItemView {
  constructor(
    leaf: WorkspaceLeaf,
    private getClient: () => Promise<EngineClient>,
    private history: () => string[],
    private pushHistory: (q: string) => Promise<void>
  ) { super(leaf); }

  getViewType() { return VIEW_CONSOLE; }
  getDisplayText() { return "SPARQL console"; }
  getIcon() { return "terminal"; }

  async onOpen() {
    const el = this.containerEl.children[1] as HTMLElement;
    el.empty();
    const select = el.createEl("select");
    select.createEl("option", { text: "History...", value: "" });
    for (const q of this.history()) select.createEl("option", { text: q.slice(0, 80), value: q });
    const input = el.createEl("textarea", { cls: "oo-console-input" });
    input.value = "SELECT ?s ?p ?o WHERE { ?s ?p ?o } LIMIT 25";
    select.addEventListener("change", () => { if (select.value) input.value = select.value; });
    const run = el.createEl("button", { text: "Run" });
    const out = el.createEl("div");
    const exec = async () => {
      out.empty();
      try {
        const c = await this.getClient();
        const text = await c.call("onto_query", { query: input.value });
        await this.pushHistory(input.value);
        const rows = parseBindings(text);
        if (rows) renderRows(out, rows);
        else out.createEl("pre", { text });
      } catch (e: any) {
        out.createEl("pre", { text: `Query failed: ${e.message}` });
      }
    };
    run.addEventListener("click", exec);
    input.addEventListener("keydown", (ev) => { if ((ev.ctrlKey || ev.metaKey) && ev.key === "Enter") exec(); });
  }
}
```

- [ ] **Step 2: Wire into main.ts** — in `onload()`:

```ts
this.registerView(VIEW_CONSOLE, (leaf) => new ConsoleView(
  leaf,
  () => this.client(),
  () => this.settings.sparqlHistory,
  async (q) => {
    this.settings.sparqlHistory = [q, ...this.settings.sparqlHistory.filter((h) => h !== q)].slice(0, 50);
    await this.saveSettings();
  }
));
this.addCommand({ id: "oo-open-console", name: "Open SPARQL console", callback: () => this.activateView(VIEW_CONSOLE) });
```

with import `import { ConsoleView } from "./views/console";`.

- [ ] **Step 3: Build and test** — `npm run build && npm test` → PASS.

- [ ] **Step 4: Commit**

```bash
git add src/views/console.ts src/main.ts && git commit -m "feat(views): SPARQL console with history"
```

---

### Task 12b: Starter vault ontology and shapes

**Files:**
- Create: `src/starter/vault-ontology.ttl.ts` (Turtle embedded as a TS string constant so esbuild bundles it), `src/starter/vault-shapes.ttl.ts`
- Modify: `src/main.ts` (command `oo-install-starter-ontology`)
- Test: `tests/starter.test.ts`

**Why this task exists:** OWL-RL over an untyped vault entails nothing. `vault:linksTo` triples alone support no interesting inference, so without a vocabulary the whole agent-facing story is vacuous. Transitive `partOf` and symmetric `relatesTo` are what give the reasoner derivations to make.

**Interfaces:**
- Produces: `VAULT_ONTOLOGY_TTL: string`, `VAULT_SHAPES_TTL: string`; command writes them to `<vault>/ontology/vault-ontology.ttl` and `<vault>/ontology/vault-shapes.ttl`, refusing to overwrite existing files.

- [ ] **Step 1: Write the failing test**

```ts
import { describe, it, expect } from "vitest";
import { VAULT_ONTOLOGY_TTL, VAULT_SHAPES_TTL } from "../src/starter/vault-ontology.ttl";

describe("starter ontology", () => {
  it("declares the second-brain vocabulary", () => {
    for (const cls of ["Note", "Person", "Project", "Task", "Source", "Idea", "Topic"]) {
      expect(VAULT_ONTOLOGY_TTL).toContain(`vault:${cls}`);
    }
  });
  it("makes partOf transitive and relatesTo symmetric so reasoning has something to derive", () => {
    expect(VAULT_ONTOLOGY_TTL).toContain("owl:TransitiveProperty");
    expect(VAULT_ONTOLOGY_TTL).toContain("owl:SymmetricProperty");
  });
  it("ships shapes targeting the vocabulary", () => {
    expect(VAULT_SHAPES_TTL).toContain("sh:NodeShape");
    expect(VAULT_SHAPES_TTL).toContain("sh:targetClass");
  });
});
```

- [ ] **Step 2: Run to verify it fails** — `npm test -- tests/starter.test.ts` → FAIL (module missing).

- [ ] **Step 3: Implement `src/starter/vault-ontology.ttl.ts`**

```ts
export const VAULT_ONTOLOGY_TTL = `@prefix owl:   <http://www.w3.org/2002/07/owl#> .
@prefix rdfs:  <http://www.w3.org/2000/01/rdf-schema#> .
@prefix vault: <vault:> .

vault:Note    a owl:Class ; rdfs:label "Note" .
vault:Person  a owl:Class ; rdfs:subClassOf vault:Note ; rdfs:label "Person" .
vault:Project a owl:Class ; rdfs:subClassOf vault:Note ; rdfs:label "Project" .
vault:Task    a owl:Class ; rdfs:subClassOf vault:Note ; rdfs:label "Task" .
vault:Source  a owl:Class ; rdfs:subClassOf vault:Note ; rdfs:label "Source" .
vault:Idea    a owl:Class ; rdfs:subClassOf vault:Note ; rdfs:label "Idea" .
vault:Topic   a owl:Class ; rdfs:subClassOf vault:Note ; rdfs:label "Topic" .

# Transitive: a Task partOf a Project partOf a Programme is entailed to be partOf the Programme.
vault:partOf a owl:ObjectProperty, owl:TransitiveProperty ;
  rdfs:label "part of" ; rdfs:domain vault:Note ; rdfs:range vault:Note .

# Symmetric: relating A to B entails B relates to A, so one-directional notes still connect.
vault:relatesTo a owl:ObjectProperty, owl:SymmetricProperty ;
  rdfs:label "relates to" ; rdfs:domain vault:Note ; rdfs:range vault:Note .

vault:authoredBy a owl:ObjectProperty ;
  rdfs:label "authored by" ; rdfs:domain vault:Note ; rdfs:range vault:Person .

vault:references a owl:ObjectProperty ;
  rdfs:label "references" ; rdfs:domain vault:Note ; rdfs:range vault:Source .

vault:about a owl:ObjectProperty ;
  rdfs:label "about" ; rdfs:domain vault:Note ; rdfs:range vault:Topic .
`;

export const VAULT_SHAPES_TTL = `@prefix sh:    <http://www.w3.org/ns/shacl#> .
@prefix xsd:   <http://www.w3.org/2001/XMLSchema#> .
@prefix vault: <vault:> .

vault:TaskShape a sh:NodeShape ;
  sh:targetClass vault:Task ;
  sh:property [ sh:path vault:partOf ; sh:minCount 1 ;
                sh:message "A Task should record which Project it is part of." ] .

vault:SourceShape a sh:NodeShape ;
  sh:targetClass vault:Source ;
  sh:property [ sh:path vault:url ; sh:minCount 1 ; sh:datatype xsd:string ;
                sh:message "A Source should record where it came from." ] .
`;
```

- [ ] **Step 4: Run to verify it passes** — `npm test -- tests/starter.test.ts` → PASS.

- [ ] **Step 5: Add the install command to `main.ts` `onload()`**

```ts
this.addCommand({
  id: "oo-install-starter-ontology",
  name: "Install starter vault ontology",
  callback: async () => {
    const { VAULT_ONTOLOGY_TTL, VAULT_SHAPES_TTL } = await import("./starter/vault-ontology.ttl");
    const targets: [string, string][] = [
      ["ontology/vault-ontology.ttl", VAULT_ONTOLOGY_TTL],
      ["ontology/vault-shapes.ttl", VAULT_SHAPES_TTL],
    ];
    let written = 0;
    for (const [rel, body] of targets) {
      if (await this.app.vault.adapter.exists(rel)) continue;
      const dir = rel.split("/").slice(0, -1).join("/");
      if (dir && !(await this.app.vault.adapter.exists(dir))) await this.app.vault.adapter.mkdir(dir);
      await this.app.vault.adapter.write(rel, body);
      written++;
    }
    new Notice(
      written === 0
        ? "Starter ontology already present; nothing overwritten."
        : `Installed ${written} starter file(s) under ontology/. Add 'type: \"[[Project]]\"' style frontmatter to your notes, then sync.`,
      10000,
    );
  },
});
```

- [ ] **Step 6: Build, test, commit**

```bash
npm run build && npm test
git add src/starter src/main.ts tests/starter.test.ts
git commit -m "feat(starter): vault ontology and SHACL shapes so reasoning has something to entail"
```

---

### Task 12c: Inferred connections

**Files:**
- Create: `src/inferred.ts`
- Modify: `src/main.ts` (command `oo-inferred-connections`)
- Test: `tests/inferred.test.ts`

**Why this task exists:** this is the feature a file-access MCP server cannot offer, and the concrete meaning of "the brain gets smarter". Obsidian shows asserted backlinks; we show entailed ones.

**Interfaces:**
- Consumes: `EngineClient` (Task 5), `parseBindings` (Task 8), `toValidationItems`/`ValidationItem` (Task 10), `mintNoteIri` (Task 2).
- Produces: `diffTriples(closure: Triple[], asserted: Triple[]): Triple[]` where `interface Triple { s: string; p: string; o: string }` — set difference keyed on `s|p|o`; `inferredToItems(rows: Triple[]): ValidationItem[]`.

- [ ] **Step 1: Write the failing test**

```ts
import { describe, it, expect } from "vitest";
import { diffTriples } from "../src/inferred";

describe("diffTriples", () => {
  it("returns closure triples that were never asserted", () => {
    const asserted = [{ s: "vault:A", p: "vault:partOf", o: "vault:B" }];
    const closure = [
      { s: "vault:A", p: "vault:partOf", o: "vault:B" },
      { s: "vault:A", p: "vault:partOf", o: "vault:C" },
    ];
    expect(diffTriples(closure, asserted)).toEqual([{ s: "vault:A", p: "vault:partOf", o: "vault:C" }]);
  });
  it("returns nothing when the closure adds nothing", () => {
    const t = [{ s: "vault:A", p: "vault:relatesTo", o: "vault:B" }];
    expect(diffTriples(t, t)).toEqual([]);
  });
});
```

- [ ] **Step 2: Run to verify it fails** — `npm test -- tests/inferred.test.ts` → FAIL.

- [ ] **Step 3: Implement `src/inferred.ts`**

```ts
import { ValidationItem } from "./views/validation";

export interface Triple { s: string; p: string; o: string }

const key = (t: Triple) => `${t.s}|${t.p}|${t.o}`;

export function diffTriples(closure: Triple[], asserted: Triple[]): Triple[] {
  const seen = new Set(asserted.map(key));
  return closure.filter((t) => !seen.has(key(t)));
}

function short(iri: string): string {
  const tail = iri.split(/[/#:]/).pop() ?? iri;
  try { return decodeURIComponent(tail); } catch { return tail; }
}

export function inferredToItems(rows: Triple[]): ValidationItem[] {
  if (!rows.length) {
    return [{ severity: "info", message: "No inferred connections. Add types and relations to your notes, then reason again." }];
  }
  return rows.map((t) => ({
    severity: "inferred",
    message: `${short(t.s)} — ${short(t.p)} → ${short(t.o)}`,
  }));
}
```

- [ ] **Step 4: Run to verify it passes** — `npm test -- tests/inferred.test.ts` → PASS.

- [ ] **Step 5: Add the command to `main.ts` `onload()`**

The order matters: capture the asserted state, reason, capture the closure, diff. `onto_clear` + reload of the vault N-Triples restores the asserted-only graph afterwards so a later sync is not confused by materialised triples.

```ts
this.addCommand({
  id: "oo-inferred-connections",
  name: "Show inferred connections for this note",
  checkCallback: (checking) => {
    const f = this.app.workspace.getActiveFile();
    if (!f || f.extension !== "md") return false;
    if (!checking) {
      (async () => {
        const c = await this.client();
        const rules = this.rules();
        const subject = `<${mintNoteIri(f.path, rules.iriBase)}>`;
        const q = `SELECT ?p ?o WHERE { ${subject} ?p ?o . FILTER(isIRI(?o)) }`;
        const before = parseBindings(await c.call("onto_query", { query: q })) ?? [];
        await c.call("onto_reason", { profile: "owl-rl" });
        const after = parseBindings(await c.call("onto_query", { query: q })) ?? [];
        const toTriple = (r: Record<string, string>): Triple => ({ s: subject, p: r.p, o: r.o });
        const fresh = diffTriples(after.map(toTriple), before.map(toTriple));
        await this.showValidation(inferredToItems(fresh));
        await this.syncVault(); // reload asserted-only state after materialisation
      })().catch((e) => new Notice(e.message, 8000));
    }
    return true;
  },
});
```

Add imports: `import { diffTriples, inferredToItems, Triple } from "./inferred";`, `import { mintNoteIri } from "./mapper/iri";`, `import { parseBindings } from "./sparql";`. Add a `.oo-sev-inferred { border-left-color: var(--text-accent); }` rule to `styles.css`.

- [ ] **Step 6: Build, test, commit**

```bash
npm run build && npm test
git add src/inferred.ts src/main.ts src/views/validation.ts styles.css tests/inferred.test.ts
git commit -m "feat(inferred): surface entailed connections a note never asserted"
```

---

### Task 13: test-vault, e2e test against the real engine, CI

**Files:**
- Create: `test-vault/People/Ada Lovelace.md`, `test-vault/Charles Babbage.md`, `test-vault/ontology/schema.ttl`, `test-vault/ontology/shapes.ttl`, `tests/e2e.test.ts`, `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: everything. The e2e test is the arbiter for two assumptions unit tests cannot check: the exact `onto_load` argument shape (`{input, inline, name}`) and the `onto_query` output shape accepted by `parseBindings`. If either fails, fix the client/main call-sites, not the engine.

- [ ] **Step 1: Create test-vault fixtures**

`test-vault/People/Ada Lovelace.md`:

```markdown
---
type: "[[Person]]"
born: 1815
---
collaboratedWith:: [[Charles Babbage]]

Wrote the first algorithm. #mathematician
```

`test-vault/Charles Babbage.md`:

```markdown
---
type: "[[Person]]"
---
Designed the Analytical Engine, see [[People/Ada Lovelace]].
```

`test-vault/ontology/schema.ttl`:

```turtle
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
<vault:Person> a owl:Class ; rdfs:label "Person" .
<vault:collaboratedWith> a owl:ObjectProperty ; rdfs:domain <vault:Person> ; rdfs:range <vault:Person> .
```

`test-vault/ontology/shapes.ttl`:

```turtle
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
<vault:PersonShape> a sh:NodeShape ;
  sh:targetClass <vault:Person> ;
  sh:property [ sh:path <vault:born> ; sh:datatype xsd:integer ; sh:maxCount 1 ] .
```

- [ ] **Step 2: Write the e2e test** (`tests/e2e.test.ts`, skipped unless `OO_E2E=1` and `OO_ENGINE_BIN` set)

```ts
import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { spawn, ChildProcess } from "node:child_process";
import net from "node:net";
import { EngineClient } from "../src/engine/client";
import { mapVault } from "../src/mapper/mapper";
import { DEFAULT_RULES } from "../src/mapper/rules";
import { parseBindings } from "../src/sparql";

const run = process.env.OO_E2E === "1" && !!process.env.OO_ENGINE_BIN;
let child: ChildProcess;
let client: EngineClient;

(run ? describe : describe.skip)("e2e against real engine", () => {
  beforeAll(async () => {
    const port = await new Promise<number>((res) => {
      const s = net.createServer();
      s.listen(0, "127.0.0.1", () => { const p = (s.address() as any).port; s.close(() => res(p)); });
    });
    child = spawn(process.env.OO_ENGINE_BIN!, ["serve-http", "--host", "127.0.0.1", "--port", String(port)], { stdio: ["ignore", "ignore", "inherit"] });
    client = new EngineClient(`http://127.0.0.1:${port}`);
    let ok = false;
    for (let i = 0; i < 60; i++) {
      try { await client.health(); ok = true; break; } catch { await new Promise((r) => setTimeout(r, 500)); }
    }
    expect(ok).toBe(true);
    await client.initialize();
  }, 60000);

  afterAll(() => { child?.kill(); });

  it("loads mapped vault triples and queries them back", async () => {
    const nt = mapVault([
      { path: "People/Ada Lovelace.md", frontmatter: { type: "[[Person]]", born: 1815 }, links: [], inlineFields: [{ key: "collaboratedWith", target: "Charles Babbage.md" }], tags: ["mathematician"] },
    ], DEFAULT_RULES);
    const res = await client.call("onto_load", { input: nt, inline: true, name: "vault" });
    expect(res).not.toContain('"error"');
    const text = await client.call("onto_query", { query: "SELECT ?s WHERE { ?s <vault:collaboratedWith> ?o }" });
    const rows = parseBindings(text);
    expect(rows, `unparseable query output: ${text}`).not.toBeNull();
    expect(JSON.stringify(rows)).toContain("Ada");
  }, 30000);

  it("reasons without error", async () => {
    const res = await client.call("onto_reason", { profile: "owl-rl" });
    expect(res).not.toContain('"error"');
  }, 30000);
});
```

- [ ] **Step 3: Run e2e locally against a built engine**

Run: `OO_E2E=1 OO_ENGINE_BIN=/Users/fabio/projects/open-ontologies/target/release/open-ontologies npm test -- tests/e2e.test.ts`
Expected: PASS. If `onto_load` rejects the `{input, inline, name}` argument shape, read the engine's `OntoLoadInput` struct in `src/server.rs` and correct the argument names here and in `syncVault()` (Task 9) to match; if `parseBindings` returns null, extend it for the engine's actual `sparql_select` JSON shape. Commit whatever correction reality dictates.

- [ ] **Step 4: CI workflow** — `.github/workflows/ci.yml`:

```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20 }
      - run: npm ci
      - run: npm run build
      - run: npm test
  e2e:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20 }
      - run: npm ci
      - name: Download engine
        run: |
          curl -fL -o /usr/local/bin/open-ontologies \
            https://github.com/fabio-rovai/open-ontologies/releases/download/v1.1.1/open-ontologies-x86_64-unknown-linux-gnu
          chmod +x /usr/local/bin/open-ontologies
      - run: OO_E2E=1 OO_ENGINE_BIN=/usr/local/bin/open-ontologies npm test
```

- [ ] **Step 5: Commit**

```bash
git add test-vault tests/e2e.test.ts .github/workflows/ci.yml
git commit -m "test: e2e against real engine, test vault, CI"
```

---

### Task 14: Plugin release workflow and README

**Files:**
- Create: `.github/workflows/release.yml`, `README.md`

- [ ] **Step 1: Release workflow** — `.github/workflows/release.yml`:

```yaml
name: Release
on:
  push:
    tags: ['[0-9]+.[0-9]+.[0-9]+']
jobs:
  release:
    runs-on: ubuntu-latest
    permissions: { contents: write }
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20 }
      - run: npm ci
      - run: npm run build
      - name: Check tag matches manifest version
        run: |
          v=$(node -p "require('./manifest.json').version")
          [ "$v" = "$GITHUB_REF_NAME" ] || { echo "manifest version $v != tag $GITHUB_REF_NAME"; exit 1; }
      - uses: softprops/action-gh-release@v2
        with:
          files: |
            main.js
            manifest.json
            styles.css
```

- [ ] **Step 2: README.md** (plugin repo)

```markdown
# Open Ontologies for Obsidian

The full [Open Ontologies](https://github.com/fabio-rovai/open-ontologies) engine inside Obsidian. All engine tools, reasoners included: validate, reason over, SHACL-check, diff, lint and SPARQL-query ontology files in your vault, and compile the vault itself (notes, frontmatter, tags, wikilinks) into an RDF graph you can reason over.

Desktop only. On first run the plugin downloads the pinned engine release for your platform (SHA-256 verified against the published SHASUMS.txt), starts it on a loopback-only port, and manages its lifecycle. Already have the engine installed? Point the plugin at your binary in settings.

## Features

- Ontology tree pane, SPARQL console with history, validation panel with per-note deep links
- Validate-on-save for .ttl, .owl, .rdf and .jsonld files
- Vault to RDF mapping: notes become individuals, `type:` frontmatter becomes `rdf:type`, `property:: [[Target]]` inline fields become object properties, tags become SKOS concepts
- Inferred connections: see the links your notes entail but never stated
- Every engine tool is reachable; reasoning profiles from RDFS to full OWL 2 DL tableaux

## Give your AI agent a reasoned second brain

The engine this plugin runs is an MCP server, so Claude Code and Claude Desktop can query your vault as a knowledge graph. This is a different thing from letting an agent read your files. Tools like mcp-obsidian already do file read, write and search well, and this plugin does not duplicate them. What it adds is reasoning: your agent can ask which notes violate a shape, what is transitively part of a project, or which people sit two hops from a topic, and get answers derived from the graph rather than guessed from text.

Open the plugin settings, copy the MCP client config, and paste it into your Claude Code config:

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

The endpoint listens on loopback only and always requires the generated token. Treat that token like a password: anything holding it can read and modify your vault graph.

One honest caveat about how much this buys you. Reasoning needs types. A vault of untyped notes and plain wikilinks compiles to a flat graph of "links to" statements, and a reasoner derives nothing interesting from it. Run the "Install starter vault ontology" command, add `type:` frontmatter to the notes you care about, and use `property:: [[Target]]` for relations that mean something. The payoff scales with how much structure your vault actually carries.

## Install

Until the community-store listing lands, install with [BRAT](https://github.com/TfTHacker/obsidian42-brat) using this repo, or copy `main.js`, `manifest.json` and `styles.css` from the latest release into `.obsidian/plugins/open-ontologies/`.

## Sibling channels

The same engine ships as a [Rust binary and MCP server](https://github.com/fabio-rovai/open-ontologies), a [Docker image](https://github.com/fabio-rovai/open-ontologies/pkgs/container/open-ontologies) and a [PyPI package](https://pypi.org/project/open-ontologies-lite/).
```

- [ ] **Step 3: Commit, tag, verify release CI**

```bash
git add .github/workflows/release.yml README.md && git commit -m "chore: release workflow and README"
```

(Do not tag `0.1.0` until manual QA in Task 16 passes.)

---

### Task 15: Engine-repo README and docs cross-link

**Files:**
- Modify: `/Users/fabio/projects/open-ontologies/README.md` (add an "Obsidian" collapsible in the install/connect section, alongside the existing client sections)
- Modify: `/Users/fabio/projects/open-ontologies/docs/` — add `docs/obsidian.md`

- [ ] **Step 1: Add README section** — after the existing MCP-client `<details>` blocks, insert:

```markdown
<details>
<summary><strong>Obsidian</strong></summary>

The [Open Ontologies plugin for Obsidian](https://github.com/fabio-rovai/obsidian-open-ontologies) runs this engine as a managed sidecar inside Obsidian: ontology tree, SPARQL console, validation panel, validate-on-save for Turtle files, and a vault-to-RDF mapper so your notes become a graph the reasoners can work on. Desktop only; the plugin auto-downloads the pinned engine release, or point it at an existing binary.
</details>
```

- [ ] **Step 2: Create `docs/obsidian.md`** describing the channel (copy the plugin README's feature list, add the settings reference: `enginePath`, mapping YAML keys with the `MappingRules` defaults from Task 3).

- [ ] **Step 3: Commit in the engine repo**

```bash
cd /Users/fabio/projects/open-ontologies
git add README.md docs/obsidian.md && git commit -m "docs: Obsidian distribution channel"
```

---

### Task 16: Manual QA in a real vault, then release + store submission

**Files:**
- No new code. Uses `test-vault/`.

- [ ] **Step 1: Manual QA** — copy the build into the test vault and exercise every surface:

```bash
cd /Users/fabio/projects/obsidian-open-ontologies
npm run build
mkdir -p test-vault/.obsidian/plugins/open-ontologies
cp main.js manifest.json styles.css test-vault/.obsidian/plugins/open-ontologies/
open -a Obsidian test-vault
```

Checklist (each must pass): plugin enables without error; engine auto-downloads and `/health` succeeds (check console log); sync vault command reports success; tree pane shows `Person` after loading `ontology/schema.ttl` via validate + sync; SPARQL console returns rows for the default query; editing `schema.ttl` triggers validate-on-save into the validation panel; SHACL command against `ontology/shapes.ttl` produces a panel result; restart-engine command works; disabling the plugin kills the engine process (verify with `pgrep -f "open-ontologies serve-http"` → empty).

- [ ] **Step 2: Fix anything the checklist surfaces, commit fixes.**

- [ ] **Step 3: Create GitHub repo, push, tag**

```bash
cd /Users/fabio/projects/obsidian-open-ontologies
gh repo create fabio-rovai/obsidian-open-ontologies --public --source . --push
git tag 0.1.0 && git push origin 0.1.0
```

Verify: release CI publishes `main.js`, `manifest.json`, `styles.css` on the `0.1.0` release.

- [ ] **Step 4: Store submission** — fork `obsidianmd/obsidian-releases`, append to `community-plugins.json`:

```json
{
  "id": "open-ontologies",
  "name": "Open Ontologies",
  "author": "Fabio Rovai",
  "description": "Validate, reason over, SHACL-check and SPARQL-query ontology files and your vault as RDF, powered by the full Open Ontologies engine.",
  "repo": "fabio-rovai/obsidian-open-ontologies"
}
```

Open the PR following their template (checklist: release assets present, `isDesktopOnly` truthful, no network calls beyond the declared engine download, id unique). Store review is asynchronous; BRAT install works meanwhile.

---

## Self-review notes

- Spec coverage: sidecar lifecycle (T7), full-tool access via `call()` (T5), both data planes (T4 files via T9/T10 commands), three panes (T10-T12), validate-on-save (T10), settings + log (T9), error handling (T7 backoff/version gate, T9 notices), testing (per-task + T13 e2e + CI), distribution (T14/T16), engine README (T15). Loopback-only: engine defaults to 127.0.0.1 and we pass `--host 127.0.0.1` explicitly (T7), satisfying the spec's security decision with no engine change.
- Known reality-check points were concentrated in T13 (e2e). **All three were wrong in the plan and have been corrected in the implementation** (verified against engine v1.1.1 at `target/release/open-ontologies`):
  1. **SSE priming frame.** The engine opens every stream with `data: \nid: 0\nretry: 3000\n\n` — an empty `data:` payload. A parser that `JSON.parse`s every `data:` line throws `Unexpected end of JSON input` on the first call. `EngineClient.post` now skips empty payloads, tolerates `\r`, and ignores unparseable frames. The mock server in `tests/client.test.ts` emits the priming frame so this stays covered.
  2. **`onto_load` arguments.** Takes `turtle` (inline) or `path`, plus optional `name` / `auto_refresh` / `force_recompile` — **not** `input`/`inline`. Confirmed in engine `src/inputs.rs:25`. Other tools were as planned: `onto_validate`/`onto_lint` take `input`+`inline`, `onto_shacl` takes `shapes`, `onto_reason` takes `profile`, `onto_diff` takes `old_path`/`new_path`, `onto_query` takes `query`.
  3. **`onto_query` output.** Returns `{"variables": [...], "results": [{var: term}, ...]}` — a third shape, and the terms are raw RDF serializations (`<vault:X>`, `"lit"@en`, `"1815"^^<...>`). Left unnormalized, angle brackets propagate into the tree view's `iriBase` prefix check and the inferred-connections labels. `parseBindings` now handles the engine shape and normalizes every term centrally via `normalizeTerm`.
- The e2e also verifies two claims the design rests on: an unauthenticated MCP call is rejected, and the starter ontology's transitive `partOf` actually entails `T1 partOf Prog` from `T1 partOf P1 partOf Prog`. The `onto_diff` (`old_path`/`new_path`, verified in engine `src/server.rs`) names are confirmed; `onto_pack`/`onto_unpack` argument names should be confirmed against `OntoPackInput`/`OntoUnpackInput` in engine `src/server.rs` during Task 9 and corrected in place if they differ from `{path}`.
- Deferred consciously: registering a text editor view for `.ttl` (Obsidian cannot edit non-md files without a custom view; validate-on-save still works when files are edited externally or via other plugins, and the workbench commands accept the active file when opened with other viewers). If in-Obsidian ttl editing is wanted, add a `TextFileView` in v0.2.
