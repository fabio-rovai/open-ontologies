import type { DemoSource } from './demo-source'
import { createReplaySource, type ReplayFixtures } from './replay-source'
import { liveSource } from './live-source'

// The single construction site: everything downstream consumes DemoSource
// and never mcp-client or the precomputed fixtures directly.
export function chooseSourceKind(env: Record<string, string | undefined>): 'live' | 'replay' {
  return env.VITE_DEMO_MODE === 'replay' ? 'replay' : 'live'
}

export async function getDemoSource(): Promise<DemoSource> {
  if (chooseSourceKind(import.meta.env as unknown as Record<string, string | undefined>) === 'replay') {
    const response = await fetch('./precomputed/bundle.json')
    if (!response.ok) {
      throw new Error(`Could not load the precomputed demonstration: ${response.status}`)
    }
    const fixtures = (await response.json()) as ReplayFixtures
    return createReplaySource(fixtures)
  }
  return liveSource()
}
