//! Deterministic persistence for the canon: BTree iteration order in,
//! byte-stable JSON out; loading RECOMPUTES every content id and
//! refuses a file whose declared ids do not match their content —
//! a corrupted or hand-edited canon fails loud.

use std::collections::{BTreeMap, BTreeSet};

use atlas_graph_types::covenant::{PlaceId, TimePoint, Year};
use map_types::UnitVec;
use serde_json::{json, Map, Value};

use crate::*;

fn ts_json(t: &Timestamp) -> Value {
    let mut m = Map::new();
    m.insert("y".into(), json!(t.year.get()));
    if let Some(month) = t.month {
        m.insert("m".into(), json!(month));
    }
    if let Some(day) = t.day {
        m.insert("d".into(), json!(day));
    }
    Value::Object(m)
}

fn ts_from(v: &Value) -> Result<Timestamp, String> {
    let y = v.get("y").and_then(Value::as_i64).ok_or("timestamp: missing y")? as i32;
    let m = v.get("m").and_then(Value::as_u64).map(|n| n as u8);
    let d = v.get("d").and_then(Value::as_u64).map(|n| n as u8);
    TimePoint::new(Year::new(y).map_err(|_| format!("timestamp: bad year {y}"))?, m, d)
        .map_err(|e| format!("timestamp: {e:?}"))
}

fn hex(h: ContentHash) -> String {
    format!("{:016x}", h.0)
}

fn unhex(s: &str) -> Result<ContentHash, String> {
    u64::from_str_radix(s, 16).map(ContentHash).map_err(|e| format!("bad id '{s}': {e}"))
}

fn layer_name(k: &LayerKind) -> &'static str {
    match k {
        LayerKind::Territory => "territory",
        LayerKind::ScriptureClaims => "scripture-claims",
        LayerKind::Journeys => "journeys",
        LayerKind::Water => "water",
        LayerKind::Relief => "relief",
        LayerKind::Background => "background",
    }
}

fn layer_from(s: &str) -> Result<LayerKind, String> {
    Ok(match s {
        "territory" => LayerKind::Territory,
        "scripture-claims" => LayerKind::ScriptureClaims,
        "journeys" => LayerKind::Journeys,
        "water" => LayerKind::Water,
        "relief" => LayerKind::Relief,
        "background" => LayerKind::Background,
        other => return Err(format!("unknown layer '{other}'")),
    })
}

fn witness_name(w: &Witness) -> &'static str {
    match w {
        Witness::Atlas => "atlas",
        Witness::Authored => "authored",
        Witness::Basemap => "basemap",
        Witness::NaturalEarth => "natural-earth",
    }
}

fn witness_from(s: &str) -> Result<Witness, String> {
    Ok(match s {
        "atlas" => Witness::Atlas,
        "authored" => Witness::Authored,
        "basemap" => Witness::Basemap,
        "natural-earth" => Witness::NaturalEarth,
        other => return Err(format!("unknown witness '{other}'")),
    })
}

