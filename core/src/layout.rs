use crate::{Graph, Meta, NodeId, bound::Bound};

/// Object-safe, immutable flat graph layout.
///
/// Shared contract for the **merge tier** (a parent flattening heterogenous
/// [Graph<I, O>] into its own adjacency) and the **runtime tier** (the engine
/// reading `meta`/`adj`/`in_degree`).
///
/// Erased on purpose: it deliberately carries no `I`/`O` type-state.
///
/// Lets a `&dyn GraphLayout` be boxed as a *borrowed* layout, so one graph can
/// back several engines/clones without re-cloning its flat data.
///
/// This is the one place the owned-vs-borrowed distinction survives - as a pure
/// lifetime concern, not as a semantic enum.
pub trait GraphLayout: Send + Sync {
    fn meta(&self) -> &[Meta];
    fn adj(&self) -> &[Vec<NodeId>];
    fn in_degree(&self) -> &[usize];
    fn sources(&self) -> Vec<NodeId>;
    fn sinks(&self) -> Vec<NodeId>;
}

impl<I: Bound, O: Bound> GraphLayout for Graph<I, O> {
    fn meta(&self) -> &[Meta] {
        self.meta()
    }

    fn adj(&self) -> &[Vec<NodeId>] {
        self.adj()
    }

    fn in_degree(&self) -> &[usize] {
        self.in_degree()
    }

    fn sources(&self) -> Vec<NodeId> {
        self.sources().collect()
    }

    fn sinks(&self) -> Vec<NodeId> {
        self.sinks().collect()
    }
}

impl<'a> GraphLayout for &'a dyn GraphLayout {
    fn meta(&self) -> &[Meta] {
        (*self).meta()
    }

    fn adj(&self) -> &[Vec<NodeId>] {
        (*self).adj()
    }

    fn in_degree(&self) -> &[usize] {
        (*self).in_degree()
    }

    fn sources(&self) -> Vec<NodeId> {
        (*self).sources()
    }

    fn sinks(&self) -> Vec<NodeId> {
        (*self).sinks()
    }
}

/// The single materialization entry point: graph-like value -> erased layout.
pub trait IntoGraphLayout<'a> {
    fn into_graph_layout(self) -> Box<dyn GraphLayout + 'a>;
}

impl<'a, I: Bound, O: Bound> IntoGraphLayout<'a> for Graph<I, O> {
    fn into_graph_layout(self) -> Box<dyn GraphLayout + 'a> {
        Box::new(self) // 'static auto-coerces to 'a
    }
}

impl<'a, I: Bound, O: Bound> IntoGraphLayout<'a> for &'a Graph<I, O> {
    fn into_graph_layout(self) -> Box<dyn GraphLayout + 'a> {
        Box::new(self as &'a dyn GraphLayout) // borrow, no clone
    }
}

impl<'a> IntoGraphLayout<'a> for &'a dyn GraphLayout {
    fn into_graph_layout(self) -> Box<dyn GraphLayout + 'a> {
        Box::new(self) // forward the existing reference
    }
}
