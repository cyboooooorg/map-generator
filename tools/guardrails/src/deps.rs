//! Direction of the dependencies between the workspace crates.

use std::collections::{BTreeMap, BTreeSet};

use cargo_metadata::Metadata;

/// Dependencies of one workspace member, split into workspace members
/// (`internal`) and everything else (`external`). Normal, build and dev
/// dependencies alike: a dev-dependency that crosses a boundary breaks it
/// as surely as a normal one.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Edges {
    pub internal: BTreeSet<String>,
    pub external: BTreeSet<String>,
}

/// The workspace graph: member name to its dependencies.
pub type Graph = BTreeMap<String, Edges>;

/// Allowed workspace-internal edges, the graph of the architecture design
/// (Architecture > Crates). A member missing here is a violation, so a new
/// crate must be placed in the graph before it builds green.
pub fn allowed() -> BTreeMap<&'static str, BTreeSet<&'static str>> {
    let engines = ["science", "timeline", "system", "world", "catalogue"];
    let set = |names: &[&'static str]| names.iter().copied().collect::<BTreeSet<_>>();
    let mut every_engine_and_adapters = set(&engines);
    every_engine_and_adapters.insert("adapters");
    let mut regression = every_engine_and_adapters.clone();
    regression.insert("map-generator");
    BTreeMap::from([
        ("science", set(&[])),
        ("timeline", set(&["science"])),
        ("system", set(&["timeline", "science"])),
        ("world", set(&["timeline", "science"])),
        ("catalogue", set(&["system", "timeline", "science"])),
        ("adapters", set(&engines)),
        ("map-generator", every_engine_and_adapters.clone()),
        // Test-only: the tier-2 tests run the CLI's library (`dispatch`).
        ("regression", regression),
        ("guardrails", set(&[])),
        ("opacity-table", set(&[])),
        ("stellar-spectra", set(&[])),
        ("rfmip-shortwave", set(&[])),
        ("figures", set(&["science"])),
    ])
}

/// External dependencies a member may have, when restricted. `science`
/// holds pure laws: each external crate it needs is added here on purpose,
/// with the reason in the commit. `None` means unrestricted.
pub fn external_allowed(package: &str) -> Option<&'static [&'static str]> {
    match package {
        "science" => Some(&["serde", "serde_json"]),
        "timeline" => Some(&["serde", "serde_json"]),
        "system" => Some(&["serde", "serde_json", "rand_core", "rand_chacha"]),
        _ => None,
    }
}

/// The workspace graph read from `cargo metadata`.
pub fn graph(meta: &Metadata) -> Graph {
    let members: BTreeSet<&str> = meta
        .workspace_packages()
        .iter()
        .map(|package| package.name.as_str())
        .collect();
    meta.workspace_packages()
        .iter()
        .map(|package| {
            let mut edges = Edges::default();
            for dependency in &package.dependencies {
                let name = dependency.name.clone();
                if members.contains(name.as_str()) {
                    edges.internal.insert(name);
                } else {
                    edges.external.insert(name);
                }
            }
            (package.name.to_string(), edges)
        })
        .collect()
}

/// One line per forbidden edge, per forbidden external dependency and per
/// member absent from [`allowed`].
pub fn violations(graph: &Graph) -> Vec<String> {
    let allowed = allowed();
    let mut found = Vec::new();
    for (member, edges) in graph {
        let Some(permitted) = allowed.get(member.as_str()) else {
            found.push(format!(
                "{member}: not in the allowlist of tools/guardrails/src/deps.rs"
            ));
            continue;
        };
        for target in &edges.internal {
            if !permitted.contains(target.as_str()) {
                found.push(format!("{member} -> {target}"));
            }
        }
        if let Some(external) = external_allowed(member) {
            for target in &edges.external {
                if !external.contains(&target.as_str()) {
                    found.push(format!("{member} -> {target} (external)"));
                }
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph_of(entries: &[(&str, &[&str], &[&str])]) -> Graph {
        entries
            .iter()
            .map(|(name, internal, external)| {
                let edges = Edges {
                    internal: internal.iter().map(|s| s.to_string()).collect(),
                    external: external.iter().map(|s| s.to_string()).collect(),
                };
                (name.to_string(), edges)
            })
            .collect()
    }

    #[test]
    fn the_designed_graph_has_no_violation() {
        let graph = graph_of(&[
            ("science", &[], &[]),
            ("timeline", &["science"], &["serde"]),
            ("world", &["timeline", "science"], &[]),
            ("adapters", &["world", "system"], &["png"]),
        ]);
        assert_eq!(violations(&graph), Vec::<String>::new());
    }

    #[test]
    fn world_depending_on_system_is_a_violation() {
        let graph = graph_of(&[("world", &["timeline", "system"], &[])]);
        assert_eq!(violations(&graph), ["world -> system"]);
    }

    #[test]
    fn a_domain_crate_depending_on_adapters_is_a_violation() {
        let graph = graph_of(&[("catalogue", &["adapters"], &[])]);
        assert_eq!(violations(&graph), ["catalogue -> adapters"]);
    }

    #[test]
    fn science_depends_on_nothing_external_unless_listed() {
        let graph = graph_of(&[("science", &[], &["rand"])]);
        assert_eq!(violations(&graph), ["science -> rand (external)"]);
    }

    #[test]
    fn an_unknown_member_is_a_violation() {
        let graph = graph_of(&[("newcrate", &[], &[])]);
        assert_eq!(
            violations(&graph),
            ["newcrate: not in the allowlist of tools/guardrails/src/deps.rs"]
        );
    }
}
