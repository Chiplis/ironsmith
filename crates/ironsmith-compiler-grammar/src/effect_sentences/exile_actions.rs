use crate::cards::builders::ConditionalEffectAst;
use crate::cards::builders::ObjectChoiceEffectAst;
use crate::cards::builders::ForEachEffectAst;
use super::*;
use crate::CardType;
use crate::cards::builders::{SubjectVerbActionAst, SubjectVerbEffectAst, LibraryActionAst};
use crate::grammar::effects as effect_grammar;
use crate::grammar::effects::control_copy_attach_shapes as cca_shapes;
use crate::lexer::LexedClause;

pub fn parse_each_opponent_exiles_card_from_their_hand_or_permanent_they_control(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    if let Some(shape) =
        effect_grammar::parse_each_player_exile_counted_hand_permanent_shape(tokens)
    {
        let effects = exile_iterated_hand_cards_and_permanents(
            tokens,
            crate::effect::ChoiceCount::dynamic_x(),
            Some(Value::X),
        );
        return Some(match shape.group {
            effect_grammar::EachPlayerExileGroup::Player => EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects }),
            effect_grammar::EachPlayerExileGroup::Opponent => {
                EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
            }
        });
    }

    let shape = effect_grammar::parse_each_opponent_exile_choice_shape(tokens)?;
    parse_exile_card_from_their_hand_or_permanent_they_control(
        &shape.choice,
        Some(SubjectAst::Player(PlayerAst::Opponent)),
    )
}

/// Keep re-quantified bare card domains independent from a preceding typed
/// collection. The ordinary shared-selector grammar can represent
/// `artifacts, creatures, and lands from the battlefield, all cards from all
/// graveyards, and all cards from all hands` as one outer type list over three
/// zone arms. That incorrectly applies the permanent types to graveyards and
/// hands. Authored `all cards from all <zone>` arms prove that those zones are
/// bare domains, so scope the outer type selector only to the remaining arms.
pub fn scope_types_away_from_requantified_bare_card_domains(
    tokens: &[OwnedLexToken],
    mut filter: ObjectFilter,
) -> ObjectFilter {
    if filter.has_conjunctive_set_surface() && filter.card_types.is_empty() {
        let mut collapsed: Vec<ObjectFilter> = Vec::new();
        for branch in std::mem::take(&mut filter.any_of) {
            let mut plain_zone = branch.clone();
            let card_types = std::mem::take(&mut plain_zone.card_types);
            let mergeable = !card_types.is_empty() && plain_zone.zone.is_some() && {
                let zone = plain_zone.zone;
                plain_zone.zone = None;
                let plain = plain_zone == ObjectFilter::default();
                plain_zone.zone = zone;
                plain
            };
            if mergeable
                && let Some(existing_idx) =
                    crate::slice_primitives::select_position(&collapsed, |existing| {
                        existing.zone == branch.zone && {
                            let mut plain = (*existing).clone();
                            plain.zone = None;
                            plain.card_types.clear();
                            plain == ObjectFilter::default()
                        }
                    })
            {
                let existing = &mut collapsed[existing_idx];
                for card_type in card_types {
                    if !crate::slice_primitives::contains(&existing.card_types, &card_type) {
                        existing.card_types.push(card_type);
                    }
                }
            } else {
                collapsed.push(branch);
            }
        }
        filter.any_of = collapsed;
    }
    let words = crate::lexer::token_word_refs(tokens);
    let mut bare_zones = Vec::new();
    let mut search_start = 0usize;
    while let Some(relative_start) =
        crate::slice_primitives::find_window_by(&words[search_start..], 5, |window| {
            crate::word_primitives::parse_sequence_complete(
                &window[..4],
                &["all", "cards", "from", "all"],
            )
        })
    {
        let window = &words[search_start + relative_start..search_start + relative_start + 5];
        if let Some(zone) = crate::util::parse_zone_word(window[4])
            && !crate::slice_primitives::contains(&bare_zones, &zone)
        {
            bare_zones.push(zone);
        }
        search_start += relative_start + 1;
    }
    if bare_zones.is_empty()
        || filter.card_types.is_empty()
        || filter.any_of.len() < 2
        || !filter.has_conjunctive_set_surface()
    {
        return filter;
    }

    let original = filter.clone();
    let selector = std::mem::take(&mut filter.card_types);
    let mut branches = std::mem::take(&mut filter.any_of);
    filter.union_surface = crate::filter::ObjectFilterUnionSurface::default();
    if filter != ObjectFilter::default() {
        return original;
    }

    let mut scoped_selector = false;
    let mut seen_bare_zones = Vec::new();
    for branch in &mut branches {
        let Some(zone) = branch.zone else {
            return original;
        };
        let mut bare_branch = branch.clone();
        bare_branch.zone = None;
        if bare_branch != ObjectFilter::default() {
            return original;
        }
        if crate::slice_primitives::contains(&bare_zones, &zone) {
            if !crate::slice_primitives::contains(&seen_bare_zones, &zone) {
                seen_bare_zones.push(zone);
            }
        } else {
            branch.card_types = selector.clone();
            scoped_selector = true;
        }
    }
    if !scoped_selector
        || bare_zones
            .iter()
            .any(|zone| !crate::slice_primitives::contains(&seen_bare_zones, zone))
    {
        return original;
    }

    let mut scoped = ObjectFilter {
        any_of: branches,
        ..ObjectFilter::default()
    };
    scoped.set_conjunctive_set_surface(true);
    scoped
}

