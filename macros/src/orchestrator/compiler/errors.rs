use proc_macro2::Span;

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
    ForkGroupSelector {
        span: Span,
    },
    InvalidSelectionMember {
        span: Span,
    },
    InvalidSelectorBound {
        span: Span,
    },
    Syn(syn::Error),
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
            CodegenError::ForkGroupSelector { span } => syn::Error::new(
                span,
                "Graph Error: Selection group (?) must be preceded strictly by a single selector node.\
                Upstream Fork group is forbidden.",
            ),
            CodegenError::InvalidSelectionMember { span } => syn::Error::new(
                span,
                "Graph Error: A Selection group member can't be a Fork group.",
            ),
            CodegenError::InvalidSelectorBound { span } => syn::Error::new(
                span,
                "Graph Error: A Selector annotated with a Fork exit is forbidden.\n\
                It must have a Join exit.",
            ),
            CodegenError::Syn(syn_err) => syn_err,
        }
    }
}
