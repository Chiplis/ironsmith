use crate::cards::builders::ForEachEffectAst;
use super::*;

/// Parse the historical block provenance shared by effects of the form
/// "destroy creatures that were blocked by target [blocker] this turn" and
/// then reanimate one creature card from each destroyed creature's historical
/// controller's graveyard. The target, successful destroy result, and block
/// event controller are all represented independently so neither current
/// combat state nor the destroyed object's later controller can stand in for
/// the authored history.
pub fn parse_destroy_historically_blocked_then_reanimate_from_historical_controller(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let first = TokenWordView::new(&first_tokens);
    let first_words = first.word_refs();
    if !crate::word_primitives::parse_sequence_prefix(
        &first_words,
        &["destroy", "all", "creatures"],
    ) || !crate::word_primitives::parse_sequence_suffix(&first_words, &["this", "turn"])
    {
        return Ok(None);
    }
    let Some(blocked_by_idx) = crate::word_primitives::parse_sequence_start(
        &first_words,
        &["that", "were", "blocked", "by"],
    ) else {
        return Ok(None);
    };
    if blocked_by_idx != 3 || first.get(blocked_by_idx + 4) != Some("target") {
        return Ok(None);
    }
    let blocker_end = first_words.len().saturating_sub(2);
    let Some(blocker_range) = first.token_span_for_words(blocked_by_idx + 4, blocker_end) else {
        return Ok(None);
    };
    let blocker_target = match parse_target_phrase(&first_tokens[blocker_range]) {
        Ok(target @ TargetAst::Object(_, Some(_), _)) => target,
        _ => return Ok(None),
    };
    let TargetAst::Object(blocker_filter, _, _) = &blocker_target else {
        return Ok(None);
    };

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let second_words = TokenWordView::new(&second_tokens).word_refs();
    if !crate::word_primitives::parse_any_sequence_complete(
        &second_words,
        &[
            &["they", "cant", "be", "regenerated"],
            &["they", "can't", "be", "regenerated"],
            &["they", "can", "t", "be", "regenerated"],
        ],
    ) {
        return Ok(None);
    }

    let third_tokens = trim_commas(sentences[sentence_idx + 2].lowered());
    let third = TokenWordView::new(&third_tokens);
    const FOLLOWUP_PREFIX: &[&str] = &[
        "for",
        "each",
        "creature",
        "that",
        "died",
        "this",
        "way",
        "put",
        "a",
        "creature",
        "card",
        "from",
        "the",
        "graveyard",
        "of",
        "the",
        "player",
        "who",
        "controlled",
        "that",
        "creature",
        "the",
        "last",
        "time",
        "it",
        "became",
        "blocked",
        "by",
        "that",
    ];
    let third_words = third.word_refs();
    if !crate::word_primitives::parse_sequence_prefix(&third_words, FOLLOWUP_PREFIX) {
        return Ok(None);
    }
    let Some(onto_offset) =
        crate::slice_primitives::select_position(&third_words[FOLLOWUP_PREFIX.len()..], |word| {
            *word == "onto"
        })
    else {
        return Ok(None);
    };
    let onto_idx = FOLLOWUP_PREFIX.len() + onto_offset;
    if onto_idx == FOLLOWUP_PREFIX.len()
        || !third_words.get(onto_idx..).is_some_and(|tail| {
            crate::word_primitives::parse_sequence_prefix(
                tail,
                &["onto", "the", "battlefield", "under", "its"],
            )
        })
        || !third_words.get(onto_idx + 5..).is_some_and(|tail| {
            crate::word_primitives::parse_any_sequence_complete(
                tail,
                &[
                    &["owners", "control"],
                    &["owner's", "control"],
                    &["owner", "s", "control"],
                ],
            )
        })
    {
        return Ok(None);
    }
    let Some(repeated_blocker_range) = third.token_span_for_words(FOLLOWUP_PREFIX.len(), onto_idx)
    else {
        return Ok(None);
    };
    let Ok(repeated_blocker_filter) =
        parse_object_filter_lexed(&third_tokens[repeated_blocker_range], false)
    else {
        return Ok(None);
    };
    if repeated_blocker_filter != *blocker_filter {
        return Ok(None);
    }

    let blocker_tag = helper_tag_for_tokens(&first_tokens, "historical_blocker");
    let destroyed_tag = helper_tag_for_tokens(&first_tokens, "destroyed");
    let target_blocker = EffectAst::TagAffected {
        effect: Box::new(EffectAst::subject_verb_explicit_target_only(blocker_target)),
        tag: crate::tag::TagRef::of(blocker_tag.clone()),
    };

    let mut destroyed_filter = ObjectFilter::creature();
    destroyed_filter.blocked_by = Some(ObjectRef::Tagged(blocker_tag.clone().into()));
    let destroy = EffectAst::TagAffected {
        effect: Box::new(EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll {
                filter: destroyed_filter,
                no_regeneration: true,
                creature_destroyed_this_way_surface: false,
            }),
        )),
        tag: crate::tag::TagRef::of(destroyed_tag.clone()),
    };

    let mut creature_card = ObjectFilter::creature();
    creature_card.zone = Some(Zone::Graveyard);
    creature_card.owner = Some(PlayerFilter::IteratedPlayer);
    creature_card.set_explicit_card_noun(true);
    let reanimate_one = EffectAst::subject_verb_move_to_zone(
        TargetAst::WithCount(
            Box::new(TargetAst::Object(creature_card, None, None)),
            ChoiceCount::exactly(1),
        ),
        Zone::Battlefield,
        false,
        ReturnControllerAst::Owner,
        false,
        None,
    );
    let followup = EffectAst::ForEach(ForEachEffectAst::ForEachTaggedWithControllerAtLastBlockedBy {
        tag: crate::tag::TagRef::of(destroyed_tag),
        blocker_tag: crate::tag::TagRef::of(blocker_tag),
        effects: vec![reanimate_one],
    });

    Ok(Some(vec![target_blocker, destroy, followup]))
}
