use atlas_graph_types::canon::{self, at_path, expect_arr, expect_exact_keys, expect_obj, field, field_i32, field_str, join, obj, Canon, CanonError, Decode, DecodeOn, Value, ROOT};
use atlas_graph_types::id::AnyNodeId;
use atlas_graph_types::map_timing::RecordedMapCause;
use atlas_graph_types::passage::Readings;
use atlas_graph_types::place_versions::Basis;
use serde::{de::DeserializeOwned, Serialize};

use super::{admit_script, FaceHolding, FaceTransfer, RecordedMapChange};
use atlas_graph_types::edge::HoldingState;

impl Canon for RecordedMapChange<AnyNodeId> {
    fn to_value(&self) -> Value {
        obj(vec![
            ("year", Value::Int(i64::from(self.year))),
            ("before_year", Value::Int(i64::from(self.before_year))),
            ("causes", Value::Arr(self.causes.iter().map(Canon::to_value).collect())),
            ("arcs", projected(&self.arcs)),
            ("transfers", Value::Arr(self.transfers.iter().map(transfer_value).collect())),
            ("script", projected(&self.script)),
        ])
    }
}

impl DecodeOn for RecordedMapChange<AnyNodeId> {
    fn from_value_on(value: &Value, readings: &Readings) -> Result<Self, CanonError> {
        let row = expect_obj(value, ROOT)?;
        expect_exact_keys(row, ROOT, &["year", "before_year", "causes", "arcs", "transfers", "script"])?;
        let (causes, causes_path) = field(row, ROOT, "causes")?;
        let causes = expect_arr(causes, &causes_path)?.iter().enumerate().map(|(index, cause)| {
            RecordedMapCause::from_value_on(cause, readings).map_err(|error| at_path(&join(&causes_path, &index.to_string()), error))
        }).collect::<Result<Vec<_>,_>>()?;
        let (transfers, transfers_path) = field(row, ROOT, "transfers")?;
        let transfers = expect_arr(transfers, &transfers_path)?.iter().enumerate().map(|(index, transfer)| {
            read_transfer(transfer, readings).map_err(|error| at_path(&join(&transfers_path, &index.to_string()), error))
        }).collect::<Result<Vec<_>,_>>()?;
        let (script, script_path) = field(row, ROOT, "script")?;
        let script = admit_script(read_projected(script, &script_path)?).map_err(|reason| CanonError::new(&script_path, format!("invalid paired script: {reason:?}")))?;
        let (arcs, arcs_path) = field(row, ROOT, "arcs")?;
        Ok(Self { year: field_i32(row, ROOT, "year")?, before_year: field_i32(row, ROOT, "before_year")?, causes, arcs: read_projected(arcs, &arcs_path)?, transfers, script })
    }
}

fn transfer_value(transfer: &FaceTransfer<AnyNodeId>) -> Value {
    obj(vec![
        ("subset", Value::Str(transfer.subset.hex())),
        ("boundary", projected(&transfer.boundary)),
        ("from", holding_value(&transfer.from)),
        ("to", holding_value(&transfer.to)),
        ("basis", transfer.basis.to_value()),
    ])
}

fn read_transfer(value: &Value, readings: &Readings) -> Result<FaceTransfer<AnyNodeId>, CanonError> {
    let row = expect_obj(value, ROOT)?;
    expect_exact_keys(row, ROOT, &["subset", "boundary", "from", "to", "basis"])?;
    let (boundary, boundary_path) = field(row, ROOT, "boundary")?;
    let (from, from_path) = field(row, ROOT, "from")?;
    let (to, to_path) = field(row, ROOT, "to")?;
    let (basis, basis_path) = field(row, ROOT, "basis")?;
    Ok(FaceTransfer {
        subset: atlas_graph_types::map_slice::ShapeId::parse(&field_str(row, ROOT, "subset")?).map_err(|error| CanonError::new("$.subset", format!("{error:?}")))?,
        boundary: read_projected(boundary, &boundary_path)?,
        from: read_holding(from, &from_path)?,
        to: read_holding(to, &to_path)?,
        basis: Basis::from_value_on(basis, readings).map_err(|error| at_path(&basis_path, error))?,
    })
}

fn projected<T: Serialize>(value: &T) -> Value {
    let mut published = serde_json::to_value(value).expect("admitted source values serialize");
    published.sort_all_objects();
    canon::parse(&serde_json::to_vec(&published).expect("source JSON values serialize")).expect("source serialization is canonical JSON data")
}

fn read_projected<T: Serialize + DeserializeOwned>(value: &Value, path: &str) -> Result<T, CanonError> {
    let bytes = canon::serialize(value);
    let admitted: T = serde_json::from_slice(&bytes).map_err(|error| CanonError::new(path, format!("invalid source shape: {error}")))?;
    if canon::serialize(&projected(&admitted)) != bytes {
        return Err(CanonError::new(path, "source shape must retain every canonical field"));
    }
    Ok(admitted)
}


fn holding_value(holding: &Option<FaceHolding<AnyNodeId>>) -> Value {
    match holding {
        None => Value::Null,
        Some(value) => obj(vec![
            ("territory", Value::Str(canon::ids::any_node_id_str(&value.territory))),
            ("holding", value.holding.as_ref().map_or(Value::Null, Canon::to_value)),
        ]),
    }
}

fn read_holding(value: &Value, path: &str) -> Result<Option<FaceHolding<AnyNodeId>>, CanonError> {
    if *value == Value::Null { return Ok(None); }
    let row = expect_obj(value, path)?;
    expect_exact_keys(row, path, &["territory", "holding"])?;
    let territory = canon::ids::parse_any_node_id(&field_str(row, path, "territory")?, &join(path, "territory"))?;
    let (holding, holding_path) = field(row, path, "holding")?;
    let holding = if *holding == Value::Null { None } else {
        Some(HoldingState::from_value(holding).map_err(|error| at_path(&holding_path, error))?)
    };
    Ok(Some(FaceHolding { territory, holding }))
}
