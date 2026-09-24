//! Dispatch helpers for compiler to process node declarations
//!
//! A bare uppercase ident inside DSL (e.g. `JustAContainer` or `A`) is
//! parsed as a *declaration*. The compiler then emits
//! ```ignore
//! (&&Tag::<#typ>(PhantomData)).resolve(&mut builder)
//! ```
//!
//! `&&Tag<T>` selects [AsSubgraphBounds] (exact receiver match) when
//! T: [IsSubGraph] (i.e. the type is annotated with `#[graph]` macro), causing the sub-graph
//! to be expanded.
//!
//! For plain marker types [AsSubgraphBounds] fails, and
//! the compiler falls back to [AsNodeBounds] (one auto-deref away), which
//! creates a leaf node.

use crate::*;
use std::marker::PhantomData;

pub struct Tag<T>(pub PhantomData<T>);

/// Leaf-node path — `&Tag<T>` via auto-deref from `&&Tag<T>`.
pub trait AsNodeBounds<'a> {
    fn resolve(self, builder: &mut GraphBuilder) -> GraphBounds;
}

impl<'a, T: Marker + 'static> AsNodeBounds<'a> for &Tag<T> {
    fn resolve(self, builder: &mut GraphBuilder) -> GraphBounds {
        builder.append_node::<T>()
    }
}

/// Sub-graph expansion path: `&&Tag<T>` exact match, preferred when bound holds.
pub trait AsSubgraphBounds<'a> {
    fn resolve(self, builder: &mut GraphBuilder) -> GraphBounds;
}

impl<'a, T: IsSubGraph + Default + IntoGraphLayout<'a>> AsSubgraphBounds<'a> for &&Tag<T> {
    fn resolve(self, builder: &mut GraphBuilder) -> GraphBounds {
        let layout = T::default().into_graph_layout();
        builder.append_graph_layout(&*layout)
    }
}
