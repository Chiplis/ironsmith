use super::*;

/// Render a procedure where a named player chooses an opponent who controls
/// more matching permanents than they do, then may perform a reveal-until
/// consult with the matching card kind. The typed chooser, relative-player
/// filter, and consult tags prove the full relationship; no card identity or
/// source text participates in the match.
pub(in crate::compiled_text) fn describe_relative_player_target_then_optional_consult(
    effects: &[Effect],
) -> Option<String> {
    let [target_effect, may_effect] = effects else {
        return None;
    };

    let target_only = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if !target_only.explicit_declaration
        || target_only.chooser.as_ref() != Some(&PlayerFilter::Active)
        || !target_only.target.is_target()
        || target_only.target.count() != crate::effect::ChoiceCount::exactly(1)
    {
        return None;
    }
    let ChooseSpec::Player(PlayerFilter::OpponentWithMoreControlledObjectsThan {
        player: reference_player,
        filter: controlled_filter,
    }) = target_only.target.base()
    else {
        return None;
    };
    if reference_player.as_ref() != &PlayerFilter::Active {
        return None;
    }

    let may = structural_unwrap_render_wrappers(may_effect)
        .downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider.as_ref() != Some(&PlayerFilter::Active) {
        return None;
    }
    let consult_effects = may
        .effects
        .iter()
        .map(structural_unwrap_render_wrappers)
        .collect::<Vec<_>>();
    // Reuse the generic consult recognizer to prove that the match tag moves
    // to the battlefield and every other revealed card moves to the graveyard.
    render_consult_reveal_put_battlefield_rest_graveyard(&consult_effects)?;

    let consult = consult_effects[0].downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.player != PlayerFilter::Active
        || consult.max_exposed.is_some()
        || !matches!(
            consult.stop_rule,
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
        )
    {
        return None;
    }

    let mut controlled_card_filter = controlled_filter.as_ref().clone();
    controlled_card_filter.zone = None;
    if controlled_card_filter != consult.filter {
        return None;
    }

    let controlled_nouns = pluralize_noun_phrase(&controlled_filter.description());
    let selection = describe_library_consult_selection_with_cards(&consult.filter);
    Some(format!(
        "That player chooses target player who controls more {controlled_nouns} than they do and is their opponent. The first player may reveal cards from the top of their library until they reveal {selection}. If the first player does, that player puts that card onto the battlefield and all other cards revealed this way into their graveyard"
    ))
}

/// Render the analogous relative-player target followed by an optional
/// library search. The active/upkeep player is both the target chooser and
/// the owner/actor of the complete search procedure.
pub(in crate::compiled_text) fn describe_relative_player_target_then_optional_search(
    effects: &[Effect],
) -> Option<String> {
    let [target_effect, may_effect] = effects else {
        return None;
    };

    let target_only = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if !target_only.explicit_declaration
        || target_only.chooser.as_ref() != Some(&PlayerFilter::Active)
        || !target_only.target.is_target()
        || target_only.target.count() != crate::effect::ChoiceCount::exactly(1)
    {
        return None;
    }
    let ChooseSpec::Player(PlayerFilter::OpponentWithMoreControlledObjectsThan {
        player: reference_player,
        filter: controlled_filter,
    }) = target_only.target.base()
    else {
        return None;
    };
    if reference_player.as_ref() != &PlayerFilter::Active {
        return None;
    }

    let may = structural_unwrap_render_wrappers(may_effect)
        .downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider.as_ref() != Some(&PlayerFilter::Active) {
        return None;
    }
    let [sequence_effect] = may.effects.as_slice() else {
        return None;
    };
    let sequence = structural_unwrap_render_wrappers(sequence_effect)
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    let [choose_effect, for_each_effect, shuffle_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.is_search
        || choose.reveal
        || choose.chooser != PlayerFilter::Active
        || choose.zone != Some(Zone::Library)
        || !choose.additional_zones.is_empty()
        || choose.count != crate::effect::ChoiceCount::exactly(1)
        || choose.filter.owner.as_ref() != Some(&PlayerFilter::Active)
    {
        return None;
    }
    let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [put_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let put = put_effect.downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()?;
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if for_each.tag != choose.tag
        || !matches!(put.target.base(), ChooseSpec::Iterated)
        || put.tapped
        || put.controller != PlayerFilter::Active
        || shuffle.player != PlayerFilter::Active
        || shuffle.target_spec.is_some()
    {
        return None;
    }

    let controlled_nouns = pluralize_noun_phrase(&controlled_filter.description());
    let mut selection_filter = choose.filter.clone();
    selection_filter.zone = None;
    selection_filter.owner = None;
    let selection = with_indefinite_article(strip_leading_article(&selection_filter.description()));
    Some(format!(
        "That player chooses target player who controls more {controlled_nouns} than they do and is their opponent. The first player may search their library for {selection}, put that card onto the battlefield, then shuffle"
    ))
}
