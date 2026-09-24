use super::{ast::*, *};

impl Context {
    pub(super) fn process_graph(&mut self, graph: &Graph) -> (Source, Sink) {
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

    fn process_graph_conn(&mut self, upstream: &Sink, conn: &ast::NodeExpr) {
        // Upstream is considered a Selector, thus it must have a single exit to connect
        // into a selection group
        if let NodeExpr::Group(block) = conn {
            if block.mode == SchedulingMode::Selection {
                let Sink(sink) = upstream;

                self.validate_selector_embedding(sink, block.span());
                self.links.push(quote! {
                    for sink_id in &#sink.sinks {
                        builder.set_upstream_role(sink_id, action_orc::UpstreamRole::Selector);
                    }
                });
            }
        }
    }
}
