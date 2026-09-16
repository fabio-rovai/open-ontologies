"""A pure-Python forward chainer that emits a certificate, and states no verdict.

# Why a second untrusted engine is safe

The house rule is that a fast UNTRUSTED engine proposes and a small VERIFIED
checker disposes. The Rust reasoner is untrusted; the certificate carries the
warrant. A second reasoner written in Python is therefore not a new risk: it is
untrusted in exactly the same way, its conclusions carry exactly the same
certificates, and the same Lean theorem checks them. Nothing about the guarantee
weakens, because the guarantee never depended on the engine.

# How independent this engine is from the Rust one, which is less than you think

Not independent. It runs the SAME semi-naive forward-chaining algorithm over the
SAME rule table, it was written with `src/reason.rs` open, and the comments below
cite that file by line number. So what `tools/horn_differential.py` measures when
the two agree is strong evidence about one thing and weak evidence about another,
and the two must not be swapped:

* STRONG against a TRANSCRIPTION slip. An index off by one, a guard dropped, a
  join in the wrong order, a rule arm reading the subject where it meant the
  object. Two people copying one design do not make the same slip twice, and a
  slip on either side shows up as a different derived set.
* CLOSE TO NOTHING against a SHARED MISREADING. If the design itself misreads a
  W3C rule, both engines implement the misreading, agree perfectly, and go on
  agreeing for ever. No number of corpus documents changes that.

The independent leg of that differential is the LEAN CHECKER: written from the
W3C rules, soundness machine-checked, and the only component of the three whose
acceptance means anything on its own. The clean sweep must never be quoted as
"two independent reasoners agree", and the tool prints that sentence next to its
agreement count on every run so it cannot be quoted without it.

# What this module does not decide

It does not print either verdict word and it never will. `oo-horn` decides which
warrant a run earned by comparing the table it was handed against the built-in
one, and a second place where the two could be confused is exactly the hazard
decision 0003 exists to prevent. What this module reports is the table it used,
a hash of it, and where the certificate is.

# The certificate

Three files land in `certificate_dir`:

* `rules.tsv`, the table the run evaluated. The checker is given this rather
  than the user's file, so what it checks and what ran are the same table.
* `asserted.tsv`, every triple the run started from.
* `horn.tsv`, one line per derived triple:
  `ruleIndex TAB bindCount TAB (var TAB term)* TAB cs TAB cp TAB co TAB (ps TAB pp TAB po)*`.

Steps are written in derivation order, and a step's premises are always facts
known before the round that derived it, so every premise is asserted or
concluded by an EARLIER line. That is the ordering `OOCert.checkHornAll`
requires, and it makes a self-citing step structurally impossible rather than
defended against.

Nothing is materialised into the store. A conclusion drawn under a supplied rule
table holds only in models that satisfy that table, and writing it in beside the
assertions is how run N's output becomes run N+1's axiom.
"""

from __future__ import annotations

import hashlib
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterator, Sequence

import pyoxigraph as ox

from .rules import (
    AtomPat,
    RulePattern,
    RuleTableError,
    builtin_rules,
    builtin_rules_bytes,
    is_var,
    parse_rules,
    rules_tsv,
    vars_of,
)
from .terms import checked_field, spell

INFERRED_GRAPH = "https://open-ontologies.org/graph/inferred"

OWL_INTERSECTION_OF = "<http://www.w3.org/2002/07/owl#intersectionOf>"
OWL_UNION_OF = "<http://www.w3.org/2002/07/owl#unionOf>"
OWL_ONE_OF = "<http://www.w3.org/2002/07/owl#oneOf>"

Fact = tuple[str, str, str]


class AssertedGraphError(ValueError):
    """The asserted graph would have contained triples a reasoner derived."""


@dataclass(frozen=True)
class HornStep:
    """One line of `horn.tsv`, before it is spelled out."""

    rule: int  # 0-based index into rules.tsv; a rule's identity is its index
    binds: tuple[tuple[str, str], ...]  # (BARE variable name, term spelling)
    conclusion: Fact
    premises: tuple[Fact, ...]  # the rule body instantiated, IN DECLARED BODY ORDER


