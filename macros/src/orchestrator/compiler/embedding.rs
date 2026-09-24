use super::{ast::*, *};

impl Context {
    /// Checks two types of embeddings:
    /// - **A**: Join variable: `... -> @some_sub_graph -> ...`
    /// - **B**: Complex expression: `@Race{ a: x, b: y }`
    ///
    /// `explicit_bounds` comes from the `@[Fork; Join]` annotation syntax.
    ///
    /// Embedding is considered an anonymous entry in a codegen context.
    pub(super) fn process_embedding(
        &mut self,
        expr: &syn::Expr,
        explicit_bounds: &Option<(Bound, Bound)>,
    ) -> (Source, Sink) {
        // Case A:
        if let syn::Expr::Path(expr_path) = expr
            && expr_path.path.leading_colon.is_none()
            && expr_path.path.segments.len() == 1
        {
            let embedding = expr_path.path.segments[0].ident.clone();
            let bounds_ident = IdFactory::embed_ident(&embedding);
            let emb_key = embedding.to_string();

            if !self.embedding_idents.contains(&emb_key) {
                self.embedding_idents.insert(emb_key);

                let decl = quote! {
                    let #bounds_ident = builder.append(&#expr);
                };
                self.embedding_expr
                    .insert(bounds_ident.clone(), quote!(#embedding));

                // When explicit bounds are present (e.g. `@[Join] amb`), the annotation
                // overrides the expression's native bound, so rule-site trait assertions
                // are skipped and the declared bounds are validated directly by codegen
                if let Some(bounds) = explicit_bounds {
                    self.embedding_annotations
                        .insert(bounds_ident.clone(), *bounds);
                }

                self.decls.push(decl);

                let node_id = self.compile_graph.add_synthetic_node();
                self.anon_map
                    .insert(bounds_ident.clone(), embedding.to_string());
                self.node_id_map.insert(bounds_ident.clone(), node_id);
            }

            return (Source(bounds_ident.clone()), Sink(bounds_ident));
        }

        // Case B:
        let bounds_ident = IdFactory::expression_ident(self.anon_id_counter);
        self.anon_id_counter += 1;

        // Use append_graph_layout because some types (e.g. Race) are not Graph<I, O>.
        // The embedding struct's own into_graph_layout body must validate I/O bounds.
        let decl = quote! {
            let _expr_layout = (#expr).into_graph_layout();
            let #bounds_ident = builder.append_graph_layout(&*_expr_layout);
        };
        self.embedding_expr
            .insert(bounds_ident.clone(), quote!(#expr));

        // Record embedding annotations for rule-site validation
        if let Some(bounds) = explicit_bounds {
            self.embedding_annotations
                .insert(bounds_ident.clone(), *bounds);
        }

        self.decls.push(decl);

        let node_id = self.compile_graph.add_synthetic_node();
        self.anon_map
            .insert(bounds_ident.clone(), quote!(#expr).to_string());
        self.node_id_map.insert(bounds_ident.clone(), node_id);

        (Source(bounds_ident.clone()), Sink(bounds_ident))
    }

    pub(super) fn validate_selector_embedding(&mut self, selector: &Ident, span: Span) {
        // The only valid annotation for a selector is `Signle`
        if let Some((_, Bound::Fork | Bound::Ambiguous)) = self.embedding_annotations.get(selector)
        {
            self.errors
                .push(CodegenError::InvalidSelectorBound { span });
        }
        // Assert embedding is valid selector by type system
        else if let Some(expr) = self.embedding_expr.get(selector) {
            self.links.push(quote! {
                {
                    fn _assert_single_exit<I: Bound, O: Exit>(_: &Graph<I, O>) {}
                    _assert_single_exit(&#expr);
                }
            });
        }
    }

    pub(super) fn validate_selection_member_embedding(&mut self, member: &Ident, span: Span) {
        // The only valid annotation for a selection group member is `Signle`
        if let Some((Bound::Fork | Bound::Ambiguous, _)) = self.embedding_annotations.get(member) {
            self.errors
                .push(CodegenError::InvalidSelectionMember { span });
        }
        // Assert embedding is valid selection member by type system
        if let Some(expr) = self.embedding_expr.get(member) {
            if !self.embedding_annotations.contains_key(member) {
                self.links.push(quote! {
                    {
                        fn _assert_single_entry<I: Entry, O: Bound>(_: &Graph<I, O>) {}
                        _assert_single_entry(&#expr);
                    }
                });
            }
        }
    }
}
