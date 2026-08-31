use winnow::combinator::{alt, opt, peek, repeat, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use crate::types::CardType;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind};
use super::super::primitives;
use super::nearby_primitives::{
    semantic_all, semantic_finish, semantic_kw, semantic_noise, semantic_phrase,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivatedAbilitySpecialSubject {
    ChosenName,
    TwoCardTypes(CardType, CardType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicPlayerKind {
    You,
    Opponent,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EarlyStaticMarkerKind {
    XMaximumPlayerCount,
    XMinimumOne,
    ExhaustAsUnactivated,
    CantAttackWithoutCreatureSpell,
    CantAttackWithoutNoncreatureSpell,
    DayNightStartsDay,
    LivingMetal,
    VehicleRulesMarker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticTextMarkerKind {
    Banding,
    AuraRetentionClarification,
    YouHaveHexproof,
    YouHaveProtectionFromOpponents,
    OpponentsCastOnlyAsSorcery,
    DoubleDamageToEnchantedPlayer,
}

pub fn parse_activated_ability_special_subject_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ActivatedAbilitySpecialSubject> {
    crate::grammar::primitives::probe_all(
        tokens,
        (
            alt((
                semantic_phrase(&["sources", "with", "chosen", "name"])
                    .value(ActivatedAbilitySpecialSubject::ChosenName),
                parse_two_card_type_subject,
            )),
            semantic_finish,
        )
            .map(|(subject, ())| subject),
        "activated-ability special subject",
    )
}

pub fn parse_cards_drawn_this_turn_player_tokens(
    tokens: &[OwnedLexToken],
) -> Option<DynamicPlayerKind> {
    let mut input = LexStream::new(tokens);
    let (_, player) = crate::grammar::primitives::take_leaf(
        &mut input,
        repeat_till::<_, _, (), _, _, _, _>(
            0..,
            any.void(),
            alt((
                parse_you_reference.value(DynamicPlayerKind::You),
                parse_opponent_reference.value(DynamicPlayerKind::Opponent),
            )),
        ),
    )?;
    find_semantic(tokens, || alt((semantic_kw("card"), semantic_kw("cards"))))?;
    find_semantic(tokens, || semantic_kw("drawn"))?;
    find_semantic(tokens, || semantic_phrase(&["this", "turn"]))?;
    Some(player)
}

pub fn parse_spell_cast_this_turn_player_tokens(
    tokens: &[OwnedLexToken],
) -> Option<DynamicPlayerKind> {
    find_semantic(tokens, || {
        alt((semantic_kw("spell"), semantic_kw("spells")))
    })?;
    find_semantic(tokens, || alt((semantic_kw("cast"), semantic_kw("casts"))))?;
    find_semantic(tokens, || semantic_phrase(&["this", "turn"]))?;
    if find_semantic(tokens, || parse_you_reference).is_some() {
        Some(DynamicPlayerKind::You)
    } else if find_semantic(tokens, || parse_opponent_reference).is_some() {
        Some(DynamicPlayerKind::Opponent)
    } else {
        Some(DynamicPlayerKind::Any)
    }
}

pub fn parse_early_static_marker_tokens(tokens: &[OwnedLexToken]) -> Option<EarlyStaticMarkerKind> {
    if parse_living_metal_tokens(tokens) {
        return Some(EarlyStaticMarkerKind::LivingMetal);
    }

    let exact = alt((
        semantic_phrase(&[
            "x", "cant", "be", "greater", "than", "number", "of", "players", "in", "game",
        ])
        .value(EarlyStaticMarkerKind::XMaximumPlayerCount),
        semantic_phrase(&["x", "cant", "be", "0"]).value(EarlyStaticMarkerKind::XMinimumOne),
        semantic_phrase(&[
            "during",
            "your",
            "turn",
            "as",
            "long",
            "as",
            "you",
            "havent",
            "activated",
            "exhaust",
            "ability",
            "this",
            "turn",
            "you",
            "may",
            "activate",
            "exhaust",
            "abilities",
            "as",
            "though",
            "they",
            "havent",
            "been",
            "activated",
        ])
        .value(EarlyStaticMarkerKind::ExhaustAsUnactivated),
        parse_cant_attack_spell_marker("creature")
            .value(EarlyStaticMarkerKind::CantAttackWithoutCreatureSpell),
        parse_cant_attack_spell_marker("noncreature")
            .value(EarlyStaticMarkerKind::CantAttackWithoutNoncreatureSpell),
        parse_vehicle_toughness_marker.value(EarlyStaticMarkerKind::VehicleRulesMarker),
    ));
    if let Ok(kind) = primitives::parse_all(
        tokens,
        (exact, semantic_finish).map(|(kind, ())| kind),
        "early static exact marker",
    ) {
        return Some(kind);
    }
    if parse_day_night_marker(tokens) {
        return Some(EarlyStaticMarkerKind::DayNightStartsDay);
    }
    parse_vehicle_prefix_suffix_marker(tokens).then_some(EarlyStaticMarkerKind::VehicleRulesMarker)
}

fn parse_living_metal_tokens(tokens: &[OwnedLexToken]) -> bool {
    let Some(((), tail)) = primitives::parse_prefix(tokens, semantic_phrase(&["living", "metal"]))
    else {
        return false;
    };

    tail.is_empty()
        || tail
            .first()
            .is_some_and(|token| matches!(token.kind, TokenKind::LParen | TokenKind::Period))
}

pub fn parse_static_text_marker_kind_tokens(
    tokens: &[OwnedLexToken],
) -> Option<StaticTextMarkerKind> {
    crate::grammar::primitives::probe_all(
        tokens,
        (
            alt((
                semantic_kw("banding").value(StaticTextMarkerKind::Banding),
                (
                    semantic_phrase(&["this", "effect"]),
                    alt((semantic_kw("doesnt"), semantic_kw("doesn't"))),
                    semantic_phrase(&["remove", "this", "aura"]),
                )
                    .value(StaticTextMarkerKind::AuraRetentionClarification),
                semantic_phrase(&["you", "have", "hexproof"])
                    .value(StaticTextMarkerKind::YouHaveHexproof),
                semantic_phrase(&[
                    "you",
                    "have",
                    "protection",
                    "from",
                    "each",
                    "of",
                    "your",
                    "opponents",
                ])
                .value(StaticTextMarkerKind::YouHaveProtectionFromOpponents),
                semantic_phrase(&[
                    "each", "opponent", "can", "cast", "spells", "only", "any", "time", "they",
                    "could", "cast", "sorcery",
                ])
                .value(StaticTextMarkerKind::OpponentsCastOnlyAsSorcery),
                semantic_phrase(&[
                    "if",
                    "source",
                    "would",
                    "deal",
                    "damage",
                    "to",
                    "enchanted",
                    "player",
                    "it",
                    "deals",
                    "double",
                    "that",
                    "damage",
                    "to",
                    "that",
                    "player",
                    "instead",
                ])
                .value(StaticTextMarkerKind::DoubleDamageToEnchantedPlayer),
            )),
            semantic_finish,
        )
            .map(|(kind, ())| kind),
        "static text marker kind",
    )
}

pub fn parse_revealed_hand_as_enters_tail_tokens(tokens: &[OwnedLexToken]) -> bool {
    semantic_all(
        tokens,
        semantic_phrase(&["each", "opponent", "reveals", "their", "hand"]),
        "revealed-hand as-enters tail",
    )
}

pub fn parse_choose_revealed_nonland_name_tail_tokens(tokens: &[OwnedLexToken]) -> bool {
    semantic_all(
        tokens,
        semantic_phrase(&[
            "you", "choose", "name", "of", "nonland", "card", "revealed", "this", "way",
        ]),
        "choose revealed nonland name tail",
    )
}

pub fn parse_trigger_duplication_tail_tokens(tokens: &[OwnedLexToken]) -> bool {
    semantic_all(
        tokens,
        (
            alt((semantic_kw("it"), semantic_phrase(&["that", "ability"]))),
            alt((
                semantic_phrase(&["triggers", "additional", "time"]),
                semantic_phrase(&["triggers", "one", "additional", "time"]),
            )),
        )
            .void(),
        "trigger duplication tail",
    )
}

fn parse_two_card_type_subject<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ActivatedAbilitySpecialSubject> {
    let left = parse_card_type_lexed(input)?;
    semantic_kw("and").parse_next(input)?;
    let right = parse_card_type_lexed(input)?;
    Ok(ActivatedAbilitySpecialSubject::TwoCardTypes(left, right))
}

fn parse_card_type_lexed<'a>(input: &mut LexStream<'a>) -> WResult<CardType> {
    repeat::<_, _, (), _, _>(0.., semantic_noise).parse_next(input)?;
    alt((
        alt((semantic_kw("artifact"), semantic_kw("artifacts"))).value(CardType::Artifact),
        alt((semantic_kw("battle"), semantic_kw("battles"))).value(CardType::Battle),
        alt((semantic_kw("creature"), semantic_kw("creatures"))).value(CardType::Creature),
        alt((semantic_kw("enchantment"), semantic_kw("enchantments"))).value(CardType::Enchantment),
        alt((semantic_kw("instant"), semantic_kw("instants"))).value(CardType::Instant),
        alt((semantic_kw("land"), semantic_kw("lands"))).value(CardType::Land),
        alt((semantic_kw("planeswalker"), semantic_kw("planeswalkers")))
            .value(CardType::Planeswalker),
        alt((semantic_kw("sorcery"), semantic_kw("sorceries"))).value(CardType::Sorcery),
    ))
    .parse_next(input)
}

fn parse_you_reference<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        semantic_kw("you"),
        semantic_kw("your"),
        semantic_kw("youve"),
        semantic_kw("you've"),
    ))
    .void()
    .parse_next(input)
}

fn parse_opponent_reference<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((semantic_kw("opponent"), semantic_kw("opponents")))
        .void()
        .parse_next(input)
}

