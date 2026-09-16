//! WHERE THE WORDS GO, stated once for every chart and every encoder:
//! the page a camera draws, and the label pass over it. A name must
//! fit what it names, stand on its own land, keep off its neighbours
//! and keep on the page; a name that cannot is not drawn, and an
//! answer that says where names sit sends no name it does not draw.

use std::collections::BTreeMap;

use map_types::camera::{Camera, ChartKind, DESIGN_WIDTH, ZOOM_MAX, ZOOM_MIN};
use map_types::scene::{LabelFace, LabelSubject};
use map_types::{covers_sphere, inside_ring, Piece, RegionId, Snapshot, UnitVec};

/// The page's inner margin, in px: the frame every chart draws inside.
pub const PAGE_PADDING: f64 = 16.0;
/// The globe cannot look closer than this, in degrees of angular
/// radius: the orthographic scale is derived from the sine of the zoom
/// and the page mirrors the same floor.
pub const GLOBE_ZOOM_MIN: f64 = 1.0;
/// Below this share of the page (never under the floor) text is noise.
const LEGIBILITY_SHARE: f64 = 1.0 / 260.0;
const LEGIBILITY_FLOOR_PX: f64 = 4.0;
/// A name sized to its land keeps this share of the land's projected
/// width and height.
const FIT_WIDTH: f64 = 0.92;
const FIT_HEIGHT: f64 = 0.8;
/// A line of text is this many em tall, with the ink standing this
/// far above and below the baseline.
const LINE_HEIGHT_EM: f64 = 1.25;
const ASCENT: f64 = 0.75;
const DESCENT: f64 = 0.35;
/// The nudges a name tries, in line heights, when its anchor is taken.
const NUDGES: [f64; 5] = [0.0, -1.0, 1.0, -2.0, 2.0];
/// The sizes a land name tries, as shares of its fitted size, when its
/// words spill past its shore.
const SHRINK_LADDER: [f64; 4] = [1.0, 0.85, 0.7, 0.55];

enum Frame {
    Globe { center: UnitVec, east: UnitVec, north: UnitVec, scale: f64, cx: f64, cy: f64 },
    Flat { x0: f64, y1: f64, scale_x: f64, scale_y: f64, padding: f64 },
}

/// A chart's page: the mapping between the sphere and page pixels,
/// invertible where the page shows the sphere at all.
pub struct Projector {
    frame: Frame,
    pub width: f64,
    pub height: f64,
}

pub fn globe_basis(center: &UnitVec) -> (UnitVec, UnitVec) {
    let east = UnitVec::normalize(-center.y(), center.x(), 0.0)
        .unwrap_or_else(|_| UnitVec::from_lat_lon_deg(0.0, 90.0));
    let (nx, ny, nz) = center.cross_raw(&east);
    let north =
        UnitVec::normalize(nx, ny, nz).unwrap_or_else(|_| UnitVec::from_lat_lon_deg(90.0, 0.0));
    (east, north)
}

impl Projector {
    pub fn of(cam: &Camera, padding: f64) -> Projector {
        match cam.chart {
            ChartKind::Globe => Projector::globe(
                cam.center(),
                cam.zoom.clamp(GLOBE_ZOOM_MIN, ZOOM_MAX).to_radians().sin(),
                cam.width,
                padding,
            ),
            ChartKind::Flat => {
                let z = cam.zoom.clamp(ZOOM_MIN, ZOOM_MAX);
                let inner = cam.width - 2.0 * padding;
                let kx = cam.lat.to_radians().cos().max(0.05);
                let scale_g = inner / (2.0 * z);
                let span_x = 2.0 * z / kx;
                let span_y = (cam.width / 2.0) / scale_g;
                Projector::flat(
                    cam.lon - span_x / 2.0,
                    cam.lat + span_y / 2.0,
                    scale_g * kx,
                    scale_g,
                    cam.width,
                    span_y * scale_g + 2.0 * padding,
                    padding,
                )
            }
        }
    }

