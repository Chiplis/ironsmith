use ironsmith_core::tag::TagKeyWalk;

use crate::diagnostics::TextSpan;
use ironsmith_core::{ChoiceCount, TagKey, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum DamageBySpec {
    ThisCreature,
    EquippedCreature,
    EnchantedCreature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum PlayerAst {
    You,
    Active,
    Any,
    Chosen,
    Defending,
    Attacking,
    MostCardsInHand,
    MostLifeTied,
    LowestLifeTied,
    Target,
    TargetOpponent,
    Opponent,
    PlayerToYourLeft,
    PlayerToYourRight,
    /// The player enchanted by this Aura or Curse.
    Enchanted,
    /// "a teammate" — only meaningful in multiplayer team formats.
    Teammate,
    NotYou,
    That,
    ThatPlayerOrTargetController,
    /// The controller of the event source captured by a triggered ability.
    TriggeringSourceController,
    ItsController,
    ItsOwner,
    Implicit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum ReturnControllerAst {
    Preserve,
    Owner,
    You,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum LibraryConsultModeAst {
    Reveal,
    Exile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum LibraryConsultStopRuleAst<Value = crate::effect::Value> {
    FirstMatch,
    MatchCount(Value),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum LibraryBottomOrderAst {
    Random,
    ChooserChooses,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum ObjectRefAst<Tag = TagKey> {
    Tagged(Tag),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub struct SearchLibrarySlotAst<Filter = crate::target::ObjectFilter> {
    pub filter: Filter,
    pub optional: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum ZoneReplacementDurationAst {
    OneShot,
    UntilEndOfTurn,
    Persistent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum FutureZoneReplacementCausePolicyAst {
    /// Match zone changes regardless of what caused them.
    Any,
    /// Match only when the changed object is also the source of the effect-like cause.
    ChangedObjectIsCause,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum ControlDurationAst {
    UntilEndOfTurn,
    UntilYourNextTurnEnd,
    DuringNextTurn,
    AsLongAsYouControlSource,
    Forever,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum ExtraTurnAnchorAst {
    CurrentTurn,
    ReferencedTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum SharedTypeConstraintAst {
    CardType,
    PermanentType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum ExchangeValueKindAst {
    Power,
    Toughness,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
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
#[allow(
    clippy::large_enum_variant,
    reason = "boxing the count-bearing target value would add indirection to a pervasive canonical AST API"
)]
#[derive(TagKeyWalk)]
pub enum TargetAst<
    PlayerFilter = crate::target::PlayerFilter,
    ObjectFilter = crate::target::ObjectFilter,
    Tag = TagKey,
> {
    Source(#[tag_walk(skip)] Option<TextSpan>),
    AnyTarget(#[tag_walk(skip)] Option<TextSpan>),
    AnyOtherTarget(#[tag_walk(skip)] Option<TextSpan>),
    ObjectOrPlayer(ObjectFilter, PlayerFilter, #[tag_walk(skip)] Option<TextSpan>),
    PlayerOrPlaneswalker(PlayerFilter, #[tag_walk(skip)] Option<TextSpan>),
    AttackedPlayerOrPlaneswalker(#[tag_walk(skip)] Option<TextSpan>),
    Spell(#[tag_walk(skip)] Option<TextSpan>),
    Player(PlayerFilter, #[tag_walk(skip)] Option<TextSpan>),
    Object(ObjectFilter, #[tag_walk(skip)] Option<TextSpan>, #[tag_walk(skip)] Option<TextSpan>),
    Tagged(Tag, #[tag_walk(skip)] Option<TextSpan>),
    WithCount(Box<TargetAst<PlayerFilter, ObjectFilter, Tag>>, ChoiceCount),
    WithCountValue(
        Box<TargetAst<PlayerFilter, ObjectFilter, Tag>>,
        ChoiceCount,
        Value,
    ),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum RetargetModeAst<
    Target = TargetAst<crate::target::PlayerFilter, crate::target::ObjectFilter>,
> {
    All,
    OneToFixed { target: Target },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum PreventNextTimeDamageSourceAst<
    Filter = crate::target::ObjectFilter,
    Target = TargetAst<crate::target::PlayerFilter, crate::target::ObjectFilter>,
> {
    Choice,
    Target(Target),
    Filter(Filter),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum RedirectNextTimeDamageDestinationAst {
    SourceObject,
    Controller,
    SourceController,
    TargetObject,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(
    clippy::large_enum_variant,
    reason = "this canonical AST wrapper intentionally embeds the complete target vocabulary"
)]
#[derive(TagKeyWalk)]
pub enum PreventNextTimeDamageTargetAst {
    AnyTarget,
    You,
    Target(TargetAst),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
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