pub fn to_bytes(store: &CanonStore) -> Result<Vec<u8>, String> {
    let mut root = Map::new();

    let mut borders = Map::new();
    for (id, b) in store.borders() {
        let pts: Vec<Value> =
            b.0.iter().map(|p| json!([p.x(), p.y(), p.z()])).collect();
        borders.insert(hex(id.0), Value::Array(pts));
    }
    root.insert("borders".into(), Value::Object(borders));

    let mut features = Map::new();
    for (id, f) in store.features() {
        let v = match f {
            Feature::Area(a) => json!({
                "kind": "area",
                "entity": a.entity.0,
                "name": a.name,
                "rings": a.rings.iter().map(|r| hex(r.0)).collect::<Vec<_>>(),
                "holes": a.holes.iter().map(|r| hex(r.0)).collect::<Vec<_>>(),
            }),
            Feature::Way(r) => json!({
                "kind": "way",
                "entity": r.entity.0,
                "name": r.name,
                "legs": r.legs.iter().map(|l| json!({
                    "from": l.from.0,
                    "to": l.to.0,
                    "border": hex(l.border.0),
                    "span": [ts_json(&l.span.0), ts_json(&l.span.1)],
                })).collect::<Vec<_>>(),
            }),
            Feature::Point(p) => json!({
                "kind": "point",
                "entity": p.entity.0,
                "name": p.name,
                "at": [p.at.x(), p.at.y(), p.at.z()],
            }),
            Feature::Line(l) => json!({
                "kind": "line",
                "entity": l.entity.0,
                "name": l.name,
                "border": hex(l.border.0),
            }),
            Feature::Memory(m) => json!({
                "kind": "memory",
                "entity": m.entity.0,
                "name": m.name,
                "at": [m.at.x(), m.at.y(), m.at.z()],
            }),
        };
        features.insert(hex(id.0), v);
    }
    root.insert("features".into(), Value::Object(features));

    let mut snapshots = Map::new();
    for (id, s) in store.snapshots() {
        snapshots.insert(
            hex(id.0),
            Value::Array(s.features.iter().map(|f| json!(hex(f.0))).collect()),
        );
    }
    root.insert("snapshots".into(), Value::Object(snapshots));

    let mut layers = Map::new();
    for (kind, world) in store.layers() {
        layers.insert(
            layer_name(kind).to_string(),
            Value::Array(
                world
                    .moments()
                    .iter()
                    .map(|(t, sid)| json!([ts_json(t), hex(sid.0)]))
                    .collect(),
            ),
        );
    }
    root.insert("layers".into(), Value::Object(layers));

    let mut prov = Map::new();
    for (fid, p) in store.provenance() {
        prov.insert(
            hex(fid.0),
            json!({
                "witness": witness_name(&p.witness),
                "verses": p.verses,
                "note": p.note,
            }),
        );
    }
    root.insert("provenance".into(), Value::Object(prov));

    serde_json::to_vec_pretty(&Value::Object(root)).map_err(|e| e.to_string())
}

