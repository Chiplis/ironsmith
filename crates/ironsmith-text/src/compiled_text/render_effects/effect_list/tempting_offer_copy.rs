use super::*;

fn wrapper_contains_tag(effect: &Effect, expected: &TagKey) -> bool {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return tagged.tag == *expected || wrapper_contains_tag(&tagged.effect, expected);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return tag_all.tag == *expected || wrapper_contains_tag(&tag_all.effect, expected);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return wrapper_contains_tag(&with_id.effect, expected);
    }
    false
}

fn copy_with_id(
    effect: &Effect,
) -> Option<(
    &crate::effects::WithIdEffect,
    &crate::effects::CopySpellEffect,
)> {
    let with_id = wrapped_with_id(effect)?;
    let copy = with_id
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()?;
    Some((with_id, copy))
}

fn exact_retarget(
    effect: &Effect,
    copy_id: crate::effect::EffectId,
    chooser: PlayerFilter,
) -> bool {
    unwrap_basic_tag_wrappers(effect)
        .downcast_ref::<crate::effects::ChooseNewTargetsEffect>()
        .is_some_and(|retarget| {
            retarget.from_effect == copy_id
                && retarget.may
                && retarget.chooser == Some(chooser)
                && !retarget.single_target_surface
        })
}

fn exact_chosen_stack_spell(effect: &Effect, chosen_tag: &TagKey) -> bool {
    let Some(target_only) =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::TargetOnlyEffect>()
    else {
        return false;
    };
    let expected_filter = ObjectFilter {
        zone: Some(Zone::Stack),
        card_types: vec![CardType::Instant, CardType::Sorcery],
        has_mana_cost: true,
        ..Default::default()
    };
    target_only.explicit_declaration
        && target_only.chooser.is_none()
        && matches!(
            target_only.target.unhinted(),
            ChooseSpec::Target(inner)
                if matches!(inner.unhinted(), ChooseSpec::Object(filter) if filter == &expected_filter)
        )
        && wrapper_contains_tag(effect, chosen_tag)
}

