use winnow::combinator::{alt, opt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, rest};

use crate::cards::builders::KeywordAction;
use crate::effect::Value;
use crate::mana::ManaSymbol;
use crate::static_abilities::StaticAbilityId;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenWordView, trim_lexed_commas};
use super::super::{filters, leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhereXSpecialNumberFilterKind {
    CreaturesDiedThisTurn,
    CommanderCastCount,
    CreaturesControlledByThosePlayers,
}

pub fn parse_where_x_special_number_filter_tokens(
    tokens: &[OwnedLexToken],
) -> Option<WhereXSpecialNumberFilterKind> {
    crate::grammar::primitives::probe_all(
        tokens,
        alt((
            (
                alt((primitives::kw("creature"), primitives::kw("creatures"))),
                primitives::phrase(&["that", "died", "this", "turn"]),
            )
                .value(WhereXSpecialNumberFilterKind::CreaturesDiedThisTurn),
            alt((
                primitives::phrase(&[
                    "times", "its", "been", "cast", "from", "the", "command", "zone", "this",
                    "game",
                ]),
                primitives::phrase(&[
                    "times", "it", "has", "been", "cast", "from", "the", "command", "zone", "this",
                    "game",
                ]),
                primitives::phrase(&[
                    "times",
                    "this",
                    "commander",
                    "has",
                    "been",
                    "cast",
                    "from",
                    "the",
                    "command",
                    "zone",
                    "this",
                    "game",
                ]),
                primitives::phrase(&[
                    "times",
                    "your",
                    "commander",
                    "has",
                    "been",
                    "cast",
                    "from",
                    "the",
                    "command",
                    "zone",
                    "this",
                    "game",
                ]),
            ))
            .value(WhereXSpecialNumberFilterKind::CommanderCastCount),
            (
                alt((primitives::kw("creature"), primitives::kw("creatures"))),
                primitives::phrase(&["those", "players", "control"]),
            )
                .value(WhereXSpecialNumberFilterKind::CreaturesControlledByThosePlayers),
        )),
        "special where-X number filter",
    )
}

pub fn parse_etb_static_ability_ids_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Vec<StaticAbilityId>> {
    let parsed = crate::grammar::primitives::probe_all(
        tokens,
        parse_etb_static_ability_ids_lexed,
        "ETB static ability list",
    )?;
    let mut unique = Vec::new();
    for ability in parsed {
        if !unique.iter().any(|existing| existing == &ability) {
            unique.push(ability);
        }
    }
    (!unique.is_empty()).then_some(unique)
}

pub fn parse_number_of_counters_on_source_value_tokens(tokens: &[OwnedLexToken]) -> Option<Value> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_number_of_counters_on_source_value_lexed,
        "number of counters on source value",
    )
}

pub fn parse_snow_mana_of_spell_color_condition_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        parse_snow_mana_of_spell_color_condition_lexed,
        "snow mana of a spell color condition",
    )
    .is_ok()
}

pub fn parse_pt_choice_keyword_action_words(words: &[&str]) -> Option<KeywordAction> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let action = crate::grammar::primitives::take_leaf(
        &mut input,
        alt((
            (
                primitives::word_slice_exact("first"),
                primitives::word_slice_exact("strike"),
            )
                .value(KeywordAction::FirstStrike),
            (
                primitives::word_slice_exact("double"),
                primitives::word_slice_exact("strike"),
            )
                .value(KeywordAction::DoubleStrike),
            primitives::word_slice_exact("flying").value(KeywordAction::Flying),
            primitives::word_slice_exact("deathtouch").value(KeywordAction::Deathtouch),
            primitives::word_slice_exact("haste").value(KeywordAction::Haste),
            primitives::word_slice_exact("hexproof").value(KeywordAction::Hexproof),
            primitives::word_slice_exact("indestructible").value(KeywordAction::Indestructible),
            alt((
                primitives::word_slice_exact("lifelink").value(KeywordAction::Lifelink),
                primitives::word_slice_exact("menace").value(KeywordAction::Menace),
                primitives::word_slice_exact("reach").value(KeywordAction::Reach),
                primitives::word_slice_exact("trample").value(KeywordAction::Trample),
                primitives::word_slice_exact("vigilance").value(KeywordAction::Vigilance),
                primitives::word_slice_exact("defender").value(KeywordAction::Defender),
                primitives::word_slice_exact("flash").value(KeywordAction::Flash),
                primitives::word_slice_exact("phasing").value(KeywordAction::Phasing),
                alt((
                    primitives::word_slice_exact("shroud").value(KeywordAction::Shroud),
                    primitives::word_slice_exact("wither").value(KeywordAction::Wither),
                    primitives::word_slice_exact("infect").value(KeywordAction::Infect),
                )),
            )),
        )),
    )?;
    input.is_empty().then_some(action)
}

fn parse_etb_static_ability_ids_lexed(input: &mut LexStream<'_>) -> WResult<Vec<StaticAbilityId>> {
    let first = parse_static_ability_id_lexed(input)?;
    let rest: Vec<StaticAbilityId> = repeat(
        0..,
        (
            opt(primitives::comma()),
            opt(parse_static_ability_separator_lexed),
            parse_static_ability_id_lexed,
        )
            .map(|(_, _, ability)| ability),
    )
    .parse_next(input)?;
    let mut abilities = Vec::with_capacity(1 + rest.len());
    abilities.push(first);
    abilities.extend(rest);
    Ok(abilities)
}

fn parse_static_ability_separator_lexed(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("and"), primitives::kw("or")))
        .void()
        .parse_next(input)
}

