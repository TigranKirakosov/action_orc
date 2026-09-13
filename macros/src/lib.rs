use proc_macro::TokenStream;

mod graph_attribute_macro;
mod orchestrator;

#[proc_macro]
pub fn orc(input: TokenStream) -> TokenStream {
    orchestrator::orc(input)
}

#[proc_macro_attribute]
pub fn graph(attr: TokenStream, item: TokenStream) -> TokenStream {
    graph_attribute_macro::attr_macro(attr, item)
}
