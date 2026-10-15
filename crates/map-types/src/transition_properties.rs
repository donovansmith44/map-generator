use proptest::prelude::*;

use crate::algebra::Monoid;
use crate::geom::UnitVec;
use crate::transition::{TransitionDraw, TransitionScript, TransitionStep};

proptest! {
    #[test]
    fn generic_script_composition_preserves_exact_clip_and_presentation_order(
        region in any::<u64>(), boundary in any::<u32>(), clip in "[a-z0-9]{1,20}") {
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
        boundary in any::<u32>(), clip in "[a-z0-9]{1,20}",
        lat in -80.0f64..80.0, lon in -170.0f64..170.0, n in 2usize..12) {
        use crate::atlas_transition::{admit_script, TransitionRefusal};
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
