/**
 * Parse the engine's `/api/query` payload into rows.
 *
 * The engine does not return W3C SPARQL JSON. It returns
 * `{"variables":[...],"results":[{"x":"<iri>"}, ...]}` with every value an
 * N-Triples term. W3C-shaped values (`{type, value, xml:lang}`) are accepted
 * too, so the parser also works against a standard endpoint.
 */
export interface Term {
  type: 'uri' | 'literal' | 'bnode';
  value: string;
  lang?: string;
  datatype?: string;
}

export type Row = Record<string, Term>;

const LITERAL = /^"((?:[^"\\]|\\.)*)"(?:@([\w-]+)|\^\^<([^>]*)>)?$/s;

export function parseTerm(raw: unknown): Term {
  if (raw && typeof raw === 'object' && 'value' in raw) {
    const v = raw as Record<string, string>;
    const type = v.type === 'uri' ? 'uri' : v.type === 'bnode' ? 'bnode' : 'literal';
    return { type, value: v.value ?? '', lang: v['xml:lang'] || undefined, datatype: v.datatype };
  }
  const s = String(raw);
  if (s.startsWith('<') && s.endsWith('>')) return { type: 'uri', value: s.slice(1, -1) };
  if (s.startsWith('_:')) return { type: 'bnode', value: s.slice(2) };
  const m = s.match(LITERAL);
  if (!m) return { type: 'literal', value: s };
  const value = m[1].replace(/\\(["\\])/g, '$1').replace(/\\n/g, '\n').replace(/\\t/g, '\t');
  return { type: 'literal', value, lang: m[2] || undefined, datatype: m[3] || undefined };
}

export function parseRows(text: string): Row[] {
  let parsed: unknown;
  try { parsed = JSON.parse(text); } catch { return []; }
  const p = parsed as { results?: unknown };
  const list = Array.isArray(p?.results)
    ? p.results
    : ((p?.results as { bindings?: unknown[] } | undefined)?.bindings ?? []);
  return (list as Record<string, unknown>[]).map((r) => {
    const row: Row = {};
    for (const [k, v] of Object.entries(r)) row[k] = parseTerm(v);
    return row;
  });
}

export function localName(uri: string): string {
  const i = Math.max(uri.lastIndexOf('#'), uri.lastIndexOf('/'));
  return i >= 0 ? uri.slice(i + 1) : uri;
}