    /// The orthographic globe: `r_view` is the sine of the view's
    /// angular radius, so the page's inner width spans the view.
    pub fn globe(center: UnitVec, r_view: f64, width: f64, padding: f64) -> Projector {
        let (east, north) = globe_basis(&center);
        let inner = width - 2.0 * padding;
        Projector {
            frame: Frame::Globe {
                center,
                east,
                north,
                scale: inner / 2.0 / r_view,
                cx: width / 2.0,
                cy: width / 2.0,
            },
            width,
            height: width,
        }
    }

    pub fn flat(
        x0: f64,
        y1: f64,
        scale_x: f64,
        scale_y: f64,
        width: f64,
        height: f64,
        padding: f64,
    ) -> Projector {
        Projector { frame: Frame::Flat { x0, y1, scale_x, scale_y, padding }, width, height }
    }

    /// Page coordinates of a sphere point; None where the chart cannot
    /// show it (the globe's far side).
    pub fn place(&self, p: &UnitVec) -> Option<(f64, f64)> {
        match &self.frame {
            Frame::Globe { center, east, north, scale, cx, cy } => {
                if p.dot(center) < 0.0 {
                    return None;
                }
                Some((cx + p.dot(east) * scale, cy - p.dot(north) * scale))
            }
            Frame::Flat { x0, y1, scale_x, scale_y, padding } => {
                let (lat, lon) = p.to_lat_lon_deg();
                Some((padding + (lon - x0) * scale_x, padding + (y1 - lat) * scale_y))
            }
        }
    }

    /// The sphere point under a page point; None off the globe's disc
    /// or off the flat plate's world.
    pub fn unplace(&self, x: f64, y: f64) -> Option<UnitVec> {
        match &self.frame {
            Frame::Globe { center, east, north, scale, cx, cy } => {
                let ex = (x - cx) / scale;
                let ny = (cy - y) / scale;
                let rr = ex * ex + ny * ny;
                if rr > 1.0 {
                    return None;
                }
                let f = (1.0 - rr).sqrt();
                UnitVec::normalize(
                    ex * east.x() + ny * north.x() + f * center.x(),
                    ex * east.y() + ny * north.y() + f * center.y(),
                    ex * east.z() + ny * north.z() + f * center.z(),
                )
                .ok()
            }
            Frame::Flat { x0, y1, scale_x, scale_y, padding } => {
                let lon = x0 + (x - padding) / scale_x;
                let lat = y1 - (y - padding) / scale_y;
                if !(-90.0..=90.0).contains(&lat) {
                    return None;
                }
                Some(UnitVec::from_lat_lon_deg(lat, lon))
            }
        }
    }
}

/// One name, settled: where its baseline sits, how large it is drawn
/// and the box its ink covers, all in page px; `index` is the label's
/// place in the scene, which is its priority.
#[derive(Clone, Debug, PartialEq)]
pub struct Laid {
    pub index: usize,
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub size: f64,
    pub bounds: (f64, f64, f64, f64),
}

struct Land<'a> {
    rings: Vec<&'a [UnitVec]>,
    shores: Vec<Vec<Option<(f64, f64)>>>,
    extent: Option<(f64, f64, f64, f64)>,
}

fn lands<'a>(scene: &'a Snapshot, proj: &Projector) -> BTreeMap<RegionId, Land<'a>> {
    let mut out: BTreeMap<RegionId, Land<'a>> = BTreeMap::new();
    for r in &scene.regions {
        let land = out.entry(r.region).or_insert_with(|| Land { rings: Vec::new(), shores: Vec::new(), extent: None });
        for ring in r.outer.iter().chain(&r.holes) {
            let pts = ring.points();
            land.rings.push(pts);
            let shore: Vec<Option<(f64, f64)>> = pts.iter().map(|p| proj.place(p)).collect();
            for (x, y) in shore.iter().flatten() {
                land.extent = Some(match land.extent {
                    None => (*x, *y, *x, *y),
                    Some((x0, y0, x1, y1)) => (x0.min(*x), y0.min(*y), x1.max(*x), y1.max(*y)),
                });
            }
            land.shores.push(shore);
        }
    }
    out
}

