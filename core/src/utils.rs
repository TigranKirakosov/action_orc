pub fn get_type_name<T>() -> &'static str {
    let full_name = std::any::type_name::<T>();
    full_name
        .rsplit_once("::")
        .map(|(_, name)| name)
        .unwrap_or(full_name)
}
