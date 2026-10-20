#![cfg(all(feature = "serde", feature = "canon-ids"))]

use atlas_graph_types::map_slice::{CollectionType, FaceStatus, FeatureCollection, Geometry, ShapeId, StoredShape, WaterBody, shape_key};
use map_types::atlas_transition::{MapKeyframe, MapTransition, RecordedMapChange, TransitionRead, TransitionRefusal};
use map_types::transition::TransitionScript;

#[test]
fn ready_transitions_share_geometry_without_losing_scenes_or_frame_order() {
    // Arrange
    let geometry = Geometry::Point([35.0, 32.0]);
    let shape = shape_key(&geometry);
    let scene = FeatureCollection {
        r#type: CollectionType::FeatureCollection,
        features: vec![],
        bbox: [34.0, 31.0, 36.0, 33.0],
        waters: vec![WaterBody { geometry: shape, area_km2: 2.0, within: None, ground: FaceStatus::Background, min_zoom: None, shore_max_zoom: Some(5) }],
    };
    let change = RecordedMapChange::<u8, u8> { year: 2, before_year: 1, causes: vec![], arcs: vec![], transfers: vec![], script: TransitionScript { steps: vec![] } };
    let frame = MapKeyframe { change: change.clone(), before: scene.clone(), after: scene.clone() };
    let shapes = vec![StoredShape { shape, geometry }];
    let input = TransitionRead::Ready { transition: MapTransition::<u8, u8, ShapeId> { from_year: 1, to_year: 2, origin: scene.clone(), destination: scene.clone(), shapes: shapes.clone(), keyframes: vec![frame], after_year: None } };
    let expected = serde_json::json!({ "status": "ready", "transition": { "from_year": 1, "to_year": 2, "origin": scene, "destination": scene, "shapes": shapes, "keyframes": [{ "change": change, "before": scene, "after": scene }], "after_year": null } });
    // Act
    let actual = serde_json::to_value(input).unwrap();
    // Assert
    assert_eq!(actual, expected, "The source transition serializes every scene and frame with served shape references.");
}

#[test]
fn refused_transitions_retain_the_complete_destination_geometry() {
    // Arrange
    let destination = FeatureCollection {
        waters: vec![WaterBody { geometry: Geometry::Point([35.0, 32.0]), area_km2: 2.0, within: None, ground: FaceStatus::Background, min_zoom: None, shore_max_zoom: Some(5) }],
        ..FeatureCollection::empty([34.0, 31.0, 36.0, 33.0])
    };
    let reason = TransitionRefusal::<u8>::NotRecorded;
    let expected = serde_json::json!({ "status": "refused", "reason": reason, "destination": destination });
    // Act
    let actual = serde_json::to_value(TransitionRead::<u8, u8, ShapeId>::Refused { reason, destination }).unwrap();
    // Assert
    assert_eq!(actual, expected, "A refusal remains a complete ordinary drawing without a shape dictionary.");
}
