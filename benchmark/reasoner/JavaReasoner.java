import org.semanticweb.owlapi.apibinding.OWLManager;
import org.semanticweb.owlapi.model.*;
import org.semanticweb.owlapi.model.parameters.Imports;
import org.semanticweb.owlapi.reasoner.*;
import java.io.*;
import java.lang.management.ManagementFactory;
import java.util.*;

/**
 * Reference classification for the reasoner benchmarks.
 *
 * Output is the FULL entailed named-class hierarchy, not the direct one. For
 * every named class C that the reasoner finds satisfiable, it emits "C -> D"
 * for every named D with C entailed to be a subclass of D: all superclasses
 * (direct=false) plus all equivalent classes. owl:Thing and owl:Nothing never
 * appear in a pair. Unsatisfiable classes are listed under "unsatisfiable" and
 * contribute no pairs, because an unsatisfiable class is trivially a subclass
 * of everything and would swamp the comparison.
 *
 * Why not direct superclasses: the engine reports all superclasses, so a
 * direct-only reference makes every indirect pair look like an unsound extra.
 * Direct superclasses also never include equivalent classes. In ORE file
 * ore_ont_8937, axioms SubClassOf(owl:Thing C23) make C23 equivalent to
 * owl:Thing, so the true pair C25 -> C23 is absent from a direct-only
 * reference and a correct engine answer was misread as unsound.
 *
 * It also emits "told_subsumptions": the reflexive-free transitive closure of
 * the ASSERTED named hierarchy, i.e. SubClassOf(A, B) with A and B named,
 * pairwise EquivalentClasses among named members, and the tautological edge
 * A -> owl:Thing for every named A. No reasoner is involved in computing it.
 * A structural walk of the asserted axioms finds exactly these pairs, so the
 * benchmark subtracts them to measure what reasoning adds.
 *
 * Times are reported separately and never summed silently:
 *   jvm_start_ms         JVM start to entry of main (process boot cost)
 *   load_ms              parse the document into an OWLOntology
 *   reasoner_create_ms   construct the reasoner (HermiT clausifies here)
 *   classify_ms          precomputeInferences(CLASS_HIERARCHY) only
 *   extract_ms           read the hierarchy back out (not reasoning cost)
 *   time_ms              reasoner_create_ms + classify_ms, kept for older callers
 */
public class JavaReasoner {
    public static void main(String[] args) throws Exception {
        long mainEntry = System.currentTimeMillis();
        long jvmStart = ManagementFactory.getRuntimeMXBean().getStartTime();
        if (args.length < 3) {
            System.err.println("Usage: JavaReasoner <hermit|pellet> <ontology> <output.json>");
            System.exit(1);
        }
        String reasonerName = args[0];
        String ontologyPath = args[1];
        String outputPath = args[2];

        long t0 = System.nanoTime();
        OWLOntologyManager manager = OWLManager.createOWLOntologyManager();
        OWLOntology ontology = manager.loadOntologyFromOntologyDocument(new File(ontologyPath));
        long loadNs = System.nanoTime() - t0;

        OWLReasonerFactory factory = factoryFor(reasonerName);

        long t1 = System.nanoTime();
        OWLReasoner reasoner = factory.createReasoner(ontology);
        long createNs = System.nanoTime() - t1;

        long t2 = System.nanoTime();
        reasoner.precomputeInferences(InferenceType.CLASS_HIERARCHY);
        long classifyNs = System.nanoTime() - t2;

        long t3 = System.nanoTime();
        boolean consistent = reasoner.isConsistent();
        Set<OWLClass> classes = ontology.getClassesInSignature(Imports.INCLUDED);
        List<String> subsumptions = new ArrayList<>();
        List<String> unsatisfiable = new ArrayList<>();
        int namedClasses = 0;
        for (OWLClass cls : classes) {
            if (cls.isOWLThing() || cls.isOWLNothing()) continue;
            namedClasses++;
            if (!consistent) continue;
            if (!reasoner.isSatisfiable(cls)) {
                unsatisfiable.add(cls.getIRI().toString());
                continue;
            }
            Set<OWLClass> supers = new HashSet<>(reasoner.getSuperClasses(cls, false).getFlattened());
            supers.addAll(reasoner.getEquivalentClasses(cls).getEntities());
            for (OWLClass sup : supers) {
                if (sup.equals(cls) || sup.isOWLThing() || sup.isOWLNothing()) continue;
                subsumptions.add(cls.getIRI().toString() + " -> " + sup.getIRI().toString());
            }
        }
        Collections.sort(subsumptions);
        Collections.sort(unsatisfiable);
        long extractNs = System.nanoTime() - t3;

        long t4 = System.nanoTime();
        List<String> told = toldClosure(ontology, classes);
        long toldNs = System.nanoTime() - t4;

        long createMs = createNs / 1_000_000L;
        long classifyMs = classifyNs / 1_000_000L;
        StringBuilder sb = new StringBuilder();
        sb.append("{\"reasoner\":").append(q(reasonerName)).append(",");
        sb.append("\"ontology\":").append(q(ontologyPath)).append(",");
        sb.append("\"pairs\":\"all_superclasses_plus_equivalents\",");
        sb.append("\"jvm_start_ms\":").append(mainEntry - jvmStart).append(",");
        sb.append("\"load_ms\":").append(loadNs / 1_000_000L).append(",");
        sb.append("\"reasoner_create_ms\":").append(createMs).append(",");
        sb.append("\"classify_ms\":").append(classifyMs).append(",");
        sb.append("\"extract_ms\":").append(extractNs / 1_000_000L).append(",");
        sb.append("\"told_ms\":").append(toldNs / 1_000_000L).append(",");
        sb.append("\"time_ms\":").append(createMs + classifyMs).append(",");
        sb.append("\"consistent\":").append(consistent).append(",");
        sb.append("\"axioms\":").append(ontology.getLogicalAxiomCount()).append(",");
        sb.append("\"classes\":").append(namedClasses).append(",");
        appendArray(sb, "unsatisfiable", unsatisfiable);
        sb.append(",");
        appendArray(sb, "subsumptions", subsumptions);
        sb.append(",");
        appendArray(sb, "told_subsumptions", told);
        sb.append("}");

        try (Writer w = new OutputStreamWriter(new FileOutputStream(outputPath), "UTF-8")) {
            w.write(sb.toString());
        }
        reasoner.dispose();
        System.out.println("Done: " + reasonerName + " create " + createMs + "ms, classify "
            + classifyMs + "ms, " + subsumptions.size() + " entailed pairs, "
            + told.size() + " told pairs, " + unsatisfiable.size() + " unsatisfiable");
    }

