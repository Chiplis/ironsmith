use crate::zone::Zone;
pub use ironsmith_core::{AlternativeCastRequirements, TrapCondition};

pub type AlternativeCastingMethod = ironsmith_core::AlternativeCastingMethod<
    crate::effect::Effect,
    crate::costs::Cost,
    crate::static_abilities::ThisSpellCostCondition,
>;

/// Which method is being used to cast a spell.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CastingMethod {
    #[default]
    Normal,
    FaceDown,
    SplitOtherHalf,
    Fuse,
    Alternative(usize),
    GrantedEscape {
        source: crate::ids::ObjectId,
        exile_count: u32,
    },
    GrantedFlashback,
    PlayFrom {
        source: crate::ids::ObjectId,
        zone: Zone,
        use_alternative: Option<usize>,
    },
    SplitOtherHalfPlayFrom {
        source: crate::ids::ObjectId,
        zone: Zone,
        use_alternative: usize,
    },
}

impl CastingMethod {
    pub fn is_alternative(&self) -> bool {
        matches!(self, Self::Alternative(_) | Self::FaceDown)
    }

    pub fn exiles_after_resolution(&self) -> bool {
        matches!(
            self,
            Self::GrantedFlashback
                | Self::GrantedEscape { .. }
                | Self::SplitOtherHalfPlayFrom { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mana::{ManaCost, ManaSymbol};

    #[test]
    fn test_flashback_properties() {
        let flashback = AlternativeCastingMethod::Flashback {
            total_cost: crate::cost::TotalCost::mana(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(2)],
                vec![ManaSymbol::Blue],
            ])),
        };

        assert_eq!(flashback.cast_from_zone(), Zone::Graveyard);
        assert!(flashback.exiles_after_resolution());
        assert!(flashback.mana_cost().is_some());
        assert_eq!(flashback.name(), "Flashback");
    }

    #[test]
    fn test_jump_start_properties() {
        let jump_start = AlternativeCastingMethod::JumpStart;

        assert_eq!(jump_start.cast_from_zone(), Zone::Graveyard);
        assert!(jump_start.exiles_after_resolution());
        assert!(jump_start.mana_cost().is_none());
        assert_eq!(jump_start.name(), "Jump-start");
    }

    #[test]
    fn test_escape_properties() {
        let escape = AlternativeCastingMethod::Escape {
            cost: Some(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Black],
            ])),
            exile_count: 4,
        };

        assert_eq!(escape.cast_from_zone(), Zone::Graveyard);
        assert!(escape.exiles_after_resolution());
        assert!(escape.mana_cost().is_some());
        assert_eq!(escape.name(), "Escape");
    }

    #[test]
    fn test_casting_method() {
        let normal = CastingMethod::Normal;
        let alternative = CastingMethod::Alternative(0);

        assert!(!normal.is_alternative());
        assert!(alternative.is_alternative());
        assert_eq!(CastingMethod::default(), CastingMethod::Normal);
    }
}
