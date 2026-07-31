use super::*;
use crate::runtime_backend::grammar::trigger_subjects as trigger_subject_grammar;

fn trigger_controller_player_filter(
    reference: crate::runtime_backend::grammar::trigger_subjects::TriggerControllerReference,
) -> PlayerFilter {
    use crate::runtime_backend::grammar::trigger_subjects::TriggerControllerReference;

    match reference {
        TriggerControllerReference::You => PlayerFilter::You,
        TriggerControllerReference::NotYou => PlayerFilter::NotYou,
        TriggerControllerReference::ChosenPlayer => PlayerFilter::ChosenPlayer,
        TriggerControllerReference::EnchantedPlayer => {
            PlayerFilter::TaggedPlayer(crate::tag::TagKey::from("enchanted"))
        }
        TriggerControllerReference::EffectController => PlayerFilter::EffectController,
        TriggerControllerReference::AnyPlayer => PlayerFilter::Any,
        TriggerControllerReference::Opponent => PlayerFilter::Opponent,
    }
}

fn trigger_source_words(words: &[&str]) -> bool {
    crate::runtime_backend::grammar::trigger_subjects::parse_trigger_source_subject_words(words)
        .is_some()
}

pub(crate) fn parse_discard_trigger_card_filter(
    after_discard_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let remainder = trim_commas(after_discard_tokens);
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing discard trigger card qualifier (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let Some(envelope) =
        crate::runtime_backend::grammar::trigger_subjects::parse_discard_trigger_envelope(
            &remainder,
        )
    else {
        return Err(CardTextError::ParseError(format!(
            "missing discard trigger card keyword (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    let mut qualifier_tokens = strip_leading_articles(envelope.qualifier);
    let qualifier_words = crate::runtime_backend::token_word_refs(&qualifier_tokens);
    if trigger_subject_grammar::trigger_words_are_one_or_more(&qualifier_words) {
        qualifier_tokens.clear();
    }
    if qualifier_tokens.len() >= 2
        && qualifier_tokens
            .first()
            .and_then(OwnedLexToken::as_word)
            .and_then(parse_cardinal_u32)
            .is_some()
        && qualifier_tokens
            .get(1)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(trigger_subject_grammar::trigger_word_is_connector)
    {
        qualifier_tokens = qualifier_tokens[2..].to_vec();
    } else if qualifier_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .and_then(parse_cardinal_u32)
        .is_some()
    {
        qualifier_tokens = qualifier_tokens[1..].to_vec();
    }

    let trailing_tokens = envelope.trailing.to_vec();
    if !trailing_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing discard trigger clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if qualifier_tokens.is_empty() {
        return Ok(None);
    }

    let qualifier_words = crate::runtime_backend::token_word_refs(&qualifier_tokens);
    if let Ok(filter) = parse_object_filter(&qualifier_tokens, false) {
        return Ok(Some(filter));
    }

    let mut fallback = ObjectFilter::default();
    let mut parsed_any = false;
    for word in qualifier_words {
        if trigger_subject_grammar::trigger_word_is_connector(word) {
            continue;
        }
        if let Some(non_type) = parse_non_type(word) {
            if !fallback
                .excluded_card_types
                .iter()
                .any(|existing| existing == &non_type)
            {
                fallback.excluded_card_types.push(non_type);
            }
            parsed_any = true;
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            if !fallback
                .card_types
                .iter()
                .any(|existing| existing == &card_type)
            {
                fallback.card_types.push(card_type);
            }
            parsed_any = true;
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported discard trigger card qualifier (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if parsed_any {
        Ok(Some(fallback))
    } else {
        Err(CardTextError::ParseError(format!(
            "unsupported discard trigger card qualifier (clause: '{}')",
            clause_words.join(" ")
        )))
    }
}

fn subtype_list_controller_suffix(words: &[&str]) -> (Option<PlayerFilter>, usize) {
    if let Some(suffix) =
        crate::runtime_backend::grammar::trigger_subjects::parse_trigger_control_suffix(words)
    {
        (
            Some(trigger_controller_player_filter(suffix.controller)),
            suffix.subject_end,
        )
    } else {
        (None, words.len())
    }
}

pub(crate) fn parse_possessive_clause_player_filter(words: &[&str]) -> PlayerFilter {
    use crate::runtime_backend::grammar::trigger_subjects::{
        AttachedControllerSubject, PossessivePlayerReference,
    };

    match crate::runtime_backend::grammar::trigger_subjects::parse_possessive_player_reference(
        words,
    ) {
        PossessivePlayerReference::EnchantedPlayer => {
            PlayerFilter::TaggedPlayer(TagKey::from("enchanted"))
        }
        PossessivePlayerReference::AttachedController(subject) => {
            let tag = match subject {
                AttachedControllerSubject::Enchanted => "enchanted",
                AttachedControllerSubject::Equipped => "equipped",
            };
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(TagKey::from(tag)))
        }
        PossessivePlayerReference::ChosenPlayer => PlayerFilter::ChosenPlayer,
        PossessivePlayerReference::You => PlayerFilter::You,
        PossessivePlayerReference::Opponent => PlayerFilter::Opponent,
        PossessivePlayerReference::Any => PlayerFilter::Any,
    }
}

pub(crate) fn parse_subject_clause_player_filter(words: &[&str]) -> PlayerFilter {
    let facts = trigger_subject_grammar::parse_trigger_subject_surface_facts(words);
    if facts.on_your_team || facts.contains_you {
        PlayerFilter::You
    } else if facts.contains_enchanted_player {
        PlayerFilter::TaggedPlayer(TagKey::from("enchanted"))
    } else if facts.contains_chosen_player {
        PlayerFilter::ChosenPlayer
    } else if facts.contains_opponent {
        PlayerFilter::Opponent
    } else {
        PlayerFilter::Any
    }
}

pub(crate) fn parse_trigger_subject_player_filter(subject: &[&str]) -> Option<PlayerFilter> {
    trigger_subject_grammar::parse_trigger_subject_surface_facts(subject)
        .player
        .map(trigger_controller_player_filter)
}

pub(crate) fn split_target_clause_before_comma(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    crate::runtime_backend::grammar::trigger_subjects::parse_clause_before_first_comma(tokens)
}

pub(crate) fn parse_shuffle_trigger_subject(
    subject: &[&str],
) -> Option<(PlayerFilter, bool, bool)> {
    let facts = trigger_subject_grammar::parse_shuffle_trigger_subject_facts(subject)?;
    Some((
        trigger_controller_player_filter(facts.player),
        facts.caused_by_spell_or_ability,
        facts.use_effect_controller,
    ))
}

pub(crate) fn parse_spell_or_ability_controller_tail(words: &[&str]) -> Option<PlayerFilter> {
    let controller =
        crate::runtime_backend::grammar::trigger_subjects::parse_spell_or_ability_controller_tail(
            words,
        )?;
    Some(trigger_controller_player_filter(controller))
}

pub(crate) fn parse_spell_controller_tail(words: &[&str]) -> Option<PlayerFilter> {
    let controller =
        crate::runtime_backend::grammar::trigger_subjects::parse_spell_controller_tail(words)?;
    Some(trigger_controller_player_filter(controller))
}

pub(crate) fn attacking_filter_for_player(player: PlayerFilter) -> ObjectFilter {
    let mut filter = ObjectFilter::creature();
    if !matches!(player, PlayerFilter::Any) {
        filter.controller = Some(player);
    }
    filter
}

pub(crate) fn strip_leading_one_or_more_lexed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    if let Some(used) = leading_one_or_more_prefix_len(tokens) {
        &tokens[used..]
    } else {
        tokens
    }
}

pub(crate) fn parse_subtype_list_enters_trigger_filter_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<ObjectFilter> {
    let words = ActivationRestrictionCompatWords::new(tokens);
    let words = words.to_word_refs();
    if words.is_empty() {
        return None;
    }

    let (controller, subject_end) = subtype_list_controller_suffix(&words);

    let mut subtypes = Vec::new();
    for word in &words[..subject_end] {
        if trigger_subject_grammar::trigger_word_is_connector(word) {
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(word) {
            if !subtypes.iter().any(|existing| existing == &subtype) {
                subtypes.push(subtype);
            }
        }
    }
    if subtypes.is_empty() {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.subtypes = subtypes;
    filter.controller = controller;
    filter.other = other;
    if tokens.iter().any(|token| token.is_word("and/or")) {
        filter.set_union_connective(crate::filter::ObjectFilterUnionConnective::AndOr);
    }
    Some(filter)
}

#[test]
fn subtype_list_enter_trigger_preserves_and_or_surface() {
    let tokens =
        crate::runtime_backend::lexer::lex_line("Rabbits, Bats, Birds, and/or Mice you control", 0)
            .expect("lex subtype-list trigger subject");
    let filter = parse_subtype_list_enters_trigger_filter_lexed(&tokens, true)
        .expect("parse subtype-list trigger subject");

    assert_eq!(filter.subtypes.len(), 4);
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert!(filter.other);
    assert_eq!(
        filter.union_connective(),
        crate::filter::ObjectFilterUnionConnective::AndOr
    );
}

fn parse_source_or_another_trigger_subject_filter_lexed(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let word_view = ActivationRestrictionCompatWords::new(subject_tokens);
    let subject_words = word_view.to_word_refs();
    let Some(shape) =
        crate::runtime_backend::grammar::trigger_subjects::parse_source_or_another_shape(
            &subject_words,
        )
    else {
        return Ok(None);
    };
    let source_words = &subject_words[..shape.source_word_end];
    if !is_source_reference_words(source_words) {
        return Ok(None);
    }
    let Some(other_token_idx) =
        crate::runtime_backend::grammar::trigger_subjects::parse_trigger_word_span(
            subject_tokens,
            shape.other_word,
        )
        .map(|span| span.first)
    else {
        return Ok(None);
    };
    let Some(other_filter) =
        parse_trigger_subject_filter_lexed(&subject_tokens[other_token_idx..])?
    else {
        return Ok(None);
    };

    let source_filter = this_source_surface_for_words(source_words)
        .map(ObjectFilter::source_with_surface)
        .unwrap_or_else(ObjectFilter::source);
    let mut filter = ObjectFilter::default();
    filter.any_of = vec![source_filter, other_filter];
    Ok(Some(filter))
}

pub(crate) fn parse_trigger_subject_filter_lexed(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let mut subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
    let mut other = false;
    if subject_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(trigger_subject_grammar::trigger_word_is_other_modifier)
    {
        other = true;
        subject_tokens = &subject_tokens[1..];
    }
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    if subject_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| word == "target")
    {
        subject_tokens = &subject_tokens[1..];
    }
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_words = ActivationRestrictionCompatWords::new(subject_tokens);
    let subject_words = subject_words.to_word_refs();
    let intrinsic_attachment_state = subject_words.iter().enumerate().find_map(|(idx, word)| {
        if !matches!(*word, "enchanted" | "equipped") {
            return None;
        }
        idx.checked_sub(1)
            .and_then(|prev| subject_words.get(prev))
            .is_some_and(|copula| matches!(*copula, "is" | "are" | "that's" | "thats"))
            .then_some(*word)
    });
    if let Some(filter) = parse_source_or_another_trigger_subject_filter_lexed(subject_tokens)? {
        return Ok(Some(filter));
    }
    if is_source_reference_words(&subject_words) {
        return Ok(None);
    }
    if let Some(suffix) =
        crate::runtime_backend::grammar::trigger_subjects::parse_trigger_control_suffix(
            &subject_words,
        )
        && trigger_source_words(&subject_words[..suffix.subject_end])
    {
        let mut filter = ObjectFilter::default();
        filter.controller = Some(trigger_controller_player_filter(suffix.controller));
        return Ok(Some(filter));
    }
    let subject_facts =
        trigger_subject_grammar::parse_trigger_subject_surface_facts(&subject_words);
    if subject_facts.any_source {
        return Ok(Some(ObjectFilter::default()));
    }
    if subject_facts.relative_pronoun {
        return Err(CardTextError::ParseError(format!(
            "unsupported trigger subject filter (clause: '{}')",
            subject_words.join(" ")
        )));
    }

    if subject_facts.power_greater_than_base_power {
        let mut filter = ObjectFilter::creature().in_zone(Zone::Battlefield);
        filter.power_greater_than_base_power = true;
        if other {
            filter.other = true;
        }
        if let Some(controller_phrase) =
            crate::runtime_backend::grammar::trigger_subjects::parse_trigger_control_phrase(
                &subject_words,
            )
        {
            filter.controller = Some(trigger_controller_player_filter(
                controller_phrase.controller,
            ));
        }
        return Ok(Some(filter));
    }

    let normalized_subject_tokens =
        trigger_subject_grammar::normalize_each_with_tokens(subject_tokens);

    // A controller phrase in one arm of a coordinated subject is local to
    // that arm.  The legacy controller override below intentionally lifts an
    // embedded controller out of a single filter, but doing so before parsing
    // `an attacking creature you control or a blocking creature an opponent
    // controls` erases the distinction between the two arms.
    if intrinsic_attachment_state.is_none()
        && !crate::runtime_backend::families::object_filters::has_shared_terminal_object_noun(
            &normalized_subject_tokens,
        )
        && let Some(mut filter) =
            crate::runtime_backend::front_end::grammar::filters::
                parse_branch_scoped_object_filter_union_lexed(
                    &normalized_subject_tokens,
                    other,
                )
    {
        if filter.zone.is_none() {
            filter.zone = Some(Zone::Battlefield);
        }
        return Ok(Some(filter));
    }

    let mut normalized_subject_tokens = normalized_subject_tokens;

    let mut controller_override = None;
    let word_view = ActivationRestrictionCompatWords::new(&normalized_subject_tokens);
    let normalized_words = word_view.to_word_refs();
    let controller_phrase = if let Some(controller_phrase) =
        crate::runtime_backend::grammar::trigger_subjects::parse_trigger_control_phrase(
            &normalized_words,
        )
        .filter(|phrase| phrase.start.saturating_add(phrase.words) < normalized_words.len())
    {
        controller_override = Some(trigger_controller_player_filter(
            controller_phrase.controller,
        ));
        Some((controller_phrase.start, controller_phrase.words))
    } else {
        None
    };

    if let Some((word_idx, len)) = controller_phrase
        && let Some(start) =
            crate::runtime_backend::grammar::trigger_subjects::parse_trigger_word_span(
                &normalized_subject_tokens,
                word_idx,
            )
            .map(|span| span.first)
        && let Some(end) =
            crate::runtime_backend::grammar::trigger_subjects::parse_trigger_word_span(
                &normalized_subject_tokens,
                word_idx + len,
            )
            .map(|span| span.first)
    {
        normalized_subject_tokens.drain(start..end);
    }

    parse_object_filter_lexed(&normalized_subject_tokens, other)
        .map(|mut filter| {
            if filter.zone.is_none()
                && filter.tagged_constraints.is_empty()
                && filter.specific.is_none()
                && !filter.source
            {
                filter.zone = Some(Zone::Battlefield);
            }
            if let Some(controller) = controller_override {
                filter.controller = Some(controller);
                filter.zone.get_or_insert(Zone::Battlefield);
            }
            if let Some(tag) = intrinsic_attachment_state
                && !filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == tag
                        && constraint.relation
                            == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                })
            {
                filter
                    .tagged_constraints
                    .push(crate::filter::TaggedObjectConstraint {
                        tag: crate::tag::TagKey::from(tag),
                        relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                    });
            }
            if intrinsic_attachment_state.is_some() {
                filter.set_relative_attachment_state_surface(true);
            }
            Some(filter)
        })
        .map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported trigger subject filter (clause: '{}')",
                subject_words.join(" ")
            ))
        })
}

pub(crate) fn trigger_subject_player_selector_lexed(
    subject_tokens: &[OwnedLexToken],
) -> Option<PlayerFilter> {
    let subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
    let subject_words = ActivationRestrictionCompatWords::new(subject_tokens);
    let subject_words = subject_words.to_word_refs();
    parse_trigger_subject_player_filter(&subject_words)
}

pub(crate) fn parse_attack_trigger_subject_filter_lexed(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if let Some(player) = trigger_subject_player_selector_lexed(subject_tokens) {
        return Ok(Some(attacking_filter_for_player(player)));
    }
    // "this [creature] or equipped creature" — an Equipment triggering for
    // itself (while reconfigured) or its bearer.
    {
        let word_refs = crate::runtime_backend::token_word_refs(subject_tokens);
        if matches!(
            word_refs.as_slice(),
            ["this", "or", "equipped", "creature"]
                | ["this", "creature", "or", "equipped", "creature"]
                | ["this", "permanent", "or", "equipped", "creature"]
        ) {
            if let Some(or_token_idx) = subject_tokens.iter().position(|t| t.is_word("or"))
                && let Some(equipped) =
                    parse_trigger_subject_filter_lexed(&subject_tokens[or_token_idx + 1..])?
            {
                let mut union = ObjectFilter::default();
                union.any_of = vec![ObjectFilter::source(), equipped];
                return Ok(Some(union));
            }
        }
    }
    let Some(mut filter) = parse_trigger_subject_filter_lexed(subject_tokens)? else {
        return Ok(None);
    };

    if filter.card_types.is_empty() {
        // Only creatures attack, so a bare-subtype subject ("a Samurai or
        // Warrior you control") or a commander subject ("a commander you
        // control") needs no Creature type for matching — and injecting one
        // made the description render an unauthored "creature" noun.
        if filter.subtypes.is_empty() && filter.any_of.is_empty() && !filter.is_commander {
            filter.card_types.push(crate::types::CardType::Creature);
        }
    } else if filter.card_types.len() > 1 && filter.all_card_types.is_empty() {
        filter.all_card_types = std::mem::take(&mut filter.card_types);
    }

    Ok(Some(filter))
}

#[test]
fn suspected_attack_subject_preserves_the_designation_filter() {
    let tokens =
        crate::runtime_backend::lexer::lex_line("one or more suspected creatures you control", 0)
            .expect("lex suspected attack subject");
    let filter = parse_attack_trigger_subject_filter_lexed(&tokens)
        .expect("parse suspected attack subject")
        .expect("object-filter subject");

    assert!(filter.suspected);
    assert_eq!(filter.card_types, [crate::types::CardType::Creature]);
    assert_eq!(filter.controller, Some(PlayerFilter::You));
}

pub(crate) fn parse_draw_numbers_each_turn(words: &[&str]) -> Vec<u32> {
    trigger_subject_grammar::parse_draw_turn_surface_facts(words).draw_numbers_this_turn
}

pub(crate) fn has_draw_except_first_in_draw_step_pattern(words: &[&str]) -> bool {
    trigger_subject_grammar::parse_draw_turn_surface_facts(words).except_first_in_draw_step
}

pub(crate) fn parse_spell_activity_trigger(
    tokens: &[OwnedLexToken],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let activity_facts = trigger_subject_grammar::parse_spell_activity_surface_facts(&clause_words);
    if !activity_facts.has_spell_noun {
        return Ok(None);
    }

    let verb_facts =
        crate::runtime_backend::grammar::trigger_subjects::parse_spell_activity_verb_facts(tokens);
    let cast_idx = verb_facts.cast;
    let copy_idx = verb_facts.copy;
    if cast_idx.is_none() && copy_idx.is_none() {
        return Ok(None);
    }

    let mut actor = parse_subject_clause_player_filter(&clause_words);
    let mut during_turn = activity_facts
        .during_turn
        .map(trigger_controller_player_filter);
    if activity_facts.during_their_turn {
        if matches!(actor, PlayerFilter::Any) {
            actor = PlayerFilter::Active;
            during_turn = None;
        } else if during_turn.is_none() {
            during_turn = Some(actor.clone());
        }
    }
    let exact_spells_this_turn = activity_facts.exact_spells_this_turn;
    let min_spells_this_turn = activity_facts.min_spells_this_turn;
    let from_not_hand = activity_facts.from_not_hand;
    let timing = activity_facts
        .during_combat
        .then_some(ironsmith_core::TriggerTimingRestriction::DuringCombat);

    if activity_facts.count_all_spells_this_turn
        && cast_idx.is_some()
        && let Some(spell_number) = exact_spells_this_turn
    {
        return Ok(Some(TriggerSpec::NthSpellOfTurnCast { spell_number }));
    }

    let parse_filter =
        |filter_tokens: &[OwnedLexToken]| -> Result<Option<ObjectFilter>, CardTextError> {
            let envelope =
                crate::runtime_backend::grammar::trigger_subjects::parse_spell_filter_envelope(
                    filter_tokens,
                );
            let filter_tokens = &filter_tokens[..envelope.end];
            let filter_words: Vec<&str> = filter_tokens
                .iter()
                .filter_map(OwnedLexToken::as_word)
                .collect();
            let filter_facts =
                trigger_subject_grammar::parse_spell_filter_surface_facts(&filter_words);
            if filter_tokens.is_empty() || filter_facts.is_unqualified_spell {
                Ok(None)
            } else {
                let parse_spell_origin_zone_filter = || -> Option<ObjectFilter> {
                    use trigger_subject_grammar::{SpellOriginSurface, SpellOwnerSurface};

                    let zone = match filter_facts.origin? {
                        SpellOriginSurface::Graveyard => Zone::Graveyard,
                        SpellOriginSurface::Exile => Zone::Exile,
                        SpellOriginSurface::Hand => Zone::Hand,
                    };
                    if !filter_facts.has_spell_noun {
                        return None;
                    }
                    let mut filter = ObjectFilter::spell().in_zone(zone);
                    match filter_facts.owner {
                        Some(SpellOwnerSurface::SubjectActor) => {
                            filter.owner = Some(actor.clone());
                        }
                        Some(SpellOwnerSurface::Opponent) => {
                            filter.owner = Some(PlayerFilter::Opponent);
                        }
                        None => {}
                    }
                    Some(filter)
                };
                if filter_facts.chosen_color_qualifier {
                    return Ok(Some(ObjectFilter::spell().of_chosen_color()));
                }
                match parse_object_filter(filter_tokens, false) {
                    Ok(mut filter) => {
                        if let Some(origin_filter) = parse_spell_origin_zone_filter() {
                            filter.zone = origin_filter.zone;
                            if filter.owner.is_none() {
                                filter.owner = origin_filter.owner;
                            }
                        }
                        Ok(Some(filter))
                    }
                    Err(err) => {
                        if let Some(color_words) = filter_facts.qualifier_words.as_deref() {
                            if !color_words.is_empty()
                                && color_words.iter().all(|word| parse_color(word).is_some())
                            {
                                let mut colors = ColorSet::new();
                                for word in color_words {
                                    colors = colors
                                        .union(parse_color(word).expect("validated color word"));
                                }
                                let mut filter = ObjectFilter::spell();
                                filter.colors = Some(colors);
                                return Ok(Some(filter));
                            }
                            if filter_facts.chosen_color_qualifier {
                                return Ok(Some(ObjectFilter::spell().of_chosen_color()));
                            }
                        }
                        if let Some(origin_filter) = parse_spell_origin_zone_filter() {
                            Ok(Some(origin_filter))
                        } else {
                            Err(err)
                        }
                    }
                }
            }
        };

    if let (Some(cast), Some(copy)) = (cast_idx, copy_idx) {
        let (first, second, first_is_cast) = if cast < copy {
            (cast, copy, true)
        } else {
            (copy, cast, false)
        };
        let between_words = crate::runtime_backend::token_word_refs(&tokens[first + 1..second]);
        if trigger_subject_grammar::spell_activity_words_are_or_separator(&between_words) {
            let filter = parse_filter(tokens.get(second + 1..).unwrap_or_default())?;
            let cast_trigger = TriggerSpec::SpellCast {
                filter: filter.clone(),
                mana_source_filter: None,
                caster: actor.clone(),
                timing,
                during_turn: during_turn.clone(),
                min_spells_this_turn,
                exact_spells_this_turn,
                from_not_hand,
            };
            let copied_trigger = TriggerSpec::SpellCopied {
                filter,
                copier: actor,
            };
            return Ok(Some(if first_is_cast {
                TriggerSpec::Either(Box::new(cast_trigger), Box::new(copied_trigger))
            } else {
                TriggerSpec::Either(Box::new(copied_trigger), Box::new(cast_trigger))
            }));
        }
    }

    if let Some(cast) = cast_idx {
        let suffix_tokens = tokens.get(cast + 1..).unwrap_or_default();
        let suffix_envelope =
            crate::runtime_backend::grammar::trigger_subjects::parse_spell_filter_envelope(
                suffix_tokens,
            );
        let mut filter_tokens = &suffix_tokens[..suffix_envelope.end];
        if filter_tokens.is_empty() {
            let prefix_tokens =
                trigger_subject_grammar::trim_trailing_spell_auxiliary_tokens(&tokens[..cast]);
            if trigger_subject_grammar::spell_tokens_have_noun(prefix_tokens) {
                filter_tokens = prefix_tokens;
            }
        }
        let filter = parse_filter(filter_tokens)?;
        return Ok(Some(TriggerSpec::SpellCast {
            filter,
            mana_source_filter: None,
            caster: actor,
            timing,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
            from_not_hand,
        }));
    }

    if let Some(copy) = copy_idx {
        let filter = parse_filter(tokens.get(copy + 1..).unwrap_or_default())?;
        return Ok(Some(TriggerSpec::SpellCopied {
            filter,
            copier: actor,
        }));
    }

    Ok(None)
}

pub(crate) fn is_spawn_scion_token_mana_reminder(tokens: &[OwnedLexToken]) -> bool {
    trigger_subject_grammar::parse_trigger_sentence_surface_facts(tokens).spawn_scion_mana_reminder
}

pub(crate) fn is_round_up_each_time_sentence(tokens: &[OwnedLexToken]) -> bool {
    trigger_subject_grammar::parse_trigger_sentence_surface_facts(tokens).round_up_each_time
}

pub(crate) enum MayCastItVerb {
    Cast,
    Play,
}

pub(crate) struct MayCastTaggedSpec {
    pub(crate) tag: TagKey,
    pub(crate) player: PlayerAst,
    pub(crate) verb: MayCastItVerb,
    pub(crate) as_copy: bool,
    pub(crate) without_paying_mana_cost: bool,
    pub(crate) predicate: Option<PredicateAst>,
    pub(crate) cost_reduction: Option<ManaCost>,
}

pub(crate) fn parse_may_cast_it_sentence(tokens: &[OwnedLexToken]) -> Option<MayCastTaggedSpec> {
    let clause_words = crate::runtime_backend::lexer::parser_token_word_refs(tokens);
    let facts = trigger_subject_grammar::parse_may_cast_sentence_facts(&clause_words)?;
    use trigger_subject_grammar::{
        MayCastManaValueParity, MayCastSurfaceReference, MayCastSurfaceSubject, MayCastSurfaceVerb,
        MayCastTailSurface,
    };

    let (player, subject_tag) = match facts.subject {
        MayCastSurfaceSubject::You => (PlayerAst::Implicit, None),
        MayCastSurfaceSubject::ExiledCardsOwner => (
            PlayerAst::ItsOwner,
            Some(TagKey::from(crate::tag::SOURCE_EXILED_TAG)),
        ),
    };
    let verb = match facts.verb {
        MayCastSurfaceVerb::Cast => MayCastItVerb::Cast,
        MayCastSurfaceVerb::Play => MayCastItVerb::Play,
    };
    let (tag, as_copy) = match facts.reference {
        MayCastSurfaceReference::It => (TagKey::from(IT_TAG), false),
        MayCastSurfaceReference::ThatCard => {
            (subject_tag.unwrap_or_else(|| TagKey::from(IT_TAG)), false)
        }
        MayCastSurfaceReference::ExiledCard => (TagKey::from(crate::tag::SOURCE_EXILED_TAG), false),
        MayCastSurfaceReference::RevealedCard => (TagKey::from("__last_revealed__"), false),
        MayCastSurfaceReference::Copy => (TagKey::from(IT_TAG), true),
    };
    let (without_paying_mana_cost, predicate) = match facts.tail {
        MayCastTailSurface::None => (false, None),
        MayCastTailSurface::WithoutPayingManaCost => (true, None),
        MayCastTailSurface::ManaValueAtMost { value_words } => {
            let value_words = clause_words.get(value_words)?;
            let (value, used) = parse_value_expr_words(value_words)?;
            if used != value_words.len() {
                return None;
            }
            (
                true,
                Some(PredicateAst::ItMatches(
                    ObjectFilter::default().with_mana_value(
                        crate::filter::Comparison::LessThanOrEqualExpr(Box::new(value)),
                    ),
                )),
            )
        }
        MayCastTailSurface::ManaValueParity(parity) => {
            let parity = match parity {
                MayCastManaValueParity::Odd => crate::filter::ParityRequirement::Odd,
                MayCastManaValueParity::Even => crate::filter::ParityRequirement::Even,
            };
            (
                true,
                Some(PredicateAst::ItMatches(
                    ObjectFilter::default().with_mana_value_parity(parity),
                )),
            )
        }
    };

    Some(MayCastTaggedSpec {
        tag,
        player,
        verb,
        as_copy,
        without_paying_mana_cost,
        predicate,
        cost_reduction: None,
    })
}

pub(crate) fn parse_copy_reference_cost_reduction_sentence(
    tokens: &[OwnedLexToken],
) -> Option<ManaCost> {
    let shape = crate::runtime_backend::grammar::trigger_subjects::parse_copy_reference_cost_reduction_shape_tokens(tokens)?;
    let reduction_tokens = trim_commas(&tokens[shape.reduction_tokens]).to_vec();
    let (reduction, consumed) = parse_cost_modifier_mana_cost(&reduction_tokens)?;
    if consumed != reduction_tokens.len() {
        return None;
    }
    Some(reduction)
}

pub(crate) fn build_may_cast_tagged_effect(spec: &MayCastTaggedSpec) -> EffectAst {
    let cast = EffectAst::subject_verb_cast_tagged(
        spec.tag.clone(),
        spec.player,
        matches!(spec.verb, MayCastItVerb::Play),
        spec.as_copy,
        spec.without_paying_mana_cost,
        spec.cost_reduction.clone(),
    );
    let may = if matches!(spec.player, PlayerAst::Implicit | PlayerAst::You) {
        EffectAst::May {
            effects: vec![cast],
        }
    } else {
        EffectAst::MayByPlayer {
            player: spec.player,
            effects: vec![cast],
        }
    };
    if let Some(predicate) = &spec.predicate {
        EffectAst::Conditional {
            predicate: predicate.clone(),
            if_true: vec![may],
            if_false: Vec::new(),
        }
    } else {
        may
    }
}

pub(crate) fn is_simple_copy_reference_sentence(tokens: &[OwnedLexToken]) -> bool {
    crate::runtime_backend::grammar::trigger_subjects::parse_simple_copy_reference_tokens(tokens)
        .is_some()
}

pub(crate) fn token_name_mentions_eldrazi_spawn_or_scion(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    (lower.matches("eldrazi").next().is_some() && lower.matches("spawn").next().is_some())
        || (lower.matches("eldrazi").next().is_some() && lower.matches("scion").next().is_some())
}

pub(crate) fn effect_creates_eldrazi_spawn_or_scion(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(subject_verb)
            if matches!(
                &subject_verb.action,
                crate::runtime_backend::ast::SubjectVerbActionAst::CreateTokenWithMods {
                    name,
                    ..
                } if token_name_mentions_eldrazi_spawn_or_scion(name)
            ) =>
        {
            true
        }
        _ => {
            let mut found = false;
            for_each_nested_effects(effect, false, |nested| {
                if !found && nested.iter().any(effect_creates_eldrazi_spawn_or_scion) {
                    found = true;
                }
            });
            found
        }
    }
}

