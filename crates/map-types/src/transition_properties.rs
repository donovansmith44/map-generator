use proptest::prelude::*;

use crate::algebra::Monoid;
use crate::geom::UnitVec;
use crate::transition::{TransitionDraw, TransitionScript, TransitionStep};

proptest! {
    #[test]
    fn generic_script_composition_preserves_exact_clip_and_presentation_order(
        region in any::<u64>(), boundary in any::<u32>(), clip in any::<u64>()) {
        let clip = atlas_graph_types::map_slice::ShapeId::parse(&format!("{clip:032x}")).unwrap();
        let first = TransitionDraw { step: TransitionStep::<u64,u32>::FadeIn { region }, clip: Some(clip.clone()) };
        let second = TransitionDraw { step: TransitionStep::<u64,u32>::Morph { entity: u64::from(boundary), boundary, from_pts: vec![UnitVec::from_lat_lon_deg(0.0,0.0)], to_pts: vec![UnitVec::from_lat_lon_deg(0.0,1.0)] }, clip: None };
        let left = TransitionScript { steps: vec![first.clone()] };
        let right = TransitionScript { steps: vec![second.clone()] };
        let joined = left.clone().combine(right);
        prop_assert_eq!(&joined.steps, &vec![first, second], "composition keeps every verb, its exact clip and served order");
        prop_assert_eq!(TransitionScript::empty().combine(left.clone()), left, "the empty script remains the composition identity");
    }
}

#[cfg(feature = "serde")]
proptest! {
    #[test]
    fn morph_serialization_retains_its_served_entity(
        entity in any::<u64>(), boundary in any::<u32>(), lat in -80.0f64..80.0, lon in -170.0f64..170.0) {
        // Arrange
        let body = serde_json::json!({ "step": "morph", "entity": entity, "boundary": boundary,
            "from_pts": vec![UnitVec::from_lat_lon_deg(lat, lon); 3],
            "to_pts": vec![UnitVec::from_lat_lon_deg(lat + 0.1, lon + 0.1); 3] });
        // Act
        let restored: TransitionStep<u64, u32> = serde_json::from_value(body).expect("the served morph retains its original entity");
        let mut encoded = serde_json::to_value(restored).expect("the admitted morph serializes");
        // Assert
        prop_assert_eq!(encoded["entity"].as_u64(), Some(entity), "the morph's entity is the served identity, independent of its boundary");
        encoded.as_object_mut().unwrap().remove("entity");
        prop_assert!(serde_json::from_value::<TransitionStep<u64, u32>>(encoded).is_err(), "a morph without its served entity cannot choose a presentation");
    }

    #[test]
    fn directed_paths_and_point_tracks_publish_named_fields(
        boundary in any::<u32>(), entity in any::<u64>(), radius in any::<u32>(),
        lat in -80.0f64..80.0, lon in -170.0f64..170.0) {
        // Arrange
        use crate::boundary::{Orientation, RegionGeom, RegionPart};
        use crate::transition::PlaceExtentKind;
        let from = UnitVec::from_lat_lon_deg(lat, lon);
        let to = UnitVec::from_lat_lon_deg(lat + 0.1, lon + 0.1);
        let region = RegionGeom { parts: vec![RegionPart { cycle: vec![crate::boundary::OrientedBoundary { boundary: boundary, orientation: Orientation::Forward }], holes: vec![vec![crate::boundary::OrientedBoundary { boundary: boundary, orientation: Orientation::Reverse }]] }] };
        let step = TransitionStep::<u64, u32>::PointTrack { entity, from: crate::transition::PointTrackEndpoint { at: from, extent: PlaceExtentKind::Point, radius_m: 0 }, to: crate::transition::PointTrackEndpoint { at: to, extent: PlaceExtentKind::Vicinity, radius_m: radius } };
        // Act
        let path = serde_json::to_value(&region).unwrap();
        let track = serde_json::to_value(&step).unwrap();
        // Assert
        prop_assert_eq!(&path["parts"][0]["cycle"][0], &serde_json::json!({ "boundary": boundary, "orientation": "Forward" }), "the original directed outer arc has named fields");
        prop_assert_eq!(&path["parts"][0]["holes"][0][0], &serde_json::json!({ "boundary": boundary, "orientation": "Reverse" }), "a hole retains its original reverse direction through the same record");
        prop_assert_eq!(&track["from"], &serde_json::json!({ "at": from, "extent": "Point", "radius_m": 0 }), "the origin point owns its original extent and radius");
        prop_assert_eq!(&track["to"], &serde_json::json!({ "at": to, "extent": "Vicinity", "radius_m": radius }), "the destination point owns its original extent and radius");
        prop_assert_eq!(serde_json::from_value::<RegionGeom<u32>>(path).unwrap(), region, "the named directed path roundtrips without changing topology");
        prop_assert_eq!(serde_json::from_value::<TransitionStep<u64, u32>>(track.clone()).unwrap(), step.clone(), "the named endpoints roundtrip every original point fact");
        let reversed = crate::transition::invert(TransitionScript { steps: vec![step.into()] });
        let reverse = serde_json::to_value(&reversed.steps[0].step).unwrap();
        prop_assert_eq!(&reverse["from"], &track["to"], "inversion moves the complete destination endpoint to the origin");
        prop_assert_eq!(&reverse["to"], &track["from"], "inversion moves the complete origin endpoint to the destination");
        prop_assert!(serde_json::from_value::<TransitionStep<u64, u32>>(serde_json::json!({ "step": "point-track", "entity": entity, "from": from, "to": to, "extent": ["Point", "Vicinity"], "radius_m": [0, radius] })).is_err(), "legacy positional point facts cannot bypass the named endpoint source");
    }

    #[test]
    fn unit_vector_serialization_roundtrips_only_admitted_points(lat in -89.0f64..89.0, lon in -179.0f64..179.0, invalid in 2.0f64..20.0) {
        let point = UnitVec::from_lat_lon_deg(lat,lon);
        let json = serde_json::to_string(&point).expect("an admitted point serializes");
        let restored: UnitVec = serde_json::from_str(&json).expect("an admitted point is accepted");
        prop_assert_eq!(restored, point, "the existing unit vector's exact coordinates survive serialization");
        let bad = format!("{{\"x\":{invalid},\"y\":0,\"z\":0}}");
        prop_assert!(serde_json::from_str::<UnitVec>(&bad).is_err(), "the source door refuses a non-unit vector instead of bypassing construction");
    }
}