pub fn from_bytes(bytes: &[u8]) -> Result<CanonStore, String> {
    let v: Value = serde_json::from_slice(bytes).map_err(|e| format!("canon: bad json: {e}"))?;
    let obj = |name: &str| -> Result<&Map<String, Value>, String> {
        v.get(name)
            .and_then(Value::as_object)
            .ok_or_else(|| format!("canon: missing '{name}'"))
    };

    let mut store = CanonStore::default();

    for (declared, pts) in obj("borders")? {
        let pts = pts.as_array().ok_or("border: not an array")?;
        let mut out = Vec::with_capacity(pts.len());
        for p in pts {
            let a = p.as_array().filter(|a| a.len() == 3).ok_or("border point: not [x,y,z]")?;
            let (x, y, z) = (
                a[0].as_f64().ok_or("x")?,
                a[1].as_f64().ok_or("y")?,
                a[2].as_f64().ok_or("z")?,
            );
            out.push(UnitVec::new(x, y, z).map_err(|_| "border point: not a direction")?);
        }
        let id = store.insert_border(Border(out));
        if id.0 != unhex(declared)? {
            return Err(format!("border {declared}: content does not match its id"));
        }
    }

    for (declared, f) in obj("features")? {
        let kind = f.get("kind").and_then(Value::as_str).ok_or("feature: missing kind")?;
        let entity = EntityId(
            f.get("entity").and_then(Value::as_str).ok_or("feature: missing entity")?.to_string(),
        );
        let name =
            f.get("name").and_then(Value::as_str).ok_or("feature: missing name")?.to_string();
        let ids = |key: &str| -> Result<BTreeSet<BorderId>, String> {
            f.get(key)
                .and_then(Value::as_array)
                .ok_or_else(|| format!("feature: missing {key}"))?
                .iter()
                .map(|s| {
                    s.as_str()
                        .ok_or_else(|| format!("{key}: not a string"))
                        .and_then(unhex)
                        .map(BorderId)
                })
                .collect()
        };
        let feature = match kind {
            "area" => Feature::Area(Area { entity, name, rings: ids("rings")?, holes: ids("holes")? }),
            "way" => {
                let legs = f
                    .get("legs")
                    .and_then(Value::as_array)
                    .ok_or("way: missing legs")?
                    .iter()
                    .map(|l| -> Result<Leg, String> {
                        let span =
                            l.get("span").and_then(Value::as_array).ok_or("leg: missing span")?;
                        if span.len() != 2 {
                            return Err("leg: span is not a pair".to_string());
                        }
                        Ok(Leg {
                            from: PlaceId::new(
                                l.get("from").and_then(Value::as_str).ok_or("leg: from")?.to_string(),
                            ),
                            to: PlaceId::new(
                                l.get("to").and_then(Value::as_str).ok_or("leg: to")?.to_string(),
                            ),
                            border: BorderId(unhex(
                                l.get("border").and_then(Value::as_str).ok_or("leg: border")?,
                            )?),
                            span: (ts_from(&span[0])?, ts_from(&span[1])?),
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Feature::Way(Route { entity, name, legs })
            }
            "line" => Feature::Line(PathLine {
                entity,
                name,
                border: BorderId(unhex(
                    f.get("border").and_then(Value::as_str).ok_or("line: border")?,
                )?),
            }),
            "memory" => {
                let a = f
                    .get("at")
                    .and_then(Value::as_array)
                    .filter(|a| a.len() == 3)
                    .ok_or("memory: missing at")?;
                Feature::Memory(crate::Memory {
                    entity,
                    name,
                    at: UnitVec::new(
                        a[0].as_f64().ok_or("x")?,
                        a[1].as_f64().ok_or("y")?,
                        a[2].as_f64().ok_or("z")?,
                    )
                    .map_err(|_| "memory: not a direction")?,
                })
            }
            "point" => {
                let a = f
                    .get("at")
                    .and_then(Value::as_array)
                    .filter(|a| a.len() == 3)
                    .ok_or("point: missing at")?;
                Feature::Point(Landmark {
                    entity,
                    name,
                    at: UnitVec::new(
                        a[0].as_f64().ok_or("x")?,
                        a[1].as_f64().ok_or("y")?,
                        a[2].as_f64().ok_or("z")?,
                    )
                    .map_err(|_| "point: not a direction")?,
                })
            }
            other => return Err(format!("feature: unknown kind '{other}'")),
        };
        let id = store.insert_feature(feature);
        if id.0 != unhex(declared)? {
            return Err(format!("feature {declared}: content does not match its id"));
        }
    }

    for (declared, fids) in obj("snapshots")? {
        let features: BTreeSet<FeatureId> = fids
            .as_array()
            .ok_or("snapshot: not an array")?
            .iter()
            .map(|s| s.as_str().ok_or("snapshot id".to_string()).and_then(|s| unhex(s)).map(FeatureId))
            .collect::<Result<_, _>>()?;
        let id = store.insert_snapshot(Snapshot { features });
        if id.0 != unhex(declared)? {
            return Err(format!("snapshot {declared}: content does not match its id"));
        }
    }

    for (name, moments) in obj("layers")? {
        let mut world = World::default();
        for m in moments.as_array().ok_or("layer: not an array")? {
            let pair = m.as_array().filter(|a| a.len() == 2).ok_or("moment: not a pair")?;
            let t = ts_from(&pair[0])?;
            let sid = SnapshotId(unhex(pair[1].as_str().ok_or("moment: snapshot id")?)?);
            world
                .insert(t, sid)
                .map_err(|_| format!("layer {name}: two snapshots at one instant"))?;
        }
        store.set_layer(layer_from(name)?, world);
    }

    for (fid, p) in obj("provenance")? {
        let witness =
            witness_from(p.get("witness").and_then(Value::as_str).ok_or("provenance: witness")?)?;
        let verses = p
            .get("verses")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
            .unwrap_or_default();
        let note =
            p.get("note").and_then(Value::as_str).unwrap_or_default().to_string();
        store.set_provenance(FeatureId(unhex(fid)?), Provenance { witness, verses, note });
    }

    Ok(store)
}

// Silence unused-import lint gymnastics: BTreeMap is used via CanonStore internals.
#[allow(unused)]
fn _typecheck(_: &BTreeMap<(), ()>) {}