fn inside_land(p: &UnitVec, rings: &[&[UnitVec]]) -> bool {
    rings.iter().filter(|ring| inside_ring(p, ring)).count() % 2 == 1
}

fn segment_distance((x, y): (f64, f64), (ax, ay): (f64, f64), (bx, by): (f64, f64)) -> f64 {
    let (dx, dy) = (bx - ax, by - ay);
    let l2 = dx * dx + dy * dy;
    let t = if l2 <= 0.0 { 0.0 } else { (((x - ax) * dx + (y - ay) * dy) / l2).clamp(0.0, 1.0) };
    let (cx, cy) = (ax + t * dx, ay + t * dy);
    ((x - cx) * (x - cx) + (y - cy) * (y - cy)).sqrt()
}

fn shore_distance(q: (f64, f64), shores: &[Vec<Option<(f64, f64)>>]) -> f64 {
    let mut best = f64::INFINITY;
    for shore in shores {
        let n = shore.len();
        for i in 0..n {
            if let (Some(a), Some(b)) = (shore[i], shore[(i + 1) % n]) {
                best = best.min(segment_distance(q, a, b));
            }
        }
    }
    best
}

/// Every corner of the box stands on the land, or within `allowance`
/// px of its shore.
fn within_land(proj: &Projector, b: (f64, f64, f64, f64), land: &Land, allowance: f64) -> bool {
    [(b.0, b.1), (b.2, b.1), (b.0, b.3), (b.2, b.3)].iter().all(|&(x, y)| match proj.unplace(x, y) {
        None => false,
        Some(p) => inside_land(&p, &land.rings) || shore_distance((x, y), &land.shores) <= allowance,
    })
}

fn on_page(proj: &Projector, b: (f64, f64, f64, f64)) -> bool {
    b.0 >= 0.0 && b.1 >= 0.0 && b.2 <= proj.width && b.3 <= proj.height
}

fn collides(b: &(f64, f64, f64, f64), placed: &[(f64, f64, f64, f64)]) -> bool {
    placed.iter().any(|p| b.0 < p.2 && p.0 < b.2 && b.1 < p.3 && p.1 < b.3)
}

/// Why a name is not drawn at this view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Yield {
    /// its anchor is on the far side of the globe
    Behind,
    /// the thing it names never reaches the page
    Unseen,
    /// the thing it names spans less than the legibility floor
    Unresolved,
    /// even the fitted size is below the legibility floor
    TooSmall,
    /// every candidate box runs off the page
    OffPage,
    /// every candidate box on the page lies over an earlier name
    Crowded,
    /// every candidate box on the page and clear of its neighbours
    /// spills past its own shore by more than the overflow
    Spilled,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    Settled(Laid),
    Yielded(usize, Yield),
}

/// The label pass. Scene order is priority: an earlier name keeps its
/// place and a later one nudges or yields.
pub fn layout(scene: &Snapshot, proj: &Projector, overflow_em: f64) -> Vec<Laid> {
    verdicts(scene, proj, overflow_em)
        .into_iter()
        .filter_map(|v| match v {
            Verdict::Settled(laid) => Some(laid),
            Verdict::Yielded(..) => None,
        })
        .collect()
}

