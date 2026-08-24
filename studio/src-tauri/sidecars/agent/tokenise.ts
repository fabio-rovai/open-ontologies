import { createHmac } from 'node:crypto';

/**
 * Tokenisation at the chat entrance.
 *
 * Ingestion tokenises documents before any model sees them; a question typed
 * into the chat went to the model untouched, which is a leak path the moment
 * the provider is remote. The same detection patterns and the same keyed
 * token scheme as the pipeline (TOK_{KIND}_{hmac[:12]}) run here, so a value
 * mentioned in a question maps to the SAME token the ingested documents use:
 * the model can still join "ask about Dr Jane Smith" to the graph node,
 * without ever seeing the name.
 */

const KEY = process.env.ONTO_VAULT_KEY ?? 'demo-key-not-for-production';

const PATTERNS: Array<[string, RegExp]> = [
  ['EMAIL', /\b[\w.+-]+@[\w-]+\.[\w.]{2,}\b/g],
  ['PHONE', /\b(?:\+\d{1,3}[ -]?)?(?:\(?\d{3,5}\)?[ -]?){2,3}\d{3,4}\b/g],
  ['PERSON', /\b(?:Dr|Prof|Mr|Mrs|Ms)\.? [A-Z][a-z]+ [A-Z][a-z]+\b/g],
  ['ID', /\b[A-Z]{2,4}-\d{4,}\b/g],
];

function token(kind: string, value: string): string {
  const digest = createHmac('sha256', KEY).update(value).digest('hex').slice(0, 12);
  return `TOK_${kind}_${digest}`;
}

export function tokeniseInbound(text: string): { text: string; count: number } {
  let out = text;
  let count = 0;
  for (const [kind, pattern] of PATTERNS) {
    out = out.replace(pattern, (m) => {
      count += 1;
      return token(kind, m);
    });
  }
  return { text: out, count };
}
