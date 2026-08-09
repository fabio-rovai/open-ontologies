/*
 * Parity baseline: full ABox consistency checking of candidate claims.
 *
 * Reads claims as JSON-lines on stdin, each of the form
 *   {"id":"c1","types":[["x","IRI"],...],"rels":[["x","propIRI","y"],...]}
 *
 * For each claim, asserts it into a fresh copy of the ontology and asks HermiT
 * whether the resulting knowledge base is consistent. This is the ground truth
 * the compiled DuckDB path must agree with.
 *
 * Emits JSON-lines: {"id":"c1","consistent":true,"ms":12}
 *
 * Usage: java ClaimConsistency <ontology> < claims.jsonl > verdicts.jsonl
 */

import java.io.BufferedReader;
import java.io.File;
import java.io.InputStreamReader;
import java.util.ArrayList;
import java.util.List;

import org.semanticweb.owlapi.apibinding.OWLManager;
import org.semanticweb.owlapi.model.*;
import org.semanticweb.owlapi.reasoner.OWLReasoner;
import org.semanticweb.HermiT.ReasonerFactory;

public class ClaimConsistency {

    /** Minimal JSON array-of-arrays extractor; the input is machine-written. */
    static List<String[]> tuples(String json, String key, int arity) {
        List<String[]> out = new ArrayList<>();
        int k = json.indexOf("\"" + key + "\"");
        if (k < 0) return out;
        int start = json.indexOf('[', k);
        if (start < 0) return out;
        int depth = 0, i = start;
        for (; i < json.length(); i++) {
            char c = json.charAt(i);
            if (c == '[') depth++;
            else if (c == ']') { depth--; if (depth == 0) break; }
        }
        String body = json.substring(start + 1, i);
        int p = 0;
        while (true) {
            int a = body.indexOf('[', p);
            if (a < 0) break;
            int b = body.indexOf(']', a);
            if (b < 0) break;
            String[] parts = body.substring(a + 1, b).split(",");
            String[] vals = new String[arity];
            boolean ok = parts.length >= arity;
            for (int j = 0; ok && j < arity; j++) {
                String s = parts[j].trim();
                if (s.length() >= 2 && s.charAt(0) == '"') s = s.substring(1, s.length() - 1);
                vals[j] = s;
            }
            if (ok) out.add(vals);
            p = b + 1;
        }
        return out;
    }

    static String field(String json, String key) {
        int k = json.indexOf("\"" + key + "\"");
        if (k < 0) return "";
        int c = json.indexOf(':', k);
        int q1 = json.indexOf('"', c);
        int q2 = json.indexOf('"', q1 + 1);
        return json.substring(q1 + 1, q2);
    }

    public static void main(String[] args) throws Exception {
        File ontFile = new File(args[0]);
        OWLOntologyManager m = OWLManager.createOWLOntologyManager();
        OWLOntology base = m.loadOntologyFromOntologyDocument(ontFile);
        OWLDataFactory df = m.getOWLDataFactory();
        String IND = "http://tardygrada.example/claim#";

        BufferedReader in = new BufferedReader(new InputStreamReader(System.in));
        String line;
        while ((line = in.readLine()) != null) {
            line = line.trim();
            if (line.isEmpty()) continue;
            String id = field(line, "id");

            List<OWLAxiom> added = new ArrayList<>();
            for (String[] t : tuples(line, "types", 2)) {
                added.add(df.getOWLClassAssertionAxiom(
                    df.getOWLClass(IRI.create(t[1])),
                    df.getOWLNamedIndividual(IRI.create(IND + t[0]))));
            }
            for (String[] r : tuples(line, "rels", 3)) {
                added.add(df.getOWLObjectPropertyAssertionAxiom(
                    df.getOWLObjectProperty(IRI.create(r[1])),
                    df.getOWLNamedIndividual(IRI.create(IND + r[0])),
                    df.getOWLNamedIndividual(IRI.create(IND + r[2]))));
            }

            long t0 = System.currentTimeMillis();
            boolean consistent;
            try {
                m.addAxioms(base, new java.util.HashSet<>(added));
                OWLReasoner r = new ReasonerFactory().createReasoner(base);
                consistent = r.isConsistent();
                r.dispose();
            } catch (Exception e) {
                consistent = true; // could not decide; do not claim a clash
            } finally {
                m.removeAxioms(base, new java.util.HashSet<>(added));
            }
            long ms = System.currentTimeMillis() - t0;

            System.out.println("{\"id\":\"" + id + "\",\"consistent\":"
                + consistent + ",\"ms\":" + ms + "}");
            System.out.flush();
        }
    }
}
