use super::super::*;

use crate::grammar::leaf;
use winnow::combinator::{alt, opt, peek, repeat};
use winnow::error::ModalResult as WResult;

#[derive(Debug, Clone)]
pub struct CounterAbilityTargetShape {
    pub target_filter: ObjectFilter,
    pub target_count: Option<ChoiceCount>,
    pub explicit_target: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CounterTargetTerm {
    ActivatedAbility,
    TriggeredAbility,
    Ability,
    Spell,
    InstantSpell,
    SorcerySpell,
    LegendarySpell,
    NoncreatureSpell,
    ColorlessSpell,
    ActivatedOrTriggeredAbility,
    TriggeredOrActivatedAbility,
}

fn connector<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    // "target instant spell, sorcery spell, activated ability, or triggered
    // ability" (Return the Favor): list commas join terms too.
    alt((
        (
            primitives::comma(),
            opt(alt((primitives::kw("and"), primitives::kw("or")))),
        )
            .void(),
        alt((primitives::kw("and"), primitives::kw("or"))).void(),
    ))
    .parse_next(input)
}

fn target_word<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("target"), primitives::kw("targets")))
        .void()
        .parse_next(input)
}

fn parse_count_before_target<'a>(input: &mut LexStream<'a>) -> WResult<ChoiceCount> {
    (
        alt((
            leaf::parse_leaf_target_count_range_prefix_lexed,
            leaf::parse_leaf_choice_count_prefix_lexed,
        )),
        peek(target_word),
    )
        .map(|(count, _)| count)
        .parse_next(input)
}

fn parse_target_selector<'a>(input: &mut LexStream<'a>) -> WResult<(Option<ChoiceCount>, bool)> {
    let count = opt(parse_count_before_target).parse_next(input)?;
    let explicit = if target_word.parse_next(input).is_ok() {
        true
    } else {
        alt((primitives::kw("all"), primitives::kw("each"))).parse_next(input)?;
        false
    };
    Ok((count, explicit))
}

fn parse_counter_term<'a>(input: &mut LexStream<'a>) -> WResult<CounterTargetTerm> {
    alt((
        (
            primitives::phrase(&["activated", "or", "triggered"]),
            ability_noun,
        )
            .value(CounterTargetTerm::ActivatedOrTriggeredAbility),
        (
            primitives::phrase(&["triggered", "or", "activated"]),
            ability_noun,
        )
            .value(CounterTargetTerm::TriggeredOrActivatedAbility),
        primitives::phrase(&["activated", "ability"]).value(CounterTargetTerm::ActivatedAbility),
        primitives::phrase(&["triggered", "ability"]).value(CounterTargetTerm::TriggeredAbility),
        primitives::phrase(&["instant", "spell"]).value(CounterTargetTerm::InstantSpell),
        primitives::phrase(&["sorcery", "spell"]).value(CounterTargetTerm::SorcerySpell),
        primitives::phrase(&["legendary", "spell"]).value(CounterTargetTerm::LegendarySpell),
        primitives::phrase(&["noncreature", "spell"]).value(CounterTargetTerm::NoncreatureSpell),
        alt((
            primitives::phrase(&["colorless", "spell"]).value(CounterTargetTerm::ColorlessSpell),
            alt((
                alt((primitives::kw("ability"), primitives::kw("abilities")))
                    .value(CounterTargetTerm::Ability),
                primitives::kw("spell").value(CounterTargetTerm::Spell),
            )),
        )),
    ))
    .parse_next(input)
}

fn ability_noun<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("ability"), primitives::kw("abilities")))
        .void()
        .parse_next(input)
}

fn control_verb<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("control"), primitives::kw("controls")))
        .void()
        .parse_next(input)
}

fn parse_controller_tail<'a>(input: &mut LexStream<'a>) -> WResult<PlayerFilter> {
    alt((
        (
            primitives::kw("you"),
            alt((primitives::kw("dont"), primitives::kw("don't"))),
            control_verb,
        )
            .value(PlayerFilter::NotYou),
        (primitives::phrase(&["you", "do", "not"]), control_verb).value(PlayerFilter::NotYou),
        (primitives::kw("you"), control_verb).value(PlayerFilter::You),
        (primitives::phrase(&["your", "opponents"]), control_verb).value(PlayerFilter::Opponent),
        (
            alt((primitives::kw("opponent"), primitives::kw("opponents"))),
            control_verb,
        )
            .value(PlayerFilter::Opponent),
        (primitives::phrase(&["an", "opponent"]), control_verb).value(PlayerFilter::Opponent),
    ))
    .parse_next(input)
}

fn article<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("a"),
        primitives::kw("an"),
        primitives::kw("the"),
    ))
    .void()
    .parse_next(input)
}