fn exile_iterated_hand_cards_and_permanents(
    tokens: &[OwnedLexToken],
    count: crate::effect::ChoiceCount,
    count_value: Option<Value>,
) -> Vec<EffectAst> {
    let mut hand_card = ObjectFilter::default().in_zone(Zone::Hand);
    hand_card.owner = Some(PlayerFilter::IteratedPlayer);
    let mut permanent = ObjectFilter::permanent_card().in_zone(Zone::Battlefield);
    permanent.controller = Some(PlayerFilter::IteratedPlayer);

    let mut filter = ObjectFilter::default();
    filter.any_of = vec![hand_card, permanent];
    let tag = helper_tag_for_tokens(tokens, "exiled");
    vec![
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            count_value,
            player: PlayerAst::That,
            tag: crate::tag::TagRef::of(tag.clone()),
            zones: vec![Zone::Hand, Zone::Battlefield],
            search_mode: None,
        }),
        EffectAst::subject_verb_exile(TargetAst::Tagged(crate::tag::TagRef::of(tag), None), false),
    ]
}

fn parse_exile_card_from_their_hand_or_permanent_they_control(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Option<EffectAst> {
    if !effect_grammar::is_exile_hand_or_permanent_choice_shape(tokens) {
        return None;
    }

    let wrap_for_each_opponent = matches!(subject, Some(SubjectAst::Player(PlayerAst::Opponent)));
    let chooser = match subject {
        Some(SubjectAst::Player(PlayerAst::That | PlayerAst::Opponent)) => PlayerAst::That,
        Some(SubjectAst::This) => PlayerAst::That,
        _ => return None,
    };

    let mut effects = exile_iterated_hand_cards_and_permanents(
        tokens,
        crate::effect::ChoiceCount::exactly(1),
        None,
    );
    if let Some(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones { player, .. })) = effects.first_mut() {
        *player = chooser;
    }

    Some(if wrap_for_each_opponent {
        EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
    } else {
        EffectAst::Sequence { effects }
    })
}

pub use effect_grammar::ParsedExileOwnerPrefix as ParsedOwnerPrefix;

fn with_exile_actor(mut effect: EffectAst, subject: Option<SubjectAst>) -> EffectAst {
    if let Some(player) = extract_subject_player(subject)
        && let EffectAst::SubjectVerb(subject_verb) = &mut effect
    {
        subject_verb.subject.player = player;
    }
    effect
}

fn strip_source_top_only_prefix(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], bool) {
    use winnow::Parser as _;

    crate::grammar::primitives::parse_prefix(
        tokens,
        crate::grammar::primitives::phrase(&["the", "top"]).void(),
    )
    .map(|(_, rest)| (rest, true))
    .unwrap_or((tokens, false))
}

fn expand_hand_or_graveyard_target(mut target: TargetAst, tokens: &[OwnedLexToken]) -> TargetAst {
    if !cca_shapes::contains_graveyard_and_hand(tokens) {
        return target;
    }

    fn apply(target: &mut TargetAst) {
        match target {
            TargetAst::Object(filter, ..) | TargetAst::ObjectOrPlayer(filter, ..) => {
                filter.zone = None;
                filter.any_of = [Zone::Hand, Zone::Graveyard]
                    .into_iter()
                    .map(|zone| ObjectFilter::default().in_zone(zone))
                    .collect();
            }
            TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, ..) => apply(inner),
            _ => {}
        }
    }

    apply(&mut target);
    target
}

/// Parse an authored pair such as "a Human you control and an artifact you
/// control" as two independent selections. Repeated indefinite articles are
/// the semantic boundary: without them, `and` can still be joining
/// characteristics inside one object filter.
fn parse_independent_exile_pair(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    until_source_leaves: bool,
    face_down: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let words = crate::lexer::token_word_refs(tokens);
    if words.iter().filter(|word| **word == "and").count() != 1 {
        return Ok(None);
    }
    let Some((first_tokens, second_tokens)) =
        crate::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            crate::grammar::primitives::kw("and").void()
        })
    else {
        return Ok(None);
    };
    let starts_with_indefinite_article = |branch: &[OwnedLexToken]| {
        crate::lexer::token_word_refs(branch)
            .first()
            .is_some_and(|word| matches!(*word, "a" | "an"))
    };
    if !starts_with_indefinite_article(first_tokens)
        || !starts_with_indefinite_article(second_tokens)
    {
        return Ok(None);
    }

    let mut first = parse_target_phrase(first_tokens)?;
    let mut second = parse_target_phrase(second_tokens)?;
    let is_non_target_object = |target: &TargetAst| match target {
        TargetAst::Object(_, explicit_target_span, _) => explicit_target_span.is_none(),
        TargetAst::WithCount(inner, count) if count.is_single() => {
            matches!(
                inner.as_ref(),
                TargetAst::Object(_, explicit_target_span, _)
                    if explicit_target_span.is_none()
            )
        }
        _ => false,
    };
    if !is_non_target_object(&first) || !is_non_target_object(&second) {
        return Ok(None);
    }

    apply_exile_subject_hand_owner_context(&mut first, subject);
    apply_exile_subject_hand_owner_context(&mut second, subject);
    let exile = |target| {
        if until_source_leaves {
            EffectAst::subject_verb_exile_until_source_leaves(target, face_down)
        } else {
            EffectAst::subject_verb_exile(target, face_down)
        }
    };
    Ok(Some(EffectAst::Coordinated {
        effects: vec![exile(first), exile(second)],
        leading_duration: false,
        result_conjunction: false,
    }))
}

