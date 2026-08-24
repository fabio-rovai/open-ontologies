/*
 * Offline compile step: compute the pairwise class-compatibility matrix.
 *
 * For every pair of named classes (A,B), ask a real OWL reasoner whether
 * A ⊓ B is satisfiable. The pairs where it is NOT are the complete
 * contradiction surface of the ontology, including every disjointness that
 * follows by INFERENCE rather than by an explicit owl:disjointWith axiom.
 *
 * This is expensive (O(n^2) satisfiability tests) but it runs ONCE per
 * ontology, offline. The output is a flat table that a columnar engine can
 * check with a single indexed lookup at query time.
 *
 * Usage: java DisjointnessMatrix <ontology> <out.csv>
 */

import java.io.PrintWriter;
import java.io.File;
import java.util.ArrayList;
import java.util.List;

import org.semanticweb.owlapi.apibinding.OWLManager;
import org.semanticweb.owlapi.model.*;
import org.semanticweb.owlapi.reasoner.OWLReasoner;
import org.semanticweb.HermiT.ReasonerFactory;

public class DisjointnessMatrix {
    public static void main(String[] args) throws Exception {
        String ontPath = args[0];
        String outPath = args[1];

        OWLOntologyManager m = OWLManager.createOWLOntologyManager();
        OWLOntology ont = m.loadOntologyFromOntologyDocument(new File(ontPath));
        OWLDataFactory df = m.getOWLDataFactory();
        OWLReasoner r = new ReasonerFactory().createReasoner(ont);

        long tStart = System.currentTimeMillis();
        r.precomputeInferences();

        List<OWLClass> classes = new ArrayList<>();
        for (OWLClass c : ont.getClassesInSignature()) {
            if (!c.isOWLThing() && !c.isOWLNothing() && r.isSatisfiable(c)) {
                classes.add(c);
            }
        }

        PrintWriter out = new PrintWriter(outPath);
        out.println("a,b");
        int pairs = 0, incompatible = 0;
        for (int i = 0; i < classes.size(); i++) {
            for (int j = i + 1; j < classes.size(); j++) {
                OWLClass a = classes.get(i), b = classes.get(j);
                OWLObjectIntersectionOf both =
                    df.getOWLObjectIntersectionOf(a, b);
                pairs++;
                if (!r.isSatisfiable(both)) {
                    incompatible++;
                    out.println("\"" + a.getIRI() + "\",\"" + b.getIRI() + "\"");
                    out.println("\"" + b.getIRI() + "\",\"" + a.getIRI() + "\"");
                }
            }
        }
        out.close();
        long ms = System.currentTimeMillis() - tStart;

        System.out.println("{\"classes\":" + classes.size()
            + ",\"pairs_tested\":" + pairs
            + ",\"incompatible_pairs\":" + incompatible
            + ",\"compile_ms\":" + ms + "}");
        r.dispose();
    }
}
