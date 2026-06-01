use crate::diagnostics::TextSpan;
use ironsmith_core::{ChoiceCount, TagKey, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageBySpec {
    ThisCreature,
    EquippedCreature,
    EnchantedCreature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAst {
    You,
    Any,
    Chosen,
    Defending,
    Attacking,
    Active,
    MostCardsInHand,
    MostLifeTied,
    LowestLifeTied,
    Target,
    TargetOpponent,
    Opponent,
    NotYou,
    That,
    ThatPlayerOrTargetController,
    ItsController,
    ItsOwner,
    Implicit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnControllerAst {
    Preserve,
    Owner,
    You,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryConsultModeAst {
    Reveal,
    Exile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryConsultStopRuleAst<Value = crate::effect::Value> {
    FirstMatch,
    MatchCount(Value),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryBottomOrderAst {
    Random,
    ChooserChooses,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectRefAst<Tag = TagKey> {
    Tagged(Tag),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchLibrarySlotAst<Filter = crate::target::ObjectFilter> {
    pub filter: Filter,
    pub optional: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneReplacementDurationAst {
    OneShot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlDurationAst {
    UntilEndOfTurn,
    UntilYourNextTurnEnd,
    DuringNextTurn,
    AsLongAsYouControlSource,
    Forever,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtraTurnAnchorAst {
    CurrentTurn,
    ReferencedTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedTypeConstraintAst {
    CardType,
    PermanentType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeValueKindAst {
    Power,
    Toughness,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExchangeValueAst<
    Player = PlayerAst,
    Target = TargetAst<crate::target::PlayerFilter, crate::target::ObjectFilter>,
> {
    LifeTotal(Player),
    Stat {
        target: Target,
        kind: ExchangeValueKindAst,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum TargetAst<
    PlayerFilter = crate::target::PlayerFilter,
    ObjectFilter = crate::target::ObjectFilter,
    Tag = TagKey,
> {
    Source(Option<TextSpan>),
    AnyTarget(Option<TextSpan>),
    AnyOtherTarget(Option<TextSpan>),
    PlayerOrPlaneswalker(PlayerFilter, Option<TextSpan>),
    AttackedPlayerOrPlaneswalker(Option<TextSpan>),
    Spell(Option<TextSpan>),
    Player(PlayerFilter, Option<TextSpan>),
    Object(ObjectFilter, Option<TextSpan>, Option<TextSpan>),
    Tagged(Tag, Option<TextSpan>),
    WithCount(Box<TargetAst<PlayerFilter, ObjectFilter, Tag>>, ChoiceCount),
    WithCountValue(
        Box<TargetAst<PlayerFilter, ObjectFilter, Tag>>,
        ChoiceCount,
        Value,
    ),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetargetModeAst<
    Target = TargetAst<crate::target::PlayerFilter, crate::target::ObjectFilter>,
> {
    All,
    OneToFixed { target: Target },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreventNextTimeDamageSourceAst<Filter = crate::target::ObjectFilter> {
    Choice,
    Filter(Filter),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedirectNextTimeDamageDestinationAst {
    SourceObject,
    Controller,
    SourceController,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreventNextTimeDamageTargetAst {
    AnyTarget,
    You,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClashOpponentAst {
    Opponent,
    TargetOpponent,
    DefendingPlayer,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_ast_can_wrap_choice_counts() {
        let target = TargetAst::<&'static str, &'static str>::WithCount(
            Box::new(TargetAst::Player("you", None)),
            ChoiceCount::up_to(2),
        );

        match target {
            TargetAst::WithCount(inner, count) => {
                assert!(matches!(*inner, TargetAst::Player("you", None)));
                assert_eq!(count.max, Some(2));
            }
            _ => panic!("expected counted target"),
        }
    }

    #[test]
    fn exchange_value_ast_preserves_target_kind() {
        let value: ExchangeValueAst<PlayerAst, &'static str> = ExchangeValueAst::Stat {
            target: "that creature",
            kind: ExchangeValueKindAst::Power,
        };

        assert!(matches!(
            value,
            ExchangeValueAst::Stat {
                target: "that creature",
                kind: ExchangeValueKindAst::Power
            }
        ));
    }
}
