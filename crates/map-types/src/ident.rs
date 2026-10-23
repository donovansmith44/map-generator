//! Identity: the atlas's content-address discipline (C1), carried by the
//! map system's own kind vocabulary.
//!
//! DELIBERATE DEVIATION (recorded for owner review): the atlas exposes
//! `ContentAddressed`, whose `Pid` is typed over the ATLAS's closed
//! `PositionKind`/`NodeKind` enums. Map subjects (Region, Boundary,
//! Style, ...) are not atlas node kinds, the atlas enum is closed by
//! design, and this session is forbidden to edit the atlas repo — so the
//! map system mirrors the DISCIPLINE (one canonical byte form, id =
//! kind + hash of those bytes) with its own kind enum, while reusing the
//! atlas's `ContentHash` so the two systems' hashes are the same shape.
//! When the atlas-side `covenant` module lands (spec §C1 prerequisite),
//! collapsing this mirror is a mechanical follow-up.

#[cfg(not(feature = "canon-ids"))]
use std::collections::hash_map::DefaultHasher;
#[cfg(not(feature = "canon-ids"))]
use std::hash::{Hash, Hasher};

pub use atlas_graph_types::covenant::ContentHash;

/// Closed map-side kind vocabulary. Extending it is a deliberate act
/// every exhaustive match must acknowledge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MapKind {
    Region,
    Boundary,
    ChangeEvent,
    Style,
    Query,
    Scene,
    Timeline,
}

/// Content-addressed map identity: kind + hash(canonical bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MapPid {
    pub kind: MapKind,
    pub hash: ContentHash,
}

/// Everything addressable defines one canonical byte form; the id is a
/// key from which the thing is derivable. Same skeleton simplification
/// as the atlas: a 64-bit std hash stands in for a real multihash.
pub trait MapAddressed {
    fn canonical_bytes(&self) -> Vec<u8>;
    fn map_kind(&self) -> MapKind;
    fn map_pid(&self) -> MapPid {
        #[cfg(feature = "canon-ids")]
        let hash = atlas_graph_types::sections::logical_hash(&self.canonical_bytes());
        #[cfg(not(feature = "canon-ids"))]
        let hash = {
            let mut h = DefaultHasher::new();
            self.canonical_bytes().hash(&mut h);
            ContentHash(h.finish())
        };
        MapPid { kind: self.map_kind(), hash }
    }
}

pub use atlas_graph_types::map_native::NativeCanon as Canon;

macro_rules! content_ids {
    ($($(#[$doc:meta])* $name:ident),+ $(,)?) => {$(
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub ContentHash);
    )+};
}

content_ids! {
    /// A polity/province/realm — content-addressed like atlas Pids.
    RegionId,
    /// A shared border arc.
    BoundaryId,
    /// A narrated border change.
    ChangeEventId,
    /// A style, addressed by its content so caching survives restyling.
    StyleId,
}
