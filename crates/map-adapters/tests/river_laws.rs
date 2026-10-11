use map_adapters::hydro::{
    read_rivers, RiverCourse, RiverError, RiverNumber, RiverShape, RiverSource,
};
use map_types::UnitVec;
use proptest::prelude::*;
use serde_json::{json, Value};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn generated_documents_preserve_every_identity_name_part_and_vertex(
        sources in proptest::collection::vec(source(), 0..8),
    ) {
        let document = json!({"type":"FeatureCollection", "features":sources.iter().map(|(value, _)| value).collect::<Vec<_>>()});
        let expected: Vec<_> = sources.into_iter().map(|(_, source)| source).collect();
        prop_assert_eq!(read_rivers(&document.to_string()), Ok(expected), "the entire generated river document retains source identities, names, classes, multipart order and Unlocated records");
    }

    #[test]
    fn generated_invalid_fields_refuse_the_entire_document(
        sources in proptest::collection::vec(source(), 1..8),
        failure in failure(), ordinal in any::<usize>(),
        invalid_name in any::<i64>(), invalid_position in 180.0001f64..1000f64,
    ) {
        let index = ordinal % sources.len();
        let mut document = json!({"type":"FeatureCollection", "features":sources.iter().map(|(value, _)| value).collect::<Vec<_>>()});
        let expected = match failure {
            Failure::Collection => { document["type"] = json!("Feature"); RiverError::Collection },
            Failure::Features => { document["features"] = Value::Null; RiverError::Collection },
            Failure::Feature => { document["features"][index]["type"] = Value::Null; RiverError::Feature },
            Failure::Number => { document["features"][index]["properties"]["rivernum"] = json!("unknown"); RiverError::Number },
            Failure::Name => { document["features"][index]["properties"]["name"] = json!(invalid_name); RiverError::Name },
            Failure::Course => { document["features"][index]["properties"]["featurecla"] = json!("Unknown"); RiverError::Course },
            Failure::Geometry => { document["features"][index]["geometry"] = json!({"type":"Polygon", "coordinates":[]}); RiverError::Geometry },
            Failure::Multipart => { document["features"][index]["geometry"] = json!({"type":"MultiLineString", "coordinates":null}); RiverError::Geometry },
            Failure::Path => { document["features"][index]["geometry"] = json!({"type":"LineString", "coordinates":[[0,0]]}); RiverError::Path },
            Failure::Position => { document["features"][index]["geometry"] = json!({"type":"LineString", "coordinates":[[0,0],[invalid_position,0]]}); RiverError::Position },
        };
        prop_assert_eq!(read_rivers(&document.to_string()), Err(expected), "one malformed field refuses the complete generated document without shortening or dropping a course");
    }
}

fn source() -> impl Strategy<Value = (Value, RiverSource)> {
    (any::<i64>(), proptest::option::of(any::<String>()), prop_oneof![Just(RiverCourse::River), Just(RiverCourse::LakeCenterline)], shape())
        .prop_map(|(number, name, course, paths)| {
            let geometry = match &paths {
                Shape::Single(path) => json!({"type":"LineString","coordinates":path}),
                Shape::Multipart(paths) => json!({"type":"MultiLineString","coordinates":paths}),
            };
            let shape = match paths {
                Shape::Single(path) => RiverShape::Course(vec![points(path)]),
                Shape::Multipart(paths) if paths.is_empty() => RiverShape::Unlocated,
                Shape::Multipart(paths) => RiverShape::Course(paths.into_iter().map(points).collect()),
            };
            let label = match course { RiverCourse::River => "River", RiverCourse::LakeCenterline => "Lake Centerline" };
            let value = json!({"type":"Feature","properties":{"rivernum":number,"name":name,"featurecla":label},"geometry":geometry});
            (value, RiverSource { number:RiverNumber(number), name, course, shape })
        })
}

#[derive(Clone, Debug)]
enum Shape {
    Single(Vec<(f64, f64)>),
    Multipart(Vec<Vec<(f64, f64)>>),
}

fn shape() -> impl Strategy<Value = Shape> {
    let path = || proptest::collection::vec((-180f64..180f64, -90f64..90f64), 2..16);
    prop_oneof![
        path().prop_map(Shape::Single),
        proptest::collection::vec(path(), 0..8).prop_map(Shape::Multipart)
    ]
}

fn points(path: Vec<(f64, f64)>) -> Vec<UnitVec> {
    path.into_iter()
        .map(|(lon, lat)| UnitVec::from_lat_lon_deg(lat, lon))
        .collect()
}

#[derive(Clone, Debug)]
enum Failure {
    Collection,
    Features,
    Feature,
    Number,
    Name,
    Course,
    Geometry,
    Multipart,
    Path,
    Position,
}

fn failure() -> impl Strategy<Value = Failure> {
    prop_oneof![
        Just(Failure::Collection),
        Just(Failure::Features),
        Just(Failure::Feature),
        Just(Failure::Number),
        Just(Failure::Name),
        Just(Failure::Course),
        Just(Failure::Geometry),
        Just(Failure::Multipart),
        Just(Failure::Path),
        Just(Failure::Position)
    ]
}
