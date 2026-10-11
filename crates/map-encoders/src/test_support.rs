use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub(crate) struct LimbCoordinate(pub(crate) f64);

impl PartialEq for LimbCoordinate {
    fn eq(&self, other: &Self) -> bool {
        approx::abs_diff_eq!(self.0, other.0, epsilon = f64::EPSILON)
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct LimbFixtures {
    pub(crate) cases: Vec<LimbCase>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct LimbCase {
    pub(crate) name: String,
    pub(crate) c: Vec<LimbCoordinate>,
    pub(crate) ring: Vec<LimbCoordinate>,
    pub(crate) probes: Vec<LimbProbe>,
    pub(crate) clip: LimbClip,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct LimbProbe {
    pub(crate) p: Vec<LimbCoordinate>,
    pub(crate) inside: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub(crate) enum LimbClip {
    Same,
    None,
    Loops { loops: Vec<Vec<LimbCoordinate>> },
}
