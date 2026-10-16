use std::collections::BTreeSet;

use atlas_graph_types::canon::{Canon, DecodeOn};
use atlas_graph_types::edge::{Ground, Justification, Holder, HoldingState, HoldHow};
use atlas_graph_types::id::{PlaceId, PolityId};
use atlas_graph_types::passage::{Reading, Readings, Written};
use atlas_graph_types::place_versions::{Basis, VersionDate};
use atlas_graph_types::map_timing::{ChangeRowKind, RecordedMapCause, SourceChangeDating};
use atlas_graph_types::text::{BibleTag, ConcordTag, VerseRef};
use proptest::prelude::*;

use crate::atlas_transition::{ArcChange, ArcRef, FaceHolding, FaceTransfer, RecordedMapChange, TransitionBoundary};
use crate::boundary::{Orientation, RegionGeom, RegionPart};
use crate::geom::UnitVec;
use crate::transition::{TransitionDraw, TransitionScript, TransitionStep};

proptest! {
    #[test]
    fn stored_changes_roundtrip_only_on_their_admitted_reading(
        seed in any::<u32>(), holding_case in 0u8..3, year in -4004i32..100, first in 1u16..40, count in 1u16..10,
        wording in "[a-z ]{1,30}", lat in -80.0f64..80.0, lon in -170.0f64..170.0) {
        prop_assume!(year != 0);
        let units: Vec<VerseRef> = (first..first+count).map(|verse| VerseRef { book: 1, chapter: 1, verse }).collect();
        let readings = Readings { bible: Reading::<BibleTag>::of(units.clone()).unwrap(), concord: Reading::<ConcordTag>::of(vec![]).unwrap() };
        let admitted = readings.bible.place(&Written::new(units[0].clone(), units[units.len()-1].clone()).unwrap()).unwrap();
        let basis = Basis { reading: Justification { text: Some(wording.clone()), grounds: BTreeSet::from([Ground::Scripture(admitted)]) }, alternatives: vec![], provenance: format!("source-{seed}") };
        let arc = ArcRef { store: format!("store-{seed}"), key: format!("arc-{seed}") };
        let node = PlaceId::new(format!("place-{seed}")).erase();
        let change = RecordedMapChange {
            year, before_year: -4004, causes: vec![RecordedMapCause { kind: ChangeRowKind::FirstGrounded, dating: SourceChangeDating::Version(VersionDate::Year(year)), basis: basis.clone() }],
            arcs: vec![ArcChange { arc: arc.clone(), before_shape: None, after_shape: Some(format!("shape-{seed}")) }],
            transfers: vec![FaceTransfer { subset: format!("subset-{seed}"), boundary: RegionGeom { parts: vec![RegionPart { cycle: vec![crate::boundary::OrientedBoundary { boundary: arc, orientation: Orientation::Forward }], holes: vec![] }] }, from: None, to: Some(FaceHolding { territory: node.clone(), holding: match holding_case { 0 => None, 1 => Some(HoldingState::Unclaimed), _ => Some(HoldingState::Held { holder: Holder::Polity(PolityId::new(format!("holder-{seed}"))), how: HoldHow::Governs }) } }), basis }],
            script: TransitionScript { steps: vec![TransitionDraw { step: TransitionStep::PointTrack {
                entity: node.clone(),
                from: crate::transition::PointTrackEndpoint { at: UnitVec::from_lat_lon_deg(lat, lon), extent: crate::transition::PlaceExtentKind::Point, radius_m: 0 },
                to: crate::transition::PointTrackEndpoint { at: UnitVec::from_lat_lon_deg(lat + 0.1, lon + 0.1), extent: crate::transition::PlaceExtentKind::Vicinity, radius_m: seed },
            }, clip: Some(format!("subset-{seed}")) }] },
        };
        let mut change = change;
        let path = RegionGeom { parts: vec![RegionPart { cycle: vec![crate::boundary::OrientedBoundary { boundary: ArcRef { store: format!("store-{seed}"), key: format!("outer-{seed}") }, orientation: Orientation::Forward }], holes: vec![vec![crate::boundary::OrientedBoundary { boundary: ArcRef { store: format!("store-{seed}"), key: format!("hole-{seed}") }, orientation: Orientation::Reverse }]] }] };
        change.script.steps.push(TransitionStep::Morph { entity: node.clone(), boundary: TransitionBoundary::Ring(path.clone()), from_pts: vec![UnitVec::from_lat_lon_deg(lat, lon); 3], to_pts: vec![UnitVec::from_lat_lon_deg(lat + 0.1, lon + 0.1); 3] }.into());
        change.script.steps.push(TransitionStep::SegmentFade { entity: node.clone(), segment: TransitionBoundary::Arc(ArcRef { store: format!("store-{seed}"), key: format!("complete-{seed}") }), course: vec![UnitVec::from_lat_lon_deg(lat, lon), UnitVec::from_lat_lon_deg(lat + 0.1, lon + 0.1)], appears: true }.into());
        let bytes = change.encode();
        let restored = RecordedMapChange::decode_on(&bytes, &readings).expect("compiled grounds are admitted on the same reading");
        prop_assert_eq!(&restored, &change, "stored change retains geometry, endpoints, identity and exact Scripture grounds");
        prop_assert_eq!(&restored.script.steps[1].step, &change.script.steps[1].step, "whole-ring correspondence retains its exact oriented outer path and hole");
        prop_assert_eq!(restored.encode(), bytes.clone(), "one stored change has one canonical byte spelling");
        let missing = Readings { bible: Reading::<BibleTag>::of(vec![]).unwrap(), concord: Reading::<ConcordTag>::of(vec![]).unwrap() };
        prop_assert!(RecordedMapChange::decode_on(&bytes, &missing).is_err(), "Scripture proof cannot be decoded on another reading");
        let mut malformed = change.clone();
        malformed.script.steps.push(TransitionStep::Morph {
            entity: node.clone(),
            boundary: TransitionBoundary::Arc(ArcRef { store: format!("store-{seed}"), key: format!("arc-{seed}") }),
            from_pts: vec![UnitVec::from_lat_lon_deg(lat, lon); 2],
            to_pts: vec![UnitVec::from_lat_lon_deg(lat, lon); 1],
        }.into());
        let malformed_result = RecordedMapChange::decode_on(&malformed.encode(), &readings);
        prop_assert!(malformed_result.is_err(), "stored mismatched tracks cannot bypass canonical script admission");
        prop_assert_eq!(malformed_result.unwrap_err().path, "$.script", "a malformed stored script reports its exact field");
        let mut missing_field = change.to_value();
        if let atlas_graph_types::canon::Value::Obj(fields) = &mut missing_field { fields.remove("year"); }
        let missing_result = RecordedMapChange::from_value_on(&missing_field, &readings);
        prop_assert!(missing_result.is_err(), "stored changes refuse a missing recorded boundary year");
    }
}

