//! Witnesses become the canon. All intelligence lives here — the
//! renderer downstream is dumb. Every translation is total or fails
//! loud with the offending row NAMED.

use std::collections::{BTreeMap, BTreeSet};

use atlas_graph_types::covenant::{PlaceId, TimePoint, Year};
use map_canon::{
    Area, Border, CanonStore, EntityId, Feature, LayerKind, Leg, Provenance, Route, Snapshot,
    Timestamp, Witness, World,
};
use map_types::UnitVec;

use crate::vendor::{EventRow, NarrativeRow, PolityRow};

fn ts(y: i32) -> Result<Timestamp, String> {
    Year::new(y).map(TimePoint::year_only).map_err(|_| format!("no such year {y}"))
}

/// The year after y, minding the missing year zero.
fn year_after(y: i32) -> i32 {
    if y == -1 {
        1
    } else {
        y + 1
    }
}

fn uv(lat: f64, lon: f64) -> UnitVec {
    UnitVec::from_lat_lon_deg(lat, lon)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CompileReport {
    pub polity_eras: usize,
    pub routes: usize,
}

/// Atlas polity eras → the Territory layer. The world changes exactly
/// at era boundaries (each era's beginning, and the year after each
/// era's end); at every moment the snapshot holds the areas whose era
/// contains that moment.
pub fn compile_polities(store: &mut CanonStore, rows: &[PolityRow]) -> Result<CompileReport, String> {
    let mut features: Vec<(i32, i32, map_canon::FeatureId)> = Vec::new();
    for row in rows {
        if row.to_year < row.from_year {
            return Err(format!("polity '{}': era runs backward", row.id));
        }
        let mut rings = BTreeSet::new();
        for ring in &row.rings {
            if ring.len() < 3 {
                return Err(format!("polity '{}': a ring with {} points", row.id, ring.len()));
            }
            let pts: Vec<UnitVec> = ring.iter().map(|(lat, lon)| uv(*lat, *lon)).collect();
            rings.insert(store.insert_border(Border(pts)));
        }
        let fid = store.insert_feature(Feature::Area(Area {
            entity: EntityId(row.id.clone()),
            name: row.name.clone(),
            rings,
            holes: BTreeSet::new(),
        }));
        let mut verses = row.transition_verses.clone();
        verses.extend(row.fall_verses.iter().cloned());
        store.set_provenance(
            fid,
            Provenance {
                witness: Witness::Atlas,
                verses,
                note: format!("atlas polity era {}..{}", row.from_year, row.to_year),
            },
        );
        features.push((row.from_year, row.to_year, fid));
    }

    let mut edges: BTreeSet<i32> = BTreeSet::new();
    for (from, to, _) in &features {
        edges.insert(*from);
        edges.insert(year_after(*to));
    }
    let mut world = World::default();
    for edge in edges {
        let active: BTreeSet<_> = features
            .iter()
            .filter(|(from, to, _)| *from <= edge && edge <= *to)
            .map(|(_, _, fid)| *fid)
            .collect();
        let sid = store.insert_snapshot(Snapshot { features: active });
        world
            .insert(ts(edge)?, sid)
            .map_err(|_| format!("territory: contradiction at {edge}"))?;
    }
    store.set_layer(LayerKind::Territory, world);
    Ok(CompileReport { polity_eras: rows.len(), ..Default::default() })
}

/// Atlas narratives + their dated leg events → the Journeys layer.
/// Stations are events at gazetteer places; a leg spans from the end
/// of one event to the start of the next. A place the gazetteer cannot
/// resolve is a NAMED error, never a silently skipped station.
pub fn compile_narratives(
    store: &mut CanonStore,
    narratives: &[NarrativeRow],
    events: &[EventRow],
    places: &BTreeMap<String, (f64, f64)>,
) -> Result<CompileReport, String> {
    let by_id: BTreeMap<&str, &EventRow> = events.iter().map(|e| (e.id.as_str(), e)).collect();
    let mut spans: Vec<(i32, i32, map_canon::FeatureId)> = Vec::new();
    let mut routes = 0usize;

    for n in narratives {
        // Resolve stations: event → (place id, position, when).
        struct Station<'a> {
            place: &'a str,
            pos: UnitVec,
            when: Option<(i32, i32)>,
            verses: &'a [String],
        }
        let mut stations = Vec::new();
        for leg in &n.legs {
            let ev = by_id
                .get(leg.as_str())
                .ok_or_else(|| format!("narrative '{}': unknown event '{leg}'", n.id))?;
            let place = ev
                .places
                .first()
                .ok_or_else(|| format!("narrative '{}': event '{}' names no place", n.id, ev.id))?;
            let (lat, lon) = places.get(place).ok_or_else(|| {
                format!("narrative '{}': place '{place}' not in the gazetteer", n.id)
            })?;
            stations.push(Station {
                place,
                pos: uv(*lat, *lon),
                when: ev.when,
                verses: &ev.verses,
            });
        }
        if stations.len() < 2 {
            continue; // a single dated stop is not a walk
        }
        // Fill missing whens from neighbors, forward then backward.
        let mut whens: Vec<Option<(i32, i32)>> = stations.iter().map(|s| s.when).collect();
        for i in 1..whens.len() {
            if whens[i].is_none() {
                whens[i] = whens[i - 1];
            }
        }
        for i in (0..whens.len().saturating_sub(1)).rev() {
            if whens[i].is_none() {
                whens[i] = whens[i + 1];
            }
        }
        let whens: Vec<(i32, i32)> = whens
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| format!("narrative '{}': no event carries a date", n.id))?;

        let mut legs = Vec::new();
        let mut verses: Vec<String> = Vec::new();
        for i in 0..stations.len() - 1 {
            let border = store
                .insert_border(Border(vec![stations[i].pos, stations[i + 1].pos]));
            // Depart when this event ends, arrive when the next begins;
            // contiguous events walk within the shared year.
            let (depart, arrive) = (whens[i].1, whens[i + 1].0);
            let (a, b) = if depart <= arrive { (depart, arrive) } else { (arrive, depart) };
            legs.push(Leg {
                from: PlaceId::new(stations[i].place.to_string()),
                to: PlaceId::new(stations[i + 1].place.to_string()),
                border,
                span: (ts(a)?, ts(b)?),
            });
            verses.extend(stations[i].verses.iter().cloned());
        }
        verses.extend(stations.last().unwrap().verses.iter().cloned());

        let start = legs.first().map(|l| l.span.0.year.get()).unwrap();
        let end = legs.last().map(|l| l.span.1.year.get()).unwrap();
        let fid = store.insert_feature(Feature::Way(Route {
            entity: EntityId(n.id.clone()),
            name: n.name.clone(),
            legs,
        }));
        store.set_provenance(
            fid,
            Provenance {
                witness: Witness::Atlas,
                verses,
                note: format!("atlas narrative, walked {start}..{end}"),
            },
        );
        spans.push((start, end, fid));
        routes += 1;
    }

    let mut edges: BTreeSet<i32> = BTreeSet::new();
    for (start, end, _) in &spans {
        edges.insert(*start);
        edges.insert(year_after(*end));
    }
    let mut world = World::default();
    for edge in edges {
        let active: BTreeSet<_> = spans
            .iter()
            .filter(|(start, end, _)| *start <= edge && edge <= *end)
            .map(|(_, _, fid)| *fid)
            .collect();
        let sid = store.insert_snapshot(Snapshot { features: active });
        world
            .insert(ts(edge)?, sid)
            .map_err(|_| format!("journeys: contradiction at {edge}"))?;
    }
    store.set_layer(LayerKind::Journeys, world);
    Ok(CompileReport { routes, ..Default::default() })
}

