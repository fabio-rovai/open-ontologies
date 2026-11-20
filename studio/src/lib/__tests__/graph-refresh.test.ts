import { describe, it, expect, vi, beforeEach } from 'vitest'

// Fresh module per test: the registry is module state, and a listener left
// over from one test must not answer a notification in the next.
let graphRefresh: typeof import('../graph-refresh')
beforeEach(async () => {
  vi.resetModules()
  graphRefresh = await import('../graph-refresh')
})

// In live mode TreeView and LiveChrome both want to hear about an ontology
// change made through chat or the Property Inspector. They used to share one
// window global: TreeView set it and deleted it on unmount, which also removed
// the wrapper LiveChrome had put around it, so after leaving the 2D view no
// change refreshed anything (#421).
describe('graph refresh registry', () => {
  it('keeps notifying the chrome after the tree view unsubscribes (#421)', () => {
    const { onGraphChanged, notifyGraphChanged } = graphRefresh
    const loadTree = vi.fn()
    const refreshStore = vi.fn()

    // Mount order: TreeView is LiveChrome's child, so its effect runs first.
    const offTree = onGraphChanged(loadTree)
    const offChrome = onGraphChanged(refreshStore)

    notifyGraphChanged()
    expect(loadTree).toHaveBeenCalledTimes(1)
    expect(refreshStore).toHaveBeenCalledTimes(1)

    // Switching from 2D to GRAPH or 3D unmounts TreeView.
    offTree()
    notifyGraphChanged()
    expect(loadTree).toHaveBeenCalledTimes(1)
    expect(refreshStore).toHaveBeenCalledTimes(2)

    // Switching back mounts it again, and both hear the next change.
    const offTree2 = onGraphChanged(loadTree)
    notifyGraphChanged()
    expect(loadTree).toHaveBeenCalledTimes(2)
    expect(refreshStore).toHaveBeenCalledTimes(3)

    offTree2()
    offChrome()
    notifyGraphChanged()
    expect(loadTree).toHaveBeenCalledTimes(2)
    expect(refreshStore).toHaveBeenCalledTimes(3)
  })

  it('removes only the subscription it was returned for, even for the same function', () => {
    const { onGraphChanged, notifyGraphChanged } = graphRefresh
    const refresh = vi.fn()
    const offA = onGraphChanged(refresh)
    onGraphChanged(refresh)
    offA()
    notifyGraphChanged()
    expect(refresh).toHaveBeenCalledTimes(1)
  })

  it('unsubscribing twice does not remove anyone else', () => {
    const { onGraphChanged, notifyGraphChanged } = graphRefresh
    const a = vi.fn()
    const b = vi.fn()
    const offA = onGraphChanged(a)
    onGraphChanged(b)
    offA()
    offA()
    notifyGraphChanged()
    expect(a).not.toHaveBeenCalled()
    expect(b).toHaveBeenCalledTimes(1)
  })

  it('a listener that throws or rejects does not stop the others', async () => {
    const { onGraphChanged, notifyGraphChanged } = graphRefresh
    const error = vi.spyOn(console, 'error').mockImplementation(() => {})
    const after = vi.fn()
    onGraphChanged(() => {
      throw new Error('tree query failed')
    })
    onGraphChanged(async () => {
      throw new Error('store refresh failed')
    })
    onGraphChanged(after)
    expect(() => notifyGraphChanged()).not.toThrow()
    expect(after).toHaveBeenCalledTimes(1)
    await Promise.resolve()
    expect(error).toHaveBeenCalledTimes(2)
    error.mockRestore()
  })

  it('notifying with nobody subscribed is a no-op', () => {
    expect(() => graphRefresh.notifyGraphChanged()).not.toThrow()
  })
})
