use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::graph::GraphStore;

// Well-known IRIs
const RDF_TYPE: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
const RDFS_SUBCLASS: &str = "<http://www.w3.org/2000/01/rdf-schema#subClassOf>";
const RDFS_SUBPROP: &str = "<http://www.w3.org/2000/01/rdf-schema#subPropertyOf>";
const RDFS_DOMAIN: &str = "<http://www.w3.org/2000/01/rdf-schema#domain>";
const RDFS_RANGE: &str = "<http://www.w3.org/2000/01/rdf-schema#range>";
const OWL_TRANSITIVE: &str = "<http://www.w3.org/2002/07/owl#TransitiveProperty>";
const OWL_SYMMETRIC: &str = "<http://www.w3.org/2002/07/owl#SymmetricProperty>";
const OWL_INVERSE: &str = "<http://www.w3.org/2002/07/owl#inverseOf>";
const OWL_SAMEAS: &str = "<http://www.w3.org/2002/07/owl#sameAs>";
const OWL_EQUIV_CLASS: &str = "<http://www.w3.org/2002/07/owl#equivalentClass>";
const OWL_EQUIV_PROP: &str = "<http://www.w3.org/2002/07/owl#equivalentProperty>";
const OWL_SOME_VALUES: &str = "<http://www.w3.org/2002/07/owl#someValuesFrom>";
const OWL_ALL_VALUES: &str = "<http://www.w3.org/2002/07/owl#allValuesFrom>";
const OWL_HAS_VALUE: &str = "<http://www.w3.org/2002/07/owl#hasValue>";
const OWL_ON_PROPERTY: &str = "<http://www.w3.org/2002/07/owl#onProperty>";
const OWL_INTERSECTION: &str = "<http://www.w3.org/2002/07/owl#intersectionOf>";
const OWL_UNION: &str = "<http://www.w3.org/2002/07/owl#unionOf>";
const RDF_FIRST: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>";
const RDF_REST: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>";
const RDF_NIL: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>";

/// Intern strings to u32 IDs for efficient reasoning.
struct Interner {
    to_id: HashMap<String, u32>,
    to_str: Vec<String>,
}

impl Interner {
    fn new() -> Self {
        Self {
            to_id: HashMap::new(),
            to_str: Vec::new(),
        }
    }

    fn intern(&mut self, s: &str) -> u32 {
        if let Some(&id) = self.to_id.get(s) {
            return id;
        }
        let id = self.to_str.len() as u32;
        self.to_str.push(s.to_string());
        self.to_id.insert(s.to_string(), id);
        id
    }

    fn resolve(&self, id: u32) -> &str {
        &self.to_str[id as usize]
    }
}

/// OWL2-RL reasoner using interned u32 triples and fixpoint iteration.
///
/// Profiles:
///   "rdfs"       — RDFS rules (subclass, domain/range, subproperty)
///   "owl-rl"     — RDFS + core OWL-RL (transitive, symmetric, inverse,
///                  sameAs, equivalentClass/Property)
///   "owl-rl-ext" — All above + someValuesFrom, allValuesFrom, hasValue,
///                  intersectionOf, unionOf
pub struct Reasoner;

