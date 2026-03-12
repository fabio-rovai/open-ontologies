import org.semanticweb.owlapi.apibinding.OWLManager;
import org.semanticweb.owlapi.model.*;
import org.semanticweb.owlapi.reasoner.*;
import java.io.*;
import java.util.*;

public class JavaReasoner {
    public static void main(String[] args) throws Exception {
        if (args.length < 3) {
            System.err.println("Usage: JavaReasoner <reasoner> <ontology.owl> <output.json>");
            System.exit(1);
        }
        String reasonerName = args[0];  // "hermit" or "pellet"
        String ontologyPath = args[1];
        String outputPath = args[2];

        OWLOntologyManager manager = OWLManager.createOWLOntologyManager();
        OWLOntology ontology = manager.loadOntologyFromOntologyDocument(new File(ontologyPath));

        OWLReasonerFactory factory;
        if (reasonerName.equals("hermit")) {
            factory = new org.semanticweb.HermiT.ReasonerFactory();
        } else if (reasonerName.equals("pellet")) {
            // Load Pellet via reflection (may not be on classpath)
            try {
                Class<?> cls = Class.forName("com.clarkparsia.pellet.owlapi.PelletReasonerFactory");
                factory = (OWLReasonerFactory) cls.getMethod("getInstance").invoke(null);
            } catch (ClassNotFoundException e) {
                // Try Openllet fork
                try {
                    Class<?> cls = Class.forName("openllet.owlapi.OpenlletReasonerFactory");
                    factory = (OWLReasonerFactory) cls.getMethod("getInstance").invoke(null);
                } catch (ClassNotFoundException e2) {
                    System.err.println("ERROR: Neither Pellet nor Openllet found on classpath");
                    System.exit(1);
                    return;
                }
            }
        } else {
            throw new IllegalArgumentException("Unknown reasoner: " + reasonerName);
        }

        long startTime = System.currentTimeMillis();
        OWLReasoner reasoner = factory.createReasoner(ontology);
        reasoner.precomputeInferences(InferenceType.CLASS_HIERARCHY);
        long elapsed = System.currentTimeMillis() - startTime;

        // Extract all subsumption pairs
        Set<OWLClass> classes = ontology.getClassesInSignature();
        List<String> subsumptions = new ArrayList<>();
        for (OWLClass cls : classes) {
            for (OWLClass sup : reasoner.getSuperClasses(cls, true).getFlattened()) {
                subsumptions.add(cls.getIRI().toString() + " -> " + sup.getIRI().toString());
            }
        }
        Collections.sort(subsumptions);

        // Write JSON output
        StringBuilder sb = new StringBuilder();
        sb.append("{\"reasoner\":\"").append(reasonerName).append("\",");
        sb.append("\"time_ms\":").append(elapsed).append(",");
        sb.append("\"classes\":").append(classes.size()).append(",");
        sb.append("\"subsumptions\":[");
        for (int i = 0; i < subsumptions.size(); i++) {
            if (i > 0) sb.append(",");
            sb.append("\"").append(subsumptions.get(i).replace("\"", "\\\"")).append("\"");
        }
        sb.append("]}");

        try (FileWriter fw = new FileWriter(outputPath)) {
            fw.write(sb.toString());
        }
        System.out.println("Done: " + reasonerName + " in " + elapsed + "ms, " + subsumptions.size() + " subsumptions");
    }
}