fn parse_cant_attack_spell_marker<'a>(
    spell_kind: &'static str,
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    move |input: &mut LexStream<'a>| {
        semantic_kw("this").parse_next(input)?;
        opt(semantic_kw("creature")).parse_next(input)?;
        semantic_phrase(&["cant", "attack", "unless", "youve", "cast"]).parse_next(input)?;
        semantic_kw(spell_kind).parse_next(input)?;
        semantic_phrase(&["spell", "this", "turn"]).parse_next(input)
    }
}

fn parse_vehicle_toughness_marker<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    semantic_phrase(&["this", "creature"]).parse_next(input)?;
    opt(semantic_phrase(&["saddles", "mounts", "and"])).parse_next(input)?;
    semantic_phrase(&[
        "crews",
        "vehicles",
        "using",
        "its",
        "toughness",
        "rather",
        "than",
        "its",
        "power",
    ])
    .parse_next(input)
}

fn parse_day_night_marker(tokens: &[OwnedLexToken]) -> bool {
    find_semantic(tokens, || {
        alt((
            semantic_phrase(&["its", "neither", "day", "nor", "night"]),
            semantic_phrase(&["it's", "neither", "day", "nor", "night"]),
        ))
    })
    .is_some()
        && find_semantic(tokens, || semantic_phrase(&["it", "becomes", "day"])).is_some()
        && find_semantic(tokens, || {
            (
                semantic_phrase(&["as", "this"]),
                alt((
                    semantic_kw("creature"),
                    semantic_kw("permanent"),
                    semantic_kw("object"),
                )),
                semantic_kw("enters"),
            )
                .void()
        })
        .is_some()
}

