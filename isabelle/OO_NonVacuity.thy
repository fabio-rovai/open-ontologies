(* OO_NonVacuity.thy -- the conditions are satisfiable, and satisfiable NON-DEGENERATELY.

   A soundness theorem over an unsatisfiable model class proves nothing: if no
   interpretation met the 26 conditions, every triple would be entailed and the checker
   could accept anything.  That is the standing obligation.  It is easy to discharge
   BADLY, and this file is arranged so that the bad discharge is visible.

   THE WEAK WITNESS, and why it is not enough.  The one-point interpretation W (below)
   satisfies every condition, so the class is non-empty.  But W makes ICEXT constant and
   IC the whole universe, so c_sco_bwd, c_dom_bwd and c_rng_bwd -- the BACKWARD halves of
   RBS Table 5.8, which twelve of the 27 arms depend on -- are satisfied by collapse
   rather than by construction.  Worse, the only triple W fails to satisfy is one whose
   subject is a LITERAL with no denotation, so the refutation turns entirely on the
   partiality of IL (M3) and says nothing about the IRI-only case, which is the
   interesting one.

   THE WITNESS FOR TABLE 5.8 AND FOR THE FIXTURE GRAPH is M3.  It is a model of the
   ACTUAL FIXTURE GRAPH tests/fixtures/horn/asserted.tsv; it has IC non-empty, so the
   Table 5.8 backward conditions are LIVE rather than vacuous; it satisfies the
   conclusion the fixture certificate derives, as soundness requires; and it refutes an
   IRI-only triple over that same non-empty graph.

   It does NOT separate the IP clause of the truth condition M5, although the comment at
   M3_refutes_sameAs reads as though it does.  zz is outside IP AND has an empty
   extension, so that triple is false for two reasons at once and would stay false if the
   IP clause were dropped.  The same is true of M4.  Isolating that clause needs a
   predicate outside IP with a NON-EMPTY extension, which costs bridge coherence, and no
   witness in this file has one.

   M3 IS NOT THE WITNESS FOR THE REST OF THE TABLE, and an earlier version of this
   header called it THE REAL WITNESS with no such qualification.  FIFTEEN of the
   twenty-six conditions hold in M3 only because the extension their antecedent reads is
   EMPTY: c_dom_fwd, c_rng_fwd, c_sameAs_fwd, c_eqc_fwd, c_eqp_fwd, c_inv_fwd,
   c_sym_fwd, c_trp_fwd, c_svf, c_avf, c_hv, c_restr_IC, c_svf_typ, c_avf_typ and
   c_onp_typ.  Each of those is discharged below by a bare "by simp" off the single fact
   that rdfs:domain, rdfs:range and every owl: term denote the junk element zz, whose
   extension is {}.  Nothing about M3 is wrong; what is wrong is calling it the witness
   for conditions it never touches.

   THE WITNESS FOR THE OTHER FIFTEEN is M4, in section N4.  Every one of the twenty-six
   conditions has a satisfied antecedent there (M4_every_condition_has_a_live_antecedent)
   and all fourteen derived monotonicity and subsumption lemmas of OO_Builtin_Sound fire
   at concrete elements of it (M4_exercises_every_derivation), so an edit that hollows
   the model out breaks the build instead of passing quietly. *)

theory OO_NonVacuity
  imports OO_Builtin_Sound
begin

section \<open>N1: the weak witness, recorded with its weakness\<close>

definition W :: "unit interp" where
  "W = \<lparr> IR = UNIV, IP = UNIV, IEXT = (\<lambda>x. UNIV),
         IS = (\<lambda>u. ()), IB = (\<lambda>b. ()), IL = (\<lambda>l. None) \<rparr>"

lemma W_simps [simp]:
  "IR W = UNIV" "IP W = UNIV" "IEXT W x = UNIV"
  "IS W u = ()" "IB W b = ()" "IL W l = None"
  by (simp_all add: W_def)

lemma W_ICEXT [simp]: "ICEXT W y = UNIV" by (simp add: ICEXT_def)
lemma W_IC [simp]: "IC W = UNIV" by (simp add: IC_def)

theorem conditions_satisfiable: "owl_rl_interp W"
  by (simp add: owl_rl_interp_def
      c_type_IP_def c_sco_IP_def c_spo_IP_def
      c_sco_fwd_def c_sco_bwd_def c_spo_fwd_def c_spo_bwd_def
      c_sco_trans_def c_spo_trans_def
      c_dom_fwd_def c_dom_bwd_def c_rng_fwd_def c_rng_bwd_def
      c_sameAs_fwd_def c_eqc_fwd_def c_eqp_fwd_def c_inv_fwd_def
      c_sym_fwd_def c_trp_fwd_def c_svf_def c_avf_def c_hv_def
      c_restr_IC_def c_svf_typ_def c_avf_typ_def c_onp_typ_def)

(* W also satisfies the domain conditions, so the model class stays non-empty when the
   bridge theorem T4 adds them back. *)
lemma W_wf: "wf_interp W" by (simp add: wf_interp_def)

section \<open>The fixture IRIs, transcribed from tests/fixtures/horn/asserted.tsv\<close>

(* Angle brackets stripped: see DECISION D-PARSE-2 in OO_Parse.  ex_C does not occur in
   any fixture; it is the fresh IRI whose triple M3 refutes. *)

definition ex_a :: string where "ex_a = ''http://ex.org/a''"
definition ex_A :: string where "ex_A = ''http://ex.org/A''"
definition ex_B :: string where "ex_B = ''http://ex.org/B''"
definition ex_C :: string where "ex_C = ''http://ex.org/C''"

definition fixture_graph :: "triple list" where
  "fixture_graph = [ (Iri ex_a, Iri rdf_type, Iri ex_A),
                     (Iri ex_A, Iri rdfs_subClassOf, Iri ex_B) ]"

section \<open>N3: a non-degenerate model of the fixture graph\<close>

(* THE CONSTRUCTION, AND WHY IT IS NOT OBVIOUS.  Once IC is non-empty the backward halves
   of Table 5.8 bite, and they are close to circular: c_spo_bwd forces
   (p1,p2) into IEXT(I(rdfs:subPropertyOf)) whenever p1, p2 are in IP and IEXT p1 is
   contained in IEXT p2 -- and I(rdfs:subPropertyOf) is itself in IP, by c_spo_IP, so the
   set being defined occurs on both sides of its own defining condition, once
   monotonically and once antitonically.  A naive "collapse everything" interpretation
   does not solve it.

   The circularity is broken by SEPARATING THE KINDS.  Property elements, class elements,
   the individual and the junk element are distinct constructors, and the three property
   extensions T3, S3, Q3 live in disjoint parts of the pair space.  Then no two of them
   are comparable under containment except reflexively, the defining condition for Q3
   collapses to the DIAGONAL on IP, and the diagonal is a fixpoint of it.  Verified
   below, case by case, rather than asserted.

   Every unused vocabulary IRI is sent to the single junk element zz with EMPTY
   extension.  That is what makes sameAs, equivalentClass, equivalentProperty, inverseOf,
   someValuesFrom, allValuesFrom, hasValue, onProperty, domain, range, Restriction,
   SymmetricProperty and TransitiveProperty all vacuous at once -- and it is sound
   precisely because nothing in the condition set FORCES a pair into any of those
   extensions.  It is also the LIMIT of what M3 establishes: fifteen of the twenty-six
   conditions are satisfied here by having nothing to check, and M4 in section N4 is
   the model that checks them.  Checked: only c_sco_bwd, c_spo_bwd, c_dom_bwd and c_rng_bwd have a
   consequent that asserts membership, and their consequents name subClassOf,
   subPropertyOf, domain and range.  domain and range are handled by showing their
   antecedents false. *)

datatype E = pT | pS | pQ | cA | cB | cls | ia | zz

definition T3 :: "(E \<times> E) set" where
  "T3 = {(ia,cA), (ia,cB), (cA,cls), (cB,cls)}"
definition S3 :: "(E \<times> E) set" where
  "S3 = {(cA,cA), (cA,cB), (cB,cA), (cB,cB)}"
definition Q3 :: "(E \<times> E) set" where
  "Q3 = {(pT,pT), (pS,pS), (pQ,pQ)}"

definition M3 :: "E interp" where
  "M3 = \<lparr> IR = UNIV,
          IP = {pT, pS, pQ},
          IEXT = (\<lambda>x. if x = pT then T3 else if x = pS then S3
                      else if x = pQ then Q3 else {}),
          IS = (\<lambda>u. if u = ex_a then ia
                    else if u = ex_A then cA
                    else if u = ex_B then cB
                    else if u = rdf_type then pT
                    else if u = rdfs_subClassOf then pS
                    else if u = rdfs_subPropertyOf then pQ
                    else if u = rdfs_Class then cls
                    else zz),
          IB = (\<lambda>b. zz),
          IL = (\<lambda>l. None) \<rparr>"

lemma M3_IP [simp]: "IP M3 = {pT, pS, pQ}" by (simp add: M3_def)
lemma M3_IR [simp]: "IR M3 = UNIV" by (simp add: M3_def)
lemma M3_IB [simp]: "IB M3 b = zz" by (simp add: M3_def)
lemma M3_IL [simp]: "IL M3 l = None" by (simp add: M3_def)

lemma M3_IEXT [simp]:
  "IEXT M3 pT = T3" "IEXT M3 pS = S3" "IEXT M3 pQ = Q3"
  "IEXT M3 cA = {}" "IEXT M3 cB = {}" "IEXT M3 cls = {}"
  "IEXT M3 ia = {}" "IEXT M3 zz = {}"
  by (simp_all add: M3_def)

lemma M3_IS [simp]:
  "IS M3 ex_a = ia" "IS M3 ex_A = cA" "IS M3 ex_B = cB"
  "IS M3 rdf_type = pT" "IS M3 rdfs_subClassOf = pS"
  "IS M3 rdfs_subPropertyOf = pQ" "IS M3 rdfs_Class = cls"
  "IS M3 ex_C = zz"
  "IS M3 rdfs_domain = zz" "IS M3 rdfs_range = zz"
  "IS M3 owl_sameAs = zz" "IS M3 owl_equivalentClass = zz"
  "IS M3 owl_equivalentProperty = zz" "IS M3 owl_inverseOf = zz"
  "IS M3 owl_SymmetricProperty = zz" "IS M3 owl_TransitiveProperty = zz"
  "IS M3 owl_Restriction = zz" "IS M3 owl_onProperty = zz"
  "IS M3 owl_someValuesFrom = zz" "IS M3 owl_allValuesFrom = zz"
  "IS M3 owl_hasValue = zz"
  by (simp_all add: M3_def ex_a_def ex_A_def ex_B_def ex_C_def
      rdf_type_def rdfs_Class_def rdfs_subClassOf_def rdfs_subPropertyOf_def
      rdfs_domain_def rdfs_range_def owl_sameAs_def owl_equivalentClass_def
      owl_equivalentProperty_def owl_inverseOf_def owl_SymmetricProperty_def
      owl_TransitiveProperty_def owl_Restriction_def owl_onProperty_def
      owl_someValuesFrom_def owl_allValuesFrom_def owl_hasValue_def)

lemma M3_ICEXT_raw: "ICEXT M3 y = {x. (x,y) \<in> T3}"
  by (simp add: ICEXT_def)

lemma M3_ICEXT [simp]:
  "ICEXT M3 pT = {}" "ICEXT M3 pS = {}" "ICEXT M3 pQ = {}"
  "ICEXT M3 cA = {ia}" "ICEXT M3 cB = {ia}" "ICEXT M3 cls = {cA, cB}"
  "ICEXT M3 ia = {}" "ICEXT M3 zz = {}"
  by (auto simp: M3_ICEXT_raw T3_def)

lemma M3_IC [simp]: "IC M3 = {cA, cB}" by (simp add: IC_def)

subsection \<open>Each condition, discharged separately so a failure names itself\<close>

lemma M3_type_IP: "c_type_IP M3" by (simp add: c_type_IP_def)
lemma M3_sco_IP:  "c_sco_IP M3"  by (simp add: c_sco_IP_def)
lemma M3_spo_IP:  "c_spo_IP M3"  by (simp add: c_spo_IP_def)

lemma M3_sco_fwd: "c_sco_fwd M3"
  unfolding c_sco_fwd_def by (auto simp: S3_def)

lemma M3_sco_bwd: "c_sco_bwd M3"
  unfolding c_sco_bwd_def by (auto simp: S3_def)

lemma M3_sco_trans: "c_sco_trans M3"
  unfolding c_sco_trans_def by (auto simp: S3_def)

(* The fixpoint.  Q3 is exactly the diagonal on IP, and the nine cases below are the
   check that it IS a fixpoint of the condition rather than merely a plausible guess. *)
lemma M3_spo_fwd: "c_spo_fwd M3"
  unfolding c_spo_fwd_def by (auto simp: Q3_def)

lemma M3_spo_bwd: "c_spo_bwd M3"
  unfolding c_spo_bwd_def
proof (intro allI impI)
  fix p1 p2 :: E
  assume a1: "p1 \<in> IP M3" and a2: "p2 \<in> IP M3" and sub: "IEXT M3 p1 \<subseteq> IEXT M3 p2"
  from a1 have c1: "p1 = pT \<or> p1 = pS \<or> p1 = pQ" by auto
  from a2 have c2: "p2 = pT \<or> p2 = pS \<or> p2 = pQ" by auto
  from c1 c2 sub show "(p1,p2) \<in> IEXT M3 (IS M3 rdfs_subPropertyOf)"
    by (elim disjE) (auto simp: T3_def S3_def Q3_def)
qed

lemma M3_spo_trans: "c_spo_trans M3"
  unfolding c_spo_trans_def by (auto simp: Q3_def)

(* domain and range: the extension of the junk element is empty, so the consequent is
   False, and the proof must show the ANTECEDENT is false for every p in IP and c in IC.
   It is, because every property extension has a first (resp. second) component outside
   ICEXT cA = ICEXT cB = {ia}. *)
lemma M3_dom_fwd: "c_dom_fwd M3" unfolding c_dom_fwd_def by simp
lemma M3_rng_fwd: "c_rng_fwd M3" unfolding c_rng_fwd_def by simp

lemma M3_dom_bwd: "c_dom_bwd M3"
  unfolding c_dom_bwd_def