@dataclass
class HornResult:
    """What the run did. No verdict: see the module docstring."""

    certificate_dir: str
    rules_count: int
    rules_source: str
    asserted_graphs: list[str]
    asserted_triples: int
    distinct_asserted_triples: int
    derived_triples: int
    iterations: int
    fixpoint_reached: bool
    skipped_unserialisable: int
    skipped_examples: list[str]
    quoted_triples: int
    by_rule: list[dict]
    sample_derivations: list[str]
    rules_tsv_sha256: str
    not_covered: dict | None = None
    incomplete: str | None = None
    steps: list[HornStep] = field(default_factory=list, repr=False, compare=False)

    def derived(self) -> set[Fact]:
        """The set of triples this run derived. The differential compares SETS."""
        return {st.conclusion for st in self.steps}

    def to_dict(self) -> dict:
        d = Path(self.certificate_dir)
        out: dict = {
            "mode": "horn",
            "engine": "open-ontologies-lite (pure Python)",
            "rules_source": self.rules_source,
            "rules": self.rules_count,
            "asserted_graphs": self.asserted_graphs,
            "asserted_triples": self.asserted_triples,
            "distinct_asserted_triples": self.distinct_asserted_triples,
            "derived_triples": self.derived_triples,
            "iterations": self.iterations,
            "fixpoint_reached": self.fixpoint_reached,
            "skipped_unserialisable": self.skipped_unserialisable,
            "quoted_triples": self.quoted_triples,
            **(
                {
                    "quoted_triples_note": (
                        "an asserted term is not a single N-Triples term, which under "
                        "pyoxigraph 0.5 means an RDF-star quoted triple in object "
                        "position. The certificate is still sound, because the checker "
                        "treats the term as an opaque name, but asserted.tsv is no "
                        "longer re-parsable as N-Triples and no rule can put such a "
                        "term in predicate position"
                    )
                }
                if self.quoted_triples
                else {}
            ),
            "materialized": False,
            "why_not_materialized": (
                "a conclusion drawn under a supplied rule table holds only in models that "
                "satisfy that table, so it is not written into the store beside the "
                "assertions. The certificate is the output of this run"
            ),
            "conditional_on": (
                f"the rules in {self.rules_source}, which this run ASSUMED and never checked. "
                f"A certificate is only as good as the table it cites: a rule saying every "
                f"supplier is compliant produces steps that check green for ever"
            ),
            "sample_derivations": self.sample_derivations,
            "certificate": {
                "dir": self.certificate_dir,
                "format": "oo-horn/1",
                "rules": self.rules_count,
                "rules_tsv_sha256": self.rules_tsv_sha256,
                "asserted": self.asserted_triples,
                "derivations": self.derived_triples,
                "by_rule": self.by_rule,
                "check_with": (
                    f"oo-horn check {d / 'rules.tsv'} {d / 'asserted.tsv'} {d / 'horn.tsv'}"
                ),
                "pronounced_by": (
                    "lean/, through `oo-horn check`, which decides what this run earned by "
                    "comparing the table against the built-in one. This engine emits the "
                    "certificate and states no verdict of its own"
                ),
            },
        }
        if self.skipped_unserialisable:
            out["skipped_examples"] = self.skipped_examples
            out["skipped_reason"] = (
                "the rule head instantiated to a triple no RDF serialiser can write (a "
                "literal in subject position, or a non-IRI in predicate position). Such "
                "conclusions are neither certified nor used as premises, so this run derives "
                "LESS than the table licenses"
            )
        if self.not_covered is not None:
            out["not_covered"] = self.not_covered
        if self.incomplete is not None:
            out["incomplete"] = self.incomplete
        return out


# ---------------------------------------------------------------------------
# Facts
# ---------------------------------------------------------------------------


def _graph_label(graph_name) -> str:
    """`str(DefaultGraph())` is the word DEFAULT, not an IRI, so it is renamed
    here to the same token `graphs=` accepts and nothing downstream has to know
    about pyoxigraph's spelling."""
    return "default" if isinstance(graph_name, ox.DefaultGraph) else str(graph_name)


def _named_graph(name: str):
    """Accept a graph either bare (`http://x/g`) or in its N-Triples spelling
    (`<http://x/g>`), because a user reading `asserted_graphs` out of a previous
    report will paste back the spelled form."""
    if name == "default":
        return ox.DefaultGraph()
    iri = name[1:-1] if name.startswith("<") and name.endswith(">") else name
    return ox.NamedNode(iri)


