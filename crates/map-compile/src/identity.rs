//! THE REGISTRY, READ: data/authored/registry.json is the one place two
//! minted ids become one real thing, and every bridge resolves the id
//! it mints through it before a feature is stored. A row is a written
//! act with a reason; a chain, a self-unification or an unknown kind
//! refuses the whole compile by name.

use map_canon::{EntityId, EntityKind, Registry, RegistryViolation, Unification};
use serde_json::Value;

fn kind_of(s: &str) -> Result<EntityKind, String> {
    Ok(match s {
        "Polity" => EntityKind::Polity,
        "District" => EntityKind::District,
        "People" => EntityKind::People,
        "Allotment" => EntityKind::Allotment,
        "Promise" => EntityKind::Promise,
        "Vision" => EntityKind::Vision,
        "Waterbody" => EntityKind::Waterbody,
        "Terrain" => EntityKind::Terrain,
        "Place" => EntityKind::Place,
        "Route" => EntityKind::Route,
        other => return Err(format!("registry: unknown entity kind '{other}'")),
    })
}

pub fn load_registry(json: &str, source: &str) -> Result<Registry, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("registry: bad json: {e}"))?;
    let mut reg = Registry::default();
    for row in v.get("unifications").and_then(Value::as_array).into_iter().flatten() {
        let field = |k: &str| -> Result<String, String> {
            row.get(k)
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| format!("registry: a unification lacks '{k}'"))
        };
        let (canonical, minted) = (field("canonical")?, field("minted")?);
        kind_of(&field("kind")?)?;
        let reason = field("reason")?;
        reg.declare(
            EntityId(canonical),
            EntityId(minted),
            Unification::Declared { reason, source: source.to_string() },
        )
        .map_err(|e| format!("registry: {e}"))?;
    }
    let chains: Vec<String> = reg
        .validate()
        .into_iter()
        .filter_map(|v| match v {
            RegistryViolation::ChainedUnification { minted, via } => {
                Some(format!("{} resolves through {}", minted.0, via.0))
            }
            _ => None,
        })
        .collect();
    if !chains.is_empty() {
        return Err(format!("registry: unification chains: {}", chains.join("; ")));
    }
    Ok(reg)
}
