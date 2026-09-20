use action_orc_core::IdentityGraph;
#[allow(unused)]
use action_orc_core::{Graph as OrcGraph, GraphEntry, GraphError as OrcGraphError};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use std::collections::{HashMap, HashSet};
use syn::Ident;

use super::{
    ast::{self},
    format_type,
};

mod codegen;

type TypeStr = String;

#[derive(Default)]
struct Context {
    compile_graph: IdentityGraph,
    decls: Vec<TokenStream2>,
    links: Vec<TokenStream2>,
    parallel_group_id_counter: usize,
    anon_id_counter: usize,
    node_id_map: HashMap<Ident, usize>,
    anon_map: HashMap<Ident, TypeStr>,
    unbound_types: HashSet<TypeStr>,
    errors: Vec<CodegenError>,
}

struct Source(Ident);
struct Sink(Ident);

enum CodegenError {
    DuplicateUnboundType {
        type_key: String,
        span: Span,
    },
    CircularDependency {
        from: String,
        to: String,
        span: Span,
    },
    VariableCollision {
        var: String,
        span: Span,
    },
    MultiPivotSelector {
        span: Span,
    },
    InvalidSelectionBranch {
        span: Span,
    },
    Syn(syn::Error),
}

pub(super) fn generate(ast: ast::SyntaxTree) -> TokenStream {
    let mut cx = Context::default();

    for graph in ast.graphs {
        let _ = cx.process_graph(graph);
    }

    let (input_bound, output_bound) = cx.evaluate_bounds();

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
