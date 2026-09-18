// TCB-20 without a length bound: the rule-table round trip.
//
// `docs/trusted-computing-base.md` states TCB-20 as `parse_rules(rules_tsv(r)) == r`,
// "in particular a constant never renders to something that reads back as a variable,
// and a variable name never renders to something that reads back as a constant".
// Two things verify part of that today and neither verifies this statement:
//
// * `kani_harnesses::pat_of_and_render_are_inverse_at_2` and `_at_3` prove the
//   PER-FIELD half over every byte pattern at a field length of exactly two and
//   exactly three bytes. The length is fixed because a symbolic length made CBMC
//   reach fifteen gigabytes without a verdict, and the whole-line statement is out
//   of reach for a further reason recorded on that page: asserting over
//   `body.split('\t')` put CBMC inside `CharSearcher`.
// * Aeneas translates `src/boundary_core.rs` and `aeneas/lean/` proves things about
//   it for every input. It does not reach this function. `docs/aeneas-boundary.md`
//   says why under "What Aeneas could not handle": `Pat` carries a `String`, and
//   Aeneas "has a `String` model but not enough of one". Every other serialisation
//   property on the TCB page lost its length bound that way. This one did not.
//
// So this file models the grammar over bytes and proves the whole statement for
// every input: every field length, every field count, every rule table.
//
// WHAT IS MODELLED AND WHAT IS ASSUMED is the only thing that makes the result
// mean anything, and it is stated in
// `docs/decisions/0013-a-verifier-that-cannot-read-the-code-verifies-a-rewrite.md`
// rather than here.
// The short version: this is a REIMPLEMENTATION, so a theorem here is a theorem
// about this file. It is not a theorem about `src/reason.rs`. The decision record
// measures exactly what that is worth.

module RuleTable {

  // An octet. `parse_rules` takes a `&str` and `rules_tsv` returns a `String`, but
  // every discrimination either of them makes is on an ASCII byte: `strip_prefix('?')`,
  // `starts_with('<')`, `starts_with("_:")`, `starts_with('"')`, `split('\t')`,
  // `split('\n')` and `is_ascii_digit`. None of those can straddle a UTF-8 code
  // point, so the byte model loses nothing the function can see. This is the same
  // concession `src/boundary_core.rs` made for Aeneas, and it is a concession: a
  // theorem here is about bytes, and the Rust is about a `&str` whose well-formedness
  // is guaranteed elsewhere.
  newtype byte = x: int | 0 <= x < 256

  const TAB: byte := 9
  const LF: byte := 10
  const CR: byte := 13
  const QUOTE: byte := 34    // "
  const QUESTION: byte := 63 // ?
  const LT: byte := 60       // <
  const COLON: byte := 58    // :
  const UNDERSCORE: byte := 95
  const ZERO: byte := 48
  const NINE: byte := 57

  datatype Option<T> = None | Some(value: T)

  // `Pat` in `src/reason.rs`, and `OOCert.Pat` on the Lean side.
  datatype Pat = Const(bytes: seq<byte>) | Var(name: seq<byte>)

  datatype AtomPat = AtomPat(s: Pat, p: Pat, o: Pat)

  datatype RulePattern = RulePattern(name: seq<byte>, body: seq<AtomPat>, head: AtomPat)

  // ---------------------------------------------------------------------------
  // The field grammar
  // ---------------------------------------------------------------------------

  // The two classifications the Kani harness computes by hand, so that the lemma
  // below is the same statement rather than a paraphrase of it.
  predicate IsNamedVar(f: seq<byte>) {
    |f| > 1 && f[0] == QUESTION
  }

  predicate IsNtTerm(f: seq<byte>) {
    |f| > 0 && (f[0] == LT || f[0] == QUOTE
                || (f[0] == UNDERSCORE && |f| > 1 && f[1] == COLON))
  }