proof (intro allI impI)
  fix p c :: E
  assume a1: "p \<in> IP M3" and a2: "c \<in> IC M3"
    and h: "\<forall>x y. (x,y) \<in> IEXT M3 p \<longrightarrow> x \<in> ICEXT M3 c"
  have icx: "ICEXT M3 c = {ia}" using a2 by auto
  have False
  proof -
    from a1 consider (t) "p = pT" | (s) "p = pS" | (q) "p = pQ" by auto
    then show False
    proof cases
      case t
      have m: "(cA, cls) \<in> IEXT M3 p" using t by (simp add: T3_def)
      from h m have "cA \<in> ICEXT M3 c" by blast
      with icx show False by simp
    next
      case s
      have m: "(cA, cA) \<in> IEXT M3 p" using s by (simp add: S3_def)
      from h m have "cA \<in> ICEXT M3 c" by blast
      with icx show False by simp
    next
      case q
      have m: "(pT, pT) \<in> IEXT M3 p" using q by (simp add: Q3_def)
      from h m have "pT \<in> ICEXT M3 c" by blast
      with icx show False by simp
    qed
  qed
  then show "(p,c) \<in> IEXT M3 (IS M3 rdfs_domain)" by simp
qed

lemma M3_rng_bwd: "c_rng_bwd M3"
  unfolding c_rng_bwd_def
proof (intro allI impI)
  fix p c :: E
  assume a1: "p \<in> IP M3" and a2: "c \<in> IC M3"
    and h: "\<forall>x y. (x,y) \<in> IEXT M3 p \<longrightarrow> y \<in> ICEXT M3 c"
  have icx: "ICEXT M3 c = {ia}" using a2 by auto
  have False
  proof -
    from a1 consider (t) "p = pT" | (s) "p = pS" | (q) "p = pQ" by auto
    then show False
    proof cases
      case t
      have m: "(ia, cA) \<in> IEXT M3 p" using t by (simp add: T3_def)
      from h m have "cA \<in> ICEXT M3 c" by blast
      with icx show False by simp
    next
      case s
      have m: "(cA, cA) \<in> IEXT M3 p" using s by (simp add: S3_def)
      from h m have "cA \<in> ICEXT M3 c" by blast
      with icx show False by simp
    next
      case q
      have m: "(pT, pT) \<in> IEXT M3 p" using q by (simp add: Q3_def)
      from h m have "pT \<in> ICEXT M3 c" by blast
      with icx show False by simp
    qed
  qed
  then show "(p,c) \<in> IEXT M3 (IS M3 rdfs_range)" by simp
qed

lemma M3_sameAs_fwd: "c_sameAs_fwd M3" unfolding c_sameAs_fwd_def by simp
lemma M3_eqc_fwd: "c_eqc_fwd M3" unfolding c_eqc_fwd_def by simp
lemma M3_eqp_fwd: "c_eqp_fwd M3" unfolding c_eqp_fwd_def by simp
lemma M3_inv_fwd: "c_inv_fwd M3" unfolding c_inv_fwd_def by simp
lemma M3_sym_fwd: "c_sym_fwd M3" unfolding c_sym_fwd_def by simp
lemma M3_trp_fwd: "c_trp_fwd M3" unfolding c_trp_fwd_def by simp
lemma M3_svf: "c_svf M3" unfolding c_svf_def by simp
lemma M3_avf: "c_avf M3" unfolding c_avf_def by simp
lemma M3_hv: "c_hv M3" unfolding c_hv_def by simp
lemma M3_restr_IC: "c_restr_IC M3" unfolding c_restr_IC_def by simp
lemma M3_svf_typ: "c_svf_typ M3" unfolding c_svf_typ_def by simp
lemma M3_avf_typ: "c_avf_typ M3" unfolding c_avf_typ_def by simp
lemma M3_onp_typ: "c_onp_typ M3" unfolding c_onp_typ_def by simp

theorem M3_is_a_model: "owl_rl_interp M3"
  unfolding owl_rl_interp_def
  by (simp add: M3_type_IP M3_sco_IP M3_spo_IP M3_sco_fwd M3_sco_bwd
      M3_spo_fwd M3_spo_bwd M3_sco_trans M3_spo_trans
      M3_dom_fwd M3_dom_bwd M3_rng_fwd M3_rng_bwd
      M3_sameAs_fwd M3_eqc_fwd M3_eqp_fwd M3_inv_fwd M3_sym_fwd M3_trp_fwd
      M3_svf M3_avf M3_hv M3_restr_IC M3_svf_typ M3_avf_typ M3_onp_typ)

(* M3 also satisfies the domain conditions of RBS Table 5.1, so it witnesses
   satisfiability for the bridge theorem T4 as well as for entails. *)
theorem M3_wf: "wf_interp M3" by (simp add: wf_interp_def)

(* THE POINT OF M3, and its exact limit.  The Table 5.8 backward conditions are LIVE in
   it, not vacuous.  A witness with IC empty satisfies c_sco_bwd, c_dom_bwd and c_rng_bwd
   by having nothing to quantify over, and therefore provides NO evidence that those
   conditions -- which twelve of the 27 arms depend on -- are jointly satisfiable
   alongside the rest.  What M3 does NOT establish is stated as a theorem rather than
   left to inspection: M3_leaves_these_extensions_empty below names the thirteen extensions
   that are empty here, and M4 in section N4 is where each of them has something in it. *)
theorem M3_is_not_degenerate:
  "IC M3 \<noteq> {}" and "ICEXT M3 cA \<noteq> {}" and "IEXT M3 (IS M3 rdfs_subClassOf) \<noteq> {}"
  by (simp_all add: S3_def)

(* AND THE EXACT LIMIT OF M3, as a theorem rather than as a remark.  These are the
   thirteen extensions the fifteen unexercised conditions read, and every one of them is
   empty here, which is the only reason those fifteen conditions hold.  Stated so that
   the claim in the file header cannot drift from the model.  Section N4 is where each of
   these has something in it. *)
theorem M3_leaves_these_extensions_empty:
  "IEXT M3 (IS M3 rdfs_domain) = {}
     \<and> IEXT M3 (IS M3 rdfs_range) = {}
     \<and> IEXT M3 (IS M3 owl_sameAs) = {}
     \<and> IEXT M3 (IS M3 owl_equivalentClass) = {}
     \<and> IEXT M3 (IS M3 owl_equivalentProperty) = {}
     \<and> IEXT M3 (IS M3 owl_inverseOf) = {}
     \<and> IEXT M3 (IS M3 owl_someValuesFrom) = {}
     \<and> IEXT M3 (IS M3 owl_allValuesFrom) = {}
     \<and> IEXT M3 (IS M3 owl_hasValue) = {}
     \<and> IEXT M3 (IS M3 owl_onProperty) = {}
     \<and> ICEXT M3 (IS M3 owl_Restriction) = {}
     \<and> ICEXT M3 (IS M3 owl_SymmetricProperty) = {}
     \<and> ICEXT M3 (IS M3 owl_TransitiveProperty) = {}"
  by simp

subsection \<open>What M3 makes true, and what it makes false\<close>

lemma M3_sat_asserted_1: "sat M3 (Iri ex_a, Iri rdf_type, Iri ex_A)"
  by (simp add: T3_def)
lemma M3_sat_asserted_2: "sat M3 (Iri ex_A, Iri rdfs_subClassOf, Iri ex_B)"
  by (simp add: S3_def)

theorem M3_models_fixture_graph: "models M3 (set fixture_graph)"
  by (simp add: models_def fixture_graph_def T3_def S3_def)

(* Consistency check on the whole development: the conclusion the fixture certificate
   good.tsv derives by rdfs9 MUST be true here, since soundness says so.  If this failed,
   either the checker or the semantics would be wrong. *)
theorem M3_sat_the_certified_conclusion: "sat M3 (Iri ex_a, Iri rdf_type, Iri ex_B)"
  by (simp add: T3_def)

(* Refutation 1: an IRI-only triple, over the NON-EMPTY fixture graph.  Nothing here
   turns on the partiality of IL. *)
theorem M3_refutes_fresh_class: "\<not> sat M3 (Iri ex_a, Iri rdf_type, Iri ex_C)"
  by (simp add: T3_def)

(* Refutation 2: exercises the IP clause of the truth condition (M5) specifically.  The
   predicate owl:sameAs denotes zz, which is not in IP, so the triple is false however
   its subject and object are related.  Under a truth clause that omits the IP
   requirement this triple would be false only for a different reason, and under one that
   cases on the predicate it might not be false at all. *)
theorem M3_refutes_sameAs: "\<not> sat M3 (Iri ex_a, Iri owl_sameAs, Iri ex_a)"
  by simp

section \<open>The headline non-vacuity results\<close>

theorem entailment_is_not_trivial:
  "\<not> entails TYPE(E) (set fixture_graph) (Iri ex_a, Iri rdf_type, Iri ex_C)"
  unfolding entails_def
  using M3_is_a_model M3_models_fixture_graph M3_refutes_fresh_class by blast

(* Entailment from a larger graph is easier, so a refutation over the fixture graph
   gives the empty-graph refutation for free.  Recorded because the brief's suggested
   separate empty-graph witness is subsumed by this one. *)
theorem entailment_is_not_trivial_empty:
  "\<not> entails TYPE(E) {} (Iri ex_a, Iri rdf_type, Iri ex_C)"
  unfolding entails_def
  using M3_is_a_model M3_refutes_fresh_class by (auto simp: models_def)

theorem the_IP_clause_has_content:
  "\<not> entails TYPE(E) (set fixture_graph) (Iri ex_a, Iri owl_sameAs, Iri ex_a)"
  unfolding entails_def
  using M3_is_a_model M3_models_fixture_graph M3_refutes_sameAs by blast

section \<open>N4: a model in which no condition holds for want of anything to check\<close>

(* WHAT M3 LEAVES UNEXERCISED, and why a second witness is needed.

   M3 is a genuine model of the fixture graph and it makes the Table 5.8 BACKWARD
   conditions live, which is what the section above claims for it.  It is not the real
   witness for the rest of the table, and the header above overstates it.  FIFTEEN of
   the twenty-six conditions hold in M3 because the extension their antecedent reads is
   EMPTY: c_dom_fwd, c_rng_fwd, c_sameAs_fwd, c_eqc_fwd, c_eqp_fwd, c_inv_fwd,
   c_sym_fwd, c_trp_fwd, c_svf, c_avf, c_hv, c_restr_IC, c_svf_typ, c_avf_typ and
   c_onp_typ.  Every one of them is discharged there by a bare "by simp" off the single
   fact that rdfs:domain, rdfs:range and every owl: term denote the junk element zz,
   whose extension is {}.  A condition satisfied that way is evidence that the condition
   set is consistent with having nothing to check, and nothing more.

   The same defect reaches further than the conditions.  Twelve of the twenty-seven arms
   of the built-in table rest on those fifteen conditions, so over M3 they are supported
   by a model that never fires them.

   M4 below is what exercises them.  Every one of the twenty-six conditions has a
   SATISFIED ANTECEDENT in it, stated as the block of lemmas M4_ante_*, and all fourteen
   derived monotonicity and subsumption lemmas of OO_Builtin_Sound fire at concrete
   elements of it, stated as M4_arm_* and collected in M4_exercises_every_derivation.
   M3 is kept, unchanged, because it is still the smallest thing that makes the point of
   section N3.

   THE CONSTRUCTION.  Three individuals Alice, Bob and Carl; seven classes; six ordinary
   properties; the schema and constructor vocabulary as thirty-four elements in all.
   The shape is forced far more than it is chosen.

     Alice has one PrP1-successor, Carl, and two PrP2-successors, Bob and Carl.  So
     IEXT(PrP1) is NON-EMPTY and a PROPER subset of IEXT(PrP2), and the subPropertyOf
     fact between them holds for a reason rather than for want of anything to check.
     ICEXT(ClY) is {Carl}: Carl is in it and Bob is not.

     ClC1 is the universal restriction on PrP1 with filler ClY, ClC2 the universal
     restriction on PrP2 with filler ClY, ClA2 the universal restriction on PrP2 with
     filler ClC1.  Table 5.6 then FORCES ICEXT(ClC1) and ICEXT(ClA2) to be the whole
     carrier and ICEXT(ClC2) to be the carrier without Alice, because Alice's second
     PrP2-successor Bob is outside ICEXT(ClY).  ClS1, ClS2 and ClS3 are the existential
     restrictions on PrP2 with filler ClY, on PrP2 with filler ClC1 and on PrP1 with
     filler ClY; Table 5.6 forces each of their class extensions to be {Alice}.

     ClS1 carries an owl:hasValue pair to Bob as well as its owl:someValuesFrom pair.
     That is legitimate and it is not an accident of drafting: RBS Table 5.6's rows are
     IF-THEN conditions on an element, not definitions of one element per class
     expression, so one element may satisfy several of them, and here both rows force the
     SAME class extension {Alice}.  Doing it this way saves an element, and the size of
     the carrier is what the proofs below cost.

     PrQ has the same extension as PrP1 and is a different element, which is what
     owl:equivalentProperty needs.  ClC1 and ClA2 have the same class extension and are
     different elements, which is what owl:equivalentClass needs; ClS1 and ClS2 are a
     second such pair.  PrR is the converse of PrP1, for owl:inverseOf.  PrSym is
     symmetric and not transitive, PrTrans is transitive and not symmetric, and each is
     typed accordingly, so neither of those two conditions is met by a relation that
     happens to satisfy both.

     IEXT(owl:sameAs) is the DIAGONAL Id on IR, and IR is the whole carrier.  See
     sameAs_is_exercised_only_on_the_diagonal below for why that is the only choice a
     conforming interpretation has and why it is nevertheless not vacuous.

     The four relations rdfs:subClassOf, rdfs:subPropertyOf, rdfs:domain and rdfs:range
     are then a FIXPOINT and not a free choice, because Table 5.8 is an iff in all four
     rows and because those four elements are themselves in IP and in the scope of the
     conditions they carry.  The tables Sco4, Spo4, Dm4 and Rg4 below are that fixpoint.
     Nothing in them can be adjusted without some condition below failing.

   WHAT M4 IS NOT.  It is not a conforming OWL 2 RDF-Based interpretation, and no
   theorem here says it is.  owl:Thing, rdf:Property and rdfs:Resource denote VJunk with
   an empty class extension, where RBS Table 5.2 requires ICEXT(owl:Thing) = IR,
   ICEXT(rdf:Property) = IP and ICEXT(rdfs:Resource) = IR; the axiomatic triple tables of
   RDF 1.1 Semantics sections 8 and 9 are absent; and owl:Restriction is kept out of IC,
   where Table 5.2's row reads "owl:Restriction | in IC | subset of IC" and so asserts
   the membership in its second column as well as the inclusion in its third.  That row
   was re-read in the raw HTML of the Recommendation on 15 September 2026, and the
   membership half of it is what M4 does not have.

   The same is true of owl:SymmetricProperty and owl:TransitiveProperty, whose Table 5.2
   rows read "in IC | subset of IP" and which M4 also keeps out of IC.  A draft of this
   paragraph said those two IRIs "do not appear in Table 5.2 at all", which is false: the
   table has a row for each, checked in the raw HTML on 15 September 2026, and the entry
   in the second column is the same membership owl:Restriction has.  The correction is
   recorded rather than silently applied, because a claim about a table that nobody
   re-reads is exactly the defect this file exists to remove.  What M4 does satisfy is
   the third column and Table 5.13, which is what c_sym_fwd and c_trp_fwd encode and all
   that any arm consumes.

   owl_rl_interp demands none of the rows M4 lacks, by DECISION M8 and the closing note
   of OO_Semantics, so M4 is a model of the class the soundness theorems quantify over.
   Closing the gap to a conforming interpretation means formalising Table 5.2 and the
   axiomatic triple tables first.  That is a different piece of work and it is not
   claimed here. *)

