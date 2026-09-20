use crate::Graph;

pub type IdentityGraph = Graph<Regular, Regular>;

pub trait Bound: Send + Sync + 'static {}

pub struct Regular; // 1 entry/exit point
pub struct Parallel; // Multi-entry / multi-exit fork points
pub struct Selection; // A choice intersection node
pub struct Sequence; // For intermediate pipelines

impl Bound for Regular {}
impl Bound for Parallel {}
impl Bound for Selection {}
impl Bound for Sequence {}

pub trait CanSelect {}
pub trait CanParallel {}

impl CanSelect for Regular {}
impl CanSelect for Sequence {}

impl CanParallel for Regular {}
impl CanParallel for Sequence {}

// Parallel explicitly lacks CanSelect!
impl CanParallel for Parallel {}

pub trait StaticTopologyVerify<I: Bound, O: Bound> {
    fn verify_selectable(self) -> Self
    where
        O: CanSelect,
        Self: Sized,
    {
        self
    }

    fn verify_parallel(self) -> Self
    where
        O: CanParallel,
        Self: Sized,
    {
        self
    }
}

impl<I: Bound, O: Bound> StaticTopologyVerify<I, O> for Graph<I, O> {}