  // `pat_of` in `src/reason.rs`, byte for byte:
  //
  //   if let Some(name) = field.strip_prefix('?') {
  //       if name.is_empty() { return None; }
  //       return Some(Pat::Var(name.to_string()));
  //   }
  //   if field.starts_with('<') || field.starts_with("_:") || field.starts_with('"') {
  //       return Some(Pat::Const(field.to_string()));
  //   }
  //   None
  //
  // The leading `?` arm does not fall through on an empty name, and neither does
  // this. That matters: a fall-through would make `"?"` a constant.
  function PatOf(f: seq<byte>): Option<Pat> {
    if |f| > 0 && f[0] == QUESTION then
      (if |f| == 1 then None else Some(Var(f[1..])))
    else if IsNtTerm(f) then
      Some(Const(f))
    else
      None
  }

  // `Pat::render`.
  function Render(p: Pat): seq<byte> {
    match p
    case Const(c) => c
    case Var(v) => [QUESTION] + v
  }

  // The image of `PatOf`. A `Pat` the parser can return, as opposed to a `Pat` the
  // Rust type permits. The distinction is the whole content of this file: `Pat` is
  // an enum over two `String`s and nothing in the type stops either of them holding
  // a tab or a leading `?`.
  predicate WellFormedPat(p: Pat) {
    match p
    case Var(v) => |v| > 0
    case Const(c) => IsNtTerm(c)
  }

  // The Kani harness's statement, for every length rather than at two and three.
  //
  //   match pat_of(field) {
  //       Some(p) => { assert!(named_var || nt_term); assert!(p.render() == field); }
  //       None => assert!(!named_var && !nt_term),
  //   }
  lemma PatOfClassifies(f: seq<byte>)
    ensures PatOf(f).Some? <==> (IsNamedVar(f) || IsNtTerm(f))
    ensures PatOf(f).Some? ==> Render(PatOf(f).value) == f
    ensures PatOf(f).Some? ==> WellFormedPat(PatOf(f).value)
  {
    if |f| > 0 && f[0] == QUESTION && |f| > 1 {
      assert [QUESTION] + f[1..] == f;
    }
  }

  // The other direction, which the Kani harness does not state at all: rendering a
  // well-formed `Pat` and parsing it back is the identity.
  lemma RenderRoundTripsPat(p: Pat)
    requires WellFormedPat(p)
    ensures PatOf(Render(p)) == Some(p)
  {
    match p
    case Var(v) =>
      var r := [QUESTION] + v;
      assert |r| > 1 && r[0] == QUESTION;
      assert r[1..] == v;
    case Const(c) =>
      assert c[0] != QUESTION;
  }

  // TCB-5's shape at the field level: an accepted variable and an accepted constant
  // are never the same bytes, so the two kinds cannot be confused by a reader that
  // compares fields as opaque strings.
  lemma VariablesAndConstantsDoNotCollide(f: seq<byte>, g: seq<byte>)
    requires PatOf(f).Some? && PatOf(g).Some?
    requires PatOf(f).value.Var? && PatOf(g).value.Const?
    ensures f != g
  {
    assert f[0] == QUESTION;
    assert g[0] != QUESTION;
  }

  // ---------------------------------------------------------------------------
  // Splitting and joining, which is where the Kani route stopped
  // ---------------------------------------------------------------------------

  function IndexOf(s: seq<byte>, b: byte): nat
    ensures IndexOf(s, b) <= |s|
    ensures IndexOf(s, b) < |s| ==> s[IndexOf(s, b)] == b
    ensures forall k :: 0 <= k < IndexOf(s, b) ==> s[k] != b
  {
    if |s| == 0 then 0
    else if s[0] == b then 0
    else 1 + IndexOf(s[1..], b)
  }

  // `str::split(sep)`, and `String.splitOn` on the Lean side: n separators give
  // n+1 fields, empty fields included, so the result is never empty.
  function Split(s: seq<byte>, sep: byte): seq<seq<byte>>
    ensures |Split(s, sep)| > 0
    decreases |s|
  {
    var i := IndexOf(s, sep);
    if i == |s| then [s]
    else [s[..i]] + Split(s[i + 1..], sep)
  }

  function Join(parts: seq<seq<byte>>, sep: byte): seq<byte>
    requires |parts| > 0
    decreases |parts|
  {
    if |parts| == 1 then parts[0]
    else parts[0] + [sep] + Join(parts[1..], sep)
  }

  predicate NoByte(s: seq<byte>, b: byte) {
    forall k :: 0 <= k < |s| ==> s[k] != b
  }