datatype E4 =
    Alice | Bob | Carl
  | ClY | ClC1 | ClC2 | ClA2 | ClS1 | ClS2 | ClS3
  | PrP1 | PrP2 | PrQ | PrR | PrSym | PrTrans
  | VType | VSco | VSpo | VDom | VRng
  | VAvf | VSvf | VOnp | VEqc | VEqp | VSame | VInv | VHas
  | VClass | VRestr | VSymC | VTransC
  | VJunk

definition Ty4 :: "(E4 \<times> E4) set" where
  "Ty4 = {(x,y). y = ClC1 \<or> y = ClA2
               \<or> (y = ClC2 \<and> x \<noteq> Alice)
               \<or> (y = ClY \<and> x = Carl)
               \<or> (y \<in> {ClS1, ClS2, ClS3} \<and> x = Alice)
               \<or> (y = VSymC \<and> x = PrSym)
               \<or> (y = VTransC \<and> x = PrTrans)
               \<or> (y = VClass \<and> x \<in> {ClY, ClC1, ClC2, ClA2, ClS1, ClS2, ClS3})
               \<or> (y = VRestr \<and> x \<in> {ClC1, ClC2, ClA2, ClS1, ClS2, ClS3})}"

definition Sco4 :: "(E4 \<times> E4) set" where
  "Sco4 = {(ClA2, ClA2), (ClA2, ClC1), (ClC1, ClA2), (ClC1, ClC1), (ClC2, ClA2), (ClC2,
     ClC1), (ClC2, ClC2), (ClS1, ClA2), (ClS1, ClC1), (ClS1, ClS1), (ClS1, ClS2), (ClS1,
     ClS3), (ClS2, ClA2), (ClS2, ClC1), (ClS2, ClS1), (ClS2, ClS2), (ClS2, ClS3), (ClS3,
     ClA2), (ClS3, ClC1), (ClS3, ClS1), (ClS3, ClS2), (ClS3, ClS3), (ClY, ClA2), (ClY, ClC1),
     (ClY, ClC2), (ClY, ClY)}"

definition Spo4 :: "(E4 \<times> E4) set" where
  "Spo4 = {(VAvf, VAvf), (VDom, VDom), (VEqc, VEqc), (VEqc, VSco), (VEqp, VEqp), (VEqp,
     VSpo), (VHas, VHas), (VInv, VInv), (VOnp, VOnp), (PrP1, PrP1), (PrP1, PrP2), (PrP1,
     PrQ), (PrP1, PrTrans), (PrP2, PrP2), (PrP2, PrTrans), (PrQ, PrP1), (PrQ, PrP2), (PrQ,
     PrQ), (PrQ, PrTrans), (PrR, PrR), (VRng, VRng), (VSame, VSame), (VSco, VSco), (VSpo,
     VSpo), (VSvf, VSvf), (PrSym, PrSym), (PrTrans, PrTrans), (VType, VType)}"

definition Dm4 :: "(E4 \<times> E4) set" where
  "Dm4 = {(VAvf, ClA2), (VAvf, ClC1), (VAvf, ClC2), (VDom, ClA2), (VDom, ClC1), (VDom, ClC2),
     (VEqc, ClA2), (VEqc, ClC1), (VEqc, ClC2), (VEqp, ClA2), (VEqp, ClC1), (VEqp, ClC2),
     (VHas, ClA2), (VHas, ClC1), (VHas, ClC2), (VInv, ClA2), (VInv, ClC1), (VInv, ClC2),
     (VOnp, ClA2), (VOnp, ClC1), (VOnp, ClC2), (PrP1, ClA2), (PrP1, ClC1), (PrP1, ClS1),
     (PrP1, ClS2), (PrP1, ClS3), (PrP2, ClA2), (PrP2, ClC1), (PrP2, ClS1), (PrP2, ClS2),
     (PrP2, ClS3), (PrQ, ClA2), (PrQ, ClC1), (PrQ, ClS1), (PrQ, ClS2), (PrQ, ClS3), (PrR,
     ClA2), (PrR, ClC1), (PrR, ClC2), (PrR, ClY), (VRng, ClA2), (VRng, ClC1), (VRng, ClC2),
     (VSame, ClA2), (VSame, ClC1), (VSco, ClA2), (VSco, ClC1), (VSco, ClC2), (VSpo, ClA2),
     (VSpo, ClC1), (VSpo, ClC2), (VSvf, ClA2), (VSvf, ClC1), (VSvf, ClC2), (PrSym, ClA2),
     (PrSym, ClC1), (PrSym, ClC2), (PrTrans, ClA2), (PrTrans, ClC1), (VType, ClA2), (VType,
     ClC1)}"

definition Rg4 :: "(E4 \<times> E4) set" where
  "Rg4 = {(VAvf, ClA2), (VAvf, ClC1), (VAvf, ClC2), (VDom, ClA2), (VDom, ClC1), (VDom, ClC2),
     (VEqc, ClA2), (VEqc, ClC1), (VEqc, ClC2), (VEqp, ClA2), (VEqp, ClC1), (VEqp, ClC2),
     (VHas, ClA2), (VHas, ClC1), (VHas, ClC2), (VInv, ClA2), (VInv, ClC1), (VInv, ClC2),
     (VOnp, ClA2), (VOnp, ClC1), (VOnp, ClC2), (PrP1, ClA2), (PrP1, ClC1), (PrP1, ClC2),
     (PrP1, ClY), (PrP2, ClA2), (PrP2, ClC1), (PrP2, ClC2), (PrQ, ClA2), (PrQ, ClC1), (PrQ,
     ClC2), (PrQ, ClY), (PrR, ClA2), (PrR, ClC1), (PrR, ClS1), (PrR, ClS2), (PrR, ClS3),
     (VRng, ClA2), (VRng, ClC1), (VRng, ClC2), (VSame, ClA2), (VSame, ClC1), (VSco, ClA2),
     (VSco, ClC1), (VSco, ClC2), (VSpo, ClA2), (VSpo, ClC1), (VSpo, ClC2), (VSvf, ClA2),
     (VSvf, ClC1), (VSvf, ClC2), (PrSym, ClA2), (PrSym, ClC1), (PrSym, ClC2), (PrTrans,
     ClA2), (PrTrans, ClC1), (PrTrans, ClC2), (VType, ClA2), (VType, ClC1), (VType, ClC2)}"

definition Avf4 :: "(E4 \<times> E4) set" where
  "Avf4 = {(ClA2, ClC1), (ClC1, ClY), (ClC2, ClY)}"

definition Svf4 :: "(E4 \<times> E4) set" where
  "Svf4 = {(ClS1, ClY), (ClS2, ClC1), (ClS3, ClY)}"

definition Onp4 :: "(E4 \<times> E4) set" where
  "Onp4 = {(ClA2, PrP2), (ClC1, PrP1), (ClC2, PrP2), (ClS1, PrP2), (ClS2, PrP2), (ClS3,
     PrP1)}"

definition Eqc4 :: "(E4 \<times> E4) set" where
  "Eqc4 = {(ClA2, ClC1), (ClC1, ClA2), (ClS1, ClS2), (ClS2, ClS1)}"

definition Eqp4 :: "(E4 \<times> E4) set" where
  "Eqp4 = {(PrP1, PrQ), (PrQ, PrP1)}"

definition Inv4 :: "(E4 \<times> E4) set" where
  "Inv4 = {(PrP1, PrR)}"

definition Hv4 :: "(E4 \<times> E4) set" where
  "Hv4 = {(ClS1, Bob)}"

definition P1_4 :: "(E4 \<times> E4) set" where
  "P1_4 = {(Alice, Carl)}"

definition P2_4 :: "(E4 \<times> E4) set" where
  "P2_4 = {(Alice, Bob), (Alice, Carl)}"

definition Q4 :: "(E4 \<times> E4) set" where
  "Q4 = {(Alice, Carl)}"

definition R4 :: "(E4 \<times> E4) set" where
  "R4 = {(Carl, Alice)}"

definition Sy4 :: "(E4 \<times> E4) set" where
  "Sy4 = {(Bob, Carl), (Carl, Bob)}"

definition Tr4 :: "(E4 \<times> E4) set" where
  "Tr4 = {(Alice, Bob), (Alice, Carl), (Bob, Carl)}"

primrec ext4 :: "E4 \<Rightarrow> (E4 \<times> E4) set" where
  "ext4 Alice = {}"
| "ext4 Bob = {}"
| "ext4 Carl = {}"
| "ext4 ClY = {}"
| "ext4 ClC1 = {}"
| "ext4 ClC2 = {}"
| "ext4 ClA2 = {}"
| "ext4 ClS1 = {}"
| "ext4 ClS2 = {}"
| "ext4 ClS3 = {}"
| "ext4 PrP1 = P1_4"
| "ext4 PrP2 = P2_4"
| "ext4 PrQ = Q4"
| "ext4 PrR = R4"
| "ext4 PrSym = Sy4"
| "ext4 PrTrans = Tr4"
| "ext4 VType = Ty4"
| "ext4 VSco = Sco4"
| "ext4 VSpo = Spo4"
| "ext4 VDom = Dm4"
| "ext4 VRng = Rg4"
| "ext4 VAvf = Avf4"
| "ext4 VSvf = Svf4"
| "ext4 VOnp = Onp4"
| "ext4 VEqc = Eqc4"
| "ext4 VEqp = Eqp4"
| "ext4 VSame = Id"
| "ext4 VInv = Inv4"
| "ext4 VHas = Hv4"
| "ext4 VClass = {}"
| "ext4 VRestr = {}"
| "ext4 VSymC = {}"
| "ext4 VTransC = {}"
| "ext4 VJunk = {}"

definition IP4 :: "E4 set" where
  "IP4 = {VAvf, VDom, VEqc, VEqp, VHas, VInv, VOnp, PrP1, PrP2, PrQ, PrR, VRng, VSame, VSco,
     VSpo, VSvf, PrSym, PrTrans, VType}"

definition M4 :: "E4 interp" where
  "M4 = \<lparr> IR = UNIV, IP = IP4, IEXT = ext4,
          IS = (\<lambda>u. if u = ex_a then Carl
                    else if u = ex_A then ClY
                    else if u = ex_B then ClC2
                    else if u = rdf_type then VType
                    else if u = rdfs_subClassOf then VSco
                    else if u = rdfs_subPropertyOf then VSpo
                    else if u = rdfs_domain then VDom
                    else if u = rdfs_range then VRng
                    else if u = rdfs_Class then VClass
                    else if u = owl_Restriction then VRestr
                    else if u = owl_onProperty then VOnp
                    else if u = owl_someValuesFrom then VSvf
                    else if u = owl_allValuesFrom then VAvf
                    else if u = owl_hasValue then VHas
                    else if u = owl_equivalentClass then VEqc
                    else if u = owl_equivalentProperty then VEqp
                    else if u = owl_sameAs then VSame
                    else if u = owl_inverseOf then VInv
                    else if u = owl_SymmetricProperty then VSymC
                    else if u = owl_TransitiveProperty then VTransC
                    else VJunk),
          IB = (\<lambda>b. VJunk),
          IL = (\<lambda>l. None) \<rparr>"

lemma M4_IR [simp]: "IR M4 = UNIV" by (simp add: M4_def)
lemma M4_IP [simp]: "IP M4 = IP4" by (simp add: M4_def)
lemma M4_IEXT [simp]: "IEXT M4 x = ext4 x" by (simp add: M4_def)
lemma M4_IB [simp]: "IB M4 b = VJunk" by (simp add: M4_def)
lemma M4_IL [simp]: "IL M4 l = None" by (simp add: M4_def)

lemma M4_IS [simp]:
  "IS M4 ex_a = Carl" "IS M4 ex_A = ClY" "IS M4 ex_B = ClC2"
  "IS M4 rdf_type = VType" "IS M4 rdfs_subClassOf = VSco" "IS M4 rdfs_subPropertyOf = VSpo"
  "IS M4 rdfs_domain = VDom" "IS M4 rdfs_range = VRng" "IS M4 rdfs_Class = VClass"
  "IS M4 owl_Restriction = VRestr" "IS M4 owl_onProperty = VOnp" "IS M4 owl_someValuesFrom = VSvf"
  "IS M4 owl_allValuesFrom = VAvf" "IS M4 owl_hasValue = VHas" "IS M4 owl_equivalentClass = VEqc"
  "IS M4 owl_equivalentProperty = VEqp" "IS M4 owl_sameAs = VSame" "IS M4 owl_inverseOf = VInv"
  "IS M4 owl_SymmetricProperty = VSymC" "IS M4 owl_TransitiveProperty = VTransC" "IS M4 ex_C = VJunk"
  by (simp_all add: M4_def ex_a_def ex_A_def ex_B_def ex_C_def rdf_type_def rdfs_subClassOf_def rdfs_subPropertyOf_def rdfs_domain_def rdfs_range_def rdfs_Class_def owl_Restriction_def owl_onProperty_def owl_someValuesFrom_def owl_allValuesFrom_def owl_hasValue_def owl_equivalentClass_def owl_equivalentProperty_def owl_sameAs_def owl_inverseOf_def owl_SymmetricProperty_def owl_TransitiveProperty_def)