pub(crate) fn effect_creates_any_token(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(subject_verb)
            if matches!(
                &subject_verb.action,
                crate::runtime_backend::ast::SubjectVerbActionAst::Populate { .. }
                    | crate::runtime_backend::ast::SubjectVerbActionAst::CreateTokenWithMods {
                        ..
                    }
                    | crate::runtime_backend::ast::SubjectVerbActionAst::CreateTokenCopy { .. }
                    | crate::runtime_backend::ast::SubjectVerbActionAst::CreateTokenCopyFromSource {
                        ..
                    }
            ) =>
        {
            true
        }
        _ => {
            let mut found = false;
            for_each_nested_effects(effect, false, |nested| {
                if !found && nested.iter().any(effect_creates_any_token) {
                    found = true;
                }
            });
            found
        }
    }
}

pub(crate) fn last_created_token_info(
    effects: &[EffectAst],
) -> Option<(
    String,
    crate::runtime_backend::token_definition::TokenDefinitionSpec,
    PlayerAst,
)> {
    for effect in effects.iter().rev() {
        if let Some(info) = created_token_info_from_effect(effect) {
            return Some(info);
        }
    }
    None
}

pub(crate) fn created_token_info_from_effect(
    effect: &EffectAst,
) -> Option<(
    String,
    crate::runtime_backend::token_definition::TokenDefinitionSpec,
    PlayerAst,
)> {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            crate::runtime_backend::ast::SubjectVerbActionAst::CreateTokenWithMods {
                name,
                definition,
                player,
                ..
            } => Some((name.clone(), definition.clone(), *player)),
            _ => {
                let mut found = None;
                for_each_nested_effects(effect, true, |nested| {
                    if found.is_none() {
                        found = last_created_token_info(nested);
                    }
                });
                found
            }
        },
        _ => {
            let mut found = None;
            for_each_nested_effects(effect, true, |nested| {
                if found.is_none() {
                    found = last_created_token_info(nested);
                }
            });
            found
        }
    }
}

