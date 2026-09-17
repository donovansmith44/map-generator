//! What a source permits, as a type.
//!
//! Law 6 makes provenance total: every drawn thing names where it came
//! from. A name alone cannot answer the question a redistributor actually
//! has, which is what the terms require of them. The consuming atlas has a
//! hard free-and-open-source requirement, and "free and open source" is a
//! property of the license, not of the source's name.
//!
//! So the set is CLOSED. A dataset whose terms are not one of these cannot
//! enter the canon, because there is nowhere to put it: `Witness::license`
//! is a total match and the compiler refuses a new origin that names no
//! terms.

use std::fmt;

use atlas_graph_types::covenant::SourceId;

/// The terms a source is available under.
///
/// Ordered by what redistribution asks of you, most permissive first. The
/// order is load-bearing: `ALL` is asserted whole, and the two obligations
/// are monotone along it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum License {
    /// No rights reserved and no credit asked: Natural Earth, ETOPO1.
    PublicDomain,
    Cc0,
    CcBy4,
    CcBySa3,
    CcBySa4,
    /// Open Database License: share-alike over a DERIVED database, which
    /// is a narrower trigger than a code copyleft.
    Odbl1,
    Gpl3,
}

impl License {
    /// Every license this canon can carry, most permissive first.
    pub const ALL: [License; 7] = [
        License::PublicDomain,
        License::Cc0,
        License::CcBy4,
        License::CcBySa3,
        License::CcBySa4,
        License::Odbl1,
        License::Gpl3,
    ];

    /// The stable token this license is published under. SPDX identifiers
    /// where one exists; public domain has none, so it carries our own.
    pub fn id(self) -> &'static str {
        match self {
            License::PublicDomain => "public-domain",
            License::Cc0 => "CC0-1.0",
            License::CcBy4 => "CC-BY-4.0",
            License::CcBySa3 => "CC-BY-SA-3.0",
            License::CcBySa4 => "CC-BY-SA-4.0",
            License::Odbl1 => "ODbL-1.0",
            License::Gpl3 => "GPL-3.0-only",
        }
    }

    /// Redistribution must name the source.
    pub fn credit_required(self) -> bool {
        !matches!(self, License::PublicDomain | License::Cc0)
    }

    /// Redistribution of a derived work must carry these same terms.
    pub fn share_alike(self) -> bool {
        matches!(
            self,
            License::CcBySa3 | License::CcBySa4 | License::Odbl1 | License::Gpl3
        )
    }
}

impl fmt::Display for License {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// A source and its terms, inseparable.
///
/// Attribution is only ever extended through one of these, so a scene can
/// never name a source whose terms it does not also carry. That is the
/// whole point of the type: the two facts cannot drift apart, because
/// there is no API that carries one without the other.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Credit {
    pub source: SourceId,
    pub license: License,
}

impl Credit {
    pub fn new(source: SourceId, license: License) -> Self {
        Credit { source, license }
    }
}

impl fmt::Display for Credit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.source.0, self.license)
    }
}

/// Asking a credited set what it names, without caring about terms.
pub trait Credited {
    /// Whether this set credits `source`, under any terms.
    fn names(&self, source: &str) -> bool;
}

impl Credited for std::collections::BTreeSet<Credit> {
    fn names(&self, source: &str) -> bool {
        self.iter().any(|c| c.source.0 == source)
    }
}