  lemma IndexOfSkipsASeparatorFreePrefix(a: seq<byte>, rest: seq<byte>, sep: byte)
    requires NoByte(a, sep)
    ensures IndexOf(a + [sep] + rest, sep) == |a|
    decreases |a|
  {
    if |a| == 0 {
      assert (a + [sep] + rest)[0] == sep;
    } else {
      var whole := a + [sep] + rest;
      assert whole[0] == a[0] != sep;
      assert whole[1..] == a[1..] + [sep] + rest;
      IndexOfSkipsASeparatorFreePrefix(a[1..], rest, sep);
    }
  }

  // Split and Join are inverse exactly when no field carries the separator. This is
  // the lemma the Kani harnesses could not have: CBMC had to flatten `str::split`,
  // and the TCB page records that doing so measured `CharSearcher` rather than the
  // property. Here it is one induction.
  lemma SplitInvertsJoin(parts: seq<seq<byte>>, sep: byte)
    requires |parts| > 0
    requires forall i :: 0 <= i < |parts| ==> NoByte(parts[i], sep)
    ensures Split(Join(parts, sep), sep) == parts
    decreases |parts|
  {
    if |parts| == 1 {
      assert Join(parts, sep) == parts[0];
      assert IndexOf(parts[0], sep) == |parts[0]| by {
        assert NoByte(parts[0], sep);
        IndexOfIsLengthWhenAbsent(parts[0], sep);
      }
    } else {
      var rest := Join(parts[1..], sep);
      var whole := Join(parts, sep);
      assert whole == parts[0] + [sep] + rest;
      IndexOfSkipsASeparatorFreePrefix(parts[0], rest, sep);
      assert IndexOf(whole, sep) == |parts[0]|;
      assert whole[..|parts[0]|] == parts[0];
      assert whole[|parts[0]| + 1..] == rest;
      SplitInvertsJoin(parts[1..], sep);
      assert Split(whole, sep) == [parts[0]] + parts[1..];
    }
  }

  lemma IndexOfIsLengthWhenAbsent(s: seq<byte>, b: byte)
    requires NoByte(s, b)
    ensures IndexOf(s, b) == |s|
    decreases |s|
  {
    if |s| > 0 {
      IndexOfIsLengthWhenAbsent(s[1..], b);
    }
  }

  // ---------------------------------------------------------------------------
  // Decimal body length
  // ---------------------------------------------------------------------------

  predicate IsDigit(b: byte) { ZERO <= b <= NINE }

  function Digits(n: nat): seq<byte>
    ensures |Digits(n)| > 0
    ensures forall k :: 0 <= k < |Digits(n)| ==> IsDigit(Digits(n)[k])
    decreases n
  {
    if n < 10 then [ZERO + (n as byte)]
    else Digits(n / 10) + [ZERO + ((n % 10) as byte)]
  }

  function ToNat(s: seq<byte>): Option<nat>
    decreases |s|
  {
    if |s| == 0 then None
    else if |s| == 1 then
      (if IsDigit(s[0]) then Some((s[0] - ZERO) as nat) else None)
    else
      match ToNat(s[..|s| - 1])
      case None => None
      case Some(hi) =>
        if IsDigit(s[|s| - 1]) then Some(hi * 10 + (s[|s| - 1] - ZERO) as nat) else None
  }

  lemma DigitsRoundTrip(n: nat)
    ensures ToNat(Digits(n)) == Some(n)
    decreases n
  {
    if n < 10 {
      assert Digits(n) == [ZERO + (n as byte)];
    } else {
      DigitsRoundTrip(n / 10);
      var d := Digits(n);
      assert d == Digits(n / 10) + [ZERO + ((n % 10) as byte)];
      assert |d| > 1;
      assert d[..|d| - 1] == Digits(n / 10);
      assert d[|d| - 1] == ZERO + ((n % 10) as byte);
    }
  }

  // ---------------------------------------------------------------------------
  // The line
  // ---------------------------------------------------------------------------

  function AtomFields(a: AtomPat): seq<seq<byte>> {
    [Render(a.s), Render(a.p), Render(a.o)]
  }

