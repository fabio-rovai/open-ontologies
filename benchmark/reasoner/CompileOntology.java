/*
 * The mature compile step: ONE classification, then read the axioms off.
 *
 * Emits, as JSON:
 *   subsumptions   the INFERRED class hierarchy (reflexive-transitive)
 *   disjoint       the ASSERTED disjointness axioms, minimal set
 *   unsatisfiable  classes equivalent to owl:Nothing
 *
 * Incompatibility is then DERIVED at query time by a two-hop join:
 *
 *     A ⊓ B unsatisfiable  iff  ∃ A' ⊒ A, B' ⊒ B  with  disjoint(A', B')
 *
 * This replaces materialising the O(n^2) pairwise matrix. Pizza needs 3,944
 * stored pairs the brute-force way; the same information is carried by ~400
 * subsumptions plus a few dozen disjointness axioms, and costs ONE
 * classification instead of n^2/2 satisfiability tests.
 *
 * Caveat this design must be measured against, not assumed away: the two-hop
 * join captures incompatibility that follows from subsumption + disjointness.
 * It does NOT capture incompatibility arising some other way (e.g. cardinality
 * clashes, or a class unsatisfiable only in combination via property
 * restrictions). CompareCompiled checks exactly that gap.
 *
 * Usage: java CompileOntology <ontology> <out.json>
 */

import java.io.File;
import java.io.PrintWriter;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

import org.semanticweb.owlapi.apibinding.OWLManager;
import org.semanticweb.owlapi.model.*;
import org.semanticweb.owlapi.reasoner.OWLReasoner;
import org.semanticweb.owlapi.reasoner.InferenceType;
import org.semanticweb.owlapi.reasoner.Node;
import org.semanticweb.HermiT.ReasonerFactory;

public class CompileOntology {

    static String q(String s) {
        return "\"" + s.replace("\\", "\\\\").replace("\"", "\\\"") + "\"";
    }