lemma M4_ICEXT_raw: "ICEXT M4 y = {x. (x,y) \<in> Ty4}"
  by (simp add: ICEXT_def)

lemma M4_ICEXT [simp]:
  "ICEXT M4 Alice = {}"
  "ICEXT M4 Bob = {}"
  "ICEXT M4 Carl = {}"
  "ICEXT M4 ClY = {Carl}"
  "ICEXT M4 ClC1 = UNIV"
  "ICEXT M4 ClC2 = - {Alice}"
  "ICEXT M4 ClA2 = UNIV"
  "ICEXT M4 ClS1 = {Alice}"
  "ICEXT M4 ClS2 = {Alice}"
  "ICEXT M4 ClS3 = {Alice}"
  "ICEXT M4 PrP1 = {}"
  "ICEXT M4 PrP2 = {}"
  "ICEXT M4 PrQ = {}"
  "ICEXT M4 PrR = {}"
  "ICEXT M4 PrSym = {}"
  "ICEXT M4 PrTrans = {}"
  "ICEXT M4 VType = {}"
  "ICEXT M4 VSco = {}"
  "ICEXT M4 VSpo = {}"
  "ICEXT M4 VDom = {}"
  "ICEXT M4 VRng = {}"
  "ICEXT M4 VAvf = {}"
  "ICEXT M4 VSvf = {}"
  "ICEXT M4 VOnp = {}"
  "ICEXT M4 VEqc = {}"
  "ICEXT M4 VEqp = {}"
  "ICEXT M4 VSame = {}"
  "ICEXT M4 VInv = {}"
  "ICEXT M4 VHas = {}"
  "ICEXT M4 VClass = {ClA2, ClC1, ClC2, ClS1, ClS2, ClS3, ClY}"
  "ICEXT M4 VRestr = {ClA2, ClC1, ClC2, ClS1, ClS2, ClS3}"
  "ICEXT M4 VSymC = {PrSym}"
  "ICEXT M4 VTransC = {PrTrans}"
  "ICEXT M4 VJunk = {}"
  by (auto simp: M4_ICEXT_raw Ty4_def)

lemma M4_IC [simp]: "IC M4 = {ClA2, ClC1, ClC2, ClS1, ClS2, ClS3, ClY}"
  by (simp add: IC_def)

(* every enumerated table in one bundle, for the case analyses below *)
lemmas M4_tables = Ty4_def Sco4_def Spo4_def Dm4_def Rg4_def Avf4_def Svf4_def Onp4_def Eqc4_def Eqp4_def Inv4_def Hv4_def P1_4_def P2_4_def Q4_def R4_def Sy4_def Tr4_def IP4_def

subsection \<open>Each of the 26 conditions over M4, discharged separately\<close>

lemma M4_type_IP: "c_type_IP M4" by (simp add: c_type_IP_def IP4_def)
lemma M4_sco_IP:  "c_sco_IP M4"  by (simp add: c_sco_IP_def IP4_def)
lemma M4_spo_IP:  "c_spo_IP M4"  by (simp add: c_spo_IP_def IP4_def)

lemma M4_sco_fwd: "c_sco_fwd M4"
  unfolding c_sco_fwd_def by (auto simp: Sco4_def)

lemma M4_sco_trans: "c_sco_trans M4"
  unfolding c_sco_trans_def by (auto simp: Sco4_def)

lemma M4_spo_fwd: "c_spo_fwd M4"
  unfolding c_spo_fwd_def by (auto simp: M4_tables)

lemma M4_spo_trans: "c_spo_trans M4"
  unfolding c_spo_trans_def by (auto simp: Spo4_def)

lemma M4_dom_fwd: "c_dom_fwd M4"
  unfolding c_dom_fwd_def by (auto simp: M4_tables)

lemma M4_rng_fwd: "c_rng_fwd M4"
  unfolding c_rng_fwd_def by (auto simp: M4_tables)

(* RBS Table 5.9 row 1 is an iff whose right-hand side is a1 = a2, so the extension of
   owl:sameAs in a conforming interpretation is exactly the diagonal on IR.  Id is that
   diagonal, since IR M4 is the whole carrier. *)
lemma M4_sameAs_fwd: "c_sameAs_fwd M4" unfolding c_sameAs_fwd_def by simp

lemma M4_eqc_fwd: "c_eqc_fwd M4"
  unfolding c_eqc_fwd_def by (auto simp: Eqc4_def)

lemma M4_eqp_fwd: "c_eqp_fwd M4"
  unfolding c_eqp_fwd_def by (auto simp: Eqp4_def P1_4_def Q4_def IP4_def)

lemma M4_inv_fwd: "c_inv_fwd M4"
  unfolding c_inv_fwd_def by (auto simp: Inv4_def P1_4_def R4_def IP4_def)

lemma M4_sym_fwd: "c_sym_fwd M4"
  unfolding c_sym_fwd_def by (auto simp: Sy4_def)

lemma M4_trp_fwd: "c_trp_fwd M4"
  unfolding c_trp_fwd_def by (auto simp: Tr4_def)

(* The three rows of RBS Table 5.6.  Each class extension on the left is FORCED by the
   row, and the proof is the check that the value written into Ty4 is the forced one. *)
lemma M4_svf: "c_svf M4"
  unfolding c_svf_def by (auto simp: Svf4_def Onp4_def P1_4_def P2_4_def)

lemma M4_avf: "c_avf M4"
  unfolding c_avf_def by (auto simp: Avf4_def Onp4_def P1_4_def P2_4_def)

lemma M4_hv: "c_hv M4"
  unfolding c_hv_def by (auto simp: Hv4_def Onp4_def P2_4_def)

lemma M4_restr_IC: "c_restr_IC M4" unfolding c_restr_IC_def by simp

lemma M4_svf_typ: "c_svf_typ M4"
  unfolding c_svf_typ_def by (auto simp: Svf4_def)

lemma M4_avf_typ: "c_avf_typ M4"
  unfolding c_avf_typ_def by (auto simp: Avf4_def)

lemma M4_onp_typ: "c_onp_typ M4"
  unfolding c_onp_typ_def by (auto simp: Onp4_def IP4_def)

(* The four BACKWARD halves of RBS Table 5.8.  These are the expensive ones, because
   each quantifies over IC or IP twice and demands a set inclusion, and with nineteen
   properties in IP a blind case analysis is 361 inclusion tests over tables of up to a
   hundred and twenty pairs.  The proofs below do it in nineteen cases instead: from the
   inclusion hypothesis they take ONE pair (or, where one does not separate enough, two)
   out of the source's own table, push it through the hypothesis, and let the resulting
   membership decide which targets are possible.  The witnesses are not guesses; they
   were computed so that the set of targets they admit is EXACTLY the row of the table
   being proved, which is why each case closes with a single auto. *)
lemma M4_sco_bwd: "c_sco_bwd M4"
  unfolding c_sco_bwd_def
proof (intro allI impI)
  fix c1 c2 :: E4
  assume a1: "c1 \<in> IC M4" and a2: "c2 \<in> IC M4"
    and sub: "ICEXT M4 c1 \<subseteq> ICEXT M4 c2"
  from a1 consider "c1 = ClA2"
    | "c1 = ClC1"
    | "c1 = ClC2"
    | "c1 = ClS1"
    | "c1 = ClS2"
    | "c1 = ClS3"
    | "c1 = ClY"
    by (auto simp: IC_def)
  then show "(c1, c2) \<in> IEXT M4 (IS M4 rdfs_subClassOf)"
  proof cases
    case 1
    from 1 sub have s: "ICEXT M4 ClA2 \<subseteq> ICEXT M4 c2" by simp
    from s have w1: "Alice \<in> ICEXT M4 c2" by auto
    from s have w2: "VAvf \<in> ICEXT M4 c2" by auto
    show ?thesis using 1 a2 w1 w2 by (auto simp: Sco4_def)
  next
    case 2
    from 2 sub have s: "ICEXT M4 ClC1 \<subseteq> ICEXT M4 c2" by simp
    from s have w1: "Alice \<in> ICEXT M4 c2" by auto
    from s have w2: "VAvf \<in> ICEXT M4 c2" by auto
    show ?thesis using 2 a2 w1 w2 by (auto simp: Sco4_def)
  next
    case 3
    from 3 sub have s: "ICEXT M4 ClC2 \<subseteq> ICEXT M4 c2" by simp
    from s have w1: "VAvf \<in> ICEXT M4 c2" by auto
    show ?thesis using 3 a2 w1 by (auto simp: Sco4_def)
  next
    case 4
    from 4 sub have s: "ICEXT M4 ClS1 \<subseteq> ICEXT M4 c2" by simp
    from s have w1: "Alice \<in> ICEXT M4 c2" by auto
    show ?thesis using 4 a2 w1 by (auto simp: Sco4_def)
  next
    case 5
    from 5 sub have s: "ICEXT M4 ClS2 \<subseteq> ICEXT M4 c2" by simp
    from s have w1: "Alice \<in> ICEXT M4 c2" by auto
    show ?thesis using 5 a2 w1 by (auto simp: Sco4_def)
  next
    case 6
    from 6 sub have s: "ICEXT M4 ClS3 \<subseteq> ICEXT M4 c2" by simp
    from s have w1: "Alice \<in> ICEXT M4 c2" by auto
    show ?thesis using 6 a2 w1 by (auto simp: Sco4_def)
  next
    case 7
    from 7 sub have s: "ICEXT M4 ClY \<subseteq> ICEXT M4 c2" by simp
    from s have w1: "Carl \<in> ICEXT M4 c2" by auto
    show ?thesis using 7 a2 w1 by (auto simp: Sco4_def)
  qed
qed

lemma M4_spo_bwd: "c_spo_bwd M4"
  unfolding c_spo_bwd_def
proof (intro allI impI)
  fix p1 p2 :: E4
  assume a1: "p1 \<in> IP M4" and a2: "p2 \<in> IP M4"
    and sub: "IEXT M4 p1 \<subseteq> IEXT M4 p2"
  from a1 consider "p1 = VAvf"
    | "p1 = VDom"
    | "p1 = VEqc"
    | "p1 = VEqp"
    | "p1 = VHas"
    | "p1 = VInv"
    | "p1 = VOnp"
    | "p1 = PrP1"
    | "p1 = PrP2"
    | "p1 = PrQ"
    | "p1 = PrR"
    | "p1 = VRng"
    | "p1 = VSame"
    | "p1 = VSco"
    | "p1 = VSpo"
    | "p1 = VSvf"
    | "p1 = PrSym"
    | "p1 = PrTrans"
    | "p1 = VType"
    by (auto simp: IP4_def)
  then show "(p1, p2) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf)"
  proof cases
    case 1
    from 1 sub have s: "ext4 VAvf \<subseteq> ext4 p2" by simp
    have m1: "(ClC1, ClY) \<in> ext4 VAvf" by (simp add: Avf4_def)
    from s m1 have w1: "(ClC1, ClY) \<in> ext4 p2" by blast
    show ?thesis using 1 a2 w1 by (auto simp: M4_tables)
  next
    case 2
    from 2 sub have s: "ext4 VDom \<subseteq> ext4 p2" by simp
    have m1: "(PrP1, ClS1) \<in> ext4 VDom" by (simp add: Dm4_def)
    from s m1 have w1: "(PrP1, ClS1) \<in> ext4 p2" by blast
    show ?thesis using 2 a2 w1 by (auto simp: M4_tables)
  next
    case 3
    from 3 sub have s: "ext4 VEqc \<subseteq> ext4 p2" by simp
    have m1: "(ClS1, ClS2) \<in> ext4 VEqc" by (simp add: Eqc4_def)
    from s m1 have w1: "(ClS1, ClS2) \<in> ext4 p2" by blast
    show ?thesis using 3 a2 w1 by (auto simp: M4_tables)
  next
    case 4
    from 4 sub have s: "ext4 VEqp \<subseteq> ext4 p2" by simp
    have m1: "(PrP1, PrQ) \<in> ext4 VEqp" by (simp add: Eqp4_def)
    from s m1 have w1: "(PrP1, PrQ) \<in> ext4 p2" by blast
    show ?thesis using 4 a2 w1 by (auto simp: M4_tables)
  next
    case 5
    from 5 sub have s: "ext4 VHas \<subseteq> ext4 p2" by simp
    have m1: "(ClS1, Bob) \<in> ext4 VHas" by (simp add: Hv4_def)
    from s m1 have w1: "(ClS1, Bob) \<in> ext4 p2" by blast
    show ?thesis using 5 a2 w1 by (auto simp: M4_tables)
  next
    case 6
    from 6 sub have s: "ext4 VInv \<subseteq> ext4 p2" by simp
    have m1: "(PrP1, PrR) \<in> ext4 VInv" by (simp add: Inv4_def)
    from s m1 have w1: "(PrP1, PrR) \<in> ext4 p2" by blast
    show ?thesis using 6 a2 w1 by (auto simp: M4_tables)
  next
    case 7
    from 7 sub have s: "ext4 VOnp \<subseteq> ext4 p2" by simp
    have m1: "(ClA2, PrP2) \<in> ext4 VOnp" by (simp add: Onp4_def)
    from s m1 have w1: "(ClA2, PrP2) \<in> ext4 p2" by blast
    show ?thesis using 7 a2 w1 by (auto simp: M4_tables)
  next
    case 8
    from 8 sub have s: "ext4 PrP1 \<subseteq> ext4 p2" by simp
    have m1: "(Alice, Carl) \<in> ext4 PrP1" by (simp add: P1_4_def)
    from s m1 have w1: "(Alice, Carl) \<in> ext4 p2" by blast
    show ?thesis using 8 a2 w1 by (auto simp: M4_tables)
  next
    case 9
    from 9 sub have s: "ext4 PrP2 \<subseteq> ext4 p2" by simp
    have m1: "(Alice, Bob) \<in> ext4 PrP2" by (simp add: P2_4_def)
    from s m1 have w1: "(Alice, Bob) \<in> ext4 p2" by blast
    show ?thesis using 9 a2 w1 by (auto simp: M4_tables)
  next
    case 10
    from 10 sub have s: "ext4 PrQ \<subseteq> ext4 p2" by simp
    have m1: "(Alice, Carl) \<in> ext4 PrQ" by (simp add: Q4_def)
    from s m1 have w1: "(Alice, Carl) \<in> ext4 p2" by blast
    show ?thesis using 10 a2 w1 by (auto simp: M4_tables)
  next
    case 11
    from 11 sub have s: "ext4 PrR \<subseteq> ext4 p2" by simp
    have m1: "(Carl, Alice) \<in> ext4 PrR" by (simp add: R4_def)
    from s m1 have w1: "(Carl, Alice) \<in> ext4 p2" by blast
    show ?thesis using 11 a2 w1 by (auto simp: M4_tables)
  next
    case 12
    from 12 sub have s: "ext4 VRng \<subseteq> ext4 p2" by simp
    have m1: "(PrP1, ClY) \<in> ext4 VRng" by (simp add: Rg4_def)
    from s m1 have w1: "(PrP1, ClY) \<in> ext4 p2" by blast
    show ?thesis using 12 a2 w1 by (auto simp: M4_tables)
  next
    case 13
    from 13 sub have s: "ext4 VSame \<subseteq> ext4 p2" by simp
    have m1: "(Alice, Alice) \<in> ext4 VSame" by (simp)
    from s m1 have w1: "(Alice, Alice) \<in> ext4 p2" by blast
    show ?thesis using 13 a2 w1 by (auto simp: M4_tables)
  next
    case 14
    from 14 sub have s: "ext4 VSco \<subseteq> ext4 p2" by simp
    have m1: "(ClS1, ClS3) \<in> ext4 VSco" by (simp add: Sco4_def)
    from s m1 have w1: "(ClS1, ClS3) \<in> ext4 p2" by blast
    show ?thesis using 14 a2 w1 by (auto simp: M4_tables)
  next
    case 15
    from 15 sub have s: "ext4 VSpo \<subseteq> ext4 p2" by simp
    have m1: "(VEqc, VSco) \<in> ext4 VSpo" by (simp add: Spo4_def)
    from s m1 have w1: "(VEqc, VSco) \<in> ext4 p2" by blast
    show ?thesis using 15 a2 w1 by (auto simp: M4_tables)
  next
    case 16
    from 16 sub have s: "ext4 VSvf \<subseteq> ext4 p2" by simp
    have m1: "(ClS1, ClY) \<in> ext4 VSvf" by (simp add: Svf4_def)
    from s m1 have w1: "(ClS1, ClY) \<in> ext4 p2" by blast
    show ?thesis using 16 a2 w1 by (auto simp: M4_tables)
  next
    case 17
    from 17 sub have s: "ext4 PrSym \<subseteq> ext4 p2" by simp
    have m1: "(Carl, Bob) \<in> ext4 PrSym" by (simp add: Sy4_def)
    from s m1 have w1: "(Carl, Bob) \<in> ext4 p2" by blast
    show ?thesis using 17 a2 w1 by (auto simp: M4_tables)
  next
    case 18
    from 18 sub have s: "ext4 PrTrans \<subseteq> ext4 p2" by simp
    have m1: "(Alice, Bob) \<in> ext4 PrTrans" by (simp add: Tr4_def)
    from s m1 have w1: "(Alice, Bob) \<in> ext4 p2" by blast
    have m2: "(Bob, Carl) \<in> ext4 PrTrans" by (simp add: Tr4_def)
    from s m2 have w2: "(Bob, Carl) \<in> ext4 p2" by blast
    show ?thesis using 18 a2 w1 w2 by (auto simp: M4_tables)
  next
    case 19
    from 19 sub have s: "ext4 VType \<subseteq> ext4 p2" by simp
    have m1: "(Alice, ClA2) \<in> ext4 VType" by (simp add: Ty4_def)
    from s m1 have w1: "(Alice, ClA2) \<in> ext4 p2" by blast
    show ?thesis using 19 a2 w1 by (auto simp: M4_tables)
  qed
