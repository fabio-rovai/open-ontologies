import { describe, it, expect } from 'vitest'
import { resolveVisibility, visibilityFor, ROLE_OPTIONS } from '../roles'

/**
 * `roles.ts` exports pure functions (`resolveVisibility`, `visibilityFor`)
 * for resolving a role to the documents it may see, plus the `ROLE_OPTIONS`
 * data they resolve against. These are the real exports: the module has no
 * class or default export, and nothing here is async or requires a live MCP
 * connection.
 *
 * The three cases below are the security-relevant ones. The happy path (an
 * unrestricted role seeing everything) is exercised incidentally by the
 * fixtures but is not itself the point.
 */

const DOCS = [
  { doc: 'DCAT-001', groups: ['public'] },
  { doc: 'DCAT-002', groups: ['compliance'] },
  { doc: 'DCAT-003', groups: ['public', 'compliance'] },
  { doc: 'DCAT-004', groups: ['editor'] },
]

describe('resolveVisibility', () => {
  it('a role granted access to a subset of documents sees exactly that subset', () => {
    const { visible, withheld } = resolveVisibility('reader', DOCS)
    // 'reader' holds only the 'public' group.
    expect(visible.sort()).toEqual(['DCAT-001', 'DCAT-003'])
    expect(withheld.sort()).toEqual(['DCAT-002', 'DCAT-004'])
  })

  it('a role with no grants sees nothing, rather than everything', () => {
    const emptyGrantRole = { id: 'unassigned', label: 'Unassigned', hint: '', groups: [], unrestricted: false }
    const { visible, withheld } = visibilityFor(emptyGrantRole, DOCS)
    expect(visible).toEqual([])
    expect(withheld.sort()).toEqual([...DOCS.map(d => d.doc)].sort())
  })

  it('an unknown role name is denied, rather than defaulting to permitted', () => {
    const { visible, withheld } = resolveVisibility('does-not-exist', DOCS)
    expect(visible).toEqual([])
    expect(withheld.sort()).toEqual([...DOCS.map(d => d.doc)].sort())
  })

  it('the unrestricted sentinel role sees everything, and only that role does', () => {
    const { visible, withheld } = resolveVisibility('all', DOCS)
    expect(withheld).toEqual([])
    expect(visible.sort()).toEqual([...DOCS.map(d => d.doc)].sort())
  })

  it('an undefined role id (no selection made) defaults to unrestricted, not denied', () => {
    const { visible, withheld } = resolveVisibility(undefined, DOCS)
    expect(withheld).toEqual([])
    expect(visible.sort()).toEqual([...DOCS.map(d => d.doc)].sort())
  })

  it('every declared role option other than the sentinel is restricted', () => {
    const nonSentinel = ROLE_OPTIONS.filter(r => r.id !== 'all')
    expect(nonSentinel.length).toBeGreaterThan(0)
    for (const role of nonSentinel) {
      expect(role.unrestricted).toBe(false)
      expect(role.groups.length).toBeGreaterThan(0)
    }
    // Exactly one role is the unrestricted sentinel.
    expect(ROLE_OPTIONS.filter(r => r.unrestricted)).toHaveLength(1)
  })
})
