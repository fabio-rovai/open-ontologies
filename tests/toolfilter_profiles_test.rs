//! Task profiles: the named subsets of the tool surface, and the one property
//! that has to hold for ever.
//!
//! THE COVERAGE TEST IS THE POINT. The profile table in `src/toolfilter.rs` is
//! a hand-maintained list of names next to a router that grows on its own, and
//! the failure mode is not a wrong grouping: it is a tool added to
//! `src/server.rs` next year, filed under nothing, reachable only from `full`,
//! and never missed, because a tool that is never advertised is never missed.
//! `every_registered_tool_is_in_some_profile` turns that into a failure on the
//! commit that causes it, which is the only moment anyone can fix it.
//!
//! The rest pins what makes profiles safe to ship: the default does not move, a
//! profile only narrows, the spine is in all of them, and an unknown name is
//! refused rather than widened to `full`.

use std::collections::BTreeSet;
use std::sync::Arc;

use open_ontologies::config::{
    CacheConfig, EmbeddingsConfig, ToolsConfig, resolve_tool_profile_from,
};
use open_ontologies::graph::GraphStore;
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use open_ontologies::toolfilter::{
    self, FEATURE_GATED_TOOLS, FULL, Mode, SPINE, ToolFilter, expand_profile, is_profile,
    profile_description, profiles,
};

fn build_server(filter: ToolFilter) -> (tempfile::TempDir, OpenOntologiesServer) {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("s.db")).unwrap();
    let cache = CacheConfig {
        enabled: true,
        dir: tmp.path().join("cache").to_string_lossy().into_owned(),
        idle_ttl_secs: 0,
        evictor_interval_secs: 30,
        auto_refresh: false,
        hash_prefix_bytes: 64 * 1024,
    };
    let server = OpenOntologiesServer::new_with_registry_options(
        db,
        Arc::new(GraphStore::new()),
        None,
        EmbeddingsConfig::default(),
        cache,
        filter,
    );
    (tmp, server)
}

fn tool_names(server: &OpenOntologiesServer) -> BTreeSet<String> {
    server.list_tool_definitions().into_iter().map(|t| t.name.to_string()).collect()
}

/// Every tool THIS build advertises by default, read off the router. A list
/// written here would be the same hand-maintained thing the test exists to
/// police.
fn advertised_by_default() -> BTreeSet<String> {
    let (_tmp, server) = build_server(ToolFilter::default());
    tool_names(&server)
}

/// Every tool the binary registers, feature-gated ones included.
///
/// `remove_unavailable` runs before the operator's filter, so in a default
/// build the router has already lost the eight tools behind a Cargo feature.
/// The union is what a profile name is legitimately allowed to be: CI runs
/// `cargo test` with default features and again with `--all-features`, and a
/// ghost test that read only the router would call `onto_embed` a dead string
/// in the first job and not the second.
fn nameable() -> BTreeSet<String> {
    let mut out = advertised_by_default();
    out.extend(FEATURE_GATED_TOOLS.iter().map(|(t, _)| t.to_string()));
    out
}

#[test]
fn every_registered_tool_is_in_some_profile() {
    let mut covered: BTreeSet<String> = BTreeSet::new();
    for name in profiles() {
        if name == FULL {
            continue;
        }
        for t in expand_profile(name).expect("a listed profile expands") {
            covered.insert(t.to_string());
        }
    }
    let registered = nameable();
    let orphans: Vec<&String> = registered.difference(&covered).collect();
    assert!(
        orphans.is_empty(),
        "{} registered tool(s) belong to no profile, so a client that selects any \
         profile can never reach them: {:?}\nAdd each one to a profile in \
         src/toolfilter.rs. `full` does not count: it is the absence of a profile, \
         not a home for orphans.",
        orphans.len(),
        orphans
    );
}

#[test]
fn no_profile_names_a_tool_that_does_not_exist() {
    // The other direction. A renamed tool leaves a dead string behind, the
    // profile silently shrinks by one, and nothing says so.
    let registered = nameable();
    let mut ghosts = Vec::new();
    for name in profiles() {
        if name == FULL {
            continue;
        }
        for t in expand_profile(name).unwrap() {
            if !registered.contains(t) {
                ghosts.push(format!("{name}: {t}"));
            }
        }
    }
    assert!(
        ghosts.is_empty(),
        "profile(s) name tools that are not registered, so the profile is quietly \
         smaller than it reads: {ghosts:?}"
    );
}