fn parse_source_and_target_exile_pair(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    until_source_leaves: bool,
    face_down: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = effect_grammar::parse_source_and_target_exile_shape(tokens) else {
        return Ok(None);
    };
    let mut source = parse_target_phrase(shape.source_tokens)?;
    let mut target = parse_target_phrase(shape.target_tokens)?;
    if !matches!(source, TargetAst::Source(_)) {
        return Ok(None);
    }
    apply_exile_subject_hand_owner_context(&mut source, subject.clone());
    apply_exile_subject_hand_owner_context(&mut target, subject);
    let exile = |target| {
        if until_source_leaves {
            EffectAst::subject_verb_exile_until_source_leaves(target, face_down)
        } else {
            EffectAst::subject_verb_exile(target, face_down)
        }
    };
    Ok(Some(EffectAst::Coordinated {
        effects: vec![exile(source), exile(target)],
        leading_duration: false,
        result_conjunction: false,
    }))
}

#[path = "exile_actions/exile_clause_readings.rs"]
mod exile_clause_readings;

pub fn parse_exile(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    if let Some((target_tokens, leave_watcher_tokens)) = split_until_target_leaves_tail(tokens) {
        let (target_tokens, face_down) = split_exile_face_down_suffix(target_tokens);
        let mut target =
            expand_hand_or_graveyard_target(parse_target_phrase(target_tokens)?, target_tokens);
        apply_exile_subject_hand_owner_context(&mut target, subject);
        let leave_watcher = parse_target_phrase(leave_watcher_tokens)?;
        return Ok(EffectAst::subject_verb_exile_until_target_leaves(
            target,
            leave_watcher,
            face_down,
        ));
    }

    if let Some(target_tokens) = split_until_opponent_becomes_monarch_tail(tokens) {
        let (target_tokens, face_down) = split_exile_face_down_suffix(target_tokens);
        let mut target =
            expand_hand_or_graveyard_target(parse_target_phrase(target_tokens)?, target_tokens);
        apply_exile_subject_hand_owner_context(&mut target, subject);
        return Ok(with_exile_actor(
            EffectAst::subject_verb_exile_until_opponent_becomes_monarch(target, face_down),
            subject,
        ));
    }

    let (tokens, until_source_leaves) = split_until_source_leaves_tail(tokens);
    let (tokens, face_down) = split_exile_face_down_suffix(tokens);
    let tokens = split_exile_graveyard_replacement_suffix(tokens);
    let clause_words = crate::lexer::token_word_refs(tokens);
    if grammar::contains_word(tokens, "unless") {
        return Err(CardTextError::ParseError(format!(
            "unsupported exile-unless clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_face_down_manifest_tail = (grammar::contains_word(tokens, "face-down")
        || grammar::contains_word(tokens, "facedown")
        || grammar::contains_word(tokens, "manifest")
        || grammar::contains_word(tokens, "pile"))
        && grammar::contains_word(tokens, "then");
    if has_face_down_manifest_tail {
        return Err(CardTextError::ParseError(format!(
            "unsupported face-down/manifest exile clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_until_total_mana_value = grammar::contains_word(tokens, "until")
        && grammar::contains_word(tokens, "exiled")
        && grammar::contains_word(tokens, "total")
        && grammar::contains_word(tokens, "mana")
        && grammar::contains_word(tokens, "value");
    if has_until_total_mana_value {
        return Err(CardTextError::ParseError(format!(
            "unsupported iterative exile-total clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let input = exile_clause_readings::ExileClause {
        tokens,
        subject,
        until_source_leaves,
        face_down,
        clause_words: &clause_words,
        read_by_cache: Default::default(),
    };
    match exile_clause_readings::read(&input) {
        crate::recognition::ParseOutcome::Match(matched) => return Ok(matched.value.value),
        crate::recognition::ParseOutcome::NoMatch => {}
        crate::recognition::ParseOutcome::Error(diagnostic) => {
            return Err(diagnostic.into_card_text_error());
        }
    }
    if let Some(spec) = split_trailing_if_clause_lexed(tokens) {
        let (target_tokens, source_top_only) = strip_source_top_only_prefix(spec.leading_tokens);
        if source_top_only && until_source_leaves {
            return Err(CardTextError::ParseError(
                "top-of-zone exile-until-source-leaves is not supported".to_string(),
            ));
        }
        let mut target = parse_target_phrase(target_tokens)?;
        apply_exile_subject_hand_owner_context(&mut target, subject);
        let plural_surface = cca_shapes::is_plural_tagged_object_reference(target_tokens);
        return Ok(EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
            predicate: spec.predicate,
            effects: vec![with_exile_actor(
                if until_source_leaves {
                    EffectAst::subject_verb_exile_until_source_leaves(target, face_down)
                } else {
                    EffectAst::subject_verb_exile(target, face_down)
                        .with_source_top_only(source_top_only)
                }
                .with_move_to_zone_plural_surface_if(plural_surface),
                subject,
            )],
        }));
    } else if grammar::contains_word(tokens, "if") {
        return Err(CardTextError::ParseError(format!(
            "unsupported conditional exile clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let (target_tokens, source_top_only) = strip_source_top_only_prefix(tokens);
    if source_top_only && until_source_leaves {
        return Err(CardTextError::ParseError(
            "top-of-zone exile-until-source-leaves is not supported".to_string(),
        ));
    }
    let mut target =
        expand_hand_or_graveyard_target(parse_target_phrase(target_tokens)?, target_tokens);
    apply_exile_subject_hand_owner_context(&mut target, subject);
    let plural_surface = cca_shapes::is_plural_tagged_object_reference(target_tokens);
    Ok(with_exile_actor(
        if until_source_leaves {
            EffectAst::subject_verb_exile_until_source_leaves(target, face_down)
        } else {
            EffectAst::subject_verb_exile(target, face_down).with_source_top_only(source_top_only)
        }
        .with_move_to_zone_plural_surface_if(plural_surface),
        subject,
    ))
}

fn parse_attached_object_exile_bundle(
    tokens: &[OwnedLexToken],
    face_down: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = effect_grammar::parse_attached_object_exile_shape(tokens) else {
        return Ok(None);
    };
    let target = parse_target_phrase(&shape.target)?;
    let mut attachment_filter = parse_object_filter_lexed(&shape.attachment_filter, false)?;
    let antecedent_surface = crate::lexer::token_word_refs(&shape.target)
        .into_iter()
        .rev()
        .find_map(ironsmith_core::DemonstrativeAntecedentSurface::from_noun);
    attachment_filter.set_demonstrative_antecedent_surface(antecedent_surface);
    Ok(Some(EffectAst::subject_verb_exile_all_attached_to(
        attachment_filter,
        target,
        face_down,
    )))
}

pub fn parse_same_name_exile_hand_and_graveyard_clause(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    until_source_leaves: bool,
    face_down: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = effect_grammar::parse_same_name_hand_graveyard_exile_shape(tokens) else {
        return Ok(None);
    };
    let owner_from_subject = match subject {
        Some(SubjectAst::Player(player)) => controller_filter_for_token_player(player),
        _ => None,
    };
    let owner = match shape.owner {
        effect_grammar::SameNameExileOwnerShape::TargetPlayer => {
            Some(PlayerFilter::target_player())
        }
        effect_grammar::SameNameExileOwnerShape::TargetOpponent => {
            Some(PlayerFilter::target_opponent())
        }
        effect_grammar::SameNameExileOwnerShape::ThatPlayer => Some(PlayerFilter::IteratedPlayer),
        effect_grammar::SameNameExileOwnerShape::You => Some(PlayerFilter::You),
        effect_grammar::SameNameExileOwnerShape::TheirOrHisHer => {
            owner_from_subject.or(Some(PlayerFilter::IteratedPlayer))
        }
        effect_grammar::SameNameExileOwnerShape::FromSubject => owner_from_subject,
    };
    let Some(owner) = owner else {
        return Ok(None);
    };

    let mut filter = ObjectFilter::default();
    filter.owner = Some(owner);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
        relation: TaggedOpbjectRelation::SameNameAsTagged,
    });
    filter.any_of = [Zone::Hand, Zone::Graveyard]
        .into_iter()
        .map(|zone| ObjectFilter::default().in_zone(zone))
        .collect();

    Ok(Some(if until_source_leaves {
        EffectAst::subject_verb_exile_until_source_leaves(
            TargetAst::Object(filter, None, None),
            face_down,
        )
    } else {
        EffectAst::subject_verb_exile_all(filter, face_down)
    }))
}

fn strip_exile_list_segment_leading_conjunction(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let trimmed = trim_commas(tokens);
    if trimmed
        .first()
        .is_some_and(|token| token.as_word().is_some_and(|word| word == "and"))
    {
        trim_commas(&trimmed[1..])
    } else {
        trimmed
    }
}

fn split_exile_all_list_tail_segment(tokens: Vec<OwnedLexToken>) -> Vec<Vec<OwnedLexToken>> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut idx = 0usize;
    while idx + 1 < tokens.len() {
        let is_and = tokens[idx].as_word().is_some_and(|word| word == "and");
        let next_starts_all_clause =
            effect_grammar::strip_exile_all_or_each_shape(&tokens[idx + 1..]).is_some();
        if is_and && next_starts_all_clause {
            let part = trim_commas(&tokens[start..idx]);
            if !part.is_empty() {
                parts.push(part);
            }
            start = idx + 1;
            idx += 1;
        }
        idx += 1;
    }
    let part = trim_commas(&tokens[start..]);
    if !part.is_empty() {
        parts.push(part);
    }
    parts
}

fn parse_except_then_additional_exile_all_filter(
    filter_tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    until_source_leaves: bool,
    face_down: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::grammar::primitives as grammar;
    use crate::target::ObjectCharacteristicRelationKind;
    use winnow::Parser as _;

    let segments = grammar::split_lexed_slices_on_comma(filter_tokens);
    if segments.len() < 2 {
        return Ok(None);
    }
    let first = trim_commas(segments[0]);
    let Some((base_tokens, exception_tokens)) =
        grammar::split_lexed_once_on_separator(&first, || grammar::kw("except").void())
    else {
        return Ok(None);
    };
    let base_tokens = trim_commas(base_tokens);
    let mut exception_tokens = trim_commas(exception_tokens);
    if let Some(stripped) = grammar::strip_lexed_prefix_phrase(&exception_tokens, &["for"]) {
        exception_tokens = trim_commas(stripped);
    }
    if base_tokens.is_empty() || exception_tokens.is_empty() {
        return Ok(None);
    }

    let mut additional_segments = Vec::new();
    for segment in segments.iter().skip(1) {
        let segment = strip_exile_list_segment_leading_conjunction(segment);
        for segment in split_exile_all_list_tail_segment(segment) {
            let Some(tokens) = effect_grammar::strip_exile_all_or_each_shape(&segment) else {
                return Ok(None);
            };
            let tokens = trim_commas(tokens);
            if tokens.is_empty() {
                return Ok(None);
            }
            additional_segments.push(tokens);
        }
    }
    if additional_segments.is_empty() {
        return Ok(None);
    }

    let mut base = parse_object_filter_lexed(&base_tokens, false)?;
    let mut exception = parse_object_filter_lexed(&exception_tokens, false)?;
    if exception.characteristic_relations.is_empty() {
        return Ok(None);
    }
    let mut inverse_relations = std::mem::take(&mut exception.characteristic_relations);
    if exception != ObjectFilter::default() {
        return Ok(None);
    }
    for relation in &mut inverse_relations {
        relation.kind = match relation.kind {
            ObjectCharacteristicRelationKind::SharesAny => {
                ObjectCharacteristicRelationKind::SharesNone
            }
            ObjectCharacteristicRelationKind::SharesNone => {
                ObjectCharacteristicRelationKind::SharesAny
            }
        };
    }
    base.characteristic_relations.extend(inverse_relations);
    apply_exile_subject_owner_context(&mut base, subject);

    let mut branches = vec![base];
    for segment in additional_segments {
        let mut filter = parse_object_filter_lexed(&segment, false)?;
        apply_exile_subject_owner_context(&mut filter, subject);
        branches.push(filter);
    }
    let mut union = ObjectFilter::default();
    union.any_of = branches;

    Ok(Some(if until_source_leaves {
        EffectAst::subject_verb_exile_all_until_source_leaves(
            TargetAst::Object(union, None, None),
            face_down,
        )
    } else {
        EffectAst::subject_verb_exile_all(union, face_down)
    }))
}

fn parse_battlefield_graveyard_exile_all_pair(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    until_source_leaves: bool,
    face_down: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let segments = split_exile_all_list_tail_segment(tokens.to_vec());
    let [first, second] = segments.as_slice() else {
        return Ok(None);
    };
    let mut filters = Vec::with_capacity(2);
    for segment in [first, second] {
        let Some(filter_tokens) = effect_grammar::strip_exile_all_or_each_shape(segment) else {
            return Ok(None);
        };
        let mut filter = parse_object_filter_lexed(filter_tokens, false)?;
        apply_exile_subject_owner_context(&mut filter, subject);
        filters.push(filter);
    }

    let is_creature_planeswalker_limit = |filter: &ObjectFilter| {
        filter.card_types.len() == 2
            && crate::slice_primitives::contains(&filter.card_types, &CardType::Creature)
            && crate::slice_primitives::contains(&filter.card_types, &CardType::Planeswalker)
            && filter.mana_value.is_some()
    };
    if !filters.iter().all(is_creature_planeswalker_limit)
        || !matches!(
            (filters[0].zone, filters[1].zone),
            (Some(Zone::Battlefield), Some(Zone::Graveyard))
                | (Some(Zone::Graveyard), Some(Zone::Battlefield))
        )
        || filters[0].mana_value != filters[1].mana_value
    {
        return Ok(None);
    }

    let effects = filters
        .into_iter()
        .map(|filter| {
            if until_source_leaves {
                EffectAst::subject_verb_exile_all_until_source_leaves(
                    TargetAst::Object(filter, None, None),
                    face_down,
                )
            } else {
                EffectAst::subject_verb_exile_all(filter, face_down)
            }
        })
        .collect();
    Ok(Some(EffectAst::Sequence { effects }))
}

fn parse_mixed_target_and_all_exile_list(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    until_source_leaves: bool,
    face_down: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let segments = crate::grammar::primitives::split_lexed_slices_on_comma(tokens);
    if segments.len() < 2 {
        return Ok(None);
    }
    let first_segment = trim_commas(segments[0]);
    if first_segment.is_empty() {
        return Ok(None);
    }
    let mut all_segments = Vec::new();
    for segment in segments.iter().skip(1) {
        let segment = strip_exile_list_segment_leading_conjunction(segment);
        for segment in split_exile_all_list_tail_segment(segment) {
            if segment.is_empty() {
                return Ok(None);
            }
            if effect_grammar::strip_exile_all_or_each_shape(&segment).is_none() {
                return Ok(None);
            }
            all_segments.push(segment);
        }
    }

    let mut effects = Vec::new();
    let mut target = parse_target_phrase(&first_segment)?;
    apply_exile_subject_hand_owner_context(&mut target, subject);
    effects.push(if until_source_leaves {
        EffectAst::subject_verb_exile_until_source_leaves(target, face_down)
    } else {
        EffectAst::subject_verb_exile(target, face_down)
    });

    for segment in all_segments {
        let Some(filter_tokens) = effect_grammar::strip_exile_all_or_each_shape(&segment) else {
            return Ok(None);
        };
        let filter_tokens = trim_commas(filter_tokens);
        if filter_tokens.is_empty() {
            return Ok(None);
        }
        let mut filter = parse_object_filter_lexed(&filter_tokens, false)?;
        apply_exile_subject_owner_context(&mut filter, subject);
        effects.push(if until_source_leaves {
            EffectAst::subject_verb_exile_all_until_source_leaves(
                TargetAst::Object(filter, None, None),
                face_down,
            )
        } else {
            EffectAst::subject_verb_exile_all(filter, face_down)
        });
    }

    Ok(Some(EffectAst::Sequence { effects }))
}

pub fn split_exile_face_down_suffix(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], bool) {
    let shape = effect_grammar::parse_exile_face_down_suffix_shape(tokens);
    (shape.core, shape.face_down)
}

pub fn split_exile_graveyard_replacement_suffix(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    use crate::grammar::primitives as grammar;

    let Some((main_slice, tail_slice)) = grammar::split_lexed_once_on_separator(tokens, || {
        use winnow::Parser as _;
        grammar::kw("instead").void()
    }) else {
        return tokens;
    };
    if main_slice.is_empty() {
        return tokens;
    }

    let is_graveyard_replacement =
        grammar::strip_lexed_prefix_phrase(tail_slice, &["of", "putting"]).is_some()
            && (grammar::contains_word(tail_slice, "graveyard")
                || grammar::contains_word(tail_slice, "graveyards"));
    if is_graveyard_replacement {
        main_slice
    } else {
        tokens
    }
}

pub fn parse_graveyard_owner_prefix_lexed(tokens: &[OwnedLexToken]) -> Option<ParsedOwnerPrefix> {
    effect_grammar::parse_exile_graveyard_owner_shape(tokens)
}

fn parse_exile_dynamic_count_from_top_library_clause(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    face_down: bool,
) -> Option<EffectAst> {
    let default_player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let shape = effect_grammar::parse_exile_dynamic_top_library_shape(tokens, default_player)?;
    let effect_grammar::ExileLibraryPlayerShape::Player(player) = shape.player else {
        return None;
    };
    let tag_tokens = trim_commas(tokens);
    let surface = (default_player != PlayerAst::Implicit && default_player == player)
        .then_some(ironsmith_core::ExileTopLibrarySurface::LibraryOwnerAsActor);

    Some(exile_top_library_effect(
        player,
        shape.count,
        vec![helper_tag_for_tokens(&tag_tokens, "exiled").into()],
        Vec::new(),
        surface,
        face_down || shape.face_down,
    ))
}

fn exile_top_library_effect(
    player: PlayerAst,
    count: Value,
    tags: Vec<TagKey>,
    accumulated_tags: Vec<TagKey>,
    surface: Option<ironsmith_core::ExileTopLibrarySurface>,
    face_down: bool,
) -> EffectAst {
    let mut effect = EffectAst::subject_verb_exile_top_of_library_with_optional_surface(
        player,
        count,
        tags.into_iter().map(crate::tag::TagRef::of).collect(),
        accumulated_tags.into_iter().map(crate::tag::TagRef::of).collect(),
        surface,
    );
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Library(LibraryActionAst::ExileTopOfLibrary {
                face_down: effect_face_down,
                ..
            }),
        ..
    }) = &mut effect
    else {
        unreachable!("exile-top constructor must produce a subject-verb action");
    };
    *effect_face_down = face_down;
    effect
}

pub fn parse_exile_top_library_clause(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    face_down: bool,
) -> Option<EffectAst> {
    let default_player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let shape = effect_grammar::parse_exile_top_library_shape(tokens, default_player)?;
    let tag_tokens = trim_commas(tokens);
    match shape.player {
        effect_grammar::ExileLibraryPlayerShape::EachPlayer => Some(EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: vec![exile_top_library_effect(
                PlayerAst::That,
                shape.count,
                Vec::new(),
                vec![helper_tag_for_tokens(&tag_tokens, "exiled").into()],
                None,
                face_down,
            )],
        })),
        effect_grammar::ExileLibraryPlayerShape::EachOpponent => Some(EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
            effects: vec![exile_top_library_effect(
                PlayerAst::That,
                shape.count,
                Vec::new(),
                vec![helper_tag_for_tokens(&tag_tokens, "exiled").into()],
                None,
                face_down,
            )],
        })),
        effect_grammar::ExileLibraryPlayerShape::Player(player) => {
            let surface = (default_player != PlayerAst::Implicit && default_player == player)
                .then_some(ironsmith_core::ExileTopLibrarySurface::LibraryOwnerAsActor);
            Some(exile_top_library_effect(
                player,
                shape.count,
                vec![helper_tag_for_tokens(&tag_tokens, "exiled").into()],
                Vec::new(),
                surface,
                face_down,
            ))
        }
    }
}

fn parse_exile_bottom_library_clause(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    face_down: bool,
) -> Option<EffectAst> {
    let default_player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let shape = effect_grammar::parse_exile_bottom_library_shape(tokens, default_player)?;
    let tag_tokens = trim_commas(tokens);
    let tag = helper_tag_for_tokens(&tag_tokens, "exiled");
    let mut filter = ObjectFilter::default();
    filter.zone = Some(Zone::Library);

    let choose_and_exile = |player: PlayerAst, tag: TagKey| {
        vec![
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsBottomOfLibrary {
                filter: filter.clone(),
                count: crate::effect::ChoiceCount::exactly(1),
                count_value: None,
                player,
                tag: crate::tag::TagRef::of(tag.clone()),
            }),
            EffectAst::subject_verb_exile(TargetAst::Tagged(crate::tag::TagRef::of(tag), None), face_down),
        ]
    };

    match shape.player {
        effect_grammar::ExileLibraryPlayerShape::EachPlayer => Some(EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: choose_and_exile(PlayerAst::That, tag.key.clone()),
        })),
        effect_grammar::ExileLibraryPlayerShape::EachOpponent => Some(EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
            effects: choose_and_exile(PlayerAst::That, tag.key.clone()),
        })),
        effect_grammar::ExileLibraryPlayerShape::Player(player) => Some(EffectAst::Sequence {
            effects: choose_and_exile(player, tag.key.clone()),
        }),
    }
}

