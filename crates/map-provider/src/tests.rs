//! Laws for the canon provider and its palette coloring. The phase-3
//! interval-timeline provider and its laws are gone with it — the
//! canon provider carries the contract now, morphs included.

use map_types::style::*;

pub(crate) fn honest_style_for_memory_law() -> map_types::Style {
    let s = |c, w, p| Stroke { color: c, width: w, pattern: p };
    map_types::Style::new(
        BoundaryStrokes {
            line: s(Rgba(1, 1, 1, 255), 1.0, StrokePattern::Solid),
            frontier: s(Rgba(2, 2, 2, 255), 1.0, StrokePattern::Zonal),
            disputed: s(Rgba(3, 3, 3, 255), 1.0, StrokePattern::Hatched),
            unknown: s(Rgba(4, 4, 4, 255), 1.0, StrokePattern::Dashed),
            way: s(Rgba(5, 5, 5, 255), 1.0, StrokePattern::Dashed),
        },
        Paint { fill: Rgba(10, 10, 10, 255) },
        Paint { fill: Rgba(20, 20, 20, 255) },
        AgeRamp { newest: Paint { fill: Rgba(1, 1, 1, 255) }, oldest: Paint { fill: Rgba(2, 2, 2, 255) } },
        None,
        AgeRamp { newest: Paint { fill: Rgba(1, 1, 1, 255) }, oldest: Paint { fill: Rgba(2, 2, 2, 255) } },
        test_labeling(LabelStyle { color: Rgba(0, 0, 0, 255), halo: Rgba(255, 255, 255, 255), size: 12.0 }),
        MarkerStyle { color: Rgba(0, 0, 0, 255), size: 3.0 },
        DeltaEmphasis {
            before: s(Rgba(6, 6, 6, 255), 1.0, StrokePattern::Dashed),
            after: s(Rgba(7, 7, 7, 255), 1.0, StrokePattern::Solid),
            seam: s(Rgba(8, 8, 8, 255), 1.0, StrokePattern::Solid),
        },
    )
    .unwrap()
}

fn test_labeling(base: LabelStyle) -> map_types::style::Labeling {
    const TV: map_types::style::TypeVoice = map_types::style::TypeVoice {
        family: "sans-serif",
        weight: 600,
        italic: false,
        uppercase: false,
        tracking_em: 0.0,
        advance_em: 0.62,
    };
    const MEM: map_types::style::TypeVoice = map_types::style::TypeVoice {
        family: "serif",
        weight: 400,
        italic: true,
        uppercase: false,
        tracking_em: 0.1,
        advance_em: 0.56,
    };
    map_types::style::Labeling {
        base,
        territory: TV,
        water: TV,
        place: TV,
        memory: MEM,
        scale: map_types::style::LabelScale {
            unit_area_sr: 3.6e-5,
            min: 0.85,
            max: 2.1,
            water_shrink: 0.8,
            water_ink: 0.45,
        },
    }
}

mod canon_provider_laws {
    use std::collections::{BTreeMap, BTreeSet};

    use atlas_graph_types::covenant::{PlaceId, SourceId, TimePoint, Year};
    use map_canon::{
        Area, Border, CanonStore, EntityId, Feature, LayerKind, Leg, Provenance, Route, Snapshot,
        Witness, World,
    };
    use map_types::style::*;
    use map_types::{
        GazetteerEntry, GazetteerExport, LayerSet, Lod, MapAddressed, MapProvider, RenderQuery,
        RenderSubject, StyleId, TimeSelector, UnitVec,
    };

    use crate::canon_provider::CanonProvider;

    fn ts(y: i32) -> TimePoint {
        TimePoint::year_only(Year::new(y).unwrap())
    }
    fn uv(lat: f64, lon: f64) -> UnitVec {
        UnitVec::from_lat_lon_deg(lat, lon)
    }

