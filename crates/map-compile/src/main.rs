use map_compile::vendor::*;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("refresh") => {}
        Some("build") => return build(&args),
        _ => {
            eprintln!("usage: map-compile refresh [--base host:port] [--out dir]");
            eprintln!("       map-compile build [--vendor dir] [--out dir]");
            std::process::exit(2);
        }
    }
    let opt = |flag: &str, default: &str| -> String {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
            .unwrap_or_else(|| default.to_string())
    };
    let base = opt("--base", "127.0.0.1:8080");
    let out = opt("--out", "data/atlas-vendor");
    let (host, port) = base.split_once(':').expect("base is host:port");
    let port: u16 = port.parse().expect("port");

    let get = |path: &str| -> Vec<u8> {
        eprintln!("GET {path}");
        http_get(host, port, path).unwrap_or_else(|e| {
            eprintln!("refresh FAILED: {e}");
            std::process::exit(1);
        })
    };

    let mut payloads: Vec<(String, Vec<u8>)> = Vec::new();

    let polities = get("/api/polities?from=-4004&to=2000");
    let polity_rows =
        parse_polities(&String::from_utf8_lossy(&polities)).unwrap_or_else(|e| die(&e));
    eprintln!("  {} polity eras", polity_rows.len());
    payloads.push(("polities.json".to_string(), polities));

    let narratives = get("/api/narratives");
    let narrative_rows =
        parse_narratives(&String::from_utf8_lossy(&narratives)).unwrap_or_else(|e| die(&e));
    eprintln!("  {} narratives", narrative_rows.len());
    payloads.push(("narratives.json".to_string(), narratives));

    let mut seen = std::collections::BTreeSet::new();
    let mut events = Vec::new();
    for n in &narrative_rows {
        for leg in &n.legs {
            if !seen.insert(leg.clone()) {
                continue;
            }
            let body = get(&format!("/api/event/{leg}"));
            let row = parse_event(&String::from_utf8_lossy(&body)).unwrap_or_else(|e| die(&e));
            events.push(serde_json::json!({
                "id": row.id,
                "label": row.label,
                "when": row.when.map(|(f, t)| serde_json::json!({"from_year": f, "to_year": t})),
                "places": row.places,
                "verses": row.verses,
            }));
        }
    }
    eprintln!("  {} leg events", events.len());
    payloads.push((
        "events.json".to_string(),
        serde_json::to_string_pretty(&serde_json::Value::Array(events))
            .unwrap()
            .into_bytes(),
    ));

    for (name, path) in [
        ("eras.json", "/api/eras"),
        ("landmarks.json", "/api/landmarks"),
        ("land-mask.json", "/api/land-mask"),
    ] {
        let body = get(path);
        let text = String::from_utf8_lossy(&body);
        match name {
            "eras.json" => drop(parse_eras(&text).unwrap_or_else(|e| die(&e))),
            "landmarks.json" => drop(parse_landmarks(&text).unwrap_or_else(|e| die(&e))),
            _ => drop(parse_land_mask(&text).unwrap_or_else(|e| die(&e))),
        }
        payloads.push((name.to_string(), body));
    }

    let pin = write_vendor(std::path::Path::new(&out), &payloads).unwrap_or_else(|e| die(&e));
    eprintln!(
        "vendored {} files to {out}, pin {pin:016x}",
        payloads.len() + 1
    );
}

fn die(e: &str) -> ! {
    eprintln!("map-compile FAILED: {e}");
    std::process::exit(1);
}

fn ts_or_die(y: i32) -> map_canon::Timestamp {
    atlas_graph_types::covenant::Year::new(y)
        .map(atlas_graph_types::covenant::TimePoint::year_only)
        .unwrap_or_else(|_| die(&format!("no such year {y}")))
}

