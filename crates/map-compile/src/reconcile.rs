//! Reconciliation: no witness's claim is dropped SILENTLY. Every
//! authored route (and superseded region group) must have a row in
//! data/authored/reconcile.json; a missing row fails the compile, a
//! supersession must point at a real atlas id, and the verdicts feed
//! the report.

use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub enum RouteRule {
    /// Kept: compiled into the Journeys layer under the Authored witness.
    Keep,
    /// Dropped: this atlas narrative id carries the same walk.
    SupersededBy(String),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reconcile {
    pub routes: Vec<(String, RouteRule)>,
    /// Authored region-group slugs superseded by atlas polities.
    pub region_drops: Vec<(String, String)>, // (slug, atlas polity id)
    /// ACKNOWLEDGED territorial conflicts awaiting an upstream ruling:
    /// (entity a, entity b, note). Waived pairs downgrade to warnings;
    /// everything else stays fatal.
    pub territory_waivers: Vec<(String, String, String)>,
}

pub fn parse_reconcile(json: &str) -> Result<Reconcile, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("reconcile: bad json: {e}"))?;
    let mut out = Reconcile::default();
    if let Some(rows) = v.get("routes").and_then(Value::as_array) {
        for row in rows {
            let authored = row
                .get("authored")
                .and_then(Value::as_str)
                .ok_or("reconcile route: missing 'authored'")?
                .to_string();
            let rule = if row.get("keep").and_then(Value::as_bool) == Some(true) {
                RouteRule::Keep
            } else if let Some(s) = row.get("superseded_by").and_then(Value::as_str) {
                RouteRule::SupersededBy(s.to_string())
            } else {
                return Err(format!(
                    "reconcile route '{authored}': needs 'keep': true or 'superseded_by'"
                ));
            };
            out.routes.push((authored, rule));
        }
    }
    if let Some(rows) = v.get("territory_conflicts").and_then(Value::as_array) {
        for row in rows {
            let a = row
                .get("a")
                .and_then(Value::as_str)
                .ok_or("territory_conflicts: missing 'a'")?
                .to_string();
            let b = row
                .get("b")
                .and_then(Value::as_str)
                .ok_or("territory_conflicts: missing 'b'")?
                .to_string();
            let note = row.get("note").and_then(Value::as_str).unwrap_or_default().to_string();
            out.territory_waivers.push((a, b, note));
        }
    }
    if let Some(rows) = v.get("regions").and_then(Value::as_array) {
        for row in rows {
            let slug = row
                .get("authored_slug")
                .and_then(Value::as_str)
                .ok_or("reconcile region: missing 'authored_slug'")?
                .to_string();
            let by = row
                .get("superseded_by")
                .and_then(Value::as_str)
                .ok_or("reconcile region: missing 'superseded_by'")?
                .to_string();
            out.region_drops.push((slug, by));
        }
    }
    Ok(out)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RouteVerdicts {
    pub kept: Vec<String>,
    pub dropped: Vec<String>,
}

/// Apply the route rules: every authored route must be mentioned, and
/// every supersession must name a real atlas narrative.
pub fn reconcile_routes(
    rec: &Reconcile,
    authored: &[String],
    atlas_narratives: &[String],
) -> Result<RouteVerdicts, String> {
    let mut verdicts = RouteVerdicts::default();
    for tag in authored {
        let rule = rec
            .routes
            .iter()
            .find(|(a, _)| a == tag)
            .map(|(_, r)| r)
            .ok_or_else(|| {
                format!("authored route '{tag}' has no reconciliation row — no silent precedence")
            })?;
        match rule {
            RouteRule::Keep => verdicts.kept.push(tag.clone()),
            RouteRule::SupersededBy(n) => {
                if !atlas_narratives.contains(n) {
                    return Err(format!(
                        "route '{tag}' superseded by '{n}', but no such atlas narrative exists"
                    ));
                }
                verdicts.dropped.push(tag.clone());
            }
        }
    }
    Ok(verdicts)
}

/// Is this territorial pair acknowledged (either order)?
pub fn is_waived(rec: &Reconcile, a: &str, b: &str) -> bool {
    rec.territory_waivers
        .iter()
        .any(|(x, y, _)| (x == a && y == b) || (x == b && y == a))
}
