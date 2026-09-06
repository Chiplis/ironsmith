use crate::cards::builders::PlayerPredicateAst;
use crate::cards::builders::PermissionEffectAst;
use crate::cards::builders::ObjectChoiceEffectAst;
use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::DamageActionAst;
use crate::cards::builders::LifeResourceActionAst;
use crate::cards::builders::ZoneMoveActionAst;
use super::*;
use crate::grammar::effects::choice_damage_shapes as choice_shapes;

fn is_explicit_target_clause(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    choice_shapes::first_choice_damage_word_is(&clause.word_refs(), "target")
        || parse_choice_count_before_target_prefix(clause.tokens()).is_some()
}

pub fn parse_sentence_each_opponent_loses_x_and_you_gain_x(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = choice_shapes::parse_opponent_drain_sentence_shape(clause.tokens()) else {
        return Ok(None);
    };
    let where_value = parse_value_binding_clause(shape.where_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported where-x value in opponent life-drain clause (clause: '{}')",
            clause.text()
        ))
    })?;

    Ok(Some(vec![
        EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
            effects: vec![EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::Implicit,
                SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife {
                    amount: where_value.clone(),
                }),
            )],
        }),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife {
                amount: where_value,
            }),
        ),
    ]))
}

pub fn parse_sentence_relative_opponent_damage_difference(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) =
        choice_shapes::parse_relative_opponent_damage_difference_shape(clause.tokens())
    else {
        return Ok(None);
    };
    let mut base_filter = parse_object_filter(shape.filter_tokens, false)?;
    if base_filter.controller.is_some() || base_filter.owner.is_some() {
        return Ok(None);
    }

    let mut iterated_filter = base_filter.clone();
    iterated_filter.controller = Some(PlayerFilter::IteratedPlayer);
    let mut your_filter = base_filter.clone();
    your_filter.controller = Some(PlayerFilter::You);
    let amount = Value::Add(
        Box::new(Value::Count(iterated_filter)),
        Box::new(Value::Scaled(Box::new(Value::Count(your_filter)), -1)),
    )
    .with_surface_hint(ironsmith_core::ValueSurfaceHint::Difference);

    // The authored source phrase is deliberately validated by the grammar
    // shape but lowered to the ordinary source reference. This keeps named
    // self-references and "this spell/source" reusable without embedding a
    // card name in the AST.
    let _source_surface = shape.source_tokens;
    base_filter.zone = None;
    Ok(Some(vec![EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
        effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::Player(PlayerPredicateAst::PlayerControlsMoreThanYou {
                player: PlayerAst::That,
                filter: base_filter,
            }),
            if_true: vec![EffectAst::subject_verb_damage_with_source(
                TargetAst::Source(None),
                amount,
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
            if_false: Vec::new(),
        })],
    })]))
}

pub fn parse_sentence_same_name_target_fanout(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_same_name_target_fanout_sentence)
}

pub fn parse_sentence_shared_color_target_fanout(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_shared_color_target_fanout_sentence)
}

pub fn parse_sentence_compound_damage_fanout(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_compound_damage_fanout_sentence)
}

pub fn parse_sentence_serial_target_pt_modifiers(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_serial_target_pt_modifiers_sentence)
}

pub fn parse_sentence_same_name_gets_fanout(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_same_name_gets_fanout_sentence)
}

pub fn parse_sentence_delayed_until_next_end_step(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_delayed_until_next_end_step_sentence)
}

pub fn parse_sentence_destroy_or_exile_all_split(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_destroy_or_exile_all_split_sentence)
}

pub fn parse_sentence_exile_up_to_one_each_target_type(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_exile_up_to_one_each_target_type_sentence)
}

