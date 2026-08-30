//! PLUG-AND-CHUG STYLE TEMPLATES (the canon design's promise): a
//! style is a DATA FILE in `templates/*.ron`, loaded at startup and
//! validated through `Style::new` — the honesty laws run on every
//! template, so a dishonest file refuses to serve, loudly, by name.
//! A new look is a new file; no recompile.
//!
//! The mirror types below are the file schema. They convert into
//! `map_types::style::Style` and nothing else — rendering code never
//! sees a template. Font family strings are leaked to `'static` once
//! per process (templates load exactly once), which keeps the whole
//! Style tree `Copy` as the scene types require.

use map_types::style::{
    AgeRamp, BoundaryStrokes, DeltaEmphasis, LabelScale, LabelStyle, Labeling, MarkerStyle, Paint,
    Rgba, Stroke, StrokePattern, Style, TypeVoice,
};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TStroke {
    color: [u8; 4],
    width: f64,
    pattern: TPattern,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum TPattern {
    Solid,
    Dashed,
    Hatched,
    Zonal,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TRamp {
    newest: [u8; 4],
    oldest: [u8; 4],
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TVoice {
    family: String,
    weight: u16,
    italic: bool,
    uppercase: bool,
    tracking_em: f64,
    advance_em: f64,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TLabel {
    color: [u8; 4],
    halo: [u8; 4],
    size: f64,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TScale {
    unit_area_sr: f64,
    min: f64,
    max: f64,
    water_shrink: f64,
    water_ink: f64,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TLabeling {
    base: TLabel,
    territory: TVoice,
    water: TVoice,
    place: TVoice,
    memory: TVoice,
    scale: TScale,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TBoundaries {
    line: TStroke,
    frontier: TStroke,
    disputed: TStroke,
    unknown: TStroke,
    way: TStroke,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TMarker {
    color: [u8; 4],
    size: f64,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TDelta {
    before: TStroke,
    after: TStroke,
    seam: TStroke,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Template {
    boundaries: TBoundaries,
    region: [u8; 4],
    water: [u8; 4],
    topo: TRamp,
    palette: Option<[[u8; 4]; 8]>,
    age: TRamp,
    labeling: TLabeling,
    marker: TMarker,
    delta: TDelta,
}

fn rgba(c: [u8; 4]) -> Rgba {
    Rgba(c[0], c[1], c[2], c[3])
}

fn paint(c: [u8; 4]) -> Paint {
    Paint { fill: rgba(c) }
}

fn stroke(s: &TStroke) -> Stroke {
    Stroke {
        color: rgba(s.color),
        width: s.width,
        pattern: match s.pattern {
            TPattern::Solid => StrokePattern::Solid,
            TPattern::Dashed => StrokePattern::Dashed,
            TPattern::Hatched => StrokePattern::Hatched,
            TPattern::Zonal => StrokePattern::Zonal,
        },
    }
}

fn ramp(r: &TRamp) -> AgeRamp {
    AgeRamp { newest: paint(r.newest), oldest: paint(r.oldest) }
}

fn voice(v: TVoice) -> TypeVoice {
    TypeVoice {
        // leaked once per process at load: templates are read exactly
        // once, and 'static keeps the Style tree Copy
        family: Box::leak(v.family.into_boxed_str()),
        weight: v.weight,
        italic: v.italic,
        uppercase: v.uppercase,
        tracking_em: v.tracking_em,
        advance_em: v.advance_em,
    }
}

fn build(t: Template) -> Result<Style, map_types::style::StyleError> {
    Style::new(
        BoundaryStrokes {
            line: stroke(&t.boundaries.line),
            frontier: stroke(&t.boundaries.frontier),
            disputed: stroke(&t.boundaries.disputed),
            unknown: stroke(&t.boundaries.unknown),
            way: stroke(&t.boundaries.way),
        },
        paint(t.region),
        paint(t.water),
        ramp(&t.topo),
        t.palette.map(|slots| slots.map(paint)),
        ramp(&t.age),
        Labeling {
            base: LabelStyle {
                color: rgba(t.labeling.base.color),
                halo: rgba(t.labeling.base.halo),
                size: t.labeling.base.size,
            },
            territory: voice(t.labeling.territory),
            water: voice(t.labeling.water),
            place: voice(t.labeling.place),
            memory: voice(t.labeling.memory),
            scale: LabelScale {
                unit_area_sr: t.labeling.scale.unit_area_sr,
                min: t.labeling.scale.min,
                max: t.labeling.scale.max,
                water_shrink: t.labeling.scale.water_shrink,
                water_ink: t.labeling.scale.water_ink,
            },
        },
        MarkerStyle { color: rgba(t.marker.color), size: t.marker.size },
        DeltaEmphasis {
            before: stroke(&t.delta.before),
            after: stroke(&t.delta.after),
            seam: stroke(&t.delta.seam),
        },
    )
}

/// Parse one template source into an honest Style; the error names
/// what went wrong (schema or honesty law) for the loud path.
pub fn parse_template(src: &str) -> Result<Style, String> {
    let t: Template = ron::from_str(src).map_err(|e| format!("template schema: {e}"))?;
    build(t).map_err(|e| format!("dishonest template: {e:?}"))
}

/// Every `*.ron` in the templates directory, alphabetically — the
/// style book is the directory listing, nothing else. A file that
/// fails to parse or fails the honesty laws kills startup by name.
pub fn load_templates(dir: &std::path::Path) -> Vec<(&'static str, Style)> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("no style templates at {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "ron"))
        .collect();
    entries.sort();
    assert!(!entries.is_empty(), "no *.ron templates in {}", dir.display());
    entries
        .into_iter()
        .map(|path| {
            let name = path.file_stem().expect("stem").to_string_lossy().into_owned();
            let src = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("template {}: {e}", path.display()));
            let style = parse_template(&src)
                .unwrap_or_else(|e| panic!("template {}: {e}", path.display()));
            (&*Box::leak(name.into_boxed_str()), style)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    /// Every shipped template parses and passes the honesty laws —
    /// the style book cannot drift into unservable states silently.
    #[test]
    fn shipped_templates_are_honest() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../templates");
        let loaded = super::load_templates(&dir);
        assert!(loaded.len() >= 3, "the style book has its dresses");
        let names: Vec<_> = loaded.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"canaan") && names.contains(&"parchment") && names.contains(&"slate"));
    }

    /// A dishonest template refuses to become a Style: the honesty
    /// laws run on DATA now, same as they ran on code.
    #[test]
    fn dishonest_template_refuses() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../templates");
        let mut src =
            std::fs::read_to_string(dir.join("parchment.ron")).expect("parchment template");
        // make `unknown` identical to `line`: law 6 must reject it
        src = src.replace(
            "unknown:  (color: (120, 116, 105, 255), width: 1.1, pattern: dashed)",
            "unknown:  (color: (74, 52, 34, 255),    width: 1.2, pattern: solid)",
        );
        let err = super::parse_template(&src).unwrap_err();
        assert!(err.contains("dishonest"), "the refusal names the crime: {err}");
    }

    /// Schema drift is loud: an unknown field is an error naming it,
    /// never silently ignored styling.
    #[test]
    fn unknown_fields_are_named_errors() {
        let err = super::parse_template("(bogus: 1)").unwrap_err();
        assert!(err.contains("schema"), "{err}");
    }
}