def _refuse_inferred(labels: Sequence[str], how: str) -> None:
    if any(lbl == f"<{INFERRED_GRAPH}>" for lbl in labels):
        raise AssertedGraphError(
            f"{how} would put <{INFERRED_GRAPH}> into asserted.tsv. That graph holds what a "
            f"reasoner DERIVED, and the soundness theorem is conditional on the assertions: a "
            f"certificate whose asserted set contains a previous run's conclusions certifies "
            f"entailment from a graph nobody asserted, and the checker cannot detect it "
            f"because it is TOLD what the assertions are. Reason over the graph you asserted, "
            f"or copy those triples into a graph of your own naming so the decision is visible"
        )


def _facts(store, graphs) -> tuple[list[Fact], list[str]]:
    """Collect the asserted triples, and say which graphs they came from.

    The default is the DEFAULT GRAPH ALONE, which is where `OntologyEngine.load()`
    puts everything, so this package's own users lose nothing by it. The Rust
    engine flattens every named graph here EXCEPT the inferred one
    (`src/reason.rs:2968`, `graph.triples_outside(&[INFERRED_GRAPH])`). Before
    15 September 2026 it excepted nothing, which was the defect above: a store
    holding a prior materialisation turned derived triples into assertions.
    """
    if graphs == "default":
        quads = list(store.quads_for_pattern(None, None, None, ox.DefaultGraph()))
        labels = ["default"]
    elif graphs == "all":
        quads = list(store)
        labels = sorted({_graph_label(q.graph_name) for q in quads})
        _refuse_inferred(labels, "graphs='all'")
    elif isinstance(graphs, str):
        raise ValueError(
            f"graphs={graphs!r} is neither 'default' nor 'all'. Pass a list to name graphs "
            f"explicitly"
        )
    else:
        wanted = list(graphs)
        if not wanted:
            raise ValueError("graphs=[] selects nothing, so there is no graph to reason over")
        labels = sorted({_graph_label(_named_graph(g)) for g in wanted})
        _refuse_inferred(labels, "naming that graph")
        quads = []
        for g in wanted:
            quads.extend(store.quads_for_pattern(None, None, None, _named_graph(g)))
    facts = [
        (
            spell(q.subject, "subject"),
            spell(q.predicate, "predicate"),
            spell(q.object, "object"),
        )
        for q in quads
    ]
    return facts, labels


def is_atomic_term(term: str) -> bool:
    """True when this spelling is ONE N-Triples term rather than something else.

    An IRI is verified here rather than pattern-guessed: pyoxigraph's `NamedNode`
    refuses a space, a quote and a `>` outright ("Invalid IRI code point"), so an
    IRI that came out of a store is exactly `<`, no space, `>`. Anything else
    beginning with `<` came from somewhere that is not an IRI.

    The case that matters is an RDF-star quoted triple. Measured against
    pyoxigraph 0.5.9 and 0.5.11: a quoted triple is accepted in OBJECT position
    only, `Quad` refuses one in subject or predicate position with a TypeError,
    and its `str()` is `<s> <p> "v"` with RAW SPACES, NOT the `<<( ... )>>`
    spelling. So a `<<`-prefix test alone would check for something these
    versions never produce, and a bare `<`-prefix test would let the
    space-separated spelling through as a predicate.

    Both exclusions are kept, because neither subsumes the other and each is one
    comparison: the space test catches what pyoxigraph 0.5 actually writes, and
    the `<<` test catches the `<<( ... )>>` form other RDF-star tooling uses and
    a future pyoxigraph may adopt.
    """
    if term.startswith("_:") or term.startswith('"'):
        return True
    if term.startswith("<<"):
        return False
    return term.startswith("<") and term.endswith(">") and " " not in term


def _serialisable(s: str, p: str) -> bool:
    """A rule head can instantiate to a literal in subject position or a non-IRI
    in predicate position. Lean will happily certify such a step, because it
    compares strings and has no RDF well-formedness notion, but no serialiser can
    write the triple. Refusing derives LESS, which is the sound direction.

    A non-atomic term in SUBJECT position is allowed through, because dropping an
    assertion is strictly worse than carrying one the checker treats as an opaque
    name, and an opaque name is still sound: `Interp.ι` maps any term string to a
    domain element. The report counts them, because such an `asserted.tsv` is no
    longer re-parsable as N-Triples.
    """
    return not s.startswith('"') and is_atomic_term(p) and p.startswith("<")


