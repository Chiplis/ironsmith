use super::*;

struct CompleteConsultDamageProcedure<'a> {
    consult: &'a crate::effects::ConsultTopOfLibraryEffect,
    remainder: &'a crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
}

fn authored_consult_damage_effects(effects: &[Effect]) -> Option<(&Effect, &Effect, &Effect)> {
    if let [consult, damage, remainder] = effects {
        return Some((consult, damage, remainder));
    }

    // Same-sentence comma-then lowering preserves the authored grouping:
    //
    //   Sequence(consult, Sequence(damage, remainder))
    //
    // Accept that exact structure while refusing unrelated sequence surfaces
    // or extra actions.
    let [outer_effect] = effects else {
        return None;
    };
    let outer = structural_unwrap_render_wrappers(outer_effect)
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    if outer.surface != ironsmith_core::SequenceSurface::CommaThen {
        return None;
    }
    if let [consult, damage, remainder] = outer.effects.as_slice() {
        return Some((consult, damage, remainder));
    }
    let [consult, tail_effect] = outer.effects.as_slice() else {
        return None;
    };
    let tail = structural_unwrap_render_wrappers(tail_effect)
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    if tail.surface != ironsmith_core::SequenceSurface::CommaThen {
        return None;
    }
    let [damage, remainder] = tail.effects.as_slice() else {
        return None;
    };
    Some((consult, damage, remainder))
}

fn complete_consult_damage_procedure<'a>(
    effects: &'a [Effect],
    damage_targets_current_member: impl FnOnce(&ChooseSpec) -> bool,
) -> Option<CompleteConsultDamageProcedure<'a>> {
    let (consult_effect, damage_effect, remainder_effect) =
        authored_consult_damage_effects(effects)?;
    let consult = structural_unwrap_render_wrappers(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    let damage = structural_unwrap_render_wrappers(damage_effect)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    let remainder = structural_unwrap_render_wrappers(remainder_effect)
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;

    if consult.player != PlayerFilter::You
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || consult.max_exposed.is_some()
        || !matches!(
            consult.stop_rule,
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
                | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1))
        )
        || damage.source_is_combat
        || damage.unpreventable
        || !damage_targets_current_member(&damage.target)
        || !matches!(
            damage.amount.unhinted(),
            Value::ManaValueOf(spec)
                if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag == &consult.match_tag)
        )
        || remainder.tag != consult.all_tag
        || remainder.keep_tagged.is_some()
        || remainder.player != consult.player
        || remainder.surface != ironsmith_core::LibraryRemainderSurface::Rest
    {
        return None;
    }

    Some(CompleteConsultDamageProcedure { consult, remainder })
}

