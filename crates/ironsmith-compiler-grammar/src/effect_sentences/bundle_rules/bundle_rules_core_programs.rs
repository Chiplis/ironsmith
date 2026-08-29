use super::*;

pub fn parse_typed_effect_bundle_lexed(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(tokens);
    if sentences.len() == 2 {
        let sentence_inputs = sentences
            .iter()
            .map(|sentence| SentenceInput::from_lexed(sentence))
            .collect::<Vec<_>>();
        if let Ok(Some(effects)) = super::super::sequence_rules::generic_subject_verb_sequences::exile_permission_followups::parse_dynamic_exile_top_then_play_for_as_long_as_exiled(
            &sentence_inputs,
            0,
        ) {
            return Some(effects);
        }
    }
    if sentences.len() == 2
        && bundle_grammar::is_resolving_card_exile_then_return_next_end_step_shape(
            sentences[0],
            sentences[1],
        )
    {
        return Some(vec![
            EffectAst::subject_verb_register_zone_replacement_with_linked_exile_follow_up(
                TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.key(), None),
                Some(Zone::Stack),
                Some(Zone::Graveyard),
                Zone::Exile,
                ZoneReplacementDurationAst::OneShot,
                ironsmith_core::LinkedExileFollowUp::ReturnToHandAtNextEndStep,
            ),
        ]);
    }
    // A consult procedure nested under "for each of" belongs to the declared
    // mixed target collection. Claim that typed declaration/iteration shape
    // before the broad consult-disposition recognizer can start at the inner
    // reveal clause and discard the outer target declaration.
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_mixed_targets_then_for_each_bundle(sentences[0], sentences[1], None)
    {
        return Some(effects);
    }
    if let Some(effects) = parse_untap_then_phase_out_until_source_leaves_bundle(tokens) {
        return Some(effects);
    }
    if let Ok(Some(effects)) = parse_inline_look_exile_face_down_permission_bundle(tokens) {
        return Some(effects);
    }
    if let Ok(Some(effects)) = parse_inline_exile_top_then_put_from_among_bundle(tokens) {
        return Some(effects);
    }
    if let Ok(Some(effects)) = parse_inline_mill_then_put_from_among_bundle(tokens) {
        return Some(effects);
    }
    if let Ok(Some(effects)) = parse_hidden_exile_partition_permission_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_discard_redraw_mana_value_ladder_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_energy_pay_any_destroy_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_consult_disposition_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_reveal_repeated_disposition_bundle(tokens) {
        return Some(effects);
    }
    if let Ok(Some(effects)) = parse_reveal_from_outside_game_to_hand(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_each_player_hand_exile_play_constraints_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_look_hand_optional_exile_play_tax_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_persistent_exile_play_tax_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_controller_sacrifice_consult_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_each_player_shuffle_then_consult_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_proliferate_choose_phase_out_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_tap_controlled_objects_then_empty_mana_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_reveal_until_land_put_all_graveyard_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_bid_life_for_control_bundle(tokens) {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_exile_collection_each_upkeep_return_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_each_graveyard_then_owner_shuffle_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Some(mut effects) =
            parse_untap_then_phase_out_until_source_leaves_bundle(sentences[0])
        && let Ok(mut follow_up) = effect_sentences::parse_effect_sentence_lexed(sentences[1])
    {
        effects.append(&mut follow_up);
        return Some(effects);
    }
    if sentences.len() == 2
        && let Some(effects) =
            parse_regenerate_then_gain_control_if_regenerates_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) = parse_consult_then_put_matches_battlefield_rest_bottom_bundle(
            sentences[0],
            sentences[1],
        )
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_exile_then_source_leaves_return_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_exile_top_library_then_play_bundle(sentences[0], sentences[1], None)
    {
        return Some(effects);
    }
    if sentences.len() == 3
        && let Ok(Some(effects)) = parse_optional_result_exile_choice_play_bundle(&sentences)
    {
        return Some(effects);
    }
    if sentences.len() == 3
        && matches!(
            words(sentences[1]).as_slice(),
            ["choose", "one", "of", "them"]
                | ["you", "choose", "one", "of", "them"]
                | ["choose", "one", "of", "those", "cards"]
                | ["you", "choose", "one", "of", "those", "cards"]
        )
        && let Ok(Some(effects)) =
            parse_exile_top_library_then_play_bundle(sentences[0], sentences[1], Some(sentences[2]))
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Some(effects) =
            parse_may_cast_spell_for_alternative_cost_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_type_then_phase_out_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) = parse_reveal_from_outside_game_or_choose_face_up_exile_to_hand(
            sentences[0],
            sentences[1],
        )
    {
        return Some(effects);
    }
    if sentences.len() == 3
        && let Ok(Some(effects)) = parse_selected_hand_double_choice_discard_bundle(&sentences)
    {
        return Some(effects);
    }
    if sentences.len() == 3
        && let Ok(Some(effects)) = parse_discard_reveal_choose_discard_chosen_bundle(&sentences)
    {
        return Some(effects);
    }
    if sentences.len() == 3
        && let Ok(Some(effects)) = parse_choose_mixed_targets_then_for_each_bundle(
            sentences[0],
            sentences[1],
            Some(sentences[2]),
        )
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_mixed_targets_then_for_each_bundle(sentences[0], sentences[1], None)
    {
        return Some(effects);
    }
    if sentences.len() == 3
        && let Ok(Some(effects)) = parse_choose_objects_then_for_each_of_those_bundle(
            sentences[0],
            sentences[1],
            Some(sentences[2]),
        )
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_objects_then_for_each_of_those_bundle(sentences[0], sentences[1], None)
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_counter_on_target_then_put_or_remove_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_counter_on_target_then_put_additional_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            effect_sentences::parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
                sentences[0],
                sentences[1],
            )
    {
        return Some(effects);
    }
    if sentences.len() == 3
        && {
            let first_words = crate::lexer::token_word_refs(sentences[0]);
            let choice_words = if first_words.first().copied() == Some("you") {
                &first_words[1..]
            } else {
                &first_words[..]
            };
            matches!(
                parse_choose_card_type_phrase_words(choice_words),
                Ok(Some((consumed, _))) if consumed == choice_words.len()
            )
        }
        && let Ok(Some(mut effects)) =
            parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard(
                sentences[1],
                sentences[2],
            )
    {
        let first_words = crate::lexer::token_word_refs(sentences[0]);
        let choice_words = if first_words.first().copied() == Some("you") {
            &first_words[1..]
        } else {
            &first_words[..]
        };
        let (_, options) = parse_choose_card_type_phrase_words(choice_words)
            .ok()
            .flatten()
            .expect("validated choose-card-type bundle prefix");
        let mut combined = vec![EffectAst::subject_verb_choose_card_type(
            PlayerAst::You,
            options,
        )];
        combined.append(&mut effects);
        return Some(combined);
    }
    if let Some(effects) = parse_kicked_counter_mana_value_replacement_bundle(tokens) {
        return Some(effects);
    }
    if let Ok(Some(effects)) = parse_search_library_slots_to_hand_bundle(tokens) {
        return Some(effects);
    }
    None
}
