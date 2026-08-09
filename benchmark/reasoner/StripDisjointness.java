/*
 * Remove every disjointness axiom from an ontology.
 *
 * Produces the held-out variant for evaluating assumed-disjointness recovery:
 * strip the axioms, let a proposer + vetter reconstruct a candidate surface,
 * then score it against the ORIGINAL ontology's exhaustive incompatibility
 * matrix. Also mirrors the dominant real-world case, since most large
 * ontologies declare no disjointness at all.
 *
 * Usage: java StripDisjointness <in.owl> <out.owl>
 */

import java.io.File;
import java.util.ArrayList;
import java.util.List;

import org.semanticweb.owlapi.apibinding.OWLManager;
import org.semanticweb.owlapi.formats.FunctionalSyntaxDocumentFormat;
import org.semanticweb.owlapi.model.*;

public class StripDisjointness {
    public static void main(String[] args) throws Exception {
        OWLOntologyManager m = OWLManager.createOWLOntologyManager();
        OWLOntology ont = m.loadOntologyFromOntologyDocument(new File(args[0]));

        List<OWLAxiom> drop = new ArrayList<>();
        for (OWLAxiom ax : ont.getAxioms()) {
            if (ax instanceof OWLDisjointClassesAxiom
                    || ax instanceof OWLDisjointUnionAxiom) {
                drop.add(ax);
            }
        }
        m.removeAxioms(ont, new java.util.HashSet<>(drop));
        m.saveOntology(ont, new FunctionalSyntaxDocumentFormat(),
            IRI.create(new File(args[1]).toURI()));
        System.out.println("{\"removed\":" + drop.size() + "}");
    }
}