/// Rejoin the executable split for one authored mixed collection.
///
/// Player and planeswalker targets require different runtime iterators, but a
/// shared target declaration, matching object tag, and two complete equivalent
/// procedures prove Oracle's single “for each of them” instruction.
pub(in crate::compiled_text) fn describe_mixed_target_collection_consult_damage(
    effects: &[Effect],
) -> Option<String> {
    let [declaration_effect, player_loop_effect, object_loop_effect] = effects else {
        return None;
    };
    let declaration_tag = if let Some(declaration) =
        declaration_effect.downcast_ref::<crate::effects::TagAllEffect>()
    {
        &declaration.tag
    } else if let Some(declaration) =
        declaration_effect.downcast_ref::<crate::effects::TaggedEffect>()
    {
        &declaration.tag
    } else {
        return None;
    };
    let target_only = structural_unwrap_render_wrappers(declaration_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let player_loop = structural_unwrap_render_wrappers(player_loop_effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let object_loop = structural_unwrap_render_wrappers(object_loop_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;

    if !target_only.explicit_declaration
        || target_only.chooser.is_some()
        || target_only.target.count() != crate::effect::ChoiceCount::any_number()
        || !matches!(
            target_only.target.base(),
            ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any)
        )
        || player_loop.filter != PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any))
        || player_loop.starting_with_controller
        || player_loop.stop_after_first_happened
        || &object_loop.tag != declaration_tag
    {
        return None;
    }

    let player_procedure = complete_consult_damage_procedure(&player_loop.effects, |target| {
        matches!(
            target.base(),
            ChooseSpec::Player(PlayerFilter::IteratedPlayer)
        )
    })?;
    let object_procedure = complete_consult_damage_procedure(&object_loop.effects, |target| {
        matches!(target.base(), ChooseSpec::Iterated)
    })?;
    if player_procedure.consult != object_procedure.consult
        || player_procedure.remainder != object_procedure.remainder
        || player_procedure.remainder.order
            != crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses
    {
        return None;
    }

    let selection = describe_library_consult_selection_with_cards(&player_procedure.consult.filter);
    Some(format!(
        "Choose any number of target players or planeswalkers. For each of them, reveal cards from the top of your library until you reveal {selection}, this spell deals damage equal to that card's mana value to that player or planeswalker, then you put the revealed cards on the bottom of your library in any order"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn procedure(all_tag: &TagKey, match_tag: &TagKey, damage_target: ChooseSpec) -> Vec<Effect> {
        let consult = Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
            PlayerFilter::You,
            crate::effects::consult_helpers::LibraryConsultMode::Reveal,
            ObjectFilter::default().without_type(CardType::Land),
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch,
            all_tag.clone(),
            match_tag.clone(),
        ));
        let damage = Effect::new(crate::effects::DealDamageEffect::new(
            Value::ManaValueOf(Box::new(ChooseSpec::Tagged(match_tag.clone()))),
            damage_target,
        ));
        let remainder = Effect::new(
            crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
                all_tag.clone(),
                None,
                crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses,
                PlayerFilter::You,
            ),
        );
        vec![Effect::new(crate::effects::SequenceEffect::comma_then(
            vec![
                consult,
                Effect::new(crate::effects::SequenceEffect::comma_then(vec![
                    damage, remainder,
                ])),
            ],
        ))]
    }

    fn declaration(object_targets: &TagKey) -> Effect {
        Effect::new(crate::effects::TaggedEffect::new(
            object_targets.clone(),
            Effect::new(crate::effects::TargetOnlyEffect::explicit(
                ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any)
                    .with_count(crate::effect::ChoiceCount::any_number()),
            )),
        ))
    }

    fn player_loop(all_tag: &TagKey, match_tag: &TagKey) -> Effect {
        Effect::new(crate::effects::ForPlayersEffect::new(
            PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any)),
            procedure(
                all_tag,
                match_tag,
                ChooseSpec::Player(PlayerFilter::IteratedPlayer),
            ),
        ))
    }

    fn object_loop(
        object_targets: &TagKey,
        all_tag: &TagKey,
        match_tag: &TagKey,
        damage_target: ChooseSpec,
    ) -> Effect {
        Effect::new(crate::effects::ForEachTaggedEffect::new(
            object_targets.clone(),
            procedure(all_tag, match_tag, damage_target),
        ))
    }

    #[test]
    fn rejoins_disjoint_player_and_planeswalker_iterations() {
        let object_targets = TagKey::from("chosen_target_objects");
        let all_tag = TagKey::from("revealed");
        let match_tag = TagKey::from("matched");
        let declaration = declaration(&object_targets);
        let player_loop = player_loop(&all_tag, &match_tag);
        let object_loop = object_loop(&object_targets, &all_tag, &match_tag, ChooseSpec::Iterated);

        assert_eq!(
            describe_mixed_target_collection_consult_damage(&[
                declaration,
                player_loop,
                object_loop,
            ])
            .as_deref(),
            Some(
                "Choose any number of target players or planeswalkers. For each of them, reveal cards from the top of your library until you reveal a nonland card, this spell deals damage equal to that card's mana value to that player or planeswalker, then you put the revealed cards on the bottom of your library in any order"
            )
        );
    }

    #[test]
    fn refuses_a_planeswalker_loop_that_damages_a_player_instead() {
        let object_targets = TagKey::from("chosen_target_objects");
        let all_tag = TagKey::from("revealed");
        let match_tag = TagKey::from("matched");
        let effects = [
            declaration(&object_targets),
            player_loop(&all_tag, &match_tag),
            object_loop(
                &object_targets,
                &all_tag,
                &match_tag,
                ChooseSpec::Player(PlayerFilter::IteratedPlayer),
            ),
        ];

        assert!(describe_mixed_target_collection_consult_damage(&effects).is_none());
    }

    #[test]
    fn refuses_player_and_planeswalker_loops_with_different_result_tags() {
        let object_targets = TagKey::from("chosen_target_objects");
        let all_tag = TagKey::from("revealed");
        let match_tag = TagKey::from("matched");
        let other_match_tag = TagKey::from("different_match");
        let effects = [
            declaration(&object_targets),
            player_loop(&all_tag, &match_tag),
            object_loop(
                &object_targets,
                &all_tag,
                &other_match_tag,
                ChooseSpec::Iterated,
            ),
        ];

        assert!(describe_mixed_target_collection_consult_damage(&effects).is_none());
    }
}
