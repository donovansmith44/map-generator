use proptest::prelude::*;

use crate::algebra::Monoid;
use crate::geom::UnitVec;
use crate::transition::{TransitionDraw, TransitionScript, TransitionStep};

proptest! {
    #[test]
    fn generic_script_composition_preserves_exact_clip_and_presentation_order(
        region in any::<u64>(), boundary in any::<u32>(), clip in "[a-z0-9]{1,20}") {
        let first = TransitionDraw { step: TransitionStep::<u64,u32>::FadeIn { region }, clip: Some(clip.clone()) };
        let second = TransitionDraw { step: TransitionStep::<u64,u32>::Morph { boundary, from_pts: vec![UnitVec::from_lat_lon_deg(0.0,0.0)], to_pts: vec![UnitVec::from_lat_lon_deg(0.0,1.0)] }, clip: None };
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
    fn unit_vector_serialization_roundtrips_only_admitted_points(lat in -89.0f64..89.0, lon in -179.0f64..179.0, invalid in 2.0f64..20.0) {
        let point = UnitVec::from_lat_lon_deg(lat,lon);
        let json = serde_json::to_string(&point).expect("an admitted point serializes");
        let restored: UnitVec = serde_json::from_str(&json).expect("an admitted point is accepted");
        prop_assert_eq!(restored, point, "the existing unit vector's exact coordinates survive serialization");
        let bad = format!("{{\"x\":{invalid},\"y\":0,\"z\":0}}");
        prop_assert!(serde_json::from_str::<UnitVec>(&bad).is_err(), "the source door refuses a non-unit vector instead of bypassing construction");
    }
}
