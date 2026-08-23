//! Reading the source's feature collections. This module is the ONLY
//! place that knows the input serialization; everything after it works
//! on `SourceFeature`s. (Law 11 concerns OUTPUT formats — inputs are an
//! adapter's whole business — but the same terminality discipline
//! applies inward: nothing downstream of this module parses text.)

use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct SourcePolygon {
    pub outer: Vec<(f64, f64)>,        // (lon, lat) in degrees
    pub holes: Vec<Vec<(f64, f64)>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceFeature {
    pub name: Option<String>,
    /// The source's own honesty signal (BORDERPRECISION), carried
    /// through to justifications verbatim — never reinterpreted.
    pub precision: Option<i64>,
    pub polygons: Vec<SourcePolygon>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParseError {
    BadSyntax(String),
    /// Structurally not a feature collection of polygonal features.
    BadShape(&'static str),
}

fn ring(v: &Value) -> Result<Vec<(f64, f64)>, ParseError> {
    let arr = v.as_array().ok_or(ParseError::BadShape("ring is not an array"))?;
    let mut pts = Vec::with_capacity(arr.len());
    for p in arr {
        let xy = p.as_array().ok_or(ParseError::BadShape("position is not an array"))?;
        let (Some(lon), Some(lat)) = (xy.first().and_then(Value::as_f64), xy.get(1).and_then(Value::as_f64))
        else {
            return Err(ParseError::BadShape("position is not numeric"));
        };
        pts.push((lon, lat));
    }
    Ok(pts)
}

fn polygon(rings: &Value) -> Result<SourcePolygon, ParseError> {
    let arr = rings.as_array().ok_or(ParseError::BadShape("polygon is not an array"))?;
    let mut it = arr.iter();
    let outer = ring(it.next().ok_or(ParseError::BadShape("polygon has no outer ring"))?)?;
    let holes = it.map(ring).collect::<Result<Vec<_>, _>>()?;
    Ok(SourcePolygon { outer, holes })
}

/// Parse a feature collection's text into source features. Non-polygon
/// geometries are a shape error — this source is a border atlas.
pub fn parse_features(text: &str) -> Result<Vec<SourceFeature>, ParseError> {
    let root: Value =
        serde_json::from_str(text).map_err(|e| ParseError::BadSyntax(e.to_string()))?;
    let features = root
        .get("features")
        .and_then(Value::as_array)
        .ok_or(ParseError::BadShape("no features array"))?;
    let mut out = Vec::with_capacity(features.len());
    for f in features {
        let props = f.get("properties").unwrap_or(&Value::Null);
        let name = props.get("NAME").and_then(Value::as_str).map(str::to_string);
        let precision = props.get("BORDERPRECISION").and_then(Value::as_i64);
        let geom = f.get("geometry").ok_or(ParseError::BadShape("feature has no geometry"))?;
        let coords = geom.get("coordinates").ok_or(ParseError::BadShape("no coordinates"))?;
        let polygons = match geom.get("type").and_then(Value::as_str) {
            Some("Polygon") => vec![polygon(coords)?],
            Some("MultiPolygon") => coords
                .as_array()
                .ok_or(ParseError::BadShape("multipolygon is not an array"))?
                .iter()
                .map(polygon)
                .collect::<Result<Vec<_>, _>>()?,
            _ => return Err(ParseError::BadShape("geometry is not polygonal")),
        };
        out.push(SourceFeature { name, precision, polygons });
    }
    Ok(out)
}
