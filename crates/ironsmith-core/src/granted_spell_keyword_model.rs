//! Keyword abilities granted to spells as they are cast (CR 601.2b, 601.2f).
//!
//! "Each instant and sorcery spell you cast has replicate. The replicate cost
//! is equal to its mana cost." (Djinn Illuminatus), "Each spell you cast
//! that's exactly three colors has replicate {3}." (Threefold Signal),
//! "Creature spells you cast gain offspring {2} as you cast them." (Zinnia),
//! "Each red or green instant or sorcery spell you cast has conspire." (Wort)
//! and "Creature spells you cast have demonstrate." (Silverquill Lecturer).
//!
//! A grant is typed by its keyword and price. The engine discovers every
//! grant that applies to a spell while it is being cast: optional-cost
//! keywords become announced optional costs on that spell (CR 601.2b) paid
//! with its other costs (CR 601.2f-h), and keyword-linked triggered abilities
//! are attached to that spell. Nothing is matched by display text.

use crate::tag::TagKeyWalk;
use crate::{CostComponent, TotalCost};

/// The granted keyword.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TagKeyWalk)]
pub enum GrantedSpellKeywordKind {
    /// CR 702.56a: an optional additional cost that may be paid any number of
    /// times; a cast trigger copies the spell once per payment.
    Replicate,
    /// CR 702.175a: an optional additional cost; when the permanent enters,
    /// if it was paid, create a 1/1 token copy of it.
    Offspring,
    /// CR 702.78a: tap two untapped creatures you control that share a color
    /// with the spell; a cast trigger copies it.
    Conspire,
    /// CR 702.144a: no cost; a cast trigger may copy the spell and, if it
    /// does, an opponent also copies it.
    Demonstrate,
}

impl GrantedSpellKeywordKind {
    pub fn keyword_text(self) -> &'static str {
        match self {
            Self::Replicate => "replicate",
            Self::Offspring => "offspring",
            Self::Conspire => "conspire",
            Self::Demonstrate => "demonstrate",
        }
    }

    /// Whether the keyword is an optional cost announced while casting.
    pub fn is_optional_cost(self) -> bool {
        !matches!(self, Self::Demonstrate)
    }

    /// Whether the keyword's own definition fixes what is paid (conspire's
    /// tapping) or pays nothing (demonstrate), so no price is written.
    pub fn has_intrinsic_price(self) -> bool {
        matches!(self, Self::Conspire | Self::Demonstrate)
    }
}

/// What a granted keyword's cost is.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub enum GrantedSpellKeywordPrice<C> {
    /// The keyword itself defines what is paid (conspire, demonstrate).
    Intrinsic,
    /// A printed cost ("replicate {3}", "offspring {2}").
    Fixed(TotalCost<C>),
    /// "The <keyword> cost is equal to its mana cost." The spell's own mana
    /// cost, read from the spell being cast.
    SpellManaCost,
}

impl<C> GrantedSpellKeywordPrice<C> {
    pub fn try_map<C2, Error>(
        self,
        map_cost: impl FnMut(C) -> Result<C2, Error>,
    ) -> Result<GrantedSpellKeywordPrice<C2>, Error> {
        Ok(match self {
            Self::Intrinsic => GrantedSpellKeywordPrice::Intrinsic,
            Self::Fixed(cost) => GrantedSpellKeywordPrice::Fixed(cost.try_map(map_cost)?),
            Self::SpellManaCost => GrantedSpellKeywordPrice::SpellManaCost,
        })
    }
}

/// One granted keyword and its price.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct GrantedSpellKeyword<C> {
    pub kind: GrantedSpellKeywordKind,
    pub price: GrantedSpellKeywordPrice<C>,
}

impl<C> GrantedSpellKeyword<C> {
    pub fn new(kind: GrantedSpellKeywordKind, price: GrantedSpellKeywordPrice<C>) -> Self {
        Self { kind, price }
    }

    pub fn intrinsic(kind: GrantedSpellKeywordKind) -> Self {
        Self::new(kind, GrantedSpellKeywordPrice::Intrinsic)
    }

    pub fn try_map<C2, Error>(
        self,
        map_cost: impl FnMut(C) -> Result<C2, Error>,
    ) -> Result<GrantedSpellKeyword<C2>, Error> {
        Ok(GrantedSpellKeyword {
            kind: self.kind,
            price: self.price.try_map(map_cost)?,
        })
    }
}

impl<C: CostComponent> GrantedSpellKeyword<C> {
    /// The keyword as it reads on the granting card ("replicate {3}",
    /// "conspire", "replicate equal to its mana cost").
    pub fn display(&self) -> String {
        let keyword = self.kind.keyword_text();
        match &self.price {
            GrantedSpellKeywordPrice::Intrinsic => keyword.to_string(),
            GrantedSpellKeywordPrice::Fixed(cost) => format!("{keyword} {}", cost.display()),
            GrantedSpellKeywordPrice::SpellManaCost => {
                format!("{keyword}. The {keyword} cost is equal to its mana cost")
            }
        }
    }
}
