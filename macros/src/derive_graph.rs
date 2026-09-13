use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Meta, parse_macro_input};

pub fn as_graph_entry_proxy_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let generics = input.generics;

    let orc_attr = input.attrs.iter().find(|attr| attr.path().is_ident("orc"));
    let dsl_tokens = match orc_attr {
        Some(attr) => {
            if let Meta::List(meta_list) = &attr.meta {
                let tokens = &meta_list.tokens;
                quote! { #tokens }
            } else {
                quote! { compile_error!("The #[orc(...)] attribute expects a valid pipeline layout list."); }
            }
        }
        None => {
            quote! { compile_error!("Missing required #[orc(...)] configuration layout attribute."); }
        }
    };

    let destructure_stmt = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields_named) => {
                let field_idents = fields_named.named.iter().map(|f| &f.ident);
                quote! {
                    let Self { #(#field_idents),* } = self;
                }
            }
            Fields::Unnamed(fields_unnamed) => {
                let positional_vars =
                    (0..fields_unnamed.unnamed.len()).map(|i| format_ident!("_{}", i));
                quote! {
                    let Self(#(#positional_vars),*) = self;
                }
            }
            Fields::Unit => quote! {},
        },
        _ => {
            quote! { compile_error!("Graph can only be derived on struct data models."); }
        }
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Synthesize no-op lifetime generic to impl AsGraphEntryProxy
    let final_impl_generics = if generics.lifetimes().next().is_some() {
        quote! { #impl_generics }
    } else {
        quote! { <'a> }
    };

    let expanded = quote! {
        impl #final_impl_generics AsGraphEntryProxy<'a> for #name #ty_generics #where_clause {
            fn as_entry_proxy(self) -> GraphEntry<'a> {
                #destructure_stmt

                let compiled_graph = orc! {
                    #dsl_tokens
                };

                GraphEntry::OwnedGraph(compiled_graph)
            }
        }
    };

    TokenStream::from(expanded)
}