fn parse_vehicle_prefix_suffix_marker(tokens: &[OwnedLexToken]) -> bool {
    if crate::grammar::token_definitions::parse_token_power_as_though_greater_shape_tokens(tokens)
        .is_some()
    {
        return true;
    }
    primitives::parse_all(
        tokens,
        (
            semantic_phrase(&[
                "you",
                "may",
                "remove",
                "loyalty",
                "counter",
                "from",
                "planeswalker",
                "you",
                "control",
                "rather",
                "than",
                "pay",
            ]),
            repeat_till::<_, _, (), _, _, _, _>(
                0..,
                any.void(),
                peek(semantic_phrase(&["crew", "cost"])),
            )
            .void(),
            semantic_phrase(&["crew", "cost"]),
            semantic_finish,
        )
            .void(),
        "loyalty crew-cost marker",
    )
    .is_ok()
}

fn find_semantic<'a, P, O, F>(tokens: &'a [OwnedLexToken], make_parser: F) -> Option<O>
where
    F: Fn() -> P,
    P: Parser<LexStream<'a>, O, ErrMode<ContextError>>,
{
    primitives::find_prefix(tokens, make_parser).map(|(_, parsed, _)| parsed)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_subjects_and_dynamic_players() {
        let tokens = lex_line("artifacts and creatures", 0).unwrap();
        assert_eq!(
            parse_activated_ability_special_subject_tokens(&tokens),
            Some(ActivatedAbilitySpecialSubject::TwoCardTypes(
                CardType::Artifact,
                CardType::Creature,
            ))
        );
        let tokens = lex_line("the number of cards you've drawn this turn", 0).unwrap();
        assert_eq!(
            parse_cards_drawn_this_turn_player_tokens(&tokens),
            Some(DynamicPlayerKind::You)
        );
    }

    #[test]
    fn parses_early_and_static_markers() {
        let tokens = lex_line("X can't be 0.", 0).unwrap();
        assert_eq!(
            parse_early_static_marker_tokens(&tokens),
            Some(EarlyStaticMarkerKind::XMinimumOne)
        );
        let tokens = lex_line("You have hexproof.", 0).unwrap();
        assert_eq!(
            parse_static_text_marker_kind_tokens(&tokens),
            Some(StaticTextMarkerKind::YouHaveHexproof)
        );
        let tokens = lex_line("This effect doesn't remove this Aura.", 0).unwrap();
        assert_eq!(
            parse_static_text_marker_kind_tokens(&tokens),
            Some(StaticTextMarkerKind::AuraRetentionClarification)
        );

        let tokens = lex_line(
            "This creature saddles Mounts and crews Vehicles as though its power were 2 greater.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_early_static_marker_tokens(&tokens),
            Some(EarlyStaticMarkerKind::VehicleRulesMarker)
        );

        for text in [
            "Living metal",
            "Living metal (During your turn, this Vehicle is also a creature.)",
        ] {
            let tokens = lex_line(text, 0).unwrap();
            assert_eq!(
                parse_early_static_marker_tokens(&tokens),
                Some(EarlyStaticMarkerKind::LivingMetal)
            );
        }
    }
}
