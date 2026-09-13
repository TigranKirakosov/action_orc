use std::marker::PhantomData;

use crate::{Graph, GraphBuilder, Marker, Meta};

pub enum GraphEntry<'a> {
    Node(Meta),
    BorrowedGraph(&'a Graph),
    OwnedGraph(Graph),
}

pub trait AsGraphEntry<'a> {
    fn as_entry(this: Self) -> GraphEntry<'a>;
}

pub trait AsGraphEntryProxy<'a> {
    fn as_entry_proxy(self) -> GraphEntry<'a>;

    fn into_compiled_graph(self) -> Graph
    where
        Self: Sized,
    {
        match self.as_entry_proxy() {
            GraphEntry::Node(meta) => {
                let mut builder = GraphBuilder::new();
                builder.append(GraphEntry::Node(meta));
                builder.build()
            }
            GraphEntry::OwnedGraph(g) => g,
            GraphEntry::BorrowedGraph(g) => g.clone(),
        }
    }
}

/// Leaf node wrapper to provide marker types with default [AsGraphEntry::as_entry] implementation.
pub struct Tag<T>(pub PhantomData<T>);

impl<'a, T: Marker> AsGraphEntry<'a> for Tag<T> {
    fn as_entry(_this: Self) -> GraphEntry<'a> {
        GraphEntry::Node(T::meta())
    }
}

impl<'a> AsGraphEntry<'a> for &'a Graph {
    fn as_entry(this: Self) -> GraphEntry<'a> {
        GraphEntry::BorrowedGraph(this)
    }
}

impl<'a> AsGraphEntry<'a> for Graph {
    fn as_entry(this: Self) -> GraphEntry<'a> {
        GraphEntry::OwnedGraph(this)
    }
}

impl<'a> AsGraphEntryProxy<'a> for &'a Graph {
    fn as_entry_proxy(self) -> GraphEntry<'a> {
        GraphEntry::BorrowedGraph(self)
    }
}

impl<'a> AsGraphEntryProxy<'a> for Graph {
    fn as_entry_proxy(self) -> GraphEntry<'a> {
        GraphEntry::OwnedGraph(self)
    }
}
