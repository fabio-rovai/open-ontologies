/**
 * Access control over retrieval.
 *
 * The corpus documents carry their own access metadata: every header states a
 * Classification and an Acl Group, and `demo/acl_normalise.py` lifts those into
 * one flat `:aclGroup` triple per document per group. This module is the half
 * that enforces them.
 *
 * Enforcement is at RETRIEVAL time, not at presentation time. A document the
 * viewer's role cannot reach never enters the subgraph, so it cannot reach the
 * model, cannot be paraphrased into an answer, and cannot leak through a
 * citation. Filtering after generation would be theatre: the model would
 * already have seen the text.
 *
 * The consequence is deliberate and worth showing rather than hiding. Two
 * roles asking the same question can receive different answers, and a role
 * that can see only one side of a disputed fact receives a confident answer
 * that the full corpus would contradict. That is not a defect of the method;
 * it is what access control means, made visible.
 *
 * DENY BY DEFAULT. Two failure modes matter more than the happy path:
 *   - An unrecognised role id must be denied, not treated as unrestricted.
 *   - A role with an empty `groups` list must see nothing, not everything.
 * Only the single sentinel role explicitly marked `unrestricted: true` (see
 * `ROLES[0]` below) bypasses filtering; that must never be inferred from an
 * empty group list, because every other role's default state IS an empty
 * group list before someone assigns it access.
 */

import type { McpClient } from './mcp.js';

const NS = process.env.ONTO_NS ?? 'https://w3id.org/dcat-us-demo#';

export interface Role {
  id: string;
  label: string;
  groups: string[];
  /**
   * True only for the one role that deliberately bypasses access control.
   * Never derive this from `groups.length === 0`: a role awaiting a group
   * assignment and a role that has opted out of filtering are different
   * things, and conflating them is what makes an access-control layer fail
   * open.
   */
  unrestricted: boolean;
  /** Shown in the UI so a viewer understands what this role is meant to be. */
  description: string;
}

/**
 * Roles as the corpus defines them. The groups are not invented: they are the
 * exact values found in the document headers, so a role either matches real
 * documents or matches nothing.
 */
export const ROLES: Role[] = [
  {
    id: 'all',
    label: 'Unrestricted',
    groups: [],
    unrestricted: true,
    description: 'No access control applied. Every document is retrievable.',
  },
  {
    id: 'editor',
    label: 'Catalogue Editor',
    groups: ['editor'],
    unrestricted: false,
    description: 'Sees every dataset and distribution record, published or still in draft.',
  },
  {
    id: 'reader',
    label: 'Public Reader',
    groups: ['public'],
    unrestricted: false,
    description: 'Sees only records marked published; drafts and internal review notes are withheld.',
  },
  {
    id: 'compliance',
    label: 'Compliance Reviewer',
    groups: ['compliance'],
    unrestricted: false,
    description: 'Sees the conformance and licence-review records used to check DCAT-US compliance.',
  },
  {
    id: 'publisher-ops',
    label: 'Publisher Operations',
    groups: ['publisher-ops'],
    unrestricted: false,
    description: 'Sees the publisher and distribution records for datasets under active curation.',
  },
];

/**
 * A denial sentinel for a role id this list does not recognise.
 *
 * Returned in place of a lookup failure rather than defaulting to `ROLES[0]`
 * (the unrestricted role). Falling back to full access on an unrecognised
 * name is the exact failure mode this module exists to prevent: it would
 * look like access control was applied, when in fact nothing was withheld.
 */
const DENIED_ROLE: Role = {
  id: '__denied__',
  label: 'Access Denied',
  groups: [],
  unrestricted: false,
  description: 'Unrecognised role: no documents are visible.',
};

/**
 * Resolve a role id.
 *
 * `undefined` means "no role was selected", the legitimate default state of a
 * fresh session, and resolves to the unrestricted role, matching the UI's own
 * default. Any OTHER string that does not match a known id is treated as a
 * bad or spoofed value, not a missing selection, and is denied rather than
 * silently granted the unrestricted role.
 */
