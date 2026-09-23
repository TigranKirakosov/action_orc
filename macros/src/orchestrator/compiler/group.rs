use action_orc_core::DownstreamRole;

use super::{ast::*, *};

impl Context {
    pub(super) fn process_group(
        &mut self,
        GroupBlock { mode, graphs, .. }: &GroupBlock,
    ) -> (Source, Sink) {
        match mode {
            SchedulingMode::Fork | SchedulingMode::Selection => {
                let group_id = self.group_id_counter;
                self.group_id_counter += 1;

                let (group_source, group_sink) = IdFactory::group_bounds(group_id);

                self.links.push(quote! {
                    let mut #group_source = Vec::new();
                    let mut #group_sink = Vec::new();
                });

                for graph in graphs {
                    let (Source(sub_source), Sink(sub_sink)) = self.process_graph(&graph);

                    self.assert_group_rules(graph, mode, &sub_source, &sub_sink);
                    self.process_group_roles(mode, &sub_source, &sub_sink);

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

                let source_node_id = self.compile_graph.add_synthetic_node();
                self.node_id_map
                    .insert(aggregated_source.clone(), source_node_id);

                let sink_node_id = self.compile_graph.add_synthetic_node();
                self.node_id_map
                    .insert(aggregated_sink.clone(), sink_node_id);

                self.compile_graph.add_edge(source_node_id, sink_node_id);

                (Source(aggregated_source), Sink(aggregated_sink))
            }
            SchedulingMode::Sequence => {
                let mut graphs = graphs.into_iter();

                let (source, mut prev_sink) = self.process_graph(&graphs.next().unwrap());
                for graph in graphs {
                    let span = graph.span();
                    let (sub_source, sub_sink) = self.process_graph(&graph);

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

    fn assert_group_rules(
        &mut self,
        graph: &Graph,
        mode: &SchedulingMode,
        sub_source: &Ident,
        _sub_sink: &Ident,
    ) {
        match mode {
            SchedulingMode::Selection => {
                // Check upstreama of selection group
                let upstream = &graph.entry;
                if let NodeExpr::Group(gruop) = upstream {
                    // it must be anything but `Fork` by the selection group rules
                    if gruop.mode == SchedulingMode::Fork {
                        let span = gruop.span();
                        self.errors
                            .push(CodegenError::InvalidSelectionMember { span });
                    }
                }

                self.validate_selection_member_embedding(sub_source, graph.span());
            }
            _ => {}
        }
    }

    fn process_group_roles(
        &mut self,
        mode: &SchedulingMode,
        sub_source: &Ident,
        _sub_sink: &Ident,
    ) {
        match mode {
            SchedulingMode::Fork => {
                self.links.push(quote! {
                    for source_id in &#sub_source.sources {
                        builder.set_downstream_role(source_id, DownstreamRole::ForkMember);
                    }
                });

                let node_id = self.node_id_map[&sub_source];
                self.compile_graph.meta_mut()[node_id].set_role_ds(DownstreamRole::ForkMember);
            }
            SchedulingMode::Selection => {
                self.links.push(quote! {
                    for source_id in &#sub_source.sources {
                        builder.set_downstream_role(source_id, DownstreamRole::SelectionMember);
                    }
                });

                let node_id = self.node_id_map[&sub_source];
                self.compile_graph.meta_mut()[node_id].set_role_ds(DownstreamRole::SelectionMember);
            }
            _ => unreachable!(),
        }
    }
}
