#[macro_export]
macro_rules! declare_tags {
    ($($type:ident),* $(,)?) => {
        $(

            #[derive(Debug)]
            struct $type;
            impl Into<&'static str> for $type {
                fn into(self) -> &'static str {
                    get_type_name::<$type>()
                }
            }
            // Struct == &str
            impl PartialEq<&'static str> for $type {
                fn eq(&self, other: &&'static str) -> bool {
                    &get_type_name::<$type>() == other
                }
            }

            // &str == Struct
            impl PartialEq<$type> for &'static str {
                fn eq(&self, _other: &$type) -> bool {
                    *self == get_type_name::<$type>()
                }
            }
        )*
    };
}
pub use declare_tags;

#[macro_export]
macro_rules! tags {
    ($($tag:expr),* $(,)?) => {
        vec![$( Into::<&'static str>::into($tag) ),*]
    };
}
pub use tags;

#[macro_export]
macro_rules! add_nodes {
    ($graph:expr, $($tag:ident : $type:ty),* $(,)?) => {
        $(
            #[allow(unused)]
            let $tag = $graph.add_node::<$type>();
        )*
    };
}
pub use add_nodes;
