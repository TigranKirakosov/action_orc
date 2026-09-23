use action_orc_core::DownstreamRole;
#[allow(unused)]
use action_orc_core::{Graph as OrcGraph, GraphError as OrcGraphError};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned};
use std::collections::{HashMap, HashSet};
use syn::Ident;

use crate::orchestrator::compiler::errors::CodegenError;

use super::{
    ast::{self, Bound, SyntaxTree},
    format_type,
};

use compile_graph::CompileGraph;

mod compile_graph;
mod declaration;
mod embedding;
mod errors;
mod graph;
mod group;
mod identity;
mod misc;

#[derive(Default)]
struct Context {
    compile_graph: CompileGraph,
    decls: Vec<TokenStream2>,
    links: Vec<TokenStream2>,
    group_id_counter: usize,
    anon_id_counter: usize,
    node_id_map: HashMap<Ident, usize>,
    anon_map: HashMap<Ident, String>,
    unbound_types: HashSet<String>,
    embedding_expr: HashMap<Ident, TokenStream2>,
    embedding_annotations: HashMap<Ident, (Bound, Bound)>,
    embedding_idents: HashSet<String>,
    errors: Vec<CodegenError>,
}

struct IdFactory;
struct Source(Ident);
struct Sink(Ident);

pub(super) fn generate(ast: SyntaxTree) -> TokenStream {
    let mut cx = Context::default();

    for graph in &ast.graphs {
        cx.compile_graph.note_new_line_entry();
        let _ = cx.process_graph(graph);
    }

    let (input_bound, output_bound) = cx.compile_graph.evaluate_graph_bounds();

    let Context {
        decls,
        links,
        errors,
        ..
    } = cx;

    let compile_errors = errors.into_iter().map(|codegen_err| {
        let syn_err = syn::Error::from(codegen_err);
        let err_msg = syn_err.to_string();
        let err_span = syn_err.span();

        quote_spanned! { err_span =>
            compile_error!{#err_msg};
        }
    });

    let out_stream = quote! {
        {
            let mut builder = GraphBuilder::new();
            #(#decls)*
            #(#links)*
            #(#compile_errors)*
            builder.build::<#input_bound, #output_bound>()
        }
    };

    out_stream.into()
}
