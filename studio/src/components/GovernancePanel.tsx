import { useEffect, useState } from 'react';
import { callTool } from '../lib/mcp-client';
import { ROLE_OPTIONS, resolveVisibility } from '../lib/roles';

/**
 * Governance: who can read what, and where sensitive values live.
 *
 * Two controls that are usually asserted in an architecture diagram and never
 * shown running. Both are read from the live graph rather than configured
 * here, so what the panel displays is what the retrieval layer is actually
 * enforcing.
 *
 * ACCESS. Documents carry their own access metadata; the corpus states a
 * Classification and an Acl Group in every header. The withheld list is shown
 * as prominently as the visible one, because the honest part of a filtered
 * answer is knowing how much was filtered. The role picker here is local to
 * this panel: the sidecar's chat loop does not yet take a role parameter (a
 * later task wires the two together), so this reads the same document/group
 * data the sidecar's ACL module would and resolves it the same deny-by-default
 * way, via `resolveVisibility` in `../lib/roles`.
 *
 * CUSTODY. Detected values are replaced by keyed tokens before any model sees
 * the text, and the mapping lives in a vault rather than in the graph. The
 * counts here are read from the graph itself: every token present is a value
 * that is NOT present.
 */

interface TokenKind {
  kind: string;
  count: number;
}

const VAULT_PROVIDER = 'Skyflow';
const NS = 'https://w3id.org/dcat-us-demo#';

