//! Serverless file artifacts (phase 7): the same composition root and
//! the same routes as the workbench, with files instead of sockets.
//! The artifact filename IS the cache key: the content hash of the
//! canonicalized query (params sorted) plus the world's atlas pin —
//! same query against the same world, same name, same bytes.
//!
//!   map-cli render "year=-1000&zoom=8&center=32.5,35.5" [more…]
//!   map-cli plates            # the canonical plate book + manifest
//!
//! Output lands in MAP_CLI_OUT (default out/artifacts): one file per
//! query, named <hash16>.<ext>, plus manifest.json mapping each query
//! (and its optional plate name) to its artifact.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Sorted-params canonical form: the SAME query written two ways must
/// name the same artifact.
fn canonical_query(q: &str) -> String {
    let mut parts: Vec<&str> = q.split('&').filter(|p| !p.is_empty()).collect();
    parts.sort_unstable();
    parts.join("&")
}

fn artifact_name(canonical: &str, world_pin: &str, ext: &str) -> String {
    let mut h = DefaultHasher::new();
    world_pin.hash(&mut h);
    canonical.hash(&mut h);
    format!("{:016x}.{ext}", h.finish())
}

/// The canonical plate book — the owner's Bible maps, as (name, query)
/// rows. Mirrors scripts/make-maps.sh; the CLI needs no server.
const PLATES: &[(&str, &str)] = &[
    ("01-eden-and-the-nations", "subject=world&year=-2200&center=33,36&zoom=22&bible=1"),
    ("02-the-land-promised-num34", "subject=world&year=-1450&center=32.6,35.6&zoom=4.5&bible=1"),
    ("03-allotments-of-israel-jos13-19", "subject=world&year=-1400&center=32.6,35.6&zoom=4.5&bible=1"),
    ("04-united-kingdom", "subject=world&year=-1000&center=32.6,35.6&zoom=4.5&bible=1"),
    ("05-divided-kingdoms", "subject=world&year=-900&center=32.6,35.6&zoom=4.5&bible=1"),
    ("06-after-samaria-falls", "subject=world&year=-721&center=32.6,35.6&zoom=4.5&bible=1"),
    ("07-exile", "subject=world&year=-586&center=33.5,36.5&zoom=8&bible=1"),
    ("08-exodus-journeys-num33", "subject=world&year=-1470&center=30.2,34.3&zoom=5&bible=1"),
    ("09-ezekiel-vision-ezk47", "subject=world&year=-570&center=32.6,35.6&zoom=4.5&bible=1"),
    ("10-tetrarchies-luk3", "subject=world&year=30&center=32.3,35.4&zoom=2.5&bible=1"),
    ("11-pauls-journeys-acts", "subject=world&year=64&center=37.5,28&zoom=12&bible=1"),
    ("12-world-at-abraham", "subject=world&year=-1900&center=33,38&zoom=18"),
    ("13-world-at-the-exile", "subject=world&year=-586&center=33,38&zoom=18"),
    ("14-world-at-messiah", "subject=world&year=-1&center=33,38&zoom=18"),
    ("15-relief-near-east", "subject=world&year=-2500&center=32.5,38&zoom=12&relief=1"),
    ("16-whole-globe", "subject=world&year=-1000&center=31,35&zoom=90"),
];

fn ext_for(ctype: &str) -> &'static str {
    match ctype {
        c if c.contains("svg") => "svg",
        c if c.contains("json") => "json",
        _ => "txt",
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out_dir = std::env::var("MAP_CLI_OUT").unwrap_or_else(|_| "out/artifacts".to_string());
    std::fs::create_dir_all(&out_dir).expect("artifact directory");

    eprintln!("loading the world…");
    let app = map_viewer::load();
    let world_pin = {
        // The world side of the cache key: the anchor frame/year the
        // composition root reports. A re-vendored atlas that moves the
        // frame moves every artifact name — staleness is visible.
        let (frame, year) = app.anchor.clone().unwrap_or_default();
        format!("{frame}@{year}")
    };

    let jobs: Vec<(Option<String>, String)> = match args.first().map(String::as_str) {
        Some("render") if args.len() > 1 => {
            args[1..].iter().map(|q| (None, q.clone())).collect()
        }
        Some("plates") | None => {
            PLATES.iter().map(|(n, q)| (Some((*n).to_string()), (*q).to_string())).collect()
        }
        _ => {
            eprintln!("usage: map-cli render <query>…  |  map-cli plates");
            std::process::exit(2);
        }
    };

    let mut manifest = Vec::new();
    for (name, query) in &jobs {
        let canonical = canonical_query(query);
        let (status, ctype, body, _) = map_viewer::route(&app, "/api/render", &canonical);
        if status != 200 {
            eprintln!("FAILED {query}: {}", String::from_utf8_lossy(&body));
            std::process::exit(1);
        }
        let file = artifact_name(&canonical, &world_pin, ext_for(ctype));
        let path = std::path::Path::new(&out_dir).join(&file);
        std::fs::write(&path, &body).expect("artifact writes");
        eprintln!("{} <- {}", file, name.as_deref().unwrap_or(&canonical));
        manifest.push(serde_json::json!({
            "name": name,
            "query": canonical,
            "artifact": file,
            "bytes": body.len(),
        }));
    }
    let doc = serde_json::json!({
        "world": world_pin,
        "artifacts": manifest,
    });
    std::fs::write(
        std::path::Path::new(&out_dir).join("manifest.json"),
        serde_json::to_string_pretty(&doc).unwrap(),
    )
    .expect("manifest writes");
    eprintln!("done: {} artifacts in {out_dir}", jobs.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cache key ignores param order but nothing else — and a
    /// moved world pin renames every artifact.
    #[test]
    fn artifact_names_are_canonical() {
        let a = canonical_query("year=-1000&zoom=8");
        let b = canonical_query("zoom=8&year=-1000");
        assert_eq!(a, b);
        assert_eq!(artifact_name(&a, "w@1", "svg"), artifact_name(&b, "w@1", "svg"));
        assert_ne!(artifact_name(&a, "w@1", "svg"), artifact_name(&a, "w@2", "svg"));
        assert_ne!(
            artifact_name(&canonical_query("year=-999&zoom=8"), "w@1", "svg"),
            artifact_name(&a, "w@1", "svg")
        );
    }
}
