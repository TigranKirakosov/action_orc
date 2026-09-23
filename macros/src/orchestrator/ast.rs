use proc_macro2::Span;
use quote::{ToTokens, quote};
use syn::spanned::Spanned;
use syn::{Expr, Ident, Type};

use super::format_type;
use super::parser::SpanInfo;

#[derive(Debug)]
pub(super) struct SyntaxTree {
    pub(super) graphs: Vec<Graph>,
}

/// `A -> (B | [in]) -> C`
#[derive(Debug, PartialEq)]
pub(super) struct Graph {
    // A
    pub(super) entry: NodeExpr,
    // B, [in], C
    pub(super) conns: Vec<NodeExpr>,
    pub(super) span_info: SpanInfo,
}

#[derive(Debug, PartialEq)]
pub(super) enum NodeExpr {
    /// A local variable declaration
    Declaration(Declartaion),

    /// A bound node identifier reference, e.g. [in]
    Binding(Ident),

    /// Any expression that results in a graph-like value (embedding via `@`).
    /// Optionally carries explicit I/O bound annotations like `@[Fork; Single] expr`.
    Expression {
        expr: Expr,
        explicit_bounds: Option<(Bound, Bound)>,
    },

    /// (A | B | C) or (A, B, C) or (A -> B -> C) or (A ? B ? C)
    Group(GroupBlock),
}

/// Local declaration signature
/// Either full match `var: typ` (`a: TaskA`) or type only (`TaskA`)
#[derive(PartialEq)]
pub(super) struct Declartaion {
    pub(super) var: Option<Ident>,
    pub(super) typ: Type,
}

#[derive(Debug, PartialEq)]
pub(super) struct GroupBlock {
    pub(super) mode: SchedulingMode,
    pub(super) graphs: Vec<Graph>,
    pub(super) span_info: SpanInfo,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum SchedulingMode {
    /// (A, B) or (A -> B)
    Sequence,
    /// (A | B | C)
    Fork,
    /// (A : B : C)
    Selection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bound {
    Single,
    Fork,
    Ambiguous,
}

impl ToTokens for Bound {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ts = match self {
            Bound::Single => quote! { Single },
            Bound::Fork => quote! { Fork },
            Bound::Ambiguous => quote! { Ambiguous },
        };
        tokens.extend(ts);
    }
}

impl std::fmt::Debug for Declartaion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_string = format_type(&self.typ);

        f.debug_struct("Declaration")
            .field("var", &self.var.clone().map(|var| var.to_string()))
            .field("typ", &type_string)
            .finish()
    }
}

impl Graph {
    pub(crate) fn span(&self) -> Span {
        self.span_info.span
    }
}

impl GroupBlock {
    pub(crate) fn span(&self) -> Span {
        self.span_info.span
    }
}

impl NodeExpr {
    pub(crate) fn span(&self) -> Span {
        match self {
            NodeExpr::Declaration(task) => task
                .var
                .as_ref()
                .map(|v| v.span())
                .unwrap_or_else(|| task.typ.span()),
            NodeExpr::Binding(var) => var.span(),
            NodeExpr::Expression { expr, .. } => expr.span(),
            NodeExpr::Group(block) => block.span(),
        }
    }
}
