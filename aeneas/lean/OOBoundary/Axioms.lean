import OOBoundary.Proofs

/-!
# The axiom footprint of this workspace, measured rather than assumed

`lean/` pins every headline theorem to `[propext, Classical.choice, Quot.sound]`
with a `#guard_msgs`-checked `#print axioms`, and the pin has no exceptions.
This directory is a DIFFERENT workspace on a different Lean with Mathlib and
eight more packages behind it, so its footprint is its own question and is
answered here rather than assumed to be the same.

Every line below is checked by `#guard_msgs`, so a change in what these theorems
rest on breaks `lake build` in this directory instead of quietly widening the
trust surface. `docs/aeneas-boundary.md` reproduces the result and says what it
does and does not mean.
-/

namespace OOBoundary

/-- info: 'OOBoundary.starts_with_byte_eq' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms starts_with_byte_eq

/-- info: 'OOBoundary.writable_triple_bytes_eq' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms writable_triple_bytes_eq

/-- info: 'OOBoundary.field_fits_the_format_eq' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms field_fits_the_format_eq

/-- info: 'OOBoundary.field_fits_the_format_iff' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms field_fits_the_format_iff

/-- info: 'OOBoundary.name_is_safe_bytes_eq' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms name_is_safe_bytes_eq

/-- info: 'OOBoundary.name_is_safe_bytes_iff' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms name_is_safe_bytes_iff

/-- info: 'OOBoundary.term_fits_the_format_eq' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms term_fits_the_format_eq

/-- info: 'OOBoundary.push_asserted_line_bytes_ok' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms push_asserted_line_bytes_ok

/-- info: 'OOBoundary.push_triple_fields_bytes_ok' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms push_triple_fields_bytes_ok

/-- info: 'OOBoundary.push_asserted_line_bytes_refuses' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms push_asserted_line_bytes_refuses

/-- info: 'OOBoundary.push_triple_fields_bytes_refuses' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms push_triple_fields_bytes_refuses

/-- info: 'OOBoundary.asserted_line_reads_back' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms asserted_line_reads_back

/-- info: 'OOBoundary.triple_fields_adds_exactly_three_fields' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms triple_fields_adds_exactly_three_fields

/-- info: 'OOBoundary.triple_fields_are_the_three_terms' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms triple_fields_are_the_three_terms

end OOBoundary