pub(crate) fn title_case_token_word(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => {
            let mut out = first.to_uppercase().to_string();
            out.push_str(chars.as_str());
            out
        }
        None => String::new(),
    }
}

pub(crate) fn controller_filter_for_token_player(player: PlayerAst) -> Option<PlayerFilter> {
    match player {
        PlayerAst::You | PlayerAst::Implicit => Some(PlayerFilter::You),
        PlayerAst::Opponent => Some(PlayerFilter::Opponent),
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::target_opponent()),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        PlayerAst::Defending => Some(PlayerFilter::Defending),
        _ => None,
    }
}

pub(crate) fn parse_sentence_exile_that_token_when_source_leaves(
    tokens: &[OwnedLexToken],
    prior_effects: &[EffectAst],
) -> Option<EffectAst> {
    use crate::runtime_backend::grammar::trigger_subjects::TokenLifecycleSentenceKind;

    let kind =
        crate::runtime_backend::grammar::trigger_subjects::parse_token_lifecycle_sentence_tokens(
            tokens,
        )?;
    if kind != TokenLifecycleSentenceKind::ExileCreatedTokenWhenSourceLeaves {
        return None;
    }

    let _ = last_created_token_info(prior_effects)?;

    Some(EffectAst::subject_verb_exile_when_source_leaves(
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
    ))
}

