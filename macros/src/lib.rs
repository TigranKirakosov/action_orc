use proc_macro::TokenStream;

mod derive_graph;
mod orchestrator;

#[proc_macro]
pub fn orc(input: TokenStream) -> TokenStream {
    orchestrator::orc(input)
}

#[proc_macro_derive(Graph, attributes(orc))]
pub fn graph(input: TokenStream) -> TokenStream {
    derive_graph::as_graph_entry_proxy_impl(input)
}
