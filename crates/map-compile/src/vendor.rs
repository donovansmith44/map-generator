//! Refresh: pull the atlas API into typed vendored files. Every
//! payload is PARSED before it is written — shape drift fails loud and
//! vendors nothing — and the writer is deterministic: same payloads,
//! same bytes, same pin. Errors are Strings naming what went missing.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{Read as _, Write as _};
use std::path::Path;

use serde_json::Value;

// ------------------------------------------------------- typed rows

#[derive(Clone, Debug, PartialEq)]
pub struct PolityRow {
    pub id: String,
    pub name: String,
    pub from_year: i32,
    pub to_year: i32,
    pub rings: Vec<Vec<(f64, f64)>>, // (lat, lon)
    pub color_key: Option<u8>,
    pub transition_verses: Vec<String>,
    pub fall_verses: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NarrativeRow {
    pub id: String,
    pub name: String,
    pub color: String,
    pub legs: Vec<String>, // event ids, in walk order
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventRow {
    pub id: String,
    pub label: String,
    pub when: Option<(i32, i32)>,
    pub places: Vec<String>, // place ids, text order
    pub verses: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EraRow {
    pub id: String,
    pub name: String,
    pub from_year: i32,
    pub to_year: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LandmarkRow {
    pub name: String,
    pub kind: String,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LandMask {
    pub rings: Vec<Vec<(f64, f64)>>,
}

// ------------------------------------------------------- field access

fn field<'a>(v: &'a Value, ctx: &str, name: &str) -> Result<&'a Value, String> {
    v.get(name).ok_or_else(|| format!("{ctx}: missing field '{name}'"))
}

fn str_field(v: &Value, ctx: &str, name: &str) -> Result<String, String> {
    field(v, ctx, name)?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("{ctx}: field '{name}' is not a string"))
}

fn i32_field(v: &Value, ctx: &str, name: &str) -> Result<i32, String> {
    field(v, ctx, name)?
        .as_i64()
        .map(|n| n as i32)
        .ok_or_else(|| format!("{ctx}: field '{name}' is not a number"))
}

fn rings_field(v: &Value, ctx: &str, name: &str) -> Result<Vec<Vec<(f64, f64)>>, String> {
    let arr = field(v, ctx, name)?
        .as_array()
        .ok_or_else(|| format!("{ctx}: '{name}' is not an array"))?;
    let mut rings = Vec::new();
    for ring in arr {
        let pts = ring
            .as_array()
            .ok_or_else(|| format!("{ctx}: '{name}' ring is not an array"))?;
        let mut out = Vec::with_capacity(pts.len());
        for p in pts {
            let pair = p
                .as_array()
                .filter(|a| a.len() == 2)
                .ok_or_else(|| format!("{ctx}: '{name}' point is not a [lat,lon] pair"))?;
            let lat = pair[0].as_f64().ok_or_else(|| format!("{ctx}: lat not a number"))?;
            let lon = pair[1].as_f64().ok_or_else(|| format!("{ctx}: lon not a number"))?;
            out.push((lat, lon));
        }
        rings.push(out);
    }
    Ok(rings)
}

fn delta_verses(v: &Value, name: &str) -> Vec<String> {
    v.get(name)
        .and_then(|d| d.get("verses"))
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default()
}

// ------------------------------------------------------- the parsers

pub fn parse_polities(json: &str) -> Result<Vec<PolityRow>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("polities: bad json: {e}"))?;
    let rows = field(&v, "polities", "polities")?
        .as_array()
        .ok_or("polities: 'polities' is not an array")?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let ctx = format!(
            "polity '{}'",
            row.get("id").and_then(Value::as_str).unwrap_or("?")
        );
        out.push(PolityRow {
            id: str_field(row, &ctx, "id")?,
            name: str_field(row, &ctx, "name")?,
            from_year: i32_field(row, &ctx, "from")?,
            to_year: i32_field(row, &ctx, "to")?,
            rings: rings_field(row, &ctx, "rings")?,
            color_key: row.get("color_key").and_then(Value::as_u64).map(|n| n as u8),
            transition_verses: delta_verses(row, "transition"),
            fall_verses: delta_verses(row, "fall"),
        });
    }
    Ok(out)
}

pub fn parse_narratives(json: &str) -> Result<Vec<NarrativeRow>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("narratives: bad json: {e}"))?;
    let rows = v.as_array().ok_or("narratives: not an array")?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let ctx = format!(
            "narrative '{}'",
            row.get("id").and_then(Value::as_str).unwrap_or("?")
        );
        let legs = field(row, &ctx, "legs")?
            .as_array()
            .ok_or_else(|| format!("{ctx}: 'legs' is not an array"))?
            .iter()
            .map(|l| l.as_str().map(str::to_string).ok_or_else(|| format!("{ctx}: leg id")))
            .collect::<Result<Vec<_>, _>>()?;
        out.push(NarrativeRow {
            id: str_field(row, &ctx, "id")?,
            name: str_field(row, &ctx, "name")?,
            color: str_field(row, &ctx, "color")?,
            legs,
        });
    }
    Ok(out)
}

