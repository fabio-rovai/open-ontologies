import OOCert.Institution
import OOCert.Comorphism
import OOCert.InstitutionRdf
import OOCert.InstitutionFol
import OOCert.InstitutionWitness

/-!
# `Institution`: what a logic is, what a translation between two logics is, and
# the obligation neither may be built without

The root module. Importing this builds every proof in the institution layer,
including the axiom pins in `OOCert/InstitutionWitness.lean`, so a `sorry`, a
`native_decide` or a changed axiom footprint anywhere in the layer fails
`lake build`.

It is a SEPARATE library from `OOCert` on purpose. `OOCert.lean` is what
`Main.lean` and `HMain.lean` import, so anything added to it goes into the
`oo-cert` and `oo-horn` binaries. The institution layer imports `Fol.Semantics`
and has no run-time part at all: it is theorems about the two model classes the
other libraries already carry, and there is no executable that would read it.

* `OOCert/Institution.lean` the structure, with the satisfaction condition as a
  field; entailment; the institution of theories; and the quarantined
  functoriality laws.
* `OOCert/Comorphism.lean` the translation, with its own satisfaction condition
  as a field; entailment PRESERVATION with no hypothesis; entailment REFLECTION
  under model expansiveness, which is a different theorem and is kept one.
* `OOCert/InstitutionRdf.lean` two instances out of `Semantics.lean`: simple RDF
  interpretations and OWL 2 RL ones, with `conditions_reduct` proving that the
  twenty-three semantic conditions survive exactly those renamings that fix the
  reserved vocabulary.
* `OOCert/InstitutionFol.lean` the first-order instance out of
  `Fol/Semantics.lean`, and the theoroidal comorphism from RDF into it.
* `OOCert/InstitutionWitness.lean` the non-vacuity: distinct models in every
  institution, models of the background theory, and the machine-checked pair
  that separates preservation from reflection.

What this layer is NOT is in `docs/decisions/0009-a-translation-between-logics-carries-its-satisfaction-condition.md`:
no DOL parser, no Hets dependency, no naturality, and no borrowing theorem for
the first-order comorphism.
-/