qed

lemma M4_dom_bwd: "c_dom_bwd M4"
  unfolding c_dom_bwd_def
proof (intro allI impI)
  fix p c :: E4
  assume a1: "p \<in> IP M4" and a2: "c \<in> IC M4"
    and h: "\<forall>x y. (x,y) \<in> IEXT M4 p \<longrightarrow> x \<in> ICEXT M4 c"
  from a1 consider "p = VAvf"
    | "p = VDom"
    | "p = VEqc"
    | "p = VEqp"
    | "p = VHas"
    | "p = VInv"
    | "p = VOnp"
    | "p = PrP1"
    | "p = PrP2"
    | "p = PrQ"
    | "p = PrR"
    | "p = VRng"
    | "p = VSame"
    | "p = VSco"
    | "p = VSpo"
    | "p = VSvf"
    | "p = PrSym"
    | "p = PrTrans"
    | "p = VType"
    by (auto simp: IP4_def)
  then show "(p, c) \<in> IEXT M4 (IS M4 rdfs_domain)"
  proof cases
    case 1
    from 1 h have h1: "\<forall>x y. (x,y) \<in> ext4 VAvf \<longrightarrow> x \<in> ICEXT M4 c"
      by simp
    have m1: "(ClA2, ClC1) \<in> ext4 VAvf" by (simp add: Avf4_def)
    from h1 m1 have w1: "ClA2 \<in> ICEXT M4 c" by blast
    show ?thesis using 1 a2 w1 by (auto simp: Dm4_def)
  next
    case 2
    from 2 h have h1: "\<forall>x y. (x,y) \<in> ext4 VDom \<longrightarrow> x \<in> ICEXT M4 c"
      by simp
    have m1: "(VAvf, ClA2) \<in> ext4 VDom" by (simp add: Dm4_def)
    from h1 m1 have w1: "VAvf \<in> ICEXT M4 c" by blast
    show ?thesis using 2 a2 w1 by (auto simp: Dm4_def)
  next
    case 3
    from 3 h have h1: "\<forall>x y. (x,y) \<in> ext4 VEqc \<longrightarrow> x \<in> ICEXT M4 c"
      by simp
    have m1: "(ClA2, ClC1) \<in> ext4 VEqc" by (simp add: Eqc4_def)
    from h1 m1 have w1: "ClA2 \<in> ICEXT M4 c" by blast
    show ?thesis using 3 a2 w1 by (auto simp: Dm4_def)
  next
    case 4
    from 4 h have h1: "\<forall>x y. (x,y) \<in> ext4 VEqp \<longrightarrow> x \<in> ICEXT M4 c"
      by simp
    have m1: "(PrP1, PrQ) \<in> ext4 VEqp" by (simp add: Eqp4_def)
    from h1 m1 have w1: "PrP1 \<in> ICEXT M4 c" by blast
    show ?thesis using 4 a2 w1 by (auto simp: Dm4_def)
  next
    case 5
    from 5 h have h1: "\<forall>x y. (x,y) \<in> ext4 VHas \<longrightarrow> x \<in> ICEXT M4 c"
      by simp
    have m1: "(ClS1, Bob) \<in> ext4 VHas" by (simp add: Hv4_def)
    from h1 m1 have w1: "ClS1 \<in> ICEXT M4 c" by blast
    show ?thesis using 5 a2 w1 by (auto simp: Dm4_def)
  next
    case 6
    from 6 h have h1: "\<forall>x y. (x,y) \<in> ext4 VInv \<longrightarrow> x \<in> ICEXT M4 c"
      by simp
    have m1: "(PrP1, PrR) \<in> ext4 VInv" by (simp add: Inv4_def)
    from h1 m1 have w1: "PrP1 \<in> ICEXT M4 c" by blast
    show ?thesis using 6 a2 w1 by (auto simp: Dm4_def)
  next
    case 7
    from 7 h have h1: "\<forall>x y. (x,y) \<in> ext4 VOnp \<longrightarrow> x \<in> ICEXT M4 c"
      by simp
    have m1: "(ClA2, PrP2) \<in> ext4 VOnp" by (simp add: Onp4_def)
    from h1 m1 have w1: "ClA2 \<in> ICEXT M4 c" by blast
    show ?thesis using 7 a2 w1 by (auto simp: Dm4_def)
  next
    case 8
    from 8 h have h1: "\<forall>x y. (x,y) \<in> ext4 PrP1 \<longrightarrow> x \<in> ICEXT M4 c"
      by simp
    have m1: "(Alice, Carl) \<in> ext4 PrP1" by (simp add: P1_4_def)
    from h1 m1 have w1: "Alice \<in> ICEXT M4 c" by blast
    show ?thesis using 8 a2 w1 by (auto simp: Dm4_def)
  next
    case 9
    from 9 h have h1: "\<forall>x y. (x,y) \<in> ext4 PrP2 \<longrightarrow> x \<in> ICEXT M4 c"
      by simp
    have m1: "(Alice, Bob) \<in> ext4 PrP2" by (simp add: P2_4_def)
    from h1 m1 have w1: "Alice \<in> ICEXT M4 c" by blast
    show ?thesis using 9 a2 w1 by (auto simp: Dm4_def)
  next
    case 10
    from 10 h have h1: "\<forall>x y. (x,y) \<in> ext4 PrQ \<longrightarrow> x \<in> ICEXT M4 c"
      by simp
    have m1: "(Alice, Carl) \<in> ext4 PrQ" by (simp add: Q4_def)
    from h1 m1 have w1: "Alice \<in> ICEXT M4 c" by blast
    show ?thesis using 10 a2 w1 by (auto simp: Dm4_def)
  next
    case 11
    from 11 h have h1: "\<forall>x y. (x,y) \<in> ext4 PrR \<longrightarrow> x \<in> ICEXT M4 c"
      by simp
    have m1: "(Carl, Alice) \<in> ext4 PrR" by (simp add: R4_def)
    from h1 m1 have w1: "Carl \<in> ICEXT M4 c" by blast
    show ?thesis using 11 a2 w1 by (auto simp: Dm4_def)
  next
    case 12
    from 12 h have h1: "\<forall>x y. (x,y) \<in> ext4 VRng \<longrightarrow> x \<in> ICEXT M4 c"
      by simp
    have m1: "(VAvf, ClA2) \<in> ext4 VRng" by (simp add: Rg4_def)
    from h1 m1 have w1: "VAvf \<in> ICEXT M4 c" by blast
    show ?thesis using 12 a2 w1 by (auto simp: Dm4_def)
  next
    case 13
    from 13 h have h1: "\<forall>x y. (x,y) \<in> ext4 VSame \<longrightarrow> x \<in> ICEXT M4 c"
      by simp
    have m1: "(Alice, Alice) \<in> ext4 VSame" by (simp)
    from h1 m1 have w1: "Alice \<in> ICEXT M4 c" by blast
    have m2: "(VAvf, VAvf) \<in> ext4 VSame" by (simp)
    from h1 m2 have w2: "VAvf \<in> ICEXT M4 c" by blast
    show ?thesis using 13 a2 w1 w2 by (auto simp: Dm4_def)
  next
    case 14
    from 14 h have h1: "\<forall>x y. (x,y) \<in> ext4 VSco \<longrightarrow> x \<in> ICEXT M4 c"
      by simp
    have m1: "(ClA2, ClA2) \<in> ext4 VSco" by (simp add: Sco4_def)
    from h1 m1 have w1: "ClA2 \<in> ICEXT M4 c" by blast
    show ?thesis using 14 a2 w1 by (auto simp: Dm4_def)
  next
    case 15
    from 15 h have h1: "\<forall>x y. (x,y) \<in> ext4 VSpo \<longrightarrow> x \<in> ICEXT M4 c"
      by simp
    have m1: "(VAvf, VAvf) \<in> ext4 VSpo" by (simp add: Spo4_def)
    from h1 m1 have w1: "VAvf \<in> ICEXT M4 c" by blast
    show ?thesis using 15 a2 w1 by (auto simp: Dm4_def)
  next
    case 16
    from 16 h have h1: "\<forall>x y. (x,y) \<in> ext4 VSvf \<longrightarrow> x \<in> ICEXT M4 c"
      by simp
    have m1: "(ClS1, ClY) \<in> ext4 VSvf" by (simp add: Svf4_def)
    from h1 m1 have w1: "ClS1 \<in> ICEXT M4 c" by blast
    show ?thesis using 16 a2 w1 by (auto simp: Dm4_def)
  next
    case 17
    from 17 h have h1: "\<forall>x y. (x,y) \<in> ext4 PrSym \<longrightarrow> x \<in> ICEXT M4 c"
      by simp
    have m1: "(Bob, Carl) \<in> ext4 PrSym" by (simp add: Sy4_def)
    from h1 m1 have w1: "Bob \<in> ICEXT M4 c" by blast
    show ?thesis using 17 a2 w1 by (auto simp: Dm4_def)
  next
    case 18
    from 18 h have h1: "\<forall>x y. (x,y) \<in> ext4 PrTrans \<longrightarrow> x \<in> ICEXT M4 c"
      by simp
    have m1: "(Alice, Bob) \<in> ext4 PrTrans" by (simp add: Tr4_def)
    from h1 m1 have w1: "Alice \<in> ICEXT M4 c" by blast
    have m2: "(Bob, Carl) \<in> ext4 PrTrans" by (simp add: Tr4_def)
    from h1 m2 have w2: "Bob \<in> ICEXT M4 c" by blast
    show ?thesis using 18 a2 w1 w2 by (auto simp: Dm4_def)
  next
    case 19
    from 19 h have h1: "\<forall>x y. (x,y) \<in> ext4 VType \<longrightarrow> x \<in> ICEXT M4 c"
      by simp
    have m1: "(Alice, ClA2) \<in> ext4 VType" by (simp add: Ty4_def)
    from h1 m1 have w1: "Alice \<in> ICEXT M4 c" by blast
    have m2: "(VAvf, ClA2) \<in> ext4 VType" by (simp add: Ty4_def)
    from h1 m2 have w2: "VAvf \<in> ICEXT M4 c" by blast
    show ?thesis using 19 a2 w1 w2 by (auto simp: Dm4_def)
  qed
qed

lemma M4_rng_bwd: "c_rng_bwd M4"
  unfolding c_rng_bwd_def
