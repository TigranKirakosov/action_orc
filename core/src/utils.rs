pub fn get_type_name<T>() -> &'static str {
    let full_name = std::any::type_name::<T>();
    full_name
}

pub mod test_utils {
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

    #[macro_export]
    macro_rules! tags {
        ($($tag:expr),* $(,)?) => {
            vec![$( Into::<&'static str>::into($tag) ),*]
        };
    }

    #[macro_export]
    macro_rules! add_nodes {
        ($graph:expr, $($tag:ident : $type:ty),* $(,)?) => {
            $(
                #[allow(unused)]
                let $tag = $graph.add_node::<$type>();
            )*
        };
    }
}