# ---------------------------------------------------------------------------
# Matching
# ---------------------------------------------------------------------------


def _join_order(body: Sequence[AtomPat]) -> tuple[int, ...]:
    """Choose the order body atoms are JOINED in.

    THIS IS NOT THE ORDER PREMISES ARE WRITTEN IN. `OOCert.checkHornStep` demands
    `st.premises = r.body.map (inst)`, the DECLARED body order, exactly; a
    certificate with the right triples in the wrong order is rejected. The two
    orders are separate on purpose and the emitter builds premises from
    `rule.body` directly, never from the order they were matched in. Anyone who
    "fixes" the premise list to match the join order will lose every certificate.

    Reordering is sound because the answers to a conjunctive query do not depend
    on join order, and it is worth doing because `rdfs2`, `rdfs3` and `rdfs7` all
    declare `?s ?p ?o` first, an atom with nothing fixed that scans every fact.
    Putting the selective atom first binds `?p` and the scan becomes an index
    lookup. The choice is static, so it is deterministic and the line order of
    `horn.tsv` is stable across runs.
    """
    remaining = list(range(len(body)))
    bound: set[str] = set()
    order: list[int] = []
    while remaining:
        best_i = remaining[0]
        best_score = _selectivity(body[best_i], bound)
        for i in remaining[1:]:
            score = _selectivity(body[i], bound)
            if score > best_score:  # strict: ties keep the lower index
                best_i, best_score = i, score
        order.append(best_i)
        remaining.remove(best_i)
        for f in body[best_i].fields():
            if is_var(f):
                bound.add(f[1:])
    return tuple(order)


def _selectivity(atom: AtomPat, bound: set[str]) -> tuple[int, int]:
    """How constrained this atom is once `bound` is known. Predicate first,
    because the predicate is the only position indexed."""

    def fixed(f: str) -> bool:
        return not is_var(f) or f[1:] in bound

    return (1 if fixed(atom.p) else 0, sum(1 for f in atom.fields() if fixed(f)))


def _match_body(
    body: Sequence[AtomPat],
    order: Sequence[int],
    all_facts: Sequence[Fact],
    by_pred: dict[str, list[Fact]],
) -> Iterator[dict[str, str]]:
    """Backtracking join. Yields the SHARED environment dict, which the caller
    must read before asking for the next solution; every consumer here builds the
    binding tuple and the conclusion from it immediately."""
    env: dict[str, str] = {}

    def rec(k: int) -> Iterator[dict[str, str]]:
        if k == len(order):
            yield env
            return
        atom = body[order[k]]
        key = env.get(atom.p[1:]) if is_var(atom.p) else atom.p
        candidates = all_facts if key is None else by_pred.get(key, ())
        for fact in candidates:
            newly: list[str] = []
            ok = True
            for pat, value in zip(atom.fields(), fact):
                if is_var(pat):
                    name = pat[1:]
                    current = env.get(name)
                    if current is None:
                        env[name] = value
                        newly.append(name)
                    elif current != value:
                        ok = False
                        break
                elif pat != value:
                    ok = False
                    break
            if ok:
                yield from rec(k + 1)
            for name in newly:
                del env[name]

    yield from rec(0)


def _inst(atom: AtomPat, env: dict[str, str]) -> Fact:
    return tuple(env[f[1:]] if is_var(f) else f for f in atom.fields())  # type: ignore[return-value]


# ---------------------------------------------------------------------------
# The run
# ---------------------------------------------------------------------------


