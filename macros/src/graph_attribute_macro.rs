use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Ident, ItemStruct, parse_macro_input};

pub fn attr_macro(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_struct = parse_macro_input!(item as ItemStruct);
    let name = &input_struct.ident;
    let generics = &input_struct.generics;

    // #[graph(R -> (@a | @b))]
    let dsl_tokens: TokenStream2 = if !attr.is_empty() {
        attr.into()
    } else {
        quote! { compile_error!("Missing required pipeline layout configuration inside #[graph(...)]."); }
    };

    let field_idents: Vec<Ident> = input_struct
        .fields
        .iter()
        .filter_map(|f| f.ident.clone())
        .collect();

    let destructure = if !field_idents.is_empty() {
        quote! {
            #[deny(unused_variables)]
            let Self { #(#field_idents),* } = self;
        }
    } else {
        quote! {}
    };

    // A unit struct (no fields, no generics) can be referenced inside `orc!` as a
    // bare, non-`@` identifier and is then expanded as a sub-graph.
    // This requires the `IsSubGraph + Default + IntoGraphLayout` dispatch to hold, so we derive
    // `Default` and mark it `IsSubGraph`.
    // Structs with fields are only usable via the `@` embedding operator and need neither derive nor the marker.
    let is_unit = field_idents.is_empty() && generics.params.is_empty();

    let final_struct = if is_unit {
        quote! {
            #[derive(Default)]
            #input_struct
        }
    } else {
        quote! { #input_struct }
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Users declare struct fields manually (including lifetime annotations).
    // A unit struct has no `'a` to reference, so the impl gets a fresh `<'a>`.
    // Otherwise we reuse the struct's own generics.
    let has_lt = generics.lifetimes().next().is_some();
    let (final_impl_generics, final_ty_generics) = if has_lt {
        (quote! { #impl_generics }, quote! { #ty_generics })
    } else {
        (quote! { <'a> }, quote! {})
    };

    let is_subgraph_impl = if is_unit {
        quote! {
            impl action_orc::IsSubGraph for #name {}
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #final_struct

        #is_subgraph_impl

        impl #final_impl_generics action_orc::IntoGraphLayout<'a> for #name #final_ty_generics #where_clause {
            fn into_graph_layout(self) -> Box<dyn action_orc::GraphLayout + 'a> {
                #destructure

                let compiled_graph = orc! {
                    #dsl_tokens
                };

                Box::new(compiled_graph)
            }
        }
    };

    TokenStream::from(expanded)
}
