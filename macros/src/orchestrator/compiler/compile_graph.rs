use super::ast::Bound;
use action_orc_core::*;

struct SyntheticMarker;

pub struct CompileGraph {
    pub(crate) graph: Graph<Single, Single>,
    pub(crate) total_lines_processed: usize,
    pub(crate) total_unbound_sinks: usize,
    pub(crate) entry_is_fork: bool,
    pub(crate) exit_is_fork: bool,
    /// A user-provided annotation for the entry embedding (`orc!(@[Fork] ident);`).\
    /// `Some(Ambiguous)` means unannotated embedding at the entry -
    /// bound can't be inferred, embedding is opaque to compiler.
    pub(crate) entry_annotation: Option<Bound>,
    /// Same as for [Self::entry_annotation].
    pub(crate) exit_annotation: Option<Bound>,
}

impl Default for CompileGraph {
    fn default() -> Self {
        Self {
            graph: Graph::new(),
            total_lines_processed: 0,
            total_unbound_sinks: 0,
            entry_is_fork: false,
            exit_is_fork: false,
            entry_annotation: None,
            exit_annotation: None,
        }
    }
}

impl CompileGraph {
    pub fn add_synthetic_node(&mut self) -> usize {
        self.graph.add_node::<SyntheticMarker>()
    }

    pub fn add_edge(&mut self, lhs: usize, rhs: usize) {
        self.graph.add_edge(lhs, rhs);
    }

    pub fn sort_ordered(&mut self) -> Result<Vec<NodeId>, GraphError> {
        self.graph.sort_ordered()
    }

    pub fn in_degree(&mut self) -> &[NodeId] {
        self.graph.in_degree()
    }

    pub fn meta(&mut self) -> &[Meta] {
        self.graph.meta()
    }

    pub fn meta_mut(&mut self) -> &mut [Meta] {
        self.graph.meta_mut()
    }

    pub fn note_new_line_entry(&mut self) {
        self.total_lines_processed += 1;
        self.total_unbound_sinks += 1;
    }

    pub fn note_graph_entry_binding(&mut self) {
        self.total_lines_processed = self.total_lines_processed.saturating_sub(1);
    }

    pub fn note_binding(&mut self) {
        self.total_unbound_sinks = self.total_unbound_sinks.saturating_sub(1);
    }

    pub fn evaluate_graph_bounds(&self) -> (Bound, Bound) {
        let input =
            // user annotated that is
            self.entry_annotation.unwrap_or_else(|| {
            // or inferred
            if self.entry_is_fork || self.total_lines_processed > 1 {
                Bound::Fork
            } else {
                Bound::Single
            }
        });

        // same for exit
        let output = self.exit_annotation.unwrap_or_else(|| {
            if self.exit_is_fork || self.total_unbound_sinks > 1 {
                Bound::Fork
            } else {
                Bound::Single
            }
        });

        (input, output)
    }
}
