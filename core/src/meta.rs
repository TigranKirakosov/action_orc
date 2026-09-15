use std::any::TypeId;

pub trait Marker: Sized + 'static {
    fn meta() -> Meta {
        Meta::of::<Self>()
    }
}

impl<T: 'static> Marker for T {}

#[derive(Clone)]
pub struct Meta {
    pub(crate) type_id: TypeId,
    #[cfg(any(test, feature = "visualizer"))]
    pub(crate) type_name: &'static str,
}

impl Meta {
    pub(crate) fn of<T: Marker>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            #[cfg(any(test, feature = "visualizer"))]
            type_name: {
                use crate::get_type_name;
                get_type_name::<T>()
            },
        }
    }

    pub fn type_id(&self) -> &TypeId {
        &self.type_id
    }

    #[cfg(any(test, feature = "visualizer"))]
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }
}
