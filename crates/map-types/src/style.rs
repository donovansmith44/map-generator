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
    /// Halo stroke width as a fraction of the label size — dress data,
    /// once a constant buried in two renderers.
    pub halo_width_em: f64,
}

/// The cartographic voice of a label: territories speak in spaced
/// capitals, water in italic, places in the plain hand. A voice is
/// STYLE DATA — the encoder renders exactly what the voice declares
/// and invents nothing, so changing type is editing a style, never
/// hunting constants in rendering code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, PartialOrd, Ord)]
pub enum LabelFace {
    /// A named land region.
    Territory,
    /// A sea, lake, or river.
    Water,
    /// A settlement or station.
    #[default]
    Place,
    /// A place that no longer stands, remembered at its traditional
    /// site: an inscription, not a dot.
    Memory,
}

/// Everything the layout and the glyphs need, declared: family stack,
/// weight, posture, case, tracking — and the mean glyph advance the
/// layout uses for fitting and collision boxes, so the measure always
/// matches the dress it measures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TypeVoice {
    pub family: &'static str,
    pub weight: u16,
    pub italic: bool,
    pub uppercase: bool,
    /// extra tracking between letters, in em
    pub tracking_em: f64,
    /// mean glyph advance INCLUDING tracking, in em
    pub advance_em: f64,
}

/// How an area label scales with the ground it names — declared, so a
/// style may choose quiet uniform labels or loud hierarchical ones.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelScale {
    /// ring area (steradians) whose label renders at exactly base size
    pub unit_area_sr: f64,
    /// clamp on the scale factor derived from area
    pub min: f64,
    pub max: f64,
    /// extra factor applied to water labels
    pub water_shrink: f64,
    /// water label ink = the water fill dimmed by this factor (0..1)
    pub water_ink: f64,
    /// extra factor applied to memory-site inscriptions
    pub memory_scale: f64,
    /// extra factor applied to journey-station names
    pub station_scale: f64,
    /// extra factor applied to settlement names — a city is a note,
    /// not a shout
    pub city_scale: f64,
}

/// The complete labeling dress: base ink plus the three voices plus
/// the scaling law. One value per style; every label decision reads
/// from here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Labeling {
    pub base: LabelStyle,
    pub territory: TypeVoice,
    pub water: TypeVoice,
    pub place: TypeVoice,
    pub memory: TypeVoice,
    pub scale: LabelScale,
}

impl Labeling {
    pub fn voice(&self, face: LabelFace) -> TypeVoice {
        match face {
            LabelFace::Territory => self.territory,
            LabelFace::Water => self.water,
            LabelFace::Place => self.place,
            LabelFace::Memory => self.memory,
        }
    }
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
    /// EdgeCharacter::Way — a journey's dress, distinct from every border.
    pub way: Stroke,
}

/// The page's own ground — the paper the whole map composes against,
/// and the globe's chrome (limb circle, graticule). Dress data: a dark
/// style gets a dark page, never a hardcoded cream.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlobeChrome {
    pub limb: Stroke,
    pub limb_fill: Paint,
    pub graticule: Stroke,
}

/// How the ghost backdrop fades a style — the disclosure dress the
/// rest of the world wears when one subject is realized. Injectable:
/// these factors were once constants in viewer code.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GhostDress {
    /// stroke alpha multiplier (0..=1)
    pub stroke_alpha: f64,
    /// fill alpha multiplier (0..=1)
    pub fill_alpha: f64,
    /// stroke width multiplier (> 0)
    pub width_factor: f64,
}

/// How stroke patterns realize on the page: dash rhythms and the zonal
/// band's proportions. One source of truth — every terminal encoder
/// reads THIS, instead of hardcoding the same convention twice and
/// drifting.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PatternGeometry {
    pub dashed_on: f64,
    pub dashed_off: f64,
    pub hatched_on: f64,
    pub hatched_off: f64,
    /// a zonal band is the stroke width times this
    pub zonal_width: f64,
    /// at the stroke alpha times this
    pub zonal_alpha: f64,
}

