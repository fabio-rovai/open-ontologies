import { describe, it, expect } from 'vitest'
import { createElement } from 'react'
import { renderToStaticMarkup } from 'react-dom/server'
import { FindingsPanel, type FindingsPanelProps } from '../FindingsPanel'

function render(props: Partial<FindingsPanelProps>): string {
  return renderToStaticMarkup(
    createElement(FindingsPanel, {
      findings: [],
      error: null,
      selected: null,
      onSelect: () => {},
      ...props,
    }),
  )
}

// A findings run that could not decide (conforms: null) or could not run
// arrives as an error, and must never read as a clean pass (#422).
describe('FindingsPanel', () => {
  it('shows why there are no findings instead of "No findings" when the run failed', () => {
    const html = render({
      error: 'SHACL validation is undetermined, not clean: conforms is null with 0 focus node(s).',
    })
    expect(html).toContain('undetermined, not clean')
    expect(html).not.toContain('No findings')
  })

  it('still says "No findings" when the run succeeded and found nothing', () => {
    expect(render({})).toContain('No findings in this corpus.')
  })

  it('lists findings when there are some', () => {
    const html = render({
      findings: [
        { id: 'f1', subject: 'ex:s', kind: 'conformance', claims: [{ document: 'doc-1', predicate: 'p', object: 'o' }] },
      ],
    })
    expect(html).toContain('ex:s')
    expect(html).toContain('doc-1')
  })
})