def run_horn(
    store,
    rules: Sequence[RulePattern] | None = None,
    *,
    certificate_dir: str | Path,
    graphs: str | Sequence[str] = "default",
    max_iterations: int | None = None,
    rules_source: str | None = None,
) -> HornResult:
    """Derive the closure of a Horn rule table over `store` and write a
    certificate the proved-sound Lean checker in `lean/` verifies.

    `rules=None` means the shipped built-in table, whose bytes are written
    verbatim. That is the only table that can earn the absolute verdict, and only
    the Lean checker decides whether it did.

    `graphs="default"` asserts only the default graph. `"all"` flattens every
    graph and RAISES if the inferred graph is present, which is stricter than the
    Rust engine: that one excludes the inferred graph rather than refusing the
    run.

    `max_iterations=None` runs to a genuine fixpoint. No cap is needed for
    termination: `parse_rules` refuses a head variable that does not occur in the
    body, so no rule can mint a term, the Herbrand base over the terms in the
    graph and the table is finite, and `known` grows monotonically. The cap
    exists as an opt-in memory guard, and a run that hits it reports `incomplete`
    because its derived count is then a lower bound and not the closure.
    """
    certificate_dir = Path(certificate_dir)
    if rules is None:
        table = list(builtin_rules())
        rules_bytes = builtin_rules_bytes()
        source = rules_source or "builtin"
    else:
        table = list(rules)
        if not table:
            raise RuleTableError(
                "the rule table holds no rules. An empty table derives nothing, so there is "
                "no certificate to write"
            )
        text = rules_tsv(table)
        # The checker verifies the steps against the FILE, and the run evaluated
        # the table in memory. If the rendering lost or changed anything the two
        # would be different rule sets, and a step would be checked against a
        # rule nobody ran.
        if parse_rules(text) != table:
            raise RuleTableError(
                "the rendered rule table does not parse back to the table that ran, so the "
                "certificate would be checked against different rules. Refusing to write it"
            )
        rules_bytes = text.encode("utf-8")
        source = rules_source or "supplied"
    if b"\r" in rules_bytes:
        raise RuleTableError(
            "the rule table carries a carriage return. HornParse splits on '\\n' alone, so a "
            "CR becomes part of a term"
        )

    facts, graph_labels = _facts(store, graphs)
    known: set[Fact] = set(facts)

    rule_vars = [tuple(vars_of(r.atoms())) for r in table]
    orders = [_join_order(r.body) for r in table]

    steps: list[HornStep] = []
    # A SET, not a counter. A refused conclusion never enters `known`, so every
    # later round matches the same body and re-derives it; a counter would report
    # attempts and grow with the iteration count, which is not what a reader takes
    # "skipped 2" to mean.
    refused: set[Fact] = set()
    skipped_examples: list[str] = []
    iterations = 0
    fixpoint = False

    while max_iterations is None or iterations < max_iterations:
        iterations += 1
        # Sorted so the candidate order, and therefore the order of the lines in
        # horn.tsv, does not depend on set iteration order. Python's string
        # hashing is randomised per process, so without this two runs over one
        # graph produce different bytes.
        all_facts = sorted(known)
        by_pred: dict[str, list[Fact]] = {}
        for f in all_facts:
            by_pred.setdefault(f[1], []).append(f)
        # by_pred is rebuilt INSIDE the loop, every round. Hoisting it is the
        # exact defect the CHANGELOG records against an earlier engine: "only the
        # rdf:type, rdfs:subClassOf and rdfs:subPropertyOf indices were rebuilt
        # each iteration ... a schema triple the reasoner itself derived was
        # never used".
        round_steps: list[HornStep] = []
        pending: set[Fact] = set()
        for ri, rule in enumerate(table):
            for env in _match_body(rule.body, orders[ri], all_facts, by_pred):
                conclusion = _inst(rule.head, env)
                if conclusion in known or conclusion in pending:
                    continue
                if not _serialisable(conclusion[0], conclusion[1]):
                    if conclusion not in refused:
                        refused.add(conclusion)
                        if len(skipped_examples) < 3:
                            skipped_examples.append(" ".join(conclusion))
                    continue
                pending.add(conclusion)
                round_steps.append(
                    HornStep(
                        rule=ri,
                        binds=tuple((v, env[v]) for v in rule_vars[ri]),
                        conclusion=conclusion,
                        # Declared body order, never the join order.
                        premises=tuple(_inst(a, env) for a in rule.body),
                    )
                )
        if not round_steps:
            # Tests the ROUND list and not the refusal set: a graph that only
            # produces refusals terminates correctly on round two.
            fixpoint = True
            break
        # `known` is mutated ONLY here, at the end of the round. Body matching ran
        # against `all_facts`, the round's starting set, which is what makes every
        # premise of every step asserted-or-concluded on a strictly earlier line.
        # That is `checkHornAll`'s requirement, and it is why a self-citing step
        # is impossible rather than defended against.
        for st in round_steps:
            known.add(st.conclusion)
            steps.append(st)

    _write_certificate(certificate_dir, rules_bytes, facts, steps)

    by_rule_counts = [0] * len(table)
    for st in steps:
        by_rule_counts[st.rule] += 1
    return HornResult(
        certificate_dir=str(certificate_dir),
        rules_count=len(table),
        rules_source=source,
        asserted_graphs=graph_labels,
        asserted_triples=len(facts),
        distinct_asserted_triples=len(set(facts)),
        derived_triples=len(steps),
        iterations=iterations,
        fixpoint_reached=fixpoint,
        skipped_unserialisable=len(refused),
        skipped_examples=skipped_examples,
        # Asserted terms whose spelling is not one N-Triples term: in practice an
        # RDF-star quoted triple, which pyoxigraph 0.5 accepts in object position
        # and spells with raw spaces. Counted rather than dropped, and named in
        # the report, because the certificate stays sound (the checker treats the
        # term as an opaque name) while `asserted.tsv` stops being re-parsable as
        # N-Triples, and a reader is entitled to know which of those they have.
        quoted_triples=sum(1 for f in facts for t in f if not is_atomic_term(t)),
        by_rule=[
            {"index": i, "name": r.name, "derivations": by_rule_counts[i]}
            for i, r in enumerate(table)
        ],
        sample_derivations=[" ".join(st.conclusion) for st in steps[:10]],
        rules_tsv_sha256=hashlib.sha256(rules_bytes).hexdigest(),
        not_covered=_not_covered(facts),
        incomplete=None
        if fixpoint
        else (
            f"the run stopped at the {max_iterations}-iteration cap with rules still firing. "
            f"Every step in the certificate is still a step the checker can verify, but "
            f"derived_triples is a LOWER BOUND on the closure of this table, not the closure"
        ),
        steps=steps,
    )