#[cfg(feature = "openapi")]
proptest! {
    #[test]
    fn refusal_shapes_remain_in_tool_diagnostics_without_entering_the_public_contract(
        year in any::<i32>(), shape in "[a-z][a-z0-9-]{1,30}") {
        // Arrange
        use crate::atlas_transition::TransitionRefusal;
        let refusals = [
            ("missing-correspondence", TransitionRefusal::MissingCorrespondence { year, shape: shape.clone() }),
            ("unclassified-subset", TransitionRefusal::UnclassifiedSubset { year, shape: shape.clone() }),
        ];
        // Act
        let schema = serde_json::to_value(<TransitionRefusal as utoipa::PartialSchema>::schema()).unwrap();
        // Assert
        for (reason, refusal) in refusals {
            prop_assert_eq!(serde_json::to_value(&refusal).unwrap(), serde_json::json!({ "reason": reason, "year": year }), "the public refusal retains its category and source year without a compiler locator");
            prop_assert!(format!("{refusal:?}").contains(&shape), "the original locator remains available to compiler diagnostics");
            let case = schema["oneOf"].as_array().unwrap().iter().find(|case| case["properties"]["reason"]["enum"][0] == reason).expect("the original public refusal case remains in its source schema");
            prop_assert!(case["properties"].get("shape").is_none(), "the public source schema contains no compiler locator property");
            prop_assert!(!case["required"].as_array().unwrap().iter().any(|field| field == "shape"), "the public source schema does not require a compiler locator");
        }
    }
}