pub fn parse_target_player_graveyard_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let tokens = trim_commas(tokens);
    let owner = parse_graveyard_owner_prefix_lexed(&tokens)?;
    if owner.consumed_words != LexedClause::new(&tokens).word_refs().len() {
        return None;
    }

    let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
    filter.owner = match owner.player {
        PlayerAst::You => Some(PlayerFilter::You),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::Target(Box::new(PlayerFilter::Opponent))),
        PlayerAst::ItsController => Some(PlayerFilter::ControllerOf(
            crate::filter::ObjectRef::tagged(crate::tag::CompilerReferenceTag::Triggering.bind()),
        )),
        PlayerAst::ItsOwner => Some(PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(
            crate::tag::CompilerReferenceTag::Triggering.bind(),
        ))),
        _ => None,
    };
    filter.owner.as_ref()?;
    Some(filter)
}

#[cfg(test)]
mod tests {
    use crate::cards::builders::ZoneMoveActionAst;
    use super::*;
    use crate::Subtype;
    use crate::lexer::lex_line;
    use crate::model::ast::{SubjectVerbActionAst, SubjectVerbEffectAst};

    #[test]
    fn attached_exile_bundle_retains_the_authored_antecedent_noun() {
        let tokens = lex_line("enchanted creature and all Auras attached to it", 0)
            .expect("attached exile bundle should lex");
        let effect = parse_attached_object_exile_bundle(&tokens, false)
            .expect("attached exile bundle should parse")
            .expect("attached exile bundle should be recognized");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAllAttachedTo { filter, .. }),
            ..
        }) = effect
        else {
            panic!("expected a typed attached-object exile bundle");
        };
        assert_eq!(
            filter.demonstrative_antecedent_surface(),
            Some(ironsmith_core::DemonstrativeAntecedentSurface::Creature)
        );
    }

    #[test]
    fn plural_demonstrative_exile_retains_the_referenced_collection() {
        let tokens = lex_line("those Auras", 0).expect("plural demonstrative should lex");
        let effect = parse_exile(&tokens, None).expect("plural demonstrative should parse");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile {
                    target: TargetAst::Object(filter, ..),
                    target_plural_surface,
                    ..
                }),
            ..
        }) = effect
        else {
            panic!("expected a typed object-filter exile");
        };
        assert!(filter.subtypes.contains(&Subtype::Aura));
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
        assert!(filter.has_plural_object_noun_surface());
        assert!(target_plural_surface);
    }

    #[test]
    fn face_down_singular_per_each_exile_keeps_top_order_and_typed_count() {
        let tokens = lex_line(
            "a card from the top of your library face down for each opponent you have",
            0,
        )
        .expect("top-library clause should lex");
        let effect = parse_exile(&tokens, None).expect("top-library clause should parse");

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject:
                crate::model::ast::SubjectVerbSubjectAst {
                    player: PlayerAst::You,
                    ..
                },
            action:
                SubjectVerbActionAst::Library(LibraryActionAst::ExileTopOfLibrary {
                    count, face_down, ..
                }),
            ..
        }) = effect
        else {
            panic!("expected an ordered exile-top action: {effect:#?}");
        };
        assert!(face_down);
        assert_eq!(
            count.unhinted(),
            &Value::CountPlayers(PlayerFilter::Opponent)
        );
        assert!(count.has_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach));
    }

    #[test]
    fn exile_until_distinct_target_leaves_keeps_both_targets_in_order() {
        let tokens = lex_line(
            "target creature or enchantment you don't control until target enchantment you control leaves the battlefield",
            0,
        )
        .expect("clause should lex");
        let effect = parse_exile(&tokens, None).expect("clause should parse");

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileUntilSourceLeaves {
                    target: TargetAst::Object(exiled_filter, Some(_), _),
                    leave_watcher: Some(TargetAst::Object(watcher_filter, Some(_), _)),
                    ..
                }),
            ..
        }) = effect
        else {
            panic!("expected typed exile target and distinct leave watcher");
        };

        assert!(exiled_filter.card_types.contains(&CardType::Creature));
        assert!(exiled_filter.card_types.contains(&CardType::Enchantment));
        assert_eq!(watcher_filter.card_types, vec![CardType::Enchantment]);
        assert_eq!(watcher_filter.controller, Some(PlayerFilter::You));
    }

    #[test]
    fn exile_until_opponent_becomes_monarch_keeps_the_event_duration() {
        let tokens = lex_line(
            "target creature an opponent controls until an opponent becomes the monarch",
            0,
        )
        .expect("clause should lex");
        let effect = parse_exile(&tokens, None).expect("clause should parse");

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileUntilSourceLeaves {
                    target: TargetAst::Object(filter, Some(_), _),
                    duration: ironsmith_core::ExileUntilDuration::OpponentBecomesMonarch,
                    ..
                }),
            ..
        }) = effect
        else {
            panic!("expected a typed targeted exile with a monarch-event duration");
        };

        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.controller, Some(PlayerFilter::Opponent));
    }

    #[test]
    fn exile_one_per_card_type_keeps_owner_and_selection_constraint() {
        let tokens = lex_line(
            "up to one card of each card type from defending player's graveyard",
            0,
        )
        .expect("clause should lex");
        let effect = parse_exile(&tokens, None).expect("clause should parse");

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile {
                    target: TargetAst::WithCount(target, count),
                    ..
                }),
            ..
        }) = effect
        else {
            panic!("expected counted exile");
        };
        let TargetAst::Object(filter, None, None) = target.as_ref() else {
            panic!("expected graveyard object filter");
        };
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert_eq!(filter.owner, Some(PlayerFilter::Defending));
        assert!(filter.one_per_card_type);
        assert_eq!(count, crate::effect::ChoiceCount::any_number());
    }

    #[test]
    fn requantified_bare_card_domains_do_not_inherit_the_battlefield_type_list() {
        let tokens = lex_line(
            "all artifacts, creatures, and lands from the battlefield, all cards from all graveyards, and all cards from all hands",
            0,
        )
        .expect("collection should lex");
        let effect = parse_exile(&tokens, None).expect("collection should parse");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAll { filter, .. }),
            ..
        }) = effect
        else {
            panic!("expected one typed exhaustive exile union: {effect:#?}");
        };

        assert!(filter.card_types.is_empty(), "{filter:#?}");
        assert!(filter.has_conjunctive_set_surface(), "{filter:#?}");
        assert_eq!(filter.any_of.len(), 3, "{filter:#?}");
        let battlefield = filter
            .any_of
            .iter()
            .find(|branch| branch.zone == Some(Zone::Battlefield))
            .expect("battlefield arm");
        assert_eq!(
            battlefield.card_types,
            [CardType::Artifact, CardType::Creature, CardType::Land]
        );
        for zone in [Zone::Graveyard, Zone::Hand] {
            let branch = filter
                .any_of
                .iter()
                .find(|branch| branch.zone == Some(zone))
                .expect("bare card-domain arm");
            assert!(branch.card_types.is_empty(), "{branch:#?}");
        }
    }

    #[test]
    fn explicitly_typed_later_card_domains_are_not_treated_as_bare() {
        let tokens = lex_line(
            "artifacts from the battlefield, all artifact cards from all graveyards",
            0,
        )
        .expect("near miss should lex");
        let mut filter = ObjectFilter {
            card_types: vec![CardType::Artifact],
            any_of: vec![
                ObjectFilter::default().in_zone(Zone::Battlefield),
                ObjectFilter::default().in_zone(Zone::Graveyard),
            ],
            ..ObjectFilter::default()
        };
        filter.set_conjunctive_set_surface(true);
        assert_eq!(
            scope_types_away_from_requantified_bare_card_domains(&tokens, filter.clone()),
            filter
        );
    }
}
