import org.semanticweb.owlapi.apibinding.OWLManager;
import org.semanticweb.owlapi.formats.RDFXMLDocumentFormat;
import org.semanticweb.owlapi.model.*;
import java.io.*;

/**
 * Re-serialise an ontology to RDF/XML with the OWL API.
 *
 * The ORE 2015 corpus ships in OWL Functional Syntax. run_ore.py converts each
 * input ONCE with this program and then feeds the converted file to both the
 * engine and HermiT, so the two reasoners always read the same bytes. Run
 * run_ore.py with --native to skip the conversion and feed the original file.
 *
 * Usage: java -cp <classes>:<lib>/* Convert <in> <out.rdf>
 */
public class Convert {
    public static void main(String[] args) throws Exception {
        if (args.length != 2) {
            System.err.println("Usage: Convert <in> <out.rdf>");
            System.exit(1);
        }
        OWLOntologyManager m = OWLManager.createOWLOntologyManager();
        OWLOntology o = m.loadOntologyFromOntologyDocument(new File(args[0]));
        try (OutputStream out = new BufferedOutputStream(new FileOutputStream(args[1]))) {
            m.saveOntology(o, new RDFXMLDocumentFormat(), out);
        }
    }
}
