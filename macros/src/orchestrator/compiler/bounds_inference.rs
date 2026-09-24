use std::collections::HashSet;

use super::ast::*;

pub(super) fn infer_graph_bounds(ast: &SyntaxTree) -> (Bound, Bound) {
    // Idents consumed as a Source anchor: `[x] ->`, a binding at a line entry
    let mut source_used = HashSet::new();

    // Idents consumed as a Sink anchor: `-> [x]`, a binding at a line conn
    let mut sink_used = HashSet::new();

    // Idents that appear as a Non-last binding (`-> [x] ->`):
    // these are intermediate nodes, NOT sinks (i.e., they have outgoing edges)
    let mut intermediate_nodes = HashSet::new();

    for g in &ast.graphs {
        if let NodeExpr::Binding(var) = &g.entry {
            source_used.insert(var.to_string());
        }

        // every binding is a sink anchor,
        // and every binding but the last one is also an intermediate node
        let n = g.conns.len();
        for (i, node) in g.conns.iter().enumerate() {
            if let NodeExpr::Binding(var) = node {
                let name = var.to_string();
                if i + 1 < n {
                    intermediate_nodes.insert(name.clone());
                }
                sink_used.insert(name);
            }
        }
    }

    let distinct_sources = ast
        .graphs
        .iter()
        .filter(|g| match &g.entry {
            // continuation, refers to an existing node, not a new source
            NodeExpr::Binding(_) => false,
            // a group entry introduces sources (checked via node_input_bound below)
            NodeExpr::Group(_) => true,
            // an embedding is always a distinct source
            NodeExpr::Expression { .. } => true,
            NodeExpr::Declaration(d) => match &d.var {
                // anonymous declarations are always distinct source nodes
                None => true,
                Some(v) => {
                    // has outgoing edges on its own line
                    !g.conns.is_empty()
                    // is used as a source elsewhere
                    // or is never consumed as a sink (an isolated node)
                    || {
                        let name = v.to_string();
                        source_used.contains(&name) || !sink_used.contains(&name)
                    }
                }
            },
        })
        .count();

    // Multiple of distinct sources make up a Fork
    let input = if distinct_sources > 1 {
        Bound::Fork
    }
    // Otherwise the entry of the first top-level graph decides
    else if let Some(g) = ast.graphs.first() {
        node_input_bound(&g.entry)
    } else {
        Bound::Join
    };

    // Multiple of distinct sink nodes make up a Fork
    let output = match count_distinct_sinks(ast, &source_used, &intermediate_nodes) {
        Some(n) if n > 1 => Bound::Fork,

        // fallback to the structural bound of the last top-level graph
        _ => ast
            .graphs
            .last()
            // when opaque `@`-embedding is at the sink edge or it is a single sink
            // it is either `Ambiguous` or user-annotated bound (`Fork` or `Join`)
            .map(graph_output_bound)
            .unwrap_or(Bound::Join),
    };

    (input, output)
}

/// Number of distinct sink nodes across the whole graph.\
/// Returns `None` when an opaque (unannotated) `@`-embedding sits at a sink edge,\
/// so the caller falls back to structural analysis.
///
/// A sink is a node with zero out-degree:
/// - a declared node is a sink only if it is NOT used as a `[x] ->` source
/// - bindings (`-> [x]`) join into an existing node, deduping by var name
/// - group terminals recurse into their members
fn count_distinct_sinks(
    ast: &SyntaxTree,
    source_used: &HashSet<String>,
    intermediate_names: &HashSet<String>,
) -> Option<usize> {
    // Named sinks are deduped by name
    // Anonymous ones are distinct by construction, so they're counted rather than keyed
    let mut named = HashSet::new();
    let mut anon = 0;
    let mut opaque = false;

    for g in &ast.graphs {
        collect(
            g,
            source_used,
            intermediate_names,
            &mut named,
            &mut anon,
            &mut opaque,
        );
    }

    if opaque {
        None
    } else {
        Some(named.len() + anon)
    }
}

fn collect(
    g: &Graph,
    source_used: &HashSet<String>,
    intermediate_names: &HashSet<String>,
    named: &mut HashSet<String>,
    anon: &mut usize,
    opaque: &mut bool,
) {
    let last = g.conns.last().unwrap_or(&g.entry);
    match last {
        NodeExpr::Declaration(d) => match &d.var {
            Some(v) => insert_if_sink(named, v.to_string(), source_used, intermediate_names),
            // anonymous declarations are each a distinct sink
            None => *anon += 1,
        },
        NodeExpr::Binding(v) => {
            insert_if_sink(named, v.to_string(), source_used, intermediate_names)
        }
        NodeExpr::Expression {
            explicit_bounds, ..
        } => match explicit_bounds {
            Some((_, Bound::Fork)) => {
                *anon += 2;
            }
            Some((_, _)) => *anon += 1,
            None => *opaque = true,
        },
        NodeExpr::Group(block) => match block.mode {
            SchedulingMode::Fork => {
                // each parallel member terminates in its own sink
                for member in &block.graphs {
                    collect(member, source_used, intermediate_names, named, anon, opaque);
                }
            }
            SchedulingMode::Sequence | SchedulingMode::Selection => {
                // sequential/selection groups converge to their LAST member's sink
                if let Some(member) = block.graphs.last() {
                    collect(member, source_used, intermediate_names, named, anon, opaque);
                }
            }
        },
    }
}

// Deduped by var name:
// a named node that has outgoing edges or is intermediate is NOT a sink,
// so the name is only kept when it sinks.
fn insert_if_sink(
    named: &mut HashSet<String>,
    name: String,
    source_used: &HashSet<String>,
    intermediate_names: &HashSet<String>,
) {
    if !source_used.contains(&name) && !intermediate_names.contains(&name) {
        named.insert(name);
    }
}

/// Bound of a whole graph's rightmost side (sinks).
fn graph_output_bound(g: &Graph) -> Bound {
    // single-node graph: entry is also the exit
    node_output_bound(g.conns.last().unwrap_or(&g.entry))
}

fn node_output_bound(node: &NodeExpr) -> Bound {
    match node {
        NodeExpr::Group(block) => match block.mode {
            SchedulingMode::Fork => Bound::Fork,
            SchedulingMode::Sequence | SchedulingMode::Selection => block
                .graphs
                .last()
                .map(graph_output_bound)
                .unwrap_or(Bound::Join),
        },
        NodeExpr::Expression {
            explicit_bounds, ..
        } => explicit_bounds.map_or(Bound::Ambiguous, |(_, o)| o),
        _ => Bound::Join, // Declaration | Binding
    }
}

fn node_input_bound(node: &NodeExpr) -> Bound {
    match node {
        NodeExpr::Group(block) => match block.mode {
            SchedulingMode::Fork => Bound::Fork,
            SchedulingMode::Sequence | SchedulingMode::Selection => block
                .graphs
                .first()
                .map(|g| node_input_bound(&g.entry))
                .unwrap_or(Bound::Join),
        },
        NodeExpr::Expression {
            explicit_bounds, ..
        } => explicit_bounds.map_or(Bound::Ambiguous, |(i, _)| i),
        _ => Bound::Join, // Declaration | Binding
    }
}
