//! THE ENTITY REGISTRY (spec §2 equation 1, 2026-09-07): "no witness
//! mints an entity; every witness references the registry." Today six
//! id namespaces (`basemap:`, `natural-earth:`, `etopo:`, `partition:`,
//! `place:`, `authored:`, plus bare atlas ids) each mint their own
//! `EntityId`, and the live census proves the consequence — Phoenicia
//! is both `partition:phoenicia` and `phoenicia`; Dan is both
//! `partition:dan` and `place:dan`; Judea is both `basemap:judea` and
//! `authored:judea`.
//!
//! This module fixes identity, not names. Every minted id stands for
//! itself until a person writes a `Unification` explaining why two
//! minted ids are one real thing. Name equality — slugified or not —
//! has NO authority here; that discipline is what replaces the
//! `bg_shadows` slug matcher.
//!
//! Laws upheld here (see tests.rs, written first):
//! - `resolve` is TOTAL and ONE-HOP: an unknown id resolves to itself,
//!   and a declared alias is never chased through a second hop — a
//!   chain is a data error, named by `validate`, not silently chased;
//! - `declare` records a reason; it never infers one. The only reason
//!   `observe` itself ever writes is `Unification::SameId`, and only
//!   when it sees a minted id it has already seen;
//! - `observe` merges names into a sorted, deduped list and witnesses
//!   into a list sorted by (witness, layer, minted_as) — the entity's
//!   byte form is a pure function of what it has seen, never of
//!   insertion order;
//! - `validate` returns EVERY violation it finds, never just the
//!   first — a validator the owner must run twenty times to see all
//!   twenty typos is not a validator;
//! - no slugify, no name normalization for matching anywhere in this
//!   file. Name equality is not identity.

use std::collections::{BTreeMap, BTreeSet};

use crate::{EntityId, LayerKind, Witness};

/// WHAT KIND of real thing an entity is — coarse enough to catch a
/// witness disagreement (a promise mistaken for a polity), fine enough
/// to matter to rendering.
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

/// WHY two minted ids are one entity. A written, typed reason — never
/// a string coincidence. `bg_shadows` matched slugs; this does not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unification {
    /// Two witnesses happened to mint the exact same `EntityId` — not
    /// a claim about the world, just the trivial case of one id
    /// appearing twice. `observe` is the ONLY producer of this variant.
    SameId,
    /// A person looked at both minted ids and said, in their own
    /// words, why they are the same real thing — and where that
    /// judgment is written down for review.
    Declared { reason: String, source: String },
}

/// One witness's testimony to one minted entity: which id it minted,
/// which witness spoke, which layer it spoke into, and what kind of
/// feature it drew (the geometry kind — "area", "way", "point", "line",
/// "memory" — as in `CensusRow::kind`, distinct from `EntityKind`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WitnessRef {
    pub minted_as: EntityId,
    pub witness: Witness,
    pub layer: LayerKind,
    pub kind: &'static str,
}

/// One real thing: its canonical id, every name it has been called,
/// what kind of thing it is, and every witness that has spoken to it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entity {
    pub id: EntityId,
    pub names: Vec<String>,
    pub kind: EntityKind,
    pub witnesses: Vec<WitnessRef>,
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

/// Every fact the registry has been told, keyed so `resolve` can be
/// total and O(1), `get` can hand back a live `Entity`, and `validate`
/// can name every violation without re-deriving anything.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Registry {
    /// minted -> canonical, written ONLY by `declare`. One hop: never
    /// chased, never rewritten to skip an intermediate.
    aliases: BTreeMap<EntityId, EntityId>,
    /// minted -> the reason it unifies with something. Written by
    /// `declare` (a `Declared` reason) or by `observe` (`SameId`, and
    /// only on a repeat sighting of the identical minted id).
    why: BTreeMap<EntityId, Unification>,
    /// Every raw id any witness has ever minted via `observe` — the
    /// set `validate` checks a declared canonical against, to catch a
    /// canonical id that is a typo nothing ever minted.
    minted_ids: BTreeSet<EntityId>,
    /// home id -> the materialized entity living there right now.
    /// "Home" starts as the id itself and moves to a declared
    /// canonical the moment a `declare` call merges it in.
    entities: BTreeMap<EntityId, Entity>,
    /// home id -> every (minted_as, kind) vote cast for it, kept
    /// alongside `entities` purely so `validate` can name a
    /// `KindConflict` without `Entity` needing a second kind field.
    kind_votes: BTreeMap<EntityId, Vec<(EntityId, EntityKind)>>,
}