  function BodyFields(body: seq<AtomPat>): seq<seq<byte>>
    ensures |BodyFields(body)| == 3 * |body|
    decreases |body|
  {
    if |body| == 0 then []
    else AtomFields(body[0]) + BodyFields(body[1..])
  }

  // `rules_tsv`, one line, and `OOCert.HornParse.ruleStr`:
  //   name TAB bodyLength TAB (s TAB p TAB o)* TAB hs TAB hp TAB ho
  function RuleFields(r: RulePattern): seq<seq<byte>> {
    [r.name, Digits(|r.body|)] + BodyFields(r.body) + AtomFields(r.head)
  }

  function RuleStr(r: RulePattern): seq<byte> {
    Join(RuleFields(r), TAB)
  }

  // The precondition TCB-20 does not state, and the point of this file.
  //
  // `RulePattern` is a name and two `String`s per position, and nothing in the Rust
  // type refuses a tab in any of them. A rule carrying one renders to a line with
  // more fields than it has positions, so it reads back as a DIFFERENT rule or as
  // an error. `RuleIsWritable` is what has to hold for the round trip to be true,
  // and `ATabInANameBreaksTheRoundTrip` below shows it is not decoration.
  predicate PatIsWritable(p: Pat) {
    WellFormedPat(p) && NoByte(Render(p), TAB) && NoByte(Render(p), LF)
  }

  predicate AtomIsWritable(a: AtomPat) {
    PatIsWritable(a.s) && PatIsWritable(a.p) && PatIsWritable(a.o)
  }

  predicate RuleIsWritable(r: RulePattern) {
    |r.name| > 0
    && NoByte(r.name, TAB)
    && NoByte(r.name, LF)
    && (forall i :: 0 <= i < |r.body| ==> AtomIsWritable(r.body[i]))
    && AtomIsWritable(r.head)
  }

  // Sequential rather than a match on a triple of `Option`s, which is both closer
  // to the Rust (`parse_pat(f)?` three times) and markedly cheaper to verify: the
  // tuple-match formulation made `ParseAtomsInvertsRender` time out at thirty
  // seconds, and this one discharges in under two.
  function ParseAtoms(fields: seq<seq<byte>>): Option<seq<AtomPat>>
    decreases |fields|
  {
    if |fields| == 0 then Some([])
    else if |fields| < 3 then None
    else
      var s := PatOf(fields[0]);
      var p := PatOf(fields[1]);
      var o := PatOf(fields[2]);
      if s.None? || p.None? || o.None? then None
      else
        var rest := ParseAtoms(fields[3..]);
        if rest.None? then None
        else Some([AtomPat(s.value, p.value, o.value)] + rest.value)
  }

  lemma ParseAtomsCountsFields(fields: seq<seq<byte>>)
    ensures ParseAtoms(fields).Some? ==> |fields| == 3 * |ParseAtoms(fields).value|
    decreases |fields|
  {
    if |fields| >= 3 {
      ParseAtomsCountsFields(fields[3..]);
    }
  }

  // `parse_rules`, one line. The head-variable-occurs-in-the-body check and the
  // error messages are left out: both are refusals, and a refusal cannot make a
  // round trip succeed that would otherwise fail. What is kept is everything that
  // decides WHICH rule a line denotes.
  function ParseRuleLine(line: seq<byte>): Option<RulePattern> {
    var fields := Split(line, TAB);
    if |fields| < 2 then None
    else if |fields[0]| == 0 then None
    else
      var len := ToNat(fields[1]);
      if len.None? then None
      else
        var m := len.value;
        var rest := fields[2..];
        if |rest| != 3 * m + 3 then None
        else
          var body := ParseAtoms(rest[..3 * m]);
          var head := ParseAtoms(rest[3 * m..]);
          if body.None? || head.None? then None
          else if |head.value| != 1 then None
          else Some(RulePattern(fields[0], body.value, head.value[0]))
  }

