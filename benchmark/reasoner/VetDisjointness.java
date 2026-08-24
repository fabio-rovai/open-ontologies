/*
 * Vet PROPOSED disjointness axioms against an ontology.
 *
 * The MCP-native split for enriching a zero-disjointness ontology: a proposer
 * (an LLM over MCP, or a structural heuristic) suggests class pairs that
 * "should" be disjoint; this tool gives each proposal one of three verdicts:
 *
 *   entailed      A ⊓ B is already unsatisfiable — the pair belongs in the
 *                 HARD tier; no assumption needed.
 *   inadmissible  assuming disjoint(A,B) would break the ontology: A and B
 *                 are subsumption-comparable (the subclass would become
 *                 empty), or they share a satisfiable named subclass.
 *   admissible    consistent to assume. Suitable for the WARN tier ONLY —
 *                 admissible means "does not contradict the ontology", never
 *                 "true".
 *
 * Protocol: stdin lines "<iriA>\t<iriB>", one JSON verdict per line.
 *
 * Usage: java VetDisjointness <ontology>
 */

import java.io.BufferedReader;
import java.io.File;
import java.io.InputStreamReader;
import java.util.HashSet;
import java.util.Set;

import org.semanticweb.owlapi.apibinding.OWLManager;
import org.semanticweb.owlapi.model.*;
import org.semanticweb.owlapi.reasoner.OWLReasoner;
import org.semanticweb.HermiT.ReasonerFactory;

public class VetDisjointness {
    public static void main(String[] args) throws Exception {
        OWLOntologyManager m = OWLManager.createOWLOntologyManager();
        OWLOntology ont = m.loadOntologyFromOntologyDocument(new File(args[0]));
        OWLDataFactory df = m.getOWLDataFactory();
        OWLReasoner r = new ReasonerFactory().createReasoner(ont);
        r.isConsistent();

        BufferedReader in = new BufferedReader(new InputStreamReader(System.in));
        String line;
        while ((line = in.readLine()) != null) {
            line = line.trim();
            if (line.isEmpty()) continue;
            String[] p = line.split("\t");
            if (p.length < 2) continue;
            OWLClass a = df.getOWLClass(IRI.create(p[0]));
            OWLClass b = df.getOWLClass(IRI.create(p[1]));

            String verdict;
            long t0 = System.nanoTime();
            try {
                OWLClassExpression both = df.getOWLObjectIntersectionOf(a, b);
                if (!r.isSatisfiable(both)) {
                    verdict = "entailed";
                } else if (r.getSuperClasses(a, false).containsEntity(b)
                        || r.getSuperClasses(b, false).containsEntity(a)
                        || r.getEquivalentClasses(a).contains(b)) {
                    // Comparable classes: disjoint(A,B) with A ⊑ B forces
                    // A ⊑ ⊥.
                    verdict = "inadmissible";
                } else {
                    // A satisfiable named class under both would become
                    // unsatisfiable under the assumption.
                    Set<OWLClass> subA = new HashSet<>(r.getSubClasses(a, false).getFlattened());
                    boolean shared = false;
                    for (OWLClass c : r.getSubClasses(b, false).getFlattened()) {
                        if (c.isOWLNothing()) continue;
                        if (subA.contains(c) && r.isSatisfiable(c)) {
                            shared = true;
                            break;
                        }
                    }
                    verdict = shared ? "inadmissible" : "admissible";
                }
            } catch (Exception e) {
                verdict = "error";
            }
            double ms = (System.nanoTime() - t0) / 1e6;
            System.out.println("{\"a\":\"" + p[0] + "\",\"b\":\"" + p[1]
                + "\",\"verdict\":\"" + verdict + "\",\"ms\":"
                + String.format("%.2f", ms) + "}");
            System.out.flush();
        }
        r.dispose();
    }
}
