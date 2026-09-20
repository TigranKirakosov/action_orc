use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Token;
use syn::punctuated::Punctuated;
use syn::{Fields, FieldsNamed, Ident, ItemStruct, Meta, Visibility, parse_macro_input};

pub fn attr_macro(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input_struct = parse_macro_input!(item as ItemStruct);
    let name = &input_struct.ident;
    let generics = &input_struct.generics;

    // #[graph(R -> (@a | @b))]
    let dsl_tokens: TokenStream2 = if !attr.is_empty() {
        attr.into()
    } else {
        quote! { compile_error!("Missing required pipeline layout configuration inside #[graph(...)]."); }
    };

    let mut param_idents: Vec<Ident> = Vec::new();

    input_struct.attrs.retain(|attr| {
        if attr.path().is_ident("params") {
            if let Meta::List(meta_list) = &attr.meta {
                let nested_res =
                    meta_list.parse_args_with(Punctuated::<Ident, Token![,]>::parse_terminated);
                if let Ok(idents) = nested_res {
                    param_idents.extend(idents.into_iter());
                }
            }
            false
        } else {
            true
        }
    });

    let has_params = !param_idents.is_empty();
    if has_params {
        // Schema: pub a: &'a dyn AnyGraph
        let generated_fields = param_idents.iter().map(|id| syn::Field {
            attrs: Vec::new(),
            vis: Visibility::Public(syn::token::Pub::default()),
            mutability: syn::FieldMutability::None,
            ident: Some(id.clone()),
            colon_token: Some(syn::token::Colon::default()),
            ty: syn::parse_quote! { &'a dyn AnyGraph },
        });

        let mut fields_named = FieldsNamed {
            brace_token: syn::token::Brace::default(),
            named: Punctuated::new(),
        };
        for field in generated_fields {
            fields_named.named.push(field);
        }
        input_struct.fields = Fields::Named(fields_named);
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let (final_impl_generics, final_ty_generics, final_struct) =
        // Presence of #[params(x, y, z)] means we must inject <'a> lifetime to
        // support 'pub member: &'a dyn AnyGraph' contract
        if has_params || generics.lifetimes().next().is_some() {
            let current_struct = if generics.lifetimes().next().is_none() {
                let mut modified_struct = input_struct.clone();
                modified_struct
                    .generics
                    .params
                    .push(syn::parse_quote! { 'a });
                quote! { #modified_struct }
            } else {
                quote! { #input_struct }
            };

            let impl_g = if generics.lifetimes().next().is_some() {
                quote! { #impl_generics }
            } else {
                quote! { <'a> }
            };
            let ty_g = if generics.lifetimes().next().is_some() {
                quote! { #ty_generics }
            } else {
                quote! { <'a> }
            };

            (impl_g, ty_g, current_struct)
        }
        // Unit struct derives Default so `orc!` macro can make instance to access transitive traits
        else {
            (
                quote! { <'a> },
                quote! {},
                quote! {
                    #[derive(Default)]
                    #input_struct
                },
            )
        };

    let destructure_stmt = if has_params {
        quote! {
            // Presence of unused variables hints user forgot to add '@' prefix before compund subgraph
            #[deny(unused_variables)]
            let Self { #(#param_idents),* } = self;
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #final_struct

        impl #final_impl_generics IsSubGraph for #name #final_ty_generics #where_clause {}

        impl #final_impl_generics AsGraphEntryProxy<'a> for #name #final_ty_generics #where_clause {
            fn as_entry_proxy(self) -> GraphEntry<'a> {
                #destructure_stmt

                let compiled_graph = orc! {
                    #dsl_tokens
                };

                GraphEntry::OwnedGraph(Box::new(compiled_graph))
            }
        }
    };

    TokenStream::from(expanded)
}