pub fn parse_sentence_exile_multi_target(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !choice_shapes::first_choice_damage_word_is(&clause.word_refs(), "exile")
        || choice_shapes::has_unless_shape(&clause.word_refs())
    {
        return Ok(None);
    }

    let Some(and_idx) = clause.find_token_word_where("and", |idx, tail_clause| {
        idx > 0 && !tail_clause.is_empty() && is_explicit_target_clause(tail_clause)
    }) else {
        return Ok(None);
    };

    let first_clause = clause.between(1, and_idx).trimmed();
    let second_clause = clause.from(and_idx + 1).trimmed();
    if first_clause.is_empty() || second_clause.is_empty() {
        return Ok(None);
    }

    let first_words = first_clause.word_refs();
    let first_is_explicit_target = is_explicit_target_clause(first_clause);
    let second_is_explicit_target = is_explicit_target_clause(second_clause);

    let mut first_target = if !first_is_explicit_target
        && choice_shapes::is_likely_named_or_source_reference_shape(&first_words)
    {
        if let Some(surface) = crate::util::source_reference_surface_for_words(&first_words)
            .or_else(|| crate::util::this_source_surface_for_words(&first_words))
        {
            TargetAst::Object(
                ObjectFilter::source_with_surface(surface),
                None,
                first_clause.span(),
            )
        } else {
            TargetAst::Source(first_clause.span())
        }
    } else {
        parse_target_phrase(first_clause.tokens())?
    };
    let mut second_target = parse_target_phrase(second_clause.tokens())?;

    if first_is_explicit_target
        && second_is_explicit_target
        && let (Some((mut first_filter, first_count)), Some((mut second_filter, second_count))) = (
            object_target_with_count(&first_target),
            object_target_with_count(&second_target),
        )
        && first_filter.zone == Some(Zone::Graveyard)
        && second_filter.zone == Some(Zone::Graveyard)
    {
        if first_filter.controller.is_none() {
            first_filter.controller = Some(PlayerFilter::Any);
        }
        if second_filter.controller.is_none() {
            second_filter.controller = Some(PlayerFilter::Any);
        }
        let tag = helper_tag_for_tokens(clause.tokens(), "exiled");
        return Ok(Some(vec![
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter: first_filter,
                count: first_count,
                count_value: None,
                player: PlayerAst::You,
                tag: crate::tag::TagRef::of(tag.clone()),
            }),
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter: second_filter,
                count: second_count,
                count_value: None,
                player: PlayerAst::You,
                tag: crate::tag::TagRef::of(tag.clone()),
            }),
            EffectAst::subject_verb_exile(TargetAst::Tagged(crate::tag::TagRef::of(tag), None), false),
        ]));
    }

    apply_exile_subject_hand_owner_context(&mut first_target, None);
    apply_exile_subject_hand_owner_context(&mut second_target, None);
    Ok(Some(vec![EffectAst::Coordinated {
        effects: vec![
            EffectAst::subject_verb_exile(first_target, false),
            EffectAst::subject_verb_exile(second_target, false),
        ],
        leading_duration: false,
        result_conjunction: false,
    }]))
}

pub fn split_destroy_target_segments(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Vec<SubjectVerbPrimitiveClause<'_>> {
    let mut segments = Vec::new();
    for segment_clause in clause.trimmed_and_comma_segments() {
        let split_starts = choice_shapes::up_to_one_target_word_starts(&segment_clause.word_refs());

        if split_starts.len() <= 1 {
            segments.push(segment_clause);
            continue;
        }

        for (idx, start) in split_starts.iter().enumerate() {
            let end = split_starts
                .get(idx + 1)
                .copied()
                .unwrap_or(segment_clause.len());
            let segment = segment_clause.between(*start, end).trimmed();
            if !segment.is_empty() {
                segments.push(segment);
            }
        }
    }

    segments
}

