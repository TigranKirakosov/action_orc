use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned};
use std::collections::{HashMap, HashSet};
use syn::{Ident, spanned::Spanned};

use super::{
    ast::{self, Declartaion, GroupBlock, NodeExpr, SchedulingMode},
    format_type,
};

#[derive(Default)]
struct Context {
    compile_graph: action_orc_core::Graph,
    decls: Vec<TokenStream2>,
    links: Vec<TokenStream2>,
    parallel_group_id_counter: usize,
    anon_id_counter: usize,
    node_id_map: HashMap<Ident, usize>,
    anon_map: HashMap<Ident, TypeStr>,
    unbound_types: HashSet<TypeStr>,
    embeddings: HashSet<Ident>,
    errors: Vec<CodegenError>,
}

pub(super) enum CodegenError {
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
    Syn(syn::Error),
}

struct IdFactory;
struct DummyMarker;
struct Source(Ident);
struct Sink(Ident);

type TypeStr = String;

pub(super) fn generate(ast: ast::SyntaxTree) -> TokenStream {
    let mut cx = Context::default();

    for graph in ast.graphs {
        let _ = cx.process_graph(graph);
    }

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
            builder.build()
       }
    };

    out_stream.into()
}

impl Context {
    fn process_graph(&mut self, graph: ast::Graph) -> (Source, Sink) {
        let (source, mut prev_sink) = self.process_node(graph.entry);

        for conn in graph.conns {
            let node_span = match &conn {
                NodeExpr::Declaration(task) => task
                    .var
                    .as_ref()
                    .map(|v| v.span())
                    .unwrap_or_else(|| task.typ.span()),
                NodeExpr::Binding(var) => var.span(),
                NodeExpr::Embedding(emb) => emb.span(),
                NodeExpr::Group(block) => block.span_info.span,
            };

            let (sub_source, sub_sink) = self.process_node(conn);
            self.track_edge(node_span, &prev_sink, &sub_source);

            let prev_sink_ident = &prev_sink.0;
            let sub_source_ident = &sub_source.0;
            self.links.push(quote! {
                builder.connect(&#prev_sink_ident, &#sub_source_ident);
            });

            prev_sink = sub_sink;
        }

        (source, prev_sink)
    }

    fn process_node(&mut self, node: NodeExpr) -> (Source, Sink) {
        match node {
            NodeExpr::Declaration(task) => self.process_declaration(task),
            NodeExpr::Binding(binding) => self.process_binding(binding),
            NodeExpr::Embedding(embedding) => self.process_embedding(embedding),
            NodeExpr::Group(group) => self.process_group(group),
        }
    }

    fn process_group(&mut self, GroupBlock { mode, graphs, .. }: GroupBlock) -> (Source, Sink) {
        match mode {
            SchedulingMode::Parallel => {
                let group_id = self.parallel_group_id_counter;
                self.parallel_group_id_counter += 1;

                let (group_source, group_sink) = IdFactory::parallel_group_bounds(group_id);

                self.links.push(quote! {
                    let mut #group_source = Vec::new();
                    let mut #group_sink = Vec::new();
                });

                for graph in graphs {
                    let (Source(sub_source), Sink(sub_sink)) = self.process_graph(graph);

                    self.links.push(quote! {
                        #group_source.extend(&#sub_source.sources);
                        #group_sink.extend(&#sub_sink.sinks);
                    });
                }

                let (aggregated_source, aggregated_sink) =
                    IdFactory::aggregate_group_bounds(group_id);

                self.links.push(quote! {
                    let #aggregated_source = GraphBounds { sources: #group_source, sinks: vec![] };
                    let #aggregated_sink = GraphBounds { sources: vec![], sinks: #group_sink };
                });

                let src_node_id = self.compile_graph.add_node::<DummyMarker>();
                self.node_id_map
                    .insert(aggregated_source.clone(), src_node_id);

                let snk_node_id = self.compile_graph.add_node::<DummyMarker>();
                self.node_id_map
                    .insert(aggregated_sink.clone(), snk_node_id);

                self.compile_graph.add_edge(src_node_id, snk_node_id);

                (Source(aggregated_source), Sink(aggregated_sink))
            }
            SchedulingMode::Sequence => {
                let mut graphs = graphs.into_iter();

                let (source, mut prev_sink) = self.process_graph(graphs.next().unwrap());
                for graph in graphs {
                    let span = graph.span_info.span;
                    let (sub_source, sub_sink) = self.process_graph(graph);

                    self.track_edge(span, &prev_sink, &sub_source);

                    let prev_sink_ident = &prev_sink.0;
                    let sub_source_ident = &sub_source.0;
                    self.links.push(quote! {
                        builder.connect(&#prev_sink_ident, &#sub_source_ident);
                    });

                    prev_sink = sub_sink;
                }

                (source, prev_sink)
            }
        }
    }

    fn process_declaration(&mut self, decl: Declartaion) -> (Source, Sink) {
        let Declartaion { var, typ } = decl;
        let type_key = format_type(&typ);

        if var.is_none() {
            if self.unbound_types.contains(&type_key) {
                self.errors.push(CodegenError::DuplicateUnboundType {
                    type_key: type_key.clone(),
                    span: typ.span(),
                });
            }
            self.unbound_types.insert(type_key.clone());
        }

        let bounds_ident = var.unwrap_or_else(|| {
            let anon = IdFactory::anonymous_node(self.anon_id_counter);
            self.anon_map.insert(anon.clone(), type_key);
            self.anon_id_counter += 1;
            anon
        });

        let decl = quote! {
            let #bounds_ident = builder.append(
                AsGraphEntry::as_entry(Tag::<#typ>(std::marker::PhantomData))
            );
        };
        self.decls.push(decl);

        let node_id = self.compile_graph.add_node::<DummyMarker>();
        if self.node_id_map.contains_key(&bounds_ident) {
            self.errors.push(CodegenError::VariableCollision {
                var: bounds_ident.to_string(),
                span: bounds_ident.span(),
            });
        }
        self.node_id_map.insert(bounds_ident.clone(), node_id);

        (Source(bounds_ident.clone()), Sink(bounds_ident))
    }

    fn process_binding(&mut self, binding: Ident) -> (Source, Sink) {
        (Source(binding.clone()), Sink(binding))
    }

    /// Processes embedding once to build its bounds.
    ///
    /// On subsequent calls just returns stable [Ident] source and sink.
    fn process_embedding(&mut self, embedding: Ident) -> (Source, Sink) {
        let bounds_ident = IdFactory::embed_ident(&embedding);

        if !self.embeddings.contains(&embedding) {
            self.embeddings.insert(embedding.clone());

            let decl = quote! {
                let #bounds_ident = builder.append(AsGraphEntry::as_entry(#embedding));
            };
            let node_id = self.graph_as_node(&embedding);

            self.decls.push(decl);
            self.anon_map
                .insert(bounds_ident.clone(), embedding.clone().to_string());
            self.node_id_map.insert(bounds_ident.clone(), node_id);
        }

        (Source(bounds_ident.clone()), Sink(bounds_ident))
    }

    fn track_edge(&mut self, span: Span, Sink(from): &Sink, Source(to): &Source) {
        let from_id = *self
            .node_id_map
            .get(from)
            .expect("Missing source loop identifier mapping.");

        let to_id = *self
            .node_id_map
            .get(to)
            .expect("Missing target loop identifier mapping.");

        self.compile_graph.add_edge(from_id, to_id);

        if let Err(action_orc_core::GraphError::CycleDetected) = self.compile_graph.sort_ordered() {
            let from_name = self
                .anon_map
                .get(from)
                .cloned()
                .unwrap_or_else(|| from.to_string());
            let to_name = self
                .anon_map
                .get(to)
                .cloned()
                .unwrap_or_else(|| to.to_string());

            self.errors.push(CodegenError::CircularDependency {
                from: from_name,
                to: to_name,
                span,
            });
        }
    }

    /// Register group or embedded graph as a flat node of compile graph
    /// to resolve circular dependencies at compile-time
    fn graph_as_node(&mut self, ident: &Ident) -> usize {
        if let Some(&id) = self.node_id_map.get(ident) {
            id
        } else {
            let node_id = self.compile_graph.add_node::<DummyMarker>();
            self.node_id_map.insert(ident.clone(), node_id);
            node_id
        }
    }
}

impl From<syn::Error> for CodegenError {
    fn from(err: syn::Error) -> Self {
        CodegenError::Syn(err)
    }
}

impl From<CodegenError> for syn::Error {
    fn from(err: CodegenError) -> syn::Error {
        match err {
            CodegenError::DuplicateUnboundType { type_key, span } => syn::Error::new(
                span,
                format!(
                    "Task Graph Error: Duplicate unbound type declaration: {type_key}.\n\
                    Multiple instances of the same task type must be assigned to unique variables (e.g., foo: {type_key} -> bar: {type_key})."
                ),
            ),
            CodegenError::CircularDependency { from, to, span } => syn::Error::new(
                span,
                format!("Task Graph Error: Circular dependency: {from} -> {to}."),
            ),
            CodegenError::VariableCollision { var, span } => syn::Error::new(
                span,
                format!("Task Graph Error: Variable {var} has been already declared."),
            ),
            CodegenError::Syn(syn_err) => syn_err,
        }
    }
}

impl IdFactory {
    #[inline]
    fn aggregate_group_bounds(group_id: usize) -> (Ident, Ident) {
        (
            format_ident!("_aggr_group_{}_source", group_id),
            format_ident!("_aggr_group_{}_sink", group_id),
        )
    }

    #[inline]
    fn parallel_group_bounds(group_id: usize) -> (Ident, Ident) {
        (
            format_ident!("group_{}_source", group_id),
            format_ident!("group_{}_sink", group_id),
        )
    }

    #[inline]
    fn embed_ident(embedding: &Ident) -> Ident {
        format_ident!("_embed_bounds_{}", embedding)
    }

    #[inline]
    fn anonymous_node(counter: usize) -> Ident {
        format_ident!("anon_{}", counter)
    }
}
