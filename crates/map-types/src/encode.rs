//! OUTPUT DECOUPLING: encoders are TERMINAL (spec §B; owner: "we need
//! to be decoupled from particular output formats because i don't know
//! what's gonna work").
//!
//! The model commits to no format. `Snapshot` is the last semantic
//! type; every concrete format — SVG, GeoJSON, vector tiles, raster
//! PNG (a weak tablet GPU may well want pre-rasterized output!), WebGL
//! buffers, PDF plates for print — is an encoder backend behind this
//! one trait. Adding a format touches nothing upstream; killing one
//! loses nothing but itself. Performance testing on the actual tablet
//! decides the format later; the architecture refuses to decide it now.
//!
//! Law 11 (encoder terminality), enforced by test:
//! - no type or function upstream of this module names a concrete
//!   output format (grep-enforced over the crate source);
//! - every encoder is deterministic: same scene + same encoder config
//!   -> same bytes, so content-addressed caching survives encoding;
//! - composition never happens post-encoding: overlay and accumulate
//!   operate on scenes only — encoded artifacts are leaves.
//!
//! `TransitionScript` gets the same treatment when transition encoding
//! lands: a semantic script, encodable per backend.

use crate::scene::Snapshot;
use crate::transition::TransitionScript;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodeError(pub String);

pub trait SceneEncoder {
    type Output;
    fn encode(&self, scene: &Snapshot) -> Result<Self::Output, EncodeError>;
}

/// The promised treatment (phase 6): a `TransitionScript` is the last
/// semantic type on the animation path, and every concrete animation
/// format — JSON for a web player, CSS keyframes, a video timeline —
/// is a terminal backend behind this trait, under the same law 11
/// terms as scenes: deterministic bytes, no composition post-encode.
pub trait TransitionEncoder {
    type Output;
    fn encode_transition(&self, script: &TransitionScript) -> Result<Self::Output, EncodeError>;
}
