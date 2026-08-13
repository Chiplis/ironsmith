//! Typed grammar for source attack restrictions with an `unless` requirement.
//!
//! The direct-cant cluster already owns the max-speed attack-or-block surface;
//! this parser intentionally declines that line so there is one semantic owner.

use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::effect::{Value, ValueComparisonOperator};
use crate::filter::StackObjectKind;
use crate::mana::ManaSymbol;
use crate::object::CounterType;
use crate::grammar::{conditions, filters, leaf, primitives};
use crate::front_end::lexer::{LexStream, OwnedLexToken};
use crate::static_abilities::{
    AttackCostCondition, AttackingGroupAttackCondition, CantAttackUnlessConditionSpec,
    DefendingPlayerAttackCondition,
};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AttackUnlessScope {
    Attack,
    AttackOrBlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AttackUnlessSurface {
    ControllerCastCreatureSpellThisTurn,
    ControllerCastNoncreatureSpellThisTurn,
    ControllerControlsMoreCreatures,
    ControllerControlsMoreLands,
    ControllerControlCondition,
    MountainOnBattlefield,
    ControllerGraveyardCount,
    IslandsOnBattlefield,
    CardsInExile,
    DefendingPlayerPoisoned,
    DefendingPlayerGraveyardCount,
    DefendingPlayerControlsEnchantmentOrEnchantedPermanent,
    DefendingPlayerControls,
    DefendingPlayerMonarch,
    OtherCreaturesAttack,
    CreatureWithGreaterPowerAttacks,
    BlackOrGreenCreatureAttacks,
    OpponentDealtDamageThisTurn,
    SacrificeLand,
    SacrificeIslands,
    ReturnEnchantment,
    PayPerPlusOnePlusOneCounter,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AttackUnlessConditionFact<'a> {
    pub(crate) scope: AttackUnlessScope,
    pub(crate) surface: AttackUnlessSurface,
    pub(crate) condition: CantAttackUnlessConditionSpec,
    pub(crate) display_tokens: &'a [OwnedLexToken],
    pub(crate) tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
struct AttackUnlessLineCapture<'a> {
    scope: AttackUnlessScope,
    display_tokens: &'a [OwnedLexToken],
    tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
struct ParsedRequirement {
    surface: AttackUnlessSurface,
    condition: CantAttackUnlessConditionSpec,
}

pub(crate) fn parse_attack_unless_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AttackUnlessConditionFact<'_>> {
    let capture =
        primitives::parse_all(tokens, parse_attack_unless_line_lexed, "attack-unless-line").ok()?;

    if capture.scope == AttackUnlessScope::AttackOrBlock
        && primitives::parse_all(
            capture.tail_tokens,
            parse_max_speed_tail_lexed,
            "reserved-max-speed-tail",
        )
        .is_ok()
    {
        return None;
    }

    let parsed = primitives::parse_all(
        capture.tail_tokens,
        move |input: &mut LexStream<'_>| parse_requirement_lexed(input, capture.scope),
        "attack-unless-requirement",
    )
    .ok()?;

    Some(AttackUnlessConditionFact {
        scope: capture.scope,
        surface: parsed.surface,
        condition: parsed.condition,
        display_tokens: capture.display_tokens,
        tail_tokens: capture.tail_tokens,
    })
}

fn parse_attack_unless_line_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AttackUnlessLineCapture<'a>> {
    let ((scope, tail_tokens), display_tokens) = (
        parse_attack_unless_head_lexed,
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
            .map(|((), ())| ())
            .take(),
    )
        .with_taken()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(AttackUnlessLineCapture {
        scope,
        display_tokens,
        tail_tokens,
    })
}

fn parse_attack_unless_head_lexed(input: &mut LexStream<'_>) -> WResult<AttackUnlessScope> {
    parse_source_subject_lexed.parse_next(input)?;
    parse_cant_lexed.parse_next(input)?;
    alt((
        primitives::phrase(&["attack", "or", "block", "unless"])
            .value(AttackUnlessScope::AttackOrBlock),
        primitives::phrase(&["attack", "unless"]).value(AttackUnlessScope::Attack),
    ))
    .parse_next(input)
}

