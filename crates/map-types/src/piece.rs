//! THE PIECE: what a scene is composed of (spec §3). A piece is a
//! first-class thing that owns its own geometry — not a render flag.
//! `LayerSet`, which this replaces, lumped Background, Territory and
//! ScriptureClaims under one GEOMETRY bit, so fills, borders and claims
//! could not be selected apart; that is why only four of ten pieces
//! were toggleable on the wire, and why the manifest had nothing
//! truthful to stamp on an entry.
//!
//! Laws (crates/map-types/src/tests.rs): PieceSet is a monoid under
//! union with the empty set as identity; with/without are inverse;
//! every one of the 2^10 subsets round-trips through render/parse.

/// Extensible by adding a variant here and one arm in `name`/`parse` —
/// the compiler names every site that must follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Piece {
    Ground,
    Water,
    Fills,
    Borders,
    Claims,
    Labels,
    Markers,
    Journeys,
    Chrome,
    Veil,
}

impl Piece {
    pub const ALL: [Piece; 10] = [
        Piece::Ground,
        Piece::Water,
        Piece::Fills,
        Piece::Borders,
        Piece::Claims,
        Piece::Labels,
        Piece::Markers,
        Piece::Journeys,
        Piece::Chrome,
        Piece::Veil,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Piece::Ground => "ground",
            Piece::Water => "water",
            Piece::Fills => "fills",
            Piece::Borders => "borders",
            Piece::Claims => "claims",
            Piece::Labels => "labels",
            Piece::Markers => "markers",
            Piece::Journeys => "journeys",
            Piece::Chrome => "chrome",
            Piece::Veil => "veil",
        }
    }
    pub fn parse(s: &str) -> Option<Piece> {
        Piece::ALL.into_iter().find(|p| p.name() == s.trim())
    }
    fn bit(self) -> u16 {
        1 << Piece::ALL.iter().position(|p| *p == self).expect("Piece::ALL is total")
    }
}

/// A subset of the pieces. The empty set is the monoid identity and a
/// legal query, never an error (spec §3 law 1, omission-totality).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PieceSet(u16);

impl PieceSet {
    pub fn empty() -> Self {
        PieceSet(0)
    }
    pub fn all() -> Self {
        Piece::ALL.into_iter().fold(PieceSet(0), |s, p| s.with(p))
    }
    pub fn with(self, p: Piece) -> Self {
        PieceSet(self.0 | p.bit())
    }
    pub fn without(self, p: Piece) -> Self {
        PieceSet(self.0 & !p.bit())
    }
    pub fn contains(self, p: Piece) -> bool {
        self.0 & p.bit() != 0
    }
    pub fn union(self, other: Self) -> Self {
        PieceSet(self.0 | other.0)
    }
    pub fn bits(self) -> u16 {
        self.0
    }
    pub fn iter(self) -> impl Iterator<Item = Piece> {
        Piece::ALL.into_iter().filter(move |p| self.contains(*p))
    }
    /// "none" is the empty set's own token: "" would be ambiguous with a
    /// malformed list, and a set that cannot be written cannot be a
    /// value in a contract scenario.
    pub fn render(self) -> String {
        if self.0 == 0 {
            return "none".to_string();
        }
        self.iter().map(Piece::name).collect::<Vec<_>>().join(", ")
    }
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if s == "none" {
            return Ok(PieceSet::empty());
        }
        if s == "all" {
            return Ok(PieceSet::all());
        }
        let mut out = PieceSet::empty();
        for part in s.split(',') {
            match Piece::parse(part) {
                Some(p) => out = out.with(p),
                None => {
                    return Err(format!(
                        "'{}' is not a piece. Pieces are: {}",
                        part.trim(),
                        Piece::ALL.iter().map(|p| p.name()).collect::<Vec<_>>().join(", ")
                    ))
                }
            }
        }
        Ok(out)
    }
}