  lemma ParseAtomsInvertsRender(body: seq<AtomPat>)
    requires forall i :: 0 <= i < |body| ==> AtomIsWritable(body[i])
    ensures ParseAtoms(BodyFields(body)) == Some(body)
    decreases |body|
  {
    if |body| == 0 {
      assert BodyFields(body) == [];
    } else {
      var a := body[0];
      var tail := BodyFields(body[1..]);
      var f := BodyFields(body);
      assert f == AtomFields(a) + tail;
      assert |f| >= 3;
      assert f[0] == Render(a.s) && f[1] == Render(a.p) && f[2] == Render(a.o);
      assert f[3..] == tail;
      RenderRoundTripsPat(a.s);
      RenderRoundTripsPat(a.p);
      RenderRoundTripsPat(a.o);
      ParseAtomsInvertsRender(body[1..]);
      assert ParseAtoms(tail) == Some(body[1..]);
      assert AtomPat(a.s, a.p, a.o) == a;
      assert [a] + body[1..] == body;
    }
  }

  lemma NoFieldCarriesATab(r: RulePattern)
    requires RuleIsWritable(r)
    ensures forall i :: 0 <= i < |RuleFields(r)| ==> NoByte(RuleFields(r)[i], TAB)
  {
    var f := RuleFields(r);
    forall i | 0 <= i < |f| ensures NoByte(f[i], TAB) {
      if i == 0 {
        assert f[0] == r.name;
      } else if i == 1 {
        assert f[1] == Digits(|r.body|);
        assert forall k :: 0 <= k < |f[1]| ==> IsDigit(f[1][k]);
      } else if i < 2 + 3 * |r.body| {
        BodyFieldAt(r.body, i - 2);
        assert f[i] == BodyFields(r.body)[i - 2];
      } else {
        assert f[i] == AtomFields(r.head)[i - 2 - 3 * |r.body|];
      }
    }
  }

  // Every field of `BodyFields` is a rendering of a position of some atom, so it
  // inherits that atom's writability. Stated as its own induction because the
  // quantifier in `RuleIsWritable` is over ATOMS and the one above is over FIELDS.
  lemma BodyFieldAt(body: seq<AtomPat>, j: nat)
    requires j < 3 * |body|
    requires forall i :: 0 <= i < |body| ==> AtomIsWritable(body[i])
    ensures NoByte(BodyFields(body)[j], TAB)
    ensures NoByte(BodyFields(body)[j], LF)
    decreases |body|
  {
    var f := BodyFields(body);
    assert f == AtomFields(body[0]) + BodyFields(body[1..]);
    if j < 3 {
      assert AtomIsWritable(body[0]);
    } else {
      assert f[j] == BodyFields(body[1..])[j - 3];
      BodyFieldAt(body[1..], j - 3);
    }
  }

  // TCB-20, for every rule table whose fields can be written at all, with no bound
  // on any length and no bound on the body's size.
  lemma RuleStrRoundTrips(r: RulePattern)
    requires RuleIsWritable(r)
    ensures ParseRuleLine(RuleStr(r)) == Some(r)
  {
    var f := RuleFields(r);
    NoFieldCarriesATab(r);
    SplitInvertsJoin(f, TAB);
    assert Split(RuleStr(r), TAB) == f;
    assert |f| == 2 + 3 * |r.body| + 3;
    assert f[0] == r.name;
    assert f[1] == Digits(|r.body|);
    DigitsRoundTrip(|r.body|);
    var rest := f[2..];
    assert rest == BodyFields(r.body) + AtomFields(r.head);
    assert |rest| == 3 * |r.body| + 3;
    assert rest[..3 * |r.body|] == BodyFields(r.body);
    assert rest[3 * |r.body|..] == AtomFields(r.head);
    ParseAtomsInvertsRender(r.body);
    RenderRoundTripsPat(r.head.s);
    RenderRoundTripsPat(r.head.p);
    RenderRoundTripsPat(r.head.o);
    assert ParseAtoms(AtomFields(r.head)) == Some([r.head]) by {
      assert AtomFields(r.head)[3..] == [];
      assert ParseAtoms([]) == Some([]);
      assert [AtomPat(r.head.s, r.head.p, r.head.o)] + [] == [r.head];
    }
  }

  // ---------------------------------------------------------------------------
  // The precondition is not decoration
  // ---------------------------------------------------------------------------