fn parse_requirement_lexed(
    input: &mut LexStream<'_>,
    scope: AttackUnlessScope,
) -> WResult<ParsedRequirement> {
    match scope {
        AttackUnlessScope::Attack => alt((
            parse_cast_spell_this_turn,
            parse_controls_more,
            parse_mountain_present,
            parse_there_are_count,
            parse_defending_player_requirement,
            parse_attacking_group_requirement,
            parse_opponent_damaged,
            parse_attack_cost_requirement,
            parse_controller_control_requirement,
        ))
        .parse_next(input),
        AttackUnlessScope::AttackOrBlock => alt((
            parse_counted_controller_control_requirement,
            parse_there_are_exile_count,
        ))
        .parse_next(input),
    }
}

fn parse_cast_spell_this_turn(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    alt((
        parse_cast_creature_spell_this_turn,
        parse_cast_noncreature_spell_this_turn,
    ))
    .parse_next(input)
}

fn parse_cast_creature_spell_this_turn(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    parse_youve_cast.parse_next(input)?;
    opt(parse_indefinite_article).parse_next(input)?;
    primitives::phrase(&["creature", "spell", "this", "turn"]).parse_next(input)?;
    Ok(ParsedRequirement {
        surface: AttackUnlessSurface::ControllerCastCreatureSpellThisTurn,
        condition: CantAttackUnlessConditionSpec::SourceCondition(spell_cast_condition(false)),
    })
}

fn parse_cast_noncreature_spell_this_turn(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    parse_youve_cast.parse_next(input)?;
    opt(parse_indefinite_article).parse_next(input)?;
    primitives::phrase(&["noncreature", "spell", "this", "turn"]).parse_next(input)?;
    Ok(ParsedRequirement {
        surface: AttackUnlessSurface::ControllerCastNoncreatureSpellThisTurn,
        condition: CantAttackUnlessConditionSpec::SourceCondition(spell_cast_condition(true)),
    })
}

fn parse_controls_more(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    primitives::phrase(&["you", "control", "more"]).parse_next(input)?;
    let (surface, filter) = alt((
        primitives::phrase(&["creatures", "than", "defending", "player"]).value((
            AttackUnlessSurface::ControllerControlsMoreCreatures,
            ObjectFilter::creature(),
        )),
        primitives::phrase(&["lands", "than", "defending", "player"]).value((
            AttackUnlessSurface::ControllerControlsMoreLands,
            ObjectFilter::land(),
        )),
    ))
    .parse_next(input)?;
    Ok(ParsedRequirement {
        surface,
        condition: CantAttackUnlessConditionSpec::ControllerControlsMoreThanDefendingPlayer(filter),
    })
}

fn parse_mountain_present(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    primitives::phrase(&["there", "is"]).parse_next(input)?;
    opt(parse_indefinite_article).parse_next(input)?;
    primitives::kw("mountain").parse_next(input)?;
    primitives::kw("on").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::kw("battlefield").parse_next(input)?;
    Ok(ParsedRequirement {
        surface: AttackUnlessSurface::MountainOnBattlefield,
        condition: CantAttackUnlessConditionSpec::BattlefieldCountAtLeast {
            filter: ObjectFilter::land().with_subtype(Subtype::Mountain),
            count: 1,
        },
    })
}

