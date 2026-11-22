//! `--explain`: the cause tree of one drawn value, as text.
//!
//! It works on the report serialised to JSON, so it needs no type of the
//! engines: a traced value is an object with exactly `value` and `node`,
//! and its graph is the `causes.nodes` of its nearest ancestor that has
//! a `causes` key (a star keyframe's own graph, or the report's).

use std::collections::BTreeSet;
use std::fmt::{self, Write as _};

use serde_json::Value;

/// Why a value could not be explained. `Display` starts with the code.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExplainError {
    /// Nothing at the pointer, or no node with the code.
    NotFound { path: String },
    /// The pointer leads to a value that has no cause tree.
    NotTraced { path: String },
    /// Several traced values share the code: pick one by pointer.
    Ambiguous { code: String, pointers: Vec<String> },
}

impl fmt::Display for ExplainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { path } => write!(f, "cli.explain.not-found: nothing at {path}"),
            Self::NotTraced { path } => {
                write!(f, "cli.explain.not-traced: {path} has no cause tree")
            }
            Self::Ambiguous { code, pointers } => write!(
                f,
                "cli.explain.ambiguous: {code} is drawn {} times, pick one of: {}",
                pointers.len(),
                pointers.join(", ")
            ),
        }
    }
}

impl std::error::Error for ExplainError {}

/// Explains `path` in `report`: a JSON pointer (starts with `/`) to a
/// traced value, or a cause code.
pub fn explain(report: &Value, path: &str) -> Result<String, ExplainError> {
    if path.starts_with('/') {
        explain_pointer(report, path)
    } else {
        explain_code(report, path)
    }
}

fn not_found(path: &str) -> ExplainError {
    ExplainError::NotFound {
        path: path.to_owned(),
    }
}

/// The node id of a traced value: an object with exactly `value` and `node`.
fn traced_node(value: &Value) -> Option<usize> {
    let object = value.as_object()?;
    if object.len() != 2 || !object.contains_key("value") {
        return None;
    }
    usize::try_from(object.get("node")?.as_u64()?).ok()
}

fn nodes_of(holder: &Value) -> Option<&Vec<Value>> {
    holder.get("causes")?.get("nodes")?.as_array()
}

fn explain_pointer(report: &Value, path: &str) -> Result<String, ExplainError> {
    let target = report.pointer(path).ok_or_else(|| not_found(path))?;
    let node = traced_node(target).ok_or_else(|| ExplainError::NotTraced {
        path: path.to_owned(),
    })?;
    // The nearest ancestor with a graph: try the longest prefix first.
    let mut end = path.len();
    loop {
        end = path[..end].rfind('/').unwrap_or(0);
        let holder = if end == 0 {
            Some(report)
        } else {
            report.pointer(&path[..end])
        };
        if let Some(nodes) = holder.and_then(nodes_of) {
            return render_tree(nodes, node).ok_or_else(|| not_found(path));
        }
        if end == 0 {
            return Err(not_found(path));
        }
    }
}

/// Every graph of the report: its own and each keyframe's.
fn graphs(report: &Value) -> Vec<&Vec<Value>> {
    let keyframes = report
        .pointer("/star/keyframes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten();
    nodes_of(report)
        .into_iter()
        .chain(keyframes.filter_map(nodes_of))
        .collect()
}

fn explain_code(report: &Value, code: &str) -> Result<String, ExplainError> {
    let mut trees: Vec<String> = Vec::new();
    for nodes in graphs(report) {
        for (id, node) in nodes.iter().enumerate() {
            if node.get("code").and_then(Value::as_str) == Some(code)
                && let Some(tree) = render_tree(nodes, id)
                && !trees.contains(&tree)
            {
                trees.push(tree);
            }
        }
    }
    match trees.len() {
        0 => Err(not_found(code)),
        1 => Ok(trees.remove(0)),
        _ => {
            let mut pointers = Vec::new();
            collect_pointers(report, code, None, "", &mut pointers);
            Err(ExplainError::Ambiguous {
                code: code.to_owned(),
                pointers,
            })
        }
    }
}

/// Walks the JSON depth-first and lists the traced values whose node has
/// `code` in their own graph.
fn collect_pointers(
    value: &Value,
    code: &str,
    graph: Option<&Vec<Value>>,
    pointer: &str,
    out: &mut Vec<String>,
) {
    if let (Some(nodes), Some(id)) = (graph, traced_node(value)) {
        let found = nodes
            .get(id)
            .and_then(|node| node.get("code"))
            .and_then(Value::as_str);
        if found == Some(code) {
            out.push(pointer.to_owned());
        }
        return;
    }
    let graph = nodes_of(value).or(graph);
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if key == "causes" {
                    continue;
                }
                let key = key.replace('~', "~0").replace('/', "~1");
                collect_pointers(child, code, graph, &format!("{pointer}/{key}"), out);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                collect_pointers(child, code, graph, &format!("{pointer}/{index}"), out);
            }
        }
        _ => {}
    }
}

/// The tree of node `root`, one line per node; `None` if an id is out of range.
fn render_tree(nodes: &[Value], root: usize) -> Option<String> {
    let mut out = String::new();
    let mut seen = BTreeSet::new();
    render_node(nodes, root, 0, &mut seen, &mut out)?;
    out.pop();
    Some(out)
}