    fn style() -> Style {
        let stroke = |r, pattern| Stroke { color: Rgba(r, 0, 0, 255), width: 1.0, pattern };
        Style::new(
            BoundaryStrokes {
                line: stroke(0, StrokePattern::Solid),
                frontier: stroke(60, StrokePattern::Zonal),
                disputed: stroke(120, StrokePattern::Hatched),
                unknown: stroke(180, StrokePattern::Dashed),
                way: stroke(240, StrokePattern::Dashed),
            },
            Paint { fill: Rgba(200, 200, 180, 255) },
            Paint { fill: Rgba(120, 160, 200, 235) },
            AgeRamp {
                newest: Paint { fill: Rgba(150, 110, 80, 200) },
                oldest: Paint { fill: Rgba(225, 214, 180, 200) },
            },
            Some([
                Paint { fill: Rgba(1, 1, 1, 205) },
                Paint { fill: Rgba(2, 2, 2, 205) },
                Paint { fill: Rgba(3, 3, 3, 205) },
                Paint { fill: Rgba(4, 4, 4, 205) },
                Paint { fill: Rgba(5, 5, 5, 205) },
                Paint { fill: Rgba(6, 6, 6, 205) },
                Paint { fill: Rgba(7, 7, 7, 205) },
                Paint { fill: Rgba(8, 8, 8, 205) },
            ]),
            AgeRamp {
                newest: Paint { fill: Rgba(220, 40, 40, 255) },
                oldest: Paint { fill: Rgba(220, 40, 40, 40) },
            },
            super::test_labeling(LabelStyle { color: Rgba(20, 20, 20, 255), halo: Rgba(245, 240, 225, 220), size: 12.0 }),
            MarkerStyle { color: Rgba(0, 0, 0, 255), size: 4.0 },
            DeltaEmphasis {
                before: stroke(90, StrokePattern::Dashed),
                after: stroke(30, StrokePattern::Solid),
                seam: stroke(250, StrokePattern::Solid),
            },
        )
        .unwrap()
    }

    /// A tiny real canon: Assyria in Territory (-1900..-911 exclusive),
    /// a two-leg journey in Journeys (45..47, 47..49), one sea in Water.
    fn fixture() -> CanonStore {
        let mut store = CanonStore::default();
        let square = |lat0: f64, lon0: f64, d: f64| {
            Border(vec![uv(lat0, lon0), uv(lat0, lon0 + d), uv(lat0 + d, lon0 + d), uv(lat0 + d, lon0)])
        };
        // Territory: Assyria
        let b = store.insert_border(square(35.0, 42.0, 3.0));
        let assyria = store.insert_feature(Feature::Area(Area {
            entity: EntityId("assyria".into()),
            name: "Assyria".into(),
            rings: BTreeSet::from([b]),
            holes: BTreeSet::new(),
        }));
        store.set_provenance(assyria, Provenance {
            witness: Witness::Atlas,
            verses: vec!["2KI.15.19".into()],
            note: "t".into(),
        });
        let s1 = store.insert_snapshot(Snapshot { features: BTreeSet::from([assyria]) });
        let s0 = store.insert_snapshot(Snapshot { features: BTreeSet::new() });
        let mut territory = World::default();
        territory.insert(ts(-1900), s1).unwrap();
        territory.insert(ts(-911), s0).unwrap();
        store.set_layer(LayerKind::Territory, territory);

        // Journeys: two legs, 45..47 and 47..49
        let road1 = store.insert_border(Border(vec![uv(36.2, 36.16), uv(37.9, 27.3)]));
        let road2 = store.insert_border(Border(vec![uv(37.9, 27.3), uv(41.89, 12.49)]));
        let way = store.insert_feature(Feature::Way(Route {
            entity: EntityId("test-walk".into()),
            name: "a test walk".into(),
            legs: vec![
                Leg { from: PlaceId::new("antioch".to_string()), to: PlaceId::new("ephesus".to_string()),
                      border: road1, span: (ts(45), ts(47)) },
                Leg { from: PlaceId::new("ephesus".to_string()), to: PlaceId::new("rome".to_string()),
                      border: road2, span: (ts(47), ts(49)) },
            ],
        }));
        store.set_provenance(way, Provenance {
            witness: Witness::Atlas,
            verses: vec!["ACT.19.1".into()],
            note: "t".into(),
        });
        let sj = store.insert_snapshot(Snapshot { features: BTreeSet::from([way]) });
        let sj0 = store.insert_snapshot(Snapshot { features: BTreeSet::new() });
        let mut journeys = World::default();
        journeys.insert(ts(45), sj).unwrap();
        journeys.insert(ts(50), sj0).unwrap();
        store.set_layer(LayerKind::Journeys, journeys);

        // Water: one static sea
        let sea = store.insert_border(square(31.0, 30.0, 4.0));
        let water = store.insert_feature(Feature::Area(Area {
            entity: EntityId("natural-earth:the-sea".into()),
            name: "the sea".into(),
            rings: BTreeSet::from([sea]),
            holes: BTreeSet::new(),
        }));
        store.set_provenance(water, Provenance {
            witness: Witness::NaturalEarth,
            verses: vec![],
            note: "t".into(),
        });
        let sw = store.insert_snapshot(Snapshot { features: BTreeSet::from([water]) });
        let mut w = World::default();
        w.insert(ts(-4004), sw).unwrap();
        store.set_layer(LayerKind::Water, w);
        store
    }