fn parse_there_are_count(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    primitives::phrase(&["there", "are"]).parse_next(input)?;
    let count = parse_minimum_count_lexed.parse_next(input)?;
    alt((
        primitives::phrase(&["cards", "in", "your", "graveyard"]).value(ParsedRequirement {
            surface: AttackUnlessSurface::ControllerGraveyardCount,
            condition: CantAttackUnlessConditionSpec::ControllerGraveyardHasCardsAtLeast(count),
        }),
        (
            primitives::kw("islands"),
            primitives::kw("on"),
            opt(primitives::kw("the")),
            primitives::kw("battlefield"),
        )
            .value(ParsedRequirement {
                surface: AttackUnlessSurface::IslandsOnBattlefield,
                condition: CantAttackUnlessConditionSpec::BattlefieldCountAtLeast {
                    filter: ObjectFilter::land().with_subtype(Subtype::Island),
                    count,
                },
            }),
    ))
    .parse_next(input)
}

fn parse_there_are_exile_count(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    primitives::phrase(&["there", "are"]).parse_next(input)?;
    let count = parse_minimum_count_lexed.parse_next(input)?;
    primitives::phrase(&["cards", "in", "exile"]).parse_next(input)?;
    let filter = ObjectFilter::default().in_zone(Zone::Exile).nontoken();
    Ok(ParsedRequirement {
        surface: AttackUnlessSurface::CardsInExile,
        condition: CantAttackUnlessConditionSpec::SourceCondition(
            crate::ConditionExpr::ValueComparison {
                left: Value::Count(filter),
                operator: ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(count as i32),
            },
        ),
    })
}

fn parse_defending_player_requirement(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    alt((
        parse_defending_player_poisoned,
        parse_defending_player_graveyard_count,
        parse_defending_player_enchantment,
        parse_defending_player_monarch,
        parse_defending_player_controls,
    ))
    .parse_next(input)
}

fn parse_defending_player_poisoned(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    primitives::phrase(&["defending", "player", "is", "poisoned"]).parse_next(input)?;
    Ok(ParsedRequirement {
        surface: AttackUnlessSurface::DefendingPlayerPoisoned,
        condition: CantAttackUnlessConditionSpec::DefendingPlayerCondition(
            DefendingPlayerAttackCondition::IsPoisoned,
        ),
    })
}

fn parse_defending_player_graveyard_count(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    primitives::phrase(&["defending", "player", "has"]).parse_next(input)?;
    let count = parse_minimum_count_lexed.parse_next(input)?;
    primitives::phrase(&["cards", "in", "their", "graveyard"]).parse_next(input)?;
    Ok(ParsedRequirement {
        surface: AttackUnlessSurface::DefendingPlayerGraveyardCount,
        condition: CantAttackUnlessConditionSpec::DefendingPlayerCondition(
            DefendingPlayerAttackCondition::HasCardsInGraveyardOrMore(count),
        ),
    })
}

fn parse_defending_player_enchantment(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    primitives::phrase(&["defending", "player", "controls"]).parse_next(input)?;
    opt(parse_indefinite_article).parse_next(input)?;
    primitives::kw("enchantment").parse_next(input)?;
    primitives::kw("or").parse_next(input)?;
    opt(parse_indefinite_article).parse_next(input)?;
    primitives::phrase(&["enchanted", "permanent"]).parse_next(input)?;
    Ok(ParsedRequirement {
        surface: AttackUnlessSurface::DefendingPlayerControlsEnchantmentOrEnchantedPermanent,
        condition: CantAttackUnlessConditionSpec::DefendingPlayerCondition(
            DefendingPlayerAttackCondition::ControlsEnchantmentOrEnchantedPermanent,
        ),
    })
}

fn parse_defending_player_monarch(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    primitives::phrase(&["defending", "player", "is"]).parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::kw("monarch").parse_next(input)?;
    Ok(ParsedRequirement {
        surface: AttackUnlessSurface::DefendingPlayerMonarch,
        condition: CantAttackUnlessConditionSpec::DefendingPlayerCondition(
            DefendingPlayerAttackCondition::IsMonarch,
        ),
    })
}

