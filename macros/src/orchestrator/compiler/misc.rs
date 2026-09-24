use super::{ast::*, *};
use action_orc_core::{DownstreamRole, GraphError};
use syn::Ident;

impl Context {
    pub(super) fn process_node(&mut self, node: &NodeExpr) -> (Source, Sink) {
        match node {
            NodeExpr::Declaration(task) => self.process_declaration(task),
            NodeExpr::Binding(binding) => self.process_binding(binding),
            NodeExpr::Expression {
                expr,
                explicit_bounds,
            } => self.process_embedding(expr, explicit_bounds),
            NodeExpr::Group(group) => self.process_group(group),
        }
    }

    fn process_binding(&mut self, binding: &Ident) -> (Source, Sink) {
        (Source(binding.clone()), Sink(binding.clone()))
    }

    pub(super) fn track_edge(&mut self, span: Span, Sink(from): &Sink, Source(to): &Source) {
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

        if role_ds == DownstreamRole::SelectionMember && node_in_degree > 1 {
            self.errors.push(CodegenError::ForkGroupSelector { span });
        }

        if let Err(GraphError::CycleDetected) = self.compile_graph.sort_ordered() {
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
}
