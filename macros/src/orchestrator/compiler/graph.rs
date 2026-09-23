use super::{ast::*, *};

impl Context {
    pub(super) fn process_graph(&mut self, graph: &Graph) -> (Source, Sink) {
        self.process_graph_bounds(graph);
        let (source, mut prev_sink) = self.process_node(&graph.entry);

        for conn in &graph.conns {
            self.process_graph_conn(&prev_sink, &conn);
            let (sub_source, sub_sink) = self.process_node(conn);
            self.track_edge(conn.span(), &prev_sink, &sub_source);

            let prev_sink_ident = &prev_sink.0;
            let sub_source_ident = &sub_source.0;
            self.links.push(quote! {
                builder.connect(&#prev_sink_ident, &#sub_source_ident);
            });

            prev_sink = sub_sink;
        }

        (source, prev_sink)
    }

    fn process_graph_bounds(&mut self, graph: &ast::Graph) {
        // Detect edge embeddings and stamp explicit bounds (Ambiguous or user-annotated)
        self.note_entry_bound(&graph.entry);

        match graph.conns.last() {
            Some(last) => self.note_exit_bound(last),
            // A single-node graph (`orc!(@[*] emb);`) has no connections, so its entry
            // is also its exit hence the exit bound must be stamped from the same node
            None => self.note_exit_bound(&graph.entry),
        }

        match &graph.entry {
            NodeExpr::Group(block) => {
                if self.compile_graph.total_lines_processed == 1
                    && block.mode == SchedulingMode::Fork
                {
                    self.compile_graph.entry_is_fork = true;
                }
            }
            NodeExpr::Binding(_) => {
                self.compile_graph.note_graph_entry_binding();
            }
            _ => {}
        }
    }

    fn process_graph_conn(&mut self, upstream: &Sink, conn: &ast::NodeExpr) {
        if let NodeExpr::Group(block) = conn {
            self.compile_graph.exit_is_fork = block.mode == SchedulingMode::Fork;
        } else {
            self.compile_graph.exit_is_fork = false;
        }

        // Upstream is considered a Selector, thus it must have a single exit to connect
        // into a selection group
        if let NodeExpr::Group(block) = conn {
            if block.mode == SchedulingMode::Selection {
                let Sink(sink) = upstream;

                self.validate_selector_embedding(sink, block.span());
                self.links.push(quote! {
                    for sink_id in &#sink.sinks {
                        builder.set_upstream_role(sink_id, UpstreamRole::Selector);
                    }
                });
            }
        }
    }

    fn note_entry_bound(&mut self, entry: &NodeExpr) {
        if let NodeExpr::Expression {
            explicit_bounds, ..
        } = entry
        {
            self.compile_graph.entry_annotation = Some(match explicit_bounds {
                Some((entry_bound, _)) => *entry_bound,
                None => Bound::Ambiguous,
            });
        }
    }

    fn note_exit_bound(&mut self, last: &NodeExpr) {
        if let NodeExpr::Expression {
            explicit_bounds, ..
        } = last
        {
            self.compile_graph.exit_annotation = Some(match explicit_bounds {
                Some((_, exit_bound)) => *exit_bound,
                None => Bound::Ambiguous,
            });
        }
    }
}
