use map_types::UnitVec;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PointKey([u8; 24]);

impl PointKey {
    pub const LINEAGE_TOLERANCE_DEGREES: f64 = 0.000001;
    pub const LINEAGE_NEIGHBOR_COUNT: usize = Self::LINEAGE_CELL_WIDTH * Self::LINEAGE_CELL_WIDTH;
    const LINEAGE_CELL_WIDTH: usize = 3;
    const PARTITION_UNITS: f64 = 1_000_000_000.0;
    const LONGITUDE_HALF_TURN: i64 = (180.0 / Self::LINEAGE_TOLERANCE_DEGREES) as i64;

    pub fn partition(point: &UnitVec) -> Self {
        Self::quantized([point.x(), point.y(), point.z()], Self::PARTITION_UNITS)
    }

    pub fn lineage(point: &UnitVec) -> Self {
        let (lat, lon) = point.to_lat_lon_deg();
        let key = Self::quantized([lat, lon, 0.0], Self::LINEAGE_TOLERANCE_DEGREES.recip());
        let [lat, lon, _] = key.components();
        Self::integers([lat, Self::longitude(lon), 0])
    }

    pub fn lineage_neighborhood(point: &UnitVec) -> [Self; Self::LINEAGE_NEIGHBOR_COUNT] {
        let [lat, lon, _] = Self::lineage(point).components();
        std::array::from_fn(|index| {
            let latitude_offset = (index / Self::LINEAGE_CELL_WIDTH) as i64 - 1;
            let longitude_offset = (index % Self::LINEAGE_CELL_WIDTH) as i64 - 1;
            Self::integers([
                lat + latitude_offset,
                Self::longitude(lon + longitude_offset),
                0,
            ])
        })
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

    fn components(self) -> [i64; 3] {
        std::array::from_fn(|index| {
            i64::from_be_bytes(
                self.0[index * 8..(index + 1) * 8]
                    .try_into()
                    .expect("point keys contain three integers"),
            )
        })
    }

    fn longitude(value: i64) -> i64 {
        (value + Self::LONGITUDE_HALF_TURN).rem_euclid(Self::LONGITUDE_HALF_TURN * 2)
            - Self::LONGITUDE_HALF_TURN
    }
}
