import Dl.Syntax
import Dl.Semantics
import Dl.General
import Dl.Tableau
import Dl.Count
import Dl.Check
import Dl.Parse
import Dl.Witness
import Dl.Refute
import Dl.TableauDemo

/-!
# `Dl`: a model certificate for the SHIQ tableaux reasoner

The root module. Importing this builds every proof in the directory, including the axiom
pins, so a `sorry` or a changed axiom footprint anywhere fails `lake build`.

* `Dl/Syntax.lean` the fragment, the interpretation, the file format, and a list of what is
  deliberately not covered.
* `Dl/Semantics.lean` satisfaction, well-formedness, and what it means to be a model.
* `Dl/Count.lean` the pigeonhole lemma the number restrictions rest on.
* `Dl/Check.lean` the decision procedure, `Dl.satisfiable_of_checkModel`, and
  `Dl.checkModel_complete`.
* `Dl/Parse.lean` the two file readers, outside the theorem.
* `Dl/Witness.lean` an accepted model, two rejected forgeries, and an unsatisfiable axiom
  set, so that neither the checker nor `Satisfiable` is trivial.
-/