fn parse_static_ability_id_lexed(input: &mut LexStream<'_>) -> WResult<StaticAbilityId> {
    alt((
        primitives::phrase(&["first", "strike"]).value(StaticAbilityId::FirstStrike),
        primitives::phrase(&["double", "strike"]).value(StaticAbilityId::DoubleStrike),
        primitives::kw("flying").value(StaticAbilityId::Flying),
        primitives::kw("deathtouch").value(StaticAbilityId::Deathtouch),
        primitives::kw("haste").value(StaticAbilityId::Haste),
        primitives::kw("hexproof").value(StaticAbilityId::Hexproof),
        primitives::kw("indestructible").value(StaticAbilityId::Indestructible),
        alt((
            primitives::kw("lifelink").value(StaticAbilityId::Lifelink),
            primitives::kw("menace").value(StaticAbilityId::Menace),
            primitives::kw("reach").value(StaticAbilityId::Reach),
            primitives::kw("trample").value(StaticAbilityId::Trample),
            primitives::kw("vigilance").value(StaticAbilityId::Vigilance),
        )),
    ))
    .parse_next(input)
}

fn parse_number_of_counters_on_source_value_lexed<'a>(input: &mut LexStream<'a>) -> WResult<Value> {
    opt(alt((
        primitives::kw("a"),
        primitives::kw("an"),
        primitives::kw("one"),
    )))
    .parse_next(input)?;
    let counter_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((primitives::kw("counter"), primitives::kw("counters")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("counter"), primitives::kw("counters"))).parse_next(input)?;
    primitives::kw("on").parse_next(input)?;
    let source_tokens: &[OwnedLexToken] = rest.parse_next(input)?;
    let counter_type =
        filters::parse_counter_type_from_tokens(counter_tokens).ok_or_else(|| {
            primitives::backtrack_err("number of counters on source", "known counter type")
        })?;
    let source_tokens = trim_lexed_commas(source_tokens);
    let source_words = TokenWordView::new(source_tokens).word_refs();
    let surface = crate::util::source_reference_surface_for_words(&source_words)
        .or_else(|| crate::util::this_source_surface_for_words(&source_words));
    if let Some(surface) = surface {
        return Ok(Value::CountersOn(
            Box::new(crate::util::source_choose_spec_for_surface(surface)),
            Some(counter_type),
        ));
    }
    if parse_counter_source_pronoun_tokens(source_tokens) {
        return Ok(Value::CountersOnSource(counter_type));
    }
    Err(primitives::backtrack_err(
        "number of counters on source",
        "source reference",
    ))
}

fn parse_counter_source_pronoun_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        alt((
            primitives::kw("it").void(),
            primitives::kw("this").void(),
            primitives::phrase(&["this", "card"]),
            primitives::phrase(&["this", "creature"]),
            primitives::phrase(&["this", "permanent"]),
            primitives::phrase(&["this", "source"]),
            primitives::phrase(&["this", "artifact"]),
            alt((
                primitives::phrase(&["this", "land"]),
                primitives::phrase(&["this", "enchantment"]),
                primitives::phrase(&["this", "equipment"]),
                primitives::kw("thiss").void(),
                primitives::phrase(&["thiss", "card"]),
                primitives::phrase(&["thiss", "creature"]),
                primitives::phrase(&["thiss", "permanent"]),
                primitives::phrase(&["thiss", "source"]),
                alt((
                    primitives::phrase(&["thiss", "artifact"]),
                    primitives::phrase(&["thiss", "land"]),
                    primitives::phrase(&["thiss", "enchantment"]),
                    primitives::phrase(&["thiss", "equipment"]),
                )),
            )),
        )),
        "counter source pronoun",
    )
    .is_ok()
}

fn parse_snow_mana_of_spell_color_condition_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    opt(primitives::kw("if")).parse_next(input)?;
    leaf::parse_leaf_mana_group_token
        .verify(|symbols: &Vec<ManaSymbol>| {
            symbols.len() == 1 && symbols.first() == Some(&ManaSymbol::Snow)
        })
        .parse_next(input)?;
    primitives::phrase(&["of", "any", "of", "that"]).parse_next(input)?;
    alt((
        primitives::kw("spell"),
        primitives::kw("spells"),
        primitives::kw("spell's"),
    ))
    .parse_next(input)?;
    primitives::phrase(&["colors", "was", "spent", "to", "cast", "it"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_special_count_and_static_ability_values() {
        let tokens = lex_line("creatures that died this turn", 0).unwrap();
        assert_eq!(
            parse_where_x_special_number_filter_tokens(&tokens),
            Some(WhereXSpecialNumberFilterKind::CreaturesDiedThisTurn)
        );

        let tokens = lex_line("flying, first strike, and trample", 0).unwrap();
        let abilities = parse_etb_static_ability_ids_tokens(&tokens).unwrap();
        assert_eq!(abilities.len(), 3);
    }

    #[test]
    fn parses_counter_source_snow_condition_and_pt_keyword() {
        let tokens = lex_line("a quest counter on this creature", 0).unwrap();
        assert!(parse_number_of_counters_on_source_value_tokens(&tokens).is_some());

        let tokens = lex_line(
            "If {S} of any of that spell's colors was spent to cast it",
            0,
        )
        .unwrap();
        assert!(parse_snow_mana_of_spell_color_condition_tokens(&tokens));
        assert_eq!(
            parse_pt_choice_keyword_action_words(&["double", "strike"]),
            Some(KeywordAction::DoubleStrike)
        );
    }
}
