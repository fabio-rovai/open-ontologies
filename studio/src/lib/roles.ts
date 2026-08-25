/**
 * Viewer roles offered in the UI.
 *
 * Nothing in the sidecar's live chat loop currently enforces these: the
 * agent sidecar's retrieval path (graphrag.ts) takes no role parameter, so a
 * live chat session is not filtered by role today. This module and
 * `GovernancePanel.tsx` (which renders it) show what access control WOULD
 * withhold, computed live from the corpus's own :aclGroup / :classification
 * triples via `resolveVisibility` below, not a claim that retrieval is
 * currently gated by it. An earlier version of this module's docstring
 * claimed a sidecar module (`acl.ts`) enforced these roles; nothing imported
 * that module except its own test, so nothing actually enforced anything,
 * and it has been removed rather than left as a claim this codebase could
 * not back up.
 *
 * DENY BY DEFAULT regardless: an unrecognised role id or an empty group list
 * withholds every document rather than defaulting to full access, because a
 * silent fallback to "everything visible" would look identical to filtering
 * having run, which is exactly what makes it dangerous.
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

// 'aws-semantic' / 'AWS semantic platform' has been removed: it named no
// real hosted product. Amazon Neptune is AWS's actual graph/RDF offering and
// stays as a genuinely disclosed-but-unavailable option.
export const STORE_OPTIONS: StoreOption[] = [
  { id: 'embedded', label: 'Embedded engine', available: true },
  { id: 'neptune', label: 'Amazon Neptune', available: false },
];
