//! `--explain`: the cause tree of one drawn value, as text.
//!
//! It works on the report serialised to JSON, so it needs no type of the
//! engines: a traced value is an object with exactly `value` and `node`,
//! and its graph is the `causes.nodes` of its nearest ancestor that has
//! a `causes` key (a keyframe's own graph, or the report's for the drawn
//! masses of `planets`).

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
    Ambiguous {
        code: String,
        /// How many distinct cause trees hold the code.
        trees: usize,
        pointers: Vec<String>,
        /// The pointers lead to values that depend on the code: no field
        /// holds it itself.
        indirect: bool,
    },
}

impl fmt::Display for ExplainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { path } => write!(f, "cli.explain.not-found: nothing at {path}"),
            Self::NotTraced { path } => {
                write!(f, "cli.explain.not-traced: {path} has no cause tree")
            }
            Self::Ambiguous {
                code,
                trees,
                pointers,
                indirect,
            } => {
                let list = pointers.join(", ");
                if *indirect {
                    write!(
                        f,
                        "cli.explain.ambiguous: {code} is drawn {trees} times, held by no field; \
                         values that depend on it: {list}"
                    )
                } else {
                    write!(
                        f,
                        "cli.explain.ambiguous: {code} is drawn {trees} times, pick one of: {list}"
                    )
                }
            }
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

/// Every graph of the report: its own and each keyframe's, of every
/// timeline.
fn graphs(report: &Value) -> Vec<&Vec<Value>> {
    let keyframes = report
        .get("timelines")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|timeline| timeline.get("keyframes")?.as_array())
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
            let trees = trees.len();
            let mut pointers = Vec::new();
            collect_pointers(report, code, false, None, "", &mut pointers);
            let indirect = pointers.is_empty();
            if indirect {
                collect_pointers(report, code, true, None, "", &mut pointers);
            }
            Err(ExplainError::Ambiguous {
                code: code.to_owned(),
                trees,
                pointers,
                indirect,
            })
        }
    }
}

/// Whether the tree of node `id` holds a node with `code`.
fn tree_has(nodes: &[Value], id: usize, code: &str, seen: &mut BTreeSet<usize>) -> bool {
    if !seen.insert(id) {
        return false;
    }
    let Some(node) = nodes.get(id) else {
        return false;
    };
    if node.get("code").and_then(Value::as_str) == Some(code) {
        return true;
    }
    node.pointer("/origin/Derived/from")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|source| usize::try_from(source.as_u64()?).ok())
        .any(|source| tree_has(nodes, source, code, seen))
}

/// Walks the JSON depth-first and lists the traced values whose own node
/// has `code` or, with `depending`, whose tree holds a node with it.
fn collect_pointers(
    value: &Value,
    code: &str,
    depending: bool,
    graph: Option<&Vec<Value>>,
    pointer: &str,
    out: &mut Vec<String>,
) {
    if let (Some(nodes), Some(id)) = (graph, traced_node(value)) {
        let found = if depending {
            tree_has(nodes, id, code, &mut BTreeSet::new())
        } else {
            nodes
                .get(id)
                .and_then(|node| node.get("code"))
                .and_then(Value::as_str)
                == Some(code)
        };
        if found {
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
                collect_pointers(
                    child,
                    code,
                    depending,
                    graph,
                    &format!("{pointer}/{key}"),
                    out,
                );
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                collect_pointers(
                    child,
                    code,
                    depending,
                    graph,
                    &format!("{pointer}/{index}"),
                    out,
                );
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

/// A number for reading: plain up to a million and down to a thousandth,
/// scientific with 5 significant digits (`1.1447e30`) beyond, so that a
/// stellar mass in kilograms does not print thirty digits. Only the text
/// is rounded; the JSON output keeps full precision.
fn render_number(n: f64) -> String {
    let magnitude = n.abs();
    if magnitude >= 1e6 || (magnitude > 0.0 && magnitude < 1e-3) {
        format!("{n:.4e}")
    } else {
        format!("{n}")
    }
}

/// A node's value: a number with its unit (none for `1`), a code, a flag.
fn render_value(value: &Value) -> Option<String> {
    if let Some(number) = value.get("Number") {
        let n = number.get("value")?.as_f64()?;
        return Some(match number.get("unit")?.as_str()? {
            "1" | "" => render_number(n),
            unit => format!("{} {unit}", render_number(n)),
        });
    }
    if let Some(code) = value.get("Code") {
        return Some(code.as_str()?.to_owned());
    }
    Some(value.get("Flag")?.as_bool()?.to_string())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn numbers_read_plainly_or_in_scientific_notation() {
        let cases = [
            (1_144_684_557_374_294_400_000_000_000_000.0, "1.1447e30"),
            (232_006_601.403_750_3, "2.3201e8"),
            (999_999.0, "999999"),
            (1e6, "1.0000e6"),
            (0.001, "0.001"),
            (0.0009, "9.0000e-4"),
            (0.0, "0"),
            (-2.5e7, "-2.5000e7"),
            (5.350_512_601_505_564, "5.350512601505564"),
        ];
        for (n, text) in cases {
            assert_eq!(render_number(n), text, "{n}");
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