#[test]
fn the_default_is_still_every_tool() {
    let (_tmp, server) = build_server(ToolFilter::default());
    let status: serde_json::Value = serde_json::from_str(&server.status_json()).unwrap();
    assert_eq!(
        status["tool_surface"]["exposed"], status["tool_surface"]["servable_in_this_build"],
        "the default filter must advertise every tool this build can serve. Existing \
         clients were built against that surface and shrinking it under them is worse \
         than the discoverability problem profiles are for."
    );
    assert_eq!(status["tool_surface"]["withheld_by_filter"], 0);
    assert_eq!(status["tool_surface"]["profile"], "full");
    // The three counters have to stay three. Folding `servable_in_this_build`
    // into `compiled_in` leaves the assertions above still passing, because
    // both sides move together when the filter withholds nothing, so the
    // default case needs the invariant stated against the feature gate
    // directly: what this build can serve is what it compiled in MINUS what
    // the Cargo features took away, and `tools` is the router length the same
    // call reports. Without these two the collapse the plan warns about is
    // caught only by the profile test, and a default-features CI job would
    // publish a false servable count.
    assert_eq!(status["tools"], status["tool_surface"]["exposed"]);
    let compiled = status["tool_surface"]["compiled_in"].as_u64().unwrap();
    let gated = toolfilter::unavailable_in_this_build().len() as u64;
    assert_eq!(
        status["tool_surface"]["servable_in_this_build"].as_u64().unwrap(),
        compiled - gated,
        "servable must be counted AFTER remove_unavailable and compiled_in before it"
    );
}

#[test]
fn full_is_the_same_surface_as_no_profile_at_all() {
    let (_a, default_server) = build_server(ToolFilter::default());
    let (_b, full_server) = build_server(ToolFilter::with_profile(FULL).unwrap());
    assert_eq!(tool_names(&default_server), tool_names(&full_server));
}

#[test]
fn every_profile_is_smaller_than_full_and_bigger_than_the_spine() {
    let total = advertised_by_default().len();
    for name in profiles() {
        if name == FULL {
            continue;
        }
        let (_tmp, server) = build_server(ToolFilter::with_profile(name).unwrap());
        let n = tool_names(&server).len();
        assert!(
            n < total,
            "profile {name:?} advertises {n} of {total}. A profile that is not smaller \
             is not a profile; a smaller surface is the whole point."
        );
        assert!(
            n > SPINE.len(),
            "profile {name:?} advertises {n} tools, which is the spine and almost \
             nothing else. It cannot do the job it is named for."
        );
    }
}

#[test]
fn every_profile_carries_the_spine() {
    for name in profiles() {
        if name == FULL {
            continue;
        }
        let (_tmp, server) = build_server(ToolFilter::with_profile(name).unwrap());
        let exposed = tool_names(&server);
        for t in SPINE {
            assert!(
                exposed.contains(*t),
                "profile {name:?} is missing {t}, so a client cannot load a graph or see \
                 what it loaded and is stuck inside its profile"
            );
        }
    }
}

#[test]
fn every_profile_says_what_it_is_for() {
    for name in profiles() {
        let d = profile_description(name).unwrap_or_else(|| panic!("{name} has no description"));
        assert!(d.len() > 40, "profile {name:?} description is a stub: {d:?}");
    }
}

#[test]
fn a_selected_profile_is_visible_in_the_servers_own_reporting() {
    let (_tmp, server) = build_server(ToolFilter::with_profile("alignment").unwrap());
    let status: serde_json::Value = serde_json::from_str(&server.status_json()).unwrap();
    assert_eq!(status["tool_surface"]["profile"], "alignment");
    let exposed = status["tool_surface"]["exposed"].as_u64().unwrap();
    let servable = status["tool_surface"]["servable_in_this_build"].as_u64().unwrap();
    let withheld = status["tool_surface"]["withheld_by_filter"].as_u64().unwrap();
    assert!(withheld > 0 && exposed + withheld == servable);
    // `tools` is the count a client sees in tools/list and it has to match the
    // surface the same call describes, or the report is about a different server
    // than the one answering.
    assert_eq!(status["tools"].as_u64().unwrap(), exposed);
    // The way out is printed, so a user missing a tool does not have to find the
    // documentation to learn that profiles exist.
    let available = status["tool_profiles_available"].as_array().unwrap();
    assert!(available.iter().any(|v| v == "full"));
}

