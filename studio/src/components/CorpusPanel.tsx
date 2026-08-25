import type { Document } from '../lib/demo-source'

// The precomputed corpus documents carry more than demo-source.ts's Document
// type declares (file, source_url, sha256, retrieved, from
// demo/corpus/dcat-us/MANIFEST.json), and not every source populates
// `title`. Read the extra fields defensively rather than widening the shared
// type, since nothing else in the app needs them.
interface ManifestFields {
  file?: string
  source_url?: string
  sha256?: string
  retrieved?: string
}

export interface CorpusPanelProps {
  documents: Document[]
  // Set when the source has no honest way to list documents (live mode
  // today: no tool or Tauri command returns document text). Rendered as an
  // explanation rather than left to read as "this corpus is empty".
  error?: string | null
  onOpen: (id: string) => void
}

function provenanceTitle(doc: Document & ManifestFields): string {
  const parts: string[] = []
  if (doc.source_url) parts.push(`source: ${doc.source_url}`)
  if (doc.retrieved) parts.push(`retrieved: ${doc.retrieved}`)
  if (doc.sha256) parts.push(`sha256: ${doc.sha256}`)
  return parts.length > 0 ? parts.join('\n') : `${doc.id} (no provenance recorded)`
}

/**
 * One row per corpus document: id, title, and the MANIFEST.json provenance
 * (source URL, retrieval date, checksum) surfaced as a hover tooltip rather
 * than cluttering the row. Takes data and a callback only, and imports no
 * source.
 */
export function CorpusPanel({ documents, error, onOpen }: CorpusPanelProps) {
  if (error) {
    return (
      <p className="p-4 text-sm" style={{ color: 'var(--text-secondary)' }}>
        {error}
      </p>
    )
  }
  if (documents.length === 0) {
    return (
      <p className="p-4 text-sm" style={{ color: 'var(--text-secondary)' }}>
        No documents in this corpus.
      </p>
    )
  }
  return (
    <ul className="divide-y overflow-y-auto" style={{ borderColor: 'var(--border)' }}>
      {documents.map((doc) => {
        const d = doc as Document & ManifestFields
        const title = d.title || d.file || d.id
        return (
          <li
            key={d.id}
            onClick={() => onOpen(d.id)}
            title={provenanceTitle(d)}
            className="cursor-pointer px-3 py-2 text-sm"
            style={{ color: 'var(--text-primary)' }}
          >
            <span className="font-mono text-xs mr-2" style={{ color: 'var(--text-secondary)' }}>
              {d.id}
            </span>
            {title}
          </li>
        )
      })}
    </ul>
  )
}