proof (intro allI impI)
  fix p c :: E4
  assume a1: "p \<in> IP M4" and a2: "c \<in> IC M4"
    and h: "\<forall>x y. (x,y) \<in> IEXT M4 p \<longrightarrow> y \<in> ICEXT M4 c"
  from a1 consider "p = VAvf"
    | "p = VDom"
    | "p = VEqc"
    | "p = VEqp"
    | "p = VHas"
    | "p = VInv"
    | "p = VOnp"
    | "p = PrP1"
    | "p = PrP2"
    | "p = PrQ"
    | "p = PrR"
    | "p = VRng"
    | "p = VSame"
    | "p = VSco"
    | "p = VSpo"
    | "p = VSvf"
    | "p = PrSym"
    | "p = PrTrans"
    | "p = VType"
    by (auto simp: IP4_def)
  then show "(p, c) \<in> IEXT M4 (IS M4 rdfs_range)"
  proof cases
    case 1
    from 1 h have h1: "\<forall>x y. (x,y) \<in> ext4 VAvf \<longrightarrow> y \<in> ICEXT M4 c"
      by simp
    have m1: "(ClA2, ClC1) \<in> ext4 VAvf" by (simp add: Avf4_def)
    from h1 m1 have w1: "ClC1 \<in> ICEXT M4 c" by blast
    show ?thesis using 1 a2 w1 by (auto simp: Rg4_def)
  next
    case 2
    from 2 h have h1: "\<forall>x y. (x,y) \<in> ext4 VDom \<longrightarrow> y \<in> ICEXT M4 c"
      by simp
    have m1: "(VAvf, ClA2) \<in> ext4 VDom" by (simp add: Dm4_def)
    from h1 m1 have w1: "ClA2 \<in> ICEXT M4 c" by blast
    show ?thesis using 2 a2 w1 by (auto simp: Rg4_def)
  next
    case 3
    from 3 h have h1: "\<forall>x y. (x,y) \<in> ext4 VEqc \<longrightarrow> y \<in> ICEXT M4 c"
      by simp
    have m1: "(ClC1, ClA2) \<in> ext4 VEqc" by (simp add: Eqc4_def)
    from h1 m1 have w1: "ClA2 \<in> ICEXT M4 c" by blast
    show ?thesis using 3 a2 w1 by (auto simp: Rg4_def)
  next
    case 4
    from 4 h have h1: "\<forall>x y. (x,y) \<in> ext4 VEqp \<longrightarrow> y \<in> ICEXT M4 c"
      by simp
    have m1: "(PrQ, PrP1) \<in> ext4 VEqp" by (simp add: Eqp4_def)
    from h1 m1 have w1: "PrP1 \<in> ICEXT M4 c" by blast
    show ?thesis using 4 a2 w1 by (auto simp: Rg4_def)
  next
    case 5
    from 5 h have h1: "\<forall>x y. (x,y) \<in> ext4 VHas \<longrightarrow> y \<in> ICEXT M4 c"
      by simp
    have m1: "(ClS1, Bob) \<in> ext4 VHas" by (simp add: Hv4_def)
    from h1 m1 have w1: "Bob \<in> ICEXT M4 c" by blast
    show ?thesis using 5 a2 w1 by (auto simp: Rg4_def)
  next
    case 6
    from 6 h have h1: "\<forall>x y. (x,y) \<in> ext4 VInv \<longrightarrow> y \<in> ICEXT M4 c"
      by simp
    have m1: "(PrP1, PrR) \<in> ext4 VInv" by (simp add: Inv4_def)
    from h1 m1 have w1: "PrR \<in> ICEXT M4 c" by blast
    show ?thesis using 6 a2 w1 by (auto simp: Rg4_def)
  next
    case 7
    from 7 h have h1: "\<forall>x y. (x,y) \<in> ext4 VOnp \<longrightarrow> y \<in> ICEXT M4 c"
      by simp
    have m1: "(ClC1, PrP1) \<in> ext4 VOnp" by (simp add: Onp4_def)
    from h1 m1 have w1: "PrP1 \<in> ICEXT M4 c" by blast
    show ?thesis using 7 a2 w1 by (auto simp: Rg4_def)
  next
    case 8
    from 8 h have h1: "\<forall>x y. (x,y) \<in> ext4 PrP1 \<longrightarrow> y \<in> ICEXT M4 c"
      by simp
    have m1: "(Alice, Carl) \<in> ext4 PrP1" by (simp add: P1_4_def)
    from h1 m1 have w1: "Carl \<in> ICEXT M4 c" by blast
    show ?thesis using 8 a2 w1 by (auto simp: Rg4_def)
  next
    case 9
    from 9 h have h1: "\<forall>x y. (x,y) \<in> ext4 PrP2 \<longrightarrow> y \<in> ICEXT M4 c"
      by simp
    have m1: "(Alice, Bob) \<in> ext4 PrP2" by (simp add: P2_4_def)
    from h1 m1 have w1: "Bob \<in> ICEXT M4 c" by blast
    show ?thesis using 9 a2 w1 by (auto simp: Rg4_def)
  next
    case 10
    from 10 h have h1: "\<forall>x y. (x,y) \<in> ext4 PrQ \<longrightarrow> y \<in> ICEXT M4 c"
      by simp
    have m1: "(Alice, Carl) \<in> ext4 PrQ" by (simp add: Q4_def)
    from h1 m1 have w1: "Carl \<in> ICEXT M4 c" by blast
    show ?thesis using 10 a2 w1 by (auto simp: Rg4_def)
  next
    case 11
    from 11 h have h1: "\<forall>x y. (x,y) \<in> ext4 PrR \<longrightarrow> y \<in> ICEXT M4 c"
      by simp
    have m1: "(Carl, Alice) \<in> ext4 PrR" by (simp add: R4_def)
    from h1 m1 have w1: "Alice \<in> ICEXT M4 c" by blast
    show ?thesis using 11 a2 w1 by (auto simp: Rg4_def)
  next
    case 12
    from 12 h have h1: "\<forall>x y. (x,y) \<in> ext4 VRng \<longrightarrow> y \<in> ICEXT M4 c"
      by simp
    have m1: "(VAvf, ClA2) \<in> ext4 VRng" by (simp add: Rg4_def)
    from h1 m1 have w1: "ClA2 \<in> ICEXT M4 c" by blast
    show ?thesis using 12 a2 w1 by (auto simp: Rg4_def)
  next
    case 13
    from 13 h have h1: "\<forall>x y. (x,y) \<in> ext4 VSame \<longrightarrow> y \<in> ICEXT M4 c"
      by simp
    have m1: "(Alice, Alice) \<in> ext4 VSame" by (simp)
    from h1 m1 have w1: "Alice \<in> ICEXT M4 c" by blast
    have m2: "(VAvf, VAvf) \<in> ext4 VSame" by (simp)
    from h1 m2 have w2: "VAvf \<in> ICEXT M4 c" by blast
    show ?thesis using 13 a2 w1 w2 by (auto simp: Rg4_def)
  next
    case 14
    from 14 h have h1: "\<forall>x y. (x,y) \<in> ext4 VSco \<longrightarrow> y \<in> ICEXT M4 c"
      by simp
    have m1: "(ClA2, ClA2) \<in> ext4 VSco" by (simp add: Sco4_def)
    from h1 m1 have w1: "ClA2 \<in> ICEXT M4 c" by blast
    show ?thesis using 14 a2 w1 by (auto simp: Rg4_def)
  next
    case 15
    from 15 h have h1: "\<forall>x y. (x,y) \<in> ext4 VSpo \<longrightarrow> y \<in> ICEXT M4 c"
      by simp
    have m1: "(VAvf, VAvf) \<in> ext4 VSpo" by (simp add: Spo4_def)
    from h1 m1 have w1: "VAvf \<in> ICEXT M4 c" by blast
    show ?thesis using 15 a2 w1 by (auto simp: Rg4_def)
  next
    case 16
    from 16 h have h1: "\<forall>x y. (x,y) \<in> ext4 VSvf \<longrightarrow> y \<in> ICEXT M4 c"
      by simp
    have m1: "(ClS2, ClC1) \<in> ext4 VSvf" by (simp add: Svf4_def)
    from h1 m1 have w1: "ClC1 \<in> ICEXT M4 c" by blast
    show ?thesis using 16 a2 w1 by (auto simp: Rg4_def)
  next
    case 17
    from 17 h have h1: "\<forall>x y. (x,y) \<in> ext4 PrSym \<longrightarrow> y \<in> ICEXT M4 c"
      by simp
    have m1: "(Carl, Bob) \<in> ext4 PrSym" by (simp add: Sy4_def)
    from h1 m1 have w1: "Bob \<in> ICEXT M4 c" by blast
    show ?thesis using 17 a2 w1 by (auto simp: Rg4_def)
  next
    case 18
    from 18 h have h1: "\<forall>x y. (x,y) \<in> ext4 PrTrans \<longrightarrow> y \<in> ICEXT M4 c"
      by simp
    have m1: "(Alice, Bob) \<in> ext4 PrTrans" by (simp add: Tr4_def)
    from h1 m1 have w1: "Bob \<in> ICEXT M4 c" by blast
    show ?thesis using 18 a2 w1 by (auto simp: Rg4_def)
  next
    case 19
    from 19 h have h1: "\<forall>x y. (x,y) \<in> ext4 VType \<longrightarrow> y \<in> ICEXT M4 c"
      by simp
    have m1: "(Alice, ClA2) \<in> ext4 VType" by (simp add: Ty4_def)
    from h1 m1 have w1: "ClA2 \<in> ICEXT M4 c" by blast
    show ?thesis using 19 a2 w1 by (auto simp: Rg4_def)
  qed
qed


theorem M4_is_a_model: "owl_rl_interp M4"
  unfolding owl_rl_interp_def
  by (simp add: M4_type_IP M4_sco_IP M4_spo_IP M4_sco_fwd M4_sco_bwd
      M4_spo_fwd M4_spo_bwd M4_sco_trans M4_spo_trans
      M4_dom_fwd M4_dom_bwd M4_rng_fwd M4_rng_bwd
      M4_sameAs_fwd M4_eqc_fwd M4_eqp_fwd M4_inv_fwd M4_sym_fwd M4_trp_fwd
      M4_svf M4_avf M4_hv M4_restr_IC M4_svf_typ M4_avf_typ M4_onp_typ)

(* The Table 5.1 domain conditions, so M4 also witnesses satisfiability for the bridge
   theorem T4, exactly as M3 does. *)
theorem M4_wf: "wf_interp M4" by (simp add: wf_interp_def)

subsection \<open>Every condition has a SATISFIED ANTECEDENT\<close>

(* The point of this block.  owl_rl_interp M4 alone is worth about as much as
   owl_rl_interp W: a condition of the form "for all x in S, P x" is true when S is
   empty, and fifteen of the twenty-six are true in M3 for exactly that reason.  Each
   lemma below exhibits something IN the extension the corresponding condition reads, so
   that the condition is doing work.  The three IP conditions have no antecedent to
   satisfy, so what is exhibited for them is that the element they put in IP has a pair
   in its extension, which is the property that makes the membership consequential.

   Where a condition's antecedent is itself an inclusion that could hold for want of
   anything to check -- c_sco_bwd, c_spo_bwd, c_dom_bwd and c_rng_bwd -- the lemma
   exhibits the whole antecedent AND a member of the set being included, so that the
   inclusion is not the empty one. *)

lemma M4_ante_type_IP:
  "IS M4 rdf_type \<in> IP M4 \<and> (Carl, ClY) \<in> IEXT M4 (IS M4 rdf_type)"
  by (simp add: IP4_def Ty4_def)

lemma M4_ante_sco_IP:
  "IS M4 rdfs_subClassOf \<in> IP M4
     \<and> (ClY, ClC2) \<in> IEXT M4 (IS M4 rdfs_subClassOf)"
  by (simp add: IP4_def Sco4_def)

lemma M4_ante_spo_IP:
  "IS M4 rdfs_subPropertyOf \<in> IP M4
     \<and> (PrP1, PrP2) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf)"
  by (simp add: IP4_def Spo4_def)

lemma M4_ante_sco_fwd:
  "(ClY, ClC2) \<in> IEXT M4 (IS M4 rdfs_subClassOf) \<and> Carl \<in> ICEXT M4 ClY"
  by (simp add: Sco4_def)

lemma M4_ante_sco_bwd:
  "ClY \<in> IC M4 \<and> ClC2 \<in> IC M4 \<and> ICEXT M4 ClY \<subseteq> ICEXT M4 ClC2
     \<and> Carl \<in> ICEXT M4 ClY"
  by simp

lemma M4_ante_spo_fwd:
  "(PrP1, PrP2) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf)
     \<and> (Alice, Carl) \<in> IEXT M4 PrP1"
  by (simp add: Spo4_def P1_4_def)

lemma M4_ante_spo_bwd:
  "PrP1 \<in> IP M4 \<and> PrP2 \<in> IP M4 \<and> IEXT M4 PrP1 \<subseteq> IEXT M4 PrP2
     \<and> (Alice, Carl) \<in> IEXT M4 PrP1"
  by (simp add: IP4_def P1_4_def P2_4_def)

lemma M4_ante_sco_trans:
  "(ClY, ClC2) \<in> IEXT M4 (IS M4 rdfs_subClassOf)
     \<and> (ClC2, ClC1) \<in> IEXT M4 (IS M4 rdfs_subClassOf)
     \<and> ClY \<noteq> ClC2 \<and> ClC2 \<noteq> ClC1"
  by (simp add: Sco4_def)

lemma M4_ante_spo_trans:
  "(PrP1, PrQ) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf)
     \<and> (PrQ, PrP2) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf)
     \<and> PrP1 \<noteq> PrQ \<and> PrQ \<noteq> PrP2"
  by (simp add: Spo4_def)

lemma M4_ante_dom_fwd:
  "(PrP1, ClS1) \<in> IEXT M4 (IS M4 rdfs_domain)
     \<and> (Alice, Carl) \<in> IEXT M4 PrP1"
  by (simp add: Dm4_def P1_4_def)

lemma M4_ante_dom_bwd:
  "PrP1 \<in> IP M4 \<and> ClS1 \<in> IC M4
     \<and> (\<forall>x y. (x,y) \<in> IEXT M4 PrP1 \<longrightarrow> x \<in> ICEXT M4 ClS1)
     \<and> (Alice, Carl) \<in> IEXT M4 PrP1"
  by (simp add: IP4_def P1_4_def)

lemma M4_ante_rng_fwd:
  "(PrP1, ClY) \<in> IEXT M4 (IS M4 rdfs_range) \<and> (Alice, Carl) \<in> IEXT M4 PrP1"
  by (simp add: Rg4_def P1_4_def)

lemma M4_ante_rng_bwd:
  "PrP1 \<in> IP M4 \<and> ClY \<in> IC M4
     \<and> (\<forall>x y. (x,y) \<in> IEXT M4 PrP1 \<longrightarrow> y \<in> ICEXT M4 ClY)
     \<and> (Alice, Carl) \<in> IEXT M4 PrP1"
  by (simp add: IP4_def P1_4_def)