fn parse_defending_player_controls(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    primitives::phrase(&["defending", "player", "controls"]).parse_next(input)?;
    let filter_tokens = take_remaining_tokens(input)?;
    let mut filter = filters::parse_object_filter_with_grammar_entrypoint(filter_tokens, false)
        .map_err(|_| primitives::backtrack_err("defending player filter", "object filter"))?;
    if filter.zone.is_none() {
        filter.zone = Some(Zone::Battlefield);
    }
    Ok(ParsedRequirement {
        surface: AttackUnlessSurface::DefendingPlayerControls,
        condition: CantAttackUnlessConditionSpec::DefendingPlayerCondition(
            DefendingPlayerAttackCondition::Controls(filter),
        ),
    })
}

fn parse_attacking_group_requirement(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    alt((
        parse_other_creatures_attack,
        parse_greater_power_attacks,
        parse_black_or_green_attacks,
    ))
    .parse_next(input)
}

fn parse_other_creatures_attack(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    let count = parse_minimum_count_lexed.parse_next(input)?;
    primitives::phrase(&["other", "creatures", "attack"]).parse_next(input)?;
    Ok(ParsedRequirement {
        surface: AttackUnlessSurface::OtherCreaturesAttack,
        condition: CantAttackUnlessConditionSpec::AttackingGroupCondition(
            AttackingGroupAttackCondition::AtLeastNOtherCreaturesAttack(count),
        ),
    })
}

fn parse_greater_power_attacks(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    opt(parse_indefinite_article).parse_next(input)?;
    primitives::phrase(&["creature", "with", "greater", "power", "also", "attacks"])
        .parse_next(input)?;
    Ok(ParsedRequirement {
        surface: AttackUnlessSurface::CreatureWithGreaterPowerAttacks,
        condition: CantAttackUnlessConditionSpec::AttackingGroupCondition(
            AttackingGroupAttackCondition::CreatureWithGreaterPowerAlsoAttacks,
        ),
    })
}

fn parse_black_or_green_attacks(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    opt(parse_indefinite_article).parse_next(input)?;
    primitives::phrase(&["black", "or", "green", "creature", "also", "attacks"])
        .parse_next(input)?;
    Ok(ParsedRequirement {
        surface: AttackUnlessSurface::BlackOrGreenCreatureAttacks,
        condition: CantAttackUnlessConditionSpec::AttackingGroupCondition(
            AttackingGroupAttackCondition::BlackOrGreenCreatureAlsoAttacks,
        ),
    })
}

fn parse_opponent_damaged(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    opt(parse_indefinite_article).parse_next(input)?;
    primitives::phrase(&["opponent", "has", "been", "dealt", "damage", "this", "turn"])
        .parse_next(input)?;
    Ok(ParsedRequirement {
        surface: AttackUnlessSurface::OpponentDealtDamageThisTurn,
        condition: CantAttackUnlessConditionSpec::OpponentWasDealtDamageThisTurn,
    })
}

fn parse_attack_cost_requirement(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    alt((
        parse_sacrifice_islands,
        parse_sacrifice_land,
        parse_return_enchantment,
        parse_pay_per_counter,
    ))
    .parse_next(input)
}

fn parse_sacrifice_land(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    primitives::phrase(&["you", "sacrifice"]).parse_next(input)?;
    opt(parse_indefinite_article).parse_next(input)?;
    primitives::kw("land").parse_next(input)?;
    Ok(ParsedRequirement {
        surface: AttackUnlessSurface::SacrificeLand,
        condition: CantAttackUnlessConditionSpec::AttackCost(
            AttackCostCondition::SacrificePermanents {
                filter: ObjectFilter::land(),
                count: 1,
            },
        ),
    })
}

fn parse_sacrifice_islands(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    primitives::phrase(&["you", "sacrifice"]).parse_next(input)?;
    let count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    alt((primitives::kw("island"), primitives::kw("islands"))).parse_next(input)?;
    Ok(ParsedRequirement {
        surface: AttackUnlessSurface::SacrificeIslands,
        condition: CantAttackUnlessConditionSpec::AttackCost(
            AttackCostCondition::SacrificePermanents {
                filter: ObjectFilter::land().with_subtype(Subtype::Island),
                count,
            },
        ),
    })
}