fn source_noun<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("source"), primitives::kw("sources")))
        .void()
        .parse_next(input)
}

fn parse_source_card_type<'a>(input: &mut LexStream<'a>) -> WResult<CardType> {
    primitives::word_parser_text
        .verify_map(|word| {
            parse_card_type(word).or_else(|| {
                crate::string_primitives::strip_suffix_char(word, 's').and_then(parse_card_type)
            })
        })
        .parse_next(input)
}

/// The ability source's qualities: "from an artifact source", "from a
/// colorless source", "from another legendary source that's not a
/// commander" (Abstruse Archaic, The Peregrine Dynamo). Stack abilities are
/// matched through their source object.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct AbilitySourceQualifiers {
    pub(super) card_types: Vec<CardType>,
    pub(super) colorless: bool,
    pub(super) legendary: bool,
    pub(super) another: bool,
    pub(super) noncommander: bool,
}

#[derive(Clone, Copy)]
enum SourceAdjective {
    CardType(CardType),
    Colorless,
    Legendary,
}

fn source_adjective<'a>(input: &mut LexStream<'a>) -> WResult<SourceAdjective> {
    alt((
        primitives::kw("colorless").value(SourceAdjective::Colorless),
        primitives::kw("legendary").value(SourceAdjective::Legendary),
        parse_source_card_type.map(SourceAdjective::CardType),
    ))
    .parse_next(input)
}

fn parse_source_types_tail<'a>(input: &mut LexStream<'a>) -> WResult<AbilitySourceQualifiers> {
    primitives::kw("from").parse_next(input)?;
    let another = opt(primitives::kw("another")).parse_next(input)?.is_some();
    if !another {
        opt(article).parse_next(input)?;
    }
    let adjectives: Vec<SourceAdjective> = repeat(
        1..,
        (opt(connector), source_adjective).map(|(_, adjective)| adjective),
    )
    .parse_next(input)?;
    source_noun.parse_next(input)?;
    let noncommander = opt(alt((
        primitives::phrase(&["that's", "not", "a", "commander"]),
        primitives::phrase(&["thats", "not", "a", "commander"]),
        primitives::phrase(&["that", "is", "not", "a", "commander"]),
    )))
    .parse_next(input)?
    .is_some();
    let mut qualifiers = AbilitySourceQualifiers {
        another,
        noncommander,
        ..AbilitySourceQualifiers::default()
    };
    for adjective in adjectives {
        match adjective {
            SourceAdjective::CardType(card_type) => qualifiers.card_types.push(card_type),
            SourceAdjective::Colorless => qualifiers.colorless = true,
            SourceAdjective::Legendary => qualifiers.legendary = true,
        }
    }
    Ok(qualifiers)
}

fn triggered_filter() -> ObjectFilter {
    let mut filter = ObjectFilter::ability();
    filter.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
    filter
}

fn term_filters(term: CounterTargetTerm) -> Vec<(ObjectFilter, bool)> {
    match term {
        CounterTargetTerm::ActivatedAbility => vec![(ObjectFilter::activated_ability(), true)],
        CounterTargetTerm::TriggeredAbility => vec![(triggered_filter(), true)],
        CounterTargetTerm::Ability => vec![(ObjectFilter::ability(), true)],
        CounterTargetTerm::Spell => vec![(ObjectFilter::spell(), false)],
        CounterTargetTerm::InstantSpell => {
            vec![(ObjectFilter::spell().with_type(CardType::Instant), false)]
        }
        CounterTargetTerm::SorcerySpell => {
            vec![(ObjectFilter::spell().with_type(CardType::Sorcery), false)]
        }
        CounterTargetTerm::LegendarySpell => vec![(
            ObjectFilter::spell().with_supertype(crate::Supertype::Legendary),
            false,
        )],
        CounterTargetTerm::NoncreatureSpell => {
            let mut filter = ObjectFilter::noncreature_spell().in_zone(Zone::Stack);
            filter.stack_kind = Some(crate::filter::StackObjectKind::Spell);
            vec![(filter, false)]
        }
        CounterTargetTerm::ColorlessSpell => vec![(ObjectFilter::spell().colorless(), false)],
        CounterTargetTerm::ActivatedOrTriggeredAbility => vec![
            (ObjectFilter::activated_ability(), true),
            (triggered_filter(), true),
        ],
        CounterTargetTerm::TriggeredOrActivatedAbility => vec![
            (triggered_filter(), true),
            (ObjectFilter::activated_ability(), true),
        ],
    }
}

#[cfg(test)]
#[path = "counter_ability_inline_tests.rs"]
mod tests;

#[path = "counter_ability/counter.rs"]
mod counter_programs;
use counter_programs::parse_counter_ability_target_lexed;
pub use counter_programs::parse_counter_ability_target_tokens;