pub(crate) fn parse_sentence_sacrifice_source_when_that_token_leaves(
    tokens: &[OwnedLexToken],
    prior_effects: &[EffectAst],
) -> Option<EffectAst> {
    use crate::runtime_backend::grammar::trigger_subjects::TokenLifecycleSentenceKind;

    let kind =
        crate::runtime_backend::grammar::trigger_subjects::parse_token_lifecycle_sentence_tokens(
            tokens,
        )?;
    if kind != TokenLifecycleSentenceKind::SacrificeSourceWhenCreatedTokenLeaves {
        return None;
    }

    let _ = last_created_token_info(prior_effects)?;

    Some(EffectAst::subject_verb_sacrifice_source_when_leaves(
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
    ))
}

pub(crate) fn is_generic_token_reminder_sentence(tokens: &[OwnedLexToken]) -> bool {
    crate::runtime_backend::grammar::token_definitions::parse_token_reminder_sentence_kind_tokens(
        tokens,
    )
    .is_some()
}

pub(crate) fn strip_embedded_token_rules_text(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let append_outer_where_x_tail = |stripped: &mut Vec<OwnedLexToken>| {
        // Inline token rules are parsed separately and attached to the token
        // blueprint, but an outer value binding after the closing quote still
        // belongs to the create action:
        //
        // `Create X ... tokens with "...," where X is ...`
        //
        // Keep that typed tail while removing only the embedded rule. Counting
        // quotes before the marker prevents a `where X is` inside the granted
        // ability itself from being promoted to the outer create effect.
        if let Some(where_shape) =
            crate::runtime_backend::grammar::effects::sentence_predicate_shapes::parse_where_x_sentence_tokens(tokens)
        {
            let quote_count = where_shape
                .stripped_tokens
                .iter()
                .filter(|token| token.kind == crate::runtime_backend::lexer::TokenKind::Quote)
                .count();
            if quote_count >= 2 && quote_count % 2 == 0 {
                stripped.extend(where_shape.where_tokens.iter().cloned());
            }
        }
    };

    if let Some(with_idx) =
        trigger_subject_grammar::parse_embedded_token_rules_boundary_tokens(tokens)
    {
        let mut stripped = tokens[..with_idx].to_vec();
        append_outer_where_x_tail(&mut stripped);
        return stripped;
    }

    // A token can have ordinary keyword modifiers before its quoted rule:
    // `... token with flying and "{R}: This token gets ..."`.
    // Keep those modifiers available to the create parser while removing the
    // quoted suffix that contains its own verbs and activation colon. The
    // untouched source tokens are used later to attach the quoted rule to the
    // token blueprint.
    let opening_quote = tokens
        .iter()
        .position(|token| token.kind == crate::runtime_backend::lexer::TokenKind::Quote);
    if let Some(opening_quote) = opening_quote
        && tokens[opening_quote + 1..]
            .iter()
            .any(|token| token.kind == crate::runtime_backend::lexer::TokenKind::Quote)
        && tokens[..opening_quote]
            .iter()
            .any(|token| token.is_word("create"))
        && tokens[..opening_quote]
            .iter()
            .any(|token| token.is_any_word(&["token", "tokens"]))
        && let Some(with_idx) = tokens[..opening_quote]
            .iter()
            .rposition(|token| token.is_word("with"))
        && with_idx + 1 < opening_quote
    {
        let mut prefix_end = opening_quote;
        if tokens
            .get(prefix_end.saturating_sub(1))
            .is_some_and(|token| token.is_word("and"))
        {
            prefix_end -= 1;
        }
        while tokens
            .get(prefix_end.saturating_sub(1))
            .is_some_and(OwnedLexToken::is_comma)
        {
            prefix_end -= 1;
        }
        let mut stripped = tokens[..prefix_end].to_vec();
        append_outer_where_x_tail(&mut stripped);
        return stripped;
    }
    tokens.to_vec()
}