lemma M4_ante_sameAs_fwd: "(Alice, Alice) \<in> IEXT M4 (IS M4 owl_sameAs)"
  by simp

lemma M4_ante_eqc_fwd:
  "(ClS1, ClS2) \<in> IEXT M4 (IS M4 owl_equivalentClass) \<and> ClS1 \<noteq> ClS2
     \<and> Alice \<in> ICEXT M4 ClS1"
  by (simp add: Eqc4_def)

lemma M4_ante_eqp_fwd:
  "(PrP1, PrQ) \<in> IEXT M4 (IS M4 owl_equivalentProperty) \<and> PrP1 \<noteq> PrQ
     \<and> (Alice, Carl) \<in> IEXT M4 PrP1"
  by (simp add: Eqp4_def P1_4_def)

lemma M4_ante_inv_fwd:
  "(PrP1, PrR) \<in> IEXT M4 (IS M4 owl_inverseOf) \<and> PrP1 \<noteq> PrR
     \<and> (Alice, Carl) \<in> IEXT M4 PrP1 \<and> (Carl, Alice) \<in> IEXT M4 PrR"
  by (simp add: Inv4_def P1_4_def R4_def)

lemma M4_ante_sym_fwd:
  "PrSym \<in> ICEXT M4 (IS M4 owl_SymmetricProperty)
     \<and> (Bob, Carl) \<in> IEXT M4 PrSym \<and> (Carl, Bob) \<in> IEXT M4 PrSym"
  by (simp add: Sy4_def)

lemma M4_ante_trp_fwd:
  "PrTrans \<in> ICEXT M4 (IS M4 owl_TransitiveProperty)
     \<and> (Alice, Bob) \<in> IEXT M4 PrTrans \<and> (Bob, Carl) \<in> IEXT M4 PrTrans
     \<and> (Alice, Carl) \<in> IEXT M4 PrTrans"
  by (simp add: Tr4_def)

lemma M4_ante_svf:
  "(ClS1, ClY) \<in> IEXT M4 (IS M4 owl_someValuesFrom)
     \<and> (ClS1, PrP2) \<in> IEXT M4 (IS M4 owl_onProperty)
     \<and> Alice \<in> ICEXT M4 ClS1"
  by (simp add: Svf4_def Onp4_def)

lemma M4_ante_avf:
  "(ClC1, ClY) \<in> IEXT M4 (IS M4 owl_allValuesFrom)
     \<and> (ClC1, PrP1) \<in> IEXT M4 (IS M4 owl_onProperty)
     \<and> Alice \<in> ICEXT M4 ClC1"
  by (simp add: Avf4_def Onp4_def)

lemma M4_ante_hv:
  "(ClS1, Bob) \<in> IEXT M4 (IS M4 owl_hasValue)
     \<and> (ClS1, PrP2) \<in> IEXT M4 (IS M4 owl_onProperty)
     \<and> Alice \<in> ICEXT M4 ClS1"
  by (simp add: Hv4_def Onp4_def)

lemma M4_ante_restr_IC:
  "ClC1 \<in> ICEXT M4 (IS M4 owl_Restriction) \<and> ClC1 \<in> IC M4"
  by simp

lemma M4_ante_svf_typ: "(ClS1, ClY) \<in> IEXT M4 (IS M4 owl_someValuesFrom)"
  by (simp add: Svf4_def)

lemma M4_ante_avf_typ: "(ClC1, ClY) \<in> IEXT M4 (IS M4 owl_allValuesFrom)"
  by (simp add: Avf4_def)

lemma M4_ante_onp_typ: "(ClC1, PrP1) \<in> IEXT M4 (IS M4 owl_onProperty)"
  by (simp add: Onp4_def)

(* The twenty-six, in ONE conjunction, so that a later edit which hollows one of them
   out fails here rather than passing quietly.  Twenty-six is the number of conjuncts of
   owl_rl_interp, so neither list can drift from the other without this going red.

   WHY A CONJUNCTION AND NOT "and".  The first draft of this theorem used the multiple
   statement form, name: "A" and "B" and "C".  Isabelle proves every one of those, but
   it binds only the FIRST to the theorem's name, so @{thm name} in the ML gate of
   OO_Audit sees one conjunct and the other twenty-five are outside the oracle audit.
   Measured, not assumed: a probe reported length 1 for a thirteen-statement theorem.
   The same defect is present in M3_is_not_degenerate above, which is left as it is
   because M3 and everything that depends on it is kept unchanged; the audit therefore
   covers its first statement only.  Every collector written for M4 is a conjunction. *)
theorem M4_every_condition_has_a_live_antecedent:
  "(IS M4 rdf_type \<in> IP M4 \<and> (Carl, ClY) \<in> IEXT M4 (IS M4 rdf_type))
     \<and> (IS M4 rdfs_subClassOf \<in> IP M4
         \<and> (ClY, ClC2) \<in> IEXT M4 (IS M4 rdfs_subClassOf))
     \<and> (IS M4 rdfs_subPropertyOf \<in> IP M4
         \<and> (PrP1, PrP2) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf))
     \<and> ((ClY, ClC2) \<in> IEXT M4 (IS M4 rdfs_subClassOf) \<and> Carl \<in> ICEXT M4 ClY)
     \<and> (ClY \<in> IC M4 \<and> ClC2 \<in> IC M4 \<and> ICEXT M4 ClY \<subseteq> ICEXT M4 ClC2
         \<and> Carl \<in> ICEXT M4 ClY)
     \<and> ((PrP1, PrP2) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf)
         \<and> (Alice, Carl) \<in> IEXT M4 PrP1)
     \<and> (PrP1 \<in> IP M4 \<and> PrP2 \<in> IP M4 \<and> IEXT M4 PrP1 \<subseteq> IEXT M4 PrP2
         \<and> (Alice, Carl) \<in> IEXT M4 PrP1)
     \<and> ((ClY, ClC2) \<in> IEXT M4 (IS M4 rdfs_subClassOf)
         \<and> (ClC2, ClC1) \<in> IEXT M4 (IS M4 rdfs_subClassOf)
         \<and> ClY \<noteq> ClC2 \<and> ClC2 \<noteq> ClC1)
     \<and> ((PrP1, PrQ) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf)
         \<and> (PrQ, PrP2) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf)
         \<and> PrP1 \<noteq> PrQ \<and> PrQ \<noteq> PrP2)
     \<and> ((PrP1, ClS1) \<in> IEXT M4 (IS M4 rdfs_domain)
         \<and> (Alice, Carl) \<in> IEXT M4 PrP1)
     \<and> (PrP1 \<in> IP M4 \<and> ClS1 \<in> IC M4
         \<and> (\<forall>x y. (x,y) \<in> IEXT M4 PrP1 \<longrightarrow> x \<in> ICEXT M4 ClS1)
         \<and> (Alice, Carl) \<in> IEXT M4 PrP1)
     \<and> ((PrP1, ClY) \<in> IEXT M4 (IS M4 rdfs_range)
         \<and> (Alice, Carl) \<in> IEXT M4 PrP1)
     \<and> (PrP1 \<in> IP M4 \<and> ClY \<in> IC M4
         \<and> (\<forall>x y. (x,y) \<in> IEXT M4 PrP1 \<longrightarrow> y \<in> ICEXT M4 ClY)
         \<and> (Alice, Carl) \<in> IEXT M4 PrP1)
     \<and> ((Alice, Alice) \<in> IEXT M4 (IS M4 owl_sameAs))
     \<and> ((ClS1, ClS2) \<in> IEXT M4 (IS M4 owl_equivalentClass) \<and> ClS1 \<noteq> ClS2
         \<and> Alice \<in> ICEXT M4 ClS1)
     \<and> ((PrP1, PrQ) \<in> IEXT M4 (IS M4 owl_equivalentProperty) \<and> PrP1 \<noteq> PrQ
         \<and> (Alice, Carl) \<in> IEXT M4 PrP1)
     \<and> ((PrP1, PrR) \<in> IEXT M4 (IS M4 owl_inverseOf) \<and> PrP1 \<noteq> PrR
         \<and> (Alice, Carl) \<in> IEXT M4 PrP1 \<and> (Carl, Alice) \<in> IEXT M4 PrR)
     \<and> (PrSym \<in> ICEXT M4 (IS M4 owl_SymmetricProperty)
         \<and> (Bob, Carl) \<in> IEXT M4 PrSym \<and> (Carl, Bob) \<in> IEXT M4 PrSym)
     \<and> (PrTrans \<in> ICEXT M4 (IS M4 owl_TransitiveProperty)
         \<and> (Alice, Bob) \<in> IEXT M4 PrTrans \<and> (Bob, Carl) \<in> IEXT M4 PrTrans
         \<and> (Alice, Carl) \<in> IEXT M4 PrTrans)
     \<and> ((ClS1, ClY) \<in> IEXT M4 (IS M4 owl_someValuesFrom)
         \<and> (ClS1, PrP2) \<in> IEXT M4 (IS M4 owl_onProperty)
         \<and> Alice \<in> ICEXT M4 ClS1)
     \<and> ((ClC1, ClY) \<in> IEXT M4 (IS M4 owl_allValuesFrom)
         \<and> (ClC1, PrP1) \<in> IEXT M4 (IS M4 owl_onProperty)
         \<and> Alice \<in> ICEXT M4 ClC1)
     \<and> ((ClS1, Bob) \<in> IEXT M4 (IS M4 owl_hasValue)
         \<and> (ClS1, PrP2) \<in> IEXT M4 (IS M4 owl_onProperty)
         \<and> Alice \<in> ICEXT M4 ClS1)
     \<and> (ClC1 \<in> ICEXT M4 (IS M4 owl_Restriction) \<and> ClC1 \<in> IC M4)
     \<and> ((ClS1, ClY) \<in> IEXT M4 (IS M4 owl_someValuesFrom))
     \<and> ((ClC1, ClY) \<in> IEXT M4 (IS M4 owl_allValuesFrom))
     \<and> ((ClC1, PrP1) \<in> IEXT M4 (IS M4 owl_onProperty))"
  using M4_ante_type_IP M4_ante_sco_IP M4_ante_spo_IP M4_ante_sco_fwd
    M4_ante_sco_bwd M4_ante_spo_fwd M4_ante_spo_bwd M4_ante_sco_trans
    M4_ante_spo_trans M4_ante_dom_fwd M4_ante_dom_bwd M4_ante_rng_fwd
    M4_ante_rng_bwd M4_ante_sameAs_fwd M4_ante_eqc_fwd M4_ante_eqp_fwd
    M4_ante_inv_fwd M4_ante_sym_fwd M4_ante_trp_fwd M4_ante_svf M4_ante_avf
    M4_ante_hv M4_ante_restr_IC M4_ante_svf_typ M4_ante_avf_typ M4_ante_onp_typ
  by blast

subsection \<open>THE GATE: all fourteen derivations fire at concrete elements of M4\<close>

(* Non-vacuity of a CONDITION is weaker than non-vacuity of a DERIVATION.  A condition
   can have something in the extension its antecedent reads while the derivation that
   consumes it never has all of its premises met at once.  Each theorem below applies one
   of the fourteen derived lemmas of OO_Builtin_Sound to M4 at named elements, with every
   premise discharged from M4's own tables, and lands on a membership those tables
   contain.  The fourteen are exactly the lemmas the twelve schema arms of the built-in
   table consume: rdfs11, rdfs5, scm-eqc1 (two conclusions), scm-eqp1 (two conclusions),
   scm-svf1, scm-svf2, scm-avf1, scm-avf2, scm-dom1, scm-dom2, scm-rng1, scm-rng2. *)

theorem M4_arm_sco_trans: "(ClY, ClC1) \<in> IEXT M4 (IS M4 rdfs_subClassOf)"
proof (rule D_sco_trans[OF M4_is_a_model])
  show "(ClY, ClC2) \<in> IEXT M4 (IS M4 rdfs_subClassOf)" by (simp add: Sco4_def)
  show "(ClC2, ClC1) \<in> IEXT M4 (IS M4 rdfs_subClassOf)" by (simp add: Sco4_def)
qed

theorem M4_arm_spo_trans: "(PrP1, PrP2) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf)"
proof (rule D_spo_trans[OF M4_is_a_model])
  show "(PrP1, PrQ) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf)" by (simp add: Spo4_def)
  show "(PrQ, PrP2) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf)" by (simp add: Spo4_def)
qed

(* ClC1 and ClA2 are distinct elements with the SAME class extension, which is what
   scm-eqc1 needs and what M3 has nothing of.  ClS1 and ClS2 are a second such pair. *)
theorem M4_arm_eqc_sub1: "(ClC1, ClA2) \<in> IEXT M4 (IS M4 rdfs_subClassOf)"
proof (rule D_eqc_sub1[OF M4_is_a_model])
  show "(ClC1, ClA2) \<in> IEXT M4 (IS M4 owl_equivalentClass)" by (simp add: Eqc4_def)
qed

theorem M4_arm_eqc_sub2: "(ClA2, ClC1) \<in> IEXT M4 (IS M4 rdfs_subClassOf)"
proof (rule D_eqc_sub2[OF M4_is_a_model])
  show "(ClC1, ClA2) \<in> IEXT M4 (IS M4 owl_equivalentClass)" by (simp add: Eqc4_def)
qed

theorem M4_arm_eqp_sub1: "(PrP1, PrQ) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf)"
proof (rule D_eqp_sub1[OF M4_is_a_model])
  show "(PrP1, PrQ) \<in> IEXT M4 (IS M4 owl_equivalentProperty)" by (simp add: Eqp4_def)
qed

theorem M4_arm_eqp_sub2: "(PrQ, PrP1) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf)"
proof (rule D_eqp_sub2[OF M4_is_a_model])
  show "(PrP1, PrQ) \<in> IEXT M4 (IS M4 owl_equivalentProperty)" by (simp add: Eqp4_def)
qed

(* scm-svf1: two existential restrictions on the same property PrP2, ordered because
   their fillers ClY and ClC1 are ordered. *)
