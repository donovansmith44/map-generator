//! The compiler half of the canon design. Tests first: see tests.rs.

pub mod compile;
pub mod reconcile;
pub mod timeline_bridge;
pub mod vendor;

#[cfg(test)]
mod tests;
