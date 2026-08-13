use super::super::*;

use crate::grammar::leaf;
use winnow::combinator::{alt, opt, peek, repeat};
use winnow::error::ModalResult as WResult;

#[derive(Debug, Clone)]
pub(crate) struct CounterAbilityTargetShape {
    pub(crate) target_filter: ObjectFilter,
    pub(crate) target_count: Option<ChoiceCount>,
    pub(crate) explicit_target: bool,
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
    alt((primitives::kw("and"), primitives::kw("or")))
        .void()
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

fn parse_source_types_tail<'a>(input: &mut LexStream<'a>) -> WResult<Vec<CardType>> {
    primitives::kw("from").parse_next(input)?;
    opt(article).parse_next(input)?;
    let types = repeat(
        1..,
        (opt(connector), parse_source_card_type).map(|(_, card_type)| card_type),
    )
    .parse_next(input)?;
    source_noun.parse_next(input)?;
    Ok(types)
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

fn parse_counter_ability_target_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CounterAbilityTargetShape> {
    opt(primitives::kw("counter")).parse_next(input)?;
    let (target_count, explicit_target) = parse_target_selector.parse_next(input)?;
    let terms: Vec<CounterTargetTerm> = repeat(
        1..,
        (opt(connector), parse_counter_term).map(|(_, term)| term),
    )
    .parse_next(input)?;
    if terms.iter().all(|term| {
        !matches!(
            term,
            CounterTargetTerm::ActivatedAbility
                | CounterTargetTerm::TriggeredAbility
                | CounterTargetTerm::Ability
                | CounterTargetTerm::ActivatedOrTriggeredAbility
                | CounterTargetTerm::TriggeredOrActivatedAbility
        )
    }) {
        return Err(primitives::backtrack_err(
            "counter ability target",
            "at least one ability term",
        ));
    }

    let mut controller = None;
    let mut source_types = Vec::new();
    let mut targets_relation = None;
    loop {
        opt(connector).parse_next(input)?;
        let mut controller_probe = input.clone();
        if let Ok(parsed) = parse_controller_tail.parse_next(&mut controller_probe) {
            *input = controller_probe;
            controller = Some(parsed);
            continue;
        }
        let mut source_probe = input.clone();
        if let Ok(parsed) = parse_source_types_tail.parse_next(&mut source_probe) {
            *input = source_probe;
            source_types = parsed;
            continue;
        }
        // "that targets <object filter>" — the countered object's own target.
        let mut targets_probe = input.clone();
        if primitives::phrase(&["that", "targets"])
            .void()
            .parse_next(&mut targets_probe)
            .is_ok()
        {
            let only = opt(primitives::kw("only"))
                .parse_next(&mut targets_probe)?
                .is_some();
            let rest: Vec<OwnedLexToken> = repeat(
                1..,
                winnow::token::any.map(|token: &OwnedLexToken| token.clone()),
            )
            .parse_next(&mut targets_probe)?;
            if let Ok((target_player, target_object, targets_any_of)) =
                crate::families::keyword_static::parse_cost_modifier_target_spec(
                    &rest,
                )
            {
                *input = targets_probe;
                targets_relation = Some((only, target_player, target_object, targets_any_of));
                continue;
            }
        }
        break;
    }
    primitives::sentence_end().parse_next(input)?;

    let mut filters = terms.into_iter().flat_map(term_filters).collect::<Vec<_>>();
    for (filter, is_ability) in &mut filters {
        if let Some(controller) = controller.clone() {
            filter.controller = Some(controller);
        }
        if let Some((only, target_player, target_object, targets_any_of)) = targets_relation.clone()
        {
            if only {
                filter.targets_only_player = target_player;
                filter.targets_only_object = target_object;
                filter.targets_only_any_of = targets_any_of;
            } else {
                filter.targets_player = target_player;
                filter.targets_object = target_object;
                filter.targets_any_of = targets_any_of;
            }
        }
        if *is_ability {
            for card_type in &source_types {
                *filter = filter.clone().with_type(*card_type);
            }
        }
    }
    let target_filter = if filters.len() == 1 {
        filters
            .pop()
            .map(|(filter, _)| filter)
            .ok_or_else(|| primitives::backtrack_err("counter ability target", "ability filter"))?
    } else {
        let mut any = ObjectFilter::default();
        any.any_of = filters.into_iter().map(|(filter, _)| filter).collect();
        any
    };
    Ok(CounterAbilityTargetShape {
        target_filter,
        target_count,
        explicit_target,
    })
}

pub(crate) fn parse_counter_ability_target_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CounterAbilityTargetShape> {
    primitives::parse_all(
        tokens,
        parse_counter_ability_target_lexed,
        "counter ability target",
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_counted_activated_or_triggered_ability_target() {
        let tokens = lex_line(
            "up to two target activated or triggered abilities you don't control",
            0,
        )
        .unwrap();
        let shape = parse_counter_ability_target_tokens(&tokens).expect("shape");
        assert!(shape.explicit_target);
        assert!(shape.target_count.is_some());
        assert_eq!(shape.target_filter.any_of.len(), 2);
        assert!(
            shape
                .target_filter
                .any_of
                .iter()
                .all(|filter| filter.controller == Some(PlayerFilter::NotYou))
        );
    }

    #[test]
    fn parses_ability_source_type_restriction() {
        let tokens = lex_line("target activated ability from an artifact source", 0).unwrap();
        let shape = parse_counter_ability_target_tokens(&tokens).expect("shape");
        assert_eq!(shape.target_filter.card_types, vec![CardType::Artifact]);
    }

    #[test]
    fn ordinary_targets_relation_requires_any_matching_target() {
        let tokens = lex_line(
            "target spell or ability that targets a creature you control",
            0,
        )
        .unwrap();
        let shape = parse_counter_ability_target_tokens(&tokens).expect("shape");
        assert_eq!(shape.target_filter.any_of.len(), 2);
        assert!(shape.target_filter.any_of.iter().all(|filter| {
            filter.targets_object.is_some() && filter.targets_only_object.is_none()
        }));
    }

    #[test]
    fn explicit_targets_only_relation_requires_every_target_to_match() {
        let tokens = lex_line(
            "target spell or ability that targets only a creature you control",
            0,
        )
        .unwrap();
        let shape = parse_counter_ability_target_tokens(&tokens).expect("shape");
        assert_eq!(shape.target_filter.any_of.len(), 2);
        assert!(shape.target_filter.any_of.iter().all(|filter| {
            filter.targets_object.is_none() && filter.targets_only_object.is_some()
        }));
    }

    #[test]
    fn targets_relation_preserves_player_or_controlled_creature_union() {
        let tokens = lex_line(
            "target spell or ability that targets you or a creature you control",
            0,
        )
        .unwrap();
        let shape = parse_counter_ability_target_tokens(&tokens).expect("shape");
        assert_eq!(shape.target_filter.any_of.len(), 2);
        assert!(shape.target_filter.any_of.iter().all(|filter| {
            filter.targets_player == Some(PlayerFilter::You)
                && filter.targets_object.is_some()
                && filter.targets_any_of
                && filter.targets_only_player.is_none()
                && filter.targets_only_object.is_none()
        }));
    }
}
