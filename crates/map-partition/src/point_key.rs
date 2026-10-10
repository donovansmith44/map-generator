use map_types::UnitVec;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PointKey([u8; 24]);

impl PointKey {
    const PARTITION_UNITS: f64 = 1_000_000_000.0;

    pub fn partition(point: &UnitVec) -> Self {
        Self::quantized([point.x(), point.y(), point.z()], Self::PARTITION_UNITS)
    }

    pub fn bytes(self) -> [u8; 24] {
        self.0
    }

    fn quantized(values: [f64; 3], units: f64) -> Self {
        Self::integers(values.map(|value| (value * units).round() as i64))
    }

    fn integers(values: [i64; 3]) -> Self {
        let mut bytes = [0; 24];
        for (target, value) in bytes.chunks_exact_mut(8).zip(values) {
            target.copy_from_slice(&value.to_be_bytes());
        }
        Self(bytes)
    }
}
