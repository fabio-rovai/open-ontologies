import { describe, it, expect, vi } from 'vitest'
import { createLiveSource } from '../live-source'

function sparqlJson(bindings: Record<string, string>[]): string {
  return JSON.stringify({
    results: {
      bindings: bindings.map((row) =>
        Object.fromEntries(Object.entries(row).map(([k, v]) => [k, { value: v }])),
      ),
    },
  })
}

describe('LiveSource', () => {
  it('maps the class and subclass queries into a GraphView', async () => {
    const sparqlQuery = vi.fn(async (q: string) =>
      q.includes('rdfs:subClassOf ?b')
        ? sparqlJson([{ a: 'ex:Dataset', b: 'ex:Resource' }])
        : sparqlJson([{ c: 'ex:Dataset', l: 'Dataset' }]),
    )
    const callTool = vi.fn(async () => '[]')
    const src = createLiveSource({ sparqlQuery, callTool })

    const g = await src.graph()
    expect(g.classes).toContainEqual({ iri: 'ex:Dataset', label: 'Dataset' })
    expect(g.edges).toContainEqual({ source: 'ex:Dataset', target: 'ex:Resource' })
  })

  it('surfaces an engine error rather than returning an empty graph', async () => {
    const sparqlQuery = vi.fn(async () => {
      throw new Error('engine not listening')
    })
    const callTool = vi.fn(async () => '[]')
    const src = createLiveSource({ sparqlQuery, callTool })

    await expect(src.graph()).rejects.toThrow('engine not listening')
  })

  // PIVOT (see docs/superpowers/plans/2026-08-24-studio-public-port.md): findings
  // no longer come from a document contradiction scan. onto_contradiction_scan is
  // not a tool the engine exposes. The demonstration is now the validator finding:
  // corpus documents checked against SHACL shapes (demo/dcat_conformance.py runs
  // the same measurement offline). The live implementation therefore calls the
  // engine's real onto_shacl tool and joins each violation's focus node against
  // the corpus's prov:wasDerivedFrom provenance triples, the same predicate
  // Graph3D.tsx and demo/contradiction_scan.py already rely on, to name the real
  // document that asserted the offending fact.
  it('maps SHACL violations into findings that cite the document named by provenance', async () => {
    const callTool = vi.fn(async (name: string, args?: Record<string, unknown>) => {
      expect(name).toBe('onto_shacl')
      expect(args).toMatchObject({ inline: false })
      return JSON.stringify({
        conforms: false,
        violation_count: 1,
        violations: [
          {
            severity: 'Violation',
            focus_node: 'ex:conformance',
            path: 'https://example.org/dcat#distribution',
            constraint: 'minCount',
            message: 'Property <...distribution> has fewer than 1 values',
          },
        ],
      })
    })
    const sparqlQuery = vi.fn(async () =>
      sparqlJson([{ thing: 'ex:conformance', doc: 'profile-readme.md' }]),
    )
    const src = createLiveSource({ sparqlQuery, callTool })

    const found = await src.findings()
    expect(found).toHaveLength(1)
    expect(found[0].subject).toBe('ex:conformance')
    expect(found[0].kind).toBe('conformance')
    expect(found[0].claims.map((c) => c.document)).toEqual(['profile-readme.md'])
  })

  it('surfaces an engine error from findings() rather than returning an empty array', async () => {
    const callTool = vi.fn(async () => {
      throw new Error('engine not listening')
    })
    const sparqlQuery = vi.fn(async () => sparqlJson([]))
    const src = createLiveSource({ sparqlQuery, callTool })

    await expect(src.findings()).rejects.toThrow('engine not listening')
  })

  // onto_shacl reports a bad shapes path as a successful tool call carrying an
  // {"error": ...} payload, not a thrown exception. That must surface as a
  // rejection too, or a missing corpus would silently look like a clean one.
  it('surfaces a SHACL tool error payload rather than treating it as zero findings', async () => {
    const callTool = vi.fn(async () =>
      JSON.stringify({ error: 'Cannot read shapes file: No such file or directory' }),
    )
    const sparqlQuery = vi.fn(async () => sparqlJson([]))
    const src = createLiveSource({ sparqlQuery, callTool })

    await expect(src.findings()).rejects.toThrow('Cannot read shapes file')
  })

  // corpus_documents is not a real MCP tool, and no Tauri command in
  // corpus.rs (corpus_presets, ingest_corpus, read_store, list_graphs,
  // read_decisions, revert_type, list_saved, pick_ontology_file) returns
  // document text either. A live source has no honest way to answer this,
  // so it must say so rather than returning [] (which would read as "empty
  // corpus" instead of "no tool reaches the content").
  it('throws from corpus(): no tool or command returns document text', async () => {
    const callTool = vi.fn(async () => '[]')
    const sparqlQuery = vi.fn(async () => sparqlJson([]))
    const src = createLiveSource({ sparqlQuery, callTool })

    await expect(src.corpus()).rejects.toThrow(/no tool or tauri command/i)
    expect(callTool).not.toHaveBeenCalled()
  })

  // onto_apply applies a plan (plan_id, mode); it has no finding/decision
  // parameters and nothing else resolves a SHACL finding by id.
  it('throws from resolve(): onto_apply has no finding/decision parameters', async () => {
    const callTool = vi.fn(async () => '{}')
    const sparqlQuery = vi.fn(async () => sparqlJson([]))
    const src = createLiveSource({ sparqlQuery, callTool })

    await expect(src.resolve('shacl:ex:conformance|path|minCount|0', { kind: 'accept' })).rejects.toThrow(
      /onto_apply/,
    )
    expect(callTool).not.toHaveBeenCalled()
  })

  // agent_ask is not a real MCP tool. Real chat runs through the agent
  // sidecar (chat.rs / useChat.ts), a completely different transport this
  // source's LiveDeps (sparqlQuery, callTool) has no path to.
  it('throws from ask(): chat runs through the agent sidecar, not an MCP tool', async () => {
    const callTool = vi.fn(async () => '{}')
    const sparqlQuery = vi.fn(async () => sparqlJson([]))
    const src = createLiveSource({ sparqlQuery, callTool })

    const iterator = src.ask('does this profile conform?')[Symbol.asyncIterator]()
    await expect(iterator.next()).rejects.toThrow(/agent sidecar/i)
    expect(callTool).not.toHaveBeenCalled()
  })
})