fn parse_return_enchantment(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    primitives::phrase(&["you", "return"]).parse_next(input)?;
    opt(parse_indefinite_article).parse_next(input)?;
    primitives::phrase(&["enchantment", "you", "control", "to", "its"]).parse_next(input)?;
    alt((
        primitives::kw("owner's").void(),
        primitives::kw("owners").void(),
        primitives::phrase(&["owner", "s"]),
    ))
    .parse_next(input)?;
    primitives::kw("hand").parse_next(input)?;
    Ok(ParsedRequirement {
        surface: AttackUnlessSurface::ReturnEnchantment,
        condition: CantAttackUnlessConditionSpec::AttackCost(
            AttackCostCondition::ReturnPermanentsToOwnersHand {
                filter: ObjectFilter::enchantment(),
                count: 1,
            },
        ),
    })
}

fn parse_pay_per_counter(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    primitives::phrase(&["you", "pay"]).parse_next(input)?;
    leaf::parse_leaf_surface_mana_pip_lexed
        .verify(|pip| match pip {
            leaf::LeafManaPipToken::ManaGroup(symbols) => {
                symbols.first().copied() == Some(ManaSymbol::Generic(1)) && symbols.get(1).is_none()
            }
            leaf::LeafManaPipToken::LegacyBare(symbol) => *symbol == ManaSymbol::Generic(1),
        })
        .void()
        .parse_next(input)?;
    primitives::phrase(&["for", "each"]).parse_next(input)?;
    let counter_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::kw("counter")))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
    primitives::kw("counter").parse_next(input)?;
    primitives::phrase(&["on", "it"]).parse_next(input)?;
    if filters::parse_counter_type_from_tokens(counter_tokens) != Some(CounterType::PlusOnePlusOne)
    {
        return Err(primitives::backtrack_err(
            "attack counter payment",
            "+1/+1 counter",
        ));
    }
    Ok(ParsedRequirement {
        surface: AttackUnlessSurface::PayPerPlusOnePlusOneCounter,
        condition: CantAttackUnlessConditionSpec::AttackCost(
            AttackCostCondition::PayGenericPerSourceCounter {
                counter_type: CounterType::PlusOnePlusOne,
                amount_per_counter: 1,
            },
        ),
    })
}

fn parse_controller_control_requirement(input: &mut LexStream<'_>) -> WResult<ParsedRequirement> {
    parse_controller_control_requirement_inner(input, false)
}

fn parse_counted_controller_control_requirement(
    input: &mut LexStream<'_>,
) -> WResult<ParsedRequirement> {
    parse_controller_control_requirement_inner(input, true)
}

fn parse_controller_control_requirement_inner(
    input: &mut LexStream<'_>,
    require_explicit_quantity: bool,
) -> WResult<ParsedRequirement> {
    peek(primitives::phrase(&["you", "control"])).parse_next(input)?;
    let control_tokens = take_remaining_tokens(input)?;
    let parsed = conditions::parse_control_condition(
        control_tokens,
        conditions::ControlConditionOptions {
            allow_that_player: false,
            allow_opponent_players: false,
            allow_defending_player: false,
            bind_filter_controller_to_subject: true,
            allow_different_powers_tail: false,
            default_filter_zone: Some(Zone::Battlefield),
        },
    )
    .ok_or_else(|| primitives::backtrack_err("controller condition", "you control filter"))?;
    if require_explicit_quantity && !parsed.has_explicit_quantity() {
        return Err(primitives::backtrack_err(
            "controller condition",
            "explicit controlled-object quantity",
        ));
    }
    let count = parsed
        .at_least_count()
        .ok_or_else(|| primitives::backtrack_err("controller count", "minimum count"))?;
    let condition = if count > 1 || require_explicit_quantity {
        crate::ConditionExpr::PlayerHasAtLeast {
            player: parsed.player_filter.unwrap_or(PlayerFilter::You),
            filter: parsed.filter,
            count,
        }
    } else {
        crate::ConditionExpr::YouControl(parsed.filter)
    };
    Ok(ParsedRequirement {
        surface: AttackUnlessSurface::ControllerControlCondition,
        condition: CantAttackUnlessConditionSpec::SourceCondition(condition),
    })
}