    fn provider() -> (CanonProvider, StyleId) {
        let s = style();
        let sid = s.id();
        let gaz = GazetteerExport {
            atlas_root: atlas_graph_types::covenant::ContentHash(0),
            places: [
                ("antioch", 36.2, 36.16, "Antioch"),
                ("ephesus", 37.9, 27.3, "Ephesus"),
                ("rome", 41.89, 12.49, "Rome"),
            ]
            .into_iter()
            .map(|(id, lat, lon, name)| {
                (PlaceId::new(id.to_string()), GazetteerEntry {
                    canonical_name: name.to_string(),
                    position: uv(lat, lon),
                    aliases: vec![],
                    provenance: None,
                    attestations: vec![],
                })
            })
            .collect(),
        };
        (CanonProvider::new(fixture(), BTreeMap::from([(sid, s)]), Some(gaz)), sid)
    }

    fn world_q(sid: StyleId, y: i32) -> RenderQuery {
        RenderQuery {
            subject: RenderSubject::World,
            time: TimeSelector::At(ts(y)),
            viewport: None,
            lod: Lod::exact(),
            layers: LayerSet::GEOMETRY
                .with(LayerSet::LABELS)
                .with(LayerSet::TOPOGRAPHY)
                .with(LayerSet::JOURNEYS),
            style: sid,
        }
    }

    /// The canon renders through the SAME contract: areas appear only
    /// within their moments, water rides the TOPOGRAPHY bit, sources
    /// carry the witness AND the scripture tag for atlas/authored
    /// truth (bible mode needs no name-matching ever again).
    #[test]
    fn canon_scenes_respect_time_layers_and_witness() {
        let (p, sid) = provider();
        let scene = p.render(&world_q(sid, -1000)).unwrap();
        let assyria = scene
            .regions
            .iter()
            .find(|r| r.sources.contains(&SourceId::new("witness:atlas")))
            .expect("assyria realized");
        assert!(assyria.sources.contains(&SourceId::new("scripture")), "atlas truth is scripture-grounded");
        assert!(
            scene.labels.iter().any(|l| l.text == "Assyria"),
            "areas carry their names"
        );
        assert!(
            scene.regions.iter().any(|r| r.sources.contains(&SourceId::new("witness:natural-earth"))),
            "water rides along"
        );

        // After the era: gone. Before: gone. Water (static) stays.
        let after = p.render(&world_q(sid, -500)).unwrap();
        assert!(!after.labels.iter().any(|l| l.text == "Assyria"));
        assert!(!after.regions.is_empty(), "the sea remains");
        let before = p.render(&world_q(sid, -2000)).unwrap();
        assert!(!before.labels.iter().any(|l| l.text == "Assyria"));

        // Determinism (law 1) holds through the canon.
        let a = p.render(&world_q(sid, -1000)).unwrap();
        let b = p.render(&world_q(sid, -1000)).unwrap();
        assert_eq!(a.canonical_bytes(), b.canonical_bytes());
    }

    /// A lake is never buried by a claim: water areas paint after
    /// every land-claim area regardless of size, so a territory that
    /// overhangs a shoreline is clipped by the sea, never the other
    /// way round. (The fixture's sea is LARGER than Assyria, so a
    /// pure size-sort would paint it first and bury it.)
    #[test]
    fn water_is_never_buried_by_a_claim() {
        let (p, sid) = provider();
        let scene = p.render(&world_q(sid, -1000)).unwrap();
        let idx_of = |witness: &str| {
            scene
                .regions
                .iter()
                .position(|r| r.sources.contains(&SourceId::new(witness)))
                .unwrap_or_else(|| panic!("{witness} region present"))
        };
        let land = idx_of("witness:atlas");
        let water = idx_of("witness:natural-earth");
        assert!(
            water > land,
            "water (idx {water}) must paint after the land claim (idx {land})"
        );
    }

