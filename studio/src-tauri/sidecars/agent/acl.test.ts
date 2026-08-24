import { describe, it, expect } from 'vitest';
import { ROLES, roleById, isUnrestricted, docFilter, summarise, type Role } from './acl.js';
import type { McpClient } from './mcp.js';

/**
 * `acl.ts` is the layer that actually enforces access control (the sidecar
 * queries the graph through it); `studio/src/lib/roles.ts` is only a
 * frontend display mirror. A regression here reaches production undetected
 * unless this file catches it, so these tests exercise `roleById`,
 * `docFilter` and `summarise` directly rather than relying on the mirror's
 * coverage.
 *
 * The same four documents used by `roles.test.ts`'s fixtures, restated here
 * in the `onto_query` row shape `summarise` actually parses (one row per
 * doc, groups comma-joined by GROUP_CONCAT).
 */
const DOC_ROWS = [
  { doc: 'DCAT-001', cls: 'Dataset', groups: 'public' },
  { doc: 'DCAT-002', cls: 'Dataset', groups: 'compliance' },
  { doc: 'DCAT-003', cls: 'Dataset', groups: 'public,compliance' },
  { doc: 'DCAT-004', cls: 'Dataset', groups: 'editor' },
];
const ALL_DOCS = DOC_ROWS.map(r => r.doc).sort();

function mockMcp(rows: Array<Record<string, string>>): McpClient {
  return {
    callTool: async () => JSON.stringify({ results: rows }),
  } as unknown as McpClient;
}

const reader = ROLES.find(r => r.id === 'reader')!;
const unrestricted = ROLES.find(r => r.id === 'all')!;

describe('roleById', () => {
  it('resolves an unknown role id to a denied role, not the unrestricted one', () => {
    const role = roleById('does-not-exist');
    expect(role.unrestricted).toBe(false);
    expect(isUnrestricted(role)).toBe(false);
  });

  it('resolves undefined (no selection made) to the unrestricted role', () => {
    const role = roleById(undefined);
    expect(role.id).toBe('all');
    expect(isUnrestricted(role)).toBe(true);
  });

  it('resolves a known id to the matching role', () => {
    expect(roleById('reader').id).toBe('reader');
  });
});

describe('docFilter', () => {
  it('returns no restriction for the unrestricted role', () => {
    expect(docFilter(unrestricted)).toBe('');
  });

  it('returns an unsatisfiable VALUES clause for a denied (unknown) role', () => {
    const denied = roleById('does-not-exist');
    expect(docFilter(denied)).toBe('?d <https://w3id.org/dcat-us-demo#aclGroup> ?__g . VALUES ?__g {  }');
  });

  it('returns an unsatisfiable VALUES clause for a role with an empty grant list', () => {
    const emptyGrantRole: Role = {
      id: 'unassigned',
      label: 'Unassigned',
      groups: [],
      unrestricted: false,
      description: 'Awaiting a group assignment.',
    };
    expect(docFilter(emptyGrantRole)).toBe('?d <https://w3id.org/dcat-us-demo#aclGroup> ?__g . VALUES ?__g {  }');
  });

  it('returns a VALUES clause naming the granted groups for a subset role', () => {
    expect(docFilter(reader)).toBe('?d <https://w3id.org/dcat-us-demo#aclGroup> ?__g . VALUES ?__g { "public" }');
  });
});

describe('summarise', () => {
  it('an unknown role identifier resolves to a denied role and yields no visible documents', async () => {
    const denied = roleById('does-not-exist');
    const result = await summarise(mockMcp(DOC_ROWS), denied);
    expect(result.visible).toEqual([]);
    expect(result.withheld).toEqual(ALL_DOCS);
  });

  it('a role with an empty grant list yields no visible documents', async () => {
    const emptyGrantRole: Role = {
      id: 'unassigned',
      label: 'Unassigned',
      groups: [],
      unrestricted: false,
      description: 'Awaiting a group assignment.',
    };
    const result = await summarise(mockMcp(DOC_ROWS), emptyGrantRole);
    expect(result.visible).toEqual([]);
    expect(result.withheld).toEqual(ALL_DOCS);
  });

  it('a role with a subset grant yields exactly that subset and withholds the rest', async () => {
    // 'reader' holds only the 'public' group.
    const result = await summarise(mockMcp(DOC_ROWS), reader);
    expect(result.visible).toEqual(['DCAT-001', 'DCAT-003']);
    expect(result.withheld).toEqual(['DCAT-002', 'DCAT-004']);
  });

  it('an explicitly unrestricted role sees everything', async () => {
    const result = await summarise(mockMcp(DOC_ROWS), unrestricted);
    expect(result.visible).toEqual(ALL_DOCS);
    expect(result.withheld).toEqual([]);
  });
});
