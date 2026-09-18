use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::Type;

mod ast;
mod codegen;
mod parser;

pub(crate) fn orc(input: TokenStream) -> TokenStream {
    match parser::parse(input.into()) {
        Ok(ast) => codegen::generate(ast),
        Err(err) => {
            let error_msg = format!("Graph parsing error: {:?}", err);
            quote! { compile_error!(#error_msg); }.into()
        }
    }
}

pub(crate) fn format_type(typ: &Type) -> String {
    typ.clone().into_token_stream().to_string().replace(" ", "")
}
