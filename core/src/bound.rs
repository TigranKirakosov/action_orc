use crate::Graph;

pub type PlainGraph = Graph<Single, Single>;

pub trait Bound: Send + Sync + 'static {}
pub trait Entry: Bound {}
pub trait Exit: Bound {}

pub trait GroupEntry: Bound {}
pub trait GroupExit: Bound {}

pub struct Single;
pub struct Fork;

impl Bound for Single {}
impl Entry for Single {}
impl Exit for Single {}

impl Bound for Fork {}
impl GroupEntry for Fork {}
impl GroupExit for Fork {}

/// Marker trait for unit structs decorated with `#[graph(...)]`.
///
/// When such a type is used as a bare (non-`@`) identifier inside `orc!`,
/// the codegen emits `(&&Tag::<T>(...)).resolve()` which selects this impl
/// over the `Marker` leaf-node impl, expanding the sub-graph instead.
pub struct Ambiguous;

impl Bound for Ambiguous {}

/// `Ambiguous` deliberately does NOT implement `Entry` or `Exit`.
///
/// A `Graph<Ambiguous, _>` or `Graph<_, Ambiguous>` cannot be used where
/// `Entry`/`Exit` is required (selection branch, selector pivot, consumer
/// API matching). This self-limits propagation of imprecise bounds.

pub trait IsSubGraph: Send + Sync + 'static {}
