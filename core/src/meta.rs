use std::any::TypeId;

pub trait Marker: Sized + 'static {
    fn meta() -> Meta {
        Meta::of::<Self>()
    }
}

impl<T: 'static> Marker for T {}

/// A Topology role of a node inside graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// A regular node
    Regular,

    /// A node acting as a choice gate (`pivot -> (A : B)`)\
    /// Its downstream neighbors are mutually exclusive choices
    Selector,

    /// A member of a selection group (A : B)
    SelectionBranch,

    /// A member of a parallel group (A | B)
    ParallelBranch,
}

#[derive(Clone)]
pub struct Meta {
    pub(crate) role: Role,
    pub(crate) type_id: TypeId,
    #[cfg(any(test, feature = "visualizer"))]
    pub(crate) type_name: &'static str,
}

impl Meta {
    pub(crate) fn of<T: Marker>() -> Self {
        Self {
            role: Role::Regular,
            type_id: TypeId::of::<T>(),
            #[cfg(any(test, feature = "visualizer"))]
            type_name: {
                use crate::get_type_name;
                get_type_name::<T>()
            },
        }
    }

    pub fn role(&self) -> Role {
        self.role
    }

    pub fn set_role(&mut self, role: Role) {
        self.role = role;
    }

    pub fn type_id(&self) -> &TypeId {
        &self.type_id
    }

    #[cfg(any(test, feature = "visualizer"))]
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }
}