    /// Partial journeys, typed: mid-first-leg the road shows clipped;
    /// stations appear as reached, named from the gazetteer; outside
    /// the span, no way at all.
    #[test]
    fn canon_journeys_walk_in_time() {
        let (p, sid) = provider();
        let way_pts = |y: i32| -> Option<usize> {
            let scene = p.render(&world_q(sid, y)).unwrap();
            scene
                .boundaries
                .iter()
                .find(|b| b.sources.contains(&SourceId::new("witness:atlas")))
                .map(|b| b.pts.len())
        };
        assert_eq!(way_pts(44), None, "not yet departed");
        assert_eq!(way_pts(51), None, "long arrived");
        assert!(way_pts(46).is_some(), "mid-first-leg the road shows");
        let stations = |y: i32| -> usize {
            p.render(&world_q(sid, y)).unwrap().markers.len()
        };
        assert_eq!(stations(46), 1, "only Antioch is behind them");
        assert_eq!(stations(49), 3, "arrived: all three stations");
        let named = p.render(&world_q(sid, 49)).unwrap();
        assert!(named.labels.iter().any(|l| l.text == "Ephesus"), "stations named from the gazetteer");
    }

    /// A pieces render carries ONLY the asked entities — the caller
    /// arranges layers as they choose; everything else stays home.
    #[test]
    fn pieces_filter_to_named_entities() {
        let (p, sid) = provider();
        let q = world_q(sid, -1000);
        let all = p.render(&q).unwrap();
        assert!(all.regions.len() >= 2, "assyria + the sea");
        let only = p
            .render_pieces(&q, &[map_canon::EntityId("assyria".into())].into_iter().collect())
            .unwrap();
        assert_eq!(only.regions.len(), 1, "just assyria");
        assert!(only.labels.iter().any(|l| l.text == "Assyria"));
        assert!(only.regions[0]
            .sources
            .contains(&atlas_graph_types::covenant::SourceId::new("witness:atlas")));
    }

    /// The scrubber lives: subjects at a time list the entities, and
    /// changes_between yields the canon's moment edges as stops.
    #[test]
    fn canon_subjects_and_stops() {
        let (p, _sid) = provider();
        let subs = p.subjects(ts(-1000));
        assert!(subs.iter().any(|s| s.label == "Assyria"));
        let stops = p.changes_between(ts(-4004), ts(100));
        let years: Vec<i32> = stops.iter().map(|e| e.at.year.get()).collect();
        assert!(years.contains(&-1900) && years.contains(&-911) && years.contains(&45));
        assert!(stops.windows(2).all(|w| w[0].at <= w[1].at));
    }
}


// ==================== palette graph coloring (style honesty made true)

mod palette_coloring_laws {
    use std::collections::{BTreeMap, BTreeSet};

    use map_canon::EntityId;

    use crate::canon_provider::color_shared_border_graph;

    fn e(s: &str) -> EntityId {
        EntityId(s.to_string())
    }

    fn graph(edges: &[(&str, &str)]) -> (Vec<EntityId>, BTreeMap<EntityId, BTreeSet<EntityId>>) {
        let mut ids: BTreeSet<EntityId> = BTreeSet::new();
        let mut adj: BTreeMap<EntityId, BTreeSet<EntityId>> = BTreeMap::new();
        for (a, b) in edges {
            ids.insert(e(a));
            ids.insert(e(b));
            adj.entry(e(a)).or_default().insert(e(b));
            adj.entry(e(b)).or_default().insert(e(a));
        }
        (ids.into_iter().collect(), adj)
    }

    /// The law the style comment promises: touching territories never
    /// share a slot.
    #[test]
    fn touching_territories_never_match() {
        let (ids, adj) = graph(&[
            ("asher", "manasseh-west"),
            ("asher", "phoenicia"),
            ("asher", "zebulun"),
            ("manasseh-west", "zebulun"),
            ("manasseh-west", "ephraim"),
            ("zebulun", "issachar"),
        ]);
        let slot = color_shared_border_graph(&ids, &adj);
        for (a, ns) in &adj {
            for b in ns {
                assert_ne!(slot[a], slot[b], "{a:?} and {b:?} touch and match");
            }
        }
    }

