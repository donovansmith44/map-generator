//! The algebra the users feel (covenant rule 10). A trait without law
//! tests is just hope — the laws live in tests.rs and run over every
//! implementor.

/// A monoid: an identity element and an associative combine.
///
/// Implementors and what the laws buy:
/// - `Snapshot` under overlay — "overlay maps in a clean fashion" IS
///   this monoid (law 8), and accumulation views are its fold (law 9).
/// - `TransitionScript` under sequencing — composed journeys are
///   scripts too (laws 3 and 8).
pub trait Monoid {
    fn empty() -> Self;
    fn combine(self, other: Self) -> Self;
}

/// Fold a sequence of monoid values from the identity.
pub fn mconcat<M: Monoid>(xs: impl IntoIterator<Item = M>) -> M {
    xs.into_iter().fold(M::empty(), M::combine)
}
