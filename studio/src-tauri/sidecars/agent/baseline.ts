/**
 * Plain baseline: chunk retrieval over raw corpus text, no ontology at all.
 *
 * graphrag.ts answers a question by traversing the derived knowledge graph
 * from the entities the question names. This module answers the SAME
 * question a different way: split every document into paragraph-sized
 * chunks, score each chunk by how many question terms it contains, and hand
 * the top few to the model with no structure at all. It is the "just embed
 * the documents and retrieve" approach the demonstration compares against.
 *
 * This is a TypeScript port of demo/build_compare.py's `baseline_answer` /
 * `load_chunks`, kept in the sidecar so a live session can run the same
 * comparison the offline pipeline precomputed. It deliberately does not
 * decide whether the baseline or the grounded answer is "right": that
 * judgment is a human read over both transcripts (see build_compare.py's
 * module docstring), and nothing here manufactures it. Callers that want the
 * comparison surface, not just this one side of it, combine this module's
 * output with graphrag.ts's own, and leave the verdict to a person.
 */

const STOP = new Set(
  ('the a an of to in for is are be and or that this with by on at as it its what which who whom ' +
    'why how when where does do did can could would should may might will shall about any all some ' +
    'there their they them we our you your i me my show tell give list find explain').split(' '),
);

/** Terms worth scoring on: long enough to be discriminating, not stopwords. */
export function terms(question: string): string[] {
  const words = question.toLowerCase().match(/[a-z0-9][a-z0-9\-_]{2,}/g) ?? [];
  const uniq = [...new Set(words.filter(w => !STOP.has(w) && w.length >= 4))];
  return uniq.slice(0, 8);
}

export interface DocChunk {
  doc: string;
  idx: number;
  text: string;
}

/**
 * Split one document's raw text into chunks with no notion of section or
 * ontology. Prose is split on blank lines; anything without enough blank
 * lines to paragraph on (JSON, Turtle) falls back to fixed-size line blocks.
 * This is deliberately cruder than chunker.py's semantic chunking: the point
 * of the baseline is to be what a generic "split and embed" pipeline does,
 * not the best possible retrieval this codebase can build.
 */
export function chunkDocument(docId: string, text: string): DocChunk[] {
  let paragraphs = text
    .split(/\n\s*\n/)
    .map(p => p.trim())
    .filter(Boolean);
  if (paragraphs.length < 3) {
    const lines = text.split('\n').filter(l => l.trim());
    paragraphs = [];
    for (let i = 0; i < lines.length; i += 25) {
      paragraphs.push(lines.slice(i, i + 25).join('\n'));
    }
  }
  return paragraphs.map((p, idx) => ({ doc: docId, idx, text: p }));
}

/** Chunk every document in a corpus keyed by document id. */
export function chunkCorpus(docs: Record<string, string>): DocChunk[] {
  const out: DocChunk[] = [];
  for (const [docId, text] of Object.entries(docs)) {
    out.push(...chunkDocument(docId, text));
  }
  return out;
}

/**
 * Score every chunk by raw term-occurrence count and return the top `k`.
 * Chunks that score zero are dropped entirely rather than padding the
 * context with irrelevant text, mirroring the Python baseline exactly.
 */
export function topChunks(chunks: DocChunk[], question: string, k = 6): DocChunk[] {
  const qterms = terms(question);
  const scored: Array<{ score: number; chunk: DocChunk }> = [];
  for (const chunk of chunks) {
    const low = chunk.text.toLowerCase();
    let score = 0;
    for (const t of qterms) {
      let from = 0;
      for (;;) {
        const at = low.indexOf(t, from);
        if (at === -1) break;
        score += 1;
        from = at + t.length;
      }
    }
    if (score > 0) scored.push({ score, chunk });
  }
  scored.sort((a, b) => b.score - a.score);
  return scored.slice(0, k).map(s => s.chunk);
}

/** Render retrieved chunks into a labelled context block for the prompt. */
export function renderChunks(chunks: DocChunk[]): string {
  return chunks.map(c => `[${c.doc}#${c.idx}]\n${c.text.slice(0, 1200)}`).join('\n\n');
}

/**
 * The documents an answer actually cited: scanned against every corpus
 * document id, not only the ones retrieval happened to surface. A model
 * naming a document it saw in its context gets credit for that regardless of
 * which fixed list built the context; falling back to `retrieved` only fires
 * when the answer names none of them at all (a prompt-following failure, not
 * evidence of zero grounding). Mirrors build_compare.py's `cited_docs`.
 */
export function citedDocs(answer: string, allDocIds: string[], retrieved: string[]): string[] {
  const named = allDocIds.filter(d => answer.includes(d)).sort();
  return named.length > 0 ? named : [...new Set(retrieved)].sort();
}

export interface BaselineResult {
  answer: string;
  citations: string[];
}

const BASELINE_PROMPT = (context: string, question: string) =>
  'You are answering a question using ONLY the document excerpts below, retrieved by plain ' +
  'keyword matching (no ontology, no knowledge graph). Each excerpt is tagged with the document ' +
  `id it came from.\n\nEXCERPTS:\n${context || '(no excerpt matched)'}\n\nQUESTION: ${question}\n\n` +
  'Answer in 2-5 sentences. Every factual claim you make must be traceable to the excerpts above; ' +
  'when you use one, name the document id in parentheses. If the excerpts do not contain enough ' +
  'to answer, say so plainly rather than guessing.';

/**
 * Answer a question from plain keyword-chunk retrieval, no ontology.
 *
 * `answerOnce` is injected rather than called on a module-level provider so
 * this stays testable with a fake, the same way the rest of this sidecar's
 * modules take a McpClient or Provider as a parameter instead of reaching
 * for a global.
 */
export async function baselineAnswer(
  question: string,
  chunks: DocChunk[],
  allDocIds: string[],
  answerOnce: (prompt: string) => Promise<string>,
  k = 6,
): Promise<BaselineResult> {
  const top = topChunks(chunks, question, k);
  const retrievedDocs = [...new Set(top.map(c => c.doc))].sort();
  const answer = (await answerOnce(BASELINE_PROMPT(renderChunks(top), question))).trim();
  return { answer, citations: citedDocs(answer, allDocIds, retrievedDocs) };
}
