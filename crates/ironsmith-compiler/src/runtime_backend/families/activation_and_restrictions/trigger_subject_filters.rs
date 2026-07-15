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
    let intrinsic_attachment_state = subject_words
        .iter()
        .enumerate()
        .find_map(|(idx, word)| {
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

    let mut normalized_subject_tokens =
        trigger_subject_grammar::normalize_each_with_tokens(subject_tokens);

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
                filter.tagged_constraints.push(crate::filter::TaggedObjectConstraint {
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
    let Some(mut filter) = parse_trigger_subject_filter_lexed(subject_tokens)? else {
        return Ok(None);
    };

    if filter.card_types.is_empty() {
        filter.card_types.push(crate::types::CardType::Creature);
    } else if filter.card_types.len() > 1 && filter.all_card_types.is_empty() {
        filter.all_card_types = std::mem::take(&mut filter.card_types);
    }

    Ok(Some(filter))
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
                    Ok(filter) => Ok(Some(filter)),
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
                caster: actor.clone(),
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
            caster: actor,
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
    if let Some(with_idx) =
        trigger_subject_grammar::parse_embedded_token_rules_boundary_tokens(tokens)
    {
        return tokens[..with_idx].to_vec();
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
    for effect in effects.iter_mut().rev() {
        if append_token_granted_ability_to_effect(Some(effect), tokens)? {
            return Ok(true);
        }
        if append_token_reminder_to_effect(Some(effect), &reminder) {
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
            for ability in parsed {
                if !granted_abilities.contains(&ability) {
                    granted_abilities.push(ability);
                }
            }
            *ability_presentation =
                Some(ironsmith_core::TokenAbilityPresentation::SeparateSentence);
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
                ..
            } => {
                if let Some((power, toughness)) = &reminder.dynamic_power_toughness {
                    *dynamic_power_toughness = Some((power.clone(), toughness.clone()));
                    return true;
                }
                crate::runtime_backend::grammar::token_definitions::merge_token_reminder_definition(
                    definition, reminder,
                );
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
                true
            }
            _ => false,
        },
        _ => {
            let mut applied = false;
            for_each_nested_effects_mut(effect, false, |nested| {
                if !applied {
                    applied = append_token_reminder_to_effect(nested.last_mut(), reminder);
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
                caster: PlayerFilter::You,
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
            during_turn,
            ..
        } = trigger
        else {
            panic!("expected a filtered passive cast trigger, got {trigger:?}");
        };
        assert_eq!(caster, PlayerFilter::Any);
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
}
