//! map-compile refresh: pull the atlas API into data/atlas-vendor.
//!
//!   map-compile refresh [--base 127.0.0.1:8080] [--out data/atlas-vendor]
//!
//! Every payload is parse-validated BEFORE anything is written; a shape
//! error vendors nothing. The manifest pin moves iff the data moved.

use map_compile::vendor::*;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("refresh") {
        eprintln!("usage: map-compile refresh [--base host:port] [--out dir]");
        std::process::exit(2);
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

    // The whole span the anchor allows; the atlas clamps as it sees fit.
    let polities = get("/api/polities?from=-4004&to=2000");
    let polity_rows = parse_polities(&String::from_utf8_lossy(&polities))
        .unwrap_or_else(|e| die(&e));
    eprintln!("  {} polity eras", polity_rows.len());
    payloads.push(("polities.json".to_string(), polities));

    let narratives = get("/api/narratives");
    let narrative_rows = parse_narratives(&String::from_utf8_lossy(&narratives))
        .unwrap_or_else(|e| die(&e));
    eprintln!("  {} narratives", narrative_rows.len());
    payloads.push(("narratives.json".to_string(), narratives));

    // Fan out: every narrative leg's event, dated and versed.
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
        serde_json::to_string_pretty(&serde_json::Value::Array(events)).unwrap().into_bytes(),
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
    eprintln!("vendored {} files to {out}, pin {pin:016x}", payloads.len() + 1);
}

fn die(e: &str) -> ! {
    eprintln!("refresh FAILED: {e}");
    std::process::exit(1);
}