pub fn parse_sentence_destroy_multi_target(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = choice_shapes::parse_destroy_multi_target_shape(clause.tokens()) else {
        return Ok(None);
    };

    let target_clause = clause.from(shape.target_start_word).trimmed();
    if target_clause.is_empty() {
        return Ok(None);
    }
    if !shape.repeated_target_words
        && !shape.has_followup_tail
        && let Ok(target) = parse_target_phrase(target_clause.tokens())
        && let Some((filter, _)) = object_target_with_count(&target)
        && (filter.type_or_subtype_union
            || filter.card_types.len() > 1
            || filter.subtypes.len() > 1
            || filter.any_of.len() > 1)
    {
        return Ok(Some(vec![EffectAst::subject_verb_destroy(target)]));
    }

    let segments = split_destroy_target_segments(target_clause);
    if segments.len() < 2 {
        return Ok(None);
    }

    let mut effects = Vec::new();
    for segment_clause in segments {
        let segment_words = segment_clause.word_refs();
        if choice_shapes::has_choice_damage_condition_boundary(&segment_words) {
            return Ok(None);
        }
        let is_explicit_target =
            choice_shapes::first_choice_damage_word_is(&segment_words, "target")
                || parse_choice_count_before_target_prefix(segment_clause.tokens()).is_some();
        if !is_explicit_target
            && !choice_shapes::is_likely_named_or_source_reference_shape(&segment_words)
        {
            return Ok(None);
        }
        if let Some(choice) =
            crate::grammar::choices::parse_possessive_object_choice_tokens(segment_clause.tokens())
            && choice.actor == crate::grammar::choices::PossessiveObjectChoiceActor::Opponent
        {
            let target = parse_target_phrase(&choice.object_tokens)?;
            effects.push(EffectAst::Sequence {
                effects: vec![
                    EffectAst::subject_verb_explicit_target_only_for_chooser(
                        target,
                        PlayerAst::Opponent,
                    ),
                    EffectAst::subject_verb_destroy(TargetAst::Tagged(
                        crate::tag::CompilerReferenceTag::It.bind(),
                        segment_clause.span(),
                    )),
                ],
            });
            continue;
        }
        let target = match parse_target_phrase(segment_clause.tokens()) {
            Ok(target) => target,
            Err(_)
                if !is_explicit_target
                    && choice_shapes::is_likely_named_or_source_reference_shape(&segment_words) =>
            {
                TargetAst::Source(segment_clause.span())
            }
            Err(err) => return Err(err),
        };
        effects.push(EffectAst::subject_verb_destroy(target));
    }

    if effects.len() < 2 {
        return Ok(None);
    }
    Ok(Some(vec![EffectAst::Coordinated {
        effects,
        leading_duration: false,
        result_conjunction: false,
    }]))
}

pub fn parse_sentence_reveal_selected_cards_in_your_hand(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sentence_reveal_selected_cards_in_hand_for_player(
        clause,
        PlayerAst::You,
        PlayerFilter::You,
    )
}

fn parse_sentence_reveal_selected_cards_in_hand_for_player(
    clause: SubjectVerbPrimitiveClause<'_>,
    player: PlayerAst,
    owner: PlayerFilter,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = choice_shapes::parse_reveal_selected_hand_shape(clause.tokens()) else {
        return Ok(None);
    };
    let clause_text = clause.text();
    let clause_words = clause.word_refs();
    if clause_words.first() != Some(&"reveal") {
        return Ok(None);
    }
    if clause_words.iter().any(|word| {
        matches!(
            *word,
            "then" | "if" | "unless" | "where" | "when" | "whenever"
        )
    }) {
        return Ok(None);
    }

    let mut descriptor_clause = SubjectVerbPrimitiveClause::new(shape.descriptor_tokens).trimmed();
    if descriptor_clause.is_empty() {
        return Ok(None);
    }

    let mut count = ChoiceCount::exactly(1);
    if let Some((parsed_count, used)) =
        crate::util::parse_choice_count_token_prefix_consumed(descriptor_clause.tokens())
    {
        count = if parsed_count.dynamic_x {
            ChoiceCount::any_number()
        } else {
            parsed_count
        };
        descriptor_clause = descriptor_clause.from(used).trimmed();
        if choice_shapes::first_choice_damage_word_is(&descriptor_clause.word_refs(), "of") {
            descriptor_clause = descriptor_clause.from(1).trimmed();
        }
    } else if descriptor_clause
        .first_word()
        .is_some_and(choice_shapes::is_reveal_article_word)
    {
        descriptor_clause = descriptor_clause.from(1).trimmed();
    } else if choice_shapes::has_all_or_each_at(&descriptor_clause.word_refs(), 0) {
        return Ok(None);
    }

    if descriptor_clause.is_empty() {
        return Ok(None);
    }

    let mut filter = match parse_object_filter(descriptor_clause.tokens(), false) {
        Ok(filter) => filter,
        Err(_) => {
            let descriptor_words = descriptor_clause.word_refs();
            let mut filter = ObjectFilter::default();
            let mut idx = 0usize;
            if let Some(color) = descriptor_words.get(idx).and_then(|word| parse_color(word)) {
                filter.colors = Some(color);
                idx += 1;
            }
            if !choice_shapes::is_card_noun_at(&descriptor_words, idx) {
                return Err(CardTextError::ParseError(format!(
                    "unsupported reveal-hand clause (clause: '{}')",
                    clause_text
                )));
            }
            filter
        }
    };
    filter.zone = Some(Zone::Hand);
    filter.owner = Some(owner);

    let tag = helper_tag_for_tokens(clause.tokens(), "revealed");
    Ok(Some(vec![
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter,
            count,
            count_value: None,
            player,
            tag: crate::tag::TagRef::of(tag.clone()),
        }),
        EffectAst::subject_verb_reveal_tagged(crate::tag::TagRef::of(tag)),
    ]))
}

