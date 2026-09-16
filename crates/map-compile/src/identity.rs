//! THE REGISTRY, READ AND KEPT: data/authored/registry.json is the one
//! place two minted ids become one real thing. Every bridge resolves
//! the id it mints through `Identity` and observes the witness that
//! minted it, so the compile can validate the whole book at the end:
//! a canonical id no witness ever minted, a chain, or two witnesses
//! disagreeing about what kind of thing an entity is, refuses the
//! compile by name.

use std::collections::BTreeMap;

use map_canon::{EntityId, EntityKind, LayerKind, Registry, RegistryViolation, Unification, Witness, WitnessRef};
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

/// The registry plus the kind each written row declares for its
/// entity, so a witness of a declared entity votes the declared kind
/// and an undeclared witness votes what its layer and shape imply.
pub struct Identity {
    registry: Registry,
    declared: BTreeMap<EntityId, EntityKind>,
}

impl Identity {
    pub fn resolve<'a>(&'a self, minted: &'a EntityId) -> &'a EntityId {
        self.registry.resolve(minted)
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// One witness's testimony, resolved and recorded. `kind` is the
    /// geometry kind ("area", "way", "point", "line", "memory").
    pub fn witness(&mut self, minted: &EntityId, name: &str, layer: LayerKind, witness: Witness, kind: &'static str) {
        let home = self.registry.resolve(minted).clone();
        let entity_kind = self
            .declared
            .get(minted)
            .or_else(|| self.declared.get(&home))
            .copied()
            .unwrap_or_else(|| implied_kind(layer, kind));
        self.registry.observe(
            minted.clone(),
            name,
            entity_kind,
            WitnessRef { minted_as: minted.clone(), witness, layer, kind },
        );
    }

    /// Every violation the book commits, as one refusal naming them all.
    pub fn check(&self) -> Result<(), String> {
        let found: Vec<String> = self
            .registry
            .validate()
            .into_iter()
            .map(|v| match v {
                RegistryViolation::DanglingCanonical(id) => {
                    format!("canonical id '{}' was never minted by any witness", id.0)
                }
                RegistryViolation::ChainedUnification { minted, via } => {
                    format!("'{}' resolves through '{}': unification must be one hop", minted.0, via.0)
                }
                RegistryViolation::KindConflict { entity, a, b } => {
                    format!("'{}' is witnessed as both {a:?} and {b:?}", entity.0)
                }
            })
            .collect();
        if found.is_empty() {
            Ok(())
        } else {
            Err(format!("registry: {}", found.join("; ")))
        }
    }
}

fn implied_kind(layer: LayerKind, kind: &str) -> EntityKind {
    match (layer, kind) {
        (_, "way") => EntityKind::Route,
        (_, "point") | (_, "memory") => EntityKind::Place,
        (_, "line") => EntityKind::Waterbody,
        (LayerKind::Water, _) => EntityKind::Waterbody,
        (LayerKind::Relief, _) => EntityKind::Terrain,
        (LayerKind::ScriptureClaims, _) => EntityKind::Allotment,
        (LayerKind::Territory, _) | (LayerKind::Background, _) | (LayerKind::Journeys, _) => EntityKind::Polity,
    }
}

pub fn load_registry(json: &str, source: &str) -> Result<Identity, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("registry: bad json: {e}"))?;
    let mut registry = Registry::default();
    let mut declared = BTreeMap::new();
    for row in v.get("unifications").and_then(Value::as_array).into_iter().flatten() {
        let field = |k: &str| -> Result<String, String> {
            row.get(k)
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| format!("registry: a unification lacks '{k}'"))
        };
        let (canonical, minted) = (EntityId(field("canonical")?), EntityId(field("minted")?));
        let kind = kind_of(&field("kind")?)?;
        let reason = field("reason")?;
        registry
            .declare(canonical.clone(), minted.clone(), Unification::Declared { reason, source: source.to_string() })
            .map_err(|e| format!("registry: {e}"))?;
        declared.insert(canonical, kind);
        declared.insert(minted, kind);
    }
    let chains: Vec<String> = registry
        .validate()
        .into_iter()
        .filter_map(|v| match v {
            RegistryViolation::ChainedUnification { minted, via } => Some(format!("{} resolves through {}", minted.0, via.0)),
            _ => None,
        })
        .collect();
    if !chains.is_empty() {
        return Err(format!("registry: unification chains: {}", chains.join("; ")));
    }
    Ok(Identity { registry, declared })
}
