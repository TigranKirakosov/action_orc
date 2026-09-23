use super::*;

impl IdFactory {
    #[inline]
    pub(super) fn aggregate_group_bounds(group_id: usize) -> (Ident, Ident) {
        (
            format_ident!("_aggr_group_{}_source", group_id),
            format_ident!("_aggr_group_{}_sink", group_id),
        )
    }

    #[inline]
    pub(super) fn group_bounds(group_id: usize) -> (Ident, Ident) {
        (
            format_ident!("_group_{}_source", group_id),
            format_ident!("_group_{}_sink", group_id),
        )
    }

    #[inline]
    pub(super) fn embed_ident(embedding: &Ident) -> Ident {
        let lower_name = embedding.to_string().to_lowercase();
        format_ident!("_embed_bounds_{}", lower_name)
    }

    #[inline]
    pub(super) fn expression_ident(id: usize) -> Ident {
        format_ident!("_expr_bounds_{}", id)
    }

    #[inline]
    pub(super) fn anonymous_node(counter: usize) -> Ident {
        format_ident!("_anon_{}", counter)
    }
}
