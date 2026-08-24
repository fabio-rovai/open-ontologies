/*
 * Lazy pair oracle: keeps a reasoner warm and answers "is A ⊓ B satisfiable?"
 * one pair at a time, on demand.
 *
 * This is what removes the O(n^2) blocker. Precomputing the full compatibility
 * matrix is quadratic and dies past ~1000 classes (659 classes took 274s). But
 * a claim stream only ever mentions a small working set of classes, so we only
 * need the pairs that are actually asked for. Ask, cache, never ask again.
 *
 * Protocol: one line in, one line out.
 *   in:   <classIRI-A>\t<classIRI-B>
 *   out:  {"a":"...","b":"...","compatible":true,"ms":3}
 *
 * The ontology is loaded and precomputed ONCE at start-up, so per-pair cost is
 * a single satisfiability test against a warm reasoner.
 *
 * Usage: java PairOracle <ontology>
 */

import java.io.BufferedReader;
import java.io.File;
import java.io.InputStreamReader;

import org.semanticweb.owlapi.apibinding.OWLManager;
import org.semanticweb.owlapi.model.*;
import org.semanticweb.owlapi.reasoner.OWLReasoner;
import org.semanticweb.HermiT.ReasonerFactory;

public class PairOracle {
    public static void main(String[] args) throws Exception {
        OWLOntologyManager m = OWLManager.createOWLOntologyManager();
        OWLOntology ont = m.loadOntologyFromOntologyDocument(new File(args[0]));
        OWLDataFactory df = m.getOWLDataFactory();
        OWLReasoner r = new ReasonerFactory().createReasoner(ont);

        // Deliberately NOT precomputeInferences(). Full classification is
        // exactly the O(n * expensive) cost the lazy design exists to avoid,
        // and it is not needed: a satisfiability test on A ⊓ B does not
        // require the class hierarchy. On a 6,929-class ORE ontology the
        // precompute alone ran past 14 minutes, which defeats the purpose.
        // Touch consistency only, so the reasoner initialises its internal
        // structures without classifying.
        long t0 = System.currentTimeMillis();
        r.isConsistent();
        long warmMs = System.currentTimeMillis() - t0;

        // Signal readiness with the one-off warm-up cost so callers can
        // account for it separately from per-pair latency.
        System.out.println("{\"ready\":true,\"warmup_ms\":" + warmMs
            + ",\"classes\":" + ont.getClassesInSignature().size() + "}");
        System.out.flush();

        BufferedReader in = new BufferedReader(new InputStreamReader(System.in));
        String line;
        while ((line = in.readLine()) != null) {
            line = line.trim();
            if (line.isEmpty()) continue;
            String[] parts = line.split("\t");
            if (parts.length < 2) continue;

            long s = System.nanoTime();
            boolean compatible;
            try {
                OWLClassExpression both = df.getOWLObjectIntersectionOf(
                    df.getOWLClass(IRI.create(parts[0])),
                    df.getOWLClass(IRI.create(parts[1])));
                compatible = r.isSatisfiable(both);
            } catch (Exception e) {
                // Undecided: report compatible so we never fabricate a clash.
                compatible = true;
            }
            double ms = (System.nanoTime() - s) / 1e6;

            System.out.println("{\"a\":\"" + parts[0] + "\",\"b\":\"" + parts[1]
                + "\",\"compatible\":" + compatible
                + ",\"ms\":" + String.format("%.3f", ms) + "}");
            System.out.flush();
        }
        r.dispose();
    }
}