fn render_node(
    nodes: &[Value],
    id: usize,
    depth: usize,
    seen: &mut BTreeSet<usize>,
    out: &mut String,
) -> Option<()> {
    let node = nodes.get(id)?;
    let code = node.get("code")?.as_str()?;
    let indent = "  ".repeat(depth);
    if !seen.insert(id) {
        let _ = writeln!(out, "{indent}{code} (see above)");
        return Some(());
    }
    let value = render_value(node.get("value")?)?;
    let origin = node.get("origin")?;
    if let Some(provenance) = origin.get("Leaf").and_then(Value::as_str) {
        let _ = writeln!(
            out,
            "{indent}{code} = {value} [{}]",
            provenance.to_lowercase()
        );
    } else {
        let _ = writeln!(out, "{indent}{code} = {value}");
        for source in origin.get("Derived")?.get("from")?.as_array()? {
            let source = usize::try_from(source.as_u64()?).ok()?;
            render_node(nodes, source, depth + 1, seen, out)?;
        }
    }
    Some(())
}

/// A node's value: a number with its unit (none for `1`), a code, a flag.
fn render_value(value: &Value) -> Option<String> {
    if let Some(number) = value.get("Number") {
        let n = number.get("value")?.as_f64()?;
        return Some(match number.get("unit")?.as_str()? {
            "1" | "" => format!("{n}"),
            unit => format!("{n} {unit}"),
        });
    }
    if let Some(code) = value.get("Code") {
        return Some(code.as_str()?.to_owned());
    }
    Some(value.get("Flag")?.as_bool()?.to_string())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;
    use system::{INPUTS_SCHEMA, Inputs, Request, SystemRequest};
    use timeline::Seed;

    use super::*;
    use crate::report::report;

    fn report_of(seed: u64) -> Value {
        let inputs = Inputs {
            schema: INPUTS_SCHEMA,
            seed: Seed::new(seed),
            present: None,
            request: Request::System(SystemRequest {
                host_mass: None,
                metallicity: None,
                archetypes: BTreeMap::new(),
            }),
        };
        serde_json::to_value(report(&inputs).unwrap()).unwrap()
    }

    #[test]
    fn a_pointer_to_a_period_explains_it() {
        let text = explain(&report_of(1), "/planets/0/orbit/period").unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[0].starts_with("system.draw.period = "));
        assert!(text.contains("system.star.mass"));
        let depth = |line: &str| line.len() - line.trim_start().len();
        for (i, line) in lines.iter().enumerate() {
            let has_sources = lines
                .get(i + 1)
                .is_some_and(|next| depth(next) > depth(line));
            assert!(
                has_sources || line.ends_with(']') || line.ends_with("(see above)"),
                "{line}"
            );
        }
    }

    #[test]
    fn a_unique_code_explains_it() {
        let text = explain(&report_of(1), "system.star.mass").unwrap();
        assert!(text.starts_with("system.star.mass = "));
        assert_eq!(text.lines().count(), 1);
    }

    #[test]
    fn a_repeated_code_is_ambiguous() {
        let (seed, report) = (1..200)
            .map(|seed| (seed, report_of(seed)))
            .find(|(_, report)| report["planets"].as_array().is_some_and(|p| p.len() >= 2))
            .expect("a seed draws two planets");
        match explain(&report, "system.draw.flux") {
            Err(ExplainError::Ambiguous { pointers, .. }) => {
                let planets = report["planets"].as_array().unwrap().len();
                assert_eq!(pointers.len(), planets, "seed {seed}");
                assert!(pointers.iter().all(|p| p.starts_with("/planets/")));
            }
            other => panic!("expected Ambiguous, got {other:?}"),
        }
    }

    #[test]
    fn a_pointer_to_a_designation_is_not_traced() {
        let error = explain(&report_of(1), "/planets/0/designation").unwrap_err();
        assert!(matches!(error, ExplainError::NotTraced { .. }));
    }

    #[test]
    fn unknown_paths_are_not_found() {
        let report = report_of(1);
        for path in ["/planets/99/mass", "system.draw.nothing"] {
            let error = explain(&report, path).unwrap_err();
            assert!(matches!(error, ExplainError::NotFound { .. }), "{path}");
        }
    }

    #[test]
    fn a_shared_source_prints_once() {
        let report = json!({
            "causes": { "nodes": [
                { "code": "a.root", "value": { "Number": { "value": 1.0, "unit": "au" } },
                  "origin": { "Leaf": "Drawn" } },
                { "code": "a.mid", "value": { "Flag": true },
                  "origin": { "Derived": { "from": [0] } } },
                { "code": "a.top", "value": { "Code": "plasma" },
                  "origin": { "Derived": { "from": [0, 1] } } },
            ] },
            "x": { "value": 3, "node": 2 },
        });
        let text = explain(&report, "/x").unwrap();
        assert_eq!(
            text,
            "a.top = plasma\n  a.root = 1 au [drawn]\n  a.mid = true\n    a.root (see above)"
        );
    }
}
