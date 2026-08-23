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

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub use atlas_graph_types::id::ContentHash;

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
        let mut h = DefaultHasher::new();
        self.canonical_bytes().hash(&mut h);
        MapPid { kind: self.map_kind(), hash: ContentHash(h.finish()) }
    }
}

/// Canonical-byte writer: every field lands tagged and length-framed, so
/// distinct values cannot collide by concatenation ambiguity.
#[derive(Default)]
pub struct Canon(pub Vec<u8>);

impl Canon {
    pub fn new() -> Self {
        Canon(Vec::new())
    }
    pub fn tag(&mut self, t: &str) -> &mut Self {
        self.str_(t)
    }
    pub fn u8_(&mut self, v: u8) -> &mut Self {
        self.0.push(v);
        self
    }
    pub fn u64_(&mut self, v: u64) -> &mut Self {
        self.0.extend_from_slice(&v.to_be_bytes());
        self
    }
    pub fn i32_(&mut self, v: i32) -> &mut Self {
        self.0.extend_from_slice(&v.to_be_bytes());
        self
    }
    /// Floats canonicalize by bit pattern — no epsilon, no locale.
    pub fn f64_(&mut self, v: f64) -> &mut Self {
        self.u64_(v.to_bits())
    }
    pub fn str_(&mut self, s: &str) -> &mut Self {
        self.u64_(s.len() as u64);
        self.0.extend_from_slice(s.as_bytes());
        self
    }
    pub fn opt<T>(&mut self, v: &Option<T>, f: impl FnOnce(&mut Self, &T)) -> &mut Self {
        match v {
            None => self.u8_(0),
            Some(x) => {
                self.u8_(1);
                f(self, x);
                self
            }
        }
    }
    pub fn seq<T>(&mut self, xs: &[T], f: impl Fn(&mut Self, &T)) -> &mut Self {
        self.u64_(xs.len() as u64);
        for x in xs {
            f(self, x);
        }
        self
    }
    pub fn done(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.0)
    }
}

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
