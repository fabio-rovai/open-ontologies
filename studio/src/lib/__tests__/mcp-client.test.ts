import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

// mcp-client reaches the engine in two ways: the port through a Tauri
// command, and the REST API through fetch. Both are replaced here, and every
// POST body is recorded so a test can see which file a save named.
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string) => (cmd === 'get_engine_port' ? 8137 : undefined)),
}))

let posts: { url: string; body: Record<string, unknown> }[]
let reply: Record<string, unknown>
let mcp: typeof import('../mcp-client')

beforeEach(async () => {
  posts = []
  reply = { ok: true, triples_loaded: 12 }
  vi.stubGlobal(
    'fetch',
    vi.fn(async (url: string, init?: { body?: string }) => {
      if (init?.body) posts.push({ url, body: JSON.parse(init.body) })
      return { json: async () => reply }
    }),
  )
  // Fresh module per test: the project file is module state.
  vi.resetModules()
  mcp = await import('../mcp-client')
})

afterEach(() => {
  vi.unstubAllGlobals()
})

const savedPaths = () => posts.filter((p) => p.url.endsWith('/api/save')).map((p) => p.body.path)
const loadedPaths = () => posts.filter((p) => p.url.endsWith('/api/load')).map((p) => p.body.path)

// After Save As the toolbar names the new file, but every automatic save
// still wrote studio-live.ttl, so the named file went stale (#423).
describe('the project file auto-save writes', () => {
  it('is studio-live.ttl until a Save As', async () => {
    expect(mcp.currentProjectFile()).toBe('~/.open-ontologies/studio-live.ttl')
    await mcp.saveGraphToFile()
    expect(savedPaths()).toEqual(['~/.open-ontologies/studio-live.ttl'])
  })

  it('is the Save As file once that save succeeds (#423)', async () => {
    await mcp.saveGraphAs('~/.open-ontologies/pizza.ttl')
    await mcp.saveGraphToFile()
    expect(savedPaths()).toEqual(['~/.open-ontologies/pizza.ttl', '~/.open-ontologies/pizza.ttl'])
    expect(mcp.currentProjectFile()).toBe('~/.open-ontologies/pizza.ttl')
  })

  it('stays where it was when the Save As is refused', async () => {
    reply = { error: 'permission denied' }
    await expect(mcp.saveGraphAs('/read-only/pizza.ttl')).rejects.toThrow(/permission denied/)
    reply = { ok: true }
    await mcp.saveGraphToFile()
    expect(savedPaths()).toEqual(['/read-only/pizza.ttl', '~/.open-ontologies/studio-live.ttl'])
    expect(mcp.currentProjectFile()).toBe('~/.open-ontologies/studio-live.ttl')
  })

  // useEngine.connect() reloads the file whenever the engine (re)starts. Once
  // edits go to the Save As file, reloading studio-live.ttl would bring back
  // the state from before the Save As, and the next auto-save would write it
  // over the newer file.
  it('is also the file a reconnect reloads', async () => {
    await mcp.saveGraphAs('~/.open-ontologies/pizza.ttl')
    expect(await mcp.loadGraphFromFile()).toBe(true)
    expect(loadedPaths()).toEqual(['~/.open-ontologies/pizza.ttl'])
  })
})