pub fn parse_sentence_each_player_may_reveal_selected_cards_in_their_hand(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) =
        choice_shapes::parse_each_player_may_reveal_selected_hand_shape(clause.tokens())
    else {
        return Ok(None);
    };
    let Some(effects) = parse_sentence_reveal_selected_cards_in_hand_for_player(
        SubjectVerbPrimitiveClause::new(shape.action_tokens),
        PlayerAst::That,
        PlayerFilter::IteratedPlayer,
    )?
    else {
        return Ok(None);
    };

    Ok(Some(vec![EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
        effects: vec![EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
            player: PlayerAst::That,
            effects,
        })],
    })]))
}

pub fn parse_sentence_target_player_reveals_random_card_from_hand(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = choice_shapes::parse_random_hand_reveal_shape(clause.tokens()) else {
        return Ok(None);
    };
    let subject_clause = SubjectVerbPrimitiveClause::new(shape.subject_tokens);
    if subject_clause.is_empty() {
        return Ok(None);
    }

    let subject_tokens = subject_clause.trim();
    let SubjectAst::Player(player) = parse_subject(&subject_tokens) else {
        return Ok(None);
    };
    if !matches!(
        player,
        PlayerAst::You
            | PlayerAst::Target
            | PlayerAst::TargetOpponent
            | PlayerAst::Opponent
            | PlayerAst::That
    ) {
        return Ok(None);
    }

    let descriptor_clause = SubjectVerbPrimitiveClause::new(shape.descriptor_tokens);
    if descriptor_clause.is_empty()
        || !choice_shapes::is_random_card_descriptor_shape(&descriptor_clause.word_refs())
    {
        return Ok(None);
    }

    let hand_clause = SubjectVerbPrimitiveClause::new(shape.hand_tokens);
    if !is_hand_reference_clause(hand_clause) {
        return Ok(None);
    }

    let filter = ObjectFilter {
        zone: Some(Zone::Hand),
        owner: Some(match player {
            PlayerAst::You => PlayerFilter::You,
            PlayerAst::Target => PlayerFilter::target_player(),
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            PlayerAst::Opponent => PlayerFilter::Opponent,
            PlayerAst::That => PlayerFilter::IteratedPlayer,
            _ => return Ok(None),
        }),
        ..ObjectFilter::default()
    };
    let tag = helper_tag_for_tokens(clause.tokens(), "revealed");

    Ok(Some(vec![
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::exactly(1).at_random(),
            count_value: None,
            player,
            tag: crate::tag::TagRef::of(tag.clone()),
        }),
        EffectAst::subject_verb_reveal_tagged(crate::tag::TagRef::of(tag)),
    ]))
}

fn is_hand_reference_clause(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    choice_shapes::is_hand_reference_shape(&clause.word_refs())
}

pub fn object_target_with_count(target: &TargetAst) -> Option<(ObjectFilter, ChoiceCount)> {
    match target {
        TargetAst::Object(filter, _, _) => Some((filter.clone(), ChoiceCount::exactly(1))),
        TargetAst::WithCount(inner, count) => match inner.as_ref() {
            TargetAst::Object(filter, _, _) => Some((filter.clone(), *count)),
            _ => None,
        },
        _ => None,
    }
}