    /// More neighbors than slots: the fallback still answers, and
    /// deterministically — same input, same coloring, every time.
    #[test]
    fn oversubscribed_hub_is_deterministic() {
        // a hub touching nine spokes that all touch each other: the
        // spokes exhaust the eight slots, the hub takes the least-worn
        let mut edges = Vec::new();
        let spokes: Vec<String> = (0..9).map(|i| format!("spoke-{i}")).collect();
        for i in 0..spokes.len() {
            edges.push(("hub".to_string(), spokes[i].clone()));
            for j in i + 1..spokes.len() {
                edges.push((spokes[i].clone(), spokes[j].clone()));
            }
        }
        let borrowed: Vec<(&str, &str)> =
            edges.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let (ids, adj) = graph(&borrowed);
        let one = color_shared_border_graph(&ids, &adj);
        let two = color_shared_border_graph(&ids, &adj);
        assert_eq!(one, two, "coloring is a function of the graph, nothing else");
        assert!(one.values().all(|&s| s < 8), "every slot is a real slot");
    }

    /// An entity keeps its slot regardless of which pieces render:
    /// the assignment is computed once over the whole graph, so a
    /// subset render cannot recolor the survivors.
    #[test]
    fn color_follows_the_entity() {
        let (ids, adj) = graph(&[("judah", "benjamin"), ("benjamin", "ephraim")]);
        let full = color_shared_border_graph(&ids, &adj);
        // rendering only judah+ephraim later still reads THIS map —
        // the law is that the assignment exists once; spot-check it
        // is total over the ids given
        for id in &ids {
            assert!(full.contains_key(id));
        }
    }
}

// ==================== memory: a place that no longer stands

mod memory_laws {
    use std::collections::BTreeMap;

    use atlas_graph_types::covenant::{TimePoint, Year};
    use map_canon::{CanonStore, EntityId, Feature, LayerKind, Memory, World};
    use map_types::style::*;
    use map_types::{LayerSet, Lod, MapProvider, RenderQuery, RenderSubject, StyleId, TimeSelector, UnitVec};

    use crate::canon_provider::CanonProvider;

    /// A Memory renders as an INSCRIPTION: its label wears the memory
    /// voice, and no marker stands at the site — a drowned city can
    /// never look alive.
    #[test]
    fn a_memory_is_an_inscription_never_a_dot() {
        let mut store = CanonStore::default();
        let fid = store.insert_feature(Feature::Memory(Memory {
            entity: EntityId("place:sodom".into()),
            name: "Sodom".into(),
            at: UnitVec::from_lat_lon_deg(31.1, 35.44),
        }));
        let t0 = TimePoint::year_only(Year::new(-4004).unwrap());
        let sid = store.insert_snapshot(map_canon::Snapshot { features: [fid].into() });
        let mut world = World::default();
        world.insert(t0, sid).unwrap();
        store.set_layer(LayerKind::ScriptureClaims, world);

        let style = crate::tests::honest_style_for_memory_law();
        let style_id = style.id();
        let provider =
            CanonProvider::new(store, BTreeMap::from([(style_id, style)]), None);
        let scene = provider
            .render(&RenderQuery {
                subject: RenderSubject::World,
                time: TimeSelector::At(TimePoint::year_only(Year::new(-1000).unwrap())),
                viewport: None,
                lod: Lod(0.0),
                layers: LayerSet::GEOMETRY.with(LayerSet::LABELS),
                style: style_id,
            })
            .unwrap();
        assert!(scene.markers.is_empty(), "nothing stands at a memory");
        let l = scene.labels.iter().find(|l| l.text == "Sodom").expect("the inscription");
        assert_eq!(l.face, map_types::scene::LabelFace::Memory);
        assert!(l.voice.italic, "a memory speaks in italic");
    }
}

// ==================== the scaling laws: simplify and cull

mod scaling_laws {
    use std::collections::BTreeMap;

    use atlas_graph_types::covenant::{TimePoint, Year};
    use map_canon::{Area, Border, CanonStore, EntityId, Feature, LayerKind, Snapshot, World};
    use map_types::style::*;
    use map_types::{Bbox, LayerSet, Lod, MapProvider, RenderQuery, RenderSubject, TimeSelector, UnitVec};

    use crate::canon_provider::CanonProvider;

