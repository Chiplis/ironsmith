use super::*;

pub(super) fn parse_counter_ability_target_lexed<'a>(
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
                crate::keyword_static::parse_cost_modifier_target_spec(&rest)
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

pub fn parse_counter_ability_target_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CounterAbilityTargetShape> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_counter_ability_target_lexed,
        "counter ability target",
    )
}
