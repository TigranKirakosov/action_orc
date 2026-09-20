use action_orc_core::{DownstreamRole, UpstreamRole};
#[allow(unused)]
use action_orc_core::{Graph as OrcGraph, GraphEntry, GraphError as OrcGraphError};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{Ident, spanned::Spanned};

use super::{
    CodegenError, Context, Sink, Source,
    ast::{self, Declartaion, GroupBlock, NodeExpr, SchedulingMode},
    format_type,
};

struct IdFactory;
struct DummyMarker;

impl Context {
    pub(super) fn process_graph(&mut self, graph: ast::Graph) -> (Source, Sink) {
        let mut entry_mode = None;

        if let NodeExpr::Group(ref block) = graph.entry {
            entry_mode = Some(block.mode);
        }

        let (source, mut prev_sink) = self.process_node(graph.entry, entry_mode);

        for conn in graph.conns {
            let mut conn_mode = None;
            let node_span = match &conn {
                NodeExpr::Declaration(task) => task
                    .var
                    .as_ref()
                    .map(|v| v.span())
                    .unwrap_or_else(|| task.typ.span()),
                NodeExpr::Binding(var) => var.span(),
                NodeExpr::Expression(expr) => expr.span(),
                NodeExpr::Group(block) => {
                    conn_mode = Some(block.mode);
                    block.span_info.span
                }
            };

            if let NodeExpr::Group(ref block) = conn {
                if block.mode == SchedulingMode::Selection {
                    let Sink(sink) = &prev_sink;

                    self.links.push(quote! {
                        for sink_id in &#sink.sinks {
                            builder.set_upstream_role(sink_id, UpstreamRole::Selector);
                        }
                    });
                }
            }

            let (sub_source, sub_sink) = self.process_node(conn, conn_mode);
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

    fn process_node(&mut self, node: NodeExpr, mode: Option<SchedulingMode>) -> (Source, Sink) {
        match node {
            NodeExpr::Declaration(task) => self.process_declaration(task),
            NodeExpr::Binding(binding) => self.process_binding(binding),
            NodeExpr::Expression(expression) => self.process_expression(expression, mode),
            NodeExpr::Group(group) => self.process_group(group),
        }
    }

    fn process_group(&mut self, GroupBlock { mode, graphs, .. }: GroupBlock) -> (Source, Sink) {
        match mode {
            SchedulingMode::Parallel | SchedulingMode::Selection => {
                let group_id = self.parallel_group_id_counter;
                self.parallel_group_id_counter += 1;

                let (group_source, group_sink) = IdFactory::parallel_group_bounds(group_id);

                self.links.push(quote! {
                    let mut #group_source = Vec::new();
                    let mut #group_sink = Vec::new();
                });

                for graph in graphs {
                    if mode == SchedulingMode::Selection {
                        if let NodeExpr::Group(ref sub_group) = graph.entry {
                            if sub_group.mode == SchedulingMode::Parallel {
                                let span = sub_group.span_info.span;
                                self.errors
                                    .push(CodegenError::InvalidSelectionBranch { span });
                            }
                        }
                    }

                    let (Source(sub_source), Sink(sub_sink)) = self.process_graph(graph);

                    match mode {
                        SchedulingMode::Parallel => {
                            self.links.push(quote! {
                                for source_id in &#sub_source.sources {
                                    builder.set_downstream_role(source_id, DownstreamRole::ParallelBranch);
                                }
                            });

                            let node_id = self.node_id_map[&sub_source];
                            self.compile_graph.meta_mut()[node_id]
                                .set_role_ds(DownstreamRole::ParallelBranch);
                        }
                        SchedulingMode::Selection => {
                            self.links.push(quote! {
                                for source_id in &#sub_source.sources {
                                    builder.set_downstream_role(source_id, DownstreamRole::SelectionBranch);
                                }
                            });

                            let node_id = self.node_id_map[&sub_source];
                            self.compile_graph.meta_mut()[node_id]
                                .set_role_ds(DownstreamRole::SelectionBranch);
                        }
                        _ => unreachable!(),
                    }

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

    /// Resolves declaration into possible variants:
    /// - [GraphEntry::Node]
    /// - [GraphEntry::OwnedGraph]
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
            let #bounds_ident = builder.append({
                (&&Tag::<#typ>(std::marker::PhantomData)).resolve()
            });
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

    /// Checks two types of expressions:
    /// - **A**: Single variable: `... -> some_sub_graph -> ...`
    /// - **B**: Complex: `Race(x, y)`
    fn process_expression(
        &mut self,
        expr: syn::Expr,
        mode: Option<SchedulingMode>,
    ) -> (Source, Sink) {
        let type_check_suffix = match mode {
            Some(SchedulingMode::Selection) => quote! { .verify_selectable() },
            Some(SchedulingMode::Parallel) => quote! { .verify_parallel() },
            _ => quote! {},
        };

        // Case A
        if let syn::Expr::Path(ref expr_path) = expr
            && expr_path.path.leading_colon.is_none()
            && expr_path.path.segments.len() == 1
        {
            let embedding = expr_path.path.segments[0].ident.clone();
            let bounds_ident = IdFactory::embed_ident(&embedding);

            if !self.unbound_types.contains(&embedding.to_string()) {
                self.unbound_types.insert(embedding.to_string());

                let decl = quote! {
                    let #bounds_ident = builder.append(
                        AsGraphEntryProxy::as_entry_proxy((#expr)#type_check_suffix)
                    );
                };
                self.decls.push(decl);

                let node_id = self.compile_graph.add_node::<DummyMarker>();
                self.anon_map
                    .insert(bounds_ident.clone(), embedding.to_string());
                self.node_id_map.insert(bounds_ident.clone(), node_id);
            }

            return (Source(bounds_ident.clone()), Sink(bounds_ident));
        }

        // Case B
        let bounds_ident = IdFactory::expression_ident(self.anon_id_counter);
        self.anon_id_counter += 1;

        let decl = quote! {
            let #bounds_ident = builder.append(
                AsGraphEntryProxy::as_entry_proxy((#expr)#type_check_suffix)
            );
        };
        self.decls.push(decl);

        let node_id = self.compile_graph.add_node::<DummyMarker>();
        self.anon_map
            .insert(bounds_ident.clone(), quote!(#expr).to_string());
        self.node_id_map.insert(bounds_ident.clone(), node_id);

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

        let (role_ds, node_in_degree) = (
            self.compile_graph.meta()[to_id].role_ds(),
            self.compile_graph.in_degree()[to_id],
        );

        if role_ds == DownstreamRole::SelectionBranch && node_in_degree > 1 {
            self.errors.push(CodegenError::MultiPivotSelector { span });
        }

        if let Err(OrcGraphError::CycleDetected) = self.compile_graph.sort_ordered() {
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

    pub(super) fn evaluate_bounds(&self) -> (TokenStream2, TokenStream2) {
        let source_count = self.compile_graph.sources().count();
        let sink_count = self.compile_graph.sinks().count();

        let has_selection_enter = self
            .compile_graph
            .sinks()
            .any(|id| self.compile_graph.meta()[id].role_us() == UpstreamRole::Selector);

        let has_selection_exit = self
            .compile_graph
            .sinks()
            .any(|id| self.compile_graph.meta()[id].role_ds() == DownstreamRole::SelectionBranch);

        let input_bound = if has_selection_enter {
            quote! { Selection }
        } else if source_count > 1 {
            quote! { Parallel }
        } else {
            quote! { Regular }
        };

        let output_bound = if has_selection_exit {
            quote! { Selection }
        } else if sink_count > 1 {
            quote! { Parallel }
        } else {
            quote! { Regular }
        };

        (input_bound, output_bound)
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
                    "Graph Error: Duplicate unbound type declaration: {type_key}.\n\
                    Multiple instances of the same task type must be assigned to unique variables (e.g., foo: {type_key} -> bar: {type_key})."
                ),
            ),
            CodegenError::CircularDependency { from, to, span } => syn::Error::new(
                span,
                format!("Graph Error: Circular dependency: {from} -> {to}."),
            ),
            CodegenError::VariableCollision { var, span } => syn::Error::new(
                span,
                format!("Graph Error: Variable {var} has been already declared."),
            ),
            CodegenError::MultiPivotSelector { span } => syn::Error::new(
                span,
                "Graph Error: Selection group (?) must be preceded strictly by a single selector node.\
                Multiple parallel parents are forbidden.",
            ),
            CodegenError::InvalidSelectionBranch { span } => syn::Error::new(
                span,
                "Graph Error: A Selection Group member cannot be a Parallel Group.",
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
        let lower_name = embedding.to_string().to_lowercase();
        format_ident!("_embed_bounds_{}", lower_name)
    }

    #[inline]
    fn expression_ident(id: usize) -> Ident {
        format_ident!("_expr_bounds_{}", id)
    }

    #[inline]
    fn anonymous_node(counter: usize) -> Ident {
        format_ident!("anon_{}", counter)
    }
}