def _not_covered(facts: Sequence[Fact]) -> dict | None:
    """Say out loud where the Horn family stops.

    Silence here is the failure mode: a user diffs a Python derivation count
    against another engine, sees a gap and has no way to know which gap it is.
    """
    n = sum(
        1 for f in facts if f[1] in (OWL_INTERSECTION_OF, OWL_UNION_OF, OWL_ONE_OF)
    )
    if n == 0:
        return None
    return {
        "rules": ["cls-int1", "cls-int2", "cls-uni", "cls-oo"],
        "constructor_triples_found": n,
        "why": (
            "these four read an RDF list off the graph, so the premise count is the list "
            "length, which is data. A RulePattern has a finite body, so they cannot be "
            "expressed in a Horn table at all. That is the boundary of the Horn family, not "
            "a gap in this engine. Covering them needs the oo-cert/1 format and its "
            "hardcoded arms"
        ),
    }


def _write_certificate(
    certificate_dir: Path,
    rules_bytes: bytes,
    facts: Sequence[Fact],
    steps: Sequence[HornStep],
) -> None:
    """Write the three files.

    All three go out through `write_bytes` and never `open(path, "w")`: universal
    newline translation on Windows turns every LF into CRLF, `HornParse` splits on
    LF alone, and the resulting CR lands inside the last term of every line. The
    failure mode is "the reasoner found nothing", which reads as a data problem.
    """
    certificate_dir.mkdir(parents=True, exist_ok=True)
    (certificate_dir / "rules.tsv").write_bytes(rules_bytes)

    asserted = "".join(
        "\t".join(checked_field(t, "asserted.tsv term") for t in f) + "\n" for f in facts
    )
    # Store iteration order, unsorted and undeduplicated, mirroring the Rust
    # emitter. pyoxigraph's iteration is deterministic index order, so sorting
    # would buy no determinism and would gratuitously diverge from the reference.
    (certificate_dir / "asserted.tsv").write_bytes(asserted.encode("utf-8"))

    lines = []
    for st in steps:
        fields = [str(st.rule), str(len(st.binds))]
        for var, term in st.binds:
            # BARE, with no '?'. rules.tsv spells `?x`; horn.tsv spells `x`,
            # because `HornParse.patOf` strips the '?' when it parses the rule and
            # `substOf` looks the bare name up. A '?' here is rejected with exit 1
            # and a rejection JSON that names nothing at all.
            fields.append(checked_field(var, "horn.tsv binding variable"))
            fields.append(checked_field(term, "horn.tsv binding term"))
        for triple in (st.conclusion, *st.premises):
            fields.extend(checked_field(t, "horn.tsv triple term") for t in triple)
        lines.append("\t".join(fields))
    (certificate_dir / "horn.tsv").write_bytes(
        "".join(line + "\n" for line in lines).encode("utf-8")
    )
