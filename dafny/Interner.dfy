// TCB-15, TCB-16 and TCB-17 without a bound: the interner.
//
// `docs/trusted-computing-base.md` states them as
//
//   TCB-15 (round trip)   `resolve(intern(s)) == s` for every string.
//   TCB-16 (injectivity)  `intern(a) == intern(b)` if and only if `a == b`.
//   TCB-17 (stability)    an identifier's resolution does not change as more
//                         strings are interned.
//
// All three are property-tested, which samples, and none is proved. They are
// worth more than that. The interner is the one place where a term stops being
// bytes and becomes a number, and every rule the fixpoint evaluates runs on
// those numbers. If two different terms could collide on one identifier, a rule
// would fire on a premise nobody asserted, and the certificate would record a
// derivation true of the INTERNED graph and not of the real one. The Lean side
// cannot see that: it reads `asserted.tsv`, which is written from the strings.
//
// KANI CANNOT REACH THESE, and that was measured rather than assumed. A harness
// over `Interner` returns
//
//   1 of 5109 failed (5108 undetermined)
//   Failed Checks: call to foreign "C" function `CCRandomGenerateBytes`
//                  is not currently supported by Kani
//
// which is `std`'s `HashMap` seeding `RandomState` from the operating system.
// Replacing the hasher with a fixed one removes that call and the harness still
// fails, so the obstacle is `HashMap`'s internals rather than the seed. No bound
// helps, because the obstacle is an unsupported construct and not a cost.
//
// So this file models the interner as a map and a sequence, exactly the two
// fields `src/reason.rs` carries, and proves all three statements for every
// string and every number of insertions.
//
// WHAT IT IS NOT. Decision 0014 applies here unchanged: this is a
// REIMPLEMENTATION, so nothing proved here is evidence about `src/reason.rs`.
// It is evidence that the DESIGN is sound, which is a different and smaller
// claim, and the correspondence between this model and that Rust is read by a
// person. The model is deliberately small so that reading is cheap: two fields,
// two operations, and the same order of writes.

datatype Interner = Interner(toId: map<string, nat>, toStr: seq<string>)

// The invariant the Rust maintains by construction: the map and the sequence
// agree, ids are exactly the positions of the sequence, and no string is
// interned twice.
predicate Wf(i: Interner)
{
  && (forall s :: s in i.toId ==> i.toId[s] < |i.toStr|)
  && (forall s :: s in i.toId ==> i.toStr[i.toId[s]] == s)
  && (forall k :: 0 <= k < |i.toStr| ==> i.toStr[k] in i.toId && i.toId[i.toStr[k]] == k)
}

function Empty(): Interner { Interner(map[], []) }

lemma EmptyIsWf()
  ensures Wf(Empty())
{
}

// `intern` in `src/reason.rs`: return the existing id if there is one,
// otherwise append and hand back the new position.
function Intern(i: Interner, s: string): (Interner, nat)
{
  if s in i.toId then (i, i.toId[s])
  else (Interner(i.toId[s := |i.toStr|], i.toStr + [s]), |i.toStr|)
}

// `resolve` in `src/reason.rs`, which indexes the sequence.
function Resolve(i: Interner, id: nat): string
  requires id < |i.toStr|
{
  i.toStr[id]
}

lemma InternPreservesWf(i: Interner, s: string)
  requires Wf(i)
  ensures Wf(Intern(i, s).0)
  ensures Intern(i, s).1 < |Intern(i, s).0.toStr|
{
  var (j, id) := Intern(i, s);
  if s in i.toId {
  } else {
    assert j.toStr == i.toStr + [s];
    forall k | 0 <= k < |j.toStr|
      ensures k < |j.toStr| && j.toStr[k] in j.toId && j.toId[j.toStr[k]] == k
    {
      if k < |i.toStr| {
        assert j.toStr[k] == i.toStr[k];
      } else {
        assert k == |i.toStr|;
        assert j.toStr[k] == s;
      }
    }
  }
}

// TCB-15. For EVERY string, of every length, at every point in the interner's
// life.
lemma RoundTrip(i: Interner, s: string)
  requires Wf(i)
  ensures var (j, id) := Intern(i, s); id < |j.toStr| && Resolve(j, id) == s
{
  InternPreservesWf(i, s);
}

// TCB-16, both directions. The unsound direction is left to right: one
// identifier for two different strings is what would let a rule fire on a
// premise nobody asserted.
lemma Injective(i: Interner, a: string, b: string)
  requires Wf(i)
  ensures var (j, ida) := Intern(i, a);
          var (k, idb) := Intern(j, b);
          (ida == idb) <==> (a == b)
{
  InternPreservesWf(i, a);
  var (j, ida) := Intern(i, a);
  InternPreservesWf(j, b);
  var (k, idb) := Intern(j, b);
  assert Resolve(k, ida) == a by { RoundTripSurvives(i, a, b); }
  assert Resolve(k, idb) == b;
}

// TCB-17. Interning something else does not move what an issued identifier
// resolves to. This is what makes the identifier a NAME rather than a position
// that shifts, and it is what `Injective` above leans on.
lemma RoundTripSurvives(i: Interner, a: string, b: string)
  requires Wf(i)
  ensures var (j, ida) := Intern(i, a);
          var (k, _) := Intern(j, b);
          ida < |k.toStr| && Resolve(k, ida) == a
{
  InternPreservesWf(i, a);
  var (j, ida) := Intern(i, a);
  InternPreservesWf(j, b);
  var (k, idb) := Intern(j, b);
  assert Resolve(j, ida) == a;
  assert k.toStr == j.toStr || k.toStr == j.toStr + [b];
}

// The sequence only ever grows, and never at the front. A single lemma
// covering both statements the two above rely on.
lemma Monotone(i: Interner, s: string)
  requires Wf(i)
  ensures var (j, _) := Intern(i, s);
          |i.toStr| <= |j.toStr|
          && forall k :: 0 <= k < |i.toStr| ==> j.toStr[k] == i.toStr[k]
{
}

// Stability over an ARBITRARY run of later insertions, not just one. This is
// the statement the property test was sampling: intern a string, then intern
// any number of other strings, and the first identifier still resolves to what
// it did.
lemma StableAcrossManyInterns(i: Interner, a: string, rest: seq<string>)
  requires Wf(i)
  ensures var (j, ida) := Intern(i, a);
          var k := InternAll(j, rest);
          ida < |k.toStr| && Resolve(k, ida) == a
  decreases |rest|
{
  InternPreservesWf(i, a);
  var (j, ida) := Intern(i, a);
  InternAllKeepsPrefix(j, rest);
}

function InternAll(i: Interner, ss: seq<string>): Interner
  decreases |ss|
{
  if |ss| == 0 then i else InternAll(Intern(i, ss[0]).0, ss[1..])
}

lemma InternAllKeepsPrefix(i: Interner, ss: seq<string>)
  requires Wf(i)
  ensures var j := InternAll(i, ss);
          Wf(j)
          && |i.toStr| <= |j.toStr|
          && forall k :: 0 <= k < |i.toStr| ==> j.toStr[k] == i.toStr[k]
  decreases |ss|
{
  if |ss| == 0 {
  } else {
    InternPreservesWf(i, ss[0]);
    var (j, _) := Intern(i, ss[0]);
    InternAllKeepsPrefix(j, ss[1..]);
  }
}
