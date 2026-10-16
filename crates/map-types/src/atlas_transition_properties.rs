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
        let units: Vec<VerseRef> = (first..first+count).map(|verse| VerseRef { book: atlas_graph_types::text::BookId::new(1).unwrap(), chapter: 1, verse }).collect();
        let readings = Readings { bible: Reading::<BibleTag>::of(units.clone()).unwrap(), concord: Reading::<ConcordTag>::of(vec![]).unwrap() };
        let admitted = readings.bible.place(&Written::new(units[0].clone(), units[units.len()-1].clone()).unwrap()).unwrap();
        let basis = Basis { reading: Justification { text: Some(wording.clone()), grounds: BTreeSet::from([Ground::Scripture(admitted)]) }, alternatives: vec![], provenance: format!("source-{seed}") };
        let hash = |offset: u32| atlas_graph_types::id::ContentHash::from_hex(&format!("{:032x}", u64::from(seed) + u64::from(offset))).unwrap();
        let shape = atlas_graph_types::map_slice::ShapeId::parse(&hash(0).hex()).unwrap();
        let arc = ArcRef { store: hash(0), key: hash(1) };
        let node = PlaceId::new(format!("place-{seed}")).erase();
        let change = RecordedMapChange {
            year, before_year: -4004, causes: vec![RecordedMapCause { kind: ChangeRowKind::FirstGrounded, dating: SourceChangeDating::Version(VersionDate::Year(year)), basis: basis.clone() }],
            arcs: vec![ArcChange { arc: arc.clone(), before_shape: None, after_shape: Some(shape) }],
            transfers: vec![FaceTransfer { subset: shape, boundary: RegionGeom { parts: vec![RegionPart { cycle: vec![crate::boundary::OrientedBoundary { boundary: arc, orientation: Orientation::Forward }], holes: vec![] }] }, from: None, to: Some(FaceHolding { territory: node.clone(), holding: match holding_case { 0 => None, 1 => Some(HoldingState::Unclaimed), _ => Some(HoldingState::Held { holder: Holder::Polity(PolityId::new(format!("holder-{seed}"))), how: HoldHow::Governs }) } }), basis }],
            script: TransitionScript { steps: vec![TransitionDraw { step: TransitionStep::PointTrack {
                entity: node.clone(),
                from: crate::transition::PointTrackEndpoint { at: UnitVec::from_lat_lon_deg(lat, lon), extent: crate::transition::PlaceExtentKind::Point, radius_m: 0 },
                to: crate::transition::PointTrackEndpoint { at: UnitVec::from_lat_lon_deg(lat + 0.1, lon + 0.1), extent: crate::transition::PlaceExtentKind::Vicinity, radius_m: seed },
            }, clip: Some(shape) }] },
        };
        let mut change = change;
        let path = RegionGeom { parts: vec![RegionPart { cycle: vec![crate::boundary::OrientedBoundary { boundary: ArcRef { store: hash(0), key: hash(2) }, orientation: Orientation::Forward }], holes: vec![vec![crate::boundary::OrientedBoundary { boundary: ArcRef { store: hash(0), key: hash(3) }, orientation: Orientation::Reverse }]] }] };
        change.script.steps.push(TransitionStep::Morph { entity: node.clone(), boundary: TransitionBoundary::Ring(path.clone()), from_pts: vec![UnitVec::from_lat_lon_deg(lat, lon); 3], to_pts: vec![UnitVec::from_lat_lon_deg(lat + 0.1, lon + 0.1); 3] }.into());
        change.script.steps.push(TransitionStep::SegmentFade { entity: node.clone(), segment: TransitionBoundary::Arc(ArcRef { store: hash(0), key: hash(4) }), course: vec![UnitVec::from_lat_lon_deg(lat, lon), UnitVec::from_lat_lon_deg(lat + 0.1, lon + 0.1)], appears: true }.into());
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
            boundary: TransitionBoundary::Arc(ArcRef { store: hash(0), key: hash(1) }),
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

#[cfg(feature = "openapi")]
proptest! {
    #[test]
    fn geometry_and_arc_handles_keep_their_original_string_values_and_named_schemas(
        seed in any::<u64>(), lon in -170.0f64..170.0, lat in -80.0f64..80.0) {
        // Arrange
        use atlas_graph_types::id::ContentHash;
        use atlas_graph_types::map_slice::{shape_key, Geometry, ShapeId};
        let spelling = format!("{seed:032x}");
        let arc = ArcRef { store: ContentHash::from_hex(&spelling).unwrap(), key: ContentHash::from_hex(&spelling).unwrap() };
        let shape = shape_key(&Geometry::Point([lon, lat]));
        // Act
        let arc_value = serde_json::to_value(&arc).unwrap();
        let shape_value = serde_json::to_value(shape).unwrap();
        let arc_schema = serde_json::to_value(<ArcRef as utoipa::PartialSchema>::schema()).unwrap();
        let change_schema = serde_json::to_value(<ArcChange as utoipa::PartialSchema>::schema()).unwrap();
        // Assert
        prop_assert_eq!(arc_value.clone(), serde_json::json!({ "store": spelling, "key": spelling }), "the original partition and arc hex spellings stay unchanged");
        prop_assert_eq!(serde_json::from_value::<ArcRef>(arc_value).unwrap(), arc, "the original arc identity roundtrips at its source");
        prop_assert_eq!(shape_value.clone(), serde_json::json!(shape.hex()), "the shape keeps its original lowercase hex spelling");
        prop_assert_eq!(serde_json::from_value::<ShapeId>(shape_value).unwrap(), shape, "the shape string returns to the same 128-bit identity");
        prop_assert!(serde_json::from_value::<ShapeId>(serde_json::json!(format!("{}0", shape.hex()))).is_err(), "an invalid-width source handle cannot enter a script");
        prop_assert_eq!(&arc_schema["properties"]["store"], &serde_json::json!({ "$ref": "#/components/schemas/ContentHash" }), "partition identity is named at the original ArcRef source");
        prop_assert_eq!(&arc_schema["properties"]["key"], &serde_json::json!({ "$ref": "#/components/schemas/ContentHash" }), "arc identity is named at the same source");
        prop_assert!(change_schema["properties"]["after_shape"]["anyOf"].as_array().unwrap().contains(&serde_json::json!({ "$ref": "#/components/schemas/ShapeId" })), "an optional changed shape retains the shared named geometry identity");
    }
}