pub fn parse_sentence_damage_unless_controller_has_source_deal_damage(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = choice_shapes::parse_damage_unless_shape(clause.tokens()) else {
        return Ok(None);
    };
    let before_clause = SubjectVerbPrimitiveClause::new(shape.damage_tokens).trimmed();
    if before_clause.is_empty() {
        return Ok(None);
    }
    let effects = parse_effect_chain(before_clause.tokens())?;
    if effects.len() != 1 {
        return Ok(None);
    }
    let Some(main_effect) = effects.first() else {
        return Ok(None);
    };
    let main_target = match main_effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy { target, .. }) => target,
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };
    if !matches!(
        main_target,
        TargetAst::Object(_, _, _) | TargetAst::WithCount(_, _)
    ) {
        return Ok(None);
    }

    let after_unless_clause = SubjectVerbPrimitiveClause::new(shape.condition_tokens).trimmed();
    let has_controller_clause =
        choice_shapes::is_that_controller_has_shape(&after_unless_clause.word_refs());
    if !has_controller_clause {
        return Ok(None);
    }
    let Some((_controller_clause, alt_clause)) =
        after_unless_clause.split_once_on_word_any(&["has", "have"])
    else {
        return Ok(None);
    };
    if alt_clause.is_empty() {
        return Ok(None);
    }

    let Some((_before_deal, deal_tail_clause)) =
        alt_clause.split_once_on_word_any(&["deal", "deals"])
    else {
        return Ok(None);
    };
    let deal_tail = deal_tail_clause.tokens();
    let deal_words = deal_tail_clause.word_refs();
    let alt_amount = if crate::word_primitives::parse_sequence_prefix(
        &deal_words,
        &["damage", "to", "them", "equal", "to"],
    ) {
        let amount_tokens = deal_tail.get(5..).unwrap_or_default();
        let Some((amount, used)) = parse_value(amount_tokens) else {
            return Ok(None);
        };
        if used != amount_tokens.len() {
            return Ok(None);
        }
        amount.with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo)
    } else {
        let Some((amount, used)) = parse_value(deal_tail) else {
            return Ok(None);
        };
        if !deal_tail
            .get(used)
            .and_then(|token| token.as_word())
            .is_some_and(choice_shapes::is_damage_word)
        {
            return Ok(None);
        }

        let mut alt_target_clause = deal_tail_clause.from(used + 1).trimmed();
        if choice_shapes::has_leading_to_shape(&alt_target_clause.word_refs()) {
            alt_target_clause = alt_target_clause.from(1).trimmed();
        }
        if choice_shapes::parse_alternate_damage_target_shape(&alt_target_clause.word_refs())
            .is_none()
        {
            return Ok(None);
        }
        amount
    };

    let alternative = EffectAst::subject_verb_damage(
        alt_amount,
        TargetAst::Player(
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target),
            None,
        ),
    );
    let unless = EffectAst::Conditionals(ConditionalEffectAst::UnlessAction {
        effects,
        alternative: vec![alternative],
        player: PlayerAst::ItsController,
    });
    Ok(Some(vec![unless]))
}

pub fn parse_sentence_damage_to_that_player_unless_enchanted_attacked(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = choice_shapes::parse_enchanted_attacked_damage_shape(clause.tokens()) else {
        return Ok(None);
    };
    let before_clause = SubjectVerbPrimitiveClause::new(shape.damage_tokens).trimmed();
    if before_clause.is_empty() {
        return Ok(None);
    }

    let Some((subject_clause, damage_clause)) =
        before_clause.split_once_on_word_any(&["deal", "deals"])
    else {
        return Ok(None);
    };

    if !choice_shapes::is_choice_damage_source_subject_shape(&subject_clause.word_refs()) {
        return Ok(None);
    }
    let damage_tokens = damage_clause.tokens();
    let Some((amount, used)) = parse_value(damage_tokens) else {
        return Ok(None);
    };
    if !damage_tokens
        .get(used)
        .and_then(|token| token.as_word())
        .is_some_and(choice_shapes::is_damage_word)
    {
        return Ok(None);
    }

    let mut target_clause = damage_clause.from(used + 1).trimmed();
    if choice_shapes::first_choice_damage_word_is(&target_clause.word_refs(), "to") {
        target_clause = target_clause.from(1).trimmed();
    }
    if !choice_shapes::is_that_player_target_shape(&target_clause.word_refs()) {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::Conditionals(ConditionalEffectAst::TrailingUnless {
        predicate: PredicateAst::EnchantedPermanentAttackedThisTurn,
        effects: vec![EffectAst::subject_verb_damage(
            amount,
            TargetAst::Player(PlayerFilter::IteratedPlayer, None),
        )],
    })]))
}

#[cfg(test)]
#[path = "choice_damage_family_inline_opponent_choice_target_tests.rs"]
mod opponent_choice_target_tests;

#[path = "choice_damage_family/resource.rs"]
mod resource_programs;
pub use resource_programs::parse_sentence_unless_pays;