#[cfg(feature = "canon-ids")]
proptest! {
    #[test]
    fn script_admission_keeps_matched_points_and_refuses_broken_tracks(
        boundary in any::<u32>(), clip in any::<u64>(),
        lat in -80.0f64..80.0, lon in -170.0f64..170.0, n in 2usize..12) {
        use crate::atlas_transition::{admit_script, TransitionRefusal};
        let clip = atlas_graph_types::map_slice::ShapeId::parse(&format!("{clip:032x}")).unwrap();
        let start = UnitVec::from_lat_lon_deg(lat, lon);
        let end = UnitVec::from_lat_lon_deg(lat + 0.1, lon + 0.1);
        let make = |to: Vec<UnitVec>| TransitionScript { steps: vec![TransitionDraw {
            step: TransitionStep::<u64,u32>::Morph { entity: u64::from(boundary), boundary, from_pts: vec![start; n], to_pts: to },
            clip: Some(clip.clone()),
        }] };
        let valid = make(vec![end; n]);
        prop_assert_eq!(admit_script(valid.clone()), Ok(valid), "admission preserves every matched endpoint and exact clip");
        prop_assert_eq!(admit_script(make(vec![end; n - 1])), Err(TransitionRefusal::EndpointMismatch), "unequal paired tracks are refused without resampling");

    }
}

