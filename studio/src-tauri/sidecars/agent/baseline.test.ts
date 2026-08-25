import { describe, it, expect } from 'vitest';
import { terms, chunkDocument, chunkCorpus, topChunks, citedDocs, baselineAnswer } from './baseline.js';

describe('terms', () => {
  it('drops stopwords and short words, keeps discriminating ones', () => {
    expect(terms('Does this profile conform to the W3C DCAT vocabulary standard?')).toEqual(
      expect.arrayContaining(['profile', 'conform', 'vocabulary', 'standard']),
    );
    expect(terms('Does this profile conform to the W3C DCAT vocabulary standard?')).not.toContain('this');
  });
});

describe('chunkDocument', () => {
  it('splits prose on blank lines', () => {
    const chunks = chunkDocument('doc-a', 'first paragraph here\n\nsecond paragraph here\n\nthird one');
    expect(chunks).toHaveLength(3);
    expect(chunks[0]).toMatchObject({ doc: 'doc-a', idx: 0, text: 'first paragraph here' });
  });

  it('falls back to fixed-size line blocks for text with no paragraph breaks', () => {
    const lines = Array.from({ length: 60 }, (_, i) => `line ${i}`).join('\n');
    const chunks = chunkDocument('doc-b', lines);
    expect(chunks.length).toBe(3); // 60 lines / 25 per block, rounded up
    expect(chunks.every(c => c.doc === 'doc-b')).toBe(true);
  });
});

describe('chunkCorpus', () => {
  it('chunks every document and tags each chunk with its own doc id', () => {
    const chunks = chunkCorpus({
      'doc-a': 'alpha\n\nbeta\n\ngamma',
      'doc-b': 'delta\n\nepsilon\n\nzeta',
    });
    expect(chunks.map(c => c.doc).sort()).toEqual(['doc-a', 'doc-a', 'doc-a', 'doc-b', 'doc-b', 'doc-b']);
  });
});

describe('topChunks', () => {
  const chunks = [
    { doc: 'a', idx: 0, text: 'DCAT-US 3.0 is a profile of the DCAT vocabulary.' },
    { doc: 'b', idx: 0, text: 'This document describes an unrelated catalog schema.' },
    { doc: 'c', idx: 0, text: 'DCAT vocabulary terms appear here, DCAT again, and DCAT once more.' },
  ];

  it('ranks chunks by keyword occurrence, highest first', () => {
    const top = topChunks(chunks, 'what does the DCAT vocabulary cover?', 3);
    expect(top[0].doc).toBe('c'); // most occurrences of "vocabulary"/"dcat"
  });

  it('drops chunks that score zero rather than padding the context', () => {
    const top = topChunks(chunks, 'xyzzy plugh', 3);
    expect(top).toHaveLength(0);
  });

  it('respects the k limit', () => {
    const top = topChunks(chunks, 'DCAT vocabulary profile document catalog', 1);
    expect(top).toHaveLength(1);
  });
});

describe('citedDocs', () => {
  it('prefers documents actually named in the answer over the retrieved list', () => {
    const cited = citedDocs('The profile is defined in profile-readme.', ['profile-readme', 'other-doc'], [
      'other-doc',
    ]);
    expect(cited).toEqual(['profile-readme']);
  });

  it('falls back to the retrieved list when the answer names nothing', () => {
    const cited = citedDocs('No document is named here.', ['profile-readme', 'other-doc'], ['other-doc']);
    expect(cited).toEqual(['other-doc']);
  });
});

describe('baselineAnswer', () => {
  it('retrieves, prompts with only the top chunks, and cites what the answer names', async () => {
    const chunks = [
      { doc: 'profile-readme', idx: 0, text: 'DCAT-US is an implementation of the W3C DCAT standard.' },
      { doc: 'unrelated', idx: 0, text: 'Nothing to do with the question at all.' },
    ];
    let promptSeen = '';
    const answerOnce = async (prompt: string) => {
      promptSeen = prompt;
      return 'Yes, per profile-readme it claims DCAT conformance.';
    };

    const result = await baselineAnswer(
      'does the profile conform to DCAT?',
      chunks,
      ['profile-readme', 'unrelated'],
      answerOnce,
    );

    expect(promptSeen).toContain('profile-readme');
    expect(promptSeen).not.toContain('unrelated');
    expect(result.citations).toEqual(['profile-readme']);
    expect(result.answer).toContain('DCAT conformance');
  });
});