pub(crate) fn append_token_reminder_to_last_create_effect(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    if tokens.is_empty() {
        return Ok(false);
    }
    let reminder =
        crate::runtime_backend::grammar::token_definitions::parse_token_reminder_facts_tokens(
            tokens,
        );
    let sentence_kind = crate::runtime_backend::grammar::token_definitions::
        parse_token_reminder_sentence_kind_tokens(tokens);
    let ability_presentation = match sentence_kind {
        Some(
            crate::runtime_backend::grammar::token_definitions::TokenReminderSentenceKind::GrantedAbility,
        ) => Some(
            if crate::runtime_backend::grammar::token_definitions::
                token_ability_sentence_uses_gain_verb(tokens)
            {
                ironsmith_core::TokenAbilityPresentation::SeparateSentenceGain
            } else {
                ironsmith_core::TokenAbilityPresentation::SeparateSentence
            },
        ),
        _ => None,
    };
    let standalone_ability_sentence = matches!(
        sentence_kind,
        Some(
            crate::runtime_backend::grammar::token_definitions::TokenReminderSentenceKind::PronounTrigger
                | crate::runtime_backend::grammar::token_definitions::TokenReminderSentenceKind::ExplicitTokenReference
        )
    );
    for effect in effects.iter_mut().rev() {
        // A separately authored `It has "This token's power and toughness
        // ..."` sentence is a characteristic-defining ability of the token,
        // not a one-time base-P/T assignment made by the create effect. Give
        // the nested token-identity parser first refusal for this shape; the
        // dynamic reminder path below remains the fallback for unquoted
        // copy/snapshot P/T clauses.
        if reminder.dynamic_power_toughness.is_some()
            && append_token_granted_ability_to_effect(Some(effect), tokens)?
        {
            return Ok(true);
        }
        // Specialized reminder facts must win over the generic granted-ability
        // parser. Otherwise a quoted rule such as "When this token dies ..."
        // is retained as an opaque granted ability and its typed token rule is
        // lost before the merge path gets a chance to install it.
        if append_token_reminder_to_effect(
            Some(effect),
            &reminder,
            ability_presentation,
            standalone_ability_sentence,
        ) {
            return Ok(true);
        }
        if append_token_granted_ability_to_effect(Some(effect), tokens)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn append_token_granted_ability_to_effect(
    effect: Option<&mut EffectAst>,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    let Some(effect) = effect else {
        return Ok(false);
    };
    match effect {
        EffectAst::SubjectVerb(subject_verb) => {
            let crate::runtime_backend::ast::SubjectVerbActionAst::CreateTokenWithMods {
                definition,
                granted_abilities,
                ability_presentation,
                ..
            } = &mut subject_verb.action
            else {
                return Ok(false);
            };
            let Some(ability_tokens) = crate::runtime_backend::grammar::effects::dispatch_entry_shapes::parse_token_granted_ability_tokens(tokens) else {
                return Ok(false);
            };
            let Ok(parsed) = crate::runtime_backend::sentences::effect_sentences::parse_granted_abilities_for_token_definition(
                definition,
                ability_tokens,
            ) else {
                // Older token-reminder shapes below still cover several
                // specialized rules. An unsupported generic nested ability
                // must leave those fallbacks available.
                return Ok(false);
            };
            if parsed.is_empty() {
                return Ok(false);
            }
            let combine_separate_sentence =
                !definition.has_intrinsic_abilities() && granted_abilities.is_empty();
            // Same rule as the token-followup applier in dispatch_entry.rs: when
            // the creation sentence already carried the keywords inline ("… Bat
            // creature token with flying.") this sentence is an ADDITIONAL one,
            // so it must not claim the grouped presentation and pull those
            // keywords into their own "It has flying." sentence.
            let keywords_authored_inline = ability_presentation.is_none()
                && definition.has_intrinsic_abilities()
                && granted_abilities.is_empty();
            for ability in parsed {
                if !granted_abilities.contains(&ability) {
                    granted_abilities.push(ability);
                }
            }
            let presentation = if crate::runtime_backend::grammar::token_definitions::
                token_ability_sentence_uses_gain_verb(tokens)
            {
                ironsmith_core::TokenAbilityPresentation::SeparateSentenceGain
            } else {
                ironsmith_core::TokenAbilityPresentation::SeparateSentence
            };
            *ability_presentation = Some(if keywords_authored_inline {
                ironsmith_core::TokenAbilityPresentation::with_added_standalone_tail(None)
            } else if combine_separate_sentence {
                presentation.combined_separate_sentence()
            } else {
                presentation
            });
            Ok(true)
        }
        _ => {
            let mut applied = false;
            let mut error = None;
            for_each_nested_effects_mut(effect, false, |nested| {
                if applied || error.is_some() {
                    return;
                }
                match append_token_granted_ability_to_effect(nested.last_mut(), tokens) {
                    Ok(value) => applied = value,
                    Err(value) => error = Some(value),
                }
            });
            if let Some(error) = error {
                Err(error)
            } else {
                Ok(applied)
            }
        }
    }
}

pub(crate) fn append_token_reminder_to_effect(
    effect: Option<&mut EffectAst>,
    reminder: &crate::runtime_backend::grammar::token_definitions::TokenReminderFacts,
    ability_presentation: Option<ironsmith_core::TokenAbilityPresentation>,
    standalone_ability_sentence: bool,
) -> bool {
    let Some(effect) = effect else {
        return false;
    };
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            crate::runtime_backend::ast::SubjectVerbActionAst::Populate {
                has_haste,
                exile_at_end_of_combat,
                sacrifice_at_next_end_step,
                exile_at_next_end_step,
                next_end_step_player,
                ..
            } => {
                if reminder.has_haste {
                    *has_haste = true;
                    return true;
                }
                if reminder.sacrifice_at_next_end_step {
                    *sacrifice_at_next_end_step = true;
                    *next_end_step_player = reminder.next_end_step_player.clone();
                    return true;
                }
                if reminder.exile_at_next_end_step {
                    *exile_at_next_end_step = true;
                    *next_end_step_player = reminder.next_end_step_player.clone();
                    return true;
                }
                if reminder.exile_at_end_of_combat {
                    *exile_at_end_of_combat = true;
                    return true;
                }
                false
            }
            crate::runtime_backend::ast::SubjectVerbActionAst::CreateTokenCopy {
                has_haste,
                exile_at_end_of_combat,
                sacrifice_at_next_end_step,
                exile_at_next_end_step,
                next_end_step_player,
                ..
            }
            | crate::runtime_backend::ast::SubjectVerbActionAst::CreateTokenCopyFromSource {
                has_haste,
                exile_at_end_of_combat,
                sacrifice_at_next_end_step,
                exile_at_next_end_step,
                next_end_step_player,
                ..
            } => {
                if reminder.has_haste {
                    *has_haste = true;
                    return true;
                }
                if reminder.sacrifice_at_next_end_step {
                    *sacrifice_at_next_end_step = true;
                    *next_end_step_player = reminder.next_end_step_player.clone();
                }
                if reminder.exile_at_next_end_step {
                    *exile_at_next_end_step = true;
                    *next_end_step_player = reminder.next_end_step_player.clone();
                }
                if reminder.exile_at_end_of_combat {
                    *exile_at_end_of_combat = true;
                }
                *has_haste
                    || *sacrifice_at_next_end_step
                    || *exile_at_next_end_step
                    || *exile_at_end_of_combat
            }
            crate::runtime_backend::ast::SubjectVerbActionAst::CreateTokenWithMods {
                definition,
                dynamic_power_toughness,
                exile_at_end_of_combat,
                sacrifice_at_end_of_combat,
                sacrifice_at_next_end_step,
                exile_at_next_end_step,
                next_end_step_player,
                ability_presentation: create_ability_presentation,
                ..
            } => {
                if let Some((power, toughness)) = &reminder.dynamic_power_toughness {
                    *dynamic_power_toughness = Some((power.clone(), toughness.clone()));
                    return true;
                }
                let combine_separate_sentence = !definition.has_intrinsic_abilities();
                let merged_definition =
                    crate::runtime_backend::grammar::token_definitions::merge_token_reminder_definition(
                        definition, reminder,
                    );
                if merged_definition {
                    if let Some(presentation) = ability_presentation {
                        *create_ability_presentation = Some(if combine_separate_sentence {
                            presentation.combined_separate_sentence()
                        } else {
                            presentation
                        });
                    } else if standalone_ability_sentence {
                        *create_ability_presentation = Some(
                            ironsmith_core::TokenAbilityPresentation::with_added_standalone_tail(
                                *create_ability_presentation,
                            ),
                        );
                    }
                }
                if reminder.sacrifice_at_next_end_step {
                    *sacrifice_at_next_end_step = true;
                    *next_end_step_player = reminder.next_end_step_player.clone();
                }
                if reminder.exile_at_next_end_step {
                    *exile_at_next_end_step = true;
                    *next_end_step_player = reminder.next_end_step_player.clone();
                }
                if reminder.exile_at_end_of_combat {
                    *exile_at_end_of_combat = true;
                }
                if reminder.sacrifice_at_end_of_combat {
                    *sacrifice_at_end_of_combat = true;
                }
                merged_definition
                    || reminder.sacrifice_at_next_end_step
                    || reminder.exile_at_next_end_step
                    || reminder.exile_at_end_of_combat
                    || reminder.sacrifice_at_end_of_combat
            }
            _ => false,
        },
        _ => {
            let mut applied = false;
            for_each_nested_effects_mut(effect, false, |nested| {
                if !applied {
                    applied = append_token_reminder_to_effect(
                        nested.last_mut(),
                        reminder,
                        ability_presentation,
                        standalone_ability_sentence,
                    );
                }
            });
            applied
        }
    }
}

