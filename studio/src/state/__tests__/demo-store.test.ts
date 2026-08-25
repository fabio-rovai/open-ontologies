import { describe, it, expect, vi, beforeEach } from 'vitest'
import type { DemoSource } from '../../lib/demo-source'

const getDemoSource = vi.fn<() => Promise<DemoSource>>()
vi.mock('../../lib/source-factory', () => ({ getDemoSource: () => getDemoSource() }))

// Imported after the mock so the store's own `import { getDemoSource } from
// '../lib/source-factory'` resolves to the mocked function above.
const { useDemoStore } = await import('../demo-store')

function fakeSource(overrides: Partial<DemoSource> = {}): DemoSource {
  return {
    corpus: vi.fn(async () => [{ id: 'doc-1', title: 'doc-1', text: 'hello' }]),
    graph: vi.fn(async () => ({ classes: [{ iri: 'ex:Dataset' }], properties: [], edges: [] })),
    findings: vi.fn(async () => [
      { id: 'f1', subject: 'ex:s', kind: 'conformance' as const, claims: [] },
    ]),
    resolve: vi.fn(async () => {}),
    ask: vi.fn(async function* () {
      yield { type: 'text' as const, value: 'an answer' }
    }),
    ...overrides,
  }
}

const RESET = useDemoStore.getState()

beforeEach(() => {
  useDemoStore.setState(RESET, true)
  getDemoSource.mockReset()
})

describe('useDemoStore.load', () => {
  it('populates graph, findings and documents from the source', async () => {
    getDemoSource.mockResolvedValue(fakeSource())
    await useDemoStore.getState().load()
    const s = useDemoStore.getState()
    expect(s.graph.classes).toHaveLength(1)
    expect(s.findings).toHaveLength(1)
    expect(s.documents).toEqual([{ id: 'doc-1', title: 'doc-1', text: 'hello' }])
    expect(s.documentsError).toBeNull()
    expect(s.error).toBeNull()
    expect(s.loading).toBe(false)
  })

  it('surfaces an error and never masks it with empty data when graph/findings fail', async () => {
    getDemoSource.mockResolvedValue({
      ...fakeSource(),
      graph: vi.fn(async () => {
        throw new Error('engine not listening')
      }),
    })
    await useDemoStore.getState().load()
    const s = useDemoStore.getState()
    expect(s.error).toMatch(/engine not listening/)
    expect(s.loading).toBe(false)
  })

  // The behaviour this test protects: live-source.ts's corpus() throws
  // (no tool returns document text in live mode), and that must degrade
  // only the corpus panel, not the whole shell.
  it('keeps graph and findings even when corpus() has no honest implementation', async () => {
    getDemoSource.mockResolvedValue({
      ...fakeSource(),
      corpus: vi.fn(async () => {
        throw new Error('no tool or Tauri command returns document text')
      }),
    })
    await useDemoStore.getState().load()
    const s = useDemoStore.getState()
    expect(s.graph.classes).toHaveLength(1)
    expect(s.findings).toHaveLength(1)
    expect(s.error).toBeNull()
    expect(s.documents).toEqual([])
    expect(s.documentsError).toMatch(/document text/)
  })
})

describe('useDemoStore.refreshGraph', () => {
  it('re-fetches only the graph, leaving findings and documents alone', async () => {
    useDemoStore.setState({ findings: [{ id: 'kept', subject: 's', kind: 'conformance', claims: [] }] })
    getDemoSource.mockResolvedValue(
      fakeSource({ graph: vi.fn(async () => ({ classes: [{ iri: 'ex:New' }], properties: [], edges: [] })) }),
    )
    await useDemoStore.getState().refreshGraph()
    const s = useDemoStore.getState()
    expect(s.graph.classes).toEqual([{ iri: 'ex:New' }])
    expect(s.findings).toEqual([{ id: 'kept', subject: 's', kind: 'conformance', claims: [] }])
  })
})

describe('useDemoStore.select', () => {
  it('sets and clears the selected finding', () => {
    useDemoStore.getState().select('f1')
    expect(useDemoStore.getState().selectedFinding).toBe('f1')
    useDemoStore.getState().select(null)
    expect(useDemoStore.getState().selectedFinding).toBeNull()
  })
})

describe('useDemoStore.resolve', () => {
  it('appends to the ledger on success', async () => {
    const source = fakeSource()
    getDemoSource.mockResolvedValue(source)
    await useDemoStore.getState().resolve('f1', { kind: 'accept' })
    expect(useDemoStore.getState().ledger).toEqual([{ id: 'f1', decision: { kind: 'accept' } }])
    expect(source.resolve).toHaveBeenCalledWith('f1', { kind: 'accept' })
  })

  it('surfaces an error and does not touch the ledger on failure', async () => {
    getDemoSource.mockResolvedValue({
      ...fakeSource(),
      resolve: vi.fn(async () => {
        throw new Error('no engine tool resolves a finding by id')
      }),
    })
    await useDemoStore.getState().resolve('f1', { kind: 'accept' })
    const s = useDemoStore.getState()
    expect(s.ledger).toEqual([])
    expect(s.error).toMatch(/no engine tool/)
  })
})

describe('useDemoStore.ask', () => {
  it('appends streamed chunks to the chat log', async () => {
    getDemoSource.mockResolvedValue(fakeSource())
    await useDemoStore.getState().ask('what disagrees?')
    const s = useDemoStore.getState()
    expect(s.chat).toEqual([{ type: 'text', value: 'an answer' }])
    expect(s.chatPending).toBe(false)
  })

  // A distinct chunk TYPE, not wording, is what tells an unscripted answer
  // apart from a real one; the store must pass that through untouched.
  it('preserves the unscripted chunk type rather than collapsing it into text', async () => {
    getDemoSource.mockResolvedValue(
      fakeSource({
        ask: vi.fn(async function* () {
          yield {
            type: 'unscripted' as const,
            value: 'This is the offline replay of the demonstration.',
          }
        }),
      }),
    )
    await useDemoStore.getState().ask('something nobody scripted')
    expect(useDemoStore.getState().chat).toEqual([
      { type: 'unscripted', value: 'This is the offline replay of the demonstration.' },
    ])
  })

  it('surfaces an error rather than leaving chatPending stuck', async () => {
    getDemoSource.mockResolvedValue({
      ...fakeSource(),
      ask: vi.fn(() => {
        throw new Error('chat does not run through DemoSource in live mode')
      }),
    })
    await useDemoStore.getState().ask('anything')
    const s = useDemoStore.getState()
    expect(s.chatPending).toBe(false)
    expect(s.error).toMatch(/agent sidecar|live mode/i)
  })
})