export function roleById(id: string | undefined): Role {
  if (id === undefined) return ROLES[0];
  return ROLES.find(r => r.id === id) ?? DENIED_ROLE;
}

/** True when this role applies no restriction at all. */
export function isUnrestricted(role: Role): boolean {
  return role.unrestricted === true;
}

/**
 * A SPARQL fragment constraining `?doc` to documents the role can reach.
 *
 * Returns an empty string for the unrestricted role so the caller's query is
 * unchanged rather than wrapped in a tautology, which keeps the unrestricted
 * path byte-identical to the behaviour before access control existed. Every
 * other role, including one with an empty `groups` list, gets a `VALUES`
 * clause; an empty `VALUES {}` matches nothing, which is what denies a role
 * that has not been assigned to any group.
 */
export function docFilter(role: Role, docVar = '?d'): string {
  if (isUnrestricted(role)) return '';
  const values = role.groups.map(g => `"${g}"`).join(' ');
  return `${docVar} <${NS}aclGroup> ?__g . VALUES ?__g { ${values} }`;
}

export interface AclSummary {
  role: string;
  roleLabel: string;
  visible: string[];
  withheld: string[];
  classifications: Record<string, string>;
}

/**
 * Which documents this role can and cannot reach.
 *
 * Both halves are returned because the withheld list is the honest part: an
 * answer built from a filtered corpus should be able to say how much it could
 * not see, rather than presenting a partial view as a complete one.
 */
export async function summarise(mcp: McpClient, role: Role): Promise<AclSummary> {
  const rows = await query(
    mcp,
    `PREFIX dcus: <${NS}>
     SELECT ?doc ?cls (GROUP_CONCAT(?g; separator=",") AS ?groups) WHERE {
       ?d dcus:docId ?doc .
       OPTIONAL { ?d dcus:classification ?cls }
       OPTIONAL { ?d dcus:aclGroup ?g }
     } GROUP BY ?doc ?cls`,
  );

  const visible: string[] = [];
  const withheld: string[] = [];
  const classifications: Record<string, string> = {};

  for (const r of rows) {
    const doc = r.doc;
    if (!doc) continue;
    classifications[doc] = r.cls ?? 'Unclassified';
    const groups = (r.groups ?? '').split(',').map(s => s.trim()).filter(Boolean);
    // Deny by default: an unrestricted role reaches everything, and every
    // other role reaches a document only when a group it actually holds is
    // named on that document. A role holding no groups at all reaches
    // nothing, because `groups.some(...)` over an empty array is false.
    const reachable = isUnrestricted(role) || groups.some(g => role.groups.includes(g));
    (reachable ? visible : withheld).push(doc);
  }

  visible.sort();
  withheld.sort();
  return { role: role.id, roleLabel: role.label, visible, withheld, classifications };
}

async function query(mcp: McpClient, sparql: string): Promise<Array<Record<string, string>>> {
  try {
    const raw = await mcp.callTool('onto_query', { query: sparql.replace(/\s+/g, ' ') });
    const rows = JSON.parse(raw)?.results ?? [];
    return rows.map((row: Record<string, string>) => {
      const out: Record<string, string> = {};
      for (const [k, v] of Object.entries(row)) out[k] = literal(v);
      return out;
    });
  } catch {
    return [];
  }
}

/** Strip Turtle literal decoration; the engine returns values as written. */
function literal(v: string): string {
  if (!v) return v;
  if (v.startsWith('<') && v.endsWith('>')) return v.slice(1, -1).split('#').pop() ?? v;
  if (v.startsWith('"')) {
    const body = v.slice(1);
    for (const cut of ['"^^', '"@', '"']) {
      const i = body.indexOf(cut);
      if (i >= 0) return body.slice(0, i);
    }
    return body;
  }
  return v;
}
