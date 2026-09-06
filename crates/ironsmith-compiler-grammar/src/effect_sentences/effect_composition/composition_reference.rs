use crate::cards::builders::ForEachEffectAst;
use super::*;

pub(super) fn parse_each_player_hand_exile_play_constraints_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_each_player_hand_exile_play_constraints_tokens(tokens)?;
    let exiled_tag = helper_tag_for_tokens(tokens, "each_player_hand_exiled");
    let mut hand_card = ObjectFilter::default();
    hand_card.zone = Some(Zone::Hand);
    hand_card.owner = Some(PlayerFilter::IteratedPlayer);

    Some(vec![
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
            filter: shape.players,
            effects: vec![
                EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                    filter: hand_card,
                    count: ChoiceCount::exactly(1),
                    count_value: None,
                    player: PlayerAst::That,
                    tag: crate::tag::TagRef::of(exiled_tag.clone()),
                }),
                EffectAst::subject_verb_exile(TargetAst::Tagged(crate::tag::TagRef::of(exiled_tag.clone()), None), false),
            ],
        }),
        EffectAst::subject_verb_grant_play_tagged_with_play_constraints(
            crate::tag::TagRef::of(exiled_tag),
            PlayerAst::ItsOwner,
            Some(shape.additional_cost),
            shape.lands_enter_tapped,
        ),
    ])
}

pub(super) fn parse_tap_controlled_objects_then_empty_mana_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_tap_controlled_objects_then_empty_mana_tokens(tokens)?;
    Some(vec![
        EffectAst::subject_verb_target_only(TargetAst::Player(
            PlayerFilter::Any,
            span_from_tokens(tokens),
        )),
        EffectAst::subject_verb_tap_all(shape.filter),
        EffectAst::subject_verb_empty_mana_pool(PlayerAst::Target),
    ])
}

/// Parse a linked-duration sequence such as
/// "untap all creatures, then those creatures phase out until this enchantment
/// leaves the battlefield." The repeated filter is semantic identity; the
/// printed "those" does not depend on whether untapping changed each object.
pub(super) fn parse_untap_then_phase_out_until_source_leaves_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    use crate::grammar::primitives as grammar;

    let surface_words = words(tokens);
    if !crate::word_primitives::first_is(&surface_words, "untap")
        || !crate::word_primitives::sequence_occurs(
            &surface_words,
            &["phase", "out", "until", "this"],
        )
        || !crate::word_primitives::parse_sequence_suffix(
            &surface_words,
            &["leaves", "the", "battlefield"],
        )
    {
        return None;
    }
    let (untap_tokens, phase_tokens) =
        grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void())?;
    let untap_effects = crate::grammar::primitives::probe_shape(
        effect_sentences::parse_effect_sentence_lexed(&trim_commas(untap_tokens)),
    )?;
    let [untap_effect] = untap_effects.as_slice() else {
        return None;
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::PermanentState(PermanentStateActionAst::UntapAll { filter }),
        ..
    }) = untap_effect
    else {
        return None;
    };

    let trimmed_phase_tokens = trim_commas(phase_tokens);
    let phase_words = words(&trimmed_phase_tokens);
    let phase_idx = crate::word_primitives::parse_sequence_start(
        &phase_words,
        &["phase", "out", "until", "this"],
    )?;
    if phase_idx < 2
        || !crate::word_primitives::first_is(&phase_words, "those")
        || phase_words.len() < phase_idx + 7
        || !crate::word_primitives::parse_sequence_suffix(
            &phase_words,
            &["leaves", "the", "battlefield"],
        )
    {
        return None;
    }
    let source_words = &phase_words[phase_idx + 3..phase_words.len() - 3];
    if source_words.len() < 2 || !crate::word_primitives::first_is(source_words, "this") {
        return None;
    }
    let source_surface = SourceReferenceSurface::ThisPermanentType(source_words.join(" "));

    Some(vec![
        untap_effect.clone(),
        EffectAst::subject_verb_phase_out_all_until_source_leaves(filter.clone(), source_surface),
    ])
}
