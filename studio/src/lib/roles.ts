/**
 * Viewer roles offered in the UI.
 *
 * These mirror `src-tauri/sidecars/agent/acl.ts`, which is the component that
 * actually enforces them. The duplication is deliberate: the frontend and the
 * sidecar are separate processes with separate build graphs, and a shared
 * module between them would mean bundling sidecar code into the web build.
 *
 * DENY BY DEFAULT, on both sides of that duplication. An id sent from here
 * that the sidecar does not recognise must be denied there, not treated as
 * unrestricted, and this module's own `resolveVisibility` below must fail the
 * same way: an unrecognised role id or an empty group list withholds every
 * document, so the two lists cannot silently drift into one filtering and the
 * other not. Defaulting either side to full access on a mismatch would still
 * "work" in the sense of not crashing, which is exactly what makes it
 * dangerous: it would visibly stop filtering while looking like it was.
 */

export interface RoleOption {
  id: string;
  label: string;
  hint: string;
  /** ACL groups this role belongs to, exactly as `acl.ts`'s `Role.groups`. */
  groups: string[];
  /**
   * True only for the single role that deliberately bypasses access control.
   * Never inferred from `groups.length === 0`: a role with no groups yet is
   * restricted to nothing, not unrestricted.
   */
  unrestricted: boolean;
}

export const ROLE_OPTIONS: RoleOption[] = [
  { id: 'all', label: 'Unrestricted', hint: 'No access control applied', groups: [], unrestricted: true },
  {
    id: 'editor', label: 'Catalogue Editor',
    hint: 'Every dataset and distribution record, published or draft',
    groups: ['editor'], unrestricted: false,
  },
  {
    id: 'reader', label: 'Public Reader',
    hint: 'Published records only; drafts and review notes withheld',
    groups: ['public'], unrestricted: false,
  },
  {
    id: 'compliance', label: 'Compliance Reviewer',
    hint: 'Conformance and licence-review records',
    groups: ['compliance'], unrestricted: false,
  },
  {
    id: 'publisher-ops', label: 'Publisher Operations',
    hint: 'Publisher and distribution records under active curation',
    groups: ['publisher-ops'], unrestricted: false,
  },
];

/**
 * Resolve a role id to the documents it may see.
 *
 * A pure, synchronous twin of `acl.ts`'s `summarise`, over documents already
 * fetched by the caller, so the UI can show "N of M documents visible"
 * without a round trip. It must fail exactly the way the sidecar does:
 *
 *   - a role granted a subset of groups sees exactly the documents carrying
 *     one of those groups;
 *   - a role with an empty group list sees nothing, not everything, because
 *     `unrestricted` is a separate, explicit flag rather than inferred from
 *     the group list being empty;
 *   - an id this list does not recognise is DENIED (sees nothing) rather than
 *     falling back to the unrestricted role.
 *
 * `roleId === undefined` is the one case treated as "no selection made" and
 * resolves to the unrestricted role, matching the dropdown's own default
 * state, a different situation from a role name that was supplied but not
 * recognised.
 */
export function resolveVisibility(
  roleId: string | undefined,
  docs: Array<{ doc: string; groups: string[] }>,
): { visible: string[]; withheld: string[] } {
  if (roleId === undefined) {
    return visibilityFor(ROLE_OPTIONS.find(r => r.id === 'all')!, docs);
  }
  const role = ROLE_OPTIONS.find(r => r.id === roleId);
  if (!role) {
    // Unrecognised role name: deny everything rather than guessing.
    return { visible: [], withheld: docs.map(d => d.doc) };
  }
  return visibilityFor(role, docs);
}

/** The pure per-role decision `resolveVisibility` wraps with id lookup. */
export function visibilityFor(
  role: RoleOption,
  docs: Array<{ doc: string; groups: string[] }>,
): { visible: string[]; withheld: string[] } {
  if (role.unrestricted) {
    return { visible: docs.map(d => d.doc), withheld: [] };
  }
  const visible: string[] = [];
  const withheld: string[] = [];
  for (const d of docs) {
    (d.groups.some(g => role.groups.includes(g)) ? visible : withheld).push(d.doc);
  }
  return { visible, withheld };
}

/**
 * Where the knowledge graph is stored.
 *
 * Only the embedded engine is wired. The rest are shown because the store is
 * a deployment choice rather than an architectural one, and the question comes
 * up in every conversation. Selecting one does not move any data.
 */
export interface StoreOption {
  id: string;
  label: string;
  available: boolean;
}

export const STORE_OPTIONS: StoreOption[] = [
  { id: 'embedded', label: 'Embedded engine', available: true },
  { id: 'neptune', label: 'Amazon Neptune', available: false },
  { id: 'aws-semantic', label: 'AWS semantic platform', available: false },
];