fn parse_minimum_count_lexed(input: &mut LexStream<'_>) -> WResult<u32> {
    alt((
        (
            primitives::phrase(&["at", "least"]),
            leaf::parse_leaf_number_prefix_lexed,
        )
            .map(|(_, count)| count),
        parse_greater_than_count_lexed,
        (
            leaf::parse_leaf_number_prefix_lexed,
            opt(primitives::phrase(&["or", "more"])),
        )
            .map(|(count, _)| count),
    ))
    .parse_next(input)
}

fn parse_greater_than_count_lexed(input: &mut LexStream<'_>) -> WResult<u32> {
    primitives::phrase(&["greater", "than"]).parse_next(input)?;
    let count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    count.checked_add(1).ok_or_else(|| {
        primitives::backtrack_err("minimum count", "a non-overflowing numeric count")
    })
}

fn take_remaining_tokens<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(eof.void()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)
}

fn parse_max_speed_tail_lexed(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::phrase(&["you", "have", "max", "speed"]).parse_next(input)
}

fn parse_source_subject_lexed(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::phrase(&["this", "creature"]),
        primitives::kw("this").void(),
    ))
    .parse_next(input)
}

fn parse_cant_lexed(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::kw("can't"),
        primitives::kw("cant"),
        primitives::kw("cannot"),
    ))
    .void()
    .parse_next(input)
}

fn parse_youve_cast(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::phrase(&["youve", "cast"]),
        primitives::phrase(&["you", "ve", "cast"]),
        primitives::phrase(&["you've", "cast"]),
    ))
    .parse_next(input)
}

fn parse_indefinite_article(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("a"), primitives::kw("an")))
        .void()
        .parse_next(input)
}