  // `RuleIsWritable` is a real hypothesis, and this is the witness. A rule whose
  // NAME carries a tab renders to a line that reads back as a rule with a different
  // name, silently and with no error. Nothing in `RulePattern`, in `rules_tsv` or in
  // `parse_rules` refuses it: `parse_rules` checks that the name is non-empty and
  // never that it is tab-free, because a name it PARSED could not contain a tab.
  //
  // This is why the theorem above carries a hypothesis and why TCB-20's unconditional
  // phrasing is wrong. The hazard is not live today, and the decision record says
  // exactly why, but the property as written on that page is false and this is the
  // counterexample.
  lemma ATabInANameBreaksTheRoundTrip()
    ensures exists r: RulePattern ::
      |r.name| > 0 && NoByte(RuleStr(r), LF) && ParseRuleLine(RuleStr(r)) != Some(r)
  {
    var c := Const([LT, 120, 62]); // <x>
    var r := RulePattern([97, TAB, 98], [], AtomPat(c, c, c)); // name = "a<tab>b"

    assert |r.name| > 0;
    assert r.name[1] == TAB;
    assert !NoByte(r.name, TAB);
    assert !RuleIsWritable(r);

    // The rendered line carries no line feed, which is all `ParsedRulesAreWritable`
    // needs of it. Every field is a literal here, so this is a computation.
    assert RuleFields(r) == [r.name, Digits(0), [LT, 120, 62], [LT, 120, 62], [LT, 120, 62]];
    assert forall i :: 0 <= i < |RuleFields(r)| ==> NoByte(RuleFields(r)[i], LF);
    JoinAvoidsAByteNoFieldCarries(RuleFields(r), TAB, LF);
    assert NoByte(RuleStr(r), LF);

    // A rule the parser RETURNS is always writable, so a rule that is not writable
    // is not a rule the parser can return, whatever line it is given.
    if ParseRuleLine(RuleStr(r)) == Some(r) {
      ParsedRulesAreWritable(RuleStr(r));
      assert RuleIsWritable(r);
      assert false;
    }
  }

  // The same hazard one level down, in a VARIABLE name rather than a rule name, so
  // that nobody reads the lemma above as being only about the name column. This one
  // matters more, because a variable name is not something a reader thinks of as
  // carrying a separator, and `Pat::Var` is a bare `String`.
  lemma ATabInAVariableBreaksTheRoundTrip()
    ensures exists r: RulePattern ::
      WellFormedPat(r.head.s) && NoByte(RuleStr(r), LF)
      && ParseRuleLine(RuleStr(r)) != Some(r)
  {
    var bad := Var([120, TAB, 121]); // ?x<tab>y
    var c := Const([LT, 120, 62]);
    var r := RulePattern([97], [], AtomPat(bad, c, c));

    assert WellFormedPat(bad);
    assert Render(bad) == [QUESTION, 120, TAB, 121];
    assert Render(bad)[2] == TAB;
    assert !PatIsWritable(bad);
    assert !RuleIsWritable(r);

    assert RuleFields(r)
        == [[97], Digits(0), [QUESTION, 120, TAB, 121], [LT, 120, 62], [LT, 120, 62]];
    assert forall i :: 0 <= i < |RuleFields(r)| ==> NoByte(RuleFields(r)[i], LF);
    JoinAvoidsAByteNoFieldCarries(RuleFields(r), TAB, LF);
    assert NoByte(RuleStr(r), LF);

    if ParseRuleLine(RuleStr(r)) == Some(r) {
      ParsedRulesAreWritable(RuleStr(r));
      assert RuleIsWritable(r);
      assert false;
    }
  }

  lemma JoinAvoidsAByteNoFieldCarries(parts: seq<seq<byte>>, sep: byte, other: byte)
    requires |parts| > 0
    requires sep != other
    requires forall i :: 0 <= i < |parts| ==> NoByte(parts[i], other)
    ensures NoByte(Join(parts, sep), other)
    decreases |parts|
  {
    if |parts| > 1 {
      JoinAvoidsAByteNoFieldCarries(parts[1..], sep, other);
      var rest := Join(parts[1..], sep);
      var whole := parts[0] + [sep] + rest;
      assert Join(parts, sep) == whole;
      forall k | 0 <= k < |whole| ensures whole[k] != other {
        if k < |parts[0]| {
          assert whole[k] == parts[0][k];
        } else if k == |parts[0]| {
          assert whole[k] == sep;
        } else {
          assert whole[k] == rest[k - |parts[0]| - 1];
        }
      }
    }
  }