proptest! {
    #[test]
    fn vector_constructors_refuse_nonfinite_coordinates_and_preserve_valid_controls(
        lat in -80.0f64..80.0, lon in -170.0f64..170.0, scale in 0.1f64..20.0, case in 0u8..3) {
        let invalid = match case { 0 => f64::NAN, 1 => f64::INFINITY, _ => f64::NEG_INFINITY };
        prop_assert!(UnitVec::new(invalid, 0.0, 0.0).is_err(), "the checked constructor refuses nonfinite coordinates");
        prop_assert!(UnitVec::normalize(invalid, 0.0, 0.0).is_err(), "normalization refuses nonfinite coordinates");
        prop_assert!(std::panic::catch_unwind(|| UnitVec::from_lat_lon_deg(invalid, lon)).is_err(), "the infallible geographic constructor refuses nonfinite latitude");
        prop_assert!(std::panic::catch_unwind(|| UnitVec::from_lat_lon_deg(lat, invalid)).is_err(), "the infallible geographic constructor refuses nonfinite longitude");
        let admitted = UnitVec::from_lat_lon_deg(lat, lon);
        let restored = UnitVec::new(admitted.x(), admitted.y(), admitted.z()).expect("a finite spherical point remains admitted");
        prop_assert_eq!(restored, admitted, "the checked constructor preserves valid source coordinates");
        let normalized = UnitVec::normalize(admitted.x() * scale, admitted.y() * scale, admitted.z() * scale).expect("finite scaled points normalize");
        prop_assert!((normalized.dot(&admitted) - 1.0).abs() < 1e-12, "normalization preserves the finite point direction");
    }
}

proptest! {
    #[test]
    fn generic_inversion_retains_references_clips_and_reverse_composition(
        entity in any::<u64>(), boundary in "[a-z]{1,20}", clip in "[a-z]{1,20}",
        lat in -70.0f64..70.0, lon in -160.0f64..160.0, radius in any::<u32>()) {
        // Arrange
        use crate::transition::{invert, PlaceExtentKind, PlacedLabel};
        let from = UnitVec::from_lat_lon_deg(lat, lon);
        let to = UnitVec::from_lat_lon_deg(lat + 0.1, lon + 0.1);
        let label = PlacedLabel { text: boundary.clone(), anchor: from };
        let steps = vec![
            TransitionStep::Morph { entity, boundary: boundary.clone(), from_pts: vec![from; 3], to_pts: vec![to; 3] },
            TransitionStep::FadeIn { region: entity },
            TransitionStep::FadeOut { region: entity },
            TransitionStep::SplitAlong { parent: entity, seam: vec![from, to], children: vec![entity] },
            TransitionStep::MergeAcross { parents: vec![entity], child: entity },
            TransitionStep::PointTrack { entity, from: crate::transition::PointTrackEndpoint { at: from, extent: PlaceExtentKind::Point, radius_m: 0 }, to: crate::transition::PointTrackEndpoint { at: to, extent: PlaceExtentKind::Vicinity, radius_m: radius } },
            TransitionStep::PointFade { entity, at: from, radius_m: radius, appears: true },
            TransitionStep::SegmentTrack { entity, segment: boundary.clone(), from: vec![from], to: vec![to] },
            TransitionStep::SegmentFade { entity, segment: boundary.clone(), course: vec![from, to], appears: true },
            TransitionStep::LabelTrack { entity, text: boundary.clone(), from, to },
            TransitionStep::LabelFade { entity, from: Some(label), to: None },
        ].into_iter().enumerate().map(|(index, step)| TransitionDraw { step, clip: Some(format!("{clip}-{index}")) }).collect::<Vec<_>>();
        let original = TransitionScript { steps };
        let left = TransitionScript { steps: original.steps[..5].to_vec() };
        let right = TransitionScript { steps: original.steps[5..].to_vec() };
        // Act
        let reversed: TransitionScript<u64, String> = invert(original.clone());
        let combined = invert(right).combine(invert(left));
        // Assert
        prop_assert_eq!(&reversed, &combined, "reversal composes in reverse order across every existing canonical step");
        prop_assert_eq!(reversed.steps.len(), original.steps.len(), "every canonical step remains present");
        let expected_clips = original.steps.iter().rev().map(|draw| draw.clip.clone()).collect::<Vec<_>>();
        let actual_clips = reversed.steps.iter().map(|draw| draw.clip.clone()).collect::<Vec<_>>();
        prop_assert_eq!(actual_clips, expected_clips, "the original clip stays with its own reversed step");
        let TransitionStep::Morph { entity: selected, boundary: path, from_pts, to_pts } = &reversed.steps.last().unwrap().step else { prop_assert!(false, "the original morph remains a morph when reversed"); return Ok(()) };
        prop_assert_eq!(*selected, entity, "the original reference type and identity survive reversal");
        prop_assert_eq!(path, &boundary, "the original boundary reference survives reversal");
        prop_assert_eq!(from_pts, &vec![to; 3], "the destination points become the reversed origin");
        prop_assert_eq!(to_pts, &vec![from; 3], "the origin points become the reversed destination");
    }
}