/// Render the typed four-part tempting-offer copy procedure. The authored
/// copy-count surface is the provenance for the ability word; every executable
/// link remains independently guarded: the one chosen stack spell, each
/// opponent's optional single copy and retarget permission, the loop outcome
/// metric, the controller's dynamic copies, and that copy set's retarget
/// permission.
pub(super) fn describe_tempting_offer_copy_spell_bundle(effects: &[Effect]) -> Option<String> {
    let [
        choose_spell,
        opponent_loop_effect,
        your_copy_effect,
        your_retarget_effect,
    ] = effects
    else {
        return None;
    };

    let opponent_loop_with_id = wrapped_with_id(opponent_loop_effect)?;
    let opponent_loop = opponent_loop_with_id
        .effect
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if opponent_loop.filter != PlayerFilter::Opponent
        || opponent_loop.starting_with_controller
        || opponent_loop.stop_after_first_happened
    {
        return None;
    }
    let [opponent_may_effect] = opponent_loop.effects.as_slice() else {
        return None;
    };
    let opponent_may = opponent_may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if opponent_may.decider != Some(PlayerFilter::IteratedPlayer)
        || opponent_may.fallback != crate::decision::FallbackStrategy::Decline
    {
        return None;
    }
    let [opponent_copy_effect, opponent_retarget_effect] = opponent_may.effects.as_slice() else {
        return None;
    };
    let (opponent_copy_with_id, opponent_copy) = copy_with_id(opponent_copy_effect)?;

    let (your_copy_with_id, your_copy) = copy_with_id(your_copy_effect)?;
    let chosen_tag = match your_copy.target.unhinted() {
        ChooseSpec::Tagged(tag) => tag,
        _ => return None,
    };
    if !exact_chosen_stack_spell(choose_spell, chosen_tag)
        || opponent_copy.target.unhinted() != your_copy.target.unhinted()
        || opponent_copy.count != Value::Fixed(1)
        || opponent_copy.count_surface.is_some()
        || opponent_copy.copier != PlayerFilter::IteratedPlayer
        || !opponent_copy.removed_supertypes.is_empty()
        || opponent_copy.has_characteristic_modifiers()
        || opponent_copy.target_reference_pronoun
        || !exact_retarget(
            opponent_retarget_effect,
            opponent_copy_with_id.id,
            PlayerFilter::IteratedPlayer,
        )
        || your_copy.copier != PlayerFilter::You
        || !your_copy.removed_supertypes.is_empty()
        || your_copy.has_characteristic_modifiers()
        || your_copy.target_reference_pronoun
        || your_copy.count_surface
            != Some(
                ironsmith_core::effect::CopyCountSurface::OncePlusAdditionalPerOpponentWhoCopiedThisWay,
            )
        || !matches!(
            your_copy.count.unhinted(),
            Value::EffectMetricOffset {
                effect_id,
                source: ironsmith_core::EffectMetricSource::Outcome,
                metric: ironsmith_core::EffectMetric::PlayersWithPositiveCount,
                offset: 1,
            } if effect_id == &opponent_loop_with_id.id
        )
        || !exact_retarget(
            your_retarget_effect,
            your_copy_with_id.id,
            PlayerFilter::You,
        )
    {
        return None;
    }

    Some(
        "Tempting offer — Choose target instant or sorcery spell. Each opponent may copy that spell and may choose new targets for the copy they control. You copy that spell once plus an additional time for each opponent who copied the spell this way. You may choose new targets for the copies you control."
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORACLE: &str = "Tempting offer — Choose target instant or sorcery spell. Each opponent may copy that spell and may choose new targets for the copy they control. You copy that spell once plus an additional time for each opponent who copied the spell this way. You may choose new targets for the copies you control.";

    fn copy_shape() -> Vec<Effect> {
        let chosen = TagKey::from("chosen_spell");
        let filter = ObjectFilter {
            zone: Some(Zone::Stack),
            card_types: vec![CardType::Instant, CardType::Sorcery],
            has_mana_cost: true,
            ..Default::default()
        };
        let target = Effect::new(crate::effects::TargetOnlyEffect::explicit(
            ChooseSpec::target(ChooseSpec::Object(filter)),
        ))
        .tag(chosen.clone());

        let opponent_copy_id = crate::effect::EffectId(2);
        let opponent_copy = Effect::with_id(
            opponent_copy_id.0,
            Effect::new(crate::effects::CopySpellEffect::new_for_player(
                ChooseSpec::Tagged(chosen.clone()),
                Value::Fixed(1),
                PlayerFilter::IteratedPlayer,
            )),
        )
        .tag(TagKey::from("__copied_stack_object__"));
        let opponent_retarget =
            Effect::new(crate::effects::ChooseNewTargetsEffect::may_for_player(
                opponent_copy_id,
                PlayerFilter::IteratedPlayer,
            ));
        let opponent_loop_id = crate::effect::EffectId(1);
        let opponent_loop = Effect::with_id(
            opponent_loop_id.0,
            Effect::for_each_opponent(vec![Effect::may_player(
                PlayerFilter::IteratedPlayer,
                vec![opponent_copy, opponent_retarget],
            )]),
        );

        let your_copy_id = crate::effect::EffectId(3);
        let your_copy = Effect::with_id(
            your_copy_id.0,
            Effect::new(
                crate::effects::CopySpellEffect::new_for_player(
                    ChooseSpec::Tagged(chosen),
                    Value::EffectMetricOffset {
                        effect_id: opponent_loop_id,
                        source: ironsmith_core::EffectMetricSource::Outcome,
                        metric: ironsmith_core::EffectMetric::PlayersWithPositiveCount,
                        offset: 1,
                    },
                    PlayerFilter::You,
                )
                .with_count_surface(
                    ironsmith_core::effect::CopyCountSurface::OncePlusAdditionalPerOpponentWhoCopiedThisWay,
                ),
            ),
        )
        .tag(TagKey::from("__copied_stack_object__"));
        let your_retarget = Effect::new(crate::effects::ChooseNewTargetsEffect::may_for_player(
            your_copy_id,
            PlayerFilter::You,
        ));

        vec![target, opponent_loop, your_copy, your_retarget]
    }

    #[test]
    fn exact_typed_bundle_renders_every_copy_and_retarget_provenance_surface() {
        assert_eq!(
            describe_tempting_offer_copy_spell_bundle(&copy_shape()).as_deref(),
            Some(ORACLE)
        );
    }

    #[test]
    fn changed_count_metric_or_retarget_link_is_not_compacted() {
        let mut wrong_metric = copy_shape();
        let (_, copy) = copy_with_id(&wrong_metric[2]).expect("controller copy");
        let mut changed = copy.clone();
        changed.count = Value::Fixed(2);
        wrong_metric[2] =
            Effect::with_id(3, Effect::new(changed)).tag(TagKey::from("__copied_stack_object__"));
        assert_eq!(
            describe_tempting_offer_copy_spell_bundle(&wrong_metric),
            None
        );

        let mut wrong_link = copy_shape();
        wrong_link[3] = Effect::new(crate::effects::ChooseNewTargetsEffect::may_for_player(
            crate::effect::EffectId(99),
            PlayerFilter::You,
        ));
        assert_eq!(describe_tempting_offer_copy_spell_bundle(&wrong_link), None);
    }

    #[test]
    fn parsed_bundle_round_trips_the_exact_oracle_surface() {
        let definition = crate::compiler_test_support::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Tempting Copy Probe",
        )
        .card_types(vec![CardType::Instant])
        .parse_text(ORACLE)
        .expect("tempting-offer copy bundle should parse");

        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            vec![ORACLE.to_string()]
        );
    }

    #[test]
    fn only_opponents_who_accept_increase_the_controllers_copy_count() {
        use crate::decision::DecisionMaker;

        struct OnlyBobAccepts(crate::ids::PlayerId);

        impl DecisionMaker for OnlyBobAccepts {
            fn decide_boolean(
                &mut self,
                _game: &crate::game_state::GameState,
                ctx: &crate::decisions::context::BooleanContext,
            ) -> bool {
                ctx.player == self.0
            }
        }

        let mut game = crate::game_state::GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = crate::ids::PlayerId::from_index(0);
        let bob = crate::ids::PlayerId::from_index(1);
        let charlie = crate::ids::PlayerId::from_index(2);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(1), "Copy Target")
            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Red,
            ]]))
            .card_types(vec![CardType::Instant])
            .build();
        let original = game.create_object_from_card(&card, alice, Zone::Stack);
        game.stack
            .push(crate::game_state::StackEntry::new(original, alice));

        let effects = copy_shape();
        let source = game.new_object_id();
        let mut decisions = OnlyBobAccepts(bob);
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_decision_maker(&mut decisions);
        ctx.tag_object(
            "chosen_spell",
            crate::snapshot::ObjectSnapshot::from_object(
                game.object(original).expect("original spell exists"),
                &game,
            ),
        );
        for effect in &effects[1..] {
            crate::effects::execute_effect(&mut game, effect, &mut ctx)
                .expect("tempting-offer copy effect should resolve");
        }

        let controller_counts = game.stack.iter().fold(
            std::collections::HashMap::<crate::ids::PlayerId, usize>::new(),
            |mut counts, entry| {
                *counts.entry(entry.controller).or_default() += 1;
                counts
            },
        );
        assert_eq!(controller_counts.get(&alice), Some(&3));
        assert_eq!(controller_counts.get(&bob), Some(&1));
        assert_eq!(controller_counts.get(&charlie), None);
    }
}
