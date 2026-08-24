//! Styles as DATA that merge predictably — never subclassed renderers
//! (covenant rule 10). Rules are keyed per subject kind (the atlas's
//! Presentable-per-kind discipline, mirrored): strokes per
//! EdgeCharacter, paint for regions, age->paint for accumulations,
//! typography for labels, emphasis for deltas.
//!
//! Honesty renders (covenant rule 5, law 6): a Style that draws
//! Unknown indistinguishably from Line, or a Frontier as a crisp
//! stroke, cannot be constructed.

use crate::boundary::EdgeCharacter;
use crate::ident::{Canon, MapAddressed, MapKind, StyleId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

/// How a boundary stroke is drawn. `Zonal` is the frontier treatment: a
/// soft band whose width comes from the DATA (EdgeCharacter::Frontier's
/// width_km), not from the style.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StrokePattern {
    Solid,
    Dashed,
    Hatched,
    Zonal,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Stroke {
    pub color: Rgba,
    pub width: f64,
    pub pattern: StrokePattern,
}

impl Stroke {
    pub fn canon(&self, c: &mut Canon) {
        let Rgba(r, g, b, a) = self.color;
        c.u8_(r).u8_(g).u8_(b).u8_(a).f64_(self.width).u8_(self.pattern as u8);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Paint {
    pub fill: Rgba,
}

impl Paint {
    pub fn canon(&self, c: &mut Canon) {
        let Rgba(r, g, b, a) = self.fill;
        c.u8_(r).u8_(g).u8_(b).u8_(a);
    }
}

/// Temporal depth as style, not machinery: accumulations map a
/// normalized age (0 = newest sample, 1 = oldest) onto paint, so "the
/// expansion of Rome" and "the shrinking of Judah" are the same query
/// with different palettes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AgeRamp {
    pub newest: Paint,
    pub oldest: Paint,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelStyle {
    pub color: Rgba,
    /// The casing that keeps text legible over any fill.
    pub halo: Rgba,
    pub size: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarkerStyle {
    pub color: Rgba,
    pub size: f64,
}

/// How a DELTA renders: before-stroke, after-stroke, the seam of a
/// split — "what changed at the fall of Samaria" is a scene, not a
/// caption.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeltaEmphasis {
    pub before: Stroke,
    pub after: Stroke,
    pub seam: Stroke,
}

/// Per-EdgeCharacter stroke rules.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundaryStrokes {
    pub line: Stroke,
    pub frontier: Stroke,
    pub disputed: Stroke,
    pub unknown: Stroke,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleError {
    /// Law 6: Unknown must render distinctly from Line.
    UnknownIndistinctFromLine,
    /// Covenant rule 5: a frontier is a gradient of control — it must
    /// render zonally, never as a false crisp line.
    FrontierNotZonal,
}

/// A complete style. Constructed only through `new`, which enforces the
/// honesty laws — a dishonest style is unrepresentable, not discouraged.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    boundaries: BoundaryStrokes,
    region: Paint,
    /// The sea's dress — water regions are as first-class as polities.
    water: Paint,
    age: AgeRamp,
    label: LabelStyle,
    marker: MarkerStyle,
    delta: DeltaEmphasis,
}

impl Style {
    pub fn new(
        boundaries: BoundaryStrokes,
        region: Paint,
        water: Paint,
        age: AgeRamp,
        label: LabelStyle,
        marker: MarkerStyle,
        delta: DeltaEmphasis,
    ) -> Result<Self, StyleError> {
        if boundaries.unknown == boundaries.line {
            return Err(StyleError::UnknownIndistinctFromLine);
        }
        if boundaries.frontier.pattern != StrokePattern::Zonal {
            return Err(StyleError::FrontierNotZonal);
        }
        Ok(Style { boundaries, region, water, age, label, marker, delta })
    }

    pub fn stroke_for(&self, character: &EdgeCharacter) -> &Stroke {
        match character {
            EdgeCharacter::Line => &self.boundaries.line,
            EdgeCharacter::Frontier { .. } => &self.boundaries.frontier,
            EdgeCharacter::Disputed { .. } => &self.boundaries.disputed,
            EdgeCharacter::Unknown => &self.boundaries.unknown,
        }
    }
    pub fn region_paint(&self) -> Paint {
        self.region
    }
    pub fn water_paint(&self) -> Paint {
        self.water
    }
    pub fn age_ramp(&self) -> AgeRamp {
        self.age
    }
    pub fn label_style(&self) -> LabelStyle {
        self.label
    }
    pub fn marker_style(&self) -> MarkerStyle {
        self.marker
    }
    pub fn delta_emphasis(&self) -> DeltaEmphasis {
        self.delta
    }

    pub fn canon(&self, c: &mut Canon) {
        c.tag("style");
        self.boundaries.line.canon(c);
        self.boundaries.frontier.canon(c);
        self.boundaries.disputed.canon(c);
        self.boundaries.unknown.canon(c);
        self.region.canon(c);
        self.water.canon(c);
        self.age.newest.canon(c);
        self.age.oldest.canon(c);
        let Rgba(r, g, b, a) = self.label.color;
        c.u8_(r).u8_(g).u8_(b).u8_(a);
        let Rgba(r, g, b, a) = self.label.halo;
        c.u8_(r).u8_(g).u8_(b).u8_(a).f64_(self.label.size);
        let Rgba(r, g, b, a) = self.marker.color;
        c.u8_(r).u8_(g).u8_(b).u8_(a).f64_(self.marker.size);
        self.delta.before.canon(c);
        self.delta.after.canon(c);
        self.delta.seam.canon(c);
    }
}

/// Styles are content-addressed data: restyling changes the id, so
/// caches never serve a stale look.
impl MapAddressed for Style {
    fn canonical_bytes(&self) -> Vec<u8> {
        let mut c = Canon::new();
        self.canon(&mut c);
        c.done()
    }
    fn map_kind(&self) -> MapKind {
        MapKind::Style
    }
}

impl Style {
    pub fn id(&self) -> StyleId {
        StyleId(self.map_pid().hash)
    }
}

/// Which layers a query wants. Bit-set, closed vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LayerSet(u8);

impl LayerSet {
    pub const GEOMETRY: LayerSet = LayerSet(1);
    pub const TOPOGRAPHY: LayerSet = LayerSet(2);
    pub const LABELS: LayerSet = LayerSet(4);

    pub fn empty() -> Self {
        LayerSet(0)
    }
    pub fn with(self, other: LayerSet) -> Self {
        LayerSet(self.0 | other.0)
    }
    pub fn contains(self, other: LayerSet) -> bool {
        self.0 & other.0 == other.0
    }
    pub fn bits(self) -> u8 {
        self.0
    }
}
