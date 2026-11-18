import { describe, expect, it } from 'vitest';
import { parseRows } from '../sparql-rows';
import {
  buildSchema, expandInstGraph, explodeGroup, instElements, schemaElements, seedInstGraph, type NodeInfo,
} from '../graph-model';

const EX = 'http://example.org/o#';
const iri = (l: string) => `<${EX}${l}>`;
// The engine's /api/query shape: results is an array of rows of N-Triples terms; an unbound
// OPTIONAL variable is absent from its row ('' below).
const rows = (vars: string[], data: string[][]) =>
  parseRows(JSON.stringify({
    variables: vars,
    results: data.map((r) => Object.fromEntries(vars.map((v, i) => [v, r[i]]).filter(([, x]) => x !== ''))),
  }));

const OWL = 'http://www.w3.org/2002/07/owl#';

function schema() {
  const classes = rows(['c', 'label'], [
    [iri('Asset'), '"Asset"@en'], [iri('Sensor'), '"Sensor"@en'], [iri('Sensor'), '"传感器"@zh'],
    [iri('TempSensor'), '"Temperature sensor"@en'], [iri('Site'), '"Site"@en'], [iri('Reading'), '"Reading"@en'],
    [`<${OWL}Thing>`, '"Thing"'],
  ]);
  // A reasoned store: rdfs:subClassOf is transitively closed.
  const sub = rows(['s', 'o'], [
    [iri('TempSensor'), iri('Sensor')], [iri('TempSensor'), iri('Asset')], [iri('Sensor'), iri('Asset')], [iri('Site'), iri('Asset')],
  ]);
  const props = rows(['p', 't', 'label', 'code'], [
    [iri('atSite'), `<${OWL}ObjectProperty>`, '"at site"@en', ''],
    [iri('ofSensor'), `<${OWL}ObjectProperty>`, '"of sensor"@en', '"SENSOR_ID"'],
    [iri('temperature'), `<${OWL}DatatypeProperty>`, '"temperature"@en', '"TEMP_C"'],
  ]);
  // ...and every superclass of each domain and range.
  const dr = rows(['p', 'k', 'c'], [
    [iri('atSite'), '"d"', iri('Sensor')], [iri('atSite'), '"d"', iri('Asset')], [iri('atSite'), '"r"', iri('Site')],
    [iri('ofSensor'), '"d"', iri('Reading')], [iri('ofSensor'), '"r"', iri('Sensor')], [iri('ofSensor'), '"r"', iri('Asset')],
    [iri('temperature'), '"d"', iri('Reading')],
  ]);
  return buildSchema(classes, sub, props, dr);
}

describe('buildSchema', () => {
  it('reduces a closed subclass relation to direct parents and finds roots', () => {
    const s = schema();
    expect(s.classes.get(EX + 'TempSensor')!.parents).toEqual([EX + 'Sensor']);
    expect(s.classes.get(EX + 'TempSensor')!.root).toBe(EX + 'Asset');
    expect(s.classes.has(OWL + 'Thing')).toBe(false);
  });

  it('keeps only the most specific domains and ranges', () => {
    const s = schema();
    expect(s.props.get(EX + 'atSite')!.domains).toEqual([EX + 'Sensor']);
    expect(s.props.get(EX + 'ofSensor')!.ranges).toEqual([EX + 'Sensor']);
  });

  it('reads labels with their language and field codes from skos:notation', () => {
    const s = schema();
    expect(s.langs).toEqual(['en', 'zh']);
    expect(s.props.get(EX + 'temperature')!.codes).toEqual(['TEMP_C']);
  });
});

describe('schemaElements', () => {
  it('draws subclass edges parent to child and object properties domain to range', () => {
    const els = schemaElements(schema(), { lang: 'zh', expanded: new Set(), showFields: false, focus: null });
    const sub = els.find((e) => e.data.id === `sub|${EX}TempSensor|${EX}Sensor`)!;
    expect([sub.data.source, sub.data.target]).toEqual([EX + 'Sensor', EX + 'TempSensor']);
    const prop = els.find((e) => e.data.kind === 'prop' && e.data.uri === EX + 'ofSensor')!;
    expect([prop.data.source, prop.data.target]).toEqual([EX + 'Reading', EX + 'Sensor']);
    expect(els.find((e) => e.data.id === EX + 'Sensor')!.data.name).toBe('传感器');
  });

  it('lists data properties with their codes on an opened card', () => {
    const els = schemaElements(schema(), { lang: 'en', expanded: new Set([EX + 'Reading']), showFields: false, focus: null });
    expect(String(els.find((e) => e.data.id === EX + 'Reading')!.data.label)).toContain('temperature   TEMP_C');
  });

  it('focus keeps the class, its parents, children and linked classes', () => {
    const els = schemaElements(schema(), { lang: 'en', expanded: new Set(), showFields: false, focus: EX + 'Sensor' });
    const ids = els.filter((e) => e.group === 'nodes').map((e) => e.data.id).sort();
    expect(ids).toEqual([EX + 'TempSensor', EX + 'Reading', EX + 'Site', EX + 'Asset', EX + 'Sensor'].sort());
  });
});

describe('instance neighbourhood', () => {
  it('groups many neighbours of one predicate and type, and explodes the group on demand', () => {
    const s = schema();
    const sensor = EX + 'T1';
    const info = new Map<string, NodeInfo>([[sensor, { labels: [{ value: 'T1' }], types: [EX + 'TempSensor', EX + 'Sensor', EX + 'Asset'] }]]);
    const recs = Array.from({ length: 5 }, (_, i) => EX + `R${i}`);
    for (const r of recs) info.set(r, { labels: [], types: [EX + 'Reading'] });
    info.set(EX + 'L1', { labels: [{ value: 'L1' }], types: [EX + 'Site', EX + 'Asset'] });
    const nbrs = [
      ...recs.map((r) => ({ other: r, pred: EX + 'ofSensor', dir: 'in' as const })),
      { other: EX + 'L1', pred: EX + 'atSite', dir: 'out' as const },
      { other: EX + 'Sensor', pred: 'http://www.w3.org/2000/01/rdf-schema#seeAlso', dir: 'out' as const }, // a class: skipped
    ];
    const g = expandInstGraph(seedInstGraph(sensor, info.get(sensor), s), sensor, nbrs, info, s);
    expect(g.nodes.get(sensor)!.type).toBe(EX + 'TempSensor');
    expect(g.nodes.has(EX + 'L1')).toBe(true);
    expect(g.nodes.has(EX + 'Sensor')).toBe(false);
    expect([...g.groups.values()].map((x) => x.members.length)).toEqual([5]);
    const els = instElements(g, s, 'en');
    expect(els.some((e) => e.data.kind === 'group' && e.data.label === 'Reading ×5')).toBe(true);

    const g2 = explodeGroup(g, [...g.groups.keys()][0], info);
    expect(g2.groups.size).toBe(0);
    expect(recs.every((r) => g2.nodes.has(r))).toBe(true);
  });
});