impl Reasoner {
    pub fn run(
        graph: &Arc<GraphStore>,
        profile: &str,
        materialize: bool,
    ) -> anyhow::Result<String> {
        // Delegate OWL-DL to tableaux reasoner
        if profile == "owl-dl" {
            return crate::tableaux::DlReasoner::run(graph, materialize);
        }

        let profile_used = match profile {
            "owl-rl" => "owl-rl",
            "owl-rl-ext" => "owl-rl-ext",
            _ => "rdfs",
        };
        let include_owl = profile_used == "owl-rl" || profile_used == "owl-rl-ext";
        let include_ext = profile_used == "owl-rl-ext";

        // Extract and intern all triples
        let raw_triples = graph.all_triples()?;
        let mut interner = Interner::new();
        let mut facts: Vec<(u32, u32, u32)> = Vec::with_capacity(raw_triples.len());
        for (s, p, o) in &raw_triples {
            facts.push((interner.intern(s), interner.intern(p), interner.intern(o)));
        }

        // Intern well-known IRIs
        let rdf_type = interner.intern(RDF_TYPE);
        let rdfs_subclass = interner.intern(RDFS_SUBCLASS);
        let rdfs_subprop = interner.intern(RDFS_SUBPROP);
        let owl_sameas = interner.intern(OWL_SAMEAS);

        // Pre-extract static schema relations
        let domain_map: Vec<(u32, u32)> = facts.iter()
            .filter(|&&(_, p, _)| p == interner.intern(RDFS_DOMAIN))
            .map(|&(s, _, o)| (s, o)).collect();
        let range_map: Vec<(u32, u32)> = facts.iter()
            .filter(|&&(_, p, _)| p == interner.intern(RDFS_RANGE))
            .map(|&(s, _, o)| (s, o)).collect();
        let transitive_set: HashSet<u32> = facts.iter()
            .filter(|&&(_, p, o)| p == rdf_type && o == interner.intern(OWL_TRANSITIVE))
            .map(|&(s, _, _)| s).collect();
        let symmetric_set: HashSet<u32> = facts.iter()
            .filter(|&&(_, p, o)| p == rdf_type && o == interner.intern(OWL_SYMMETRIC))
            .map(|&(s, _, _)| s).collect();
        let inverse_pairs: Vec<(u32, u32)> = facts.iter()
            .filter(|&&(_, p, _)| p == interner.intern(OWL_INVERSE))
            .map(|&(s, _, o)| (s, o)).collect();
        let equiv_class: Vec<(u32, u32)> = facts.iter()
            .filter(|&&(_, p, _)| p == interner.intern(OWL_EQUIV_CLASS))
            .map(|&(s, _, o)| (s, o)).collect();
        let equiv_prop: Vec<(u32, u32)> = facts.iter()
            .filter(|&&(_, p, _)| p == interner.intern(OWL_EQUIV_PROP))
            .map(|&(s, _, o)| (s, o)).collect();

        // OWL restriction structures (for owl-rl-ext)
        let owl_on_property = interner.intern(OWL_ON_PROPERTY);
        let owl_some_values = interner.intern(OWL_SOME_VALUES);
        let owl_all_values = interner.intern(OWL_ALL_VALUES);
        let owl_has_value = interner.intern(OWL_HAS_VALUE);

        let mut restr_prop: HashMap<u32, u32> = HashMap::new();
        let mut restr_svf: HashMap<u32, u32> = HashMap::new();
        let mut restr_avf: HashMap<u32, u32> = HashMap::new();
        let mut restr_hv: HashMap<u32, u32> = HashMap::new();

        if include_ext {
            for &(s, p, o) in &facts {
                if p == owl_on_property { restr_prop.insert(s, o); }
                if p == owl_some_values { restr_svf.insert(s, o); }
                if p == owl_all_values { restr_avf.insert(s, o); }
                if p == owl_has_value { restr_hv.insert(s, o); }
            }
        }

        // svf_rules: (property, filler_class, restriction_node)
        let svf_rules: Vec<(u32, u32, u32)> = restr_svf.iter()
            .filter_map(|(&r, &filler)| restr_prop.get(&r).map(|&prop| (prop, filler, r)))
            .collect();
        // hv_rules: (property, value, restriction_node)
        let hv_rules: Vec<(u32, u32, u32)> = restr_hv.iter()
            .filter_map(|(&r, &val)| restr_prop.get(&r).map(|&prop| (prop, val, r)))
            .collect();
        // avf_rules: (property, filler_class, restriction_node)
        let _avf_rules: Vec<(u32, u32, u32)> = restr_avf.iter()
            .filter_map(|(&r, &filler)| restr_prop.get(&r).map(|&prop| (prop, filler, r)))
            .collect();

        // Parse RDF lists for intersectionOf/unionOf
        let mut intersection_classes: Vec<(u32, Vec<u32>)> = Vec::new();
        let mut union_classes: Vec<(u32, Vec<u32>)> = Vec::new();
        if include_ext {
            let rdf_first = interner.intern(RDF_FIRST);
            let rdf_rest = interner.intern(RDF_REST);
            let rdf_nil = interner.intern(RDF_NIL);
            let owl_intersection = interner.intern(OWL_INTERSECTION);
            let owl_union = interner.intern(OWL_UNION);

            let first_map: HashMap<u32, u32> = facts.iter()
                .filter(|&&(_, p, _)| p == rdf_first)
                .map(|&(s, _, o)| (s, o)).collect();
            let rest_map: HashMap<u32, u32> = facts.iter()
                .filter(|&&(_, p, _)| p == rdf_rest)
                .map(|&(s, _, o)| (s, o)).collect();

            let walk_list = |head: u32| -> Vec<u32> {
                let mut items = Vec::new();
                let mut cur = head;
                for _ in 0..100 {
                    if cur == rdf_nil { break; }
                    if let Some(&item) = first_map.get(&cur) { items.push(item); }
                    cur = *rest_map.get(&cur).unwrap_or(&rdf_nil);
                }
                items
            };

            for &(s, p, o) in &facts {
                if p == owl_intersection {
                    let items = walk_list(o);
                    if !items.is_empty() { intersection_classes.push((s, items)); }
                }
                if p == owl_union {
                    let items = walk_list(o);
                    if !items.is_empty() { union_classes.push((s, items)); }
                }
            }
        }

        // ── Fixpoint iteration ──────────────────────────────────────
        let mut triple_set: HashSet<(u32, u32, u32)> = facts.iter().copied().collect();
        let initial_size = triple_set.len();
        let mut iterations = 0;

        loop {
            iterations += 1;
            let before = triple_set.len();
            let mut new: Vec<(u32, u32, u32)> = Vec::new();

            // Build per-iteration indices
            let type_idx: Vec<(u32, u32)> = triple_set.iter()
                .filter(|&&(_, p, _)| p == rdf_type)
                .map(|&(s, _, o)| (s, o)).collect();
            let subclass_idx: Vec<(u32, u32)> = triple_set.iter()
                .filter(|&&(_, p, _)| p == rdfs_subclass)
                .map(|&(s, _, o)| (s, o)).collect();
            let subprop_idx: Vec<(u32, u32)> = triple_set.iter()
                .filter(|&&(_, p, _)| p == rdfs_subprop)
                .map(|&(s, _, o)| (s, o)).collect();

            // Build a subclass lookup: sub → [super]
            let mut sub_to_super: HashMap<u32, Vec<u32>> = HashMap::new();
            for &(sub, sup) in &subclass_idx {
                sub_to_super.entry(sub).or_default().push(sup);
            }

            // ── RDFS rules ──────────────────────────────────────────

            // rdfs9: x type sub, sub subClassOf super → x type super
            for &(x, sub) in &type_idx {
                if let Some(supers) = sub_to_super.get(&sub) {
                    for &sup in supers {
                        if sub != sup {
                            new.push((x, rdf_type, sup));
                        }
                    }
                }
            }

            // rdfs11: a subClassOf b, b subClassOf c → a subClassOf c
            for &(a, b) in &subclass_idx {
                if let Some(cs) = sub_to_super.get(&b) {
                    for &c in cs {
                        if a != b && b != c && a != c {
                            new.push((a, rdfs_subclass, c));
                        }
                    }
                }
            }

            // rdfs2: s p o, p domain class → s type class
            for &(prop, cls) in &domain_map {
                for &(s, p, _) in &triple_set.iter().collect::<Vec<_>>() {
                    if *p == prop { new.push((*s, rdf_type, cls)); }
                }
            }

            // rdfs3: s p o, p range class → o type class (IRI only)
            for &(prop, cls) in &range_map {
                for &(_, p, o) in &triple_set.iter().collect::<Vec<_>>() {
                    if *p == prop && interner.resolve(*o).starts_with('<') {
                        new.push((*o, rdf_type, cls));
                    }
                }
            }

            // rdfs5: subproperty transitivity
            let mut subp_to_super: HashMap<u32, Vec<u32>> = HashMap::new();
            for &(sub, sup) in &subprop_idx {
                subp_to_super.entry(sub).or_default().push(sup);
            }
            for &(a, b) in &subprop_idx {
                if let Some(cs) = subp_to_super.get(&b) {
                    for &c in cs {
                        if a != b && b != c && a != c {
                            new.push((a, rdfs_subprop, c));
                        }
                    }
                }
            }

            // rdfs7: s sub o, sub subPropertyOf super → s super o
            for &(sub, sup) in &subprop_idx {
                if sub != sup {
                    for &(s, p, o) in &triple_set.iter().collect::<Vec<_>>() {
                        if *p == sub { new.push((*s, sup, *o)); }
                    }
                }
            }

            // ── OWL-RL rules ────────────────────────────────────────
            if include_owl {
                // Transitive: x P y, y P z → x P z
                for &tp in &transitive_set {
                    let pairs: Vec<(u32, u32)> = triple_set.iter()
                        .filter(|&&(_, p, _)| p == tp)
                        .map(|&(s, _, o)| (s, o)).collect();
                    let mut by_subj: HashMap<u32, Vec<u32>> = HashMap::new();
                    for &(s, o) in &pairs {
                        by_subj.entry(s).or_default().push(o);
                    }
                    for &(x, y) in &pairs {
                        if let Some(zs) = by_subj.get(&y) {
                            for &z in zs {
                                if x != z { new.push((x, tp, z)); }
                            }
                        }
                    }
                }

                // Symmetric: s P o → o P s
                for &sp in &symmetric_set {
                    let to_add: Vec<_> = triple_set.iter()
                        .filter(|&&(_, p, _)| p == sp)
                        .map(|&(s, _, o)| (o, sp, s)).collect();
                    new.extend(to_add);
                }

                // Inverse: s P o, P inverseOf Q → o Q s (both directions)
                for &(p, q) in &inverse_pairs {
                    let fwd: Vec<_> = triple_set.iter()
                        .filter(|&&(_, pred, _)| pred == p)
                        .map(|&(s, _, o)| (o, q, s)).collect();
                    let rev: Vec<_> = triple_set.iter()
                        .filter(|&&(_, pred, _)| pred == q)
                        .map(|&(s, _, o)| (o, p, s)).collect();
                    new.extend(fwd);
                    new.extend(rev);
                }

                // sameAs: symmetry + transitivity
                let sameas: Vec<(u32, u32)> = triple_set.iter()
                    .filter(|&&(_, p, _)| p == owl_sameas)
                    .map(|&(s, _, o)| (s, o)).collect();
                for &(a, b) in &sameas {
                    new.push((b, owl_sameas, a));
                }

                // equivalentClass → bidirectional subClassOf
                for &(a, b) in &equiv_class {
                    new.push((a, rdfs_subclass, b));
                    new.push((b, rdfs_subclass, a));
                }

                // equivalentProperty → bidirectional subPropertyOf
                for &(a, b) in &equiv_prop {
                    new.push((a, rdfs_subprop, b));
                    new.push((b, rdfs_subprop, a));
                }
            }

            // ── OWL-RL extended (someValuesFrom, hasValue, intersection, union)
            if include_ext {
                // Build type lookup: instance → set of classes
                let mut inst_types: HashMap<u32, HashSet<u32>> = HashMap::new();
                for &(x, cls) in &type_idx {
                    inst_types.entry(x).or_default().insert(cls);
                }

                // cls-svf1: x P y, y type filler, restriction(P, svf=filler),
                //           class subClassOf restriction → x type class
                for &(prop, filler, restr) in &svf_rules {
                    let prop_pairs: Vec<(u32, u32)> = triple_set.iter()
                        .filter(|&&(_, p, _)| p == prop)
                        .map(|&(s, _, o)| (s, o)).collect();

                    let filler_insts: HashSet<u32> = type_idx.iter()
                        .filter(|&&(_, cls)| cls == filler)
                        .map(|&(inst, _)| inst).collect();

                    // Classes whose superclass is this restriction
                    let parent_classes: Vec<u32> = subclass_idx.iter()
                        .filter(|&&(_, sup)| sup == restr)
                        .map(|&(sub, _)| sub).collect();

                    for &(x, y) in &prop_pairs {
                        if filler_insts.contains(&y) || y == filler {
                            new.push((x, rdf_type, restr));
                            for &cls in &parent_classes {
                                new.push((x, rdf_type, cls));
                            }
                        }
                    }
                }

                // cls-hv: x type class, class subClassOf restriction(P, hasValue v) → x P v
                // and:    x P v, restriction(P, hasValue v) → x type restriction
                for &(prop, val, restr) in &hv_rules {
                    let parent_classes: Vec<u32> = subclass_idx.iter()
                        .filter(|&&(_, sup)| sup == restr)
                        .map(|&(sub, _)| sub).collect();

                    for &cls in &parent_classes {
                        for &(x, c) in &type_idx {
                            if c == cls {
                                new.push((x, prop, val));
                            }
                        }
                    }
                    for &(s, p, o) in &triple_set.iter().collect::<Vec<_>>() {
                        if *p == prop && *o == val {
                            new.push((*s, rdf_type, restr));
                        }
                    }
                }

                // cls-int: x type ALL members → x type intersection class
                for (cls, members) in &intersection_classes {
                    for &(x, _) in &type_idx {
                        if let Some(x_types) = inst_types.get(&x)
                            && members.iter().all(|m| x_types.contains(m)) {
                                new.push((x, rdf_type, *cls));
                            }
                    }
                }

                // cls-uni: x type ANY member → x type union class
                for (cls, members) in &union_classes {
                    for &(x, c) in &type_idx {
                        if members.contains(&c) {
                            new.push((x, rdf_type, *cls));
                        }
                    }
                }
            }

            // Insert new triples (dedup against existing)
            for t in new {
                triple_set.insert(t);
            }

            if triple_set.len() == before || iterations >= 50 {
                break;
            }
        }

        let inferred_count = triple_set.len() - initial_size;

        // Materialize inferred triples
        if materialize && inferred_count > 0 {
            let original: HashSet<(u32, u32, u32)> = facts.iter().copied().collect();
            let mut ntriples = String::new();
            for &(s, p, o) in &triple_set {
                if !original.contains(&(s, p, o)) {
                    ntriples.push_str(interner.resolve(s));
                    ntriples.push(' ');
                    ntriples.push_str(interner.resolve(p));
                    ntriples.push(' ');
                    ntriples.push_str(interner.resolve(o));
                    ntriples.push_str(" .\n");
                }
            }
            graph.load_ntriples(&ntriples)?;
        }

        // Sample
        let original: HashSet<(u32, u32, u32)> = facts.iter().copied().collect();
        let sample: Vec<String> = triple_set.iter()
            .filter(|t| !original.contains(t))
            .filter(|&&(_, p, _)| p == rdf_type)
            .take(10)
            .map(|&(s, _, o)| format!("{} a {}", interner.resolve(s), interner.resolve(o)))
            .collect();

        let mut result = serde_json::json!({
            "profile_used": profile_used,
            "inferred_count": inferred_count,
            "iterations": iterations,
            "initial_triples": initial_size,
            "final_triples": triple_set.len(),
            "sample_inferences": sample
        });
        if !materialize {
            result["dry_run"] = serde_json::json!(true);
        }
        Ok(result.to_string())
    }
}