fn build(args: &[String]) {
    use map_canon::{LayerKind, Provenance, Witness};
    use map_compile::compile::*;
    use map_compile::reconcile::*;
    use map_compile::timeline_bridge::*;
    use std::collections::{BTreeMap, BTreeSet};

    let opt = |flag: &str, default: &str| -> String {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
            .unwrap_or_else(|| default.to_string())
    };
    map_compile::exclusion::check_build_inputs(std::path::Path::new("data"))
        .unwrap_or_else(|error| die(&format!("excluded input: {error:?}")));
    let vendor_dir = opt("--vendor", "data/atlas-vendor");
    let out_dir = opt("--out", "data/canon");
    let read = |name: &str| -> String {
        std::fs::read_to_string(std::path::Path::new(&vendor_dir).join(name))
            .unwrap_or_else(|e| die(&format!("read {name}: {e} (run refresh first)")))
    };

    let mut store = map_canon::CanonStore::default();
    let mut report_md = String::from("# Canon compile report\n\n");

    let polities = parse_polities(&read("polities.json")).unwrap_or_else(|e| die(&e));
    let rep = CompileReport {
        polity_eras: polities.len(),
        ..Default::default()
    };
    eprintln!("territory: {} polity eras", rep.polity_eras);
    report_md.push_str(&format!(
        "- Territory: {} atlas polity eras\n",
        rep.polity_eras
    ));

    let narratives = parse_narratives(&read("narratives.json")).unwrap_or_else(|e| die(&e));
    let events = parse_vendored_events(&read("events.json")).unwrap_or_else(|e| die(&e));
    let exp_dir = std::path::Path::new("data/atlas-exports");
    let atlas = map_adapters::load_exports(
        &std::fs::read_to_string(exp_dir.join("gazetteer.json"))
            .unwrap_or_else(|e| die(&format!("gazetteer: {e}"))),
        &std::fs::read_to_string(exp_dir.join("chronology.json"))
            .unwrap_or_else(|e| die(&format!("chronology: {e}"))),
    )
    .unwrap_or_else(|e| die(&format!("atlas exports: {e:?}")));
    let gaz = map_adapters::merged_gazetteer(&atlas);
    let places: BTreeMap<String, (f64, f64)> = gaz
        .places
        .iter()
        .map(|(pid, e)| {
            let lat = e.position.z().asin().to_degrees();
            let lon = e.position.y().atan2(e.position.x()).to_degrees();
            (pid.0.clone(), (lat, lon))
        })
        .collect();
    let rep =
        compile_narratives(&mut store, &narratives, &events, &places).unwrap_or_else(|e| die(&e));
    eprintln!("journeys: {} atlas narratives", rep.routes);
    report_md.push_str(&format!("- Journeys: {} atlas narratives\n", rep.routes));

    let rec = parse_reconcile(
        &std::fs::read_to_string("data/authored/reconcile.json")
            .unwrap_or_else(|e| die(&format!("reconcile.json: {e}"))),
    )
    .unwrap_or_else(|e| die(&e));
    let mut identity = map_compile::identity::load_registry(
        &std::fs::read_to_string("data/authored/registry.json")
            .unwrap_or_else(|e| die(&format!("registry.json: {e}"))),
        "data/authored/registry.json",
    )
    .unwrap_or_else(|e| die(&e));
    let authored = map_adapters::authored_routes();
    let tags: Vec<String> = authored.iter().map(|r| r.tag.to_string()).collect();
    let narrative_ids: Vec<String> = narratives.iter().map(|n| n.id.clone()).collect();
    let verdicts = reconcile_routes(&rec, &tags, &narrative_ids).unwrap_or_else(|e| die(&e));
    report_md.push_str(&format!(
        "- Reconciliation: {} authored routes superseded ({})\n",
        verdicts.dropped.len(),
        verdicts.dropped.join(", ")
    ));
    report_md.push_str(&format!(
        "- Reconciliation: {} authored routes KEPT (no atlas narrative yet): {}\n",
        verdicts.kept.len(),
        verdicts.kept.join(", ")
    ));

    let mut kept_spans = Vec::new();
    for r in authored
        .iter()
        .filter(|r| verdicts.kept.contains(&r.tag.to_string()))
    {
        let end = r.to_year.unwrap_or(r.from_year).max(r.from_year);
        let mut legs = Vec::new();
        for w in r.stations.windows(2) {
            let a = map_types::UnitVec::from_lat_lon_deg(w[0].1, w[0].2);
            let b = map_types::UnitVec::from_lat_lon_deg(w[1].1, w[1].2);
            let border = store.insert_border(map_canon::Border(vec![a, b]));
            let place = |name: &str| {
                atlas_graph_types::covenant::PlaceId::new(format!(
                    "standin:{}",
                    name.replace(' ', "-")
                ))
            };
            legs.push(map_canon::Leg {
                from: place(w[0].0),
                to: place(w[1].0),
                border,
                span: (ts_or_die(r.from_year), ts_or_die(end)),
            });
        }
        let fid = store.insert_feature(map_canon::Feature::Way(map_canon::Route {
            entity: map_canon::EntityId(format!("authored:{}", r.tag.to_lowercase())),
            name: r.display.to_string(),
            legs,
        }));
        store.set_provenance(
            fid,
            Provenance {
                witness: Witness::Authored,
                verses: vec![format!(
                    "book {} ch {}-{}",
                    r.book, r.chapter_from, r.chapter_to
                )],
                note: "authored route kept by reconciliation (no atlas narrative yet)".to_string(),
            },
        );
        kept_spans.push((r.from_year, end, fid));
    }
    append_ways(&mut store, &kept_spans).unwrap_or_else(|e| die(&e));
    eprintln!("journeys: +{} authored routes kept", kept_spans.len());

    let drops: BTreeSet<String> = rec.region_drops.iter().map(|(s, _)| s.clone()).collect();
    let scripture = map_adapters::scripture_timeline_with(Some(&atlas));
    bridge_filtered(
        &mut store,
        &scripture,
        LayerKind::ScriptureClaims,
        Witness::Authored,
        "authored",
        None,
        &drops,
        &mut identity,
    )
    .unwrap_or_else(|e| die(&e));
    report_md.push_str(&format!(
        "- ScriptureClaims: authored surveys bridged; {} kingdom groups superseded by atlas polities\n",
        drops.len()
    ));

    let ne_dir = std::path::Path::new("data/natural-earth");
    let river_sources = map_adapters::hydro::read_rivers(
        &std::fs::read_to_string(ne_dir.join("ne_10m_rivers_lake_centerlines.geojson"))
            .unwrap_or_else(|error| die(&format!("Natural Earth rivers: {error}"))),
    )
    .unwrap_or_else(|error| die(&format!("Natural Earth rivers: {error:?}")));
    let unlocated: Vec<_> = river_sources
        .iter()
        .filter_map(|river| match river.shape {
            map_adapters::hydro::RiverShape::Course(_) => None,
            map_adapters::hydro::RiverShape::Unlocated => Some(format!(
                "{} (NE {})",
                river.name.as_deref().unwrap_or("unnamed"),
                river.number.0,
            )),
        })
        .collect();
    let river_census = format!(
        "Natural Earth river input: {} source records; unlocated courses: {unlocated:?}",
        river_sources.len(),
    );
    eprintln!("{river_census}");
    report_md.push_str(&format!("- {river_census}\n"));
    let creation = atlas.creation_anchor().map(|(y, _)| y).unwrap_or(-4004);
    let tp0 = ts_or_die(creation);
    let waters = vec![
        map_adapters::WaterSource {
            label_for_unnamed: "inland sea",
            text: std::fs::read_to_string(ne_dir.join("ne_110m_ocean.geojson"))
                .unwrap_or_else(|e| die(&format!("ocean: {e}"))),
            skip_largest_feature: true,
        },
        map_adapters::WaterSource {
            label_for_unnamed: "lake",
            text: std::fs::read_to_string(ne_dir.join("ne_10m_lakes.geojson"))
                .unwrap_or_else(|e| die(&format!("lakes: {e}"))),
            skip_largest_feature: false,
        },
    ];
    let water_only = map_adapters::ingest_water(
        &atlas_graph_types::covenant::SourceId::new("natural-earth"),
        tp0,
        &waters,
    )
    .unwrap_or_else(|e| die(&format!("water: {e:?}")));
    let ocean = map_adapters::ingest_ocean(
        &atlas_graph_types::covenant::SourceId::new("natural-earth"),
        tp0,
        &std::fs::read_to_string(ne_dir.join("ne_10m_land.geojson"))
            .unwrap_or_else(|e| die(&format!("land: {e}"))),
    )
    .unwrap_or_else(|e| die(&format!("ocean: {e:?}")));
    let water_tl = map_adapters::merge_timelines(water_only, ocean)
        .unwrap_or_else(|e| die(&format!("water merge: {e:?}")));
    bridge_filtered(
        &mut store,
        &water_tl,
        LayerKind::Water,
        Witness::NaturalEarth,
        "natural-earth",
        Some(map_types::RegionClass::Water),
        &BTreeSet::new(),
        &mut identity,
    )
    .unwrap_or_else(|e| die(&e));
    bridge_filtered(
        &mut store,
        &water_tl,
        LayerKind::Relief,
        Witness::NaturalEarth,
        "natural-earth",
        Some(map_types::RegionClass::Terrain(0)),
        &BTreeSet::new(),
        &mut identity,
    )
    .unwrap_or_else(|e| die(&e));
    eprintln!("water: Natural Earth; river courses and recorded gaps: docs/errata/rivers.md");
    report_md.push_str("- River courses: Natural Earth 1:10m (public domain, modern generalized courses); golden requirements and missing courses: docs/errata/rivers.md\n");

    let era_table = {
        let text = std::fs::read_to_string("data/atlas-vendor/eras.json")
            .unwrap_or_else(|e| die(&format!("eras: {e}")));
        parse_eras(&text).unwrap_or_else(|e| die(&e))
    };
    let summary =
        map_compile::partition_bridge::bridge_partition(&mut store, tp0, &polities, &mut identity)
            .unwrap_or_else(|e| die(&format!("partition: {e}")));
    eprintln!("{summary}");
    report_md.push_str(&format!(
        "- {summary}
"
    ));

    let terrain_bytes = std::fs::read("data/terrain/etopo_15min.bin")
        .unwrap_or_else(|e| die(&format!("terrain: {e}")));
    let grid = map_adapters::ElevationGrid::from_etopo_bin(&terrain_bytes)
        .unwrap_or_else(|| die("terrain grid: wrong shape"));
    let terrain_tl = map_adapters::ingest_terrain(&grid, tp0);
    bridge_filtered(
        &mut store,
        &terrain_tl,
        LayerKind::Relief,
        Witness::NaturalEarth,
        "etopo",
        Some(map_types::RegionClass::Terrain(0)),
        &BTreeSet::new(),
        &mut identity,
    )
    .unwrap_or_else(|e| die(&e));
    eprintln!("relief: bridged");

    report_md.push_str("- Historical background: GPL outlines excluded; missing coverage and permitted replacement evidence: docs/errata/background.md\n");

    let frame_edge = era_table
        .iter()
        .map(|e| e.to_year)
        .max()
        .map(ts_or_die)
        .unwrap_or_else(|| die("eras: empty table"));
    let leaks = store.validate_frame_edge(map_canon::LayerKind::ScriptureClaims, &frame_edge);
    if !leaks.is_empty() {
        die(&format!(
            "frame-edge law: {} feature(s) stand at the end of time without a declared \
             endurance: {}",
            leaks.len(),
            leaks.join("; ")
        ));
    }
    eprintln!("frame-edge law: ScriptureClaims clean at the frame's last year");

    let all_violations = store.validate();
    let (waived, violations): (Vec<_>, Vec<_>) =
        all_violations.into_iter().partition(|v| match v {
            map_canon::CanonViolation::TerritorialOverlap { a, b, .. } => {
                is_waived(&rec, &a.0, &b.0)
            }
            _ => false,
        });
    if !waived.is_empty() {
        eprintln!(
            "warnings: {} acknowledged territorial conflicts (see report)",
            waived.len()
        );
        report_md.push_str(&format!(
            "
## Acknowledged conflicts (awaiting upstream ruling)

{} waived overlap moments
",
            waived.len()
        ));
    }
    report_md.push_str(&format!(
        "\n## Validation\n\n{} violations\n",
        violations.len()
    ));
    for v in violations.iter().take(50) {
        report_md.push_str(&format!("- {v:?}\n"));
    }
    std::fs::create_dir_all(&out_dir).unwrap_or_else(|e| die(&format!("{e}")));
    std::fs::write(
        std::path::Path::new(&out_dir).join("reconciliation-report.md"),
        &report_md,
    )
    .unwrap_or_else(|e| die(&format!("{e}")));
    if !violations.is_empty() {
        eprintln!(
            "canon INVALID: {} violations (see reconciliation-report.md)",
            violations.len()
        );
        for v in violations.iter().take(10) {
            eprintln!("  {v:?}");
        }
        std::process::exit(1);
    }
    map_compile::exclusion::check_compiled(&store)
        .unwrap_or_else(|error| die(&format!("excluded output: {error:?}")));
    identity.check().unwrap_or_else(|e| die(&e));
    let bytes = map_canon::persist::to_bytes(&store).unwrap_or_else(|e| die(&e));
    std::fs::write(std::path::Path::new(&out_dir).join("canon.json"), &bytes)
        .unwrap_or_else(|e| die(&format!("{e}")));
    eprintln!(
        "canon: {} borders, {} features, {} snapshots, {} layers -> {}/canon.json ({} bytes)",
        store.borders().len(),
        store.features().len(),
        store.snapshots().len(),
        store.layers().len(),
        out_dir,
        bytes.len()
    );
}