  // ---------------------------------------------------------------------------
  // What the engine actually writes does satisfy the hypothesis
  // ---------------------------------------------------------------------------

  // The reassuring half, and it is worth proving rather than asserting: anything
  // the parser produced is writable, so rendering it and reading it back is the
  // identity. That is the property TCB-18's re-parse guard leans on, and it is
  // what makes the two counterexamples above latent rather than live: the only
  // rules the engine writes are rules it parsed, or the built-in table, and a
  // parsed rule cannot carry a separator in any position.
  lemma ParsedRulesAreWritable(line: seq<byte>)
    requires ParseRuleLine(line).Some?
    requires NoByte(line, LF)
    ensures RuleIsWritable(ParseRuleLine(line).value)
  {
    var fields := Split(line, TAB);
    SplitFieldsCarryNoSeparator(line, TAB);
    SplitFieldsInheritAbsence(line, TAB, LF);

    var r := ParseRuleLine(line).value;
    var m := ToNat(fields[1]).value;
    var rest := fields[2..];
    var bodyFields := rest[..3 * m];
    var headFields := rest[3 * m..];
    var bodyOpt := ParseAtoms(bodyFields);
    var headOpt := ParseAtoms(headFields);

    assert bodyOpt.Some? && headOpt.Some?;
    assert r.name == fields[0] && r.body == bodyOpt.value && r.head == headOpt.value[0];
    assert |headOpt.value| == 1;
    assert headOpt.value == [r.head];

    assert forall i :: 0 <= i < |bodyFields| ==>
      NoByte(bodyFields[i], TAB) && NoByte(bodyFields[i], LF);
    assert forall i :: 0 <= i < |headFields| ==>
      NoByte(headFields[i], TAB) && NoByte(headFields[i], LF);

    ParsedAtomsAreWritable(bodyFields, bodyOpt.value);
    ParsedAtomsAreWritable(headFields, headOpt.value);
    assert AtomIsWritable(r.head);
  }

  lemma ParsedAtomsAreWritable(fields: seq<seq<byte>>, atoms: seq<AtomPat>)
    requires ParseAtoms(fields) == Some(atoms)
    requires forall i :: 0 <= i < |fields| ==> NoByte(fields[i], TAB) && NoByte(fields[i], LF)
    ensures forall i :: 0 <= i < |atoms| ==> AtomIsWritable(atoms[i])
    decreases |fields|
  {
    if |fields| == 0 {
      assert atoms == [];
    } else {
      PatOfClassifies(fields[0]);
      PatOfClassifies(fields[1]);
      PatOfClassifies(fields[2]);
      var restOpt := ParseAtoms(fields[3..]);
      assert restOpt.Some?;
      var a := AtomPat(PatOf(fields[0]).value, PatOf(fields[1]).value, PatOf(fields[2]).value);
      assert atoms == [a] + restOpt.value;
      // Each position renders back to its own field, and that field carries no
      // separator, so the position is writable.
      assert Render(a.s) == fields[0] && Render(a.p) == fields[1] && Render(a.o) == fields[2];
      assert AtomIsWritable(a);
      ParsedAtomsAreWritable(fields[3..], restOpt.value);
      assert forall i :: 1 <= i < |atoms| ==> atoms[i] == restOpt.value[i - 1];
    }
  }

  lemma SplitFieldsCarryNoSeparator(s: seq<byte>, sep: byte)
    ensures forall i :: 0 <= i < |Split(s, sep)| ==> NoByte(Split(s, sep)[i], sep)
    decreases |s|
  {
    var i := IndexOf(s, sep);
    if i != |s| {
      SplitFieldsCarryNoSeparator(s[i + 1..], sep);
    }
  }

  lemma SplitFieldsInheritAbsence(s: seq<byte>, sep: byte, other: byte)
    requires NoByte(s, other)
    ensures forall i :: 0 <= i < |Split(s, sep)| ==> NoByte(Split(s, sep)[i], other)
    decreases |s|
  {
    var i := IndexOf(s, sep);
    if i != |s| {
      SplitFieldsInheritAbsence(s[i + 1..], sep, other);
    }
  }
}