theorem M4_arm_svf_mono1: "(ClS1, ClS2) \<in> IEXT M4 (IS M4 rdfs_subClassOf)"
proof (rule D_svf_mono1[OF M4_is_a_model])
  show "(ClS1, ClY) \<in> IEXT M4 (IS M4 owl_someValuesFrom)" by (simp add: Svf4_def)
  show "(ClS1, PrP2) \<in> IEXT M4 (IS M4 owl_onProperty)" by (simp add: Onp4_def)
  show "(ClS2, ClC1) \<in> IEXT M4 (IS M4 owl_someValuesFrom)" by (simp add: Svf4_def)
  show "(ClS2, PrP2) \<in> IEXT M4 (IS M4 owl_onProperty)" by (simp add: Onp4_def)
  show "(ClY, ClC1) \<in> IEXT M4 (IS M4 rdfs_subClassOf)" by (simp add: Sco4_def)
qed

(* scm-svf2: two existential restrictions with the same filler ClY, ordered because
   PrP1 is below PrP2 and that inclusion is PROPER, not an equality of empty sets. *)
theorem M4_arm_svf_mono2: "(ClS3, ClS1) \<in> IEXT M4 (IS M4 rdfs_subClassOf)"
proof (rule D_svf_mono2[OF M4_is_a_model])
  show "(ClS3, ClY) \<in> IEXT M4 (IS M4 owl_someValuesFrom)" by (simp add: Svf4_def)
  show "(ClS3, PrP1) \<in> IEXT M4 (IS M4 owl_onProperty)" by (simp add: Onp4_def)
  show "(ClS1, ClY) \<in> IEXT M4 (IS M4 owl_someValuesFrom)" by (simp add: Svf4_def)
  show "(ClS1, PrP2) \<in> IEXT M4 (IS M4 owl_onProperty)" by (simp add: Onp4_def)
  show "(PrP1, PrP2) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf)" by (simp add: Spo4_def)
qed

(* scm-avf1: a universal restriction is MONOTONE in its filler, so this runs the same
   way round as scm-svf1. *)
theorem M4_arm_avf_mono1: "(ClC2, ClA2) \<in> IEXT M4 (IS M4 rdfs_subClassOf)"
proof (rule D_avf_mono1[OF M4_is_a_model])
  show "(ClC2, ClY) \<in> IEXT M4 (IS M4 owl_allValuesFrom)" by (simp add: Avf4_def)
  show "(ClC2, PrP2) \<in> IEXT M4 (IS M4 owl_onProperty)" by (simp add: Onp4_def)
  show "(ClA2, ClC1) \<in> IEXT M4 (IS M4 owl_allValuesFrom)" by (simp add: Avf4_def)
  show "(ClA2, PrP2) \<in> IEXT M4 (IS M4 owl_onProperty)" by (simp add: Onp4_def)
  show "(ClY, ClC1) \<in> IEXT M4 (IS M4 rdfs_subClassOf)" by (simp add: Sco4_def)
qed

(* scm-avf2: and ANTITONE in its property, so the conclusion is REVERSED.  The pair
   (ClC1, ClC2) that the naive reading would draw is not merely absent from Sco4, it is
   barred from it; M4_avf2_really_is_antitone below pins that. *)
theorem M4_arm_avf_mono2: "(ClC2, ClC1) \<in> IEXT M4 (IS M4 rdfs_subClassOf)"
proof (rule D_avf_mono2[OF M4_is_a_model])
  show "(ClC1, ClY) \<in> IEXT M4 (IS M4 owl_allValuesFrom)" by (simp add: Avf4_def)
  show "(ClC1, PrP1) \<in> IEXT M4 (IS M4 owl_onProperty)" by (simp add: Onp4_def)
  show "(ClC2, ClY) \<in> IEXT M4 (IS M4 owl_allValuesFrom)" by (simp add: Avf4_def)
  show "(ClC2, PrP2) \<in> IEXT M4 (IS M4 owl_onProperty)" by (simp add: Onp4_def)
  show "(PrP1, PrP2) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf)" by (simp add: Spo4_def)
qed

(* scm-dom1: widening PrP1's domain from ClS1 to ClS2. *)
theorem M4_arm_dom_mono1: "(PrP1, ClS2) \<in> IEXT M4 (IS M4 rdfs_domain)"
proof (rule D_dom_mono1[OF M4_is_a_model])
  show "(PrP1, ClS1) \<in> IEXT M4 (IS M4 rdfs_domain)" by (simp add: Dm4_def)
  show "(ClS1, ClS2) \<in> IEXT M4 (IS M4 rdfs_subClassOf)" by (simp add: Sco4_def)
qed

(* scm-dom2: inheriting PrP2's domain ClS3 down to the subproperty PrP1. *)
theorem M4_arm_dom_mono2: "(PrP1, ClS3) \<in> IEXT M4 (IS M4 rdfs_domain)"
proof (rule D_dom_mono2[OF M4_is_a_model])
  show "(PrP2, ClS3) \<in> IEXT M4 (IS M4 rdfs_domain)" by (simp add: Dm4_def)
  show "(PrP1, PrP2) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf)" by (simp add: Spo4_def)
qed

(* scm-rng1: widening PrP1's range from ClY to ClC1. *)
theorem M4_arm_rng_mono1: "(PrP1, ClC1) \<in> IEXT M4 (IS M4 rdfs_range)"
proof (rule D_rng_mono1[OF M4_is_a_model])
  show "(PrP1, ClY) \<in> IEXT M4 (IS M4 rdfs_range)" by (simp add: Rg4_def)
  show "(ClY, ClC1) \<in> IEXT M4 (IS M4 rdfs_subClassOf)" by (simp add: Sco4_def)
qed

(* scm-rng2: inheriting PrP2's range ClC2 down to the subproperty PrP1.  The domain and
   range tables DIFFER at this row and at the ClY row, which is M4 separating the two
   halves of Table 5.8 rather than satisfying them both by accident. *)
theorem M4_arm_rng_mono2: "(PrP1, ClC2) \<in> IEXT M4 (IS M4 rdfs_range)"
proof (rule D_rng_mono2[OF M4_is_a_model])
  show "(PrP2, ClC2) \<in> IEXT M4 (IS M4 rdfs_range)" by (simp add: Rg4_def)
  show "(PrP1, PrP2) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf)" by (simp add: Spo4_def)
qed

(* All fourteen, in one statement.  A later edit that stops exercising one of them fails
   here instead of passing quietly, which is the whole point of the block. *)
theorem M4_exercises_every_derivation:
  "(ClY, ClC1) \<in> IEXT M4 (IS M4 rdfs_subClassOf)
     \<and> (PrP1, PrP2) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf)
     \<and> (ClC1, ClA2) \<in> IEXT M4 (IS M4 rdfs_subClassOf)
     \<and> (ClA2, ClC1) \<in> IEXT M4 (IS M4 rdfs_subClassOf)
     \<and> (PrP1, PrQ) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf)
     \<and> (PrQ, PrP1) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf)
     \<and> (ClS1, ClS2) \<in> IEXT M4 (IS M4 rdfs_subClassOf)
     \<and> (ClS3, ClS1) \<in> IEXT M4 (IS M4 rdfs_subClassOf)
     \<and> (ClC2, ClA2) \<in> IEXT M4 (IS M4 rdfs_subClassOf)
     \<and> (ClC2, ClC1) \<in> IEXT M4 (IS M4 rdfs_subClassOf)
     \<and> (PrP1, ClS2) \<in> IEXT M4 (IS M4 rdfs_domain)
     \<and> (PrP1, ClS3) \<in> IEXT M4 (IS M4 rdfs_domain)
     \<and> (PrP1, ClC1) \<in> IEXT M4 (IS M4 rdfs_range)
     \<and> (PrP1, ClC2) \<in> IEXT M4 (IS M4 rdfs_range)"
  using M4_arm_sco_trans M4_arm_spo_trans M4_arm_eqc_sub1 M4_arm_eqc_sub2
    M4_arm_eqp_sub1 M4_arm_eqp_sub2 M4_arm_svf_mono1 M4_arm_svf_mono2
    M4_arm_avf_mono1 M4_arm_avf_mono2 M4_arm_dom_mono1 M4_arm_dom_mono2
    M4_arm_rng_mono1 M4_arm_rng_mono2
  by blast

subsection \<open>Non-degeneracy, pinned\<close>

(* Everything above would still go through with the tables hollowed out, which is how
   the gap this section closes stayed invisible in M3 for as long as it did.  These are
   the facts that stop an emptied-out M4 from being reported as a result.  Each is stated
   at a NAMED ELEMENT rather than as a non-emptiness, so that the witness is in the
   theorem and not in a tactic, and each of the three shapes the brief asks for is here:
   IC is non-empty and is not the whole carrier; ICEXT(ClY) is a non-empty PROPER subset
   of ICEXT(ClC2), witnessed on both sides by Carl and Bob; and IEXT(PrP1) is non-empty
   and a PROPER subset of IEXT(PrP2), witnessed on both sides by (Alice, Carl) and
   (Alice, Bob), which is what makes the subPropertyOf fact between them hold for a
   reason rather than for want of anything to check. *)
theorem M4_is_live:
  "(ClC1 \<in> IC M4)
     \<and> (Alice \<notin> IC M4)
     \<and> (Carl \<in> ICEXT M4 ClY)
     \<and> (ICEXT M4 ClY \<subseteq> ICEXT M4 ClC2)
     \<and> (Bob \<in> ICEXT M4 ClC2)
     \<and> (Bob \<notin> ICEXT M4 ClY)
     \<and> ((Alice, Carl) \<in> IEXT M4 PrP1)
     \<and> (IEXT M4 PrP1 \<subseteq> IEXT M4 PrP2)
     \<and> ((Alice, Bob) \<in> IEXT M4 PrP2)
     \<and> ((Alice, Bob) \<notin> IEXT M4 PrP1)
     \<and> ((PrP1, PrP2) \<in> IEXT M4 (IS M4 rdfs_subPropertyOf))
     \<and> (ICEXT M4 (IS M4 owl_Restriction) \<subseteq> IC M4)
     \<and> (ClY \<in> IC M4)
     \<and> (ClY \<notin> ICEXT M4 (IS M4 owl_Restriction))
     \<and> ((Alice, Alice) \<in> IEXT M4 (IS M4 owl_sameAs))"
  by (simp add: P1_4_def P2_4_def Spo4_def)

(* scm-avf2 is ANTITONE, and this is what that costs: the pair the arm does NOT conclude
   is not merely absent from Sco4, it is barred from it, because c_sco_fwd would demand
   an inclusion between the two class extensions that fails at Alice. *)
theorem M4_avf2_really_is_antitone:
  "(ClC1, ClC2) \<notin> IEXT M4 (IS M4 rdfs_subClassOf)"
  by (simp add: Sco4_def)

(* Bridge coherence, and what it is and is not for.  Every element of M4 whose extension
   is NON-EMPTY is in IP.  That is a NECESSARY condition for a structure to be the image
   of a conforming interpretation under a bridge that reads IEXT only at members of IP,
   and it is not sufficient for anything; owl_rl_interp does not impose it, because no
   arm consumes it, so it is a property this model happens to have rather than a
   requirement it meets.

   It is also why M4 does not sharpen the IP-clause refutation that M3 carries.  The
   elements M4 leaves outside IP are exactly the ones whose extension is empty, so a
   triple whose predicate denotes one of them is false for two reasons at once.  That is
   equally true of M3_refutes_sameAs, whose comment above claims a little more for itself
   than it establishes: zz is outside IP AND has an empty extension, so dropping the IP
   clause from M5 would leave that triple false rather than make it true.  Neither
   witness here isolates the IP clause, and this file does not have one that does. *)
theorem M4_is_bridge_coherent: "ext4 p \<noteq> {} \<Longrightarrow> p \<in> IP M4"
  by (cases p) (auto simp: IP4_def)

subsection \<open>What M4 makes true, and what it makes false\<close>

(* The fixture IRIs land on ex_a = Carl, ex_A = ClY and ex_B = ClC2, so the asserted
   subClassOf triple of the fixture graph holds because {Carl} is a PROPER non-empty
   subset of the carrier without Alice, and not because both sides are empty. *)
lemma M4_sat_asserted_1: "sat M4 (Iri ex_a, Iri rdf_type, Iri ex_A)"
  by (simp add: IP4_def Ty4_def)
lemma M4_sat_asserted_2: "sat M4 (Iri ex_A, Iri rdfs_subClassOf, Iri ex_B)"
  by (simp add: IP4_def Sco4_def)

theorem M4_models_fixture_graph: "models M4 (set fixture_graph)"
  by (simp add: models_def fixture_graph_def IP4_def Ty4_def Sco4_def)

theorem M4_sat_the_certified_conclusion: "sat M4 (Iri ex_a, Iri rdf_type, Iri ex_B)"
  by (simp add: IP4_def Ty4_def)

theorem M4_refutes_fresh_class: "\<not> sat M4 (Iri ex_a, Iri rdf_type, Iri ex_C)"
  by (simp add: Ty4_def)

theorem entailment_is_not_trivial_M4:
  "\<not> entails TYPE(E4) (set fixture_graph) (Iri ex_a, Iri rdf_type, Iri ex_C)"
  unfolding entails_def
  using M4_is_a_model M4_models_fixture_graph M4_refutes_fresh_class by blast

subsection \<open>owl:sameAs, corrected\<close>

(* A CORRECTION, recorded rather than applied silently.  It is not true that owl:sameAs
   cannot be exercised by any model.  RBS Table 5.9 row 1 reads
   "( a1 , a2 ) in IEXT(I(owl:sameAs)) iff a1 = a2" with the variables unscoped and so
   ranging over IR, and RBS section 4.2 defines IR as "the universe of I, i.e., a
   nonempty set that contains at least the denotations of literals and IRIs in V"; both
   re-read in the raw HTML of the Recommendation on 15 September 2026.  The section
   number is 4.2, Vocabulary Interpretations, and not 4.4, which is Parts of the
   Universe and defines IC and IP; the draft this note replaces cited 4.4.  So in every
   conforming interpretation that extension is the FULL DIAGONAL on a non-empty set,
   hence non-empty, and c_sameAs_fwd has a satisfied antecedent there.  M4 carries that
   diagonal and M4_ante_sameAs_fwd exhibits an instance.

   What IS true is the sharper statement below, and it is the reason the condition can
   never be made to look interesting: no interpretation meeting the cell has an
   OFF-DIAGONAL pair, so the condition is exercised at a = a and at nothing else.  The
   obstruction is proved here rather than asserted. *)
theorem sameAs_is_exercised_only_on_the_diagonal:
  assumes "owl_rl_interp I" and "a \<noteq> b"
  shows "(a, b) \<notin> IEXT I (IS I owl_sameAs)"
  using assms by (auto simp: owl_rl_interp_def c_sameAs_fwd_def)

end