    /** Transitive closure of the asserted named-class hierarchy, no reasoner. */
    static List<String> toldClosure(OWLOntology ontology, Set<OWLClass> classes) {
        Map<OWLClass, Set<OWLClass>> up = new HashMap<>();
        OWLClass thing = null;
        for (OWLClass c : classes) {
            if (c.isOWLThing()) thing = c;
        }
        for (OWLSubClassOfAxiom ax : ontology.getAxioms(AxiomType.SUBCLASS_OF, Imports.INCLUDED)) {
            if (ax.getSubClass().isAnonymous() || ax.getSuperClass().isAnonymous()) continue;
            edge(up, ax.getSubClass().asOWLClass(), ax.getSuperClass().asOWLClass());
        }
        for (OWLEquivalentClassesAxiom ax : ontology.getAxioms(AxiomType.EQUIVALENT_CLASSES, Imports.INCLUDED)) {
            List<OWLClass> named = new ArrayList<>(ax.getNamedClasses());
            for (OWLClass a : named) {
                for (OWLClass b : named) {
                    if (!a.equals(b)) edge(up, a, b);
                }
            }
        }
        // Every named class is a subclass of owl:Thing. This only matters when
        // the ontology gives owl:Thing a told superclass, as ore_ont_8937 does.
        if (thing != null) {
            for (OWLClass c : classes) {
                if (!c.isOWLThing() && !c.isOWLNothing()) edge(up, c, thing);
            }
        }
        List<String> out = new ArrayList<>();
        for (OWLClass c : classes) {
            if (c.isOWLThing() || c.isOWLNothing()) continue;
            Set<OWLClass> seen = new HashSet<>();
            Deque<OWLClass> stack = new ArrayDeque<>(up.getOrDefault(c, Collections.emptySet()));
            while (!stack.isEmpty()) {
                OWLClass d = stack.pop();
                if (!seen.add(d)) continue;
                stack.addAll(up.getOrDefault(d, Collections.emptySet()));
            }
            for (OWLClass d : seen) {
                if (d.equals(c) || d.isOWLThing() || d.isOWLNothing()) continue;
                out.add(c.getIRI().toString() + " -> " + d.getIRI().toString());
            }
        }
        Collections.sort(out);
        return out;
    }

    static void edge(Map<OWLClass, Set<OWLClass>> up, OWLClass a, OWLClass b) {
        up.computeIfAbsent(a, k -> new HashSet<>()).add(b);
    }

    static OWLReasonerFactory factoryFor(String name) throws Exception {
        if (name.equals("hermit")) {
            return new org.semanticweb.HermiT.ReasonerFactory();
        }
        if (name.equals("pellet")) {
            for (String fqcn : new String[] {
                    "com.clarkparsia.pellet.owlapi.PelletReasonerFactory",
                    "openllet.owlapi.OpenlletReasonerFactory"}) {
                try {
                    Class<?> cls = Class.forName(fqcn);
                    return (OWLReasonerFactory) cls.getMethod("getInstance").invoke(null);
                } catch (ClassNotFoundException e) {
                    // try the next one
                }
            }
            System.err.println("ERROR: Neither Pellet nor Openllet found on classpath");
            System.exit(2);
        }
        throw new IllegalArgumentException("Unknown reasoner: " + name);
    }

    static void appendArray(StringBuilder sb, String key, List<String> items) {
        sb.append(q(key)).append(":[");
        for (int i = 0; i < items.size(); i++) {
            if (i > 0) sb.append(",");
            sb.append(q(items.get(i)));
        }
        sb.append("]");
    }

    static String q(String s) {
        StringBuilder sb = new StringBuilder("\"");
        for (char ch : s.toCharArray()) {
            switch (ch) {
                case '"': sb.append("\\\""); break;
                case '\\': sb.append("\\\\"); break;
                case '\n': sb.append("\\n"); break;
                case '\r': sb.append("\\r"); break;
                case '\t': sb.append("\\t"); break;
                default:
                    if (ch < 0x20) sb.append(String.format("\\u%04x", (int) ch));
                    else sb.append(ch);
            }
        }
        return sb.append("\"").toString();
    }
}
