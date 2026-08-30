//! Hydrography: the WHOLE world is the map — seas, lakes, coastlines
//! are explorable regions, not background (owner order: "we care about
//! oceans"). Source: Natural Earth (public domain), ingested as
//! RegionClass::Water regions whose validity opens at the configured
//! start (under the biblical frame, the waters gathered at creation,
//! GEN 1:9-10 — the DATE claim is Scripture's; the SHAPES are modern
//! coastline data and say so in their provenance).

use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use atlas_graph_types::covenant::TimePoint;
use atlas_graph_types::covenant::{Ground, Justification};
use atlas_graph_types::covenant::{ContentHash, SourceId};
use atlas_graph_types::covenant::{BibleLocus, LocusRange, VerseRef};

use map_types::{
    Boundary, BoundaryHistory, BoundaryId, BoundarySource, EdgeCharacter, Interval, Orientation,
    RegionClass, RegionGeom, RegionHistory, RegionId, RegionPart, UnitVec, WorldTimeline,
};

use crate::basemaps::IngestError;
use crate::geojson::parse_features;
use crate::quantize::clean_ring;

const HYDRO_PROVENANCE: &str = "natural-earth (public domain; modern coastline shapes)";

fn hash_id(tag: &str) -> ContentHash {
    let mut h = DefaultHasher::new();
    tag.hash(&mut h);
    ContentHash(h.finish())
}

/// One water dataset: its text and the label unnamed features carry
/// (the global ocean ships nameless — Scripture already named it).
pub struct WaterSource {
    pub label_for_unnamed: &'static str,
    pub text: String,
    /// Drop the feature with the most vertices. The ocean file's main
    /// feature is a pole-and-antimeridian-stitched envelope that is
    /// degenerate on a sphere — the real ocean is built by
    /// `ingest_ocean` as the sphere minus land; the ocean file then
    /// contributes only its interior seas (the Caspian).
    pub skip_largest_feature: bool,
}

/// Ingest water bodies as first-class regions, valid from `from` to the
/// open edge of knowledge. Same-named features (a lake in two pieces)
/// merge into one multi-part region.
pub fn ingest_water(
    source: &SourceId,
    from: TimePoint,
    waters: &[WaterSource],
) -> Result<WorldTimeline, IngestError> {
    let gen_1_9_10 = || {
        let v = |verse| BibleLocus::whole(VerseRef { book: 1, chapter: 1, verse });
        LocusRange::new(v(9), v(10)).expect("GEN 1:9-10 is ordered")
    };
    let justification = Justification {
        text: Some(
            "\"And the gathering together of the waters called he Seas\" — the date \
             is the Word's; the shapes are modern coastline data (see provenance)."
                .to_string(),
        ),
        grounds: [Ground::Scripture(gen_1_9_10()), Ground::Source(source.clone())].into(),
    };
    let valid = Interval { from, to: None };

    let mut tl = WorldTimeline::default();
    // name -> parts, accumulated across features and datasets.
    let mut regions: BTreeMap<String, Vec<RegionPart>> = BTreeMap::new();

    for w in waters {
        let mut features = parse_features(&w.text)
            .map_err(|e| IngestError::Parse(w.label_for_unnamed.to_string(), e))?;
        if w.skip_largest_feature {
            let vertex_count = |f: &crate::geojson::SourceFeature| -> usize {
                f.polygons
                    .iter()
                    .map(|p| p.outer.len() + p.holes.iter().map(Vec::len).sum::<usize>())
                    .sum()
            };
            if let Some(largest) =
                (0..features.len()).max_by_key(|&i| vertex_count(&features[i]))
            {
                features.remove(largest);
            }
        }
        for f in &features {
            let name = f.name.clone().unwrap_or_else(|| w.label_for_unnamed.to_string());
            for poly in &f.polygons {
                let mut cycles: Vec<Vec<(BoundaryId, Orientation)>> = Vec::new();
                for ring in std::iter::once(&poly.outer).chain(&poly.holes) {
                    // Water keeps its full precision: no snap — the
                    // coastline IS the detail we came for.
                    let Some(pts) = clean_ring(ring, None) else { continue };
                    let mut closed: Vec<UnitVec> =
                        pts.iter().map(|q| q.to_unit_vec()).collect();
                    closed.push(closed[0]);
                    let bid = BoundaryId({
                        let mut h = DefaultHasher::new();
                        "natural-earth/shore".hash(&mut h);
                        for q in &pts {
                            q.lon.hash(&mut h);
                            q.lat.hash(&mut h);
                        }
                        ContentHash(h.finish())
                    });
                    tl.boundaries.insert(
                        bid,
                        BoundaryHistory {
                            versions: vec![(
                                valid,
                                Boundary {
                                    pts: closed,
                                    character: EdgeCharacter::Line,
                                    source: BoundarySource::Imported { source: source.clone() },
                                    justification: justification.clone(),
                                    provenance: HYDRO_PROVENANCE.to_string(),
                                },
                            )],
                        },
                    );
                    cycles.push(vec![(bid, Orientation::Forward)]);
                }
                if cycles.is_empty() {
                    continue;
                }
                let mut it = cycles.into_iter();
                let outer = it.next().unwrap();
                regions
                    .entry(name.clone())
                    .or_default()
                    .push(RegionPart { cycle: outer, holes: it.collect() });
            }
        }
    }

    for (name, parts) in regions {
        let rid = RegionId(hash_id(&format!("natural-earth/water/{name}")));
        tl.regions.insert(
            rid,
            RegionHistory {
                class: RegionClass::Water,
                label_history: vec![(valid, name)],
                geom_history: vec![(valid, RegionGeom { parts })],
            },
        );
    }
    Ok(tl)
}