/// The classical reference values — what every template declared the
/// day these became data. Encoders fall back to these when no dress
/// is injected (tests, bare construction); the served styles always
/// carry their own.
impl Default for GlobeChrome {
    fn default() -> Self {
        GlobeChrome {
            limb: Stroke {
                color: Rgba(128, 128, 128, 128),
                width: 1.0,
                pattern: StrokePattern::Solid,
            },
            limb_fill: Paint { fill: Rgba(128, 128, 128, 15) },
            graticule: Stroke {
                color: Rgba(128, 128, 128, 56),
                width: 0.6,
                pattern: StrokePattern::Solid,
            },
        }
    }
}

impl Default for GhostDress {
    fn default() -> Self {
        GhostDress { stroke_alpha: 0.35, fill_alpha: 0.16, width_factor: 0.7 }
    }
}

/// THE FOCUS VEIL — one law, everywhere. When a selection stands,
/// the world outside it dims under THIS ink: the same veil over
/// every dress, every era, every location. Deliberately NOT a
/// per-template value — focus is a property of attention, not of
/// costume, so no dress and no place gets its own veil. Dark warm
/// ink at ~56% coverage: the background darkens but stays legible.
pub const VEIL: Paint = Paint { fill: Rgba(20, 18, 14, 143) };

impl Default for PatternGeometry {
    fn default() -> Self {
        PatternGeometry {
            dashed_on: 6.0,
            dashed_off: 4.0,
            hatched_on: 2.0,
            hatched_off: 3.0,
            zonal_width: 6.0,
            zonal_alpha: 0.35,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleError {
    /// Law 6: Unknown must render distinctly from Line.
    UnknownIndistinctFromLine,
    /// Covenant rule 5: a frontier is a gradient of control — it must
    /// render zonally, never as a false crisp line.
    FrontierNotZonal,
    /// A dress factor out of its lawful range (the message names it).
    DressOutOfRange(&'static str),
}

/// Everything a Style is made of, by name — the one argument to
/// `Style::new`, so growing the dress never grows a positional list.
#[derive(Clone, Copy, Debug)]
pub struct StyleSpec {
    pub boundaries: BoundaryStrokes,
    pub region: Paint,
    pub water: Paint,
    pub topo: AgeRamp,
    pub palette: Option<[Paint; 8]>,
    pub age: AgeRamp,
    pub labeling: Labeling,
    pub marker: MarkerStyle,
    pub delta: DeltaEmphasis,
    pub paper: Paint,
    pub chrome: GlobeChrome,
    pub ghost: GhostDress,
    /// the comparison-overlay tint's alpha
    pub tint_alpha: u8,
    pub pattern: PatternGeometry,
    /// rivers stroke at this width, in the water's own fill (a law:
    /// river ink follows the sea's)
    pub river_width: f64,
}

/// A complete style. Constructed only through `new`, which enforces the
/// honesty laws — a dishonest style is unrepresentable, not discouraged.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    boundaries: BoundaryStrokes,
    region: Paint,
    /// The sea's dress — water regions are as first-class as polities.
    water: Paint,
    /// Hypsometric tinting: lowest band -> highest band, mixed by
    /// band position (phase 5's "topo shading params" as data).
    topo: AgeRamp,
    /// The atlas convention: distinct fills so it is obvious what's
    /// what. None = every land region wears the uniform region paint;
    /// Some = territories are assigned from these slots such that
    /// touching territories never match (the provider's coloring).
    palette: Option<[Paint; 8]>,
    age: AgeRamp,
    labeling: Labeling,
    marker: MarkerStyle,
    delta: DeltaEmphasis,
    paper: Paint,
    chrome: GlobeChrome,
    ghost: GhostDress,
    tint_alpha: u8,
    pattern: PatternGeometry,
    river_width: f64,
}

impl Style {
    pub fn new(spec: StyleSpec) -> Result<Self, StyleError> {
        let StyleSpec {
            boundaries,
            region,
            water,
            topo,
            palette,
            age,
            labeling,
            marker,
            delta,
            paper,
            chrome,
            ghost,
            tint_alpha,
            pattern,
            river_width,
        } = spec;
        if boundaries.unknown == boundaries.line {
            return Err(StyleError::UnknownIndistinctFromLine);
        }
        if boundaries.frontier.pattern != StrokePattern::Zonal {
            return Err(StyleError::FrontierNotZonal);
        }
        let unit = |v: f64, name: &'static str| {
            if (0.0..=1.0).contains(&v) { Ok(()) } else { Err(StyleError::DressOutOfRange(name)) }
        };
        let positive = |v: f64, name: &'static str| {
            if v > 0.0 && v.is_finite() { Ok(()) } else { Err(StyleError::DressOutOfRange(name)) }
        };
        unit(ghost.stroke_alpha, "ghost.stroke_alpha")?;
        unit(ghost.fill_alpha, "ghost.fill_alpha")?;
        positive(ghost.width_factor, "ghost.width_factor")?;
        unit(pattern.zonal_alpha, "pattern.zonal_alpha")?;
        positive(pattern.zonal_width, "pattern.zonal_width")?;
        positive(pattern.dashed_on, "pattern.dashed_on")?;
        positive(pattern.dashed_off, "pattern.dashed_off")?;
        positive(pattern.hatched_on, "pattern.hatched_on")?;
        positive(pattern.hatched_off, "pattern.hatched_off")?;
        positive(river_width, "river_width")?;
        if !(labeling.base.halo_width_em.is_finite() && labeling.base.halo_width_em >= 0.0) {
            return Err(StyleError::DressOutOfRange("labeling.base.halo_width_em"));
        }
        positive(labeling.scale.memory_scale, "labeling.scale.memory_scale")?;
        positive(labeling.scale.station_scale, "labeling.scale.station_scale")?;
        positive(labeling.scale.city_scale, "labeling.scale.city_scale")?;
        Ok(Style {
            boundaries,
            region,
            water,
            topo,
            palette,
            age,
            labeling,
            marker,
            delta,
            paper,
            chrome,
            ghost,
            tint_alpha,
            pattern,
            river_width,
        })
    }

    pub fn stroke_for(&self, character: &EdgeCharacter) -> &Stroke {
        match character {
            EdgeCharacter::Line => &self.boundaries.line,
            EdgeCharacter::Frontier { .. } => &self.boundaries.frontier,
            EdgeCharacter::Disputed { .. } => &self.boundaries.disputed,
            EdgeCharacter::Unknown => &self.boundaries.unknown,
            EdgeCharacter::Way => &self.boundaries.way,
        }
    }
    pub fn region_paint(&self) -> Paint {
        self.region
    }
    pub fn water_paint(&self) -> Paint {
        self.water
    }
    pub fn palette(&self) -> Option<&[Paint; 8]> {
        self.palette.as_ref()
    }
    /// newest = highest band, oldest = lowest (reusing the ramp shape).
    pub fn topo_ramp(&self) -> AgeRamp {
        self.topo
    }
    pub fn age_ramp(&self) -> AgeRamp {
        self.age
    }
    pub fn label_style(&self) -> LabelStyle {
        self.labeling.base
    }
    pub fn labeling(&self) -> Labeling {
        self.labeling
    }
    pub fn marker_style(&self) -> MarkerStyle {
        self.marker
    }
    pub fn delta_emphasis(&self) -> DeltaEmphasis {
        self.delta
    }
    pub fn paper(&self) -> Paint {
        self.paper
    }
    pub fn chrome(&self) -> GlobeChrome {
        self.chrome
    }
    pub fn ghost_dress(&self) -> GhostDress {
        self.ghost
    }
    pub fn tint_alpha(&self) -> u8 {
        self.tint_alpha
    }
    pub fn pattern_geometry(&self) -> PatternGeometry {
        self.pattern
    }
    pub fn river_width(&self) -> f64 {
        self.river_width
    }

    /// IDENTITY IS TOTAL OVER THE DRESS: every field a renderer reads
    /// is hashed. A blind spot here lets two different dresses collide
    /// into one StyleId and the second silently vanishes from the
    /// style table — the way stroke, the four voices, and the scaling
    /// law were once invisible, which made a font-only restyle
    /// impossible to ship as a new template.
    pub fn canon(&self, c: &mut Canon) {
        c.tag("style");
        self.boundaries.line.canon(c);
        self.boundaries.frontier.canon(c);
        self.boundaries.disputed.canon(c);
        self.boundaries.unknown.canon(c);
        self.boundaries.way.canon(c);
        self.region.canon(c);
        self.water.canon(c);
        self.topo.newest.canon(c);
        self.topo.oldest.canon(c);
        c.opt(&self.palette, |c, pal| {
            for p in pal.iter() {
                p.canon(c);
            }
        });
        self.age.newest.canon(c);
        self.age.oldest.canon(c);
        let Rgba(r, g, b, a) = self.labeling.base.color;
        c.u8_(r).u8_(g).u8_(b).u8_(a);
        let Rgba(r, g, b, a) = self.labeling.base.halo;
        c.u8_(r).u8_(g).u8_(b).u8_(a).f64_(self.labeling.base.size);
        c.f64_(self.labeling.base.halo_width_em);
        for voice in [
            &self.labeling.territory,
            &self.labeling.water,
            &self.labeling.place,
            &self.labeling.memory,
        ] {
            c.str_(voice.family)
                .u8_((voice.weight >> 8) as u8)
                .u8_((voice.weight & 0xff) as u8)
                .u8_(voice.italic as u8)
                .u8_(voice.uppercase as u8)
                .f64_(voice.tracking_em)
                .f64_(voice.advance_em);
        }
        let sc = &self.labeling.scale;
        c.f64_(sc.unit_area_sr).f64_(sc.min).f64_(sc.max).f64_(sc.water_shrink).f64_(sc.water_ink);
        c.f64_(sc.memory_scale).f64_(sc.station_scale).f64_(sc.city_scale);
        let Rgba(r, g, b, a) = self.marker.color;
        c.u8_(r).u8_(g).u8_(b).u8_(a).f64_(self.marker.size);
        self.delta.before.canon(c);
        self.delta.after.canon(c);
        self.delta.seam.canon(c);
        self.paper.canon(c);
        self.chrome.limb.canon(c);
        self.chrome.limb_fill.canon(c);
        self.chrome.graticule.canon(c);
        c.f64_(self.ghost.stroke_alpha).f64_(self.ghost.fill_alpha).f64_(self.ghost.width_factor);
        c.u8_(self.tint_alpha);
        c.f64_(self.pattern.dashed_on)
            .f64_(self.pattern.dashed_off)
            .f64_(self.pattern.hatched_on)
            .f64_(self.pattern.hatched_off)
            .f64_(self.pattern.zonal_width)
            .f64_(self.pattern.zonal_alpha);
        c.f64_(self.river_width);
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
    /// Hypsometric elevation bands (phase 5). CONTRACT NOTE (for the
    /// C5 freeze): joined the vocabulary when relief landed.
    pub const RELIEF: LayerSet = LayerSet(8);
    /// Journeys: Way boundaries and their stations — an itinerary
    /// layer over the same globe, never part of the territorial
    /// GEOMETRY. CONTRACT NOTE (C5 freeze): joined the vocabulary when
    /// the whole-Bible route book landed.
    pub const JOURNEYS: LayerSet = LayerSet(16);

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