#[test]
fn allow_list_cannot_add_back_a_tool_the_profile_dropped() {
    // `onto_plan_classical` is in `planning` and not in `alignment`. Naming it in
    // --tools-allow must not resurrect it: both axes narrow, and a mode that
    // could widen a profile would make the profile advisory.
    let filter = ToolFilter {
        mode: Mode::Allow,
        list: vec!["onto_plan_classical".to_string(), "onto_align".to_string()],
        groups: vec![],
        profile: Some("alignment".to_string()),
    };
    let (_tmp, server) = build_server(filter);
    let exposed = tool_names(&server);
    assert!(!exposed.contains("onto_plan_classical"));
    assert!(exposed.contains("onto_align"));
}

#[test]
fn deny_narrows_a_profile_further() {
    let filter = ToolFilter {
        mode: Mode::Deny,
        list: vec!["onto_align".to_string()],
        groups: vec![],
        profile: Some("alignment".to_string()),
    };
    let (_tmp, server) = build_server(filter);
    let exposed = tool_names(&server);
    assert!(!exposed.contains("onto_align"));
    assert!(exposed.contains("onto_crosswalk"));
    assert!(exposed.contains("onto_status"));
}

#[test]
fn an_unknown_profile_is_an_error_and_not_a_silent_full() {
    let err = ToolFilter::with_profile("authorring").unwrap_err();
    assert!(err.contains("authorring"), "{err}");
    assert!(err.contains("authoring"), "the error must list the real names: {err}");
    assert!(!is_profile("authorring"));
    assert!(expand_profile("authorring").is_none());
}

#[test]
fn a_filter_holding_an_unknown_profile_exposes_nothing_rather_than_everything() {
    // Not reachable through the CLI, which refuses to start. Pinned because the
    // conservative reading of a name nobody recognises is "no tools" and the
    // dangerous reading is "all of them".
    let filter = ToolFilter { profile: Some("nonsense".to_string()), ..Default::default() };
    assert!(!filter.allows("onto_status"));
    let (_tmp, server) = build_server(filter);
    assert!(tool_names(&server).is_empty());
}

#[test]
fn profile_resolution_is_cli_then_env_then_config_then_full() {
    // Through the pure resolver: the process environment is shared mutable
    // state and cargo runs these tests on parallel threads, which is why
    // config.rs has a `_from` half at all.
    let cfg = ToolsConfig { profile: "governance".to_string(), ..Default::default() };
    assert_eq!(resolve_tool_profile_from(&cfg, Some("reasoning"), Some("data")), "reasoning");
    assert_eq!(resolve_tool_profile_from(&cfg, None, Some("data")), "data");
    assert_eq!(resolve_tool_profile_from(&cfg, None, None), "governance");
    // An empty string at any level is "not set", not "the empty profile".
    assert_eq!(resolve_tool_profile_from(&cfg, Some("  "), None), "governance");
    assert_eq!(resolve_tool_profile_from(&ToolsConfig::default(), None, Some("")), FULL);
    // Every name the resolver can return has to be one the filter accepts.
    for n in [
        resolve_tool_profile_from(&cfg, None, None),
        resolve_tool_profile_from(&ToolsConfig::default(), None, None),
    ] {
        assert!(is_profile(&n), "{n} resolved but is not a profile");
    }
}

#[test]
fn the_older_group_mechanism_still_works_with_no_profile() {
    let filter = ToolFilter {
        mode: Mode::Allow,
        list: vec![],
        groups: vec!["read_only".to_string()],
        profile: None,
    };
    let (_tmp, server) = build_server(filter);
    let exposed = tool_names(&server);
    assert!(exposed.contains("onto_query"));
    assert!(!exposed.contains("onto_push"));
    let gated: BTreeSet<&str> = FEATURE_GATED_TOOLS.iter().map(|(t, _)| *t).collect();
    for t in toolfilter::expand_group("read_only") {
        // `onto_search` and `onto_similarity` are in the group and are not in a
        // default build, and the group mechanism is not what removed them.
        if gated.contains(t) && !advertised_by_default().contains(*t) {
            continue;
        }
        assert!(exposed.contains(*t), "{t} dropped from the read_only group");
    }
}