/// A law the registry's own data broke — named, not silently fixed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryViolation {
    /// A declared canonical id nothing ever minted — a typo in the data file.
    DanglingCanonical(EntityId),
    /// A -> B -> C: resolution must be one hop, so the data says so directly.
    ChainedUnification { minted: EntityId, via: EntityId },
    /// Two witnesses of one entity disagree about its kind.
    KindConflict { entity: EntityId, a: EntityKind, b: EntityKind },
}

impl Registry {
    /// Declare that `minted` resolves to `canonical`, for the written
    /// `why`. Never infers a reason; never chases `canonical` through
    /// any alias it might already have (that is exactly how a chain
    /// gets built, and `validate` is what catches it — `declare`
    /// itself only refuses the two self-contradictory cases: unifying
    /// an id with itself, and silently redeclaring an id's canonical
    /// to something new).
    pub fn declare(
        &mut self,
        canonical: EntityId,
        minted: EntityId,
        why: Unification,
    ) -> Result<(), String> {
        if canonical == minted {
            return Err(format!("cannot unify {:?} with itself", minted.0));
        }
        if let Some(existing) = self.aliases.get(&minted) {
            if *existing != canonical {
                return Err(format!(
                    "{:?} is already declared to resolve to {:?}; refusing to silently redeclare it to {:?}",
                    minted.0, existing.0, canonical.0
                ));
            }
        }
        self.aliases.insert(minted.clone(), canonical.clone());
        self.why.insert(minted.clone(), why);

        if let Some(mut moving) = self.entities.remove(&minted) {
            let votes = self.kind_votes.remove(&minted).unwrap_or_default();
            // Merge into wherever `canonical` ITSELF currently resolves
            // (one hop through the table just updated above), never into
            // the raw `canonical` key directly. Using the raw key would
            // resurrect a home a PRIOR declare already emptied and
            // relocated — e.g. after declare(phoenicia, partition:phoenicia)
            // has moved partition:phoenicia's entity into `phoenicia`, a
            // later declare(partition:phoenicia, third:phoenicia) must
            // land third:phoenicia's testimony in `phoenicia` too, not
            // resurrect an orphaned entity at the now-empty
            // `partition:phoenicia` key. `aliases` itself still stores the
            // RAW canonical (below `resolve` sees it and `validate` names
            // the resulting chain) — only the merge TARGET is resolved.
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

    /// Record one witness's testimony: `minted` is called `name`, is a
    /// `kind` of thing, and here is the `WitnessRef` describing who
    /// said so. Names merge into a sorted, deduped list; witnesses
    /// merge into a list sorted by (witness, layer, minted_as) — the
    /// entity's byte form never depends on call order.
    ///
    /// If `minted` has already been declared to resolve elsewhere,
    /// this testimony joins that entity directly (one hop, same as
    /// `resolve`). If `minted` has been seen before under this exact
    /// id (no declaration involved — just the same id minted twice),
    /// the trivial `Unification::SameId` reason is recorded for it,
    /// UNLESS a written reason already stands.
    pub fn observe(&mut self, minted: EntityId, name: &str, kind: EntityKind, w: WitnessRef) {
        let already_minted = self.minted_ids.contains(&minted);
        self.minted_ids.insert(minted.clone());
        if already_minted {
            self.why.entry(minted.clone()).or_insert(Unification::SameId);
        }

        let home = self.resolve(&minted).clone();
        self.kind_votes.entry(home.clone()).or_default().push((minted, kind));

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

    /// TOTAL, ONE-HOP resolution: an id nobody ever declared an alias
    /// for resolves to itself, so no caller ever handles a case it
    /// cannot act on.
    pub fn resolve<'a>(&'a self, minted: &'a EntityId) -> &'a EntityId {
        self.aliases.get(minted).unwrap_or(minted)
    }

    /// The entity living at `canonical`, if anything has ever observed
    /// or been declared into it.
    pub fn get(&self, canonical: &EntityId) -> Option<&Entity> {
        self.entities.get(canonical)
    }

    /// Every entity the registry currently holds, home ids only.
    pub fn entities(&self) -> impl Iterator<Item = &Entity> {
        self.entities.values()
    }

    /// The written reason `minted` unifies with whatever it resolves
    /// to — `None` if nothing has ever unified it with anything.
    pub fn why(&self, minted: &EntityId) -> Option<&Unification> {
        self.why.get(minted)
    }

    /// Every violation the current data commits — never just the
    /// first, so the owner sees the whole list in one run.
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
            let Some((_, first_kind)) = votes.first() else { continue };
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