pub fn parse_event(json: &str) -> Result<EventRow, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("event: bad json: {e}"))?;
    let ctx = format!("event '{}'", v.get("id").and_then(Value::as_str).unwrap_or("?"));
    let when = match v.get("when") {
        Some(w) if !w.is_null() => {
            Some((i32_field(w, &ctx, "from_year")?, i32_field(w, &ctx, "to_year")?))
        }
        _ => None,
    };
    let places = field(&v, &ctx, "places")?
        .as_array()
        .ok_or_else(|| format!("{ctx}: 'places' is not an array"))?
        .iter()
        .map(|p| str_field(p, &ctx, "id"))
        .collect::<Result<Vec<_>, _>>()?;
    let mut verses = Vec::new();
    if let Some(ws) = v.get("witnesses").and_then(Value::as_array) {
        for w in ws {
            if let Some(groups) = w.get("verse_groups").and_then(Value::as_array) {
                for g in groups {
                    if let Some(vs) = g.get("verses").and_then(Value::as_array) {
                        verses
                            .extend(vs.iter().filter_map(Value::as_str).map(str::to_string));
                    }
                }
            }
        }
    }
    Ok(EventRow {
        id: str_field(&v, &ctx, "id")?,
        label: str_field(&v, &ctx, "title").or_else(|_| str_field(&v, &ctx, "label"))?,
        when,
        places,
        verses,
    })
}

pub fn parse_eras(json: &str) -> Result<Vec<EraRow>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("eras: bad json: {e}"))?;
    v.as_array()
        .ok_or("eras: not an array")?
        .iter()
        .map(|row| {
            let ctx = "era";
            Ok(EraRow {
                id: str_field(row, ctx, "id")?,
                name: str_field(row, ctx, "name")?,
                from_year: i32_field(row, ctx, "from_year")?,
                to_year: i32_field(row, ctx, "to_year")?,
            })
        })
        .collect()
}

pub fn parse_landmarks(json: &str) -> Result<Vec<LandmarkRow>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("landmarks: bad json: {e}"))?;
    v.as_array()
        .ok_or("landmarks: not an array")?
        .iter()
        .map(|row| {
            let ctx = "landmark";
            Ok(LandmarkRow {
                name: str_field(row, ctx, "name")?,
                kind: str_field(row, ctx, "kind")?,
                lat: field(row, ctx, "lat")?.as_f64().ok_or("landmark: lat")?,
                lon: field(row, ctx, "lon")?.as_f64().ok_or("landmark: lon")?,
            })
        })
        .collect()
}

pub fn parse_land_mask(json: &str) -> Result<LandMask, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("land-mask: bad json: {e}"))?;
    Ok(LandMask { rings: rings_field(&v, "land-mask", "rings")? })
}

// -------------------------------------------------- deterministic write

/// Write vendored payloads plus a manifest. The pin is the content
/// hash of every payload, in name order — same payloads, same pin,
/// byte-identical manifest (no clocks, no randomness).
pub fn write_vendor(dir: &Path, payloads: &[(String, Vec<u8>)]) -> Result<u64, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("vendor dir: {e}"))?;
    let mut sorted: Vec<&(String, Vec<u8>)> = payloads.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut h = DefaultHasher::new();
    for (name, bytes) in &sorted {
        name.hash(&mut h);
        bytes.hash(&mut h);
    }
    let pin = h.finish();
    let mut files = Vec::new();
    for (name, bytes) in &sorted {
        std::fs::write(dir.join(name), bytes).map_err(|e| format!("write {name}: {e}"))?;
        files.push(serde_json::json!({ "file": name, "bytes": bytes.len() }));
    }
    let manifest = serde_json::json!({ "pin": format!("{pin:016x}"), "files": files });
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("write manifest: {e}"))?;
    Ok(pin)
}

// ------------------------------------------------------- plain HTTP GET

/// Minimal HTTP/1.1 GET for the local atlas API (no TLS — the atlas
/// runs on loopback). Returns the body on 200, an error otherwise.
pub fn http_get(host: &str, port: u16, path: &str) -> Result<Vec<u8>, String> {
    let mut stream = std::net::TcpStream::connect((host, port))
        .map_err(|e| format!("connect {host}:{port}: {e}"))?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(30)))
        .map_err(|e| e.to_string())?;
    let req = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).map_err(|e| format!("send: {e}"))?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).map_err(|e| format!("read: {e}"))?;
    let split = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("malformed http response")?;
    let head = String::from_utf8_lossy(&buf[..split]).to_string();
    let status = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .ok_or("no status line")?
        .to_string();
    if status != "200" {
        return Err(format!("{path}: HTTP {status}"));
    }
    let mut body = buf[split + 4..].to_vec();
    // Chunked transfer: dechunk (the atlas may answer either way).
    if head.to_ascii_lowercase().contains("transfer-encoding: chunked") {
        body = dechunk(&body)?;
    }
    Ok(body)
}

fn dechunk(body: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut rest = body;
    loop {
        let line_end =
            rest.windows(2).position(|w| w == b"\r\n").ok_or("chunk: missing size line")?;
        let size = usize::from_str_radix(
            String::from_utf8_lossy(&rest[..line_end]).trim(),
            16,
        )
        .map_err(|e| format!("chunk size: {e}"))?;
        rest = &rest[line_end + 2..];
        if size == 0 {
            return Ok(out);
        }
        if rest.len() < size + 2 {
            return Err("chunk: truncated".to_string());
        }
        out.extend_from_slice(&rest[..size]);
        rest = &rest[size + 2..];
    }
}