#[cfg(test)]
mod typed_trigger_subject_migration_tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn typed_spell_activity_facts_preserve_trigger_spec_fields() {
        let tokens = lex_line("you cast a spell during your turn", 0).unwrap();
        let trigger = parse_spell_activity_trigger(&tokens).unwrap().unwrap();
        assert!(matches!(
            trigger,
            TriggerSpec::SpellCast {
                filter: None,
                mana_source_filter: None,
                caster: PlayerFilter::You,
                timing: None,
                during_turn: Some(PlayerFilter::You),
                min_spells_this_turn: None,
                exact_spells_this_turn: None,
                from_not_hand: false,
            }
        ));
    }

    #[test]
    fn passive_spell_cast_during_turn_keeps_its_preverb_filter() {
        let tokens = lex_line("an instant or sorcery spell is cast during your turn", 0).unwrap();
        let trigger = parse_spell_activity_trigger(&tokens).unwrap().unwrap();
        let TriggerSpec::SpellCast {
            filter: Some(filter),
            caster,
            timing,
            during_turn,
            ..
        } = trigger
        else {
            panic!("expected a filtered passive cast trigger, got {trigger:?}");
        };
        assert_eq!(caster, PlayerFilter::Any);
        assert_eq!(timing, None);
        assert_eq!(during_turn, Some(PlayerFilter::You));
        assert_eq!(
            filter.card_types,
            vec![
                crate::types::CardType::Instant,
                crate::types::CardType::Sorcery
            ]
        );
    }

    #[test]
    fn passive_nth_spell_of_turn_uses_global_ordinal_trigger() {
        let tokens = lex_line("the fourth spell of a turn is cast", 0).unwrap();
        let trigger = parse_spell_activity_trigger(&tokens).unwrap().unwrap();

        assert_eq!(trigger, TriggerSpec::NthSpellOfTurnCast { spell_number: 4 });
    }

    #[test]
    fn spell_cast_during_combat_keeps_typed_timing() {
        let tokens = lex_line("you cast a spell during combat", 0).unwrap();
        let trigger = parse_spell_activity_trigger(&tokens).unwrap().unwrap();
        assert!(matches!(
            trigger,
            TriggerSpec::SpellCast {
                filter: None,
                mana_source_filter: None,
                caster: PlayerFilter::You,
                timing: Some(ironsmith_core::TriggerTimingRestriction::DuringCombat),
                during_turn: None,
                min_spells_this_turn: None,
                exact_spells_this_turn: None,
                from_not_hand: false,
            }
        ));
    }

    #[test]
    fn spell_cast_filters_preserve_serial_type_and_subtype_unions() {
        let tokens = lex_line("you cast an instant, sorcery, or Wizard spell", 0).unwrap();
        let trigger = parse_spell_activity_trigger(&tokens).unwrap().unwrap();
        let TriggerSpec::SpellCast {
            filter: Some(filter),
            ..
        } = trigger
        else {
            panic!("expected a filtered spell-cast trigger, got {trigger:?}");
        };
        assert_eq!(
            filter.card_types,
            vec![
                crate::types::CardType::Instant,
                crate::types::CardType::Sorcery,
            ]
        );
        assert_eq!(filter.subtypes, vec![crate::types::Subtype::Wizard]);
        assert!(filter.type_or_subtype_union);

        let tokens = lex_line("you cast a Pegasus, Unicorn, or Horse creature spell", 0).unwrap();
        let trigger = parse_spell_activity_trigger(&tokens).unwrap().unwrap();
        let TriggerSpec::SpellCast {
            filter: Some(filter),
            ..
        } = trigger
        else {
            panic!("expected a filtered spell-cast trigger, got {trigger:?}");
        };
        assert_eq!(filter.card_types, vec![crate::types::CardType::Creature]);
        assert_eq!(
            filter.subtypes,
            vec![
                crate::types::Subtype::Pegasus,
                crate::types::Subtype::Unicorn,
                crate::types::Subtype::Horse,
            ]
        );
        assert!(!filter.type_or_subtype_union);

        let tokens = lex_line("you cast a creature spell", 0).unwrap();
        let trigger = parse_spell_activity_trigger(&tokens).unwrap().unwrap();
        let TriggerSpec::SpellCast {
            filter: Some(filter),
            ..
        } = trigger
        else {
            panic!("expected a filtered spell-cast trigger, got {trigger:?}");
        };
        assert_eq!(filter.card_types, vec![crate::types::CardType::Creature]);
        assert!(filter.subtypes.is_empty());
        assert!(!filter.type_or_subtype_union);
    }

    #[test]
    fn spell_cast_filter_preserves_graveyard_origin_when_generic_filter_parses() {
        let tokens = lex_line("you cast a spell from your graveyard", 0).unwrap();
        let trigger = parse_spell_activity_trigger(&tokens).unwrap().unwrap();
        let TriggerSpec::SpellCast {
            filter: Some(filter),
            caster,
            ..
        } = trigger
        else {
            panic!("expected a filtered spell-cast trigger, got {trigger:?}");
        };

        assert_eq!(caster, PlayerFilter::You);
        assert_eq!(filter.zone, Some(Zone::Graveyard), "{filter:#?}");
        assert_eq!(filter.owner, Some(PlayerFilter::You), "{filter:#?}");
    }

    #[test]
    fn trigger_clause_dispatch_preserves_spell_cast_origin_zones() {
        for (text, expected_zone) in [
            ("you cast a spell from exile", Zone::Exile),
            ("you cast a spell from your graveyard", Zone::Graveyard),
            ("you cast a spell from your hand", Zone::Hand),
        ] {
            let tokens = lex_line(text, 0).unwrap();
            let trigger = super::trigger_clause_core::parse_trigger_clause_lexed(&tokens).unwrap();
            let TriggerSpec::SpellCast {
                filter: Some(filter),
                caster,
                ..
            } = trigger
            else {
                panic!("expected an origin-qualified spell-cast trigger for {text}");
            };
            assert_eq!(caster, PlayerFilter::You, "{text}: {filter:#?}");
            assert_eq!(filter.zone, Some(expected_zone), "{text}: {filter:#?}");
        }
    }

    #[test]
    fn typed_subject_facts_preserve_object_filter_semantics() {
        let tokens = lex_line("other creature you control", 0).unwrap();
        let filter = parse_trigger_subject_filter_lexed(&tokens)
            .unwrap()
            .unwrap();
        assert!(filter.other);
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(filter.card_types, vec![crate::types::CardType::Creature]);
    }

    #[test]
    fn coordinated_trigger_subject_keeps_branch_local_combat_state_and_controller() {
        let tokens = lex_line(
            "an attacking creature you control or a blocking creature an opponent controls",
            0,
        )
        .unwrap();
        let filter = parse_trigger_subject_filter_lexed(&tokens)
            .unwrap()
            .expect("coordinated trigger subject");

        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(filter.controller, None);
        assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
        assert!(filter.any_of.iter().any(|branch| {
            branch.attacking && !branch.blocking && branch.controller == Some(PlayerFilter::You)
        }));
        assert!(filter.any_of.iter().any(|branch| {
            branch.blocking
                && !branch.attacking
                && branch.controller == Some(PlayerFilter::Opponent)
        }));
    }

    #[test]
    fn typed_may_cast_facts_preserve_tagged_semantics() {
        let tokens = lex_line(
            "the exiled cards owner may play that card without paying its mana cost",
            0,
        )
        .unwrap();
        let spec = parse_may_cast_it_sentence(&tokens).unwrap();
        assert_eq!(spec.tag.as_str(), crate::tag::SOURCE_EXILED_TAG);
        assert!(matches!(spec.player, PlayerAst::ItsOwner));
        assert!(matches!(spec.verb, MayCastItVerb::Play));
        assert!(!spec.as_copy);
        assert!(spec.without_paying_mana_cost);
        assert!(spec.predicate.is_none());
    }

    #[test]
    fn stripping_inline_token_rule_preserves_outer_where_x_binding() {
        let tokens = lex_line(
            "Create X 1/1 black Rat creature tokens with \"This token can't block,\" where X is the amount of damage dealt to it this turn.",
            0,
        )
        .expect("quoted token where-x text should lex");
        let stripped = strip_embedded_token_rules_text(&tokens);
        let words = crate::runtime_backend::token_word_refs(&stripped)
            .into_iter()
            .map(str::to_ascii_lowercase)
            .collect::<Vec<_>>();

        assert!(
            words.starts_with(&["create".into(), "x".into()]),
            "{words:?}"
        );
        assert!(
            words.windows(2).any(|window| window == ["black", "rat"]),
            "{words:?}"
        );
        assert!(
            words
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .ends_with(&[
                    "where", "x", "is", "the", "amount", "of", "damage", "dealt", "to", "it",
                    "this", "turn"
                ]),
            "{words:?}"
        );
        assert!(!words.iter().any(|word| word == "block"), "{words:?}");

        let inner_only = lex_line(
            "Create a 1/1 blue Illusion creature token with \"This token gets +X/+0, where X is its power.\"",
            0,
        )
        .expect("inner where-x token text should lex");
        let stripped_inner = strip_embedded_token_rules_text(&inner_only);
        let inner_words = crate::runtime_backend::token_word_refs(&stripped_inner)
            .into_iter()
            .map(str::to_ascii_lowercase)
            .collect::<Vec<_>>();
        assert!(
            !inner_words.iter().any(|word| word == "where"),
            "an inner token ability binding must not become the create count: {inner_words:?}"
        );
    }
}
