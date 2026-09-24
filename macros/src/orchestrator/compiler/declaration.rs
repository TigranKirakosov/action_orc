use super::{ast::*, *};
#[allow(unused_imports)]
use action_orc_core::*;
use syn::spanned::Spanned;

impl Context {
    /// Resolve a non-`@` declaration into either
    /// - a leaf node
    /// - an embedded sub-graph, depending on whether
    /// `#typ` implements [IsSubGraph] (i.e. is a [#[graph]]-decorated unit struct).
    ///
    /// `(&&Tag::<#typ>(...)).resolve(&mut builder)`:
    /// - `&&Tag<T>` selects [AsSubgraphBounds] (exact receiver match) when
    ///   T: [IsSubGraph] + [Default] + [IntoGraphLayout], expanding the sub-graph;
    /// - otherwise it falls back to [AsNodeBounds] (one auto-deref), creating
    ///   a leaf node via [GraphBuilder::append_node]::<T>().
    pub(super) fn process_declaration(&mut self, decl: &Declartaion) -> (Source, Sink) {
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

        let bounds_ident = var.clone().unwrap_or_else(|| {
            let anon = IdFactory::anonymous_node(self.anon_id_counter);
            self.anon_map.insert(anon.clone(), type_key);
            self.anon_id_counter += 1;
            anon
        });

        let decl = quote! {
            let #bounds_ident = (&&action_orc::Tag::<#typ>(std::marker::PhantomData))
                .resolve(&mut builder);
        };
        self.decls.push(decl);

        let node_id = self.compile_graph.add_synthetic_node();
        if self.node_id_map.contains_key(&bounds_ident) {
            self.errors.push(CodegenError::VariableCollision {
                var: bounds_ident.to_string(),
                span: bounds_ident.span(),
            });
        }
        self.node_id_map.insert(bounds_ident.clone(), node_id);

        (Source(bounds_ident.clone()), Sink(bounds_ident))
    }
}