    public static void main(String[] args) throws Exception {
        OWLOntologyManager m = OWLManager.createOWLOntologyManager();
        OWLOntology ont = m.loadOntologyFromOntologyDocument(new File(args[0]));
        OWLReasoner r = new ReasonerFactory().createReasoner(ont);

        long t0 = System.currentTimeMillis();
        r.precomputeInferences(InferenceType.CLASS_HIERARCHY);
        long classifyMs = System.currentTimeMillis() - t0;

        // ── inferred hierarchy ──────────────────────────────────────────
        List<String> subs = new ArrayList<>();
        List<String> unsat = new ArrayList<>();
        for (OWLClass c : ont.getClassesInSignature()) {
            if (c.isOWLThing() || c.isOWLNothing()) continue;
            if (!r.isSatisfiable(c)) {
                unsat.add(q(c.getIRI().toString()));
                continue;
            }
            // Reflexive.
            subs.add("[" + q(c.getIRI().toString()) + "," + q(c.getIRI().toString()) + "]");
            for (OWLClass sup : r.getSuperClasses(c, false).getFlattened()) {
                if (sup.isOWLThing()) continue;
                subs.add("[" + q(c.getIRI().toString()) + "," + q(sup.getIRI().toString()) + "]");
            }
            // Equivalent classes subsume each other.
            Node<OWLClass> eq = r.getEquivalentClasses(c);
            for (OWLClass e : eq.getEntities()) {
                if (e.isOWLThing() || e.isOWLNothing() || e.equals(c)) continue;
                subs.add("[" + q(c.getIRI().toString()) + "," + q(e.getIRI().toString()) + "]");
            }
        }

        // ── asserted disjointness, the MINIMAL set ──────────────────────
        List<String> disj = new ArrayList<>();
        for (OWLAxiom ax : ont.getAxioms()) {
            if (ax instanceof OWLDisjointClassesAxiom dca) {
                List<OWLClassExpression> ops = new ArrayList<>(dca.getClassExpressionsAsList());
                for (int i = 0; i < ops.size(); i++) {
                    for (int j = i + 1; j < ops.size(); j++) {
                        if (ops.get(i).isAnonymous() || ops.get(j).isAnonymous()) continue;
                        disj.add("[" + q(ops.get(i).asOWLClass().getIRI().toString()) + ","
                            + q(ops.get(j).asOWLClass().getIRI().toString()) + "]");
                    }
                }
            }
        }

        // ── restriction-mediated disjointness propagation ───────────────
        //
        // Two sound rules, run to fixpoint over derived disjointness:
        //   R1  A ⊑ ∃R.C, B ⊑ ∀R.(D1⊔…⊔Dn), ∀i disj*(C,Di)  ⟹  disj(A,B)
        //       (the A⊓B individual needs an R-successor in C, and all its
        //        R-successors fall in the union — the successor would be in
        //        C ⊓ Di for some i, which is empty)
        //   R2  functional(R), A ⊑ ∃R.C, B ⊑ ∃R.D, disj*(C,D) ⟹ disj(A,B)
        //       (the unique R-value cannot be in two disjoint classes)
        //
        // disj* is disjointness up to inferred subsumption (the same two-hop
        // the runtime join performs). Fixpoint matters: R2 derives
        // topping-level disjointness that R1 then lifts to pizza level.
        //
        // Deliberately NOT handled in v1, so the envelope is explicit:
        // inverse/anonymous properties, role hierarchies (∃R vs ∀S with
        // R ⊑ S), and nominal fillers (hasValue). All are sound extensions.

        // Named reflexive ancestor sets from the inferred hierarchy.
        Map<String, Set<String>> anc = new HashMap<>();
        for (OWLClass c : ont.getClassesInSignature()) {
            if (c.isOWLThing() || c.isOWLNothing() || !r.isSatisfiable(c)) continue;
            Set<String> a = new HashSet<>();
            a.add(c.getIRI().toString());
            for (OWLClass sup : r.getSuperClasses(c, false).getFlattened())
                if (!sup.isOWLThing()) a.add(sup.getIRI().toString());
            for (OWLClass eq : r.getEquivalentClasses(c).getEntities())
                if (!eq.isOWLThing() && !eq.isOWLNothing()) a.add(eq.getIRI().toString());
            anc.put(c.getIRI().toString(), a);
        }

        // Directly asserted/defined restrictions per named class, then
        // inherited along the inferred hierarchy.
        Map<String, List<String[]>> ex = new HashMap<>();   // class -> [prop, filler...]
        Map<String, List<String[]>> all = new HashMap<>();  // class -> [prop, filler...]
        Map<String, List<String[]>> unions = new HashMap<>(); // class -> union operands
        // Cardinality records. minRec: [prop, n, filler] — at least n R-successors
        // in filler ("*" = unqualified/Thing). maxRec: [prop, m, filler] — at
        // most m R-successors in filler.
        Map<String, List<String[]>> minRec = new HashMap<>();
        Map<String, List<String[]>> maxRec = new HashMap<>();
        Map<String, List<String[]>> hv = new HashMap<>();     // class -> [dataProp, datatype, lexical]
        Map<String, Set<String>> dmax1 = new HashMap<>();     // class -> dataProps with card <= 1
        for (OWLAxiom ax : ont.getAxioms()) {
            OWLClass named = null;
            List<OWLClassExpression> exprs = new ArrayList<>();
            if (ax instanceof OWLSubClassOfAxiom sca && !sca.getSubClass().isAnonymous()) {
                named = sca.getSubClass().asOWLClass();
                exprs.add(sca.getSuperClass());
            } else if (ax instanceof OWLEquivalentClassesAxiom eca) {
                for (OWLClassExpression e : eca.getClassExpressions())
                    if (!e.isAnonymous()) named = e.asOWLClass();
                if (named != null) exprs.addAll(eca.getClassExpressions());
            }
            if (named == null) continue;
            // Unfold top-level intersections so definition conjuncts count.
            List<OWLClassExpression> flat = new ArrayList<>();
            for (OWLClassExpression e : exprs) {
                if (e instanceof OWLObjectIntersectionOf io) flat.addAll(io.getOperands());
                else flat.add(e);
            }
            for (OWLClassExpression e : flat) {
                if (e instanceof OWLObjectSomeValuesFrom sv
                        && !sv.getProperty().isAnonymous()
                        && !sv.getFiller().isAnonymous()) {
                    String cn = named.getIRI().toString();
                    String pr = sv.getProperty().getNamedProperty().getIRI().toString();
                    String fl = sv.getFiller().asOWLClass().getIRI().toString();
                    ex.computeIfAbsent(cn, x -> new ArrayList<>()).add(new String[]{pr, fl});
                    minRec.computeIfAbsent(cn, x -> new ArrayList<>())
                          .add(new String[]{pr, "1", fl});
                } else if (e instanceof OWLObjectMinCardinality mc
                        && mc.getCardinality() >= 1
                        && !mc.getProperty().isAnonymous()) {
                    String cn = named.getIRI().toString();
                    String pr = mc.getProperty().getNamedProperty().getIRI().toString();
                    String fl = mc.getFiller().isAnonymous() ? "*"
                        : mc.getFiller().asOWLClass().getIRI().toString();
                    if (!mc.getFiller().isAnonymous())
                        ex.computeIfAbsent(cn, x -> new ArrayList<>()).add(new String[]{pr, fl});
                    if (fl.equals("*") || !mc.getFiller().isAnonymous())
                        minRec.computeIfAbsent(cn, x -> new ArrayList<>())
                              .add(new String[]{pr, String.valueOf(mc.getCardinality()), fl});
                } else if (e instanceof OWLObjectExactCardinality ec
                        && !ec.getProperty().isAnonymous()
                        && (ec.getFiller().isAnonymous() ? ec.getFiller().isOWLThing() || true : true)) {
                    // Exact n = (min n) ⊓ (max n). Record both sides when the
                    // filler is named or unqualified; skip complex fillers.
                    if (ec.getFiller().isAnonymous() && !ec.getFiller().isOWLThing()) {
                        // complex filler — outside the envelope
                    } else {
                        String cn = named.getIRI().toString();
                        String pr = ec.getProperty().getNamedProperty().getIRI().toString();
                        String fl = ec.getFiller().isOWLThing() ? "*"
                            : ec.getFiller().asOWLClass().getIRI().toString();
                        String n = String.valueOf(ec.getCardinality());
                        if (ec.getCardinality() >= 1) {
                            minRec.computeIfAbsent(cn, x -> new ArrayList<>())
                                  .add(new String[]{pr, n, fl});
                            if (!fl.equals("*"))
                                ex.computeIfAbsent(cn, x -> new ArrayList<>())
                                  .add(new String[]{pr, fl});
                        }
                        maxRec.computeIfAbsent(cn, x -> new ArrayList<>())
                              .add(new String[]{pr, n, fl});
                    }
                } else if (e instanceof OWLObjectMaxCardinality xc
                        && !xc.getProperty().isAnonymous()
                        && (!xc.getFiller().isAnonymous() || xc.getFiller().isOWLThing())) {
                    String cn = named.getIRI().toString();
                    String pr = xc.getProperty().getNamedProperty().getIRI().toString();
                    String fl = xc.getFiller().isOWLThing() ? "*"
                        : xc.getFiller().asOWLClass().getIRI().toString();
                    maxRec.computeIfAbsent(cn, x -> new ArrayList<>())
                          .add(new String[]{pr, String.valueOf(xc.getCardinality()), fl});
                } else if (e instanceof OWLDataExactCardinality dxc
                        && dxc.getCardinality() == 1
                        && dxc.getProperty() instanceof OWLDataProperty dxp
                        && dxc.getFiller() instanceof OWLDataOneOf oneOf
                        && oneOf.getValues().size() == 1) {
                    // ExactCardinality(1, p, DataOneOf(v)) pins exactly one
                    // p-value equal to v — a hasValue in cardinality syntax.
                    OWLLiteral lit = oneOf.getValues().iterator().next();
                    hv.computeIfAbsent(named.getIRI().toString(), x -> new ArrayList<>())
                      .add(new String[]{dxp.getIRI().toString(),
                                        lit.getDatatype().getIRI().toString(),
                                        lit.getLiteral()});
                    dmax1.computeIfAbsent(named.getIRI().toString(), x -> new HashSet<>())
                         .add(dxp.getIRI().toString());
                } else if (e instanceof OWLObjectUnionOf uo2) {
                    // B ⊑ (D1 ⊔ … ⊔ Dn). Only usable when EVERY operand is
                    // named: deriving from a subset of the operands would
                    // claim a tighter constraint than the axiom states, which
                    // is unsound.
                    List<String> ops = new ArrayList<>();
                    boolean allNamed2 = true;
                    for (OWLClassExpression u : uo2.getOperands()) {
                        if (u.isAnonymous()) { allNamed2 = false; break; }
                        ops.add(u.asOWLClass().getIRI().toString());
                    }
                    if (allNamed2 && !ops.isEmpty()) {
                        unions.computeIfAbsent(named.getIRI().toString(), x -> new ArrayList<>())
                              .add(ops.toArray(new String[0]));
                    }
                } else if (e instanceof OWLDataHasValue dhv
                        && dhv.getProperty() instanceof OWLDataProperty dp) {
                    OWLLiteral lit = dhv.getFiller();
                    hv.computeIfAbsent(named.getIRI().toString(), x -> new ArrayList<>())
                      .add(new String[]{dp.getIRI().toString(),
                                        lit.getDatatype().getIRI().toString(),
                                        lit.getLiteral()});
                } else if (e instanceof OWLDataMaxCardinality dmc
                        && dmc.getCardinality() <= 1
                        && dmc.getProperty() instanceof OWLDataProperty dp2) {
                    dmax1.computeIfAbsent(named.getIRI().toString(), x -> new HashSet<>())
                         .add(dp2.getIRI().toString());
                } else if (e instanceof OWLDataExactCardinality dec
                        && dec.getCardinality() <= 1
                        && dec.getProperty() instanceof OWLDataProperty dp3) {
                    dmax1.computeIfAbsent(named.getIRI().toString(), x -> new HashSet<>())
                         .add(dp3.getIRI().toString());
                } else if (e instanceof OWLObjectAllValuesFrom av
                        && !av.getProperty().isAnonymous()) {
                    List<String> fillers = new ArrayList<>();
                    OWLClassExpression f = av.getFiller();
                    if (!f.isAnonymous()) {
                        fillers.add(f.asOWLClass().getIRI().toString());
                    } else if (f instanceof OWLObjectUnionOf uo) {
                        boolean allNamed = true;
                        for (OWLClassExpression u : uo.getOperands()) {
                            if (u.isAnonymous()) { allNamed = false; break; }
                            fillers.add(u.asOWLClass().getIRI().toString());
                        }
                        if (!allNamed) continue;
                    } else continue;
                    String[] rec = new String[fillers.size() + 1];
                    rec[0] = av.getProperty().getNamedProperty().getIRI().toString();
                    for (int i = 0; i < fillers.size(); i++) rec[i + 1] = fillers.get(i);
                    all.computeIfAbsent(named.getIRI().toString(), x -> new ArrayList<>())
                       .add(rec);
                }
            }
        }
        // Inherit restrictions from inferred ancestors.
        Map<String, List<String[]>> exInh = new HashMap<>();
        Map<String, List<String[]>> allInh = new HashMap<>();
        Map<String, List<String[]>> unionInh = new HashMap<>();
        Map<String, List<String[]>> hvInh = new HashMap<>();
        Map<String, Set<String>> dmax1Inh = new HashMap<>();
        Map<String, List<String[]>> minInh = new HashMap<>();
        Map<String, List<String[]>> maxInh = new HashMap<>();
        for (String c : anc.keySet()) {
            for (String a : anc.get(c)) {
                if (ex.containsKey(a))
                    exInh.computeIfAbsent(c, x -> new ArrayList<>()).addAll(ex.get(a));
                if (all.containsKey(a))
                    allInh.computeIfAbsent(c, x -> new ArrayList<>()).addAll(all.get(a));
                if (unions.containsKey(a))
                    unionInh.computeIfAbsent(c, x -> new ArrayList<>()).addAll(unions.get(a));
                if (hv.containsKey(a))
                    hvInh.computeIfAbsent(c, x -> new ArrayList<>()).addAll(hv.get(a));
                if (dmax1.containsKey(a))
                    dmax1Inh.computeIfAbsent(c, x -> new HashSet<>()).addAll(dmax1.get(a));
                if (minRec.containsKey(a))
                    minInh.computeIfAbsent(c, x -> new ArrayList<>()).addAll(minRec.get(a));
                if (maxRec.containsKey(a))
                    maxInh.computeIfAbsent(c, x -> new ArrayList<>()).addAll(maxRec.get(a));
            }
        }

        Set<String> functional = new HashSet<>();
        for (OWLFunctionalObjectPropertyAxiom fa : ont.getAxioms(AxiomType.FUNCTIONAL_OBJECT_PROPERTY))
            if (!fa.getProperty().isAnonymous())
                functional.add(fa.getProperty().getNamedProperty().getIRI().toString());
        Set<String> functionalData = new HashSet<>();
        for (OWLFunctionalDataPropertyAxiom fda : ont.getAxioms(AxiomType.FUNCTIONAL_DATA_PROPERTY))
            if (fda.getProperty() instanceof OWLDataProperty fdp)
                functionalData.add(fdp.getIRI().toString());

        // Symmetric pair set seeded from the asserted axioms collected above.
        Set<String> disjSet = new HashSet<>();
        List<String[]> disjPairsList = new ArrayList<>();
        for (String d : disj) {
            // parse back "["a","b"]"
            String body = d.substring(2, d.length() - 2);
            String[] ab = body.split("\",\"");
            disjSet.add(ab[0] + "|" + ab[1]);
            disjSet.add(ab[1] + "|" + ab[0]);
            disjPairsList.add(ab);
        }

        java.util.function.BiPredicate<String, String> disjStar = (a, b) -> {
            Set<String> aa = anc.getOrDefault(a, java.util.Collections.singleton(a));
            Set<String> bb = anc.getOrDefault(b, java.util.Collections.singleton(b));
            for (String x : aa) for (String y : bb)
                if (disjSet.contains(x + "|" + y)) return true;
            return false;
        };

        List<String> derived = new ArrayList<>();

        // RD: distinct required literal values for a <=1-valued data property.
        //   A ⊑ hasValue(p, v1),  B ⊑ hasValue(p, v2),  v1 ≠ v2,
        //   and functional(p) OR some ancestor of A or B carries card<=1 on p
        //   ⟹ disj(A, B): the A⊓B individual would need two distinct values.
        // Literal distinctness is claimed ONLY for same-datatype string/boolean
        // literals, where lexically-distinct implies value-distinct. Numerics
        // are excluded ("1" vs "01" differ lexically but not in value).
        List<String> hvClasses = new ArrayList<>(hvInh.keySet());
        for (int i = 0; i < hvClasses.size(); i++) {
            String a = hvClasses.get(i);
            for (int j = i + 1; j < hvClasses.size(); j++) {
                String b = hvClasses.get(j);
                if (disjStar.test(a, b)) continue;
                outer:
                for (String[] va : hvInh.get(a)) {
                    for (String[] vb : hvInh.get(b)) {
                        if (!va[0].equals(vb[0])) continue;           // same property
                        if (!va[1].equals(vb[1])) continue;           // same datatype
                        boolean stringy = va[1].endsWith("#string") || va[1].endsWith("#boolean")
                            || va[1].equals("http://www.w3.org/2000/01/rdf-schema#Literal");
                        if (!stringy || va[2].equals(vb[2])) continue; // distinct lexical
                        String prop = va[0];
                        boolean bounded = functionalData.contains(prop)
                            || dmax1Inh.getOrDefault(a, java.util.Collections.emptySet()).contains(prop)
                            || dmax1Inh.getOrDefault(b, java.util.Collections.emptySet()).contains(prop);
                        if (bounded) {
                            disjSet.add(a + "|" + b); disjSet.add(b + "|" + a);
                            derived.add("[" + q(a) + "," + q(b) + "]");
                            break outer;
                        }
                    }
                }
            }
        }

        // R4: counting clash. A ⊑ ≤m R.C, B ⊑ ≥n R.D with n > m and D ⊑* C
        // (or C unqualified): the A⊓B individual would need at least n
        // R-successors inside a region capped at m. Functional object
        // properties act as an unqualified ≤1.
        List<String> maxClasses = new ArrayList<>(maxInh.keySet());
        List<String> minClasses = new ArrayList<>(minInh.keySet());
        for (String a : maxClasses) {
            for (String[] mx : maxInh.get(a)) {
                int m2 = Integer.parseInt(mx[1]);
                for (String b : minClasses) {
                    if (a.equals(b) || disjStar.test(a, b)) continue;
                    for (String[] mn : minInh.get(b)) {
                        if (!mn[0].equals(mx[0])) continue;
                        int n2 = Integer.parseInt(mn[1]);
                        if (n2 <= m2) continue;
                        boolean fillerOk = mx[2].equals("*")
                            || (!mn[2].equals("*")
                                && anc.getOrDefault(mn[2], java.util.Collections.singleton(mn[2]))
                                      .contains(mx[2]));
                        if (fillerOk) {
                            disjSet.add(a + "|" + b); disjSet.add(b + "|" + a);
                            derived.add("[" + q(a) + "," + q(b) + "]");
                            break;
                        }
                    }
                }
            }
        }

        int rounds = 0;
        boolean changed = true;
        while (changed && rounds++ < 20) {
            changed = false;
            for (String a : exInh.keySet()) {
                for (String[] rx : exInh.get(a)) {
                    String prop = rx[0], c = rx[1];
                    // R1: against every ALL-restriction on the same property.
                    for (String b : allInh.keySet()) {
                        if (a.equals(b) || disjStar.test(a, b)) continue;
                        for (String[] ry : allInh.get(b)) {
                            if (!ry[0].equals(prop)) continue;
                            boolean allDisj = true;
                            for (int i = 1; i < ry.length; i++)
                                if (!disjStar.test(c, ry[i])) { allDisj = false; break; }
                            if (allDisj) {
                                disjSet.add(a + "|" + b); disjSet.add(b + "|" + a);
                                derived.add("[" + q(a) + "," + q(b) + "]");
                                changed = true;
                                break;
                            }
                        }
                    }
                    // R2: functional property, against every EX on the same property.
                    if (!functional.contains(prop)) continue;
                    for (String b : exInh.keySet()) {
                        if (a.equals(b) || disjStar.test(a, b)) continue;
                        for (String[] ry : exInh.get(b)) {
                            if (!ry[0].equals(prop)) continue;
                            if (disjStar.test(c, ry[1])) {
                                disjSet.add(a + "|" + b); disjSet.add(b + "|" + a);
                                derived.add("[" + q(a) + "," + q(b) + "]");
                                changed = true;
                                break;
                            }
                        }
                    }
                }
            }
            // R5: A ⊑ ∀R.(C…), B ⊑ ∀R.(D…) with every Ci disjoint from every
            // Dj, and A or B (via ancestors) forces at least one R-successor.
            // That successor would have to be in both fillers, which are
            // incompatible. Without the ≥1, an individual with no R-successors
            // satisfies both universals, so no derivation.
            for (String a : allInh.keySet()) {
                for (String[] ra : allInh.get(a)) {
                    for (String b : allInh.keySet()) {
                        if (a.equals(b) || disjStar.test(a, b)) continue;
                        boolean forced = false;
                        for (String[] mn : minInh.getOrDefault(a, java.util.Collections.emptyList()))
                            if (mn[0].equals(ra[0])) { forced = true; break; }
                        if (!forced)
                            for (String[] mn : minInh.getOrDefault(b, java.util.Collections.emptyList()))
                                if (mn[0].equals(ra[0])) { forced = true; break; }
                        if (!forced) continue;
                        for (String[] rb : allInh.get(b)) {
                            if (!rb[0].equals(ra[0])) continue;
                            boolean allDisj = true;
                            outer5:
                            for (int i = 1; i < ra.length; i++)
                                for (int j = 1; j < rb.length; j++)
                                    if (!disjStar.test(ra[i], rb[j])) { allDisj = false; break outer5; }
                            if (allDisj) {
                                disjSet.add(a + "|" + b); disjSet.add(b + "|" + a);
                                derived.add("[" + q(a) + "," + q(b) + "]");
                                changed = true;
                                break;
                            }
                        }
                    }
                }
            }
            // RU: B is subsumed by a union of named classes, and A is
            // disjoint from EVERY operand ⟹ disj(A, B). An A⊓B individual
            // must fall in some operand, each of which excludes A. Runs
            // inside the fixpoint because disj* grows as R1/R2/RU fire
            // (e.g. category-level pairs derived here lift pizza-level pairs
            // through R1 next round).
            for (String b : unionInh.keySet()) {
                for (String[] ops : unionInh.get(b)) {
                    for (String a : anc.keySet()) {
                        if (a.equals(b) || disjStar.test(a, b)) continue;
                        boolean allDisj = true;
                        for (String d : ops)
                            if (!disjStar.test(a, d)) { allDisj = false; break; }
                        if (allDisj) {
                            disjSet.add(a + "|" + b); disjSet.add(b + "|" + a);
                            derived.add("[" + q(a) + "," + q(b) + "]");
                            changed = true;
                        }
                    }
                }
            }
        }
        disj.addAll(derived);

        PrintWriter out = new PrintWriter(args[1]);
        out.println("{\"classify_ms\":" + classifyMs
            + ",\"subsumptions\":[" + String.join(",", subs) + "]"
            + ",\"disjoint\":[" + String.join(",", disj) + "]"
            + ",\"disjoint_derived\":[" + String.join(",", derived) + "]"
            + ",\"unsatisfiable\":[" + String.join(",", unsat) + "]}");
        out.close();

        System.out.println("{\"classify_ms\":" + classifyMs
            + ",\"subsumptions\":" + subs.size()
            + ",\"disjoint_axioms\":" + disj.size()
            + ",\"disjoint_derived\":" + derived.size()
            + ",\"propagation_rounds\":" + rounds
            + ",\"unsatisfiable\":" + unsat.size() + "}");
        r.dispose();
    }
}