/// THE OCEAN, built the spherically honest way: the whole sphere minus
/// the land (RegionPart's empty-cycle convention) — no fictitious
/// envelope boundary. The land rings become the ocean's holes AND its
/// coastline arcs, so shores render as real strokes.
pub fn ingest_ocean(
    source: &SourceId,
    from: TimePoint,
    land_text: &str,
) -> Result<WorldTimeline, IngestError> {
    let gen_1_9_10 = || {
        let v = |verse| BibleLocus::whole(VerseRef { book: 1, chapter: 1, verse });
        LocusRange::new(v(9), v(10)).expect("GEN 1:9-10 is ordered")
    };
    let justification = Justification {
        text: Some(
            "\"And the gathering together of the waters called he Seas\" — the date \
             is the Word's; the shapes are modern coastline data (see provenance)."
                .to_string(),
        ),
        grounds: [Ground::Scripture(gen_1_9_10()), Ground::Source(source.clone())].into(),
    };
    let valid = Interval { from, to: None };

    let mut tl = WorldTimeline::default();
    let features =
        parse_features(land_text).map_err(|e| IngestError::Parse("land".to_string(), e))?;
    let mut holes: Vec<Vec<(BoundaryId, Orientation)>> = Vec::new();
    for f in &features {
        for poly in &f.polygons {
            // Only the land's outer rings shape the sea; land's own
            // holes are inland matters.
            let Some(pts) = clean_ring(&poly.outer, None) else { continue };
            let mut closed: Vec<UnitVec> = pts.iter().map(|q| q.to_unit_vec()).collect();
            closed.push(closed[0]);
            let bid = BoundaryId({
                let mut h = DefaultHasher::new();
                "natural-earth/coast".hash(&mut h);
                for q in &pts {
                    q.lon.hash(&mut h);
                    q.lat.hash(&mut h);
                }
                ContentHash(h.finish())
            });
            tl.boundaries.insert(
                bid,
                BoundaryHistory {
                    versions: vec![(
                        valid,
                        Boundary {
                            pts: closed,
                            character: EdgeCharacter::Line,
                            source: BoundarySource::Imported { source: source.clone() },
                            justification: justification.clone(),
                            provenance: HYDRO_PROVENANCE.to_string(),
                        },
                    )],
                },
            );
            holes.push(vec![(bid, Orientation::Forward)]);
        }
    }
    let rid = RegionId(hash_id("natural-earth/water/the sea"));
    // THE DRY LAND (GEN 1:10), worldwide: the very same coastline
    // borders the sea holds as holes, standing as land cycles —
    // content addressing makes coast and stage one line on every
    // shore of the earth, and the whole Biblical world gets its
    // parchment at every era and every zoom.
    let land_parts: Vec<RegionPart> =
        holes.iter().map(|cycle| RegionPart { cycle: cycle.clone(), holes: vec![] }).collect();
    tl.regions.insert(
        rid,
        RegionHistory {
            class: RegionClass::Water,
            label_history: vec![(valid, "the sea".to_string())],
            geom_history: vec![(
                valid,
                RegionGeom { parts: vec![RegionPart { cycle: Vec::new(), holes }] },
            )],
        },
    );
    let land_rid = RegionId(hash_id("natural-earth/land/the dry land"));
    tl.regions.insert(
        land_rid,
        RegionHistory {
            class: RegionClass::Terrain(0),
            label_history: vec![(valid, "the dry land".to_string())],
            geom_history: vec![(valid, RegionGeom { parts: land_parts })],
        },
    );
    Ok(tl)
}