/// Merge additional way spans into the Journeys layer, preserving the
/// moments already there — the union re-sweep keeps one state per
/// instant. (Used for reconciliation-kept authored routes after the
/// atlas narratives have set the layer.)
pub fn append_ways(
    store: &mut CanonStore,
    spans: &[(i32, i32, map_canon::FeatureId)],
) -> Result<(), String> {
    let existing = store.layers().get(&LayerKind::Journeys).cloned().unwrap_or_default();
    let mut existing_rows: BTreeMap<Timestamp, BTreeSet<map_canon::FeatureId>> = BTreeMap::new();
    for (t, sid) in existing.moments() {
        existing_rows.insert(*t, store.snapshots()[sid].features.clone());
    }
    let mut edges: BTreeSet<Timestamp> = existing.moments().keys().copied().collect();
    for (start, end, _) in spans {
        edges.insert(ts(*start)?);
        edges.insert(ts(year_after(*end))?);
    }
    let mut world = World::default();
    for edge in edges {
        let mut active: BTreeSet<map_canon::FeatureId> = existing_rows
            .range(..=edge)
            .next_back()
            .map(|(_, f)| f.clone())
            .unwrap_or_default();
        let y = edge.year.get();
        for (start, end, fid) in spans {
            if *start <= y && y <= *end {
                active.insert(*fid);
            }
        }
        let sid = store.insert_snapshot(Snapshot { features: active });
        world.insert(edge, sid).map_err(|_| "journeys: contradiction on append".to_string())?;
    }
    store.set_layer(LayerKind::Journeys, world);
    Ok(())
}
