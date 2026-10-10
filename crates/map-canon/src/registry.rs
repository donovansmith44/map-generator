use std::collections::{BTreeMap, BTreeSet};

use crate::{EntityId, GeometryKind, LayerKind, Witness};

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Deserialize,
    serde::Serialize,
    strum::EnumIter,
)]
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
pub enum DeclarationRefusal {
    SelfUnification(EntityId),
    BlankIdentity,
    MissingProvenance,
    Duplicate(EntityId),
    ConflictingHome {
        minted: EntityId,
        existing: EntityId,
        proposed: EntityId,
    },
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
    pub kind: GeometryKind,
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
    ) -> Result<(), DeclarationRefusal> {
        if canonical == minted {
            return Err(DeclarationRefusal::SelfUnification(minted));
        }
        if canonical.0.trim().is_empty() || minted.0.trim().is_empty() {
            return Err(DeclarationRefusal::BlankIdentity);
        }
        match &why {
            Unification::Declared { reason, source }
                if !reason.trim().is_empty() && !source.trim().is_empty() => {}
            _ => return Err(DeclarationRefusal::MissingProvenance),
        }
        if let Some(existing) = self.aliases.get(&minted) {
            if *existing != canonical {
                return Err(DeclarationRefusal::ConflictingHome {
                    minted,
                    existing: existing.clone(),
                    proposed: canonical,
                });
            }
            return Err(DeclarationRefusal::Duplicate(minted));
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
    use proptest::prelude::*;
    use strum::IntoEnumIterator;

    const MAX_DECLARATIONS: usize = 12;

    proptest! {
        #[test]
        fn generated_identities_stay_distinct_when_names_collide(
            tokens in proptest::collection::btree_set("[a-z0-9]{1,12}", 1..MAX_DECLARATIONS),
            name in any::<String>(),
        ) {
            let mut registry = Registry::default();
            let ids: Vec<_> = tokens
                .iter()
                .map(|token| EntityId(format!("id:{token}")))
                .collect();
            for minted in &ids {
                registry.observe(minted.clone(), &name, EntityKind::Place, witness(minted));
            }
            let resolved: Vec<_> = ids.iter().map(|id| registry.resolve(id).clone()).collect();
            prop_assert_eq!(
                resolved,
                ids.clone(),
                "names cannot change any minted identity"
            );
            let live: Vec<_> = registry
                .entities()
                .map(|entity| entity.id.clone())
                .collect();
            prop_assert_eq!(live, ids, "every distinct id has one live entity");
            prop_assert_eq!(
                registry.validate(),
                vec![],
                "independent identities satisfy the registry laws"
            );
        }

        #[test]
        fn generated_aliases_share_one_written_live_home(
            tokens in proptest::collection::btree_set("[a-z0-9]{1,12}", 1..MAX_DECLARATIONS),
            reason in written_text(),
            source in written_text(),
            before in any::<bool>(),
        ) {
            let mut registry = Registry::default();
            let home = EntityId("home".into());
            registry.observe(
                home.clone(),
                "Shared name",
                EntityKind::Place,
                witness(&home),
            );
            let aliases: Vec<_> = tokens
                .iter()
                .map(|token| EntityId(format!("alias:{token}")))
                .collect();
            for minted in &aliases {
                if before {
                    registry.observe(
                        minted.clone(),
                        "Shared name",
                        EntityKind::Place,
                        witness(minted),
                    );
                }
                registry
                    .declare(
                        home.clone(),
                        minted.clone(),
                        Unification::Declared {
                            reason: reason.clone(),
                            source: source.clone(),
                        },
                    )
                    .expect("written identity decision");
                if !before {
                    registry.observe(
                        minted.clone(),
                        "Shared name",
                        EntityKind::Place,
                        witness(minted),
                    );
                }
            }
            let resolved: Vec<_> = aliases
                .iter()
                .map(|id| registry.resolve(id).clone())
                .collect();
            prop_assert_eq!(
                resolved,
                vec![home.clone(); aliases.len()],
                "all aliases retain their written home"
            );
            let live: Vec<_> = registry
                .entities()
                .map(|entity| entity.id.clone())
                .collect();
            prop_assert_eq!(
                live,
                vec![home],
                "declared aliases create no additional live identities"
            );
            prop_assert_eq!(
                registry.validate(),
                vec![],
                "all written homes are live and unchained"
            );
        }

        #[test]
        fn generated_duplicate_aliases_preserve_the_entire_registry(
            token in "[a-z0-9]{1,24}",
            reason in written_text(),
            source in written_text(),
            replacement in written_text(),
        ) {
            let minted = EntityId(format!("alias:{token}"));
            let home = EntityId(format!("home:{token}"));
            let mut registry = populated(&home, &minted, &reason, &source);
            let before = registry.clone();
            let actual = registry.declare(
                home,
                minted.clone(),
                Unification::Declared {
                    reason: replacement,
                    source,
                },
            );
            prop_assert_eq!(
                actual,
                Err(DeclarationRefusal::Duplicate(minted)),
                "a duplicate alias has a closed refusal"
            );
            prop_assert_eq!(
                registry,
                before,
                "a duplicate preserves every recorded identity and justification"
            );
        }

        #[test]
        fn generated_conflicting_homes_preserve_the_entire_registry(
            token in "[a-z0-9]{1,24}",
            reason in written_text(),
            source in written_text(),
        ) {
            let minted = EntityId(format!("alias:{token}"));
            let home = EntityId(format!("home:{token}"));
            let proposed = EntityId(format!("other:{token}"));
            let mut registry = populated(&home, &minted, &reason, &source);
            let before = registry.clone();
            let actual = registry.declare(
                proposed.clone(),
                minted.clone(),
                Unification::Declared { reason, source },
            );
            prop_assert_eq!(
                actual,
                Err(DeclarationRefusal::ConflictingHome {
                    minted,
                    existing: home,
                    proposed
                }),
                "conflicting homes name the complete refused decision"
            );
            prop_assert_eq!(
                registry,
                before,
                "a conflicting home preserves the entire registry"
            );
        }

        #[test]
        fn generated_self_unifications_have_a_typed_refusal(token in "[a-z0-9]{1,24}") {
            let minted = EntityId(format!("id:{token}"));
            let mut registry = Registry::default();
            let before = registry.clone();
            let actual = registry.declare(minted.clone(), minted.clone(), Unification::SameId);
            prop_assert_eq!(
                actual,
                Err(DeclarationRefusal::SelfUnification(minted)),
                "self unification names the refused identity"
            );
            prop_assert_eq!(
                registry,
                before,
                "self unification preserves the entire registry"
            );
        }

        #[test]
        fn generated_blank_ids_never_change_the_registry(
            token in "[a-z0-9]{1,24}",
            blank in blank_text(),
            reason in written_text(),
            source in written_text(),
        ) {
            for (home, minted) in [
                (EntityId(blank.clone()), EntityId(format!("alias:{token}"))),
                (EntityId(format!("home:{token}")), EntityId(blank.clone())),
            ] {
                let mut registry = Registry::default();
                let before = registry.clone();
                let actual = registry.declare(
                    home,
                    minted,
                    Unification::Declared {
                        reason: reason.clone(),
                        source: source.clone(),
                    },
                );
                prop_assert_eq!(
                    actual,
                    Err(DeclarationRefusal::BlankIdentity),
                    "both identities must be nonblank"
                );
                prop_assert_eq!(
                    registry,
                    before,
                    "a blank identity leaves the registry unchanged"
                );
            }
        }

        #[test]
        fn generated_missing_provenance_never_changes_the_registry(
            token in "[a-z0-9]{1,24}",
            blank in blank_text(),
            reason in written_text(),
            source in written_text(),
        ) {
            for why in [
                Unification::SameId,
                Unification::Declared {
                    reason: blank.clone(),
                    source: source.clone(),
                },
                Unification::Declared {
                    reason: reason.clone(),
                    source: blank.clone(),
                },
            ] {
                let mut registry = Registry::default();
                let before = registry.clone();
                let actual = registry.declare(
                    EntityId(format!("home:{token}")),
                    EntityId(format!("alias:{token}")),
                    why,
                );
                prop_assert_eq!(
                    actual,
                    Err(DeclarationRefusal::MissingProvenance),
                    "a distinct identity requires a written reason and source"
                );
                prop_assert_eq!(
                    registry,
                    before,
                    "missing provenance leaves the registry unchanged"
                );
            }
        }

        #[test]
        fn generated_alias_witnesses_cannot_prove_a_live_home(
            token in "[a-z0-9]{1,24}",
            reason in written_text(),
            source in written_text(),
        ) {
            let home = EntityId(format!("home:{token}"));
            let minted = EntityId(format!("alias:{token}"));
            let mut registry = Registry::default();
            registry
                .declare(
                    home.clone(),
                    minted.clone(),
                    Unification::Declared { reason, source },
                )
                .expect("written identity decision");
            registry.observe(
                minted.clone(),
                "Shared name",
                EntityKind::Place,
                witness(&minted),
            );
            prop_assert_eq!(
                registry.validate(),
                vec![RegistryViolation::DanglingCanonical(home)],
                "an alias does not prove that its written home was minted"
            );
        }

        #[test]
        fn generated_witness_kind_conflicts_keep_both_votes(token in "[a-z0-9]{1,24}") {
            for first in EntityKind::iter() {
                for second in EntityKind::iter().filter(|kind| *kind != first) {
                    let minted = EntityId(format!("id:{token}"));
                    let mut registry = Registry::default();
                    registry.observe(minted.clone(), "Shared name", first, witness(&minted));
                    registry.observe(minted.clone(), "Shared name", second, witness(&minted));
                    prop_assert_eq!(
                        registry.validate(),
                        vec![RegistryViolation::KindConflict {
                            entity: minted,
                            a: first,
                            b: second
                        }],
                        "conflicting witnesses retain both entity kinds"
                    );
                }
            }
        }
    }

    fn written_text() -> impl Strategy<Value = String> {
        "[a-zA-Z][a-zA-Z0-9 .:'\\-]{0,48}"
    }

    fn blank_text() -> impl Strategy<Value = String> {
        proptest::collection::vec(
            proptest::sample::select(vec![' ', '\t', '\n', '\r', '\u{2003}']),
            0..16,
        )
        .prop_map(|chars| chars.into_iter().collect())
    }

    fn witness(minted: &EntityId) -> WitnessRef {
        WitnessRef {
            minted_as: minted.clone(),
            witness: Witness::Atlas,
            layer: LayerKind::ScriptureClaims,
            kind: GeometryKind::Point,
        }
    }

    fn populated(home: &EntityId, minted: &EntityId, reason: &str, source: &str) -> Registry {
        let mut registry = Registry::default();
        registry.observe(home.clone(), "Home", EntityKind::Place, witness(home));
        registry
            .declare(
                home.clone(),
                minted.clone(),
                Unification::Declared {
                    reason: reason.into(),
                    source: source.into(),
                },
            )
            .expect("written identity decision");
        registry.observe(minted.clone(), "Alias", EntityKind::Place, witness(minted));
        registry
    }
}