export function GovernancePanel() {
  const [roleId, setRoleId] = useState('all');
  const [tokens, setTokens] = useState<TokenKind[]>([]);
  const [docs, setDocs] = useState<Array<{ doc: string; cls: string; groups: string[] }>>([]);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        const raw = await callTool('onto_query', {
          query:
            `PREFIX dcus: <${NS}> ` +
            'SELECT ?doc ?cls (GROUP_CONCAT(?g; separator=", ") AS ?groups) WHERE { ' +
            '?d dcus:docId ?doc . OPTIONAL { ?d dcus:classification ?cls } ' +
            'OPTIONAL { ?d dcus:aclGroup ?g } } GROUP BY ?doc ?cls',
        });
        const rows = JSON.parse(raw)?.results ?? [];
        if (!cancelled) {
          setDocs(
            rows
              .map((r: Record<string, string>) => ({
                doc: lit(r.doc),
                cls: lit(r.cls) || 'Unclassified',
                groups: lit(r.groups).split(',').map(g => g.trim()).filter(Boolean),
              }))
              .sort((a: { doc: string }, b: { doc: string }) => a.doc.localeCompare(b.doc)),
          );
        }
      } catch (e) {
        if (!cancelled) setErr(String(e));
      }

      try {
        // Tokens are entities in the graph whose local name encodes the kind.
        // Counting them here means the figure cannot drift from reality: it is
        // the graph reporting what it holds.
        const raw = await callTool('onto_query', {
          query:
            'SELECT ?s WHERE { ?s ?p ?o . FILTER(CONTAINS(STR(?s), "TOK_")) }',
        });
        const rows = JSON.parse(raw)?.results ?? [];
        const seen = new Set<string>();
        const counts = new Map<string, number>();
        for (const r of rows) {
          const iri = lit(r.s);
          if (seen.has(iri)) continue;
          seen.add(iri);
          const m = /TOK_([A-Z_]+?)_[a-f0-9]+$/.exec(iri);
          const kind = m ? m[1].replace(/_/g, ' ').toLowerCase() : 'other';
          counts.set(kind, (counts.get(kind) ?? 0) + 1);
        }
        if (!cancelled) {
          setTokens([...counts.entries()].map(([kind, count]) => ({ kind, count })).sort((a, b) => b.count - a.count));
        }
      } catch {
        /* token count is informational; a failure here must not blank the panel */
      }
    })();

    return () => { cancelled = true; };
  }, []);

  const { visible: visibleDocs, withheld: withheldDocs } = resolveVisibility(
    roleId,
    docs.map(d => ({ doc: d.doc, groups: d.groups })),
  );
  const visible = new Set(visibleDocs);
  const withheld = new Set(withheldDocs);
  const filtering = withheld.size > 0;
  const role = ROLE_OPTIONS.find(r => r.id === roleId);
  const totalTokens = tokens.reduce((n, t) => n + t.count, 0);

  return (
    <div style={{ padding: '10px 12px', overflow: 'auto', fontSize: 11, color: 'var(--text-primary)' }}>
      <Section title="Access control">
        <select
          value={roleId}
          onChange={e => setRoleId(e.target.value)}
          style={{
            marginBottom: 6, fontSize: 11, padding: '2px 4px',
            background: 'var(--bg-secondary)', color: 'var(--text-primary)',
            border: '1px solid var(--border)', borderRadius: 3,
          }}
        >
          {ROLE_OPTIONS.map(r => (
            <option key={r.id} value={r.id}>{r.label}</option>
          ))}
        </select>

        {!filtering && (
          <div style={{ color: 'var(--text-secondary)', marginBottom: 6 }}>
            {roleId === 'all'
              ? 'No role selected: every document is retrievable. Pick a role to enforce the corpus’s own access groups.'
              : 'This role reaches every document currently in the graph.'}
          </div>
        )}
        {filtering && (
          <div style={{ marginBottom: 6 }}>
            <strong>{role?.label}</strong>{' '}
            <span style={{ color: 'var(--text-secondary)' }}>
              reaches {visible.size} of {visible.size + withheld.size} documents
            </span>
          </div>
        )}

        {err && <div style={{ color: 'var(--error)' }}>{err}</div>}

        <table style={{ width: '100%', borderCollapse: 'collapse' }}>
          <tbody>
            {docs.map(d => {
              const state = !filtering ? 'open' : visible.has(d.doc) ? 'visible' : 'withheld';
              return (
                <tr key={d.doc} style={{ opacity: state === 'withheld' ? 0.45 : 1 }}>
                  <td style={{ padding: '2px 4px', whiteSpace: 'nowrap' }}>
                    <span style={{
                      display: 'inline-block', width: 6, height: 6, borderRadius: 3, marginRight: 6,
                      background: state === 'withheld' ? 'var(--error)'
                        : state === 'visible' ? 'var(--success)' : 'var(--text-secondary)',
                    }} />
                    {d.doc}
                  </td>
                  <td style={{ padding: '2px 4px', color: 'var(--text-secondary)' }}>{d.cls}</td>
                  <td style={{ padding: '2px 4px', color: 'var(--text-secondary)' }}>{d.groups.join(', ')}</td>
                  {filtering && (
                    <td style={{ padding: '2px 4px', textAlign: 'right', color: state === 'withheld' ? 'var(--error)' : 'var(--success)' }}>
                      {state === 'withheld' ? 'withheld' : 'readable'}
                    </td>
                  )}
                </tr>
              );
            })}
          </tbody>
        </table>

        {filtering && (
          <div style={{ marginTop: 6, color: 'var(--text-secondary)', lineHeight: 1.5 }}>
            Withheld documents are removed during retrieval, so their text never reaches
            the model and cannot surface through a citation or a paraphrase.
          </div>
        )}
      </Section>

      <Section title="Data custody">
        <div style={{ marginBottom: 4 }}>
          Vault provider: <strong>{VAULT_PROVIDER}</strong>{' '}
          <span style={{ color: 'var(--text-secondary)' }}>
            (detection: Presidio + pattern rules)
          </span>
        </div>
        <div style={{ color: 'var(--text-secondary)', marginBottom: 6 }}>
          {totalTokens} value{totalTokens === 1 ? '' : 's'} tokenised before any model saw the text.
          The graph holds the tokens; the values stay in the vault.
        </div>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4 }}>
          {tokens.map(t => (
            <span key={t.kind} style={{
              padding: '2px 6px', borderRadius: 3, background: 'var(--bg-secondary)',
              border: '1px solid var(--border)', color: 'var(--text-secondary)',
            }}>
              {t.kind} <strong style={{ color: 'var(--text-primary)' }}>{t.count}</strong>
            </span>
          ))}
          {tokens.length === 0 && (
            <span style={{ color: 'var(--text-secondary)' }}>No tokens in the current graph.</span>
          )}
        </div>
      </Section>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div style={{ marginBottom: 14 }}>
      <div style={{
        fontSize: 11, fontWeight: 600, textTransform: 'uppercase', letterSpacing: 0.4,
        color: 'var(--text-secondary)', marginBottom: 6,
        borderBottom: '1px solid var(--border)', paddingBottom: 3,
      }}>
        {title}
      </div>
      {children}
    </div>
  );
}

function lit(v: string | undefined): string {
  if (!v) return '';
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