fn spell_cast_condition(noncreature: bool) -> crate::ConditionExpr {
    let mut filter = ObjectFilter::default();
    filter.stack_kind = Some(StackObjectKind::Spell);
    if noncreature {
        filter.excluded_card_types.push(CardType::Creature);
    } else {
        filter.card_types.push(CardType::Creature);
    }
    crate::ConditionExpr::ValueComparison {
        left: Value::SpellsCastThisTurnMatching {
            player: PlayerFilter::You,
            filter,
            exclude_source: false,
        },
        operator: ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::{lex_line, parser_token_word_refs};

    fn parse(raw: &str) -> Option<AttackUnlessConditionFact<'static>> {
        let tokens = Box::leak(Box::new(
            lex_line(raw, 0).expect("lex attack-unless fixture"),
        ));
        parse_attack_unless_condition_tokens(tokens)
    }

    #[test]
    fn parses_attack_unless_surface_table() {
        let cases = [
            (
                "This creature can't attack unless you've cast a creature spell this turn.",
                AttackUnlessSurface::ControllerCastCreatureSpellThisTurn,
            ),
            (
                "This can't attack unless you've cast a noncreature spell this turn.",
                AttackUnlessSurface::ControllerCastNoncreatureSpellThisTurn,
            ),
            (
                "This creature can't attack unless you control more creatures than defending player.",
                AttackUnlessSurface::ControllerControlsMoreCreatures,
            ),
            (
                "This creature can't attack unless you control more lands than defending player.",
                AttackUnlessSurface::ControllerControlsMoreLands,
            ),
            (
                "This creature can't attack unless you control another creature with power 4 or greater.",
                AttackUnlessSurface::ControllerControlCondition,
            ),
            (
                "This creature can't attack unless there is a Mountain on the battlefield.",
                AttackUnlessSurface::MountainOnBattlefield,
            ),
            (
                "This creature can't attack unless there are seven or more cards in your graveyard.",
                AttackUnlessSurface::ControllerGraveyardCount,
            ),
            (
                "This creature can't attack unless there are three Islands on the battlefield.",
                AttackUnlessSurface::IslandsOnBattlefield,
            ),
            (
                "This creature can't attack or block unless there are seven or more cards in exile.",
                AttackUnlessSurface::CardsInExile,
            ),
            (
                "This creature can't attack or block unless you control seven or more lands.",
                AttackUnlessSurface::ControllerControlCondition,
            ),
            (
                "This creature can't attack unless defending player is poisoned.",
                AttackUnlessSurface::DefendingPlayerPoisoned,
            ),
            (
                "This creature can't attack unless defending player has five cards in their graveyard.",
                AttackUnlessSurface::DefendingPlayerGraveyardCount,
            ),
            (
                "This creature can't attack unless defending player controls an enchantment or an enchanted permanent.",
                AttackUnlessSurface::DefendingPlayerControlsEnchantmentOrEnchantedPermanent,
            ),
            (
                "This creature can't attack unless defending player controls an Island.",
                AttackUnlessSurface::DefendingPlayerControls,
            ),
            (
                "This creature can't attack unless defending player is the monarch.",
                AttackUnlessSurface::DefendingPlayerMonarch,
            ),
            (
                "This creature can't attack unless two other creatures attack.",
                AttackUnlessSurface::OtherCreaturesAttack,
            ),
            (
                "This creature can't attack unless a creature with greater power also attacks.",
                AttackUnlessSurface::CreatureWithGreaterPowerAttacks,
            ),
            (
                "This creature can't attack unless a black or green creature also attacks.",
                AttackUnlessSurface::BlackOrGreenCreatureAttacks,
            ),
            (
                "This creature can't attack unless an opponent has been dealt damage this turn.",
                AttackUnlessSurface::OpponentDealtDamageThisTurn,
            ),
            (
                "This creature can't attack unless you sacrifice a land.",
                AttackUnlessSurface::SacrificeLand,
            ),
            (
                "This creature can't attack unless you sacrifice two Islands.",
                AttackUnlessSurface::SacrificeIslands,
            ),
            (
                "This creature can't attack unless you return an enchantment you control to its owner's hand.",
                AttackUnlessSurface::ReturnEnchantment,
            ),
            (
                "This creature can't attack unless you pay {1} for each +1/+1 counter on it.",
                AttackUnlessSurface::PayPerPlusOnePlusOneCounter,
            ),
        ];

        for (raw, surface) in cases {
            let parsed = parse(raw).unwrap_or_else(|| panic!("fixture did not parse: {raw}"));
            assert_eq!(parsed.surface, surface, "fixture: {raw}");
            assert!(!parsed.tail_tokens.is_empty(), "fixture: {raw}");
            assert!(!parsed.display_tokens.is_empty(), "fixture: {raw}");
        }
    }

    #[test]
    fn declines_owned_max_speed_and_near_misses() {
        assert!(parse("This creature can't attack or block unless you have max speed.").is_none());
        for raw in [
            "This creature can attack unless you control a land.",
            "This creature can't attack if you control a land.",
            "This creature can't attack unless defending player controls.",
            "This creature can't attack unless you sacrifice Islands.",
            "This creature can't attack unless you pay {2} for each +1/+1 counter on it.",
            "This creature can't attack or block unless there are cards in exile.",
        ] {
            assert!(parse(raw).is_none(), "near miss: {raw}");
        }
    }

    #[test]
    fn captures_original_display_and_tail_boundaries() {
        let parsed = parse("This creature can't attack unless you control an artifact.").unwrap();
        assert_eq!(
            parser_token_word_refs(parsed.tail_tokens),
            ["you", "control", "an", "artifact"]
        );
        assert_eq!(
            parser_token_word_refs(parsed.display_tokens),
            [
                "this", "creature", "cant", "attack", "unless", "you", "control", "an", "artifact"
            ]
        );
    }
}
