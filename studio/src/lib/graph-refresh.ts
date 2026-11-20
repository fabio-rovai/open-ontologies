/**
 * Who is told when the ontology in the engine changes (#421).
 *
 * Chat mutations (useChat.ts), the engine connecting (useEngine.ts) and the
 * Property Inspector change the store outside any React state, and more than
 * one view needs to hear about it: TreeView reloads its tree while it is
 * mounted, and LiveChrome refreshes the demo-store graph Graph3D draws. They
 * used to share one `window.__refreshGraph` slot, so TreeView's unmount
 * deleted LiveChrome's wrapper along with its own function and every later
 * change went unheard.
 *
 * Each subscriber now holds its own registration and removes only that one.
 * Every call to `onGraphChanged` is a separate registration even for the same
 * function, so two views sharing a callback cannot unsubscribe each other.
 */
export type GraphChangedListener = () => void | Promise<void>

const registrations = new Set<{ listener: GraphChangedListener }>()

/** Registers `listener` and returns the function that removes this registration. */
export function onGraphChanged(listener: GraphChangedListener): () => void {
  const registration = { listener }
  registrations.add(registration)
  return () => {
    registrations.delete(registration)
  }
}

/**
 * Tells every current subscriber that the graph changed. A subscriber that
 * throws or rejects is reported on the console and does not stop the rest.
 */
export function notifyGraphChanged(): void {
  // A copy, so a listener that unsubscribes while being called cannot skip another.
  for (const { listener } of [...registrations]) {
    try {
      const result = listener()
      if (result && typeof result.catch === 'function') {
        result.catch((e: unknown) => console.error('Graph refresh failed:', e))
      }
    } catch (e) {
      console.error('Graph refresh failed:', e)
    }
  }
}
