use std::any::TypeId;

pub trait Marker: Sized + 'static {
    fn meta() -> Meta {
        Meta::of::<Self>()
    }
}

impl<T: 'static> Marker for T {}

/// Node's role when it is up a stream from nodes located down a stream
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UpstreamRole {
    /// A regular node
    #[default]
    Regular,

    /// A node acting as a choice gate (`pivot -> (A : B)`)\
    /// Its downstream neighbors are mutually exclusive choices
    Selector,
}

/// Node's role when it is down a stream from nodes located up a stream
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DownstreamRole {
    /// A regular node
    #[default]
    Regular,

    /// A member of a selection group (A ? B)
    SelectionBranch,

    /// A member of a parallel group (A | B)
    ParallelBranch,
}

#[derive(Clone)]
pub struct Meta {
    pub(crate) role_us: UpstreamRole,
    pub(crate) role_ds: DownstreamRole,
    pub(crate) type_id: TypeId,
    #[cfg(any(test, feature = "visualizer"))]
    pub(crate) type_name: &'static str,
}

impl Meta {
    pub fn of<T: Marker>() -> Self {
        Self {
            role_us: UpstreamRole::Regular,
            role_ds: DownstreamRole::Regular,
            type_id: TypeId::of::<T>(),
            #[cfg(any(test, feature = "visualizer"))]
            type_name: {
                use crate::get_type_name;
                get_type_name::<T>()
            },
        }
    }

    pub fn role_us(&self) -> UpstreamRole {
        self.role_us
    }

    pub fn role_ds(&self) -> DownstreamRole {
        self.role_ds
    }

    pub fn set_role_us(&mut self, role: UpstreamRole) {
        self.role_us = role;
    }

    pub fn set_role_ds(&mut self, role: DownstreamRole) {
        self.role_ds = role;
    }

    pub fn type_id(&self) -> &TypeId {
        &self.type_id
    }

    #[cfg(any(test, feature = "visualizer"))]
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }
}