pub fn verdicts(scene: &Snapshot, proj: &Projector, overflow_em: f64) -> Vec<Verdict> {
    let lands = lands(scene, proj);
    let min_size = (proj.width * LEGIBILITY_SHARE).max(LEGIBILITY_FLOOR_PX);
    let mut placed: Vec<(f64, f64, f64, f64)> = Vec::new();
    let mut out = Vec::new();
    for (index, l) in scene.labels.iter().enumerate() {
        let Some((x, y)) = proj.place(&l.at) else {
            out.push(Verdict::Yielded(index, Yield::Behind));
            continue;
        };
        let v = l.voice;
        let text = if v.uppercase { l.text.to_uppercase() } else { l.text.clone() };
        let chars = text.chars().count().max(1) as f64;
        let mut size = l.style.size * proj.width / DESIGN_WIDTH;
        let mut land = None;
        if let LabelSubject::Region(rid) = &l.subject {
            let Some((this, (x0, y0, x1, y1))) = lands.get(rid).and_then(|t| t.extent.map(|e| (t, e))) else {
                out.push(Verdict::Yielded(index, Yield::Unseen));
                continue;
            };
            if (x1 - x0).max(y1 - y0) < min_size {
                out.push(Verdict::Yielded(index, Yield::Unresolved));
                continue;
            }
            if l.face != LabelFace::Water {
                size = size.min((x1 - x0) * FIT_WIDTH / (chars * v.advance_em)).min((y1 - y0) * FIT_HEIGHT);
            }
            if l.face == LabelFace::Territory {
                land = Some(this);
            }
        }
        if size < min_size {
            out.push(Verdict::Yielded(index, Yield::TooSmall));
            continue;
        }
        let ladder: &[f64] = if land.is_some() { &SHRINK_LADDER } else { &SHRINK_LADDER[..1] };
        let mut spot = None;
        let mut nearest = Yield::OffPage;
        'sizes: for share in ladder {
            let s = size * share;
            if s < min_size {
                break;
            }
            let (w, h) = (chars * v.advance_em * s, s * LINE_HEIGHT_EM);
            for nudge in NUDGES {
                let baseline = y + nudge * h;
                let b = (x - w / 2.0, baseline - h * ASCENT, x + w / 2.0, baseline + h * DESCENT);
                if !on_page(proj, b) {
                    continue;
                }
                if collides(&b, &placed) {
                    nearest = nearest.max(Yield::Crowded);
                    continue;
                }
                if let Some(land) = land {
                    if !within_land(proj, b, land, overflow_em * s) {
                        nearest = nearest.max(Yield::Spilled);
                        continue;
                    }
                }
                spot = Some((baseline, s, b));
                break 'sizes;
            }
        }
        let Some((baseline, s, b)) = spot else {
            out.push(Verdict::Yielded(index, nearest));
            continue;
        };
        placed.push(b);
        out.push(Verdict::Settled(Laid { index, text, x, y: baseline, size: s, bounds: b }));
    }
    out
}

/// What a city stands on: the topmost land or claim drawn under its
/// point, or ground nothing claims. A property of the scene, never of
/// the camera.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ground {
    Region(RegionId),
    Unclaimed,
}

impl Ground {
    pub fn wire(&self) -> String {
        match self {
            Ground::Region(r) => format!("region:{:016x}", r.0 .0),
            Ground::Unclaimed => "unclaimed".to_string(),
        }
    }
}

pub fn ground_of(scene: &Snapshot, at: &UnitVec) -> Ground {
    for r in scene.regions.iter().rev() {
        if !matches!(r.piece, Piece::Fills | Piece::Claims) {
            continue;
        }
        if r.outer.iter().any(|ring| covers_sphere(ring.points())) {
            continue;
        }
        let odd = r.outer.iter().chain(&r.holes).filter(|ring| inside_ring(at, ring.points())).count() % 2 == 1;
        if odd {
            return Ground::Region(r.region);
        }
    }
    Ground::Unclaimed
}

pub fn stands_on(l: &map_types::PlacedLabel, scene: &Snapshot) -> Option<Ground> {
    match &l.subject {
        LabelSubject::Place(_) => Some(ground_of(scene, &l.at)),
        _ => None,
    }
}