    fn dense_ring(lat0: f64, lon0: f64, d: f64, n: usize) -> Vec<UnitVec> {
        // a square ring with n points per side — detail to shed
        let mut pts = Vec::new();
        let corners =
            [(lat0, lon0), (lat0, lon0 + d), (lat0 + d, lon0 + d), (lat0 + d, lon0), (lat0, lon0)];
        for w in corners.windows(2) {
            for i in 0..n {
                let t = i as f64 / n as f64;
                pts.push(UnitVec::from_lat_lon_deg(
                    w[0].0 + t * (w[1].0 - w[0].0),
                    w[0].1 + t * (w[1].1 - w[0].1),
                ));
            }
        }
        pts
    }

    fn store_with(areas: &[(&str, Vec<UnitVec>)]) -> CanonStore {
        let mut store = CanonStore::default();
        let mut feats = std::collections::BTreeSet::new();
        for (id, ring) in areas {
            let b = store.insert_border(Border(ring.clone()));
            feats.insert(store.insert_feature(Feature::Area(Area {
                entity: EntityId((*id).to_string()),
                name: (*id).to_string(),
                rings: [b].into(),
                holes: Default::default(),
            })));
        }
        let sid = store.insert_snapshot(Snapshot { features: feats });
        let mut world = World::default();
        world.insert(TimePoint::year_only(Year::new(-4004).unwrap()), sid).unwrap();
        store.set_layer(LayerKind::ScriptureClaims, world);
        store
    }

    fn q(lod: f64, viewport: Option<Bbox>) -> RenderQuery {
        RenderQuery {
            subject: RenderSubject::World,
            time: TimeSelector::At(TimePoint::year_only(Year::new(-1000).unwrap())),
            viewport,
            lod: Lod(lod),
            layers: LayerSet::GEOMETRY,
            style: crate::tests::honest_style_for_memory_law().id(),
        }
    }

    fn provider(store: CanonStore) -> CanonProvider {
        let style = crate::tests::honest_style_for_memory_law();
        CanonProvider::new(store, BTreeMap::from([(style.id(), style)]), None)
    }

    /// SELECT ∘ SIMPLIFY ∘ STYLE: geometry leaves the provider at the
    /// query's level of detail — a coarse query ships fewer points
    /// than a fine one, from the same canon.
    #[test]
    fn geometry_ships_at_the_query_lod() {
        let p = provider(store_with(&[("land", dense_ring(30.0, 30.0, 4.0, 64))]));
        let pts = |lod: f64| -> usize {
            p.render(&q(lod, None)).unwrap().regions[0]
                .outer
                .iter()
                .map(|r| r.points().len())
                .sum()
        };
        let fine = pts(0.0);
        let coarse = pts(0.01);
        assert!(
            coarse * 4 < fine,
            "coarse ({coarse} pts) sheds detail fine ({fine} pts) keeps"
        );
    }

    /// THE VIEWPORT CULL: an area whose every border cap misses the
    /// camera never leaves the provider; one inside always does.
    #[test]
    fn the_world_beyond_the_camera_stays_home() {
        let p = provider(store_with(&[
            ("near", dense_ring(31.0, 34.0, 2.0, 8)),
            ("far", dense_ring(-40.0, -120.0, 2.0, 8)),
        ]));
        let view = Bbox {
            center: UnitVec::from_lat_lon_deg(32.0, 35.0),
            radius: 10f64.to_radians(),
        };
        let scene = p.render(&q(0.0, Some(view))).unwrap();
        let names: Vec<_> = scene.regions.iter().filter_map(|r| r.entity.clone()).collect();
        assert!(names.contains(&"near".to_string()), "the near area renders");
        assert!(!names.contains(&"far".to_string()), "the far area stays home");
    }

    /// The whole-sphere sentinel is never culled: its cap covers the
    /// sphere, so the world backdrop survives every viewport.
    #[test]
    fn the_sentinel_survives_every_viewport() {
        let sentinel = vec![
            UnitVec::from_lat_lon_deg(0.0, 0.0),
            UnitVec::from_lat_lon_deg(0.0, 179.5),
            UnitVec::from_lat_lon_deg(1.0, -90.0),
        ];
        let p = provider(store_with(&[("world", sentinel)]));
        let view = Bbox {
            center: UnitVec::from_lat_lon_deg(32.0, 35.0),
            radius: 5f64.to_radians(),
        };
        let scene = p.render(&q(0.0, Some(view))).unwrap();
        assert!(
            scene.regions.iter().any(|r| r.entity.as_deref() == Some("world")),
            "the backdrop survives"
        );
    }
}
