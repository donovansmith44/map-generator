use std::collections::{BTreeMap, BTreeSet};

use crate::{EntityId, LayerKind, Witness};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntityKind {
    Polity,
    District,
    People,
    Allotment,
    Promise,
    Vision,
    Waterbody,
    Terrain,
    Place,
    Route,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unification {
    SameId,
    Declared { reason: String, source: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WitnessRef {
    pub minted_as: EntityId,
    pub witness: Witness,
    pub layer: LayerKind,
    pub kind: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entity {
    pub id: EntityId,
    pub names: Vec<String>,
    pub kind: EntityKind,
    pub witnesses: Vec<WitnessRef>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Registry {
    aliases: BTreeMap<EntityId, EntityId>,
    why: BTreeMap<EntityId, Unification>,
    minted_ids: BTreeSet<EntityId>,
    entities: BTreeMap<EntityId, Entity>,
    kind_votes: BTreeMap<EntityId, Vec<(EntityId, EntityKind)>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryViolation {
    DanglingCanonical(EntityId),
    ChainedUnification {
        minted: EntityId,
        via: EntityId,
    },
    KindConflict {
        entity: EntityId,
        a: EntityKind,
        b: EntityKind,
    },
}

impl Registry {
    pub fn declare(
        &mut self,
        canonical: EntityId,
        minted: EntityId,
        why: Unification,
    ) -> Result<(), String> {
        if canonical == minted {
            return Err(format!("cannot unify {:?} with itself", minted.0));
        }
        if canonical.0.trim().is_empty() || minted.0.trim().is_empty() {
            return Err("unification ids must not be blank".into());
        }
        match &why {
            Unification::Declared { reason, source }
                if !reason.trim().is_empty() && !source.trim().is_empty() => {}
            _ => return Err("distinct ids require a written reason and source".into()),
        }
        if let Some(existing) = self.aliases.get(&minted) {
            if *existing != canonical {
                return Err(format!(
                    "{:?} is already declared to resolve to {:?}; refusing to silently redeclare it to {:?}",
                    minted.0, existing.0, canonical.0
                ));
            }
            return Err(format!(
                "{:?} is already declared; refusing a duplicate unification",
                minted.0
            ));
        }
        self.aliases.insert(minted.clone(), canonical.clone());
        self.why.insert(minted.clone(), why);

        if let Some(mut moving) = self.entities.remove(&minted) {
            let votes = self.kind_votes.remove(&minted).unwrap_or_default();
            let home = self.resolve(&canonical).clone();
            let target = self.entities.entry(home.clone()).or_insert_with(|| Entity {
                id: home.clone(),
                names: Vec::new(),
                kind: moving.kind,
                witnesses: Vec::new(),
            });
            for name in moving.names.drain(..) {
                merge_name(&mut target.names, &name);
            }
            target.witnesses.append(&mut moving.witnesses);
            resort_witnesses(&mut target.witnesses);
            self.kind_votes.entry(home).or_default().extend(votes);
        }
        Ok(())
    }

    pub fn observe(&mut self, minted: EntityId, name: &str, kind: EntityKind, w: WitnessRef) {
        let already_minted = self.minted_ids.contains(&minted);
        self.minted_ids.insert(minted.clone());
        if already_minted {
            self.why
                .entry(minted.clone())
                .or_insert(Unification::SameId);
        }

        let home = self.resolve(&minted).clone();
        self.kind_votes
            .entry(home.clone())
            .or_default()
            .push((minted, kind));

        let entity = self.entities.entry(home.clone()).or_insert_with(|| Entity {
            id: home,
            names: Vec::new(),
            kind,
            witnesses: Vec::new(),
        });
        merge_name(&mut entity.names, name);
        entity.witnesses.push(w);
        resort_witnesses(&mut entity.witnesses);
    }

    pub fn resolve<'a>(&'a self, minted: &'a EntityId) -> &'a EntityId {
        self.aliases.get(minted).unwrap_or(minted)
    }

    pub fn get(&self, canonical: &EntityId) -> Option<&Entity> {
        self.entities.get(canonical)
    }

    pub fn entities(&self) -> impl Iterator<Item = &Entity> {
        self.entities.values()
    }

    pub fn why(&self, minted: &EntityId) -> Option<&Unification> {
        self.why.get(minted)
    }

    pub fn validate(&self) -> Vec<RegistryViolation> {
        let mut violations = Vec::new();

        let declared_canonicals: BTreeSet<&EntityId> = self.aliases.values().collect();
        for canonical in declared_canonicals {
            if !self.minted_ids.contains(canonical) {
                violations.push(RegistryViolation::DanglingCanonical(canonical.clone()));
            }
        }

        for (minted, canonical) in &self.aliases {
            if self.aliases.contains_key(canonical) {
                violations.push(RegistryViolation::ChainedUnification {
                    minted: minted.clone(),
                    via: canonical.clone(),
                });
            }
        }

        for (entity, votes) in &self.kind_votes {
            let Some((_, first_kind)) = votes.first() else {
                continue;
            };
            if let Some((_, other_kind)) = votes.iter().find(|(_, k)| k != first_kind) {
                violations.push(RegistryViolation::KindConflict {
                    entity: entity.clone(),
                    a: *first_kind,
                    b: *other_kind,
                });
            }
        }

        violations
    }
}

fn witness_sort_key(w: &WitnessRef) -> (Witness, LayerKind, EntityId) {
    (w.witness, w.layer, w.minted_as.clone())
}

fn resort_witnesses(witnesses: &mut [WitnessRef]) {
    witnesses.sort_by(|a, b| witness_sort_key(a).cmp(&witness_sort_key(b)));
}

fn merge_name(names: &mut Vec<String>, name: &str) {
    if !names.iter().any(|n| n == name) {
        names.push(name.to_string());
        names.sort();
    }
}

#[cfg(test)]
mod laws {
    use super::*;

    const GENERATED_IDS: usize = 32;

    #[test]
    fn every_generated_id_keeps_its_identity_when_names_collide() {
        for name in ["", "Dan", "dan", "Dan ", "same coordinates"] {
            let mut registry = Registry::default();
            let ids: Vec<_> = (0..GENERATED_IDS).map(id).collect();
            for minted in &ids {
                registry.observe(minted.clone(), name, EntityKind::Place, witness(minted));
            }
            let resolved: Vec<_> = ids
                .iter()
                .map(|minted| registry.resolve(minted).clone())
                .collect();
            assert_eq!(resolved, ids, "names cannot change any minted identity");
            let live: Vec<_> = registry
                .entities()
                .map(|entity| entity.id.clone())
                .collect();
            assert_eq!(live, ids, "every distinct id has exactly one live entity");
            assert_eq!(
                registry.validate(),
                vec![],
                "independent identities satisfy registry laws"
            );
        }
    }

    #[test]
    fn every_generated_alias_resolves_to_one_live_home() {
        for home in (0..GENERATED_IDS).map(id) {
            let mut registry = Registry::default();
            registry.observe(
                home.clone(),
                "Shared name",
                EntityKind::Place,
                witness(&home),
            );
            let aliases: Vec<_> = (0..GENERATED_IDS)
                .map(|n| EntityId(format!("alias:{n:02}")))
                .collect();
            for alias in &aliases {
                registry
                    .declare(home.clone(), alias.clone(), reason())
                    .expect("generated declaration");
                registry.observe(
                    alias.clone(),
                    "Shared name",
                    EntityKind::Place,
                    witness(alias),
                );
            }
            let resolved: Vec<_> = aliases
                .iter()
                .map(|alias| registry.resolve(alias).clone())
                .collect();
            assert_eq!(
                resolved,
                vec![home.clone(); GENERATED_IDS],
                "all aliases resolve directly to their written home"
            );
            let live: Vec<_> = registry
                .entities()
                .map(|entity| entity.id.clone())
                .collect();
            assert_eq!(
                live,
                vec![home],
                "aliases do not create additional live ids"
            );
            assert_eq!(
                registry.validate(),
                vec![],
                "all written homes are live and unchained"
            );
        }
    }

    #[test]
    fn a_repeated_alias_cannot_replace_its_written_justification() {
        for minted in (0..GENERATED_IDS).map(id) {
            let mut registry = Registry::default();
            let home = EntityId("home".into());
            registry
                .declare(home.clone(), minted.clone(), reason())
                .expect("first declaration");
            let before = registry.clone();
            let actual = registry.declare(
                home,
                minted.clone(),
                Unification::Declared {
                    reason: "replacement".into(),
                    source: "other source".into(),
                },
            );
            assert_eq!(
                actual,
                Err(format!(
                    "{:?} is already declared; refusing a duplicate unification",
                    minted.0
                )),
                "each alias has exactly one declaration"
            );
            assert_eq!(
                registry, before,
                "a duplicate cannot replace the recorded reason"
            );
        }
    }

    #[test]
    fn different_ids_require_a_written_reason_and_source() {
        for why in [
            Unification::SameId,
            Unification::Declared {
                reason: " ".into(),
                source: "source".into(),
            },
            Unification::Declared {
                reason: "reason".into(),
                source: " ".into(),
            },
        ] {
            let mut registry = Registry::default();
            let actual = registry.declare(id(0), id(1), why);
            assert_eq!(
                actual,
                Err("distinct ids require a written reason and source".into()),
                "identity is never inferred without provenance"
            );
            assert_eq!(
                registry,
                Registry::default(),
                "an unjustified declaration leaves no alias"
            );
        }
    }

    #[test]
    fn every_generated_alias_refuses_an_unobserved_home() {
        for home in (0..GENERATED_IDS).map(id) {
            let mut registry = Registry::default();
            let alias = EntityId("alias".into());
            registry
                .declare(home.clone(), alias.clone(), reason())
                .expect("written declaration");
            registry.observe(
                alias.clone(),
                "Shared name",
                EntityKind::Place,
                witness(&alias),
            );
            assert_eq!(
                registry.validate(),
                vec![RegistryViolation::DanglingCanonical(home)],
                "an alias witness cannot prove that its declared home was minted"
            );
        }
    }

    #[test]
    fn blank_ids_cannot_enter_the_alias_table() {
        for (home, minted) in [(" ", "alias"), ("home", " ")] {
            let mut registry = Registry::default();
            let actual = registry.declare(EntityId(home.into()), EntityId(minted.into()), reason());
            assert_eq!(
                actual,
                Err("unification ids must not be blank".into()),
                "each declared identity must name an id"
            );
            assert_eq!(
                registry,
                Registry::default(),
                "blank ids cannot leave a declaration behind"
            );
        }
    }

    fn id(index: usize) -> EntityId {
        EntityId(format!("id:{index:02}"))
    }

    fn witness(minted: &EntityId) -> WitnessRef {
        WitnessRef {
            minted_as: minted.clone(),
            witness: Witness::Atlas,
            layer: LayerKind::ScriptureClaims,
            kind: "point",
        }
    }

    fn reason() -> Unification {
        Unification::Declared {
            reason: "Explicit test identity decision".into(),
            source: "generated registry laws".into(),
        }
    }
}
