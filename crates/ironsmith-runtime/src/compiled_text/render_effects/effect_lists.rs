use super::*;

#[path = "effect_list/activated_counter_removal_damage.rs"]
mod activated_counter_removal_damage;
#[path = "effect_list/chosen_kind_and_counted_consult.rs"]
mod chosen_kind_and_counted_consult;
#[path = "effect_list/chosen_type_untap.rs"]
mod chosen_type_untap;
#[path = "effect_list/coin_flip_target_backrefs.rs"]
mod coin_flip_target_backrefs;
#[path = "effect_list/combat_requirement_and_prohibition.rs"]
mod combat_requirement_and_prohibition;
#[path = "effect_list/consult_attachment.rs"]
mod consult_attachment;
#[path = "effect_list/copy_spell_modifiers.rs"]
mod copy_spell_modifiers;
#[path = "effect_list/exiled_collection_cast.rs"]
mod exiled_collection_cast;
#[path = "effect_list/forced_block_patterns.rs"]
mod forced_block_patterns;
#[path = "effect_list/graveyard_copy_cast.rs"]
mod graveyard_copy_cast;
#[path = "effect_list/graveyard_return_compaction.rs"]
mod graveyard_return_compaction;
#[path = "effect_list/helpers_00.rs"]
mod helpers_00;
#[path = "effect_list/helpers_01.rs"]
mod helpers_01;
#[path = "effect_list/helpers_02.rs"]
pub(crate) mod helpers_02;
#[path = "effect_list/historical_block_reanimation.rs"]
mod historical_block_reanimation;
#[path = "effect_list/mixed_target_consult.rs"]
mod mixed_target_consult;
#[path = "effect_list/optional_consult_partitions.rs"]
mod optional_consult_partitions;
#[path = "effect_list/optional_opponent_choice.rs"]
mod optional_opponent_choice;
#[path = "effect_list/possibility_storm.rs"]
mod possibility_storm;
#[path = "effect_list/quantified_player_sequence.rs"]
mod quantified_player_sequence;
#[path = "effect_list/relative_player_target_consult.rs"]
mod relative_player_target_consult;
#[path = "effect_list/source_exiled_copy_cast.rs"]
mod source_exiled_copy_cast;
#[path = "effect_list/source_exiled_return_partition.rs"]
mod source_exiled_return_partition;
#[path = "effect_list/synthetic_target_folding.rs"]
mod synthetic_target_folding;
#[path = "effect_list/targeted_opponent_consult.rs"]
mod targeted_opponent_consult;
#[path = "effect_list/tempting_offer_copy.rs"]
mod tempting_offer_copy;

use activated_counter_removal_damage::describe_activated_counter_removal_damage;
pub(in crate::compiled_text) use activated_counter_removal_damage::describe_activated_counter_removal_damage_with_source_surface;
use chosen_kind_and_counted_consult::*;
use chosen_type_untap::*;
use coin_flip_target_backrefs::*;
use combat_requirement_and_prohibition::*;
use consult_attachment::*;
use copy_spell_modifiers::*;
use exiled_collection_cast::*;
pub(super) use forced_block_patterns::*;
use graveyard_copy_cast::describe_graveyard_exile_copy_cast;
pub(in crate::compiled_text) use graveyard_copy_cast::{
    render_conditional_graveyard_exile_copy_cast_pair, render_graveyard_exile_copy_cast_pair,
};
pub(super) use graveyard_return_compaction::*;
pub(super) use helpers_00::describe_each_player_choose_creature_destroy_others;
pub(in crate::compiled_text) use helpers_00::describe_target_only_then_exchange_control;
pub(super) use helpers_00::player_is_controller_of_produced_target;
pub(super) use helpers_00::same_name_extraction_hand_draw_matches;
use helpers_00::wrapped_effect_tag;
use helpers_00::*;
pub(in crate::compiled_text) use helpers_00::{
    describe_choose_then_color_matched_combat_prevention,
    describe_choose_then_mount_vehicle_become, describe_energy_then_pay_any_then_destroy,
    describe_move_then_color_subtype_addition,
    describe_optional_sticker_aura_return_attach_sequence,
    describe_result_producer_then_for_each_tagged, describe_return_then_color_subtype_addition,
    describe_return_then_conditional_animation,
    describe_shuffle_reveal_repeated_permanent_groups_rest_bottom,
    describe_tagged_continuous_then_counter_conditional_draw,
    describe_tagged_pump_then_conditional_keyword, describe_target_only_then_damage_that_player,
    describe_target_player_cast_and_creatures_attack_restrictions,
    describe_targeted_conditional_action_then_fight,
    describe_two_distinct_targets_conditional_then_fight,
    describe_two_distinct_targets_counter_then_fight,
    describe_two_target_creature_exchange_or_fight, render_necromentia_shape,
};
pub(super) use helpers_01::describe_countered_spell_exile_replacement_followup;
pub(in crate::compiled_text) use helpers_01::describe_create_token_then_set_base_pt_bundle;
use helpers_01::describe_linked_graveyard_choices_then_may_return_bundle as describe_effect_list_linked_graveyard_choices_then_may_return_bundle;
pub(in crate::compiled_text) use helpers_01::describe_tagged_die_exile_replacement_followup;
pub(in crate::compiled_text) use helpers_01::describe_target_pump_unblockable_bundle;
pub(in crate::compiled_text) use helpers_01::render_search_reveal_opponent_choose_rest_bundle;
use helpers_01::*;
pub(in crate::compiled_text) use helpers_02::describe_face_down_pile_then_manifest;
pub(in crate::compiled_text) use helpers_02::render_consult_reveal_move_matches_then_bottom;
pub(in crate::compiled_text) use helpers_02::render_exile_top_then_put_from_among_onto_battlefield;
pub(super) use helpers_02::render_look_reveal_repeated_choices;
pub(in crate::compiled_text) use helpers_02::render_shuffle_exile_top_then_cast_any_number_with_mana_value_cap;
use helpers_02::*;
pub(in crate::compiled_text) use historical_block_reanimation::describe_historical_block_reanimation;
pub(in crate::compiled_text) use mixed_target_consult::*;
use optional_consult_partitions::*;
use optional_opponent_choice::*;
use possibility_storm::describe_cast_from_hand_consult_source_exiled_cleanup;
pub(in crate::compiled_text) use quantified_player_sequence::describe_quantified_player_mill_discard_draw;
pub(in crate::compiled_text) use relative_player_target_consult::*;
pub(in crate::compiled_text) use source_exiled_copy_cast::describe_optional_source_exiled_copy_then_cast_pair;
use source_exiled_return_partition::describe_source_exiled_return_partition;
use synthetic_target_folding::*;
use targeted_opponent_consult::describe_targeted_opponent_consult_may_cast_remainder;
use tempting_offer_copy::describe_tempting_offer_copy_spell_bundle;

pub(in crate::compiled_text) fn structural_unwrap_render_wrappers(effect: &Effect) -> &Effect {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return structural_unwrap_render_wrappers(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return structural_unwrap_render_wrappers(&tagged.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return structural_unwrap_render_wrappers(&tag_all.effect);
    }
    effect
}

fn exile_all_target_player_zone_view(effect: &Effect) -> Option<(Zone, PlayerFilter)> {
    let exile =
        structural_unwrap_render_wrappers(effect).downcast_ref::<crate::effects::ExileEffect>()?;
    if exile.face_down {
        return None;
    }
    let ChooseSpec::All(filter) = exile.spec.base() else {
        return None;
    };
    let mut stripped = filter.clone();
    let zone = stripped.zone.take()?;
    let owner = stripped.owner.take()?;
    if stripped != ObjectFilter::default() {
        return None;
    }
    Some((zone, owner))
}

/// A shared player declaration followed by exhaustive hand and graveyard
/// exiles is one target-player instruction. Keep the declaration implicit in
/// the rendered verb phrase and prove that both zone filters name the same
/// resolved player before folding them.
fn describe_exile_all_from_same_target_players_hand_and_graveyard(
    effects: &[Effect],
) -> Option<String> {
    if let [effect] = effects
        && let Some(sequence) = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::SequenceEffect>()
    {
        return describe_exile_all_from_same_target_players_hand_and_graveyard(&sequence.effects);
    }
    let [target_effect, first_exile, second_exile] = effects else {
        return None;
    };
    let target = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target.chooser.is_some()
        || target.explicit_declaration
        || !matches!(target.target.base(), ChooseSpec::Player(PlayerFilter::Any))
    {
        return None;
    }
    let (first_zone, first_owner) = exile_all_target_player_zone_view(first_exile)?;
    let (second_zone, second_owner) = exile_all_target_player_zone_view(second_exile)?;
    if first_owner != second_owner
        || !matches!(first_owner, PlayerFilter::Target(_))
        || !matches!(
            (first_zone, second_zone),
            (Zone::Hand, Zone::Graveyard) | (Zone::Graveyard, Zone::Hand)
        )
    {
        return None;
    }
    Some("Exile all cards from target player's hand and graveyard".to_string())
}

#[cfg(test)]
mod same_target_player_multi_zone_exile_tests {
    use super::*;

    fn exile_zone(zone: Zone, owner: PlayerFilter) -> Effect {
        let mut filter = ObjectFilter::default().in_zone(zone);
        filter.owner = Some(owner);
        Effect::new(crate::effects::ExileEffect::all(filter))
    }

    #[test]
    fn shared_target_player_hand_and_graveyard_exile_stays_one_instruction() {
        let target = Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::target(
            ChooseSpec::Player(PlayerFilter::Any),
        )));
        let owner = PlayerFilter::Target(Box::new(PlayerFilter::Any));
        assert_eq!(
            describe_exile_all_from_same_target_players_hand_and_graveyard(&[
                target,
                exile_zone(Zone::Hand, owner.clone()),
                exile_zone(Zone::Graveyard, owner),
            ]),
            Some("Exile all cards from target player's hand and graveyard".to_string())
        );
    }

    #[test]
    fn different_zone_owners_do_not_fold() {
        let target = Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::target(
            ChooseSpec::Player(PlayerFilter::Any),
        )));
        assert_eq!(
            describe_exile_all_from_same_target_players_hand_and_graveyard(&[
                target,
                exile_zone(
                    Zone::Hand,
                    PlayerFilter::Target(Box::new(PlayerFilter::Any))
                ),
                exile_zone(Zone::Graveyard, PlayerFilter::You),
            ]),
            None
        );
    }
}

fn describe_optional_target_player_mill(effects: &[Effect]) -> Option<String> {
    let [target_effect, mill_effect] = effects else {
        return None;
    };
    let target = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target.chooser.is_some()
        || target.explicit_declaration
        || target.target
            != ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Any))
                .with_count(crate::effect::ChoiceCount::up_to(1))
    {
        return None;
    }
    let mill = structural_unwrap_render_wrappers(mill_effect)
        .downcast_ref::<crate::effects::MillEffect>()?;
    if mill.player != PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any)) {
        return None;
    }
    let count = if mill
        .count
        .has_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo)
        && let Value::PowerOf(spec) = mill.count.unhinted()
        && let Some(ironsmith_core::SourceReferenceSurface::ThisPermanentType(surface)) =
            spec.source_reference_surface()
    {
        format!("cards equal to {surface}'s power")
    } else {
        describe_mill_count_for_player(&mill.count, &mill.player)
    };
    Some(format!("Up to one target player mills {count}",))
}

fn describe_target_must_be_blocked_same_tag(effects: &[Effect]) -> Option<String> {
    let [target_effect, restriction_effect] = effects else {
        return None;
    };
    let target_tag = wrapped_effect_tag(target_effect)?;
    let target = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target.chooser.is_some() || target.explicit_declaration {
        return None;
    }
    let ChooseSpec::Target(target_inner) = &target.target else {
        return None;
    };
    let ChooseSpec::Object(target_filter) = target_inner.as_ref() else {
        return None;
    };
    let mut semantic_target_filter = target_filter.clone();
    semantic_target_filter.union_surface = semantic_target_filter
        .union_surface
        .clone()
        .with_explicit_card_type_noun(None);
    if semantic_target_filter != ObjectFilter::creature() {
        return None;
    }

    let restriction = structural_unwrap_render_wrappers(restriction_effect)
        .downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::MustBeBlocked(restricted) = &restriction.restriction else {
        return None;
    };
    if restriction.duration != Until::EndOfTurn
        || !matches!(
            restriction.start,
            crate::effect::RestrictionStart::Immediate
        )
        || restriction.duration_surface != crate::effect::RestrictionDurationSurface::Default
        || !filter_is_exactly_tagged(restricted, target_tag)
    {
        return None;
    }

    Some("Target creature must be blocked this turn if able".to_string())
}

#[cfg(test)]
mod target_must_be_blocked_same_tag_tests {
    use super::*;

    fn effects(target: ChooseSpec, restriction_tag: &str) -> Vec<Effect> {
        vec![
            Effect::new(crate::effects::TargetOnlyEffect::new(target)).tag("targeted_0"),
            Effect::new(crate::effects::CantEffect::new(
                crate::effect::Restriction::must_be_blocked(ObjectFilter::tagged(restriction_tag)),
                Until::EndOfTurn,
            )),
        ]
    }

    #[test]
    fn exact_target_and_same_tag_requirement_compact() {
        let effects = effects(ChooseSpec::target(ChooseSpec::creature()), "targeted_0");
        assert_eq!(
            describe_target_must_be_blocked_same_tag(&effects).as_deref(),
            Some("Target creature must be blocked this turn if able")
        );
    }

    #[test]
    fn changed_tag_or_nontarget_choice_does_not_compact() {
        let changed_tag = effects(
            ChooseSpec::target(ChooseSpec::creature()),
            "different_target",
        );
        assert!(describe_target_must_be_blocked_same_tag(&changed_tag).is_none());

        let choice = effects(ChooseSpec::creature(), "targeted_0");
        assert!(describe_target_must_be_blocked_same_tag(&choice).is_none());
    }
}

#[cfg(test)]
mod optional_target_player_mill_tests {
    use super::*;

    fn effects(target_count: crate::effect::ChoiceCount) -> Vec<Effect> {
        vec![
            Effect::new(crate::effects::TargetOnlyEffect::new(
                ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Any)).with_count(target_count),
            )),
            Effect::new(crate::effects::MillEffect::new(
                Value::PowerOf(Box::new(ChooseSpec::Source.with_surface_hint(
                    ironsmith_core::ChooseSpecSurfaceHint::SourceReference(
                        ironsmith_core::SourceReferenceSurface::ThisPermanentType(
                            "this creature".to_string(),
                        ),
                    ),
                )))
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo),
                PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any)),
            )),
        ]
    }

    #[test]
    fn optional_target_player_is_not_erased_from_linked_mill() {
        assert_eq!(
            describe_optional_target_player_mill(&effects(crate::effect::ChoiceCount::up_to(1)))
                .as_deref(),
            Some("Up to one target player mills cards equal to this creature's power")
        );
        assert!(
            describe_optional_target_player_mill(&effects(crate::effect::ChoiceCount::exactly(1)))
                .is_none(),
            "an exact-one target declaration must retain its ordinary renderer"
        );
    }
}

fn describe_target_mill_then_may_cast_from_exact_milled_set(effects: &[Effect]) -> Option<String> {
    let [target_effect, mill_effect, choose_effect, cast_effect] = effects else {
        return None;
    };
    let target = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target.target != ChooseSpec::target_opponent() {
        return None;
    }
    let mill_tag = wrapped_effect_tag(mill_effect)?;
    let mill = structural_unwrap_render_wrappers(mill_effect)
        .downcast_ref::<crate::effects::MillEffect>()?;
    if mill.player != PlayerFilter::target_opponent() || mill.count != Value::Fixed(5) {
        return None;
    }
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let mut expected_filter = ObjectFilter::default().in_zone(Zone::Graveyard);
    expected_filter.card_types = vec![CardType::Instant, CardType::Sorcery];
    expected_filter
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: mill_tag.clone(),
            relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        });
    if choose.filter != expected_filter
        || choose.count != crate::effect::ChoiceCount::up_to(1)
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || choose.chooser != PlayerFilter::You
        || choose.zone != Some(Zone::Graveyard)
        || !choose.additional_zones.is_empty()
        || choose.is_search
        || choose.reveal
    {
        return None;
    }
    let cast = structural_unwrap_render_wrappers(cast_effect)
        .downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if cast.tag != choose.tag
        || cast.player != PlayerFilter::You
        || cast.allow_land
        || cast.as_copy
        || !cast.without_paying_mana_cost
        || cast.additional_mana_cost.is_some()
        || cast.cost_reduction.is_some()
        || cast.mana_spend_mode != ironsmith_core::value_model::ManaSpendMode::Normal
    {
        return None;
    }

    Some(
        "Target opponent mills five cards. You may cast an instant or sorcery spell from among them without paying its mana cost"
            .to_string(),
    )
}

#[cfg(test)]
mod exact_milled_cast_surface_tests {
    use super::*;

    fn mill_cast_program(milled_tag: TagKey) -> Vec<Effect> {
        let chosen_tag = TagKey::from("chosen_milled");
        let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
        filter.card_types = vec![CardType::Instant, CardType::Sorcery];
        filter
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: milled_tag.clone(),
                relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            });
        vec![
            Effect::new(crate::effects::TargetOnlyEffect::new(
                ChooseSpec::target_opponent(),
            )),
            Effect::new(crate::effects::MillEffect::new(
                Value::Fixed(5),
                PlayerFilter::target_opponent(),
            ))
            .tag(milled_tag),
            Effect::choose_objects(
                filter,
                crate::effect::ChoiceCount::up_to(1),
                PlayerFilter::You,
                chosen_tag.clone(),
            ),
            Effect::new(
                crate::effects::CastTaggedEffect::new(chosen_tag, PlayerFilter::You)
                    .without_paying_mana_cost(),
            ),
        ]
    }

    #[test]
    fn casts_only_from_the_exact_milled_set() {
        let effects = mill_cast_program(TagKey::from("milled"));
        assert_eq!(
            describe_target_mill_then_may_cast_from_exact_milled_set(&effects).as_deref(),
            Some(
                "Target opponent mills five cards. You may cast an instant or sorcery spell from among them without paying its mana cost"
            )
        );

        let mut wrong_provenance = effects.clone();
        let choose = wrong_provenance[2]
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            .expect("choice fixture")
            .clone();
        let mut choose = choose;
        choose.filter.tagged_constraints.clear();
        wrong_provenance[2] = Effect::new(choose);
        assert_eq!(
            describe_target_mill_then_may_cast_from_exact_milled_set(&wrong_provenance),
            None
        );
    }
}

#[cfg(test)]
mod hybrid_vote_surface_tests {
    use super::*;

    fn hybrid_vote_program(control_target: ChooseSpec) -> Vec<Effect> {
        let chosen = TagKey::from("money_permanent");
        let choose = Effect::new(crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::permanent()
                .in_zone(Zone::Battlefield)
                .owned_by(PlayerFilter::IteratedPlayer),
            crate::effect::ChoiceCount::exactly(1),
            PlayerFilter::You,
            chosen.clone(),
        ));
        let mut control = crate::effects::ApplyContinuousEffect::new_runtime(
            crate::continuous::EffectTarget::Source,
            crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController,
            Until::Forever,
        );
        control.target_spec = Some(control_target);
        let money = Effect::new(crate::effects::SequenceEffect::new(vec![
            choose,
            Effect::new(control),
        ]));
        let vote = Effect::new(
            crate::effects::VoteEffect::named(
                vec![
                    crate::effects::VoteOption::new("time", Vec::new()),
                    crate::effects::VoteOption::new("money", vec![money]),
                ],
                0,
                0,
            )
            .starting_with_controller(true),
        );
        let time = Effect::new(crate::effects::RepeatEffectsEffect::new(
            Value::VoteCount("time".to_string()),
            vec![Effect::new(crate::effects::ExtraTurnEffect::you())],
        ));
        vec![vote, time]
    }

    #[test]
    fn voter_relative_and_count_relative_options_keep_oracle_order() {
        let effects = hybrid_vote_program(ChooseSpec::Tagged(TagKey::from("money_permanent")));

        assert_eq!(
            describe_effect_list(&effects),
            "Council's dilemma — Starting with you, each player votes for time or money. For each time vote, take an extra turn after this one. For each money vote, choose a permanent owned by the voter and gain control of it"
        );
    }

    #[test]
    fn voter_relative_compactor_rejects_control_of_a_different_choice() {
        let effects = hybrid_vote_program(ChooseSpec::Tagged(TagKey::from("other_permanent")));

        assert_ne!(
            describe_effect_list(&effects),
            "Council's dilemma — Starting with you, each player votes for time or money. For each time vote, take an extra turn after this one. For each money vote, choose a permanent owned by the voter and gain control of it"
        );
    }
}

pub(super) fn describe_full_game_source_damage_recipient_union(
    effects: &[Effect],
) -> Option<String> {
    let [players_effect, objects_effect] = effects else {
        return None;
    };
    let players = players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if players.starting_with_controller || players.stop_after_first_happened {
        return None;
    }
    let PlayerFilter::WasDealtDamageBySourceThisGame { base } = &players.filter else {
        return None;
    };
    if base.as_ref() != &PlayerFilter::Opponent {
        return None;
    }
    let [player_damage] = players.effects.as_slice() else {
        return None;
    };
    let player_damage = structural_unwrap_render_wrappers(player_damage)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if !matches!(
        player_damage.target,
        ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    ) || player_damage.source_is_combat
        || player_damage.unpreventable
    {
        return None;
    }

    let objects = objects_effect.downcast_ref::<crate::effects::ForEachObject>()?;
    if !objects.filter.was_dealt_damage_by_source_this_game
        || objects.filter.explicit_card_type_noun() != Some(CardType::Planeswalker)
    {
        return None;
    }
    let mut surface = objects.filter.union_surface.clone();
    surface = surface.with_explicit_card_type_noun(None);
    if surface != Default::default() {
        return None;
    }
    let mut object_filter = objects.filter.clone();
    object_filter.was_dealt_damage_by_source_this_game = false;
    object_filter.union_surface = Default::default();
    if object_filter != ObjectFilter::planeswalker() {
        return None;
    }
    let [object_damage] = objects.effects.as_slice() else {
        return None;
    };
    let object_damage = structural_unwrap_render_wrappers(object_damage)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if !matches!(object_damage.target, ChooseSpec::Iterated)
        || object_damage.source_is_combat
        || object_damage.unpreventable
        || object_damage.amount != player_damage.amount
    {
        return None;
    }

    Some(format!(
        "Deal {} damage to each opponent and planeswalker it has dealt damage to this game",
        describe_value(&player_damage.amount)
    ))
}

/// Lowering can retain an authored sequence boundary around a single action.
/// The boundary is presentation metadata rather than a second runtime action;
/// structural recognizers that reconstruct the complete surrounding sentence
/// may inspect the one enclosed effect without losing identity.
pub(super) fn unwrap_singleton_sequence_member(effect: &Effect) -> &Effect {
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>()
        && let [only] = sequence.effects.as_slice()
    {
        return only;
    }
    effect
}

fn attributed_target_choice_view(
    effect: &Effect,
) -> Option<(
    effect_text_shared::TargetChoiceAttribution,
    &crate::effects::TargetOnlyEffect,
)> {
    fn walk<'a>(
        effect: &'a Effect,
        attribution: Option<effect_text_shared::TargetChoiceAttribution>,
    ) -> Option<(
        effect_text_shared::TargetChoiceAttribution,
        &'a crate::effects::TargetOnlyEffect,
    )> {
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return walk(&with_id.effect, attribution);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            let attribution =
                effect_text_shared::target_choice_attribution(tagged.tag.as_str()).or(attribution);
            return walk(&tagged.effect, attribution);
        }
        if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
            let attribution =
                effect_text_shared::target_choice_attribution(tag_all.tag.as_str()).or(attribution);
            return walk(&tag_all.effect, attribution);
        }
        Some((
            attribution?,
            effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?,
        ))
    }

    walk(effect, None)
}

/// Preserve the two authored actors in clauses where the ability controller
/// chooses one target and the opposing player tied to it chooses another.
/// The chooser relationship, not a card name, proves the coordination.
fn describe_attributed_target_choice_pair(effects: &[Effect]) -> Option<(String, usize)> {
    let [first_effect, second_effect, ..] = effects else {
        return None;
    };
    let (first_attribution, first) = attributed_target_choice_view(first_effect)?;
    let (second_attribution, second) = attributed_target_choice_view(second_effect)?;
    if first_attribution != effect_text_shared::TargetChoiceAttribution::AbilityController
        || second_attribution != effect_text_shared::TargetChoiceAttribution::Opponent
        || !first.explicit_declaration
        || !second.explicit_declaration
        || first.chooser.is_some()
        || !matches!(
            second.chooser.as_ref(),
            Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag)))
                if tag.as_str() == effect_text_shared::ABILITY_CONTROLLER_TARGET_CHOICE_TAG
        )
    {
        return None;
    }

    Some((
        format!(
            "You choose {}, and that opponent chooses {}",
            describe_choose_spec(&first.target),
            describe_choose_spec(&second.target)
        ),
        2,
    ))
}

/// The authored "destroy target creature of an opponent's choice" lowers to
/// an opponent-chosen target declaration followed by destroying the declared
/// object; restore the authored surface.
fn describe_opponent_chosen_target_action_join(
    first_effect: &Effect,
    second_effect: &Effect,
) -> Option<String> {
    let first = structural_unwrap_render_wrappers(first_effect);
    let (tag, chosen_description) = if let Some(target_only) =
        first.downcast_ref::<crate::effects::TargetOnlyEffect>()
    {
        if !matches!(target_only.chooser.as_ref(), Some(PlayerFilter::Opponent)) {
            return None;
        }
        (
            wrapped_effect_tag(first_effect)?.clone(),
            describe_target_of_opponents_choice(&target_only.target),
        )
    } else if let Some(choose) = first.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
        if choose.chooser != PlayerFilter::Opponent || choose.is_search || !choose.count.is_single()
        {
            return None;
        }
        let spec = ChooseSpec::WithCount(
            Box::new(ChooseSpec::Object(choose.filter.clone())),
            choose.count.clone(),
        );
        (
            choose.tag.clone(),
            describe_target_of_opponents_choice(&spec),
        )
    } else {
        return None;
    };
    let second = structural_unwrap_render_wrappers(second_effect);
    if let Some(destroy) = second.downcast_ref::<crate::effects::DestroyEffect>() {
        if !matches!(destroy.spec.base(), ChooseSpec::Tagged(destroy_tag) if destroy_tag == &tag) {
            return None;
        }
        return Some(format!("Destroy {chosen_description}"));
    }
    if let Some(damage) = second.downcast_ref::<crate::effects::DealDamageEffect>() {
        if !matches!(damage.target.base(), ChooseSpec::Tagged(damage_tag) if damage_tag == &tag) {
            return None;
        }
        let rendered = describe_effect(second_effect);
        let (prefix, _) = rendered.rsplit_once(" to ")?;
        return Some(format!("{prefix} to {chosen_description}"));
    }
    if let Some(tap) = second.downcast_ref::<crate::effects::TapEffect>() {
        if !matches!(tap.target.base(), ChooseSpec::Tagged(tap_tag) if tap_tag == &tag) {
            return None;
        }
        return Some(format!("Tap {chosen_description}"));
    }
    if let Some(control) = second.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        if control.target != crate::continuous::EffectTarget::Source
            || !matches!(control.target_spec.as_ref().map(ChooseSpec::base), Some(ChooseSpec::Tagged(control_tag)) if control_tag == &tag)
            || control.modification.is_some()
            || !control.additional_modifications.is_empty()
            || !matches!(
                control.runtime_modifications.as_slice(),
                [crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController]
            )
        {
            return None;
        }
        let rendered = describe_effect(second_effect);
        let rest = rendered.strip_prefix("Gain control of it")?;
        return Some(format!("Gain control of {chosen_description}{rest}"));
    }
    if let Some(return_to_hand) = second.downcast_ref::<crate::effects::ReturnToHandEffect>() {
        if !matches!(return_to_hand.spec.base(), ChooseSpec::Tagged(return_tag) if return_tag == &tag)
        {
            return None;
        }
        let rendered = describe_effect(second_effect);
        let chosen = format!("return {chosen_description}");
        if rendered.starts_with("Return it") {
            return Some(rendered.replacen("Return it", &capitalize_first(&chosen), 1));
        }
        if rendered.starts_with("return it") {
            return Some(rendered.replacen("return it", &chosen, 1));
        }
        return None;
    }
    if let Some(sacrifice) = second.downcast_ref::<crate::effects::zones::SacrificePlayerEffect>() {
        if sacrifice.player != PlayerFilter::You
            || !sacrifice
                .filter
                .tagged_constraints
                .iter()
                .any(|constraint| {
                    constraint.tag == tag
                        && matches!(
                            constraint.relation,
                            crate::target::TaggedOpbjectRelation::IsTaggedObject
                        )
                })
        {
            return None;
        }
        return Some(format!("Sacrifice {chosen_description}"));
    }
    None
}

fn describe_target_of_opponents_choice(target: &ChooseSpec) -> String {
    let text = describe_choose_spec(target)
        .replace(" that player controls", " they control")
        .replace(" that player owns", " they own");
    let insertion = [
        " you don't control",
        " you do not control",
        " you control",
        " an opponent controls",
        " target opponent controls",
        " they control",
        " they own",
        " that player controls",
    ]
    .into_iter()
    .filter_map(|suffix| text.find(suffix).map(|index| (index, suffix)))
    .min_by_key(|(index, _)| *index);
    if let Some((index, _)) = insertion {
        format!(
            "{} of an opponent's choice{}",
            &text[..index],
            &text[index..]
        )
    } else {
        format!("{text} of an opponent's choice")
    }
}

/// The play-permission + cost-waiver pair for one tagged card is oracle's
/// single "Until end of turn, you may cast that card without paying its
/// mana cost"; describing both halves doubles the surface.
pub(super) fn describe_temporary_tagged_permission_surface(
    permission: &crate::effects::GrantPlayTaggedEffect,
    without_paying_mana_cost: bool,
) -> Option<String> {
    let surface = permission.surface.as_ref()?;
    if !matches!(
        permission.duration,
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
            | crate::effects::GrantPlayTaggedDuration::UntilSourceExilesAnother
    ) {
        return None;
    }

    let object_surface = surface.object.as_ref()?;
    let (object_text, plural) = match object_surface {
        ironsmith_core::GrantPlayTaggedObjectSurface::It => ("it".to_string(), false),
        ironsmith_core::GrantPlayTaggedObjectSurface::ThatCard => ("that card".to_string(), false),
        ironsmith_core::GrantPlayTaggedObjectSurface::ThatCardFromExile => {
            ("that card from exile".to_string(), false)
        }
        ironsmith_core::GrantPlayTaggedObjectSurface::ThatSpell => {
            ("that spell".to_string(), false)
        }
        ironsmith_core::GrantPlayTaggedObjectSurface::Them => ("them".to_string(), true),
        ironsmith_core::GrantPlayTaggedObjectSurface::ThoseCards => {
            ("those cards".to_string(), true)
        }
        ironsmith_core::GrantPlayTaggedObjectSurface::SpellsFromAmongThoseCards => {
            if permission.max_plays == Some(1) {
                ("a spell from among those cards".to_string(), false)
            } else {
                ("spells from among those cards".to_string(), true)
            }
        }
        ironsmith_core::GrantPlayTaggedObjectSurface::SpellsFromAmongThoseExiledCards => {
            ("spells from among those exiled cards".to_string(), true)
        }
        ironsmith_core::GrantPlayTaggedObjectSurface::SpellFromAmongCardsExiledWithSource {
            creature_spell,
            source,
        } => {
            let spell = if *creature_spell {
                "a creature spell"
            } else {
                "a spell"
            };
            (
                format!(
                    "{spell} from among cards exiled with {}",
                    source.display_text()
                ),
                false,
            )
        }
    };
    let verb = if permission.allow_land {
        "play"
    } else {
        "cast"
    };
    let mut clause = format!(
        "{} may {verb} {object_text}",
        describe_player_filter(&permission.player)
    );
    if permission.duration == crate::effects::GrantPlayTaggedDuration::UntilSourceExilesAnother {
        let source = surface.until_source_exiles_another.as_ref()?;
        clause.push_str(" until you exile another card with ");
        clause.push_str(&source.display_text());
        return Some(clause);
    }
    if surface.leading_duration {
        clause = format!("Until end of turn, {clause}");
    } else {
        clause.push_str(" this turn");
    }
    if without_paying_mana_cost {
        clause.push_str(if plural {
            " without paying their mana costs"
        } else {
            " without paying its mana cost"
        });
    }

    let mana_reference = surface
        .mana_reference
        .map(|reference| match reference {
            ironsmith_core::GrantPlayTaggedManaReferenceSurface::It => "it",
            ironsmith_core::GrantPlayTaggedManaReferenceSurface::ThatSpell => "that spell",
            ironsmith_core::GrantPlayTaggedManaReferenceSurface::Them => "them",
            ironsmith_core::GrantPlayTaggedManaReferenceSurface::ThoseSpells => "those spells",
        })
        .unwrap_or(if plural { "them" } else { "that spell" });
    if let Some(mana_clause) = permission.mana_spend_cast_clause(mana_reference) {
        clause.push_str(", and ");
        clause.push_str(&mana_clause);
    }
    Some(clause)
}

fn describe_play_permission_then_free_cast_join(
    first_effect: &Effect,
    second_effect: &Effect,
) -> Option<String> {
    let permission = structural_unwrap_render_wrappers(first_effect)
        .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    let free_cast = structural_unwrap_render_wrappers(second_effect)
        .downcast_ref::<crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect>(
    )?;
    if permission.tag != free_cast.tag || permission.player != free_cast.player {
        return None;
    }
    if let Some(rendered) = describe_temporary_tagged_permission_surface(permission, true) {
        return Some(rendered);
    }
    let rendered = describe_effect(second_effect);
    Some(rendered.replace(" from exile ", " "))
}

fn describe_opponent_chosen_target_destroy_pair(effects: &[Effect]) -> Option<(String, usize)> {
    let [first_effect, second_effect, ..] = effects else {
        return None;
    };
    Some((
        describe_opponent_chosen_target_action_join(first_effect, second_effect)?,
        2,
    ))
}

/// Oracle's "of an opponent's choice" first lets the effect's controller
/// select an opponent in multiplayer, then delegates the object choice to
/// that player. The explicit player-choice effect is executable scaffolding;
/// the authored surface remains the compact possessive clause.
fn describe_selected_opponent_chosen_action<E: std::borrow::Borrow<Effect>>(
    effects: &[E],
) -> Option<(String, usize)> {
    let [
        choose_opponent_effect,
        choose_object_effect,
        action_effect,
        ..,
    ] = effects
    else {
        return None;
    };
    let choose_opponent_effect = choose_opponent_effect.borrow();
    let choose_object_effect = choose_object_effect.borrow();
    let action_effect = action_effect.borrow();
    let choose_opponent = structural_unwrap_render_wrappers(choose_opponent_effect)
        .downcast_ref::<crate::effects::ChoosePlayerEffect>()?;
    let choose_object = structural_unwrap_render_wrappers(choose_object_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose_opponent.chooser != PlayerFilter::You
        || choose_opponent.filter != PlayerFilter::Opponent
        || choose_opponent.random
        || !choose_opponent.excluded_tags.is_empty()
        || choose_object.chooser != PlayerFilter::TaggedPlayer(choose_opponent.tag.clone())
    {
        return None;
    }
    let mut surface_choice = choose_object.clone();
    surface_choice.chooser = PlayerFilter::Opponent;
    let surface_choice = Effect::new(surface_choice);
    Some((
        describe_opponent_chosen_target_action_join(&surface_choice, action_effect)?,
        3,
    ))
}

fn delegated_subset_choice_pool_tag(
    choice: &crate::effects::ChooseObjectsEffect,
) -> Option<&crate::TagKey> {
    let [constraint] = choice.filter.tagged_constraints.as_slice() else {
        return None;
    };
    if constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
        || choice.tag.as_str() != format!("{}__delegated_subset", constraint.tag.as_str())
    {
        return None;
    }
    let mut plain = choice.filter.clone();
    plain.tagged_constraints.clear();
    plain.zone = None;
    (plain == ObjectFilter::default()).then_some(&constraint.tag)
}

/// A delegated subset is represented by an explicit opponent-player choice
/// followed by an object choice whose filter names the prior collection. The
/// shared chooser tag and generated subset tag retain enough identity to
/// render the authored “an opponent chooses N of them” without exposing the
/// multiplayer scaffolding.
fn describe_delegated_subset_choice(effects: &[Effect]) -> Option<String> {
    let [player_effect, object_effect] = effects else {
        return None;
    };
    let player = structural_unwrap_render_wrappers(player_effect)
        .downcast_ref::<crate::effects::ChoosePlayerEffect>()?;
    let choice = structural_unwrap_render_wrappers(object_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if player.chooser != PlayerFilter::You
        || player.filter != PlayerFilter::Opponent
        || player.random
        || !player.excluded_tags.is_empty()
        || choice.chooser != PlayerFilter::TaggedPlayer(player.tag.clone())
        || choice.is_search
        || choice.count.min == 0
        || choice.count.max != Some(choice.count.min)
        || choice.count.dynamic_x
        || choice.count.random
    {
        return None;
    }
    let pool_tag = delegated_subset_choice_pool_tag(choice)?;
    let count =
        number_word(choice.count.min as i32).unwrap_or_else(|| choice.count.min.to_string());
    let collection = if pool_tag.as_str() == crate::tag::SOURCE_EXILED_TAG {
        "the exiled cards"
    } else {
        "them"
    };
    Some(format!("An opponent chooses {count} of {collection}"))
}

fn delegated_collection_complement_tags(
    target: &ChooseSpec,
) -> Option<(&crate::TagKey, &crate::TagKey)> {
    let ChooseSpec::Object(filter) = target.base() else {
        return None;
    };
    let [first, second] = filter.tagged_constraints.as_slice() else {
        return None;
    };
    let (pool, subset) = match (first.relation, second.relation) {
        (
            crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            crate::filter::TaggedOpbjectRelation::IsNotTaggedObject,
        ) => (&first.tag, &second.tag),
        (
            crate::filter::TaggedOpbjectRelation::IsNotTaggedObject,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        ) => (&second.tag, &first.tag),
        _ => return None,
    };
    if subset.as_str() != format!("{}__delegated_subset", pool.as_str()) {
        return None;
    }
    let mut plain = filter.clone();
    plain.tagged_constraints.clear();
    (plain == ObjectFilter::default()).then_some((pool, subset))
}

fn describe_delegated_collection_complement_move(effect: &Effect) -> Option<String> {
    let movement = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    delegated_collection_complement_tags(&movement.target)?;
    let mut surface = movement.clone();
    surface.target = ChooseSpec::Tagged(crate::TagKey::from("__delegated_collection_other"))
        .with_surface_hint(crate::target::ChooseSpecSurfaceHint::SourceReference(
            crate::target::SourceReferenceSurface::ThisPermanentType("the other".to_string()),
        ));
    Some(describe_effect(&Effect::new(surface)))
}

pub(super) fn describe_delegated_collection_partition_moves(effects: &[Effect]) -> Option<String> {
    let (selected_effect, complement_effect, counter_effect) = match effects {
        [selected, complement] => (selected, complement, None),
        [selected, complement, counter] => (selected, complement, Some(counter)),
        _ => return None,
    };
    let selected = structural_unwrap_render_wrappers(selected_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let (_, subset) = delegated_collection_complement_tags(
        &structural_unwrap_render_wrappers(complement_effect)
            .downcast_ref::<crate::effects::MoveToZoneEffect>()?
            .target,
    )?;
    if !matches!(selected.target.base(), ChooseSpec::Tagged(tag) if tag == subset) {
        return None;
    }
    let mut selected_surface = selected.clone();
    selected_surface.target = selected_surface.target.with_surface_hint(
        crate::target::ChooseSpecSurfaceHint::SourceReference(
            crate::target::SourceReferenceSurface::ThisPermanentType("that card".to_string()),
        ),
    );
    let mut selected_text = describe_effect(&Effect::new(selected_surface));
    if selected.zone == Zone::Library
        && let Some(rest) = selected_text.strip_prefix("Put ")
    {
        selected_text = format!("You put {rest}");
    }
    let mut complement_text = describe_delegated_collection_complement_move(complement_effect)?;
    if let Some(counter_effect) = counter_effect {
        let counter = structural_unwrap_render_wrappers(counter_effect)
            .downcast_ref::<crate::effects::PutCountersEffect>()?;
        if counter.distributed
            || counter.target_count.is_some()
            || !matches!(counter.target.base(), ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::SOURCE_EXILED_TAG)
        {
            return None;
        }
        let counter_name = describe_counter_type(counter.counter_type);
        let counter_clause = if counter.amount.unhinted() == &Value::Fixed(1) {
            format!(
                " with {} on it",
                with_indefinite_article(&format!("{counter_name} counter"))
            )
        } else {
            format!(
                " with {} {counter_name} counters on it",
                describe_value(&counter.amount)
            )
        };
        complement_text.push_str(&counter_clause);
    }
    Some(format!(
        "{selected_text} and {}",
        lowercase_first(&complement_text)
    ))
}

pub(super) fn describe_delegated_partition_conditional_without_leading_then(
    sequence: &crate::effects::SequenceEffect,
) -> Option<String> {
    if sequence.surface != ironsmith_core::SequenceSurface::SentenceLeadingThen {
        return None;
    }
    let [effect] = sequence.effects.as_slice() else {
        return None;
    };
    let conditional = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    describe_delegated_subset_with_hand_remainder(&conditional.if_false)?;
    Some(describe_effect(effect))
}

fn exact_delegated_remainder_to_hand(
    effect: &Effect,
    pool_tag: &crate::TagKey,
    subset_tag: &crate::TagKey,
) -> bool {
    let Some(for_each) = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()
    else {
        return false;
    };
    let [conditional_effect] = for_each.effects.as_slice() else {
        return false;
    };
    let Some(conditional) = structural_unwrap_render_wrappers(conditional_effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()
    else {
        return false;
    };
    let [move_effect] = conditional.if_false.as_slice() else {
        return false;
    };
    let Some(movement) = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
    else {
        return false;
    };
    let crate::effect::Condition::TaggedObjectMatches(condition_tag, membership) =
        &conditional.condition
    else {
        return false;
    };
    let [constraint] = membership.tagged_constraints.as_slice() else {
        return false;
    };
    let mut plain_membership = membership.clone();
    plain_membership.tagged_constraints.clear();
    for_each.tag == *pool_tag
        && condition_tag.as_str() == "__it__"
        && constraint.tag == *subset_tag
        && constraint.relation == crate::filter::TaggedOpbjectRelation::SameStableId
        && plain_membership == ObjectFilter::default()
        && conditional.if_true.is_empty()
        && movement.zone == Zone::Hand
        && !movement.to_top
        && movement.library_order.is_none()
        && movement.remainder_surface == Some(ironsmith_core::LibraryRemainderSurface::Rest)
        && matches!(movement.target.base(), ChooseSpec::Iterated)
}

fn describe_delegated_subset_with_hand_remainder(effects: &[Effect]) -> Option<String> {
    let [player_effect, object_effect, remainder_effect] = effects else {
        return None;
    };
    let choice = structural_unwrap_render_wrappers(object_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let pool_tag = delegated_subset_choice_pool_tag(choice)?;
    if !exact_delegated_remainder_to_hand(remainder_effect, pool_tag, &choice.tag) {
        return None;
    }
    let choice_text =
        describe_delegated_subset_choice(&[player_effect.clone(), object_effect.clone()])?;
    Some(format!(
        "{choice_text}. Leave the chosen cards in your graveyard and put the rest into your hand"
    ))
}

fn describe_declared_pool_then_delegated_partition_conditional(
    effects: &[Effect],
) -> Option<String> {
    let [target_effect, conditional_effect] = effects else {
        return None;
    };
    let pool_tag = wrapped_effect_tag(target_effect)?;
    let target = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let conditional = structural_unwrap_render_wrappers(conditional_effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !target.explicit_declaration
        || target.chooser.is_some()
        || conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf
    {
        return None;
    }
    let [_, choice_effect, _] = conditional.if_false.as_slice() else {
        return None;
    };
    let choice = structural_unwrap_render_wrappers(choice_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if delegated_subset_choice_pool_tag(choice)? != pool_tag {
        return None;
    }
    describe_delegated_subset_with_hand_remainder(&conditional.if_false)?;
    Some(format!(
        "{}. {}",
        describe_effect(target_effect).trim_end_matches('.'),
        describe_effect(conditional_effect).trim_end_matches('.')
    ))
}

/// Rejoin an ordinary target action with the same action applied to a target
/// selected by an opponent. The delegated target declaration and its tagged
/// consumer are executable scaffolding for the authored second target slot.
fn describe_primary_then_opponent_chosen_same_action<E: std::borrow::Borrow<Effect>>(
    effects: &[E],
) -> Option<(String, usize)> {
    let primary_effect: &Effect = std::borrow::Borrow::borrow(effects.first()?);
    let second_effect: &Effect = std::borrow::Borrow::borrow(effects.get(1)?);
    let (chosen_effect, delegated_effect, consumed) = if let Some(sequence) =
        structural_unwrap_render_wrappers(second_effect)
            .downcast_ref::<crate::effects::SequenceEffect>()
        && matches!(
            sequence.surface,
            ironsmith_core::SequenceSurface::Sequential
                | ironsmith_core::SequenceSurface::Coordinated
        )
        && let [chosen, delegated] = sequence.effects.as_slice()
    {
        (chosen, delegated, 2)
    } else {
        (
            second_effect,
            std::borrow::Borrow::borrow(effects.get(2)?),
            3,
        )
    };
    let chosen_tag = wrapped_effect_tag(chosen_effect)?;
    let chosen = structural_unwrap_render_wrappers(chosen_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if chosen.chooser != Some(PlayerFilter::Opponent)
        || !chosen.explicit_declaration
        || !chosen.target.is_target()
    {
        return None;
    }
    let primary = structural_unwrap_render_wrappers(primary_effect);
    let delegated = structural_unwrap_render_wrappers(delegated_effect);
    let chosen_target = describe_target_of_opponents_choice(&chosen.target);

    if let (Some(first), Some(second)) = (
        primary.downcast_ref::<crate::effects::DestroyEffect>(),
        delegated.downcast_ref::<crate::effects::DestroyEffect>(),
    ) {
        if first.spec != chosen.target
            || !matches!(second.spec.base(), ChooseSpec::Tagged(tag) if tag == chosen_tag)
        {
            return None;
        }
        return Some((
            format!(
                "Destroy {} and {chosen_target}",
                describe_choose_spec(&first.spec)
            ),
            consumed,
        ));
    }

    if let (Some(first), Some(second)) = (
        primary.downcast_ref::<crate::effects::TapEffect>(),
        delegated.downcast_ref::<crate::effects::TapEffect>(),
    ) {
        if !matches!(second.target.base(), ChooseSpec::Tagged(tag) if tag == chosen_tag) {
            return None;
        }
        return Some((
            format!(
                "Tap {} and {chosen_target}",
                describe_choose_spec(&first.target)
            ),
            consumed,
        ));
    }

    let first = primary.downcast_ref::<crate::effects::DealDamageEffect>()?;
    let second = delegated.downcast_ref::<crate::effects::DealDamageEffect>()?;
    if first.target != chosen.target
        || first.amount != second.amount
        || first.source_is_combat != second.source_is_combat
        || first.unpreventable != second.unpreventable
        || !matches!(second.target.base(), ChooseSpec::Tagged(tag) if tag == chosen_tag)
    {
        return None;
    }
    let rendered = describe_effect(primary_effect);
    let (prefix, _) = rendered.rsplit_once(" to ")?;
    Some((
        format!(
            "{prefix} to {} and {} damage to {chosen_target}",
            describe_choose_spec(&first.target),
            describe_value(&first.amount),
        ),
        consumed,
    ))
}

const CHOSEN_OBJECTS_SURFACE_TAG: &str = "__chosen_objects__";

fn is_not_you_filter(player: &PlayerFilter) -> bool {
    matches!(player, PlayerFilter::NotYou)
        || matches!(
            player,
            PlayerFilter::Excluding { base, excluded }
                if matches!(base.as_ref(), PlayerFilter::Any)
                    && matches!(excluded.as_ref(), PlayerFilter::You)
        )
}

fn chosen_controller_partition_filter(
    spec: &ChooseSpec,
    controller_matches: impl FnOnce(&PlayerFilter) -> bool,
) -> Option<ObjectFilter> {
    let ChooseSpec::All(filter) = spec.base() else {
        return None;
    };
    if !filter.controller.as_ref().is_some_and(controller_matches)
        || filter.tagged_constraints.len() != 1
        || !filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == CHOSEN_OBJECTS_SURFACE_TAG
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
    {
        return None;
    }
    let mut normalized = filter.clone();
    normalized.controller = None;
    Some(normalized)
}

/// Preserve a repeated target declaration as separate target slots, then
/// render controller-partitioned actions over the exact accumulated chosen
/// set. The shared tag and equal filters prove both follow-ups consume the
/// same objects selected by the declarations.
fn describe_repeated_targets_then_chosen_controller_partition(
    effects: &[Effect],
) -> Option<(String, usize)> {
    let mut target_parts = Vec::new();
    let mut consumed_targets = 0;
    for effect in effects {
        if wrapped_effect_tag(effect).is_none_or(|tag| tag.as_str() != CHOSEN_OBJECTS_SURFACE_TAG) {
            break;
        }
        let target_only = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
        let count = target_only.target.count();
        if !target_only.explicit_declaration
            || target_only.chooser.is_some()
            || !target_only.target.is_target()
            || !matches!(target_only.target.base(), ChooseSpec::Object(_))
            || count.min != 0
            || count.max != Some(1)
            || count.dynamic_x
            || count.random
        {
            return None;
        }
        target_parts.push(describe_choose_spec(&target_only.target));
        consumed_targets += 1;
    }
    if consumed_targets < 2 {
        return None;
    }

    let untap = structural_unwrap_render_wrappers(effects.get(consumed_targets)?)
        .downcast_ref::<crate::effects::UntapEffect>()?;
    let tap = structural_unwrap_render_wrappers(effects.get(consumed_targets + 1)?)
        .downcast_ref::<crate::effects::TapEffect>()?;
    let untap_filter = chosen_controller_partition_filter(&untap.target, |controller| {
        matches!(controller, PlayerFilter::You)
    })?;
    let tap_filter = chosen_controller_partition_filter(&tap.target, is_not_you_filter)?;
    if untap_filter != tap_filter {
        return None;
    }

    let mut noun_filter = untap_filter;
    noun_filter.tagged_constraints.clear();
    let all_nouns = describe_choose_spec(&ChooseSpec::All(noun_filter));
    let nouns = all_nouns.strip_prefix("all ")?;
    Some((
        format!(
            "Choose {}. Untap the chosen {nouns} you control. Tap the chosen {nouns} you don't control",
            join_with_and(&target_parts)
        ),
        consumed_targets + 2,
    ))
}

fn describe_untap_then_phase_out_until_source_leaves(
    effects: &[Effect],
) -> Option<(String, usize)> {
    let [first, second, ..] = effects else {
        return None;
    };
    let untap =
        structural_unwrap_render_wrappers(first).downcast_ref::<crate::effects::UntapEffect>()?;
    let phase_out = structural_unwrap_render_wrappers(second)
        .downcast_ref::<crate::effects::PhaseOutEffect>()?;
    if phase_out.duration != crate::effects::PhaseOutDuration::UntilSourceLeaves
        || untap.target != phase_out.spec
    {
        return None;
    }
    let ChooseSpec::All(_) = phase_out.spec.base() else {
        return None;
    };
    let selected = describe_choose_spec(&phase_out.spec);
    let referenced = selected.strip_prefix("all ").unwrap_or(selected.as_str());
    let source = phase_out
        .source_surface
        .as_ref()
        .map(crate::target::SourceReferenceSurface::display_text)
        .unwrap_or_else(|| "this permanent".to_string());
    Some((
        format!(
            "Untap {selected}, then those {referenced} phase out until {source} leaves the battlefield"
        ),
        2,
    ))
}

fn describe_damage_then_gain_life_this_way(effects: &[Effect]) -> Option<(String, usize)> {
    let [producer_effect, gain_effect, ..] = effects else {
        return None;
    };
    let gain = structural_unwrap_render_wrappers(gain_effect)
        .downcast_ref::<crate::effects::GainLifeEffect>()?;
    if gain.player != ChooseSpec::Player(PlayerFilter::You) {
        return None;
    }

    let producer = structural_unwrap_render_wrappers(producer_effect);
    let aggregate_inner = producer
        .downcast_ref::<crate::effects::ForPlayersEffect>()
        .and_then(|for_players| {
            (!for_players.starting_with_controller
                && !for_players.stop_after_first_happened
                && matches!(
                    for_players.filter,
                    PlayerFilter::Opponent | PlayerFilter::NotYou
                ))
            .then_some(for_players.effects.as_slice())
        })
        .and_then(|effects| match effects {
            [effect] => Some(structural_unwrap_render_wrappers(effect)),
            _ => None,
        });

    let producer_id = effect_outer_id(producer_effect)?;
    let linked_damage = producer
        .downcast_ref::<crate::effects::DealDamageEffect>()
        .or_else(|| aggregate_inner?.downcast_ref::<crate::effects::DealDamageEffect>())
        .is_some()
        && gain.amount.has_surface_hint(ValueSurfaceHint::DamageDealt)
        && matches!(
            gain.amount.unhinted(),
            Value::EffectValue(effect_id) if *effect_id == producer_id
        );
    let linked_life_loss = aggregate_inner
        .and_then(|effect| effect.downcast_ref::<crate::effects::LoseLifeEffect>())
        .is_some_and(|lose| {
            matches!(
                lose.player,
                ChooseSpec::Player(PlayerFilter::IteratedPlayer)
            )
        })
        && matches!(
            gain.amount.unhinted(),
            Value::EventValue(crate::effect::EventValueSpec::LifeAmount)
        );
    if !linked_damage && !linked_life_loss {
        return None;
    }

    let producer = describe_effect(producer_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let reference = if linked_damage {
        "the damage dealt this way"
    } else {
        "the life lost this way"
    };
    let (connective, gain_subject) = if linked_damage {
        (" and ", "you")
    } else {
        (". ", "You")
    };
    Some((
        format!("{producer}{connective}{gain_subject} gain life equal to {reference}"),
        2,
    ))
}

fn describe_participant_choose_then_untap_chosen(effects: &[Effect]) -> Option<(String, usize)> {
    let [choose_effect, untap_effect, ..] = effects else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let untap = structural_unwrap_render_wrappers(untap_effect)
        .downcast_ref::<crate::effects::UntapEffect>()?;
    if !matches!(
        choose.chooser,
        PlayerFilter::Active | PlayerFilter::IteratedPlayer
    ) || !untap_target_exactly_matches_choice(untap, choose)
    {
        return None;
    }

    let chosen = describe_choose_selection(choose);
    let chosen_noun = pluralize_noun_phrase(choose_reference_noun(choose));
    Some((
        format!("that player chooses {chosen}, then untaps those {chosen_noun}"),
        2,
    ))
}

/// Preserve a participant as the actor of both an object choice and the
/// immediately correlated return. The tag proves that the returned set is
/// exactly the one that participant chose; neither the chooser nor the
/// ownership pronoun should be inferred from an unrelated later move.
fn describe_opponent_choose_then_return_chosen(effects: &[Effect]) -> Option<(String, usize)> {
    let [choose_effect, return_effect, ..] = effects else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let return_to_hand = structural_unwrap_render_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnToHandEffect>()?;
    if choose.chooser != PlayerFilter::Opponent
        || choose.is_search
        || choose.reveal
        || choose.filter.controller != Some(PlayerFilter::Opponent)
        || !matches!(&return_to_hand.spec, ChooseSpec::All(_))
        || !choose_spec_references_exact_tag(&return_to_hand.spec, &choose.tag)
        || return_to_hand.actor_surface.is_some()
        || return_to_hand.destination_player_surface.is_some()
        || return_to_hand.exiled_with_source_surface.is_some()
        || return_to_hand.set_quantifier_surface.is_some()
        || return_to_hand.set_reference_surface.is_some()
    {
        return None;
    }

    let selection = describe_choose_selection(choose)
        .replace(" an opponent controls", " they control")
        .replace(" an opponent owns", " they own");
    let (object_pronoun, owner_pronoun) = if choose.count.is_single() {
        ("it", "its")
    } else {
        ("them", "their")
    };
    Some((
        format!(
            "An opponent chooses {selection} and returns {object_pronoun} to {owner_pronoun} owner's hand"
        ),
        2,
    ))
}

#[cfg(test)]
mod opponent_choose_then_return_chosen_tests {
    use super::*;

    fn effects(return_tag: &str, chooser: PlayerFilter) -> Vec<Effect> {
        let chosen = TagKey::from("opponent_chosen");
        vec![
            Effect::new(crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::creature()
                    .in_zone(Zone::Battlefield)
                    .controlled_by(PlayerFilter::Opponent),
                crate::effect::ChoiceCount::up_to(2),
                chooser,
                chosen,
            )),
            Effect::new(crate::effects::ReturnToHandEffect::all(
                ObjectFilter::tagged(TagKey::from(return_tag)),
            )),
        ]
    }

    #[test]
    fn refreshed_instead_opponent_choice_keeps_actor_and_chosen_set_correlation() {
        assert_eq!(
            describe_opponent_choose_then_return_chosen(&effects(
                "opponent_chosen",
                PlayerFilter::Opponent,
            )),
            Some((
                "An opponent chooses up to two creatures they control and returns them to their owner's hand"
                    .to_string(),
                2,
            ))
        );
    }

    #[test]
    fn refreshed_instead_different_chooser_or_returned_tag_does_not_fold() {
        assert!(
            describe_opponent_choose_then_return_chosen(&effects(
                "different",
                PlayerFilter::Opponent,
            ))
            .is_none()
        );
        assert!(
            describe_opponent_choose_then_return_chosen(&effects(
                "opponent_chosen",
                PlayerFilter::You,
            ))
            .is_none()
        );
    }
}

/// Render an optional compound payment as the single conjunction represented
/// by its typed cost-capable children. Sentence lowering wraps some authored
/// `and` chains in a sequential sequence so their result can be referenced by
/// "if you do"; that wrapper must not turn one optional payment into multiple
/// independent sentences.
pub(super) fn describe_may_compound_payment(may: &crate::effects::MayEffect) -> Option<String> {
    let decider = may.decider.clone().unwrap_or(PlayerFilter::You);

    let members = if let [effect] = may.effects.as_slice()
        && let Some(sequence) = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::SequenceEffect>()
        && matches!(
            sequence.surface,
            ironsmith_core::SequenceSurface::Sequential
                | ironsmith_core::SequenceSurface::Coordinated
        )
        && sequence.effects.iter().all(|effect| {
            let effect = structural_unwrap_render_wrappers(effect);
            (effect.0.as_cost_executable().is_some()
                || effect
                    .downcast_ref::<crate::effects::ChooseObjectsEffect>()
                    .is_some())
                && effect.downcast_ref::<crate::effects::MayEffect>().is_none()
                && effect
                    .downcast_ref::<crate::effects::SequenceEffect>()
                    .is_none()
        }) {
        sequence.effects.as_slice()
    } else {
        may.effects.as_slice()
    };
    if members.is_empty() {
        return None;
    }

    let mut typed_payments = 0;
    let mut previous_was_typed_payment = false;
    let mut parts = Vec::with_capacity(members.len());
    let mut member_index = 0;
    while member_index < members.len() {
        let member = structural_unwrap_render_wrappers(&members[member_index]);

        // Choosing the permanent is execution scaffolding for a sacrifice
        // cost, not a separately authored action. Preserve that distinction
        // only when the typed choice and sacrifice are adjacent, refer to the
        // same tagged set, and belong to the optional payment's decider.
        if decider == PlayerFilter::You
            && let Some(choose) = member.downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(sacrifice_effect) = members.get(member_index + 1)
            && let Some(sacrifice) =
                sacrifice_view(structural_unwrap_render_wrappers(sacrifice_effect))
            && choose.chooser == decider
            && sacrifice.player == &decider
            && let Some(compact) = describe_choose_then_sacrifice(choose, sacrifice)
            && let Some(action) = compact.strip_prefix("you ")
        {
            parts.push(action.to_string());
            previous_was_typed_payment = false;
            member_index += 2;
            continue;
        }

        let rendered = if let Some(pay) = member.downcast_ref::<crate::effects::PayManaEffect>() {
            if pay.player != ChooseSpec::Player(decider.clone()) {
                return None;
            }
            typed_payments += 1;
            let payment = describe_pay_mana_cost(pay);
            if previous_was_typed_payment {
                payment
            } else {
                format!("pay {payment}")
            }
        } else if let Some(pay) = member.downcast_ref::<crate::effects::PayLifeEffect>() {
            if pay.player != ChooseSpec::Player(decider.clone()) {
                return None;
            }
            typed_payments += 1;
            let payment = describe_life_amount_phrase(&pay.amount);
            if previous_was_typed_payment {
                payment
            } else {
                format!("pay {payment}")
            }
        } else if member.0.as_cost_executable().is_none()
            || member.downcast_ref::<crate::effects::MayEffect>().is_some()
            || member
                .downcast_ref::<crate::effects::SequenceEffect>()
                .is_some()
            || decider != PlayerFilter::You
        {
            return None;
        } else {
            describe_effect(member)
        };
        let is_typed_payment = member
            .downcast_ref::<crate::effects::PayManaEffect>()
            .is_some()
            || member
                .downcast_ref::<crate::effects::PayLifeEffect>()
                .is_some();
        previous_was_typed_payment = is_typed_payment;
        let rendered = rendered.trim().trim_end_matches('.');
        if rendered.is_empty() || rendered.contains(". ") {
            return None;
        }
        let rendered = rendered
            .strip_prefix("You ")
            .or_else(|| rendered.strip_prefix("you "))
            .unwrap_or(rendered);
        parts.push(lowercase_first(rendered));
        member_index += 1;
    }

    (typed_payments > 0).then(|| {
        let participant = if decider == PlayerFilter::Active {
            "the active player".to_string()
        } else {
            describe_player_filter(&decider)
        };
        format!("{} may {}", participant, join_with_and(&parts))
    })
}

#[cfg(test)]
mod compound_optional_payment_surface_tests {
    use super::*;

    #[test]
    fn sequential_nonmana_and_mana_costs_remain_one_optional_conjunction() {
        let payment = Effect::new(crate::effects::SequenceEffect::new(vec![
            Effect::sacrifice_source(),
            Effect::new(crate::effects::PayManaEffect::new(
                crate::mana::ManaCost::from_symbols(vec![
                    ManaSymbol::Generic(2),
                    ManaSymbol::Green,
                    ManaSymbol::Green,
                ]),
                ChooseSpec::Player(PlayerFilter::You),
            )),
        ]));
        let may = crate::effects::MayEffect::new_for_player(vec![payment], PlayerFilter::You);

        assert_eq!(
            describe_may_compound_payment(&may).as_deref(),
            Some("you may sacrifice this source and pay {2}{G}{G}")
        );
    }

    #[test]
    fn coordinated_direct_costs_share_one_optional_subject() {
        let payment = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::sacrifice_source(),
            Effect::new(crate::effects::PayManaEffect::new(
                crate::mana::ManaCost::from_symbols(vec![
                    ManaSymbol::Generic(2),
                    ManaSymbol::Green,
                    ManaSymbol::Green,
                ]),
                ChooseSpec::Player(PlayerFilter::You),
            )),
        ]));
        let may = crate::effects::MayEffect::new_for_player(vec![payment], PlayerFilter::You);

        assert_eq!(
            describe_may_compound_payment(&may).as_deref(),
            Some("you may sacrifice this source and pay {2}{G}{G}")
        );
    }

    #[test]
    fn coordinated_mana_and_choose_sacrifice_hide_choice_scaffolding() {
        let tag = TagKey::from("sacrificed_0");
        let payment = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::new(crate::effects::PayManaEffect::new(
                crate::mana::ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]),
                ChooseSpec::Player(PlayerFilter::You),
            )),
            Effect::new(crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::artifact()
                    .you_control()
                    .in_zone(Zone::Battlefield),
                ChoiceCount::exactly(1),
                PlayerFilter::You,
                tag.clone(),
            )),
            Effect::sacrifice_player(ObjectFilter::tagged(tag), 1, PlayerFilter::You),
        ]));
        let may = crate::effects::MayEffect::new_for_player(vec![payment], PlayerFilter::You);

        assert_eq!(
            describe_may_compound_payment(&may).as_deref(),
            Some("you may pay {1} and sacrifice an artifact")
        );
    }

    #[test]
    fn active_player_single_life_payment_keeps_participant_and_payment_surface() {
        let may = crate::effects::MayEffect::new_for_player(
            vec![Effect::pay_life_player(2, PlayerFilter::Active)],
            PlayerFilter::Active,
        );

        assert_eq!(
            describe_may_compound_payment(&may).as_deref(),
            Some("the active player may pay 2 life")
        );
    }

    #[test]
    fn adjacent_mana_and_life_payments_share_the_pay_verb() {
        let may = crate::effects::MayEffect::new(vec![
            Effect::new(crate::effects::PayManaEffect::new(
                crate::mana::ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]),
                ChooseSpec::Player(PlayerFilter::You),
            )),
            Effect::pay_life(3),
        ]);

        assert_eq!(
            describe_may_compound_payment(&may).as_deref(),
            Some("you may pay {1} and 3 life")
        );
    }
}

pub(crate) fn describe_action_and_get_energy_pair(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let energy = structural_unwrap_render_wrappers(second)
        .downcast_ref::<crate::effects::EnergyCountersEffect>()?;
    if energy.player != PlayerFilter::You {
        return None;
    }
    let energy_text = describe_effect(second);
    let energy_amount = energy_text
        .trim()
        .trim_end_matches('.')
        .strip_prefix("you get ")?;

    let first = structural_unwrap_render_wrappers(first);
    if let Some(gain) = first.downcast_ref::<crate::effects::GainLifeEffect>() {
        let actor = choose_spec_player_filter(&gain.player)?;
        if actor != PlayerFilter::You {
            return None;
        }
        return Some(format!(
            "you gain {} and get {energy_amount}",
            describe_life_amount_phrase(&gain.amount)
        ));
    }
    if let Some(draw) = first.downcast_ref::<crate::effects::DrawCardsEffect>() {
        if draw.player != PlayerFilter::You {
            return None;
        }
        return Some(format!(
            "Draw {} and you get {energy_amount}",
            describe_card_count(&draw.count)
        ));
    }

    let first_text = describe_effect(first)
        .trim()
        .trim_end_matches('.')
        .to_string();
    if first_text.is_empty() || first_text.contains(". ") {
        return None;
    }
    if let Some(mill) = first.downcast_ref::<crate::effects::MillEffect>() {
        return Some(if mill.player == PlayerFilter::You {
            format!("{first_text} and get {energy_amount}")
        } else {
            format!("{first_text} and you get {energy_amount}")
        });
    }
    if first
        .downcast_ref::<crate::effects::ReturnToHandEffect>()
        .is_some_and(|return_to_hand| matches!(return_to_hand.spec.base(), ChooseSpec::Source))
    {
        return Some(format!("{first_text} and you get {energy_amount}"));
    }
    None
}

fn linked_counter_followup_surface(
    put: &crate::effects::PutCountersEffect,
) -> Option<ValueSurfaceHint> {
    [
        ValueSurfaceHint::CounterFollowupThen,
        ValueSurfaceHint::CounterFollowupSeparateSentence,
    ]
    .into_iter()
    .find(|hint| put.amount.has_surface_hint(*hint))
}

fn effect_outer_id(effect: &Effect) -> Option<crate::effect::EffectId> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return Some(with_id.id);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return effect_outer_id(&tagged.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return effect_outer_id(&tag_all.effect);
    }
    None
}

fn affected_object_characteristic(
    value: &Value,
    expected_id: crate::effect::EffectId,
) -> Option<&'static str> {
    let Value::EffectMetric {
        effect_id,
        source: crate::effect::EffectMetricSource::AffectedObjects,
        metric,
    } = value.unhinted()
    else {
        return None;
    };
    if *effect_id != expected_id {
        return None;
    }
    match metric {
        crate::effect::EffectMetric::FirstPower => Some("power"),
        crate::effect::EffectMetric::FirstToughness => Some("toughness"),
        crate::effect::EffectMetric::FirstManaValue => Some("mana value"),
        _ => None,
    }
}

fn replace_affected_object_characteristic_reference(
    text: &str,
    characteristic: &str,
    antecedent: &str,
) -> String {
    [
        format!("that creature's {characteristic}"),
        format!("that card's {characteristic}"),
        format!("its {characteristic}"),
    ]
    .into_iter()
    .find_map(|surface| {
        text.contains(&surface)
            .then(|| text.replacen(&surface, antecedent, 1))
    })
    .unwrap_or_else(|| text.to_string())
}

fn tagged_characteristic_reference(value: &Value, expected_tag: &TagKey) -> Option<&'static str> {
    let (spec, characteristic) = match value.unhinted() {
        Value::PowerOf(spec) => (spec, "power"),
        Value::ToughnessOf(spec) => (spec, "toughness"),
        Value::ManaValueOf(spec) => (spec, "mana value"),
        _ => return None,
    };
    matches!(spec.base(), ChooseSpec::Tagged(tag) if tag == expected_tag).then_some(characteristic)
}

fn returned_object_reference_noun(spec: &ChooseSpec) -> &'static str {
    let filter = match spec.base() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter,
        _ => return "permanent",
    };
    if filter.card_types.contains(&CardType::Creature) {
        "creature"
    } else if filter.card_types.contains(&CardType::Artifact) {
        "artifact"
    } else if filter.card_types.contains(&CardType::Enchantment) {
        "enchantment"
    } else if filter.card_types.contains(&CardType::Planeswalker) {
        "planeswalker"
    } else if filter.card_types.contains(&CardType::Battle) {
        "battle"
    } else if filter.card_types.contains(&CardType::Land) {
        "land"
    } else {
        "permanent"
    }
}

fn describe_linked_counter_followup(effects: &[Effect]) -> Option<String> {
    let effects = match effects {
        [target, tail @ ..]
            if target
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some() =>
        {
            tail
        }
        _ => effects,
    };
    let [first_effect, put_effect] = effects else {
        return None;
    };
    let put = structural_unwrap_render_wrappers(put_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.distributed {
        return None;
    }
    let surface = linked_counter_followup_surface(put)?;
    let tagged_first_effect = if let Some(may) =
        first_effect.downcast_ref::<crate::effects::MayEffect>()
        && let [effect] = may.effects.as_slice()
    {
        effect
    } else {
        first_effect
    };
    let first_tag = effect_outer_tag(tagged_first_effect)?;

    let first = structural_unwrap_render_wrappers(tagged_first_effect);
    let mut put_text = describe_effect(put_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    if first
        .downcast_ref::<crate::effects::CreateTokenEffect>()
        .is_some()
    {
        if !matches!(put.target.base(), ChooseSpec::Tagged(tag) if tag == first_tag) {
            return None;
        }
    } else if let Some(return_to_hand) = first.downcast_ref::<crate::effects::ReturnToHandEffect>()
    {
        let characteristic = tagged_characteristic_reference(&put.amount, first_tag)?;
        let antecedent = format!(
            "that {}'s {characteristic}",
            returned_object_reference_noun(&return_to_hand.spec)
        );
        put_text = put_text.replacen(&format!("its {characteristic}"), &antecedent, 1);
    } else if let Some(move_to_zone) = first.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        if !move_to_zone_is_plain_exile(move_to_zone) {
            return None;
        }
        let characteristic =
            affected_object_characteristic(&put.amount, effect_outer_id(first_effect)?)?;
        let antecedent = match move_to_zone.target.base() {
            ChooseSpec::Object(filter) if filter.zone == Some(Zone::Graveyard) => {
                format!("the {characteristic} of the card you exiled")
            }
            ChooseSpec::Object(filter) if filter.card_types.as_slice() == [CardType::Creature] => {
                format!("the {characteristic} of the creature exiled this way")
            }
            _ => format!("the {characteristic} of the permanent exiled this way"),
        };
        put_text = replace_affected_object_characteristic_reference(
            &put_text,
            characteristic,
            &antecedent,
        );
    } else if first
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
        .is_some()
    {
        if !choose_spec_is_tagged_object(&put.target, first_tag) {
            return None;
        }
    } else if first
        .downcast_ref::<crate::effects::ExileEffect>()
        .is_some()
    {
        let Value::Count(filter) = put.amount.unhinted() else {
            return None;
        };
        if !matches!(put.target.base(), ChooseSpec::Source)
            || !filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag == *first_tag
                    && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            })
        {
            return None;
        }
    } else {
        return None;
    }

    let first_text = describe_effect(first_effect);
    let first_text = first_text.trim().trim_end_matches('.');
    Some(match surface {
        ValueSurfaceHint::CounterFollowupThen => {
            format!("{first_text}, then {}", lowercase_first(&put_text))
        }
        ValueSurfaceHint::CounterFollowupSeparateSentence => {
            format!("{first_text}. {}", capitalize_first(&put_text))
        }
        _ => return None,
    })
}

/// Honor the sentence boundary carried on a counter-placement value after
/// lowering. The split is metadata-driven: effects without the explicit
/// surface hint retain the ordinary coordination and compaction paths.
fn describe_typed_counter_sentence_split(effects: &[Effect]) -> Option<String> {
    let is_sentence_start = |effect: &Effect| {
        structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::PutCountersEffect>()
            .is_some_and(|put| {
                linked_counter_followup_surface(put)
                    == Some(ValueSurfaceHint::CounterFollowupSeparateSentence)
            })
    };

    // Every counter effect produced by one authored sentence carries the
    // same hint. Once a recursively rendered suffix starts at that sentence,
    // do not split its coordinated counter list again.
    if effects
        .first()
        .is_some_and(|effect| is_sentence_start(effect))
    {
        return None;
    }
    let split = effects
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(idx, effect)| is_sentence_start(effect).then_some(idx))?;

    let first = describe_effect_list(&effects[..split]);
    let second = describe_effect_list(&effects[split..]);
    if first.trim().is_empty() || second.trim().is_empty() {
        return None;
    }
    Some(format!(
        "{}. {}",
        first.trim().trim_end_matches('.'),
        capitalize_first(second.trim().trim_end_matches('.'))
    ))
}

fn counter_producer_object_filter(spec: &ChooseSpec) -> Option<&ObjectFilter> {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _)
        | ChooseSpec::WithCountValue(spec, _, _) => counter_producer_object_filter(spec),
        ChooseSpec::Object(filter) => Some(filter),
        _ => None,
    }
}

/// Render a counter-producing action and its linked quoted grant while the
/// typed producer target is still available. The counter may be placed
/// directly or supplied as part of returning the object to the battlefield.
/// The tagged follow-up intentionally stores identity rather than duplicating
/// the original target filter, so a standalone renderer cannot know whether
/// the granted ability's self is a land, creature, or another permanent kind.
pub(in crate::compiled_text) fn describe_counter_linked_grant_after_put(
    producer_effect: &Effect,
    grant_effect: &Effect,
) -> Option<String> {
    let tagged_producer = producer_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    if !is_implicit_reference_tag(tagged_producer.tag.as_str()) {
        return None;
    }
    let (producer_counter, filter) = if let Some(put) = tagged_producer
        .effect
        .downcast_ref::<crate::effects::PutCountersEffect>(
    ) {
        if put.distributed || put.target_count.is_some() {
            return None;
        }
        (
            &put.counter_type,
            counter_producer_object_filter(&put.target)?,
        )
    } else if let Some(returned) = tagged_producer
        .effect
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>(
    ) {
        let [entry_counter] = returned.enters_with_counters.as_slice() else {
            return None;
        };
        if entry_counter.amount != Value::Fixed(1) || entry_counter.condition.is_some() {
            return None;
        }
        (
            &entry_counter.counter_type,
            counter_producer_object_filter(&returned.target)?,
        )
    } else {
        return None;
    };

    let grant = structural_unwrap_render_wrappers(grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let Until::ForAsLongAs(ironsmith_core::ContinuousDurationPredicate::ObjectHasCounter {
        object: ironsmith_core::ContinuousDurationObject::AffectedObject,
        counter_type: duration_counter,
        minimum: 1,
    }) = &grant.until
    else {
        return None;
    };
    if duration_counter != producer_counter
        || grant.condition.is_some()
        || !grant.additional_modifications.is_empty()
        || !grant.runtime_modifications.is_empty()
        || !matches!(
            &grant.modification,
            Some(crate::continuous::Modification::AddAbilityGeneric(_))
        )
        || !grant
            .target_spec
            .as_ref()
            .is_some_and(|spec| choose_spec_references_tag(spec, tagged_producer.tag.as_str()))
    {
        return None;
    }

    let self_subject = granted_ability_self_subject_for_filter(filter);
    let object_kind = self_subject.strip_prefix("this ")?;
    let clauses = describe_apply_continuous_clauses_with_self_subject(grant, false, self_subject);
    let [grant_clause] = clauses.as_slice() else {
        return None;
    };
    let granted_ability = grant_clause.strip_prefix("has ")?;
    let counter = with_indefinite_article(&format!("{} counter", producer_counter.description()));
    let producer_text = describe_effect(producer_effect);

    Some(format!(
        "{}. For as long as that {object_kind} has {counter} on it, it has {granted_ability}",
        producer_text.trim().trim_end_matches('.')
    ))
}

fn describe_exile_top_play_then_additional_land(effects: &[Effect]) -> Option<String> {
    let [exile_effect, grant_effect, land_effect] = effects else {
        return None;
    };
    let exile = structural_unwrap_render_wrappers(exile_effect)
        .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    let grant = structural_unwrap_render_wrappers(grant_effect)
        .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    let land = structural_unwrap_render_wrappers(land_effect)
        .downcast_ref::<crate::effects::AdditionalLandPlaysEffect>()?;
    let prefix = describe_exile_top_then_play(exile, grant, false)?;
    if land.player != PlayerFilter::You || land.duration != Until::EndOfTurn {
        return None;
    }
    let land = capitalize_first(describe_effect(land_effect).trim_end_matches('.'));
    Some(format!("{}. {land}", prefix.trim_end_matches('.')))
}

fn describe_hidden_exile_partition_with_persistent_permission(
    effects: &[Effect],
) -> Option<String> {
    let complete_effects = effects.iter().collect::<Vec<_>>();
    if let Some(compact) =
        describe_target_opponent_look_exile_one_rest_bottom_cast(&complete_effects)
    {
        // Preserve the target declaration while the exact recognizer validates
        // the target/look/remainder relationship. Stripping TargetOnly first
        // leaves the generic renderer unable to distinguish "that library"
        // or the singular tagged card from a broad exiled-card collection.
        return Some(compact);
    }

    let effects = match effects {
        [target, tail @ ..]
            if target
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some() =>
        {
            tail
        }
        _ => effects,
    };
    let (look_effect, choose_effect, exile_effect, remainder_effect, look_permission, grant_effect) =
        match effects {
            [look, choose, exile, remainder, grant] => {
                (look, choose, exile, remainder, None, grant)
            }
            [look, choose, exile, remainder, look_permission, grant] => {
                (look, choose, exile, remainder, Some(look_permission), grant)
            }
            _ => return None,
        };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let exile = structural_unwrap_render_wrappers(exile_effect)
        .downcast_ref::<crate::effects::ExileEffect>()?;
    let remainder = remainder_effect
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    let look_permission = match look_permission {
        Some(effect) => Some(effect.downcast_ref::<crate::effects::LookAtObjectsEffect>()?),
        None => None,
    };
    let grant = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    describe_look_at_top_choose_exile_face_down_rest_bottom_then_play_while_exiled(
        look,
        choose,
        exile,
        remainder,
        look_permission,
        grant,
    )
}

fn describe_each_opponent_top_card_hidden_exile_permission(effects: &[Effect]) -> Option<String> {
    let [players_effect, permission_effect] = effects else {
        return None;
    };
    let players = players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if players.filter != PlayerFilter::Opponent
        || players.starting_with_controller
        || players.stop_after_first_happened
    {
        return None;
    }
    let [choose_effect, exile_effect] = players.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::You
        || !choose.count.is_single()
        || choose_primary_zone(choose) != Some(Zone::Library)
        || !choose.top_only
        || choose.bottom_only
        || choose.filter.owner != Some(PlayerFilter::IteratedPlayer)
        || choose.is_search
        || choose.reveal
    {
        return None;
    }

    let collection_tag = effect_outer_tag(exile_effect)?;
    let exile = structural_unwrap_render_wrappers(exile_effect)
        .downcast_ref::<crate::effects::ExileEffect>()?;
    if !exile.face_down
        || !matches!(exile.spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
    {
        return None;
    }

    let permission = permission_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    if permission.tag != *collection_tag
        || permission.player != PlayerFilter::You
        || permission.duration != crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled
        || !permission.allow_land
        || permission.mana_spend_mode != ironsmith_core::value_model::ManaSpendMode::Normal
        || permission.while_on_top_of_library
        || permission.filter.is_some()
    {
        return None;
    }

    Some(
        "Exile the top card of each opponent's library face down. You may look at and play those cards for as long as they remain exiled"
            .to_string(),
    )
}

fn describe_exile_all_then_each_player_may_deploy_and_return_exiled(
    effects: &[Effect],
) -> Option<String> {
    let [
        exile_effect,
        players_effect,
        return_effect,
        source_exile_effect,
    ] = effects
    else {
        return None;
    };

    let exiled_tag = effect_outer_tag(exile_effect)?;
    let exile = structural_unwrap_render_wrappers(exile_effect)
        .downcast_ref::<crate::effects::ExileEffect>()?;
    if exile.face_down
        || !matches!(exile.spec.base(), ChooseSpec::All(filter) if filter == &ObjectFilter::creature())
    {
        return None;
    }

    let players = structural_unwrap_render_wrappers(players_effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if players.filter != PlayerFilter::Any
        || players.starting_with_controller
        || players.stop_after_first_happened
    {
        return None;
    }
    let [may_effect] = players.effects.as_slice() else {
        return None;
    };
    let may = structural_unwrap_render_wrappers(may_effect)
        .downcast_ref::<crate::effects::MayEffect>()?;
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| decider != &PlayerFilter::IteratedPlayer)
    {
        return None;
    }
    let [choose_effect, deploy_effect] = may.effects.as_slice() else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::IteratedPlayer
        || choose.count != ChoiceCount::any_number()
        || choose.count_value.is_some()
        || choose.zone != Some(Zone::Hand)
        || !choose.additional_zones.is_empty()
        || choose.filter.card_types != [CardType::Creature]
        || choose.filter.owner != Some(PlayerFilter::IteratedPlayer)
        || choose.is_search
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
    {
        return None;
    }
    let deploy = structural_unwrap_render_wrappers(deploy_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if deploy.zone != Zone::Battlefield
        || !choose_spec_is_tagged_object(&deploy.target, &choose.tag)
        || deploy.battlefield_controller != crate::effects::BattlefieldController::Preserve
        || deploy.enters_tapped
        || deploy.enters_attacking
        || deploy.enters_face_down
    {
        return None;
    }

    let return_exiled = structural_unwrap_render_wrappers(return_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if return_exiled.zone != Zone::Hand
        || !choose_spec_is_tagged_object(&return_exiled.target, exiled_tag)
        || return_exiled.to_top
    {
        return None;
    }

    let source_exile = structural_unwrap_render_wrappers(source_exile_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !move_to_zone_is_plain_exile(source_exile)
        || !matches!(source_exile.target.base(), ChooseSpec::Source)
    {
        return None;
    }
    let source_exile_text = describe_effect(source_exile_effect);
    let source_exile_text = source_exile_text.trim().trim_end_matches('.');
    let source_exile_text = source_exile_text
        .strip_prefix("You ")
        .or_else(|| source_exile_text.strip_prefix("you "))
        .unwrap_or(source_exile_text);
    if !source_exile_text.to_ascii_lowercase().starts_with("exile ") {
        return None;
    }

    Some(format!(
        "Exile all creatures. Each player may put any number of creature cards from their hand onto the battlefield. Then put all cards exiled this way into their owners' hands. {}",
        capitalize_first(source_exile_text)
    ))
}

fn describe_look_hand_optional_exile_persistent_play_tax(effects: &[Effect]) -> Option<String> {
    fn face_up_exile_spec(effect: &Effect) -> Option<&ChooseSpec> {
        let effect = structural_unwrap_render_wrappers(effect);
        if let Some(exile) = effect.downcast_ref::<crate::effects::ExileEffect>() {
            return (!exile.face_down).then_some(&exile.spec);
        }
        let move_to_zone = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
        move_to_zone_is_plain_exile(move_to_zone).then_some(&move_to_zone.target)
    }

    fn single_object_filter(spec: &ChooseSpec) -> Option<&ObjectFilter> {
        match spec {
            ChooseSpec::SurfaceHinted { spec, .. } => single_object_filter(spec),
            ChooseSpec::WithCount(spec, count) if count.is_single() => single_object_filter(spec),
            ChooseSpec::Object(filter) => Some(filter),
            _ => None,
        }
    }

    let [look_effect, may_effect, permission_effect, tax_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    if look.reveal || !is_target_opponent_spec(&look.target) {
        return None;
    }
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if !may
        .decider
        .as_ref()
        .is_none_or(|decider| decider == &PlayerFilter::You)
    {
        return None;
    }
    let (exile_tag, filter) = match may.effects.as_slice() {
        [exile_effect] => {
            let exile_tag = structural_effect_tag(exile_effect)
                .cloned()
                .unwrap_or_else(|| TagKey::from(crate::tag::SOURCE_EXILED_TAG));
            (
                exile_tag,
                single_object_filter(face_up_exile_spec(exile_effect)?)?,
            )
        }
        [choose_effect, exile_effect] => {
            let choose = structural_unwrap_render_wrappers(choose_effect)
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            let exile_spec = face_up_exile_spec(exile_effect)?;
            if choose.chooser != PlayerFilter::You
                || !choose.count.is_single()
                || choose_primary_zone(choose) != Some(Zone::Hand)
                || !matches!(exile_spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
            {
                return None;
            }
            (choose.tag.clone(), &choose.filter)
        }
        _ => return None,
    };
    if filter.zone != Some(Zone::Hand)
        || !matches!(&filter.owner, None | Some(PlayerFilter::Target(_)))
        || !filter.card_types.is_empty()
        || filter.excluded_card_types != [CardType::Land]
    {
        return None;
    }

    let permission = permission_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    if permission.tag != exile_tag
        || permission.duration != crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled
        || !permission.allow_land
        || permission.allow_any_color_for_cast
        || !matches!(
            &permission.player,
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(tag)) if tag == &exile_tag
        )
    {
        return None;
    }

    let tax = tax_effect.downcast_ref::<crate::effects::GrantEffect>()?;
    if tax.duration != crate::grant::GrantDuration::Forever
        || !matches!(&tax.target, ChooseSpec::Tagged(tag) if tag == &exile_tag)
    {
        return None;
    }
    let crate::grant::Grantable::Ability(ability) = &tax.grantable else {
        return None;
    };
    let cost_increase = ability.cost_increase_mana_cost()?;
    if cost_increase.filter.stack_kind != Some(crate::filter::StackObjectKind::Spell)
        || cost_increase.filter.cast_by.is_some()
    {
        return None;
    }

    Some(format!(
        "Look at target opponent's hand. You may exile a nonland card from it. For as long as that card remains exiled, its owner may play it. A spell cast this way costs {} more to cast",
        cost_increase.increase.to_oracle()
    ))
}

fn describe_target_exile_persistent_owner_play_tax(effects: &[Effect]) -> Option<String> {
    let [exile_effect, permission_effect, tax_effect] = effects else {
        return None;
    };
    let exile_tag = structural_effect_tag(exile_effect)?;
    let exile = structural_unwrap_render_wrappers(exile_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !move_to_zone_is_plain_exile(exile) || !exile.target.is_target() {
        return None;
    }

    let permission = permission_effect.downcast_ref::<crate::effects::GrantBySpecEffect>()?;
    if !matches!(
        &permission.spec.grantable,
        crate::grant::Grantable::PlayFrom
    ) || permission.spec.zone != Zone::Exile
        || permission.spec.beneficiary != PlayerFilter::You
        || permission.spec.usage_limit.is_some()
        || !permission.spec.cast_this_way_grants.is_empty()
        || permission.spec.cast_this_way_filter.is_some()
        || permission.duration != crate::grant::GrantDuration::Forever
        || !matches!(
            &permission.player,
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(tag)) if tag == exile_tag
        )
    {
        return None;
    }
    let [permission_constraint] = permission.spec.filter.tagged_constraints.as_slice() else {
        return None;
    };
    if permission_constraint.tag != *exile_tag
        || permission_constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
    {
        return None;
    }
    let mut permission_filter = permission.spec.filter.clone();
    permission_filter.tagged_constraints.clear();
    if permission_filter != ObjectFilter::default() {
        return None;
    }

    let tax = structural_unwrap_render_wrappers(tax_effect)
        .downcast_ref::<crate::effects::GrantEffect>()?;
    if tax.duration != crate::grant::GrantDuration::Forever
        || !matches!(&tax.target, ChooseSpec::Tagged(tag) if tag == exile_tag)
        || !matches!(&tax.grantable, crate::grant::Grantable::Ability(ability)
            if ability.cost_increase_mana_cost().is_some())
    {
        return None;
    }

    let exile_text = describe_effect(exile_effect);
    let exile_text = exile_text.trim().trim_end_matches('.');
    let tax_text = describe_effect(tax_effect);
    let tax_text = tax_text.trim().trim_end_matches('.');
    if !tax_text.starts_with("A spell cast ") || !tax_text.contains(" this way costs ") {
        return None;
    }
    Some(format!(
        "{exile_text}. For as long as that card remains exiled, its owner may play it. {tax_text}"
    ))
}

fn describe_discard_redraw_mana_value_ladder(effects: &[Effect]) -> Option<String> {
    fn is_artifact_or_creature_filter(filter: &ObjectFilter) -> bool {
        let mut types = filter.card_types.clone();
        for branch in &filter.any_of {
            if branch.card_types.len() != 1 || !branch.any_of.is_empty() {
                return false;
            }
            types.extend(branch.card_types.iter().copied());
        }
        types.len() == 2
            && types.contains(&CardType::Artifact)
            && types.contains(&CardType::Creature)
    }

    let [
        discard_effect,
        draw_effect,
        first,
        second,
        third,
        return_effect,
    ] = effects
    else {
        return None;
    };
    let with_id = discard_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let discard = with_id
        .effect
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    let discarded_tag = discard.tag.as_ref()?;
    if discard.player != PlayerFilter::You
        || discard.random
        || discard.any_number
        || discard.card_filter.is_some()
        || !discard
            .count
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::AllCardsInHand)
    {
        return None;
    }
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You
        || !value_is_discarded_count_for_effect(&draw.count, with_id.id)
    {
        return None;
    }

    let choices = [
        first.downcast_ref::<crate::effects::ChooseObjectsEffect>()?,
        second.downcast_ref::<crate::effects::ChooseObjectsEffect>()?,
        third.downcast_ref::<crate::effects::ChooseObjectsEffect>()?,
    ];
    let selected_tag = &choices[0].tag;
    for (index, choice) in choices.iter().enumerate() {
        if choice.chooser != PlayerFilter::You
            || choice.count.min != 0
            || choice.count.max != Some(1)
            || &choice.tag != selected_tag
            || choose_primary_zone(choice) != Some(Zone::Graveyard)
            || choice.filter.owner != Some(PlayerFilter::You)
            || !is_artifact_or_creature_filter(&choice.filter)
            || choice.filter.mana_value
                != Some(crate::filter::Comparison::Equal((index + 1) as i32))
            || !object_filter_has_tag(&choice.filter, discarded_tag)
        {
            return None;
        }
    }

    let return_to_battlefield = return_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if return_to_battlefield.zone != Zone::Battlefield
        || return_to_battlefield.enters_tapped
        || return_to_battlefield.enters_attacking
        || !matches!(&return_to_battlefield.target, ChooseSpec::Tagged(tag) if tag == selected_tag)
    {
        return None;
    }

    Some(
        "Discard all the cards in your hand, then draw that many cards. You may choose an artifact or creature card with mana value 1 you discarded this way, then do the same for artifact or creature cards with mana values 2 and 3. Return those cards to the battlefield"
            .to_string(),
    )
}

fn describe_exile_top_choose_one_play_next_turn(effects: &[Effect]) -> Option<String> {
    let [look_effect, move_effect, choose_effect, grant_effect] = effects else {
        return None;
    };
    let look = structural_unwrap_render_wrappers(look_effect)
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let move_to_exile = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let grant = structural_unwrap_render_wrappers(grant_effect)
        .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    if look.reveal
        || !tagged_move_to_zone(move_to_exile, &look.tag, Zone::Exile, move_to_exile.to_top)
        || move_to_exile.enters_face_down
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Exile)
        || !choose.additional_zones.is_empty()
        || choose_exact_count(choose) != Some(1)
        || choose.is_search
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
        || choose.replace_tagged_objects
        || !filter_is_exactly_tagged_in_zone(&choose.filter, &look.tag, Zone::Exile)
        || grant.tag != choose.tag
        || grant.player != PlayerFilter::You
        || grant.allow_any_color_for_cast
        || grant.while_on_top_of_library
        || grant.filter.is_some()
        || grant.cast_pool_is_plural
    {
        return None;
    }
    let duration = match grant.duration {
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn => "Until end of turn",
        crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd => {
            "Until the end of your next turn"
        }
        crate::effects::GrantPlayTaggedDuration::UntilYourNextEndStep => "Until your next end step",
        _ => return None,
    };
    let owner = describe_possessive_player_filter(&look.player);
    let verb = if grant.allow_land { "play" } else { "cast" };
    let exile_clause = if value_prefers_equal_to(&look.count) {
        let amount = look
            .count
            .clone()
            .without_surface_hint(ValueSurfaceHint::EqualTo);
        format!(
            "Exile a number of cards from the top of {owner} library equal to {}",
            describe_value(&amount)
        )
    } else {
        format!(
            "Exile {} from the top of {owner} library",
            describe_card_count(&look.count)
        )
    };
    Some(format!(
        "{exile_clause}, then choose a card exiled this way. {duration}, you may {verb} that card"
    ))
}

fn describe_energy_payment_failure_fallback(effects: &[Effect]) -> Option<String> {
    let [payment_effect, fallback_effect] = effects else {
        return None;
    };
    let with_id = payment_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let payment = with_id
        .effect
        .downcast_ref::<crate::effects::PayEnergyEffect>()?;
    let fallback = fallback_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if !matches!(payment.player.base(), ChooseSpec::Player(PlayerFilter::You))
        || fallback.condition != with_id.id
        || fallback.predicate != EffectPredicate::DidNotHappen
        || fallback.then.is_empty()
        || !fallback.else_.is_empty()
    {
        return None;
    }

    let payment_text = describe_effect(&with_id.effect);
    let fallback_text = describe_effect_list(&fallback.then);
    let payment_text = payment_text.trim().trim_end_matches('.');
    let fallback_text = fallback_text.trim().trim_end_matches('.');
    (!payment_text.is_empty() && !fallback_text.is_empty()).then(|| {
        format!(
            "{payment_text}. If you can't, {}",
            lowercase_first(fallback_text)
        )
    })
}

pub(super) fn describe_tap_then_put_counters_same_target(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let tap_tag = effect_outer_tag(first)?;
    let tap =
        structural_unwrap_render_wrappers(first).downcast_ref::<crate::effects::TapEffect>()?;
    let count = tap.target.count();
    if !tap.target.is_target() || count.max != Some(1) || count.dynamic_x || count.random {
        return None;
    }

    let put = structural_unwrap_render_wrappers(second)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.distributed
        || put.target_count.is_some()
        || !matches!(put.target.base(), ChooseSpec::Tagged(found) if found == tap_tag)
    {
        return None;
    }

    let target = describe_choose_spec(&tap.target);
    let counters = describe_put_counter_phrase(&put.amount, put.counter_type);
    Some(match linked_counter_followup_surface(put) {
        Some(ValueSurfaceHint::CounterFollowupSeparateSentence) => {
            format!("Tap {target}. Put {counters} on it")
        }
        Some(ValueSurfaceHint::CounterFollowupThen) => {
            format!("Tap {target}, then put {counters} on it")
        }
        _ => format!("Tap {target} and put {counters} on it"),
    })
}

pub(super) fn describe_hand_or_graveyard_choice_target(spec: &ChooseSpec) -> Option<String> {
    if spec.is_target() || !spec.count().is_single() {
        return None;
    }
    let ChooseSpec::Object(filter) = spec.base() else {
        return None;
    };
    let [first, second] = filter.any_of.as_slice() else {
        return None;
    };
    let zones = [first.zone?, second.zone?];
    if !zones.contains(&Zone::Hand) || !zones.contains(&Zone::Graveyard) {
        return None;
    }
    let mut first_base = first.clone();
    first_base.zone = None;
    let mut second_base = second.clone();
    second_base.zone = None;
    if first_base != second_base {
        return None;
    }
    let owner = first_base.owner.take()?;
    let selection = with_indefinite_article(&first_base.description());
    Some(format!(
        "{selection} from {} hand or graveyard",
        describe_possessive_player_filter(&owner)
    ))
}

pub(super) fn tagged_battlefield_move_result_tag(effect: &Effect) -> Option<TagKey> {
    fn contains_battlefield_move(effect: &Effect) -> bool {
        let unwrapped = structural_unwrap_render_wrappers(effect);
        if unwrapped
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
            .is_some_and(|move_to_zone| move_to_zone.zone == Zone::Battlefield)
        {
            return true;
        }
        unwrapped
            .downcast_ref::<crate::effects::MayEffect>()
            .is_some_and(|may| may.effects.iter().any(contains_battlefield_move))
    }
    if !contains_battlefield_move(effect) {
        return None;
    }
    effect_outer_tag(effect).cloned().or_else(|| {
        structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::MayEffect>()
            .and_then(|may| {
                may.effects
                    .iter()
                    .find_map(|inner| effect_outer_tag(inner).cloned())
            })
    })
}

pub(super) fn describe_choose_tap_conditional_freeze_bundle(effects: &[&Effect]) -> Option<String> {
    let [target_effect, tap_effect, conditional_effect] = effects else {
        return None;
    };
    let (target_tag, target_only) = tagged_target_only_effect(target_effect)?;
    let tap = structural_unwrap_render_wrappers(tap_effect)
        .downcast_ref::<crate::effects::TapEffect>()?;
    if !matches!(tap.target.base(), ChooseSpec::Tagged(tag) if tag == target_tag) {
        return None;
    }
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
        return None;
    }
    let Condition::PlayerControls { .. } = &conditional.condition else {
        return None;
    };
    let cant = structural_unwrap_render_wrappers(&conditional.if_true[0])
        .downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::Untap(filter) = &cant.restriction else {
        return None;
    };
    if !filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && &constraint.tag == target_tag
    }) {
        return None;
    }
    let freeze = describe_untap_restriction_for_subject(
        cant,
        UntapRestrictionSubject::singular("The chosen creature"),
    )?;
    Some(format!(
        "Choose {} and tap it. If {}, {}",
        describe_choose_spec(&target_only.target),
        describe_condition(&conditional.condition),
        lowercase_first(&freeze)
    ))
}

pub(in crate::compiled_text) fn rendered_action_target(effect: &Effect) -> Option<&ChooseSpec> {
    let action = structural_unwrap_render_wrappers(effect);
    if let Some(apply) = action.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        apply.target_spec.as_ref()
    } else if let Some(destroy) = action.downcast_ref::<crate::effects::DestroyEffect>() {
        Some(&destroy.spec)
    } else if let Some(put) = action.downcast_ref::<crate::effects::PutCountersEffect>() {
        Some(&put.target)
    } else if let Some(tap) = action.downcast_ref::<crate::effects::TapEffect>() {
        Some(&tap.target)
    } else if let Some(untap) = action.downcast_ref::<crate::effects::UntapEffect>() {
        Some(&untap.target)
    } else {
        None
    }
}

fn action_consumes_implicit_target(effect: &Effect, target: &ChooseSpec) -> bool {
    if rendered_action_target(effect)
        .is_some_and(|action_target| target_specs_select_same_objects(action_target, target))
    {
        return true;
    }

    let Some(target_player) = choose_spec_player_filter(target) else {
        return false;
    };
    structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
        .is_some_and(|exile| exile.player == target_player)
}

pub(in crate::compiled_text) fn target_specs_select_same_objects(
    left: &ChooseSpec,
    right: &ChooseSpec,
) -> bool {
    use ChooseSpec::{SurfaceHinted, Target, WithCount, WithCountValue};

    match (left, right) {
        (SurfaceHinted { spec, .. }, other) | (other, SurfaceHinted { spec, .. }) => {
            target_specs_select_same_objects(spec, other)
        }
        (Target(left), Target(right)) => target_specs_select_same_objects(left, right),
        (WithCount(left, left_count), WithCount(right, right_count))
        | (WithCountValue(left, left_count, _), WithCount(right, right_count))
        | (WithCount(left, left_count), WithCountValue(right, right_count, _))
        | (WithCountValue(left, left_count, _), WithCountValue(right, right_count, _)) => {
            left_count == right_count && target_specs_select_same_objects(left, right)
        }
        _ => left == right,
    }
}

pub(super) fn describe_redundant_target_only_pair(effects: &[Effect]) -> Option<String> {
    let [target_effect, action_effect] = effects else {
        return None;
    };
    let target_only = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let redundant = if target_only.explicit_declaration {
        rendered_action_target(action_effect).is_some_and(|action_target| {
            target_specs_select_same_objects(action_target, &target_only.target)
        })
    } else {
        action_consumes_implicit_target(action_effect, &target_only.target)
    };
    redundant.then(|| describe_effect(structural_unwrap_render_wrappers(action_effect)))
}

/// Fold an attachment object's target and its separately tagged destination
/// target back into the single targeted attachment instruction that declared
/// both targets. The tag proves the destination consumed by the attachment;
/// the first declaration (or triggering-object tag) proves its object.
pub(super) fn describe_targeted_attachment_instruction(effects: &[Effect]) -> Option<String> {
    let (object_effect, destination_effect, attach_effect) = match effects {
        [object_effect, destination_effect, attach_effect] => {
            (object_effect, destination_effect, attach_effect)
        }
        _ => return None,
    };
    let (destination_tag, destination) = tagged_target_only_effect(destination_effect)?;
    if destination.explicit_declaration
        || destination.chooser.is_some()
        || !destination.target.is_target()
    {
        return None;
    }
    let attach = structural_unwrap_render_wrappers(attach_effect)
        .downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    if attach.individual_targets
        || !choose_spec_references_exact_tag(&attach.target, destination_tag)
    {
        return None;
    }

    let object = if let Some(target_only) = structural_unwrap_render_wrappers(object_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
    {
        if target_only.explicit_declaration
            || target_only.chooser.is_some()
            || !target_only.target.is_target()
            || !target_specs_select_same_objects(&target_only.target, &attach.objects)
        {
            return None;
        }
        describe_attach_objects_spec(&target_only.target)
    } else if let Some(triggering) =
        object_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
    {
        if !choose_spec_references_exact_tag(&attach.objects, &triggering.tag) {
            return None;
        }
        "it".to_string()
    } else {
        return None;
    };

    Some(format!(
        "Attach {object} to {}",
        describe_choose_spec(&destination.target)
    ))
}

pub(super) fn describe_attach_all_enchanting_target_to_same_controller(
    effects: &[Effect],
) -> Option<String> {
    let (target_effect, destination_choice, attach_effect) = match effects {
        [target_effect, attach_effect] => (target_effect, None, attach_effect),
        [triggering_tag, target_effect, attach_effect]
            if triggering_tag
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some() =>
        {
            (target_effect, None, attach_effect)
        }
        [target_effect, destination_choice, attach_effect] => {
            (target_effect, Some(destination_choice), attach_effect)
        }
        [
            triggering_tag,
            target_effect,
            destination_choice,
            attach_effect,
        ] if triggering_tag
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some() =>
        {
            (target_effect, Some(destination_choice), attach_effect)
        }
        _ => return None,
    };
    let (target_tag, target_only) = tagged_target_only_effect(target_effect)?;
    if target_only.explicit_declaration
        || describe_choose_spec(&target_only.target) != "target permanent"
    {
        return None;
    }
    let attach = structural_unwrap_render_wrappers(attach_effect)
        .downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    let ChooseSpec::All(objects) = attach.objects.base() else {
        return None;
    };
    if !objects.subtypes.contains(&Subtype::Aura)
        || !objects.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *target_tag
                && constraint.relation
                    == crate::filter::TaggedOpbjectRelation::AttachedToTaggedObject
        })
    {
        return None;
    }
    let destination = if let Some(destination_choice) = destination_choice {
        let choice = destination_choice.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
        if !choice.count.is_single()
            || choice.count_value.is_some()
            || choice.aggregate_constraint.is_some()
            || choice.is_search
            || choice.reveal
            || choose_primary_zone(choice) != Some(Zone::Battlefield)
            || !matches!(&attach.target, ChooseSpec::Tagged(tag) if tag == &choice.tag)
        {
            return None;
        }
        &choice.filter
    } else {
        let ChooseSpec::Object(destination) = attach.target.base() else {
            return None;
        };
        destination
    };
    for relation in [
        crate::filter::TaggedOpbjectRelation::SameControllerAsTagged,
        crate::filter::TaggedOpbjectRelation::IsNotTaggedObject,
    ] {
        if !destination
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag == *target_tag && constraint.relation == relation)
        {
            return None;
        }
    }
    Some(
        "Attach all Auras enchanting target permanent to another permanent with the same controller"
            .to_string(),
    )
}

pub(in crate::compiled_text) fn describe_kicked_additional_targets_put_counters(
    effects: &[&Effect],
) -> Option<String> {
    let [target_effect, for_each_effect] = effects else {
        return None;
    };
    let target_tag = effect_outer_tag(target_effect)?.clone();
    let target_only = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let ChooseSpec::WithCountValue(target, count, count_value) = &target_only.target else {
        return None;
    };
    if !count.is_dynamic_x() || count.is_up_to_dynamic_x() || count.is_random() {
        return None;
    }
    let Value::Add(left, right) = count_value else {
        return None;
    };
    let counts_one_plus_kicked = matches!(
        (left.as_ref(), right.as_ref()),
        (Value::Fixed(1), Value::KickCount) | (Value::KickCount, Value::Fixed(1))
    );
    if !counts_one_plus_kicked {
        return None;
    }

    let for_each = structural_unwrap_render_wrappers(for_each_effect)
        .downcast_ref::<crate::effects::ForEachObject>()?;
    if !for_each.filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == target_tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }) {
        return None;
    }
    let [put] = for_each.effects.as_slice() else {
        return None;
    };
    let put = structural_unwrap_render_wrappers(put)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.distributed || put.target_count.is_some() || !matches!(put.target, ChooseSpec::Iterated)
    {
        return None;
    }

    let first_target = describe_choose_spec(target);
    let additional_target = first_target
        .strip_prefix("target ")
        .map(|tail| format!("another target {tail}"))
        .unwrap_or_else(|| format!("another {first_target}"));
    Some(format!(
        "Choose {first_target}, then choose {additional_target} for each time this spell was kicked. Put {} on each {first_target}",
        describe_put_counter_phrase(&put.amount, put.counter_type)
    ))
}

pub(super) fn for_each_moves_unchosen_iterated_to_zone(
    effect: &Effect,
    revealed_tag: &crate::TagKey,
    chosen_tag: &crate::TagKey,
    zone: Zone,
) -> bool {
    let Some((_, for_each)) = for_each_tagged_for_compaction(effect) else {
        return false;
    };
    for_each_moves_unselected_to_zone(for_each, revealed_tag.as_str(), chosen_tag.as_str(), zone)
}

pub(super) fn describe_reveal_top_one_hand_gain_mana_value_rest_graveyard(
    effects: &[Effect],
) -> Option<String> {
    let (look_effect, choose_effect, move_effect, gain_effect, rest_effect) = match effects {
        [look, choose, move_effect, gain_effect, rest_effect] => {
            (look, choose, move_effect, gain_effect, rest_effect)
        }
        [look, reveal, choose, move_effect, gain_effect, rest_effect] => {
            let look_view = look.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
            let reveal_view = reveal.downcast_ref::<crate::effects::RevealTaggedEffect>()?;
            if look_view.reveal || reveal_view.tag != look_view.tag {
                return None;
            }
            (look, choose, move_effect, gain_effect, rest_effect)
        }
        _ => return None,
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let (_, move_to_hand) = for_each_tagged_for_compaction(move_effect)?;
    let gain = gain_effect.downcast_ref::<crate::effects::GainLifeEffect>()?;
    let (_, rest) = for_each_tagged_for_compaction(rest_effect)?;

    if look.player != PlayerFilter::You
        || choose.chooser != PlayerFilter::You
        || choose_exact_count(choose) != Some(1)
        || !choose_references_tag(choose, &look.tag)
        || !for_each_moves_tag_to_hand(move_to_hand, choose.tag.as_str())
        || gain.player != ChooseSpec::Player(PlayerFilter::You)
        || !matches!(
            gain.amount.unhinted(),
            Value::ManaValueOf(spec)
                if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
        )
        || !for_each_moves_unselected_to_zone(
            rest,
            look.tag.as_str(),
            choose.tag.as_str(),
            Zone::Graveyard,
        )
    {
        return None;
    }

    let (count_text, noun, _) = describe_look_count_and_noun(&look.count);
    Some(format!(
        "Reveal the top {count_text} {noun} of your library and put one of them into your hand. You gain life equal to that card's mana value. Put all other cards revealed this way into your graveyard"
    ))
}

pub(super) fn describe_reveal_top_choice_to_hand_rest_graveyard_structural(
    effects: &[Effect],
) -> Option<String> {
    if effects.len() < 5 {
        return None;
    }
    let look = effects[0].downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let reveal = effects[1].downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    if look.player != PlayerFilter::You || look.reveal || reveal.tag != look.tag {
        return None;
    }

    let mut chooses: Vec<&crate::effects::ChooseObjectsEffect> = Vec::new();
    let mut chosen_tag: Option<TagKey> = None;
    let mut idx = 2usize;
    while let Some(choose) = effects
        .get(idx)
        .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
    {
        if choose.chooser != PlayerFilter::You
            || choose.is_search
            || choose.count.min != 0
            || !matches!(choose.count.max, Some(1) | None)
            || !choose_references_tag(choose, &look.tag)
        {
            return None;
        }
        if let Some(existing) = &chosen_tag {
            if choose.tag != *existing {
                return None;
            }
        } else {
            chosen_tag = Some(choose.tag.clone());
        }
        chooses.push(choose);
        idx += 1;
    }
    let chosen_tag = chosen_tag?;
    if chooses.is_empty()
        || effects.len() != idx + 2
        || !for_each_moves_tagged_iterated_to_hand(&effects[idx], &chosen_tag)
        || !for_each_moves_unchosen_iterated_to_zone(
            &effects[idx + 1],
            &look.tag,
            &chosen_tag,
            Zone::Graveyard,
        )
    {
        return None;
    }

    let owner = describe_possessive_player_filter(&look.player);
    let (count_text, noun, _) = describe_look_count_and_noun(&look.count);
    let choice = if let [choose] = chooses.as_slice() {
        if choose.count.is_any_number() {
            format!(
                "any number of {}",
                describe_any_number_filter_from_looked_cards(look, choose)?
            )
        } else if let Some(label) = structural_revealed_choice_label(choose) {
            structural_revealed_choice_phrase(&label)
        } else {
            describe_choose_filter_from_looked_cards(look, choose)?
        }
    } else {
        if chooses.iter().any(|choose| choose.count.max.is_none()) {
            return None;
        }
        chooses
            .iter()
            .map(|choose| {
                structural_revealed_choice_label(choose)
                    .map(|label| structural_revealed_choice_phrase(&label))
            })
            .collect::<Option<Vec<_>>>()?
            .join(" and/or ")
    };
    Some(format!(
        "Reveal the top {count_text} {noun} of {owner} library. You may put {choice} from among them into your hand. Put the rest into your graveyard"
    ))
}

pub(super) fn tagged_apply_continuous_view(
    effect: &Effect,
) -> Option<(&TagKey, &crate::effects::ApplyContinuousEffect)> {
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let apply = tagged
        .effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    Some((&tagged.tag, apply))
}

pub(super) fn tagged_untap_effect_view(
    effect: &Effect,
) -> Option<(&TagKey, &crate::effects::UntapEffect)> {
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let untap = tagged
        .effect
        .downcast_ref::<crate::effects::UntapEffect>()?;
    Some((&tagged.tag, untap))
}

pub(super) fn tagged_put_counters_effect_view(
    effect: &Effect,
) -> Option<(&TagKey, &crate::effects::PutCountersEffect)> {
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let put = tagged
        .effect
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    Some((&tagged.tag, put))
}

pub(super) fn is_target_only_opponent(effect: &Effect) -> bool {
    effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
        .is_some_and(|target_only| {
            matches!(
                target_only.target.base(),
                ChooseSpec::Player(PlayerFilter::Opponent)
            )
        })
}

pub(super) fn reciprocal_creature_tag_matching(
    effect: &Effect,
    controller: &PlayerFilter,
) -> Option<crate::TagKey> {
    let tag_matching = effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    let effective_zone = match (tag_matching.zone, tag_matching.filter.zone) {
        (Some(outer), Some(inner)) if outer == inner => Some(outer),
        (Some(zone), None) | (None, Some(zone)) => Some(zone),
        _ => None,
    };
    if effective_zone != Some(Zone::Battlefield)
        || !tag_matching.additional_zones.is_empty()
        || tag_matching.filter.card_types.as_slice() != [CardType::Creature]
        || tag_matching.filter.controller.as_ref() != Some(controller)
    {
        return None;
    }
    Some(tag_matching.tag.clone())
}

pub(super) fn apply_changes_control_to_effect_controller_for_tag(
    effect: &Effect,
    tag: &crate::TagKey,
) -> bool {
    let Some((_, apply)) = tagged_apply_continuous_view(effect) else {
        return false;
    };
    apply.target == crate::continuous::EffectTarget::Source
        && apply.until == Until::EndOfTurn
        && apply.condition.is_none()
        && apply.modification.is_none()
        && apply.additional_modifications.is_empty()
        && matches!(
            apply.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController]
        )
        && apply
            .target_spec
            .as_ref()
            .is_some_and(|spec| choose_spec_references_tagged_object(spec, tag))
}

pub(super) fn apply_changes_control_to_target_opponent_for_tag(
    effect: &Effect,
    tag: &crate::TagKey,
) -> bool {
    let Some((_, apply)) = tagged_apply_continuous_view(effect) else {
        return false;
    };
    apply.target == crate::continuous::EffectTarget::Source
        && apply.until == Until::EndOfTurn
        && apply.condition.is_none()
        && apply.modification.is_none()
        && apply.additional_modifications.is_empty()
        && matches!(
            apply.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(player)]
                if matches!(player, PlayerFilter::Target(inner) if matches!(inner.as_ref(), PlayerFilter::Opponent))
        )
        && apply
            .target_spec
            .as_ref()
            .is_some_and(|spec| choose_spec_references_tagged_object(spec, tag))
}

pub(super) fn object_filter_references_tag_recursive(
    filter: &ObjectFilter,
    tag: &crate::TagKey,
) -> bool {
    filter_references_tag(filter, tag)
        || filter
            .any_of
            .iter()
            .any(|candidate| object_filter_references_tag_recursive(candidate, tag))
}

pub(super) fn choose_spec_references_tagged_filter_recursive(
    spec: &ChooseSpec,
    tag: &crate::TagKey,
) -> bool {
    match spec.base() {
        ChooseSpec::Tagged(found) => found == tag,
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            object_filter_references_tag_recursive(filter, tag)
        }
        _ => false,
    }
}

pub(super) fn choose_spec_references_both_tags(
    spec: &ChooseSpec,
    first: &crate::TagKey,
    second: &crate::TagKey,
) -> bool {
    choose_spec_references_tagged_filter_recursive(spec, first)
        && choose_spec_references_tagged_filter_recursive(spec, second)
}

pub(super) fn untaps_both_tagged_groups(
    effect: &Effect,
    first: &crate::TagKey,
    second: &crate::TagKey,
) -> bool {
    let Some(untap) = effect.downcast_ref::<crate::effects::UntapEffect>() else {
        return false;
    };
    choose_spec_references_both_tags(&untap.target, first, second)
}

pub(super) fn tags_both_tagged_groups(
    effect: &Effect,
    first: &crate::TagKey,
    second: &crate::TagKey,
) -> bool {
    let Some(tag_matching) = effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()
    else {
        return false;
    };
    tag_matching.zone.is_none()
        && tag_matching.additional_zones.is_empty()
        && object_filter_references_tag_recursive(&tag_matching.filter, first)
        && object_filter_references_tag_recursive(&tag_matching.filter, second)
}

pub(super) fn grants_haste_to_both_tagged_groups(
    effect: &Effect,
    first: &crate::TagKey,
    second: &crate::TagKey,
) -> bool {
    let Some(apply) = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
    else {
        return false;
    };
    if apply.until != Until::EndOfTurn
        || apply.condition.is_some()
        || !apply.runtime_modifications.is_empty()
        || !apply.additional_modifications.is_empty()
        || !matches!(
            &apply.modification,
            Some(crate::continuous::Modification::AddAbility(ability))
                if ability.id() == crate::static_abilities::StaticAbilityId::Haste
        )
    {
        return false;
    }
    match &apply.target {
        crate::continuous::EffectTarget::Filter(filter) => {
            object_filter_references_tag_recursive(filter, first)
                && object_filter_references_tag_recursive(filter, second)
        }
        _ => false,
    }
}

pub(super) fn describe_reciprocal_creature_control_structural(
    effects: &[Effect],
) -> Option<String> {
    let effects = if let [first, rest @ ..] = effects
        && is_target_only_opponent(first)
    {
        rest
    } else {
        effects
    };
    let [tag_yours, tag_theirs, tail @ ..] = effects else {
        return None;
    };

    let target_opponent = PlayerFilter::Target(Box::new(PlayerFilter::Opponent));
    let your_tag = reciprocal_creature_tag_matching(tag_yours, &PlayerFilter::You)?;
    let their_tag = reciprocal_creature_tag_matching(tag_theirs, &target_opponent)?;
    let valid_control_pair = |control_theirs: &Effect, control_yours: &Effect| {
        apply_changes_control_to_effect_controller_for_tag(control_theirs, &their_tag)
            && apply_changes_control_to_target_opponent_for_tag(control_yours, &your_tag)
    };
    let valid_untap = |tag: Option<&Effect>, untap: &Effect| {
        tag.is_none_or(|tag| tags_both_tagged_groups(tag, &your_tag, &their_tag))
            && untaps_both_tagged_groups(untap, &your_tag, &their_tag)
    };

    let untap_before_control = match tail {
        [control_theirs, control_yours, untap, haste]
            if valid_control_pair(control_theirs, control_yours)
                && valid_untap(None, untap)
                && grants_haste_to_both_tagged_groups(haste, &your_tag, &their_tag) =>
        {
            false
        }
        [control_theirs, control_yours, untap_tag, untap, haste]
            if valid_control_pair(control_theirs, control_yours)
                && valid_untap(Some(untap_tag), untap)
                && grants_haste_to_both_tagged_groups(haste, &your_tag, &their_tag) =>
        {
            false
        }
        [untap, control_theirs, control_yours, haste]
            if valid_untap(None, untap)
                && valid_control_pair(control_theirs, control_yours)
                && grants_haste_to_both_tagged_groups(haste, &your_tag, &their_tag) =>
        {
            true
        }
        [untap_tag, untap, control_theirs, control_yours, haste]
            if valid_untap(Some(untap_tag), untap)
                && valid_control_pair(control_theirs, control_yours)
                && grants_haste_to_both_tagged_groups(haste, &your_tag, &their_tag) =>
        {
            true
        }
        _ => return None,
    };

    Some(if untap_before_control {
        "Untap all creatures you control and all creatures target opponent controls. You and that opponent each gain control of all creatures the other controls until end of turn. Those creatures gain haste until end of turn"
            .to_string()
    } else {
        "You and target opponent each gain control of all creatures the other controls until end of turn. Untap those creatures. Those creatures gain haste until end of turn"
            .to_string()
    })
}

pub(super) fn is_gain_control_until_eot(apply: &crate::effects::ApplyContinuousEffect) -> bool {
    apply.target == crate::continuous::EffectTarget::Source
        && apply.until == Until::EndOfTurn
        && apply.condition.is_none()
        && apply.modification.is_none()
        && apply.additional_modifications.is_empty()
        && matches!(
            apply.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController]
        )
}

pub(super) fn is_haste_until_eot_for_tag(
    apply: &crate::effects::ApplyContinuousEffect,
    tag: &crate::TagKey,
) -> bool {
    apply.target == crate::continuous::EffectTarget::Source
        && apply.until == Until::EndOfTurn
        && apply.condition.is_none()
        && apply.additional_modifications.is_empty()
        && apply.runtime_modifications.is_empty()
        && apply
            .target_spec
            .as_ref()
            .is_some_and(|target| choose_spec_references_tagged_object(target, tag))
        && matches!(
            &apply.modification,
            Some(crate::continuous::Modification::AddAbility(ability))
                if ability.id() == crate::static_abilities::StaticAbilityId::Haste
        )
}

pub(super) fn gain_control_followup_untap_target_text(target: &str) -> &'static str {
    if target.contains("creature") && !target.contains("artifact or creature") {
        "that creature"
    } else if target.contains("permanent") {
        "that permanent"
    } else {
        "it"
    }
}

fn choose_spec_uses_plural_pronoun_reference(spec: &ChooseSpec) -> bool {
    match spec.base() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.has_plural_pronoun_reference_surface()
        }
        _ => false,
    }
}

fn gain_control_object_reference_tag<'a>(
    controlled_tag: &'a TagKey,
    control: &'a crate::effects::ApplyContinuousEffect,
) -> &'a TagKey {
    match control.target_spec.as_ref().map(ChooseSpec::unhinted) {
        Some(ChooseSpec::Tagged(tag)) => tag,
        _ => controlled_tag,
    }
}

pub(super) fn describe_gain_control_then_untap_structural(effects: &[Effect]) -> Option<String> {
    let [control_effect, untap_effect] = effects else {
        return None;
    };
    if let (Some((controlled_tag, control)), Some((_, untap))) = (
        tagged_apply_continuous_view(control_effect),
        tagged_untap_effect_view(untap_effect),
    ) && is_gain_control_until_eot(control)
    {
        let controlled_object_tag = gain_control_object_reference_tag(controlled_tag, control);
        if !choose_spec_references_tagged_object(&untap.target, controlled_object_tag) {
            return None;
        }
        let target = control
            .target_spec
            .as_ref()
            .map(describe_choose_spec)
            .unwrap_or_else(|| "target creature".to_string());
        let untap_target = if choose_spec_uses_plural_pronoun_reference(&untap.target) {
            "them"
        } else {
            gain_control_followup_untap_target_text(&target)
        };
        return Some(format!(
            "Gain control of {target} until end of turn. Untap {untap_target}"
        ));
    }

    let (untapped_tag, untap) = tagged_untap_effect_view(control_effect)?;
    let (_, control) = tagged_apply_continuous_view(untap_effect)?;
    if !is_gain_control_until_eot(control)
        || !control
            .target_spec
            .as_ref()
            .is_some_and(|target| choose_spec_references_tagged_object(target, untapped_tag))
    {
        return None;
    }
    Some(format!(
        "Untap {} and gain control of it until end of turn",
        describe_choose_spec(&untap.target)
    ))
}

pub(super) fn describe_gain_control_untap_haste_structural(effects: &[Effect]) -> Option<String> {
    let [first, second, third] = effects else {
        return None;
    };

    if let (Some((controlled_tag, control)), Some((untapped_tag, untap)), Some((_, haste))) = (
        tagged_apply_continuous_view(first),
        tagged_untap_effect_view(second),
        tagged_apply_continuous_view(third),
    ) && is_gain_control_until_eot(control)
    {
        let controlled_object_tag = gain_control_object_reference_tag(controlled_tag, control);
        if !choose_spec_references_tagged_object(&untap.target, controlled_object_tag)
            || !(is_haste_until_eot_for_tag(haste, untapped_tag)
                || is_haste_until_eot_for_tag(haste, controlled_object_tag))
        {
            return None;
        }
        let target = control
            .target_spec
            .as_ref()
            .map(describe_choose_spec)
            .unwrap_or_else(|| "target creature".to_string());
        let plural_reference = choose_spec_uses_plural_pronoun_reference(&untap.target);
        let untap_target = if plural_reference {
            "them"
        } else {
            gain_control_followup_untap_target_text(&target)
        };
        let haste_sentence = if plural_reference {
            "They gain haste until end of turn"
        } else {
            "It gains haste until end of turn"
        };
        return Some(format!(
            "Gain control of {target} until end of turn. Untap {untap_target}. {haste_sentence}"
        ));
    }

    if let (Some((untapped_tag, untap)), Some((controlled_tag, control)), Some((_, haste))) = (
        tagged_untap_effect_view(first),
        tagged_apply_continuous_view(second),
        tagged_apply_continuous_view(third),
    ) && is_gain_control_until_eot(control)
        && control
            .target_spec
            .as_ref()
            .is_some_and(|target| choose_spec_references_tagged_object(target, untapped_tag))
        && (is_haste_until_eot_for_tag(haste, controlled_tag)
            || is_haste_until_eot_for_tag(haste, untapped_tag))
    {
        let target = describe_choose_spec(&untap.target);
        let followup_subject = match gain_control_followup_untap_target_text(&target) {
            "that creature" => "That creature",
            "that permanent" => "That permanent",
            _ => "It",
        };
        return Some(format!(
            "Untap {target} and gain control of it until end of turn. {followup_subject} gains haste until end of turn"
        ));
    }

    None
}

fn describe_gain_control_untap_haste_clause_structural(effects: &[Effect]) -> Option<String> {
    let [control_effect, untap_effect, haste_effect] = effects else {
        return None;
    };
    let (controlled_tag, control) = tagged_apply_continuous_view(control_effect)?;
    let (untapped_tag, untap) = tagged_untap_effect_view(untap_effect)?;
    let (_, haste) = tagged_apply_continuous_view(haste_effect)?;
    if !is_gain_control_until_eot(control) {
        return None;
    }

    // The comma-joined surface is for a continuation such as Jet's
    // Brainwashing, where an earlier clause has already selected the object
    // and this conditional refers back to it.  A standalone theft effect
    // selects its target here and Oracle keeps control, untap, and haste as
    // separate sentences.
    if !matches!(
        control.target_spec.as_ref().map(ChooseSpec::unhinted),
        Some(ChooseSpec::Tagged(_))
    ) {
        return None;
    }

    let controlled_object_tag = gain_control_object_reference_tag(controlled_tag, control);
    if !choose_spec_references_tagged_object(&untap.target, controlled_object_tag)
        || !(is_haste_until_eot_for_tag(haste, untapped_tag)
            || is_haste_until_eot_for_tag(haste, controlled_object_tag))
    {
        return None;
    }

    let target = control
        .target_spec
        .as_ref()
        .map(describe_choose_spec)
        .unwrap_or_else(|| "target creature".to_string());
    let plural_reference = choose_spec_uses_plural_pronoun_reference(&untap.target);
    let untap_target = if plural_reference {
        "them"
    } else {
        gain_control_followup_untap_target_text(&target)
    };
    let haste_predicate = if plural_reference {
        "they gain haste until end of turn"
    } else {
        "it gains haste until end of turn"
    };
    Some(format!(
        "Gain control of {target} until end of turn, untap {untap_target}, and {haste_predicate}"
    ))
}

pub(super) fn describe_gain_control_counter_untap_haste_structural(
    effects: &[Effect],
) -> Option<String> {
    let [control_effect, counter_effect, untap_effect, haste_effect] = effects else {
        return None;
    };
    let (controlled_tag, control) = tagged_apply_continuous_view(control_effect)?;
    let (_, put) = tagged_put_counters_effect_view(counter_effect)?;
    let (untapped_tag, untap) = tagged_untap_effect_view(untap_effect)?;
    let (_, haste) = tagged_apply_continuous_view(haste_effect)?;
    let controlled_object_tag = gain_control_object_reference_tag(controlled_tag, control);
    if !is_gain_control_until_eot(control)
        || put.distributed
        || put.target_count.is_some()
        || !choose_spec_references_tagged_object(&put.target, controlled_object_tag)
        || !choose_spec_references_tagged_object(&untap.target, controlled_object_tag)
        || !(is_haste_until_eot_for_tag(haste, untapped_tag)
            || is_haste_until_eot_for_tag(haste, controlled_object_tag))
    {
        return None;
    }

    let target = control
        .target_spec
        .as_ref()
        .map(describe_choose_spec)
        .unwrap_or_else(|| "target creature".to_string());
    let final_subject = if gain_control_followup_untap_target_text(&target) == "that creature" {
        "That creature"
    } else {
        "It"
    };
    Some(format!(
        "Gain control of {target} until end of turn. Put {} on it and untap it. {final_subject} gains haste until end of turn",
        describe_put_counter_phrase(&put.amount, put.counter_type)
    ))
}

pub(super) fn describe_put_counters_then_untap_same_target_structural(
    effects: &[Effect],
) -> Option<String> {
    let [counter_effect, untap_effect] = effects else {
        return None;
    };
    let (countered_tag, put) = tagged_put_counters_effect_view(counter_effect)?;
    let (_, untap) = tagged_untap_effect_view(untap_effect)?;
    let count = put.target.count();
    if put.distributed
        || put
            .target_count
            .as_ref()
            .is_some_and(|target_count| target_count != &count)
        || !put.target.is_target()
        || count.max != Some(1)
        || count.dynamic_x
        || count.random
        || !matches!(untap.target.base(), ChooseSpec::Tagged(tag) if tag == countered_tag)
    {
        return None;
    }

    Some(format!(
        "{}. Untap it",
        describe_effect(counter_effect).trim_end_matches('.')
    ))
}

pub(super) fn describe_must_block_untap_then_others_cant_block_structural(
    effects: &[Effect],
) -> Option<String> {
    let [must_block_effect, untap_effect, cant_block_effect] = effects else {
        return None;
    };
    let (affected_tag, must_block) = tagged_apply_continuous_view(must_block_effect)?;
    let (_, untap) = tagged_untap_effect_view(untap_effect)?;
    let cant = cant_block_effect.downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::Block(filter) = &cant.restriction else {
        return None;
    };
    let target = must_block.target_spec.as_ref()?;
    if must_block.target != crate::continuous::EffectTarget::Source
        || must_block.until != Until::EndOfTurn
        || must_block.condition.is_some()
        || !must_block.additional_modifications.is_empty()
        || !must_block.runtime_modifications.is_empty()
        || !matches!(
            &must_block.modification,
            Some(crate::continuous::Modification::AddAbility(ability))
                if ability.id() == crate::static_abilities::StaticAbilityId::MustBlock
        )
        || !matches!(untap.target.base(), ChooseSpec::Tagged(tag) if tag == affected_tag)
        || cant.duration != Until::EndOfTurn
    {
        return None;
    }

    let mut expected_filter = ObjectFilter::creature().in_zone(Zone::Battlefield);
    expected_filter.other = true;
    expected_filter.controller = Some(PlayerFilter::AliasedControllerOf(
        crate::filter::ObjectRef::Tagged(affected_tag.clone()),
    ));
    if filter != &expected_filter {
        return None;
    }

    let target = describe_choose_spec(target);
    Some(format!(
        "{} blocks this turn if able. Untap that creature. Other creatures that player controls can't block this turn",
        capitalize_first(&target)
    ))
}

#[cfg(test)]
mod control_reference_surface_tests {
    use super::*;

    fn gain_control(target: ChooseSpec, tag: &TagKey) -> Effect {
        let mut control = crate::effects::ApplyContinuousEffect::new_runtime(
            crate::continuous::EffectTarget::Source,
            crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController,
            Until::EndOfTurn,
        );
        control.target_spec = Some(target);
        Effect::new(control).tag(tag.clone())
    }

    fn untap(tag: &TagKey) -> Effect {
        Effect::untap(ChooseSpec::Tagged(tag.clone())).tag("untapped")
    }

    fn untap_tagged_creature(reference_tag: &TagKey, effect_tag: &TagKey) -> Effect {
        let target = ChooseSpec::Object(
            ObjectFilter::creature()
                .in_zone(Zone::Battlefield)
                .match_tagged(
                    reference_tag.clone(),
                    crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                ),
        )
        .with_surface_hint(crate::target::ChooseSpecSurfaceHint::SourceReference(
            crate::target::SourceReferenceSurface::ThisPermanentType("that creature".to_string()),
        ));
        Effect::untap(target).tag(effect_tag.clone())
    }

    fn grant_haste(tag: &TagKey) -> Effect {
        let mut haste = crate::effects::ApplyContinuousEffect::new(
            crate::continuous::EffectTarget::Source,
            crate::continuous::Modification::AddAbility(
                crate::static_abilities::StaticAbility::haste(),
            ),
            Until::EndOfTurn,
        );
        haste.target_spec = Some(ChooseSpec::Tagged(tag.clone()));
        Effect::new(haste).tag("granted")
    }

    fn grant_haste_to_outer_tag(tag: &TagKey) -> Effect {
        let mut haste = crate::effects::ApplyContinuousEffect::new(
            crate::continuous::EffectTarget::Source,
            crate::continuous::Modification::AddAbility(
                crate::static_abilities::StaticAbility::haste(),
            ),
            Until::EndOfTurn,
        );
        haste.target_spec = Some(ChooseSpec::Tagged(tag.clone()).with_surface_hint(
            crate::target::ChooseSpecSurfaceHint::SourceReference(
                crate::target::SourceReferenceSurface::ThisPermanentType("it".to_string()),
            ),
        ));
        Effect::new(haste).tag("granted")
    }

    #[test]
    fn renders_attached_permanent_control_chain_with_sentence_carry() {
        let enchanted = TagKey::from("enchanted");
        let effects = vec![
            Effect::tag_attached_to_source(enchanted.clone()),
            gain_control(
                ChooseSpec::Tagged(enchanted.clone()),
                &TagKey::from("controlled"),
            ),
            untap(&enchanted),
            grant_haste(&enchanted),
        ];

        assert_eq!(
            describe_pre_clause_structural_effect_list(&effects).as_deref(),
            Some(
                "Gain control of enchanted permanent until end of turn. Untap that permanent. It gains haste until end of turn"
            )
        );
    }

    #[test]
    fn standalone_control_untap_haste_preserves_sentence_boundaries() {
        let controlled = TagKey::from("controlled_0");
        let untapped = TagKey::from("untapped_1");
        let effects = vec![
            gain_control(
                ChooseSpec::target(ChooseSpec::Object(
                    ObjectFilter::creature().in_zone(Zone::Battlefield),
                )),
                &controlled,
            ),
            untap_tagged_creature(&controlled, &untapped),
            grant_haste_to_outer_tag(&untapped),
        ];

        assert_eq!(
            describe_effect_clause_list(&effects).as_deref(),
            Some(
                "gain control of target creature until end of turn. Untap that creature. It gains haste until end of turn"
            )
        );
    }

    #[test]
    fn existing_target_control_untap_haste_uses_one_conditional_clause() {
        let selected = TagKey::from("selected");
        let controlled = TagKey::from("controlled");
        let untapped = TagKey::from("untapped");
        let effects = vec![
            gain_control(ChooseSpec::Tagged(selected.clone()), &controlled),
            untap_tagged_creature(&selected, &untapped),
            grant_haste_to_outer_tag(&untapped),
        ];

        assert_eq!(
            describe_effect_clause_list(&effects).as_deref(),
            Some(
                "gain control of it until end of turn, untap it, and it gains haste until end of turn"
            )
        );
    }

    #[test]
    fn renders_control_counter_untap_haste_as_three_sentences() {
        let controlled = TagKey::from("controlled");
        let effects = vec![
            gain_control(
                ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
                &controlled,
            ),
            Effect::plus_one_counters(Value::Fixed(1), ChooseSpec::Tagged(controlled.clone()))
                .tag("countered"),
            untap(&controlled),
            grant_haste(&controlled),
        ];

        assert_eq!(
            describe_gain_control_counter_untap_haste_structural(&effects).as_deref(),
            Some(
                "Gain control of target creature until end of turn. Put a +1/+1 counter on it and untap it. That creature gains haste until end of turn"
            )
        );
    }

    #[test]
    fn renders_optional_single_counter_target_then_untap_as_separate_sentences() {
        let countered = TagKey::from("countered");
        let target = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::default().with_subtype(Subtype::Elf),
        ))
        .with_count(ChoiceCount::up_to(1));
        let effects = vec![
            Effect::new(
                crate::effects::PutCountersEffect::plus_one_counters(Value::Fixed(1), target)
                    .with_target_count(ChoiceCount::up_to(1)),
            )
            .tag(countered.clone()),
            untap(&countered),
        ];

        assert_eq!(
            describe_put_counters_then_untap_same_target_structural(&effects).as_deref(),
            Some("Put a +1/+1 counter on up to one target Elf. Untap it")
        );
    }

    #[test]
    fn renders_must_block_untap_and_controller_lockout_with_sentence_carry() {
        let affected = TagKey::from("granted");
        let target = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::creature().controlled_by(PlayerFilter::Opponent),
        ));
        let mut must_block = crate::effects::ApplyContinuousEffect::new(
            crate::continuous::EffectTarget::Source,
            crate::continuous::Modification::AddAbility(
                crate::static_abilities::StaticAbility::must_block(),
            ),
            Until::EndOfTurn,
        );
        must_block.target_spec = Some(target);
        let mut other_creatures = ObjectFilter::creature().in_zone(Zone::Battlefield);
        other_creatures.other = true;
        other_creatures.controller = Some(PlayerFilter::AliasedControllerOf(
            crate::filter::ObjectRef::Tagged(affected.clone()),
        ));
        let effects = vec![
            Effect::new(must_block).tag(affected.clone()),
            untap(&affected),
            Effect::cant_until(
                crate::effect::Restriction::block(other_creatures),
                Until::EndOfTurn,
            ),
        ];

        assert_eq!(
            describe_must_block_untap_then_others_cant_block_structural(&effects).as_deref(),
            Some(
                "Target creature an opponent controls blocks this turn if able. Untap that creature. Other creatures that player controls can't block this turn"
            )
        );
    }
}

pub(super) fn choose_spec_has_equipment_filter(spec: &ChooseSpec) -> bool {
    matches!(
        spec.base(),
        ChooseSpec::Object(filter) | ChooseSpec::All(filter)
            if filter.subtypes.contains(&Subtype::Equipment)
    )
}

fn choose_spec_has_aura_filter(spec: &ChooseSpec) -> bool {
    match spec.unhinted() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.subtypes.contains(&Subtype::Aura)
                || filter
                    .any_of
                    .iter()
                    .any(|branch| branch.subtypes.contains(&Subtype::Aura))
        }
        ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => choose_spec_has_aura_filter(inner),
        _ => false,
    }
}

pub(super) fn is_gain_control_effect(apply: &crate::effects::ApplyContinuousEffect) -> bool {
    apply.target == crate::continuous::EffectTarget::Source
        && apply.condition.is_none()
        && apply.modification.is_none()
        && apply.additional_modifications.is_empty()
        && matches!(
            apply.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController]
        )
}

pub(super) fn describe_gain_control_create_token_attach_sequence(
    effects: &[Effect],
) -> Option<String> {
    let [gain_effect, create_effect, attach_effect] = effects else {
        return None;
    };

    let (controlled_tag, control) = tagged_apply_continuous_view(gain_effect)?;
    if !is_gain_control_effect(control)
        || !control
            .target_spec
            .as_ref()
            .is_some_and(choose_spec_has_equipment_filter)
    {
        return None;
    }

    let (created_tag, create_token) = tagged_create_token_effect(create_effect)?;
    if create_token.count != Value::Fixed(1) {
        return None;
    }

    let attach = attach_effect.downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    if !matches!(&attach.target, ChooseSpec::Tagged(tag) if tag == created_tag)
        || !choose_spec_has_equipment_filter(&attach.objects)
        || (!choose_spec_references_tagged_object(&attach.objects, controlled_tag)
            && !choose_spec_references_tagged_object(&attach.objects, created_tag))
    {
        return None;
    }

    let gain_text = describe_effect(gain_effect)
        .trim_end_matches('.')
        .to_string();
    let create_text = lowercase_first(describe_effect(create_effect).trim_end_matches('.'));
    Some(format!(
        "{gain_text}, then {create_text} and attach that Equipment to it"
    ))
}

/// Gain control of an Aura and move that same Aura to a legal new host. The
/// Aura qualifier is structural, and runtime attachment already checks the
/// Aura's current enchant restriction, so "it can enchant" is not a guessed
/// surface-only promise.
fn describe_gain_control_aura_then_legal_attach(effects: &[Effect]) -> Option<String> {
    let [gain_effect, attach_effect] = effects else {
        return None;
    };
    let (aura_tag, control) = tagged_apply_continuous_view(gain_effect)?;
    if !is_gain_control_effect(control)
        || !control
            .target_spec
            .as_ref()
            .is_some_and(choose_spec_has_aura_filter)
    {
        return None;
    }
    let attach = structural_unwrap_render_wrappers(attach_effect)
        .downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    if !choose_spec_references_exact_tag(&attach.objects, aura_tag) {
        return None;
    }
    let target = describe_choose_spec(&attach.target);
    let gain = describe_effect(gain_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    Some(format!("{gain}, then attach it to {target} it can enchant"))
}

fn choose_spec_is_source(spec: &ChooseSpec) -> bool {
    match spec.unhinted() {
        ChooseSpec::Source => true,
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter.source,
        ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => choose_spec_is_source(inner),
        _ => false,
    }
}

fn effect_produces_attachment_target(effect: &Effect, target_tag: &TagKey) -> bool {
    let Some(producer_tag) = effect_outer_tag(effect) else {
        return false;
    };
    if producer_tag != target_tag {
        return false;
    }
    let producer = structural_unwrap_render_wrappers(effect);
    producer
        .downcast_ref::<crate::effects::CreateTokenEffect>()
        .is_some()
        || producer
            .downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()
            .is_some()
        || producer
            .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
            .is_some()
        || producer
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
            .is_some_and(|move_effect| move_effect.zone == Zone::Battlefield)
}

fn with_id_sacrifice(effect: &Effect) -> Option<&crate::effects::WithIdEffect> {
    let with_id = effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    sacrifice_view(&with_id.effect)?;
    Some(with_id)
}

/// Join a typed producer/sacrifice with the immediately linked source
/// attachment. Producers preserve the explicit "then" ordering used by token
/// Equipment abilities; linked sacrifice clauses remain a conjunction. This
/// keeps the attachment in the same oracle instruction rather than emitting a
/// misleading new sentence.
fn describe_linked_source_attachment_prefix(effects: &[Effect]) -> Option<String> {
    let [first, second, rest @ ..] = effects else {
        return None;
    };
    let attach = structural_unwrap_render_wrappers(second)
        .downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    if !choose_spec_is_source(&attach.objects) {
        return None;
    }

    let linked_producer = match attach.target.unhinted() {
        ChooseSpec::Tagged(tag) => effect_produces_attachment_target(first, tag),
        _ => false,
    };
    let linked_counter = match attach.target.unhinted() {
        ChooseSpec::Tagged(tag) => structural_unwrap_render_wrappers(first)
            .downcast_ref::<crate::effects::PutCountersEffect>()
            .is_some_and(|put| {
                !put.distributed
                    && put.target_count.is_none()
                    && choose_spec_references_exact_tag(&put.target, tag)
            }),
        _ => false,
    };
    let linked_sacrifice = with_id_sacrifice(first).is_some_and(|with_id| {
        rest.first()
            .and_then(|effect| effect.downcast_ref::<crate::effects::IfEffect>())
            .is_some_and(|if_effect| if_effect.condition == with_id.id)
    });
    if !linked_producer && !linked_counter && !linked_sacrifice {
        return None;
    }

    let first = describe_effect(first)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let attach = lowercase_first(describe_effect(second).trim().trim_end_matches('.'));
    let connector = if linked_producer { ", then" } else { " and" };
    let prefix = format!("{first}{connector} {attach}");
    if rest.is_empty() {
        return Some(prefix);
    }
    let suffix = describe_effect_clause_list(rest).unwrap_or_else(|| describe_effect_list(rest));
    Some(format!(
        "{prefix}. {}",
        capitalize_first(suffix.trim().trim_end_matches('.'))
    ))
}

pub(super) fn create_token_attachment_can_compact(
    create_token: &crate::effects::CreateTokenEffect,
) -> bool {
    create_token.count == Value::Fixed(1)
        && matches!(&create_token.controller, PlayerFilter::You)
        && create_token.controller_target.is_none()
        && !create_token.enters_tapped
        && !create_token.enters_attacking
        && !create_token.exile_at_end_of_combat
        && !create_token.sacrifice_at_end_of_combat
        && !create_token.sacrifice_at_next_end_step
        && !create_token.exile_at_next_end_step
        && (create_token.token.card.subtypes.contains(&Subtype::Aura)
            || create_token.token.card.subtypes.contains(&Subtype::Role))
}

pub(super) fn describe_create_token_attached_to_target(
    create_effect: &Effect,
    attach_effect: &Effect,
) -> Option<String> {
    let (created_tag, create_token) = tagged_create_token_effect(create_effect)?;
    let attach = unwrap_for_each_attachment_wrappers(attach_effect)
        .downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    if !create_token_attachment_can_compact(create_token)
        || !choose_spec_references_exact_tag(&attach.objects, created_tag)
    {
        return None;
    }

    let token = with_indefinite_article(&describe_create_token_blueprint(create_token));
    Some(format!(
        "Create {token} attached to {}",
        describe_choose_spec(&attach.target)
    ))
}

/// Render a tagged single-target selection followed by an until-end-of-turn
/// cast grant on the same tag. Graveyard targets use the compact Oracle surface
/// "You may cast target ... from your graveyard this turn"; other structural
/// uses retain the explicit choose-then-grant form.
pub(super) fn describe_target_card_then_cast_this_turn_structural(
    effects: &[Effect],
) -> Option<String> {
    let [target_effect, grant_effect] = effects else {
        return None;
    };
    let (target_tag, target_only) = tagged_target_only_effect(target_effect)?;
    let grant = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    if &grant.tag != target_tag
        || grant.player != PlayerFilter::You
        || grant.duration != crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
        || grant.allow_land
        || grant.allow_any_color_for_cast
        || grant.while_on_top_of_library
        || grant.filter.is_some()
        || grant.cast_pool_is_plural
        || choose_spec_is_plural(&target_only.target)
        || choose_spec_allows_multiple(&target_only.target)
    {
        return None;
    }
    let target_text = describe_choose_spec(&target_only.target);
    if target_only.target.is_target()
        && matches!(
            target_only.target.base(),
            ChooseSpec::Object(filter)
                if filter.zone == Some(Zone::Graveyard)
                    && filter.owner == Some(PlayerFilter::You)
        )
    {
        return Some(format!(
            "You may cast {} this turn",
            target_text.replace(" in your graveyard", " from your graveyard")
        ));
    }
    Some(format!(
        "Choose {}. You may cast that card this turn",
        target_text
    ))
}

pub(super) fn describe_choose_top_exile_then_play_structural(effects: &[Effect]) -> Option<String> {
    let [choose_effect, exile_effect, grant_effect] = effects else {
        return None;
    };
    let choose = unwrap_basic_tag_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let exile =
        unwrap_basic_tag_wrappers(exile_effect).downcast_ref::<crate::effects::ExileEffect>()?;
    let grant = unwrap_basic_tag_wrappers(grant_effect)
        .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    if choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Library)
        || choose.filter.owner != Some(PlayerFilter::You)
        || !choose.top_only
        || choose_exact_count(choose) != Some(1)
        || !matches!(exile.spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
        || exile.face_down
        || (grant.tag != choose.tag && grant.tag.as_str() != crate::tag::SOURCE_EXILED_TAG)
        || grant.player != PlayerFilter::You
        || grant.while_on_top_of_library
        || grant.filter.is_some()
        || grant.cast_pool_is_plural
    {
        return None;
    }

    let verb = if grant.allow_land { "play" } else { "cast" };
    let spell_ref = if grant.allow_land {
        "that spell"
    } else {
        "that card"
    };
    let mana_suffix = grant
        .mana_spend_cast_clause(spell_ref)
        .map(|clause| format!(", and {clause}"))
        .unwrap_or_default();
    let permission = if let Some(counter_type) = grant.during_turns_counter_put_on_source {
        format!(
            "During any turn you put {} on this Saga, you may {verb} that card{mana_suffix}",
            with_indefinite_article(&format!("{} counter", counter_type.description()))
        )
    } else {
        match grant.duration {
            crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn => {
                format!("You may {verb} that card this turn{mana_suffix}")
            }
            crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd => {
                format!("You may {verb} that card until the end of your next turn{mana_suffix}")
            }
            crate::effects::GrantPlayTaggedDuration::UntilYourNextEndStep => {
                format!("You may {verb} that card until your next end step{mana_suffix}")
            }
            _ => return None,
        }
    };
    Some(format!("Exile the top card of your library. {permission}"))
}

pub(super) fn describe_target_modifications_then_exile_top_play(
    effects: &[Effect],
) -> Option<String> {
    let [
        first_modification,
        second_modification,
        exile_effect,
        grant_effect,
    ] = effects
    else {
        return None;
    };
    let modification_text = capitalize_first(&describe_compact_tagged_apply_continuous_pair(
        first_modification,
        second_modification,
    )?);
    let exile = exile_effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    let grant = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    let [moved_tag] = exile.moved_tags.as_slice() else {
        return None;
    };
    if exile.count != Value::Fixed(1)
        || exile.player != PlayerFilter::You
        || !exile.accumulated_tags.is_empty()
        || grant.tag != *moved_tag
        || grant.player != PlayerFilter::You
        || grant.allow_any_color_for_cast
        || grant.while_on_top_of_library
        || grant.filter.is_some()
        || grant.cast_pool_is_plural
    {
        return None;
    }

    let duration = match grant.duration {
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn => "this turn",
        crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd => {
            "until the end of your next turn"
        }
        crate::effects::GrantPlayTaggedDuration::UntilYourNextEndStep => "until your next end step",
        _ => return None,
    };
    let verb = if grant.allow_land { "play" } else { "cast" };
    let (exile_text, singular) = describe_exile_top_clause(exile, false)?;
    if !singular {
        return None;
    }

    Some(format!(
        "{modification_text}. {exile_text}. You may {verb} it {duration}"
    ))
}

pub(super) fn describe_draw_replacement_exile_top_play(
    player: &PlayerFilter,
    effects: &[Effect],
) -> Option<String> {
    let grant = match effects {
        [choose_effect, exile_effect, grant_effect] => {
            let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            let exile = exile_effect.downcast_ref::<crate::effects::ExileEffect>()?;
            let grant = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
            if &choose.chooser != player
                || choose_primary_zone(choose) != Some(Zone::Library)
                || choose.filter.owner.as_ref() != Some(player)
                || !choose.top_only
                || choose_exact_count(choose) != Some(1)
                || !matches!(exile.spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
                || exile.face_down
                || (grant.tag != choose.tag && grant.tag.as_str() != crate::tag::SOURCE_EXILED_TAG)
            {
                return None;
            }
            grant
        }
        [exile_top_effect, grant_effect] => {
            let exile_top =
                exile_top_effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
            let grant = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
            let [moved_tag] = exile_top.moved_tags.as_slice() else {
                return None;
            };
            if &exile_top.player != player
                || !matches!(&exile_top.count, Value::Fixed(1))
                || !exile_top.accumulated_tags.is_empty()
                || grant.tag != *moved_tag
                || grant.allow_any_color_for_cast
                || grant.while_on_top_of_library
                || grant.filter.is_some()
                || grant.cast_pool_is_plural
            {
                return None;
            }
            grant
        }
        _ => return None,
    };
    if grant.duration != crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
        || !grant.allow_land
    {
        return None;
    }
    let grants_to_replacement_player = grant.player == *player
        || matches!(
            &grant.player,
            PlayerFilter::OwnerOf(crate::target::ObjectRef::Tagged(tag))
                if tag.as_str() == crate::tag::SOURCE_EXILED_TAG
        );
    if !grants_to_replacement_player {
        return None;
    }

    let subject = if *player == PlayerFilter::IteratedPlayer {
        "they".to_string()
    } else {
        describe_player_filter(player)
    };
    let possessive = if *player == PlayerFilter::IteratedPlayer {
        "their".to_string()
    } else {
        describe_possessive_player_filter(player)
    };
    let verb = if subject == "they" {
        "exile"
    } else {
        player_verb(&subject, "exile", "exiles")
    };
    Some(format!(
        "{subject} {verb} the top card of {possessive} library. {} may play it this turn",
        capitalize_first(&subject)
    ))
}

pub(super) fn describe_choose_top_exile_then_conditional_cast_structural(
    effects: &[Effect],
) -> Option<String> {
    fn is_nonland_card_filter(filter: &ObjectFilter) -> bool {
        if filter.excluded_card_types.as_slice() != [CardType::Land] {
            return false;
        }
        let mut base = filter.clone();
        base.excluded_card_types.clear();
        base.zone = None;
        base == ObjectFilter::default()
    }

    fn choose_subject(chooser: &PlayerFilter) -> (String, String) {
        if matches!(chooser, PlayerFilter::You) {
            return ("you".to_string(), "your".to_string());
        }
        if matches!(
            chooser,
            PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag))
                if tag.as_str() == "triggering"
        ) {
            return ("that player".to_string(), "their".to_string());
        }
        (
            describe_player_filter(chooser),
            describe_possessive_player_filter(chooser),
        )
    }

    let (player, exiled_tag, conditional, allow_source_exiled_alias) = match effects {
        [choose_effect, exile_effect, conditional_effect] => {
            let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            let exile = exile_effect.downcast_ref::<crate::effects::ExileEffect>()?;
            let conditional =
                conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
            if choose.is_search
                || choose_primary_zone(choose) != Some(Zone::Library)
                || choose.filter.owner.as_ref() != Some(&choose.chooser)
                || !choose.top_only
                || choose_exact_count(choose) != Some(1)
                || !matches!(exile.spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
                || exile.face_down
            {
                return None;
            }
            (&choose.chooser, &choose.tag, conditional, true)
        }
        [exile_top_effect, conditional_effect] => {
            let exile_top =
                exile_top_effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
            let conditional =
                conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
            let [moved_tag] = exile_top.moved_tags.as_slice() else {
                return None;
            };
            if exile_top.count != Value::Fixed(1) || !exile_top.accumulated_tags.is_empty() {
                return None;
            }
            (&exile_top.player, moved_tag, conditional, false)
        }
        _ => return None,
    };
    if !conditional.if_false.is_empty() {
        return None;
    }

    let Condition::TaggedObjectMatches(condition_tag, filter) = &conditional.condition else {
        return None;
    };
    if condition_tag != exiled_tag
        && !(allow_source_exiled_alias && condition_tag.as_str() == crate::tag::SOURCE_EXILED_TAG)
    {
        return None;
    }
    if !is_nonland_card_filter(filter) {
        return None;
    }

    let [may_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let may = unwrap_basic_tag_wrappers(may_effect).downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast = unwrap_basic_tag_wrappers(cast_effect)
        .downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if cast.tag != *exiled_tag
        && !(allow_source_exiled_alias && cast.tag.as_str() == crate::tag::SOURCE_EXILED_TAG)
    {
        return None;
    }
    if cast.player != PlayerFilter::You
        || cast.allow_land
        || cast.as_copy
        || !cast.without_paying_mana_cost
    {
        return None;
    }

    let (subject, possessive) = choose_subject(player);
    let exile_sentence = if subject == "you" {
        "Exile the top card of your library".to_string()
    } else {
        format!(
            "{subject} {} the top card of {possessive} library",
            player_verb(&subject, "exile", "exiles")
        )
    };
    Some(format!(
        "{exile_sentence}. If it's a nonland card, you may cast it without paying its mana cost"
    ))
}

pub(in crate::compiled_text) fn describe_choose_name_target_mills_conditional_draw(
    effects: &[Effect],
) -> Option<String> {
    if let [producer_effect, matched_effect, fallback_effect] = effects
        && let Some(rendered) = describe_result_gated_choose_name_mill_draw(
            producer_effect,
            matched_effect,
            fallback_effect,
        )
    {
        return Some(rendered);
    }

    let [
        choose_effect,
        target_effect,
        mill_effect,
        conditional_effect,
    ] = effects
    else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseCardNameEffect>()?;
    if choose.chooser != PlayerFilter::You || choose.filter.is_some() {
        return None;
    }
    let target = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target.target != ChooseSpec::target_player() {
        return None;
    }
    let tagged_mill = mill_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let mill = tagged_mill
        .effect
        .downcast_ref::<crate::effects::MillEffect>()?;
    if mill.count != Value::Fixed(1) || mill.player != PlayerFilter::target_player() {
        return None;
    }
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let crate::effect::Condition::TaggedObjectMatches(milled_tag, filter) = &conditional.condition
    else {
        return None;
    };
    if milled_tag != &tagged_mill.tag {
        return None;
    }
    let mut expected_filter = ObjectFilter::default();
    expected_filter
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: choose.tag.clone(),
            relation: crate::filter::TaggedOpbjectRelation::SameNameAsTagged,
        });
    if filter != &expected_filter {
        return None;
    }
    let [draw_two_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let [draw_one_effect] = conditional.if_false.as_slice() else {
        return None;
    };
    let draw_two = draw_two_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let draw_one = draw_one_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw_two.player != PlayerFilter::You
        || draw_two.count != Value::Fixed(2)
        || draw_one.player != PlayerFilter::You
        || draw_one.count != Value::Fixed(1)
    {
        return None;
    }

    Some(
        "Choose a card name, then target player mills a card. If a card with the chosen name was milled this way, draw two cards. Otherwise, draw a card"
            .to_string(),
    )
}

/// Modern lowering assigns an ID to the whole choose/target/mill sequence,
/// then assigns a second ID to the successful result predicate so the inverse
/// branch can refer to that exact observation. Keep that typed two-stage gate
/// as an Oracle "Otherwise" branch instead of exposing either internal ID.
fn describe_result_gated_choose_name_mill_draw(
    producer_effect: &Effect,
    matched_effect: &Effect,
    fallback_effect: &Effect,
) -> Option<String> {
    let producer_with_id = producer_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let producer = producer_with_id
        .effect
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    if producer.surface != ironsmith_core::SequenceSurface::CommaThen {
        return None;
    }
    let [choose_effect, target_effect, mill_effect] = producer.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseCardNameEffect>()?;
    if choose.chooser != PlayerFilter::You || choose.filter.is_some() {
        return None;
    }
    let target = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target.target != ChooseSpec::target_player() {
        return None;
    }
    let tagged_mill = mill_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let mill = tagged_mill
        .effect
        .downcast_ref::<crate::effects::MillEffect>()?;
    if mill.count != Value::Fixed(1) || mill.player != PlayerFilter::target_player() {
        return None;
    }

    let matched_with_id = matched_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let matched = matched_with_id
        .effect
        .downcast_ref::<crate::effects::IfEffect>()?;
    if matched.condition != producer_with_id.id || !matched.else_.is_empty() {
        return None;
    }
    let EffectPredicate::PriorEffectResult(result) = &matched.predicate else {
        return None;
    };
    if result.action != crate::effect::PriorEffectAction::Milled
        || result.actor != crate::effect::PriorEffectResultActor::Passive
        || result.quantifier != crate::effect::PriorEffectResultQuantifier::One
        || result.required_count.is_some()
        || result.shared_characteristic.is_some()
    {
        return None;
    }
    let mut expected_filter = ObjectFilter::default();
    expected_filter.set_explicit_card_noun(true);
    expected_filter
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: choose.tag.clone(),
            relation: crate::filter::TaggedOpbjectRelation::SameNameAsTagged,
        });
    if result.filter != expected_filter {
        return None;
    }
    let [draw_two_effect] = matched.then.as_slice() else {
        return None;
    };
    let draw_two = draw_two_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw_two.player != PlayerFilter::You || draw_two.count != Value::Fixed(2) {
        return None;
    }

    let fallback = fallback_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if fallback.condition != matched_with_id.id
        || fallback.predicate != EffectPredicate::DidNotHappen
        || !fallback.else_.is_empty()
    {
        return None;
    }
    let [draw_one_effect] = fallback.then.as_slice() else {
        return None;
    };
    let draw_one = draw_one_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw_one.player != PlayerFilter::You || draw_one.count != Value::Fixed(1) {
        return None;
    }

    Some(
        "Choose a card name, then target player mills a card. If a card with the chosen name was milled this way, draw two cards. Otherwise, draw a card"
            .to_string(),
    )
}

pub(in crate::compiled_text) fn describe_exile_then_free_cast_while_exiled_structural(
    effects: &[Effect],
) -> Option<String> {
    let [move_effect, grant_play_effect, grant_free_cast_effect] = effects else {
        return None;
    };
    let tag = structural_effect_tag(move_effect)?;
    let move_to_zone = unwrap_structural_effect_tag(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let grant_play = grant_play_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    let grant_free_cast = grant_free_cast_effect
        .downcast_ref::<crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect>(
    )?;
    if move_to_zone.zone != Zone::Exile
        || grant_play.tag != *tag
        || grant_free_cast.tag != *tag
        || grant_play.player != grant_free_cast.player
        || grant_play.duration != crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled
        || grant_free_cast.duration != crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled
        || grant_play.allow_land
        || grant_play.allow_any_color_for_cast
        || grant_free_cast.zone != Some(Zone::Exile)
        || grant_free_cast.while_on_top_of_library
    {
        return None;
    }
    if !matches!(
        grant_play.player,
        PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(ref owner_tag)) if owner_tag == tag
    ) {
        return None;
    }

    Some(format!(
        "Exile {}. For as long as that card remains exiled, its owner may cast it without paying its mana cost",
        describe_choose_spec(&move_to_zone.target)
    ))
}

pub(super) fn tagged_damage_view(
    effect: &Effect,
) -> Option<(&TagKey, &crate::effects::DealDamageEffect)> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        let damage = tagged
            .effect
            .downcast_ref::<crate::effects::DealDamageEffect>()?;
        return Some((&tagged.tag, damage));
    }
    None
}

pub(super) fn damage_each_creature_filter_text(filter: &ObjectFilter) -> Option<String> {
    if filter.zone.is_some_and(|zone| zone != Zone::Battlefield)
        || filter.card_types != vec![CardType::Creature]
        || filter.controller.is_some()
        || !filter.static_abilities.is_empty()
        || !filter.any_of.is_empty()
    {
        return None;
    }
    if filter.excluded_static_abilities == vec![crate::static_abilities::StaticAbilityId::Flying] {
        return Some("each creature without flying".to_string());
    }
    if filter.excluded_static_abilities.is_empty() {
        return Some("each creature".to_string());
    }
    None
}

pub(super) fn filter_references_tag(filter: &ObjectFilter, tag: &TagKey) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *tag
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    | crate::filter::TaggedOpbjectRelation::SameStableId
            )
    })
}

pub(in crate::compiled_text) fn describe_may_choose_pay_for_each_then_untap_tagged(
    effects: &[&Effect],
) -> Option<String> {
    let [may_effect, if_effect] = effects else {
        return None;
    };
    let with_id = may_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let (may, for_each_effect) =
        if let Some(may) = with_id.effect.downcast_ref::<crate::effects::MayEffect>() {
            let [_, for_each_effect] = may.effects.as_slice() else {
                return None;
            };
            (may, for_each_effect)
        } else {
            let sequence = with_id
                .effect
                .downcast_ref::<crate::effects::SequenceEffect>()?;
            if sequence.surface != ironsmith_core::SequenceSurface::Coordinated {
                return None;
            }
            let [may_effect, for_each_effect] = sequence.effects.as_slice() else {
                return None;
            };
            let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
            (may, for_each_effect)
        };
    let decider = may.decider.as_ref()?;
    let conditional = if_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if conditional.condition != with_id.id
        || conditional.predicate != crate::effect::EffectPredicate::Happened
        || !conditional.else_.is_empty()
    {
        return None;
    }

    let choose_effect = may.effects.first()?;
    if may.effects.len() != 1
        && with_id
            .effect
            .downcast_ref::<crate::effects::MayEffect>()
            .is_none()
    {
        return None;
    }
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [pay_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let pay = pay_effect.downcast_ref::<crate::effects::PayManaEffect>()?;
    let [untap_effect] = conditional.then.as_slice() else {
        return None;
    };
    let untap = untap_effect.downcast_ref::<crate::effects::UntapEffect>()?;

    if choose.is_search
        || choose.top_only
        || choose.chooser != *decider
        || for_each.tag != choose.tag
    {
        return None;
    }
    let ChooseSpec::Player(pay_player) = &pay.player else {
        return None;
    };
    if pay_player != decider && *pay_player != PlayerFilter::IteratedPlayer {
        return None;
    }
    if !choose_spec_is_tagged_object(&untap.target, &choose.tag) {
        return None;
    }

    let mut selected_filter = choose.filter.clone();
    if selected_filter.controller != Some(decider.clone()) {
        return None;
    }
    selected_filter.controller = None;
    let mut selected = selected_filter.description();
    if let Some(rest) = selected.strip_prefix("a ") {
        selected = rest.to_string();
    }
    if let Some(rest) = selected.strip_prefix("an ") {
        selected = rest.to_string();
    }
    if let Some(rest) = selected.strip_suffix(" on the battlefield") {
        selected = rest.to_string();
    }
    selected = selected.replace("nongreen tapped ", "tapped nongreen ");

    let selection = if choose.count.min == 0 && choose.count.max.is_none() {
        format!("any number of {}", pluralize_noun_phrase(&selected))
    } else {
        describe_choose_selection(choose)
    };
    let chooser = describe_player_filter(decider);
    let controlled_by = if *decider == PlayerFilter::You {
        "you control"
    } else {
        "they control"
    };
    let if_player = if chooser == "that player" {
        "the player"
    } else {
        chooser.as_str()
    };
    let chosen_noun = describe_iterated_object_reference_noun(&choose.filter);
    let chosen_plural = pluralize_noun_phrase(chosen_noun);

    Some(format!(
        "{chooser} may choose {selection} {controlled_by} and pay {} for each \
         {chosen_noun} chosen this way. If {if_player} does, untap those {chosen_plural}",
        pay.cost.to_oracle()
    ))
}

pub(super) fn describe_each_creature_and_player_damage_cant_regenerate_structural(
    effects: &[Effect],
) -> Option<String> {
    let [for_each_effect, for_players_effect, cant_effect] = effects else {
        return None;
    };
    let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachObject>()?;
    if for_each.effects.len() != 1 {
        return None;
    }
    let (damaged_tag, creature_damage) = tagged_damage_view(&for_each.effects[0])?;
    if !matches!(creature_damage.target, ChooseSpec::Iterated) {
        return None;
    }
    let creature_text = damage_each_creature_filter_text(&for_each.filter)?;

    let for_players = for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::Any || for_players.effects.len() != 1 {
        return None;
    }
    let player_damage =
        for_players.effects[0].downcast_ref::<crate::effects::DealDamageEffect>()?;
    if player_damage.amount != creature_damage.amount
        || !matches!(
            player_damage.target,
            ChooseSpec::Player(PlayerFilter::IteratedPlayer)
        )
    {
        return None;
    }

    let cant = cant_effect.downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::BeRegenerated(cant_filter) = &cant.restriction else {
        return None;
    };
    if cant.duration != Until::EndOfTurn
        || cant_filter.card_types != vec![CardType::Creature]
        || !filter_references_tag(cant_filter, damaged_tag)
    {
        return None;
    }

    Some(format!(
        "Deal {} damage to {creature_text} and each player. Creatures dealt damage this way can't be regenerated this turn",
        describe_value(&creature_damage.amount)
    ))
}

pub(in crate::compiled_text) fn describe_choose_color_then_chosen_color_mana(
    effects: &[&Effect],
) -> Option<String> {
    let [choose_effect, mana_effect] = effects else {
        return None;
    };
    let choose_color = choose_effect.downcast_ref::<crate::effects::ChooseColorEffect>()?;
    let add_mana = mana_effect.downcast_ref::<crate::effects::AddManaOfChosenColorEffect>()?;
    if choose_color.chooser != PlayerFilter::You
        || add_mana.player != PlayerFilter::You
        || add_mana.fixed_option.is_some()
    {
        return None;
    }
    if let Value::DistinctPowers(filter) = &add_mana.amount {
        return Some(format!(
            "Choose a color. Add one mana of that color for each different power among {}",
            pluralize_noun_phrase(&describe_for_each_count_filter(filter))
        ));
    }
    if add_mana.amount.surface_hints() == [ValueSurfaceHint::ForEach]
        && let Value::CountersOnSource(counter_type) = add_mana.amount.unhinted()
    {
        return Some(format!(
            "Choose a color. Add one mana of that color for each {} counter on this permanent",
            describe_counter_type(*counter_type)
        ));
    }
    None
}

pub(super) fn describe_revealed_cards_opponent_may_put_or_draw(
    effects: &[&Effect],
) -> Option<String> {
    let [look_effect, may_effect, fallback_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    if !look.reveal || look.player != PlayerFilter::You {
        return None;
    }

    let with_id = may_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if !matches!(
        may.decider.as_ref(),
        Some(PlayerFilter::Target(inner)) if matches!(inner.as_ref(), PlayerFilter::Opponent)
    ) {
        return None;
    }
    let [target_effect, hand_effect] = may.effects.as_slice() else {
        return None;
    };
    let target = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if !matches!(
        target.target.base(),
        ChooseSpec::Player(PlayerFilter::Opponent)
    ) {
        return None;
    }
    let move_to_hand = hand_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_hand.zone != Zone::Hand
        || !matches!(move_to_hand.target.base(), ChooseSpec::Tagged(tag) if tag == &look.tag)
    {
        return None;
    }

    let if_effect = fallback_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != with_id.id
        || if_effect.predicate != EffectPredicate::DidNotHappen
        || !if_effect.else_.is_empty()
    {
        return None;
    }
    let [graveyard_effect, draw_effect] = if_effect.then.as_slice() else {
        return None;
    };
    let move_to_graveyard = unwrap_basic_tag_wrappers(graveyard_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_graveyard.zone != Zone::Graveyard
        || !matches!(move_to_graveyard.target.base(), ChooseSpec::Tagged(tag) if tag == &look.tag)
    {
        return None;
    }
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You {
        return None;
    }

    let is_single_card = matches!(look.count, Value::Fixed(1));
    let count_text = if is_single_card {
        "card".to_string()
    } else {
        describe_card_count(&look.count)
    };
    let object_text = if is_single_card {
        "that card"
    } else {
        "those cards"
    };
    Some(format!(
        "Reveal the top {count_text} of your library. Target opponent may choose to put {object_text} into your hand. If they don't, put {object_text} into your graveyard and draw {}",
        describe_card_count(&draw.count)
    ))
}

pub(super) fn tagged_exile_effect_tag(effect: &Effect) -> Option<&str> {
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    tagged
        .effect
        .downcast_ref::<crate::effects::ExileEffect>()
        .map(|_| tagged.tag.as_str())
}

pub(super) fn copied_spell_targets_tag(effect: &Effect, tag: &str) -> bool {
    let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() else {
        return false;
    };
    let Some(with_id) = tagged.effect.downcast_ref::<crate::effects::WithIdEffect>() else {
        return false;
    };
    with_id
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()
        .is_some_and(
            |copy| matches!(&copy.target, ChooseSpec::Tagged(copy_tag) if copy_tag.as_str() == tag),
        )
}

pub(super) fn may_cast_copy_targets_tag(effect: &Effect, tag: &str) -> bool {
    let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() else {
        return false;
    };
    let [cast_effect] = may.effects.as_slice() else {
        return false;
    };
    cast_effect
        .downcast_ref::<crate::effects::CastTaggedEffect>()
        .is_some_and(|cast| cast.as_copy && cast.tag.as_str() == tag)
}

pub(super) fn describe_player_gain_keyword(
    player: &PlayerFilter,
    keyword: &str,
    duration: &Until,
) -> String {
    let subject = describe_player_set_filter(player);
    let verb = match player {
        PlayerFilter::You
        | PlayerFilter::Any
        | PlayerFilter::Opponent
        | PlayerFilter::NotYou
        | PlayerFilter::Teammate => "gain",
        _ => "gains",
    };
    let duration_text = if *duration == Until::EndOfTurn {
        "until end of turn".to_string()
    } else {
        describe_until(duration)
    };
    format!("{subject} {verb} {keyword} {duration_text}")
}

pub(super) fn describe_player_protection_from_everything_pair(
    effects: &[&Effect],
) -> Option<String> {
    let [cant_effect, prevent_effect] = effects else {
        return None;
    };
    let cant = cant_effect.downcast_ref::<crate::effects::CantEffect>()?;
    let prevent =
        prevent_effect.downcast_ref::<crate::effects::PreventAllDamageToTargetEffect>()?;
    let crate::effect::Restriction::BeTargetedPlayer(player) = &cant.restriction else {
        return None;
    };
    let same_player = match prevent.target.base() {
        ChooseSpec::Player(prevent_player) => prevent_player == player,
        ChooseSpec::SourceController => player == &PlayerFilter::You,
        _ => false,
    };
    if !same_player
        || prevent.duration != cant.duration
        || prevent.damage_filter != crate::prevention::DamageFilter::all()
        || !prevent.follow_up_effects.is_empty()
    {
        return None;
    }

    Some(describe_player_gain_keyword(
        player,
        "protection from everything",
        &cant.duration,
    ))
}

pub(super) fn describe_life_lock_and_protection_from_everything(
    effects: &[&Effect],
) -> Option<String> {
    let [life_lock_effect, cant_target_effect, prevent_damage_effect] = effects else {
        return None;
    };
    let life_lock = life_lock_effect.downcast_ref::<crate::effects::CantEffect>()?;
    if life_lock.restriction != crate::effect::Restriction::ChangeLifeTotal(PlayerFilter::You)
        || !matches!(life_lock.start, crate::effect::RestrictionStart::Immediate)
    {
        return None;
    }
    let protection = describe_player_protection_from_everything_pair(&[
        cant_target_effect,
        prevent_damage_effect,
    ])?;
    if !protection.starts_with("you gain protection from everything ")
        || !protection.ends_with(&describe_until(&life_lock.duration))
    {
        return None;
    }
    let cant_target = cant_target_effect.downcast_ref::<crate::effects::CantEffect>()?;
    if cant_target.duration != life_lock.duration {
        return None;
    }

    Some(format!(
        "{}, your life total can't change and you gain protection from everything",
        capitalize_first(&describe_until(&life_lock.duration))
    ))
}

pub(super) fn describe_phase_out_then_life_lock_and_protection(
    effects: &[&Effect],
) -> Option<String> {
    let [
        phase_effect,
        life_lock_effect,
        cant_target_effect,
        prevent_damage_effect,
    ] = effects
    else {
        return None;
    };
    let phase = phase_effect.downcast_ref::<crate::effects::PhaseOutEffect>()?;
    let expected_filter = ObjectFilter::permanent_card()
        .in_zone(Zone::Battlefield)
        .you_control();
    let ChooseSpec::All(phase_filter) = phase.spec.base() else {
        return None;
    };
    if phase.duration != crate::effects::PhaseOutDuration::UntilNextUntap
        || phase.source_surface.is_some()
        || phase_filter != &expected_filter
    {
        return None;
    }
    let protected = describe_life_lock_and_protection_from_everything(&[
        life_lock_effect,
        cant_target_effect,
        prevent_damage_effect,
    ])?;
    Some(format!(
        "All permanents you control phase out, and {}",
        lowercase_first(&protected)
    ))
}

pub(super) fn numeric_roll_branch_label(predicate: &EffectPredicate) -> Option<String> {
    let EffectPredicate::Value(cmp) = predicate else {
        return None;
    };
    match cmp {
        Comparison::Equal(value) => Some(value.to_string()),
        Comparison::BetweenInclusive(min, max) => Some(format!("{min}—{max}")),
        _ => None,
    }
}

pub(super) fn unwrap_if_effect(effect: &Effect) -> Option<&crate::effects::IfEffect> {
    if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
        return Some(if_effect);
    }
    effect
        .downcast_ref::<crate::effects::WithIdEffect>()?
        .effect
        .downcast_ref::<crate::effects::IfEffect>()
}

fn labeled_numeric_result_branch(effects: &[Effect]) -> (Option<&str>, &[Effect]) {
    let [effect] = effects else {
        return (None, effects);
    };
    let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() else {
        return (None, effects);
    };
    let Some(label) = sequence.result_label.as_deref() else {
        return (None, effects);
    };
    if sequence.surface != ironsmith_core::SequenceSurface::Sequential
        || label.trim().is_empty()
        || sequence.effects.is_empty()
    {
        return (None, effects);
    }
    (Some(label.trim()), &sequence.effects)
}

fn roll_table_mass_exile_tag(effects: &[Effect]) -> Option<TagKey> {
    effects.iter().rev().find_map(|effect| {
        let tag = effect_outer_tag(effect)?;
        let exile = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::ExileEffect>()?;
        matches!(exile.spec.base(), ChooseSpec::All(_)).then(|| tag.clone())
    })
}

fn roll_table_chosen_target(effects: &[Effect]) -> Option<(TagKey, &'static str)> {
    effects.iter().rev().find_map(|effect| {
        let tag = effect_outer_tag(effect)?;
        let target_only = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
        let noun = tagged_reference_noun_from_target(&target_only.target)?;
        Some((tag.clone(), noun))
    })
}

fn describe_roll_branch_damage_to_chosen_target(
    effects: &[Effect],
    chosen_tag: &TagKey,
    chosen_noun: &str,
) -> Option<String> {
    let (damage_effect, trailing) = effects.split_first()?;
    let (source, damage) = damage_with_source_view(damage_effect)?;
    if damage.source_is_combat
        || damage.unpreventable
        || source.is_some()
        || !choose_spec_references_exact_tag(&damage.target, chosen_tag)
    {
        return None;
    }

    let damage_text = format!(
        "Deal {} damage to {chosen_noun}",
        describe_value(&damage.amount)
    );
    if trailing.is_empty() {
        return Some(damage_text);
    }
    let trailing_text = describe_effect_list(trailing);
    let trailing_text = trailing_text.trim().trim_end_matches('.');
    (!trailing_text.is_empty()).then(|| format!("{damage_text}. {trailing_text}"))
}

fn roll_branch_is_each_player_draw(effects: &[Effect]) -> bool {
    let [effect] = effects else {
        return false;
    };
    let Some(for_players) = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()
    else {
        return false;
    };
    let [draw_effect] = for_players.effects.as_slice() else {
        return false;
    };
    let Some(draw) = structural_unwrap_render_wrappers(draw_effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()
    else {
        return false;
    };
    for_players.filter == PlayerFilter::Any && draw.player == PlayerFilter::IteratedPlayer
}

fn describe_controller_draw_roll_branch(effects: &[Effect]) -> Option<String> {
    let [effect] = effects else {
        return None;
    };
    let draw = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You {
        return None;
    }
    let rendered = describe_effect(effect);
    let rendered = rendered.trim().trim_end_matches('.');
    let draw_tail = rendered
        .strip_prefix("Draw ")
        .or_else(|| rendered.strip_prefix("draw "))?;
    Some(format!("You draw {draw_tail}"))
}

fn describe_controller_draw_and_lose_roll_branch(effects: &[Effect]) -> Option<String> {
    let [effect] = effects else {
        return None;
    };
    let sequence = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    if !matches!(
        sequence.surface,
        ironsmith_core::SequenceSurface::ResultConjunction { .. }
    ) {
        return None;
    }
    let [draw_effect, lose_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let draw = structural_unwrap_render_wrappers(draw_effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let lose = structural_unwrap_render_wrappers(lose_effect)
        .downcast_ref::<crate::effects::LoseLifeEffect>()?;
    describe_draw_then_lose_life(draw, lose)
}

fn tagged_battlefield_return_view(effect: &Effect) -> Option<(TagKey, TagKey)> {
    let result_tag = effect_outer_tag(effect)?.clone();
    let moved = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let ChooseSpec::Tagged(target_tag) = moved.target.base() else {
        return None;
    };
    (moved.zone == Zone::Battlefield
        && moved.battlefield_controller == crate::effects::BattlefieldController::Owner
        && !moved.enters_tapped
        && !moved.enters_face_down)
        .then(|| (result_tag, target_tag.clone()))
}

fn tagged_exile_view(effect: &Effect) -> Option<(TagKey, TagKey)> {
    let result_tag = effect_outer_tag(effect)?.clone();
    let inner = structural_unwrap_render_wrappers(effect);
    if let Some(moved) = inner.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        let ChooseSpec::Tagged(target_tag) = moved.target.base() else {
            return None;
        };
        return (moved.zone == Zone::Exile && !moved.enters_face_down)
            .then(|| (result_tag, target_tag.clone()));
    }
    let exile = inner.downcast_ref::<crate::effects::ExileEffect>()?;
    let ChooseSpec::Tagged(target_tag) = exile.spec.base() else {
        return None;
    };
    (!exile.face_down).then(|| (result_tag, target_tag.clone()))
}

fn untagged_exile_target(effect: &Effect) -> Option<TagKey> {
    if effect_outer_tag(effect).is_some() {
        return None;
    }
    let inner = structural_unwrap_render_wrappers(effect);
    if let Some(moved) = inner.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        let ChooseSpec::Tagged(target_tag) = moved.target.base() else {
            return None;
        };
        return (moved.zone == Zone::Exile && !moved.enters_face_down).then(|| target_tag.clone());
    }
    let exile = inner.downcast_ref::<crate::effects::ExileEffect>()?;
    let ChooseSpec::Tagged(target_tag) = exile.spec.base() else {
        return None;
    };
    (!exile.face_down).then(|| target_tag.clone())
}

fn delayed_battlefield_return_target(effect: &Effect) -> Option<TagKey> {
    let schedule = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()?;
    schedule
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfEndStepTrigger>()?;
    let [return_effect] = schedule.effects.flattened_default_effects() else {
        return None;
    };
    let moved = structural_unwrap_render_wrappers(return_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let ChooseSpec::Tagged(target_tag) = moved.target.base() else {
        return None;
    };
    (moved.zone == Zone::Battlefield
        && moved.battlefield_controller == crate::effects::BattlefieldController::Owner
        && !moved.enters_tapped
        && !moved.enters_face_down)
        .then(|| target_tag.clone())
}

fn describe_mass_exile_roll_branch(effects: &[Effect], exiled_tag: &TagKey) -> Option<String> {
    if let [schedule] = effects
        && delayed_battlefield_return_target(schedule).as_ref() == Some(exiled_tag)
    {
        return Some(
            "Return those cards to the battlefield under their owner's control at the beginning of the next end step"
                .to_string(),
        );
    }

    let [return_effect, exile_effect, schedule] = effects else {
        return None;
    };
    let (returned_tag, return_target) = tagged_battlefield_return_view(return_effect)?;
    let delayed_target = delayed_battlefield_return_target(schedule)?;
    if return_target != *exiled_tag {
        return None;
    }
    let linked_reexile =
        tagged_exile_view(exile_effect).is_some_and(|(reexiled_tag, exile_target)| {
            exile_target == returned_tag && delayed_target == reexiled_tag
        }) || untagged_exile_target(exile_effect).is_some_and(|exile_target| {
            exile_target == returned_tag && delayed_target.as_str() == crate::tag::SOURCE_EXILED_TAG
        });
    if !linked_reexile {
        return None;
    }
    Some(
        "Return those cards to the battlefield under their owner's control, then exile them again. Return those cards to the battlefield under their owner's control at the beginning of the next end step"
            .to_string(),
    )
}

fn roll_prefix_uses_then(prefix: &str) -> bool {
    ["Choose ", "Exile "]
        .iter()
        .any(|head| prefix.starts_with(head))
}

pub(super) fn describe_roll_die_with_numeric_result_table(effects: &[Effect]) -> Option<String> {
    // Triggered abilities carry an internal snapshot tag before their visible
    // effects. It has no oracle-text surface, so let the same die-table
    // compactor handle both triggered and non-triggered result tables.
    let effects = match effects.split_first() {
        Some((first, rest))
            if first
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some() =>
        {
            rest
        }
        _ => effects,
    };
    if effects.len() < 2 {
        return None;
    }
    let roll_indices = effects
        .iter()
        .enumerate()
        .filter_map(|(idx, effect)| {
            effect
                .downcast_ref::<crate::effects::WithIdEffect>()?
                .effect
                .downcast_ref::<crate::effects::RollDieEffect>()
                .map(|_| idx)
        })
        .collect::<Vec<_>>();
    let [roll_idx] = roll_indices.as_slice() else {
        return None;
    };
    let roll_idx = *roll_idx;
    let roll_with_id = effects[roll_idx].downcast_ref::<crate::effects::WithIdEffect>()?;
    let branches = effects.get(roll_idx + 1..)?;
    if branches.is_empty() {
        return None;
    }

    if roll_idx == 0
        && let [roll_effect, branch_effect] = effects
        && let Some(if_effect) = unwrap_if_effect(branch_effect)
        && if_effect.condition == roll_with_id.id
        && if_effect.else_.is_empty()
        && let Some(condition) = describe_with_id_if_clause(roll_with_id, if_effect)
    {
        let branch = describe_result_branch_effect_list(&if_effect.then);
        return Some(format!(
            "{}. {}, {}",
            describe_effect(roll_effect).trim_end_matches('.'),
            condition,
            lowercase_first(branch.trim_end_matches('.'))
        ));
    }

    let prefix_effects = &effects[..roll_idx];
    let mass_exiled_tag = roll_table_mass_exile_tag(prefix_effects);
    let chosen_target = roll_table_chosen_target(prefix_effects);
    let table_contrasts_each_player_with_controller = branches.iter().any(|effect| {
        unwrap_if_effect(effect).is_some_and(|branch| {
            let (_, branch_effects) = labeled_numeric_result_branch(&branch.then);
            roll_branch_is_each_player_draw(branch_effects)
        })
    });
    let roll_text = describe_effect(&effects[roll_idx])
        .trim()
        .trim_end_matches('.')
        .to_string();
    let header = if prefix_effects.is_empty() {
        roll_text
    } else {
        let prefix = describe_effect_list(prefix_effects);
        let prefix = prefix.trim().trim_end_matches('.');
        if prefix.is_empty() {
            return None;
        }
        if roll_prefix_uses_then(prefix) {
            format!("{prefix}, then {}", lowercase_first(&roll_text))
        } else {
            format!("{prefix}. {roll_text}")
        }
    };
    let header = format!("{}.", header.trim_end_matches('.'));

    let mut lines = vec![header];
    for effect in branches {
        let if_effect = unwrap_if_effect(effect)?;
        if if_effect.condition != roll_with_id.id || !if_effect.else_.is_empty() {
            return None;
        }
        let numeric_label = numeric_roll_branch_label(&if_effect.predicate)?;
        let (authored_label, branch_effects) = labeled_numeric_result_branch(&if_effect.then);
        let branch = mass_exiled_tag
            .as_ref()
            .and_then(|tag| describe_mass_exile_roll_branch(branch_effects, tag))
            .or_else(|| {
                chosen_target.as_ref().and_then(|(tag, noun)| {
                    describe_roll_branch_damage_to_chosen_target(branch_effects, tag, noun)
                })
            })
            .or_else(|| describe_controller_draw_and_lose_roll_branch(branch_effects))
            .or_else(|| {
                table_contrasts_each_player_with_controller
                    .then(|| describe_controller_draw_roll_branch(branch_effects))
                    .flatten()
            })
            .unwrap_or_else(|| describe_result_branch_effect_list(branch_effects));
        let branch = capitalize_first(branch.trim_end_matches('.'));
        lines.push(if let Some(authored_label) = authored_label {
            format!(
                "{numeric_label} | {} — {branch}.",
                capitalize_first(authored_label)
            )
        } else {
            format!("{numeric_label} | {branch}.")
        });
    }

    Some(lines.join("\n"))
}

pub(super) fn describe_roll_die_then_scry_result(effects: &[Effect]) -> Option<String> {
    let [roll_effect, scry_effect] = effects else {
        return None;
    };
    let roll_with_id = roll_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let roll = roll_with_id
        .effect
        .downcast_ref::<crate::effects::RollDieEffect>()?;
    let scry = scry_effect.downcast_ref::<crate::effects::ScryEffect>()?;
    if roll.player != scry.player
        || !value_prefers_where_x(&scry.count)
        || !matches!(scry.count.unhinted(), Value::EffectValue(id) if *id == roll_with_id.id)
    {
        return None;
    }

    let scry_text = if scry.player == PlayerFilter::You {
        "Scry X, where X is the result".to_string()
    } else {
        let player = describe_player_filter(&scry.player);
        format!(
            "{player} {} X, where X is the result",
            player_verb(&player, "scry", "scries")
        )
    };
    Some(format!(
        "{}. {scry_text}",
        describe_effect(roll_effect).trim_end_matches('.')
    ))
}

pub(super) fn describe_each_opponent_exile_top_then_cast_until_eot_any_color(
    effects: &[Effect],
) -> Option<String> {
    let [for_players_effect, grant_effect] = effects else {
        return None;
    };
    let for_players = for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::Opponent || for_players.effects.len() != 1 {
        return None;
    }
    let exile_top =
        for_players.effects[0].downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    if exile_top.player != PlayerFilter::IteratedPlayer
        || exile_top.count != Value::Fixed(1)
        || !exile_top.moved_tags.is_empty()
        || exile_top.accumulated_tags.len() != 1
    {
        return None;
    }

    let grant = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    if grant.tag != exile_top.accumulated_tags[0]
        || grant.player != PlayerFilter::You
        || grant.duration != crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
        || grant.allow_land
        || !grant.allow_any_color_for_cast
        || grant.while_on_top_of_library
        || grant.filter.is_some()
    {
        return None;
    }

    let mana_clause = grant.mana_spend_cast_clause("those spells")?;
    Some(format!(
        "Exile the top card of each opponent's library. Until end of turn, you may cast spells from among those exiled cards, and {mana_clause}"
    ))
}

pub(super) fn describe_group_pump_then_conditional_untap(effects: &[Effect]) -> Option<String> {
    let [pump_effect, conditional_effect] = effects else {
        return None;
    };
    let pump_tag = wrapped_effect_tag(pump_effect)?;
    let pump = unwrap_basic_tag_wrappers(pump_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let target_spec = pump.target_spec.as_ref()?;
    if target_spec.is_target() {
        return None;
    }
    let group_noun = match target_spec.base() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter)
            if filter.card_types == [CardType::Creature] =>
        {
            "those creatures"
        }
        _ => return None,
    };

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !matches!(
        &conditional.condition,
        Condition::Not(inner) if matches!(inner.as_ref(), Condition::YourTurn)
    ) || !conditional.if_false.is_empty()
        || conditional.if_true.len() != 1
    {
        return None;
    }
    let untap = unwrap_basic_tag_wrappers(&conditional.if_true[0])
        .downcast_ref::<crate::effects::UntapEffect>()?;
    if !choose_spec_references_tagged_object(&untap.target, pump_tag) {
        return None;
    }

    Some(format!(
        "{}. If it's not your turn, untap {group_noun}",
        describe_effect(pump_effect).trim_end_matches('.')
    ))
}

pub(super) fn describe_destroy_then_color_conditional(
    destroy_effect: &Effect,
    conditional_effect: &Effect,
) -> Option<String> {
    let tagged_destroy = destroy_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let destroy = tagged_destroy
        .effect
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() {
        return None;
    }
    let crate::effect::Condition::TaggedObjectMatches(condition_tag, filter) =
        &conditional.condition
    else {
        return None;
    };
    if condition_tag != &tagged_destroy.tag {
        return None;
    }
    let colors = filter.colors?;
    let mut color_only = filter.clone();
    color_only.colors = None;
    if color_only != crate::target::ObjectFilter::default() {
        return None;
    }
    let color_text = describe_filter_color_alternatives(colors);
    if color_text.is_empty() {
        return None;
    }
    let noun = destroyed_target_reference_noun(&destroy.spec)?;
    let true_branch = lowercase_first(
        describe_effect_list(&conditional.if_true)
            .trim()
            .trim_end_matches('.'),
    );
    if true_branch.is_empty() {
        return None;
    }
    Some(format!(
        "{}. If that {noun} was {color_text}, {true_branch}",
        describe_effect(destroy_effect).trim_end_matches('.')
    ))
}

pub(super) fn describe_filter_color_alternatives(colors: crate::color::ColorSet) -> String {
    let mut names = Vec::new();
    if colors.contains(crate::color::Color::White) {
        names.push("white".to_string());
    }
    if colors.contains(crate::color::Color::Blue) {
        names.push("blue".to_string());
    }
    if colors.contains(crate::color::Color::Black) {
        names.push("black".to_string());
    }
    if colors.contains(crate::color::Color::Red) {
        names.push("red".to_string());
    }
    if colors.contains(crate::color::Color::Green) {
        names.push("green".to_string());
    }
    join_with_or(&names)
}

pub(super) fn destroyed_target_reference_noun(spec: &ChooseSpec) -> Option<&'static str> {
    let target = match spec.unhinted() {
        ChooseSpec::Target(inner) => inner.unhinted(),
        ChooseSpec::WithCount(inner, count) if count.is_single() => match inner.unhinted() {
            ChooseSpec::Target(target) => target.unhinted(),
            other => other,
        },
        _ => return None,
    };
    let ChooseSpec::Object(filter) = target else {
        return None;
    };
    // A card-type list on an ordinary target filter is an alternative domain
    // ("artifact or enchantment"). No single branch noun names every legal
    // target, so the back-reference must use the common permanent noun.
    if filter.card_types.len() > 1 {
        return Some("permanent");
    }
    if filter
        .card_types
        .contains(&crate::types::CardType::Creature)
    {
        Some("creature")
    } else if filter
        .card_types
        .contains(&crate::types::CardType::Artifact)
    {
        Some("artifact")
    } else if filter
        .card_types
        .contains(&crate::types::CardType::Enchantment)
    {
        Some("enchantment")
    } else if filter.card_types.contains(&crate::types::CardType::Land) {
        Some("land")
    } else if filter
        .card_types
        .contains(&crate::types::CardType::Planeswalker)
    {
        Some("planeswalker")
    } else {
        Some("permanent")
    }
}

#[cfg(test)]
mod destroyed_target_reference_noun_tests {
    use super::*;

    #[test]
    fn alternative_permanent_types_use_the_common_reference_noun() {
        let mut filter = ObjectFilter::default().in_zone(Zone::Battlefield);
        filter.card_types = vec![CardType::Artifact, CardType::Enchantment];
        let target = ChooseSpec::target(ChooseSpec::Object(filter));

        assert_eq!(destroyed_target_reference_noun(&target), Some("permanent"));
    }

    #[test]
    fn a_single_target_type_keeps_its_specific_reference_noun() {
        assert_eq!(
            destroyed_target_reference_noun(&ChooseSpec::target(ChooseSpec::Object(
                ObjectFilter::artifact().in_zone(Zone::Battlefield),
            ))),
            Some("artifact")
        );
    }
}

pub(super) fn describe_draw_then_for_players_choose_exile(effects: &[Effect]) -> Option<String> {
    let [draw_effect, for_players_effect] = effects else {
        return None;
    };
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You || draw.count != Value::Fixed(1) {
        return None;
    }
    let for_players = for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let exile_clause = describe_for_players_choose_then_exile(for_players)?;
    Some(format!("You draw a card. {exile_clause}"))
}

pub(super) fn describe_lose_life_then_endure(effects: &[Effect]) -> Option<String> {
    if let [effect] = effects
        && let Some(sequence) = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::SequenceEffect>()
    {
        return describe_lose_life_then_endure(&sequence.effects);
    }

    let visible = effects
        .iter()
        .filter(|effect| {
            effect
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_none()
                && effect
                    .downcast_ref::<crate::effects::TagTriggeringSourceEffect>()
                    .is_none()
                && effect
                    .downcast_ref::<crate::effects::TagTriggeringBlockersEffect>()
                    .is_none()
        })
        .collect::<Vec<_>>();
    let [lose_effect, endure_effect] = visible.as_slice() else {
        return None;
    };
    let lose = structural_unwrap_render_wrappers(lose_effect)
        .downcast_ref::<crate::effects::LoseLifeEffect>()?;
    if lose.player != ChooseSpec::Player(PlayerFilter::You) {
        return None;
    }
    let choose_mode = structural_unwrap_render_wrappers(endure_effect)
        .downcast_ref::<crate::effects::ChooseModeEffect>()?;
    let endure = describe_endure_mode(choose_mode)?;
    let amount = endure.strip_prefix("it endures ")?;
    Some(format!(
        "You lose {} life and it endures {amount}",
        describe_value(&lose.amount)
    ))
}

pub(crate) fn describe_tagged_target_then_conditional_action(effects: &[Effect]) -> Option<String> {
    let [target_effect, conditional_effect] = effects else {
        return None;
    };
    let (tag, target_only) = tagged_target_only_effect(target_effect)?;
    let conditional = structural_unwrap_render_wrappers(conditional_effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
        return None;
    }

    let target_text = describe_choose_spec(&target_only.target);
    let action_text = describe_conditional_action_on_tagged_target(
        &conditional.if_true[0],
        tag,
        &target_only.target,
        &target_text,
    )?;
    let condition_text = describe_condition_for_tagged_target(&conditional.condition, tag)?;
    Some(format!("{action_text} if {condition_text}"))
}

pub(super) fn describe_conditional_action_on_tagged_target(
    effect: &Effect,
    tag: &crate::TagKey,
    target: &ChooseSpec,
    target_text: &str,
) -> Option<String> {
    let effect = if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        if tagged.tag != *tag {
            return None;
        }
        tagged.effect.as_ref()
    } else {
        effect
    };

    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
        && move_to_zone.zone == Zone::Exile
    {
        return Some(format!("Exile {target_text}"));
    }
    if effect
        .downcast_ref::<crate::effects::CounterEffect>()
        .is_some()
    {
        return Some(format!("Counter {target_text}"));
    }
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyEffect>()
        && (matches!(destroy.spec.base(), ChooseSpec::Tagged(found) if found == tag)
            || target_specs_select_same_objects(&destroy.spec, target))
    {
        return Some(format!("Destroy {target_text}"));
    }
    None
}

pub(super) fn describe_condition_for_tagged_target(
    condition: &Condition,
    tag: &crate::TagKey,
) -> Option<String> {
    let target_is_plain_creature = |condition: &Condition| {
        let filter = match condition {
            Condition::TaggedObjectMatches(condition_tag, filter) if condition_tag == tag => filter,
            // A condition evaluated before the target is destroyed cannot use
            // the destroy-result tag yet. `TargetMatches` is the executable
            // pre-action form of the same announced-target relationship.
            Condition::TargetMatches(filter) => filter,
            _ => return false,
        };
        let mut plain = filter.clone();
        plain.zone = Some(Zone::Battlefield);
        plain.set_explicit_card_type_noun(None);
        plain == ObjectFilter::creature()
    };
    let exact_green_white_spent = |condition: &Condition| {
        let Condition::And(left, right) = condition else {
            return false;
        };
        let symbol = |condition: &Condition| match condition {
            Condition::ManaSpentToCastThisSpellAtLeast {
                amount: 1,
                symbol: Some(symbol),
            } => Some(*symbol),
            _ => None,
        };
        matches!(
            (symbol(left), symbol(right)),
            (
                Some(crate::mana::ManaSymbol::Green),
                Some(crate::mana::ManaSymbol::White)
            ) | (
                Some(crate::mana::ManaSymbol::White),
                Some(crate::mana::ManaSymbol::Green)
            )
        )
    };
    if let Condition::Or(left, right) = condition
        && ((target_is_plain_creature(left) && exact_green_white_spent(right))
            || (target_is_plain_creature(right) && exact_green_white_spent(left)))
    {
        return Some("it's a creature or if {G}{W} was spent to cast this spell".to_string());
    }

    if let Condition::TaggedObjectMatches(condition_tag, filter) = condition
        && condition_tag == tag
        && let Some(comparison) = filter.mana_value.as_ref()
    {
        let mut remainder = filter.clone();
        remainder.mana_value = None;
        if remainder == ObjectFilter::default() {
            let comparison = match comparison {
                ironsmith_core::FilterComparison::EqualExpr(value) => {
                    format!("is {}", describe_value(value))
                }
                comparison => describe_filter_comparison_clause(comparison),
            };
            return Some(format!("its mana value {comparison}"));
        }
    }

    if let Condition::PlayerControls { player, filter } = condition
        && let Some(constraint) = filter.tagged_constraints.iter().find(|constraint| {
            constraint.tag == *tag
                && constraint.relation
                    == crate::filter::TaggedOpbjectRelation::SharesColorWithTagged
        })
    {
        let _ = constraint;
        let mut base = filter.clone();
        base.tagged_constraints.retain(|constraint| {
            !(constraint.tag == *tag
                && constraint.relation
                    == crate::filter::TaggedOpbjectRelation::SharesColorWithTagged)
        });
        base.controller = None;
        let object = with_indefinite_article(strip_indefinite_article(&base.description()));
        let controller = match player {
            PlayerFilter::You => "you control".to_string(),
            PlayerFilter::Opponent => "an opponent controls".to_string(),
            _ => format!(
                "{} {}",
                describe_player_filter(player),
                player_verb(&describe_player_filter(player), "control", "controls")
            ),
        };
        return Some(format!("it shares a color with {object} {controller}"));
    }

    Some(lowercase_first(&describe_condition(condition)))
}

pub(super) fn normalize_each_becomes_plural_surface(text: &str) -> String {
    let Some(rest) = text.strip_prefix("Each ") else {
        return text.to_string();
    };
    let Some((subject, predicate)) = rest.split_once(" becomes ") else {
        return text.to_string();
    };

    let (complement, tail) = predicate
        .split_once(" until ")
        .map(|(complement, tail)| (complement, format!(" until {tail}")))
        .unwrap_or((predicate, String::new()));
    let complement = complement
        .strip_prefix("an ")
        .or_else(|| complement.strip_prefix("a "))
        .map(pluralize_noun_phrase)
        .unwrap_or_else(|| complement.to_string());
    format!(
        "{} become {}{}",
        pluralize_noun_phrase(subject),
        complement,
        tail
    )
}

fn aura_equipment_lki_provenance_tag(filter: &ObjectFilter) -> Option<TagKey> {
    if filter.zone != Some(Zone::Battlefield) {
        return None;
    }

    if filter.subtypes.len() == 2
        && filter.subtypes.contains(&Subtype::Aura)
        && filter.subtypes.contains(&Subtype::Equipment)
        && let [constraint] = filter.tagged_constraints.as_slice()
        && constraint.relation == crate::filter::TaggedOpbjectRelation::WasAttachedToTaggedObject
    {
        let mut plain = filter.clone();
        plain.zone = None;
        plain.subtypes.clear();
        plain.tagged_constraints.clear();
        if plain == ObjectFilter::default() {
            return Some(constraint.tag.clone());
        }
    }

    let [first, second] = filter.any_of.as_slice() else {
        return None;
    };
    let mut common_tag = None;
    let mut saw_aura = false;
    let mut saw_equipment = false;
    for arm in [first, second] {
        match arm.subtypes.as_slice() {
            [Subtype::Aura] if !saw_aura => saw_aura = true,
            [Subtype::Equipment] if !saw_equipment => saw_equipment = true,
            _ => return None,
        }
        let [constraint] = arm.tagged_constraints.as_slice() else {
            return None;
        };
        if constraint.relation != crate::filter::TaggedOpbjectRelation::WasAttachedToTaggedObject {
            return None;
        }
        if common_tag
            .as_ref()
            .is_some_and(|tag: &TagKey| tag != &constraint.tag)
        {
            return None;
        }
        common_tag = Some(constraint.tag.clone());

        let mut plain_arm = arm.clone();
        plain_arm.subtypes.clear();
        plain_arm.tagged_constraints.clear();
        if plain_arm != ObjectFilter::default() {
            return None;
        }
    }
    if !saw_aura || !saw_equipment {
        return None;
    }

    let mut plain = filter.clone();
    plain.zone = None;
    plain.any_of.clear();
    (plain == ObjectFilter::default())
        .then_some(common_tag)
        .flatten()
}

pub(super) fn describe_lki_control_choose_attach_sequence(effects: &[Effect]) -> Option<String> {
    let [continuous_effect, choose_effect, attach_effect] = effects else {
        return None;
    };
    let continuous = unwrap_basic_tag_wrappers(continuous_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.count.is_single() || choose.chooser != PlayerFilter::You {
        return None;
    }
    let attach = attach_effect.downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    if !matches!(&attach.target, ChooseSpec::Tagged(tag) if tag == &choose.tag) {
        return None;
    }

    let controlled_tag = direct_wrapped_effect_tag(continuous_effect)?;
    if continuous.until != Until::Forever
        || continuous.condition.is_some()
        || continuous.modification.is_some()
        || !continuous.additional_modifications.is_empty()
        || !matches!(
            continuous.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController]
        )
    {
        return None;
    }
    let crate::continuous::EffectTarget::Filter(controlled_filter) = &continuous.target else {
        return None;
    };
    aura_equipment_lki_provenance_tag(controlled_filter)?;
    let ChooseSpec::All(attached_objects) = attach.objects.base() else {
        return None;
    };
    let [controlled_constraint] = attached_objects.tagged_constraints.as_slice() else {
        return None;
    };
    if controlled_constraint.tag != *controlled_tag
        || controlled_constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
    {
        return None;
    }
    let mut plain_attached_objects = attached_objects.clone();
    plain_attached_objects.tagged_constraints.clear();
    if plain_attached_objects != ObjectFilter::default() {
        return None;
    }
    let mut choice_text = describe_choose_selection(choose);
    if let Some(base) = choice_text.strip_suffix(" on the battlefield") {
        choice_text = base.to_string();
    }
    Some(format!(
        "Gain control of all Auras and Equipment that were attached to it, then attach them to {choice_text}"
    ))
}

pub(super) fn describe_continuous_choose_attach_sequence(effects: &[Effect]) -> Option<String> {
    if let Some(compact) = describe_lki_control_choose_attach_sequence(effects) {
        return Some(compact);
    }
    let [continuous_effect, choose_effect, attach_effect] = effects else {
        return None;
    };
    unwrap_basic_tag_wrappers(continuous_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.count.is_single() || choose.chooser != PlayerFilter::You {
        return None;
    }
    let attach = attach_effect.downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    if !matches!(&attach.target, ChooseSpec::Tagged(tag) if tag == &choose.tag) {
        return None;
    }
    let continuous_text = normalize_each_becomes_plural_surface(
        describe_effect(continuous_effect).trim_end_matches('.'),
    );
    let mut choice_text = describe_choose_selection(choose);
    if let Some(base) = choice_text.strip_suffix(" on the battlefield") {
        choice_text = base.to_string();
    }
    Some(format!(
        "{continuous_text}. Choose {choice_text}. {}",
        describe_effect(attach_effect).trim_end_matches('.')
    ))
}

pub(super) fn describe_countered_spell_same_name_search_sequence(
    effects: &[Effect],
) -> Option<String> {
    let effects = if effects.first().is_some_and(|effect| {
        structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
            .is_some()
    }) {
        &effects[1..]
    } else {
        effects
    };
    let (core, draw_effect) = match effects.len() {
        4 => (effects, None),
        5 => (&effects[..4], Some(&effects[4])),
        _ => return None,
    };
    let [
        counter_effect,
        choose_effect,
        for_each_effect,
        shuffle_effect,
    ] = core
    else {
        return None;
    };
    let counter = unwrap_basic_tag_wrappers(counter_effect)
        .downcast_ref::<crate::effects::CounterEffect>()?;
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let same_name_constraints = choose
        .filter
        .tagged_constraints
        .iter()
        .filter(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
        })
        .collect::<Vec<_>>();
    if same_name_constraints.len() != 1
        || !choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose_search_zones(choose)? != [Zone::Graveyard, Zone::Hand, Zone::Library]
    {
        return None;
    }
    let counter_tag = wrapped_effect_tag(counter_effect)?;
    if same_name_constraints[0].tag != *counter_tag {
        return None;
    }
    let for_each = structural_unwrap_render_wrappers(for_each_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if !for_each_exiles_search_tag(for_each, &choose.tag) {
        return None;
    }
    let shuffle = structural_unwrap_render_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    let search_owner = choose.filter.owner.as_ref()?;
    if !player_is_controller_of_produced_target(search_owner, counter_tag)
        || !same_search_player_filter(&shuffle.player, search_owner)
    {
        return None;
    }
    let counter_text = describe_effect(counter_effect)
        .trim()
        .trim_end_matches('.')
        // A union of spell card types shares the final "spell" noun in
        // oracle text. The target filter remains typed as both alternatives;
        // this only removes the mechanically repeated surface noun.
        .replace(
            "target instant spell or sorcery spell",
            "target instant or sorcery spell",
        );
    if !counter.target.is_target() || !counter_text.starts_with("Counter target") {
        return None;
    }
    let search_origin = "its controller's graveyard, hand, and library";
    let selection = same_name_extraction_selection(choose)?;
    let prefix = format!(
        "{counter_text}. Search {search_origin} for {selection} with the same name as that spell and exile them"
    );
    if let Some(draw_effect) = draw_effect {
        if !same_name_extraction_hand_draw_matches(draw_effect, &choose.tag, search_owner) {
            return None;
        }
        Some(format!(
            "{prefix}. That player shuffles, then draws a card for each card exiled from their hand this way"
        ))
    } else {
        Some(format!("{prefix}. Then that player shuffles"))
    }
}

pub(super) fn describe_counter_and_damage_sequence(effects: &[Effect]) -> Option<String> {
    let [counter_effect, damage_effect] = effects else {
        return None;
    };
    unwrap_basic_tag_wrappers(counter_effect).downcast_ref::<crate::effects::CounterEffect>()?;
    unwrap_basic_tag_wrappers(damage_effect).downcast_ref::<crate::effects::DealDamageEffect>()?;

    let counter_text = describe_effect(unwrap_basic_tag_wrappers(counter_effect));
    let damage_text = lowercase_first(
        describe_effect(unwrap_basic_tag_wrappers(damage_effect))
            .trim_end_matches('.')
            .trim(),
    );
    Some(format!("{counter_text} and {damage_text}"))
}

pub(super) fn describe_put_counters_and_add_mana_sequence(effects: &[Effect]) -> Option<String> {
    let [counter_effect, mana_effect] = effects else {
        return None;
    };
    unwrap_basic_tag_wrappers(counter_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    unwrap_basic_tag_wrappers(mana_effect).downcast_ref::<crate::effects::AddManaEffect>()?;

    let counter_text = describe_effect(unwrap_basic_tag_wrappers(counter_effect));
    let mana_text = lowercase_first(
        describe_effect(unwrap_basic_tag_wrappers(mana_effect))
            .trim_end_matches('.')
            .trim(),
    );
    Some(format!("{counter_text} and {mana_text}"))
}

pub(super) fn describe_destroy_all_groups_then_draw_for_destroyed(
    effects: &[Effect],
) -> Option<String> {
    let (draw_effect, destroy_effects) = effects.split_last()?;
    if destroy_effects.len() < 2 {
        return None;
    }
    let draw =
        unwrap_basic_tag_wrappers(draw_effect).downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You || !is_effect_count_reference(&draw.count, None) {
        return None;
    }

    fn destroy_all_card_type(effect: &Effect) -> Option<CardType> {
        let destroy =
            unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::DestroyEffect>()?;
        let ChooseSpec::All(filter) = destroy.spec.base() else {
            return None;
        };
        [
            CardType::Creature,
            CardType::Enchantment,
            CardType::Artifact,
            CardType::Land,
            CardType::Planeswalker,
            CardType::Battle,
        ]
        .into_iter()
        .find(|card_type| filter_has_only_card_type(filter, *card_type))
    }

    let card_types = destroy_effects
        .iter()
        .map(destroy_all_card_type)
        .collect::<Option<Vec<_>>>()?;
    let mut unique = Vec::new();
    for card_type in card_types {
        if unique.contains(&card_type) {
            return None;
        }
        unique.push(card_type);
    }
    let groups = unique
        .iter()
        .map(|card_type| card_type.plural_name().to_string())
        .collect::<Vec<_>>();

    Some(format!(
        "Destroy all {}. Draw a card for each permanent destroyed this way",
        join_with_and(&groups)
    ))
}

pub(super) fn player_is_controller_reference(player: &PlayerFilter) -> bool {
    matches!(player, PlayerFilter::ControllerOf(_))
}

pub(super) fn player_filters_share_controller_reference(
    left: &PlayerFilter,
    right: &PlayerFilter,
) -> bool {
    left == right || (player_is_controller_reference(left) && player_is_controller_reference(right))
}

pub(super) fn filter_has_only_card_type(filter: &ObjectFilter, card_type: CardType) -> bool {
    filter.card_types.len() == 1
        && filter.card_types.contains(&card_type)
        && filter.all_card_types.is_empty()
        && filter.excluded_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.excluded_subtypes.is_empty()
}

pub(super) fn filter_has_only_card_types(filter: &ObjectFilter, card_types: &[CardType]) -> bool {
    filter.card_types.len() == card_types.len()
        && card_types
            .iter()
            .all(|card_type| filter.card_types.contains(card_type))
        && filter.all_card_types.is_empty()
        && filter.excluded_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.excluded_subtypes.is_empty()
}

pub(super) fn filter_any_of_has_exact_card_types(
    filter: &ObjectFilter,
    zone: Option<Zone>,
    card_types: &[CardType],
) -> bool {
    filter.any_of.len() == card_types.len()
        && card_types.iter().all(|card_type| {
            filter.any_of.iter().any(|branch| {
                (branch.zone == zone
                    || (branch.zone.is_none() && filter.zone == zone)
                    || (zone == Some(Zone::Stack)
                        && branch.stack_kind == Some(StackObjectKind::Spell)))
                    && filter_has_only_card_type(branch, *card_type)
                    && branch.any_of.is_empty()
            })
        })
}

pub(super) fn choose_spec_is_target_instant_or_sorcery_spell(spec: &ChooseSpec) -> bool {
    if !spec.is_target() {
        return false;
    }
    let ChooseSpec::Object(filter) = spec.base() else {
        return false;
    };
    let instant_or_sorcery = [CardType::Instant, CardType::Sorcery];
    let direct_instant_or_sorcery =
        filter.zone == Some(Zone::Stack) && filter_has_only_card_types(filter, &instant_or_sorcery);
    direct_instant_or_sorcery
        || filter_any_of_has_exact_card_types(filter, Some(Zone::Stack), &instant_or_sorcery)
}

pub(super) fn consult_filter_is_instant_or_sorcery_card(filter: &ObjectFilter) -> bool {
    let instant_or_sorcery = [CardType::Instant, CardType::Sorcery];
    filter.zone.is_none() && filter_has_only_card_types(filter, &instant_or_sorcery)
        || filter_any_of_has_exact_card_types(filter, None, &instant_or_sorcery)
}

pub(super) fn describe_countered_spell_controller_consult_cast_shuffle(
    effects: &[Effect],
) -> Option<String> {
    let [counter_effect, consult_effect, may_effect, shuffle_effect] = effects else {
        return None;
    };

    let counter = unwrap_basic_tag_wrappers(counter_effect)
        .downcast_ref::<crate::effects::CounterEffect>()?;
    if !choose_spec_is_target_instant_or_sorcery_spell(&counter.target) {
        return None;
    }

    let consult = unwrap_basic_tag_wrappers(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || !player_is_controller_reference(&consult.player)
        || !matches!(
            &consult.stop_rule,
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
                | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1))
        )
        || !consult_filter_is_instant_or_sorcery_card(&consult.filter)
    {
        return None;
    }

    let may = unwrap_basic_tag_wrappers(may_effect).downcast_ref::<crate::effects::MayEffect>()?;
    if !may
        .decider
        .as_ref()
        .is_some_and(|player| player_filters_share_controller_reference(player, &consult.player))
    {
        return None;
    }
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast = unwrap_basic_tag_wrappers(cast_effect)
        .downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if cast.tag != consult.match_tag
        || !player_filters_share_controller_reference(&cast.player, &consult.player)
        || cast.allow_land
        || cast.as_copy
        || !cast.without_paying_mana_cost
        || cast.cost_reduction.is_some()
    {
        return None;
    }

    let shuffle = unwrap_basic_tag_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    let shuffle_uses_controller =
        player_filters_share_controller_reference(&shuffle.player, &consult.player);
    let shuffle_uses_target_player = matches!(
        &shuffle.player,
        PlayerFilter::Target(inner) if matches!(inner.as_ref(), PlayerFilter::Any)
    );
    if !shuffle_uses_controller && !shuffle_uses_target_player {
        return None;
    }

    Some("Counter target instant or sorcery spell. Its controller reveals cards from the top of their library until they reveal an instant or sorcery card. That player may cast that card without paying its mana cost. Then the player shuffles".to_string())
}

pub(in crate::compiled_text) fn describe_choose_two_tap_then_unattach_equipment_sequence(
    effects: &[Effect],
) -> Option<String> {
    let [target_effect, tap_effect, unattach_effect] = effects else {
        return None;
    };
    let (target_tag, target_only) = tagged_target_only_effect(target_effect)?;
    let target_count = target_only.target.count();
    if target_count.min != 2
        || target_count.max != Some(2)
        || target_count.dynamic_x
        || target_count.up_to_x
        || target_count.random
        || !target_only.target.is_target()
    {
        return None;
    }
    let ChooseSpec::Object(target_filter) = target_only.target.base() else {
        return None;
    };
    if !target_filter.card_types.contains(&CardType::Creature) {
        return None;
    }

    let tap = unwrap_basic_tag_wrappers(tap_effect).downcast_ref::<crate::effects::TapEffect>()?;
    if !choose_spec_is_tagged_object(&tap.target, target_tag) {
        return None;
    }

    let unattach = unattach_effect.downcast_ref::<crate::effects::UnattachObjectsEffect>()?;
    if describe_unattach_all_equipment_from_tagged(&unattach.objects).is_none() {
        return None;
    }

    Some(
        "Choose two target creatures. Tap those creatures, then unattach all Equipment from them"
            .to_string(),
    )
}

pub(super) fn tagged_iterated_damage_tag_from_for_each(
    for_each: &crate::effects::ForEachObject,
) -> Option<&crate::TagKey> {
    if !for_each.filter.card_types.contains(&CardType::Creature) {
        return None;
    }
    let [damage_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let (tag, damage) = tagged_damage_view(damage_effect)?;
    if !matches!(damage.target.base(), ChooseSpec::Iterated) {
        return None;
    }
    Some(tag)
}

pub(super) fn describe_damage_each_then_tap_damaged_sequence(effects: &[Effect]) -> Option<String> {
    let (damage_effect, tap_effect) = match effects {
        [damage_effect, tap_effect] => (damage_effect, tap_effect),
        [tag_triggering, damage_effect, tap_effect]
            if tag_triggering
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some() =>
        {
            (damage_effect, tap_effect)
        }
        _ => return None,
    };

    let for_each =
        unwrap_basic_tag_wrappers(damage_effect).downcast_ref::<crate::effects::ForEachObject>()?;
    let damaged_tag = tagged_iterated_damage_tag_from_for_each(for_each)?;
    let tap = unwrap_basic_tag_wrappers(tap_effect).downcast_ref::<crate::effects::TapEffect>()?;
    if !choose_spec_is_tagged_object(&tap.target, damaged_tag) {
        return None;
    }

    Some(format!(
        "{}. Tap those creatures",
        describe_effect(damage_effect).trim_end_matches('.')
    ))
}

pub(super) fn describe_exile_source_and_attacking_nonflying_creature(
    effects: &[Effect],
) -> Option<String> {
    let [source_exile_effect, target_exile_effect] = effects else {
        return None;
    };
    let source_exile = source_exile_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if source_exile.zone != Zone::Exile || !matches!(source_exile.target, ChooseSpec::Source) {
        return None;
    }

    let target_exile = unwrap_basic_tag_wrappers(target_exile_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if target_exile.zone != Zone::Exile {
        return None;
    }

    let mut expected = ObjectFilter::creature()
        .attacking_player_or_planeswalker_controlled_by(PlayerFilter::You)
        .without_static_ability(crate::static_abilities::StaticAbilityId::Flying);
    expected.attacking = true;
    if !matches!(&target_exile.target, ChooseSpec::Target(inner) if matches!(inner.as_ref(), ChooseSpec::Object(filter) if filter == &expected))
    {
        return None;
    }

    Some("Exile this creature and target creature without flying that's attacking you".to_string())
}

pub(super) fn move_to_zone_is_plain_exile(move_to_zone: &crate::effects::MoveToZoneEffect) -> bool {
    move_to_zone.zone == Zone::Exile
        && move_to_zone.battlefield_controller == crate::effects::BattlefieldController::Preserve
        && !move_to_zone.enters_tapped
        && !move_to_zone.enters_attacking
        && !move_to_zone.enters_face_down
        && !move_to_zone.transfer_exiled_with_source_links
}

pub(super) fn describe_exile_source_and_target(effects: &[Effect]) -> Option<String> {
    let [source_exile_effect, target_exile_effect] = effects else {
        return None;
    };
    let source_exile = unwrap_basic_tag_wrappers(source_exile_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let target_exile = unwrap_basic_tag_wrappers(target_exile_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !move_to_zone_is_plain_exile(source_exile)
        || !move_to_zone_is_plain_exile(target_exile)
        || !matches!(source_exile.target.base(), ChooseSpec::Source)
        || !target_exile.target.is_target()
    {
        return None;
    }

    Some(format!(
        "Exile {} and {}",
        describe_choose_spec(&source_exile.target),
        describe_choose_spec(&target_exile.target)
    ))
}

pub(super) fn oath_of_ghouls_creature_graveyard_filter(
    filter: &ObjectFilter,
    owner: Option<&PlayerFilter>,
) -> bool {
    filter.zone == Some(Zone::Graveyard)
        && filter.card_types.as_slice() == [CardType::Creature]
        && filter.subtypes.is_empty()
        && filter.owner.as_ref() == owner
}

pub(super) fn describe_oath_of_ghouls_sequence(effects: &[Effect]) -> Option<String> {
    let [conditional_effect] = effects else {
        return None;
    };
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() {
        return None;
    }
    let crate::effect::Condition::AnOpponentHasFewerThanPlayer { player, filter } =
        &conditional.condition
    else {
        return None;
    };
    if player != &PlayerFilter::IteratedPlayer
        || !oath_of_ghouls_creature_graveyard_filter(filter, None)
    {
        return None;
    }
    let [may_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider.as_ref() != Some(&PlayerFilter::IteratedPlayer) {
        return None;
    }
    let [return_effect] = may.effects.as_slice() else {
        return None;
    };
    let return_from_gy =
        return_effect.downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()?;
    if return_from_gy.random || !exact_count(&return_from_gy.target.count(), 1) {
        return None;
    }
    let ChooseSpec::Object(return_filter) = return_from_gy.target.base() else {
        return None;
    };
    if !oath_of_ghouls_creature_graveyard_filter(return_filter, Some(&PlayerFilter::IteratedPlayer))
    {
        return None;
    }

    Some(
        "That player chooses target player whose graveyard has fewer creature cards in it than their graveyard does and is their opponent. The first player may return a creature card from their graveyard to their hand"
            .to_string(),
    )
}

pub(super) fn describe_gain_life_shuffle_source_and_graveyard(
    effects: &[Effect],
) -> Option<String> {
    let [
        gain_effect,
        move_effect,
        shuffle_effect,
        graveyard_shuffle_effect,
    ] = effects
    else {
        return None;
    };

    let gain = gain_effect.downcast_ref::<crate::effects::GainLifeEffect>()?;
    if gain.player != ChooseSpec::Player(PlayerFilter::You) {
        return None;
    }

    let move_tag = wrapped_effect_tag(move_effect)?;
    let move_to_zone = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !matches!(move_to_zone.target.base(), ChooseSpec::Source)
        || move_to_zone.zone != Zone::Library
        || move_to_zone.to_top
    {
        return None;
    }

    let shuffle_library = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if shuffle_library.target_spec.is_some()
        || !matches!(
            &shuffle_library.player,
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(tag)) if tag == move_tag
        )
    {
        return None;
    }

    let graveyard_shuffle = graveyard_shuffle_effect
        .downcast_ref::<crate::effects::ShuffleGraveyardIntoLibraryEffect>()?;
    if graveyard_shuffle.player != PlayerFilter::You {
        return None;
    }

    Some(format!(
        "You gain {} life. Shuffle this permanent and your graveyard into their owner's library",
        describe_value(&gain.amount)
    ))
}

pub(super) fn describe_untap_triggering_then_remove_from_combat(
    effects: &[Effect],
) -> Option<String> {
    let triggering_tag = TagKey::from("triggering");
    let (untap_effect, remove_effect) = match effects {
        [tag_triggering, untap_effect, remove_effect]
            if tag_triggering
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some() =>
        {
            (untap_effect, remove_effect)
        }
        [untap_effect, remove_effect] => (untap_effect, remove_effect),
        _ => return None,
    };

    let untap = if let Some((_, untap)) = tagged_untap_effect_view(untap_effect) {
        untap
    } else {
        untap_effect.downcast_ref::<crate::effects::UntapEffect>()?
    };
    if !choose_spec_references_exact_tag(&untap.target, &triggering_tag) {
        return None;
    }

    let remove = remove_effect.downcast_ref::<crate::effects::RemoveFromCombatEffect>()?;
    if !choose_spec_references_exact_tag(&remove.spec, &triggering_tag) {
        return None;
    }

    Some("untap it and remove it from combat".to_string())
}

pub(super) fn describe_remove_counter_then_no_counters_conditional(
    effects: &[Effect],
) -> Option<String> {
    let [remove_effect, conditional_effect] = effects else {
        return None;
    };
    let remove = unwrap_basic_tag_wrappers(remove_effect)
        .downcast_ref::<crate::effects::RemoveCountersEffect>()?;
    if !matches!(remove.target.base(), ChooseSpec::Source) {
        return None;
    }
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let Condition::SourceHasNoCounter(counter_type) = &conditional.condition else {
        return None;
    };
    if describe_no_more_counters_move_then_each_player_return(conditional).is_some() {
        return None;
    }
    if remove.counter_type != *counter_type
        || !conditional.if_false.is_empty()
        || conditional.if_true.is_empty()
    {
        return None;
    }

    let remove_text = describe_effect(remove_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let mut branch = describe_effect_clause_list(&conditional.if_true)
        .unwrap_or_else(|| describe_effect_list(&conditional.if_true));
    branch = lowercase_first(branch.trim().trim_end_matches('.'));
    branch = branch
        .replace("transform this creature", "transform it")
        .replace("transform this artifact", "transform it")
        .replace("sacrifice this creature", "sacrifice it")
        .replace("sacrifice this artifact", "sacrifice it");

    Some(format!(
        "{remove_text}. Then if it has no {} counters on it, {branch}",
        counter_type.description()
    ))
}

pub(super) fn object_filter_has_tagged_constraint(filter: &ObjectFilter, tag: &TagKey) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *tag
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            )
    })
}

pub(super) fn choose_spec_has_tagged_constraint(spec: &ChooseSpec, tag: &TagKey) -> bool {
    match spec {
        ChooseSpec::Tagged(candidate) => candidate == tag,
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            object_filter_has_tagged_constraint(filter, tag)
        }
        ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => choose_spec_has_tagged_constraint(inner, tag),
        ChooseSpec::SurfaceHinted { spec, .. } => choose_spec_has_tagged_constraint(spec, tag),
        _ => false,
    }
}

pub(super) fn aura_attachment_self_subject(filter: &ObjectFilter) -> &'static str {
    if filter.card_types.contains(&CardType::Land)
        || filter.subtypes.iter().any(|subtype| {
            matches!(
                subtype,
                Subtype::Plains
                    | Subtype::Island
                    | Subtype::Swamp
                    | Subtype::Mountain
                    | Subtype::Forest
                    | Subtype::Desert
                    | Subtype::Urzas
                    | Subtype::Cave
                    | Subtype::Gate
                    | Subtype::Locus
                    | Subtype::Town
            )
        })
    {
        "this land"
    } else if filter.card_types.contains(&CardType::Creature) {
        "this creature"
    } else if filter.card_types.contains(&CardType::Artifact) {
        "this artifact"
    } else if filter.card_types.contains(&CardType::Enchantment) {
        "this enchantment"
    } else {
        "this permanent"
    }
}

pub(super) fn move_trailing_tapped_token_surface(text: &str) -> String {
    for prefix in ["Create a ", "Create an "] {
        if let Some(rest) = text.strip_prefix(prefix)
            && let Some(rest) = rest.strip_suffix(", tapped")
        {
            return format!("{prefix}tapped {rest}");
        }
    }
    text.to_string()
}

pub(super) fn describe_return_as_aura_with_granted_abilities(effects: &[Effect]) -> Option<String> {
    let mut idx = 0usize;
    if effects
        .first()
        .and_then(|effect| effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>())
        .is_some()
    {
        idx += 1;
    }

    let choose = effects
        .get(idx)?
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.count.min != 1 || choose.count.max != Some(1) || choose.chooser != PlayerFilter::You {
        return None;
    }
    idx += 1;

    let return_effect = effects
        .get(idx)?
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()?;
    let as_aura = return_effect.as_aura.as_ref()?;
    if return_effect.tapped {
        return None;
    }
    if !object_filter_has_tagged_constraint(&as_aura.attachment_filter, &choose.tag) {
        return None;
    }
    idx += 1;

    let self_subject = aura_attachment_self_subject(&choose.filter);
    let mut granted_abilities = Vec::new();
    for effect in &effects[idx..] {
        let apply = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
        if apply.until != Until::Forever
            || apply.condition.is_some()
            || !apply.additional_modifications.is_empty()
            || !apply.runtime_modifications.is_empty()
            || !apply
                .target_spec
                .as_ref()
                .is_some_and(|spec| choose_spec_has_tagged_constraint(spec, &choose.tag))
        {
            return None;
        }
        let Some(crate::continuous::Modification::AddAbilityGeneric(ability)) = &apply.modification
        else {
            return None;
        };
        let ability_text = move_trailing_tapped_token_surface(
            &describe_inline_ability_with_self_subject(ability, self_subject),
        );
        granted_abilities.push(ability_text.trim_end_matches('.').to_string());
    }
    if granted_abilities.is_empty() {
        return None;
    }

    let enchant_target = strip_leading_article(&choose.filter.description()).to_string();
    let ability_subject = enchant_target
        .strip_suffix(" you control")
        .unwrap_or(enchant_target.as_str());
    let quoted = granted_abilities
        .iter()
        .enumerate()
        .map(|(idx, ability)| {
            if idx + 1 == granted_abilities.len() && !ability.ends_with('.') {
                format!("'{ability}.'")
            } else {
                format!("'{ability}'")
            }
        })
        .collect::<Vec<_>>();

    if as_aura.remove_all_abilities
        && let [activated] = granted_abilities.as_slice()
        && activated.contains(": ")
    {
        let activated = activated.replacen(
            &capitalize_first(self_subject),
            &capitalize_first(&format!("enchanted {ability_subject}")),
            1,
        );
        return Some(format!(
            "Return it to the battlefield. It's an Aura enchantment with enchant {enchant_target} and \"{},\" and it loses all other abilities",
            activated.trim_end_matches('.')
        ));
    }
    if as_aura.remove_all_abilities {
        return None;
    }

    Some(format!(
        "Return it to the battlefield. It's an Aura enchantment with enchant {enchant_target} and \"{} has {}\"",
        capitalize_first(&format!("enchanted {ability_subject}")),
        join_with_and(&quoted)
    ))
}

pub(super) fn describe_creature_planeswalker_source_counter_exile_item(
    filter: &ObjectFilter,
) -> Option<String> {
    let Some(crate::filter::Comparison::LessThanOrEqualExpr(value)) = filter.mana_value.as_ref()
    else {
        return None;
    };
    let Value::CountersOnSource(counter_type) = value.unhinted() else {
        return None;
    };
    if filter.card_types.len() != 2
        || !filter.card_types.contains(&CardType::Creature)
        || !filter.card_types.contains(&CardType::Planeswalker)
    {
        return None;
    }

    let mut remaining = filter.clone();
    let zone = remaining.zone.take();
    remaining.card_types.clear();
    remaining.mana_value = None;
    if remaining != ObjectFilter::default() {
        return None;
    }

    let mana_value_clause = format!(
        "with mana value less than or equal to the number of {} counters on it",
        counter_type.description()
    );
    match zone {
        None | Some(Zone::Battlefield) => Some(format!(
            "all creatures and planeswalkers {mana_value_clause}"
        )),
        Some(Zone::Graveyard) => Some(format!(
            "all creature and planeswalker cards in graveyards {mana_value_clause}"
        )),
        _ => None,
    }
}

pub(super) fn describe_mixed_exile_all_list_item(
    exile: &crate::effects::ExileEffect,
) -> Option<String> {
    if exile.face_down {
        return None;
    }
    let ChooseSpec::All(filter) = exile.spec.base() else {
        return None;
    };
    describe_creature_planeswalker_source_counter_exile_item(filter)
        .or_else(|| Some(describe_choose_spec(&exile.spec)))
}

pub(super) fn join_mixed_exile_list_items(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        2 => format!("{} and {}", items[0], items[1]),
        _ => {
            let mut out = items[..items.len() - 1].join(", ");
            out.push_str(", and ");
            out.push_str(&items[items.len() - 1]);
            out
        }
    }
}

pub(super) fn describe_mixed_move_to_exile_then_exile_all_list(
    effects: &[Effect],
) -> Option<String> {
    if effects.len() < 2 {
        return None;
    }
    let first = effects[0].downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if first.zone != Zone::Exile || matches!(first.target.base(), ChooseSpec::All(_)) {
        return None;
    }

    let mut items = Vec::with_capacity(effects.len());
    items.push(describe_choose_spec(&first.target));
    for effect in &effects[1..] {
        let exile = effect.downcast_ref::<crate::effects::ExileEffect>()?;
        items.push(describe_mixed_exile_all_list_item(exile)?);
    }

    Some(format!("Exile {}", join_mixed_exile_list_items(&items)))
}

pub(super) fn filter_controls_only_tagged_object(
    filter: &ObjectFilter,
    player: &PlayerFilter,
    tag: &TagKey,
) -> bool {
    let mut stripped = filter.clone();
    if stripped
        .controller
        .as_ref()
        .is_some_and(|controller| controller == player)
    {
        stripped.controller = None;
    }
    let Some(tagged_idx) = stripped.tagged_constraints.iter().position(|constraint| {
        constraint.tag == *tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }) else {
        return false;
    };
    if stripped.tagged_constraints.len() != 1 {
        return false;
    }
    stripped.tagged_constraints.remove(tagged_idx);
    stripped == ObjectFilter::default() || stripped == ObjectFilter::creature()
}

pub(super) fn condition_controls_tagged_object(
    condition: &Condition,
    player: &PlayerFilter,
    tag: &TagKey,
) -> bool {
    let Condition::PlayerControls {
        player: condition_player,
        filter,
    } = condition
    else {
        return false;
    };
    condition_player == player && filter_controls_only_tagged_object(filter, player, tag)
}

pub(super) fn condition_does_not_control_tagged_object(
    condition: &Condition,
    player: &PlayerFilter,
    tag: &TagKey,
) -> bool {
    let Condition::Not(inner) = condition else {
        return false;
    };
    condition_controls_tagged_object(inner, player, tag)
}

pub(super) fn describe_triggering_control_draw_else_lose(effects: &[Effect]) -> Option<String> {
    let [tag_effect, draw_conditional_effect, lose_conditional_effect] = effects else {
        return None;
    };
    let tag_triggering = tag_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let tag = &tag_triggering.tag;
    if tag.as_str() != "triggering" {
        return None;
    }

    let draw_conditional =
        draw_conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !draw_conditional.if_false.is_empty()
        || !condition_controls_tagged_object(&draw_conditional.condition, &PlayerFilter::You, tag)
    {
        return None;
    }
    let [draw_effect] = draw_conditional.if_true.as_slice() else {
        return None;
    };
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You || draw.count != Value::Fixed(1) {
        return None;
    }

    let lose_conditional =
        lose_conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !lose_conditional.if_false.is_empty()
        || !condition_does_not_control_tagged_object(
            &lose_conditional.condition,
            &PlayerFilter::You,
            tag,
        )
    {
        return None;
    }
    let [lose_effect] = lose_conditional.if_true.as_slice() else {
        return None;
    };
    let lose = lose_effect.downcast_ref::<crate::effects::LoseLifeEffect>()?;
    if lose.amount != Value::Fixed(1) {
        return None;
    }
    match &lose.player {
        ChooseSpec::Player(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(
            lose_tag,
        ))) if lose_tag == tag => Some(
            "Draw a card if you control that creature. If you don't control it, its controller loses 1 life"
                .to_string(),
        ),
        _ => None,
    }
}

/// Collection programs whose typed producer, selections, and movement already
/// have an oracle-shaped compactor. These must win before the generic clause
/// renderer considers each effect independently: the marker-safe
/// `ForEachTaggedEffect` fallback is intentionally renderable, but it loses the
/// "from among them" relationship when separated from its producer.
fn describe_typed_collection_selection_program(effects: &[Effect]) -> Option<String> {
    let refs = effects.iter().collect::<Vec<_>>();
    if let Some(compact) = render_exile_top_then_put_from_among_onto_battlefield(&refs) {
        return Some(compact);
    }

    match effects {
        [milled_effect, choose_effect, move_effect] => {
            let (source_tag, mill) = mill_with_collection_tag(milled_effect)?;
            let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            let (_, move_chosen) = for_each_tagged_for_compaction(move_effect)?;
            describe_mill_then_put_milled_cards(source_tag.as_str(), mill, &[choose], move_chosen)
        }
        [
            milled_effect,
            first_choice_effect,
            second_choice_effect,
            move_effect,
        ] => {
            let (source_tag, mill) = mill_with_collection_tag(milled_effect)?;
            let first_choice =
                first_choice_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            let second_choice =
                second_choice_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            let (_, move_chosen) = for_each_tagged_for_compaction(move_effect)?;
            describe_mill_then_put_milled_cards(
                source_tag.as_str(),
                mill,
                &[first_choice, second_choice],
                move_chosen,
            )
        }
        _ => None,
    }
}

fn describe_typed_collection_selection_prefix(effects: &[Effect]) -> Option<(String, usize)> {
    // Collection procedures are often followed by an independent rider (for
    // example, "You gain 2 life").  The typed producer/selection/move prefix
    // must still render as one procedure instead of leaking Choose/ForEach
    // implementation details merely because a later effect shares the same
    // resolution segment.
    for consumed in [4usize, 3usize] {
        if let Some(prefix) = effects.get(..consumed)
            && let Some(compact) = describe_typed_collection_selection_program(prefix)
        {
            return Some((compact, consumed));
        }
    }
    None
}

fn describe_coordinated_returns_then_discard_and_source_exile(
    effects: &[Effect],
) -> Option<String> {
    let [return_sequence_effect, discard_effect, source_exile_effect] = effects else {
        return None;
    };
    let return_sequence = structural_unwrap_render_wrappers(return_sequence_effect)
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    if matches!(
        return_sequence.surface,
        ironsmith_core::SequenceSurface::Sequential
    ) {
        return None;
    }
    let return_effects = return_sequence
        .effects
        .iter()
        .filter(|effect| {
            structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_none()
        })
        .collect::<Vec<_>>();
    let (return_text, consumed) = describe_leading_coordinated_graveyard_returns(&return_effects)?;
    if consumed != return_effects.len() {
        return None;
    }

    let discard = structural_unwrap_render_wrappers(discard_effect)
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard.player != PlayerFilter::You
        || discard.count != Value::Fixed(1)
        || discard.random
        || discard.any_number
        || discard.card_filter.is_some()
    {
        return None;
    }

    let source_exile = structural_unwrap_render_wrappers(source_exile_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !move_to_zone_is_plain_exile(source_exile)
        || !matches!(source_exile.target.base(), ChooseSpec::Source)
    {
        return None;
    }
    let rendered_exile = describe_effect(source_exile_effect);
    let rendered_exile = rendered_exile.trim().trim_end_matches('.');
    let rendered_exile = rendered_exile
        .strip_prefix("You ")
        .or_else(|| rendered_exile.strip_prefix("you "))
        .unwrap_or(rendered_exile);
    if !rendered_exile.to_ascii_lowercase().starts_with("exile ") {
        return None;
    }

    Some(format!(
        "{return_text}, then discard a card. {}",
        capitalize_first(rendered_exile)
    ))
}

/// Structural renderings that must run before `describe_effect_clause_list`.
///
/// Resolution-program rendering normally prefers the compact clause renderer,
/// so putting these patterns only in `describe_effect_list` makes them
/// unreachable for ordinary spell and triggered-ability payloads.
fn describe_embedded_suspend_setup_sequence(effects: &[Effect]) -> Option<String> {
    for start in 0..effects.len() {
        let matched = effects
            .get(start..start + 3)
            .and_then(describe_exile_with_counters_then_gain_suspend)
            .map(|text| (text, 3))
            .or_else(|| {
                effects
                    .get(start..start + 2)
                    .and_then(describe_put_counters_then_gain_suspend)
                    .map(|text| (text, 2))
            });
        let Some((compact, consumed)) = matched else {
            continue;
        };

        let mut parts = Vec::new();
        if start > 0 {
            parts.push(
                describe_effect_clause_list(&effects[..start])
                    .unwrap_or_else(|| describe_effect_list(&effects[..start])),
            );
        }
        parts.push(compact);
        if start + consumed < effects.len() {
            parts.push(
                describe_effect_clause_list(&effects[start + consumed..])
                    .unwrap_or_else(|| describe_effect_list(&effects[start + consumed..])),
            );
        }

        return Some(
            parts
                .into_iter()
                .enumerate()
                .filter_map(|(index, part)| {
                    let part = part.trim().trim_end_matches('.');
                    let part = normalize_imperative_you_clause(part);
                    if part.is_empty() {
                        None
                    } else if index == 0 {
                        Some(part)
                    } else {
                        Some(capitalize_first(&part))
                    }
                })
                .collect::<Vec<_>>()
                .join(". "),
        );
    }
    None
}

fn normalize_attached_control_chain(
    tag_attached: &crate::effects::TagAttachedToSourceEffect,
    control_effect: &Effect,
    rendered: &str,
) -> String {
    let rendered_target = match tag_attached.tag.as_str() {
        "enchanted" => "enchanted creature",
        "equipped" => "equipped creature",
        _ => return rendered.to_string(),
    };
    let target_spec = tagged_apply_continuous_view(control_effect)
        .and_then(|(_, control)| control.target_spec.as_ref());
    let attached_target = describe_attached_object_for_tag(tag_attached.tag.as_str(), target_spec);
    let rendered_followup = gain_control_followup_untap_target_text(rendered_target);
    let attached_followup = gain_control_followup_untap_target_text(&attached_target);

    let mut normalized = rendered.replace(rendered_target, &attached_target);
    normalized = normalized.replace(
        &capitalize_first(rendered_target),
        &capitalize_first(&attached_target),
    );
    if rendered_followup != attached_followup {
        normalized = normalized.replace(rendered_followup, attached_followup);
    }
    normalized.replace(
        &format!(". {} ", capitalize_first(&attached_target)),
        ". It ",
    )
}

fn describe_sacrifice_consult_with_terminal_sequence(effects: &[Effect]) -> Option<String> {
    let [choose, sacrifice, consult, terminal] = effects else {
        return None;
    };
    let sequence = terminal.downcast_ref::<crate::effects::SequenceEffect>()?;
    let [move_matches, put_remainder] = sequence.effects.as_slice() else {
        return None;
    };
    render_sacrifice_then_consult_reveal_put_battlefield_rest_bottom(&[
        choose,
        sacrifice,
        consult,
        move_matches,
        put_remainder,
    ])
}

pub(in crate::compiled_text) fn describe_nested_search_for_each_conditional_shuffle(
    effects: &[Effect],
) -> Option<String> {
    if let [effect] = effects
        && let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>()
    {
        let nested = sequence.effects.iter().collect::<Vec<_>>();
        if let Some((compact, consumed)) =
            describe_wrapped_search_for_each_then_conditional_shuffle(&nested)
            && consumed == nested.len()
        {
            return Some(compact);
        }
    }
    None
}

pub(crate) fn describe_pre_clause_structural_effect_list(effects: &[Effect]) -> Option<String> {
    if let Some(compact) = describe_exile_all_from_same_target_players_hand_and_graveyard(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_delegated_subset_with_hand_remainder(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_delegated_collection_partition_moves(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_delegated_subset_choice(effects) {
        return Some(compact);
    }
    if let [effect] = effects
        && let Some(compact) = describe_delegated_collection_complement_move(effect)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_cast_from_hand_consult_source_exiled_cleanup(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_source_exiled_return_partition(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_tempting_offer_copy_spell_bundle(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_choose_creature_type_then_untap_all(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_activated_counter_removal_damage(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_combat_requirement_then_prohibition(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_graveyard_exile_copy_cast(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_exiled_collection_cast_choice(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_exiled_collection_partition(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_nested_search_for_each_conditional_shuffle(effects) {
        return Some(compact);
    }
    if let [effect] = effects
        && let Some(compact) = describe_each_player_exile_sacrifice_return_result(effect)
    {
        return Some(compact);
    }
    let exact_refs = effects.iter().collect::<Vec<_>>();
    if let Some(compact) = describe_shape_anew_like_bundle(&exact_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_sacrifice_consult_with_terminal_sequence(effects) {
        return Some(compact);
    }
    // A discard and its exact outcome-count draw are one authored sequence.
    // Preserve the typed shared actor, "up to" count, and "that many" surface
    // before the broad prior-action consumer expands them into two sentences.
    if let Some(compact) = describe_discard_then_draw_amount_sequence(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_id_backed_prior_action_count_consumer(effects) {
        return Some(compact);
    }
    if let Some((prefix, consumed)) = describe_explicit_target_then_coin_flip(effects) {
        if consumed == effects.len() {
            return Some(prefix);
        }
        let suffix = describe_effect_list(&effects[consumed..]);
        return Some(format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }
    let raw_effects = effects.iter().collect::<Vec<_>>();
    // This looked-card partition carries an authored result-set surface on
    // the exact looked-minus-selected complement. Recognize the complete
    // producer/selection/disposition chain before broader clause compactors
    // reduce the final tagged branch to the generic "the rest" wording.
    if let Some(compact) = describe_look_at_top_choose_battlefield_rest_graveyard(effects) {
        return Some(compact);
    }
    // Target declaration, optional cast, and the exact cast-result replacement
    // are one authored procedure. The pre-clause renderer must see the complete
    // typed tag chain before a broader target/cast prefix consumes it.
    if let Some(compact) =
        describe_may_cast_target_graveyard_spell_then_exile_replacement(&raw_effects)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_choose_x_permanents_create_x_copies(&raw_effects) {
        return Some(compact);
    }
    if let [choose_effect, for_each_effect] = effects
        && let Some(choose) = unwrap_basic_tag_wrappers(choose_effect)
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()
    {
        if let Some(for_each) = unwrap_basic_tag_wrappers(for_each_effect)
            .downcast_ref::<crate::effects::ForEachTaggedEffect>()
        {
            if let Some(compact) =
                describe_choose_any_number_then_remove_counter_from_each(choose, for_each)
            {
                return Some(compact);
            }
            if let Some(compact) = describe_choose_then_for_each_copy(choose, for_each) {
                return Some(compact);
            }
        }
        if let Some(for_each) = for_each_effect.downcast_ref::<crate::effects::ForEachObject>()
            && let Some(compact) = describe_choose_then_for_each_object_copy(choose, for_each)
        {
            return Some(compact);
        }
    }
    // This seven-effect procedure proves both optional selections, their
    // distinct destinations, and the shared remainder tag. Run it before the
    // broad clause renderer can consume the same list as singular actions.
    if let Some(compact) = describe_reveal_top_two_optional_picks_rest_bottom(&raw_effects) {
        return Some(compact);
    }

    // A consult, its tagged match move, and the exact tagged remainder form
    // one typed library partition. Preserve that complete producer/consumer
    // surface before broader structural renderers can claim the individual
    // effects and expose implementation-oriented pronouns or ownership.
    if let Some(compact) = render_consult_reveal_put_hand_then_bottom(&raw_effects) {
        return Some(compact);
    }

    // These fight sequences carry two explicit target declarations whose
    // tags are consumed by both the conditional action and the final fight.
    // The ordinary clause renderer splits those declarations before the
    // relationship-aware effect-list renderer can see them, so recognize the
    // exact four-effect shape at this pre-clause dispatch point.
    if let Some(compact) = describe_two_distinct_targets_conditional_then_fight(&raw_effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_two_distinct_targets_counter_then_fight(&raw_effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_targeted_conditional_action_then_fight(&raw_effects) {
        return Some(compact);
    }

    if let [producer, consumer] = effects
        && let Some(compact) =
            describe_each_player_choose_creature_then_destroy_others_pair(producer, consumer)
    {
        return Some(compact);
    }

    if let Some(compact) = describe_split_for_players_choose_then_sacrifice(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_embedded_suspend_setup_sequence(effects) {
        return Some(compact);
    }

    if let Some((compact, consumed)) = describe_private_look_choose_one_graveyard(&raw_effects) {
        if consumed == effects.len() {
            return Some(compact);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    if let Some((compact, consumed)) = describe_revealed_top_choose_one_graveyard(&raw_effects) {
        if consumed == effects.len() {
            return Some(compact);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    if let Some((compact, consumed)) =
        describe_looked_card_selected_hand_remainder_bottom(&raw_effects)
    {
        if consumed == effects.len() {
            return Some(compact);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    if let Some((compact, consumed)) = describe_conditional_looked_hand_partition(&raw_effects) {
        if consumed == effects.len() {
            return Some(compact);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    if let Some((compact, consumed)) =
        describe_looked_battlefield_then_conditional_remainder(&raw_effects)
    {
        if consumed == effects.len() {
            return Some(compact);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    if let Some((compact, consumed)) =
        describe_look_exile_face_down_rest_graveyard_then_cast(&raw_effects)
    {
        if consumed == effects.len() {
            return Some(compact);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    if let Some((compact, consumed)) = describe_three_way_looked_card_partition(&raw_effects) {
        if consumed == effects.len() {
            return Some(compact);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    if let Some((compact, consumed)) = describe_self_look_reorder_then_may_shuffle(&raw_effects) {
        if consumed == effects.len() {
            return Some(compact);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        let suffix = normalize_imperative_you_clause(suffix.trim_end_matches('.'));
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(&suffix)
        ));
    }

    if let Some(compact) = describe_chain_copy_effect_list(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_gain_control_counter_untap_haste_structural(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_must_block_untap_then_others_cant_block_structural(effects) {
        return Some(compact);
    }

    if effects.len() >= 3
        && let Some(tag_attached) =
            effects[0].downcast_ref::<crate::effects::TagAttachedToSourceEffect>()
        && let Some(prefix) = describe_gain_control_then_untap_structural(&effects[1..3])
    {
        let rendered = if effects.len() == 3 {
            prefix
        } else {
            let suffix = describe_effect_clause_list(&effects[3..])
                .unwrap_or_else(|| describe_effect_list(&effects[3..]));
            format!(
                "{}. {}",
                prefix.trim_end_matches('.'),
                capitalize_first(suffix.trim_end_matches('.'))
            )
        };
        return Some(normalize_attached_control_chain(
            tag_attached,
            &effects[1],
            &rendered,
        ));
    }

    if effects.len() >= 2
        && let Some(prefix) = describe_put_counters_then_untap_same_target_structural(&effects[..2])
    {
        if effects.len() == 2 {
            return Some(prefix);
        }
        let suffix = describe_effect_clause_list(&effects[2..])
            .unwrap_or_else(|| describe_effect_list(&effects[2..]));
        return Some(format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    if let Some(compact) = describe_tagged_multi_copy_then_may_retarget(effects) {
        return Some(compact);
    }

    // A targeted damage effect does not make a following explicit "you"
    // discard/draw sequence refer to the damaged player. Keep the sentence
    // boundary and let the typed discard/draw pair retain its own actor.
    if let [damage_effect, tail @ ..] = effects
        && unwrap_basic_tag_wrappers(damage_effect)
            .downcast_ref::<crate::effects::DealDamageEffect>()
            .is_some()
        && let Some(discard_draw) = describe_discard_then_draw_amount_sequence(tail)
    {
        let damage = describe_effect(damage_effect)
            .trim_end_matches('.')
            .to_string();
        return Some(format!(
            "{damage}. {}",
            capitalize_first(discard_draw.trim_end_matches('.'))
        ));
    }

    if let Some(compact) = describe_tagged_forced_block_effect_list(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_coordinated_returns_then_discard_and_source_exile(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_typed_collection_selection_program(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_gain_control_aura_then_legal_attach(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_linked_source_attachment_prefix(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_battlefield_graveyard_return_pair(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_battlefield_graveyard_exile_pair(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_tap_freeze_bundle(&raw_effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_top_choice_to_hand_rest_graveyard_structural(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_become_aura_manifest_then_attach(&raw_effects) {
        return Some(compact);
    }
    if let [producer, for_each] = effects
        && let Some(compact) = describe_amass_then_amassed_army_power_damage(producer, for_each)
    {
        return Some(compact);
    }
    if let [sequence] = effects
        && let Some(sequence) = structural_unwrap_render_wrappers(sequence)
            .downcast_ref::<crate::effects::SequenceEffect>()
        && matches!(
            sequence.surface,
            ironsmith_core::SequenceSurface::CommaThen
                | ironsmith_core::SequenceSurface::Coordinated
        )
        && let [producer, for_each] = sequence.effects.as_slice()
        && let Some(compact) = describe_amass_then_amassed_army_power_damage(producer, for_each)
    {
        return Some(compact);
    }
    if let [producer, consumer] = effects
        && let Some(compact) = describe_animation_then_counters_on_result(producer, consumer)
    {
        return Some(compact);
    }
    if let [producer, consumer] = effects
        && let Some(compact) = describe_counters_then_goad_countered_result(producer, consumer)
    {
        return Some(compact);
    }
    if let [producer, for_each] = effects
        && let Some(compact) = describe_result_producer_then_for_each_tagged(producer, for_each)
    {
        return Some(compact);
    }
    if let [
        look_effect,
        reveal_effect,
        choose_effect,
        move_effect,
        rest_effect,
    ] = effects
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(reveal_tagged) =
            reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some((_, move_chosen)) = for_each_tagged_for_compaction(move_effect)
        && let Some((_, rest)) = for_each_tagged_for_compaction(rest_effect)
        && let Some(compact) = describe_look_at_top_then_put_matching_to_zone_rest_hand(
            look_at_top,
            Some(reveal_tagged),
            choose,
            move_chosen,
            rest,
        )
    {
        return Some(compact);
    }
    if let [create_effect, draw_effect] = effects
        && created_token_effect(create_effect).is_some()
        && let Some(draw) = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()
        && matches!(draw.count.unhinted(), Value::DistinctNames(_))
    {
        let create_text = describe_effect(create_effect)
            .trim_end_matches('.')
            .to_string();
        let draw_text = lowercase_first(describe_effect(draw_effect).trim_end_matches('.'));
        if !create_text.is_empty() && !draw_text.is_empty() {
            return Some(format!("{create_text}, then {draw_text}"));
        }
    }
    if let [damage_effect, exile_effect, play_effect] = effects
        && let Some(with_id) = damage_effect.downcast_ref::<crate::effects::WithIdEffect>()
        && unwrap_tag_wrappers(&with_id.effect)
            .downcast_ref::<crate::effects::DealDamageEffect>()
            .is_some()
        && let Some(exile) = exile_effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
        && matches!(
            exile.count.unhinted(),
            Value::EffectMetric {
                effect_id,
                metric: crate::effect::EffectMetric::ExcessDamage,
                ..
            } if *effect_id == with_id.id
        )
        && let Some(play) = play_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
        && exile.moved_tags.iter().any(|tag| tag == &play.tag)
    {
        let damage_text = describe_effect(damage_effect)
            .trim_end_matches('.')
            .to_string();
        let exile_text = capitalize_first(describe_effect(exile_effect).trim_end_matches('.'));
        let play_text = capitalize_first(describe_effect(play_effect).trim_end_matches('.'));
        if !damage_text.is_empty() && !exile_text.is_empty() && !play_text.is_empty() {
            return Some(format!("{damage_text}. {exile_text}. {play_text}"));
        }
    }

    None
}

fn describe_leading_coordinated_graveyard_returns(effects: &[&Effect]) -> Option<(String, usize)> {
    let mut targets = Vec::new();
    let mut shared_route: Option<(String, String)> = None;
    for effect in effects {
        let Some((target, from, to)) = coordinated_graveyard_to_hand_view(effect) else {
            break;
        };
        if let Some((expected_from, expected_to)) = &shared_route {
            if expected_from != &from || expected_to != &to {
                break;
            }
        } else {
            shared_route = Some((from, to));
        }
        targets.push(target);
    }
    if targets.len() < 2 {
        return None;
    }
    let consumed = targets.len();
    let (from, to) = shared_route?;
    Some((
        format!(
            "Return {} from {from} to {to}",
            join_coordinated_parts(&targets)?
        ),
        consumed,
    ))
}

fn describe_draw_then_additional_draw(effects: &[Effect]) -> Option<String> {
    let [first_effect, additional_effect] = effects else {
        return None;
    };
    let first = structural_unwrap_render_wrappers(first_effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let additional = structural_unwrap_render_wrappers(additional_effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if first.player != additional.player
        || first
            .count
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalCards)
        || !additional
            .count
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalCards)
    {
        return None;
    }

    let first_text = capitalize_first(&normalize_imperative_you_clause(
        describe_effect(first_effect).trim().trim_end_matches('.'),
    ));
    let additional_text = lowercase_first(&normalize_imperative_you_clause(
        describe_effect(additional_effect)
            .trim()
            .trim_end_matches('.'),
    ));
    (!first_text.is_empty() && !additional_text.is_empty())
        .then(|| format!("{first_text}, then {additional_text}"))
}

#[cfg(test)]
mod coordinated_return_runtime_tests {
    use super::*;

    #[test]
    fn flat_tagged_runtime_returns_compact_before_the_generic_sentence_loop() {
        let returned = |subtype, tag: &str| {
            let target = ChooseSpec::target(ChooseSpec::Object(
                ObjectFilter::default()
                    .in_zone(Zone::Graveyard)
                    .owned_by(PlayerFilter::You)
                    .with_subtype(subtype),
            ))
            .with_count(ChoiceCount::up_to(1));
            Effect::new(crate::effects::ReturnFromGraveyardToHandEffect::new(
                target, false,
            ))
            .tag(tag)
        };
        let effects = vec![
            returned(Subtype::Pirate, "returned_0"),
            returned(Subtype::Vampire, "returned_1"),
            returned(Subtype::Dinosaur, "returned_2"),
            Effect::exile(ChooseSpec::Source),
        ];

        let rendered = describe_effect_list(&effects);
        assert_eq!(
            rendered
                .matches(" from your graveyard to your hand")
                .count(),
            1
        );
        assert!(!rendered.contains(". Return"), "{rendered}");
        assert!(rendered.ends_with(". Exile this source"), "{rendered}");
    }
}

fn describe_each_player_reveal_set_may_move_else_draw(effects: &[Effect]) -> Option<String> {
    let [reveal_effect, may_effect, fallback_effect] = effects else {
        return None;
    };
    let reveal_for_players = reveal_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let (subject, fallback_subject) = match reveal_for_players.filter {
        PlayerFilter::Any => ("Each player", "each player"),
        PlayerFilter::Opponent => ("Each opponent", "each opponent"),
        _ => return None,
    };
    let [reveal_top_effect] = reveal_for_players.effects.as_slice() else {
        return None;
    };
    let reveal_top = reveal_top_effect.downcast_ref::<crate::effects::RevealTopEffect>()?;
    let revealed_tag = reveal_top.tag.as_ref()?;
    if reveal_top.player != PlayerFilter::IteratedPlayer {
        return None;
    }

    let with_id = may_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider.as_ref() != Some(&PlayerFilter::You) {
        return None;
    }
    let [move_effect] = may.effects.as_slice() else {
        return None;
    };
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.to_top
        || !matches!(move_to_zone.target.base(), ChooseSpec::Tagged(tag) if tag == revealed_tag)
    {
        return None;
    }
    let owners_zone = match move_to_zone.zone {
        Zone::Graveyard => "graveyards",
        Zone::Hand => "hands",
        Zone::Library => "libraries",
        _ => return None,
    };

    let fallback = fallback_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if fallback.condition != with_id.id
        || fallback.predicate != EffectPredicate::DidNotHappen
        || !fallback.else_.is_empty()
    {
        return None;
    }
    let [draw_for_players_effect] = fallback.then.as_slice() else {
        return None;
    };
    let draw_for_players =
        draw_for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if draw_for_players.filter != reveal_for_players.filter {
        return None;
    }
    let [draw_effect] = draw_for_players.effects.as_slice() else {
        return None;
    };
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::IteratedPlayer {
        return None;
    }

    Some(format!(
        "{subject} reveals the top card of their library. You may put the revealed cards into their owners' {owners_zone}. If you don't, {fallback_subject} draws {}",
        describe_card_count(&draw.count)
    ))
}

fn describe_consult_characteristic_boost_then_all_revealed_bottom(
    effects: &[Effect],
) -> Option<String> {
    let [consult_effect, boost_effect, remainder_effect] = effects else {
        return None;
    };
    let consult = structural_unwrap_render_wrappers(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    let boost = structural_unwrap_render_wrappers(boost_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let remainder = structural_unwrap_render_wrappers(remainder_effect)
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || remainder.tag != consult.all_tag
        || remainder.keep_tagged.is_some()
        || remainder.player != consult.player
        || boost.modification.is_some()
        || !boost.additional_modifications.is_empty()
    {
        return None;
    }
    let [
        crate::effects::continuous::RuntimeModification::ModifyPowerToughness { power, toughness },
    ] = boost.runtime_modifications.as_slice()
    else {
        return None;
    };
    if !matches!(toughness.unhinted(), Value::Fixed(0))
        || !matches!(
            power.unhinted(),
            Value::ManaValueOf(spec)
                if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag == &consult.match_tag)
        )
    {
        return None;
    }

    let consult_text = describe_effect(consult_effect);
    let consult_text = if consult.player == PlayerFilter::You {
        capitalize_first(consult_text.strip_prefix("you ").unwrap_or(&consult_text))
    } else {
        capitalize_first(&consult_text)
    };
    let boost_text = capitalize_first(&describe_effect(boost_effect)).replace(
        "where X is its mana value",
        "where X is that card's mana value",
    );
    let remainder_text = capitalize_first(&describe_effect(remainder_effect));
    Some(format!(
        "{}. {}. {}",
        consult_text.trim_end_matches('.'),
        boost_text.trim_end_matches('.'),
        remainder_text.trim_end_matches('.')
    ))
}

/// Render a reveal-until combat sequence while keeping its two distinct
/// antecedents explicit: "the creature" is the triggering attacker, whereas
/// "the revealed cards" is the consult's complete exposed collection.
fn describe_consult_reveal_triggering_creature_pump_then_move_revealed(
    effects: &[Effect],
) -> Option<String> {
    let visible = effects
        .iter()
        .filter(|effect| {
            effect
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_none()
        })
        .collect::<Vec<_>>();
    let [consult_effect, pump_effect, move_effect] = visible.as_slice() else {
        return None;
    };
    let with_id = consult_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let consult = structural_unwrap_render_wrappers(&with_id.effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    let pump = structural_unwrap_render_wrappers(pump_effect)
        .downcast_ref::<crate::effects::ModifyPowerToughnessForEachEffect>()?;
    let moved = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;

    if consult.player != PlayerFilter::Defending
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || !consult.filter.card_types.contains(&CardType::Land)
        || !matches!(
            consult.stop_rule,
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
                | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1))
        )
        || !matches!(pump.target.base(), ChooseSpec::Tagged(tag) if tag.as_str() == "triggering")
        || !pump
            .count
            .has_surface_hint(ValueSurfaceHint::CardsRevealedThisWay)
        || !matches!(pump.duration, Until::EndOfTurn)
        || moved.zone != Zone::Graveyard
        || !moved.target_plural_surface
        || moved.actor_surface != Some(consult.player.clone())
        || moved.destination_player_surface != Some(consult.player.clone())
        || !matches!(moved.target.base(), ChooseSpec::Tagged(tag) if tag == &consult.all_tag)
    {
        return None;
    }

    let consult_text = capitalize_first(&describe_effect(consult_effect));
    let pump_text = capitalize_first(&describe_effect(pump_effect));
    let pump_text = pump_text
        .strip_prefix("It ")
        .map(|rest| format!("The creature {rest}"))
        .unwrap_or(pump_text);
    Some(format!(
        "{}. {}. That player puts the revealed cards into their graveyard",
        consult_text.trim_end_matches('.'),
        pump_text.trim_end_matches('.'),
    ))
}

/// Keep the four independently authored steps of a shared top-card reveal
/// sequence as sentences.  The executable program deliberately shares one
/// reveal outcome between a filtered token count and a complementary pump;
/// flattening it into a comma chain obscures those boundaries and antecedents.
fn describe_each_player_reveal_filtered_token_then_pump_then_draw(
    effects: &[Effect],
) -> Option<String> {
    let [reveal_effect, repeat_effect, pump_effect, draw_effect] = effects else {
        return None;
    };
    let reveal_with_id = reveal_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let reveal_players = structural_unwrap_render_wrappers(&reveal_with_id.effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let [reveal_top_effect] = reveal_players.effects.as_slice() else {
        return None;
    };
    let reveal_top = structural_unwrap_render_wrappers(reveal_top_effect)
        .downcast_ref::<crate::effects::RevealTopEffect>()?;
    let repeat = structural_unwrap_render_wrappers(repeat_effect)
        .downcast_ref::<crate::effects::RepeatEffectsEffect>()?;
    let [create_effect] = repeat.effects.as_slice() else {
        return None;
    };
    let create = structural_unwrap_render_wrappers(create_effect)
        .downcast_ref::<crate::effects::CreateTokenEffect>()?;
    let pump = structural_unwrap_render_wrappers(pump_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let draw_players = structural_unwrap_render_wrappers(draw_effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let [draw_effect] = draw_players.effects.as_slice() else {
        return None;
    };
    let draw = structural_unwrap_render_wrappers(draw_effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;

    if reveal_players.filter != PlayerFilter::Any
        || reveal_top.player != PlayerFilter::IteratedPlayer
        || !repeat
            .count
            .has_surface_hint(ValueSurfaceHint::CardsRevealedThisWay)
        || create.controller != PlayerFilter::You
        || !matches!(create.count.unhinted(), Value::Fixed(1))
        || pump.runtime_modifications.len() != 1
        || draw_players.filter != PlayerFilter::Any
        || draw.player != PlayerFilter::IteratedPlayer
        || !matches!(draw.count.unhinted(), Value::Fixed(1))
    {
        return None;
    }

    let reveal_text = capitalize_first(&describe_effect(reveal_effect));
    let repeat_text =
        capitalize_first(&describe_effect(repeat_effect)).replacen(", create ", ", you create ", 1);
    let pump_text = capitalize_first(&describe_effect(pump_effect));
    let draw_text = capitalize_first(&describe_effect(draw_effect));
    Some(format!(
        "{}. {}. Then {}. Then {}",
        reveal_text.trim_end_matches('.'),
        repeat_text.trim_end_matches('.'),
        pump_text.trim_end_matches('.'),
        draw_text.trim_end_matches('.'),
    ))
}

fn describe_consult_reflexive_damage_then_all_revealed_bottom(
    effects: &[Effect],
) -> Option<String> {
    let [consult_effect, reflexive_effect, remainder_effect] = effects else {
        return None;
    };
    let with_id = consult_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let consult = structural_unwrap_render_wrappers(&with_id.effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    let reflexive = structural_unwrap_render_wrappers(reflexive_effect)
        .downcast_ref::<crate::effects::ReflexiveTriggerEffect>()?;
    let remainder = structural_unwrap_render_wrappers(remainder_effect)
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    let [damage_effect] = reflexive.effects.as_slice() else {
        return None;
    };
    let damage = structural_unwrap_render_wrappers(damage_effect)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;

    if consult.player != PlayerFilter::You
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || reflexive.condition != with_id.id
        || reflexive.predicate != EffectPredicate::Happened
        || remainder.tag != consult.all_tag
        || remainder.keep_tagged.is_some()
        || remainder.player != consult.player
        || !matches!(damage.target.base(), ChooseSpec::AnyTarget)
        || !matches!(
            damage.amount.unhinted(),
            Value::ManaValueOf(spec)
                if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag == &consult.match_tag)
        )
        || !matches!(
            consult.stop_rule,
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
                | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1))
        )
    {
        return None;
    }

    let consult_text = describe_effect(consult_effect);
    let consult_text = capitalize_first(consult_text.strip_prefix("you ").unwrap_or(&consult_text));
    let remainder_text = capitalize_first(&describe_effect(remainder_effect));
    let matched_reference = describe_search_selection_with_cards(&consult.filter.description());
    let triggered = "this deals damage equal to that card's mana value to any target";

    Some(format!(
        "{}. {}. When you reveal {matched_reference} this way, {}",
        consult_text.trim_end_matches('.'),
        remainder_text.trim_end_matches('.'),
        triggered
    ))
}

#[cfg(test)]
mod consult_characteristic_cleanup_tests {
    use super::*;

    #[test]
    fn keeps_match_and_full_revealed_collection_references_distinct() {
        let all_tag = TagKey::from("__sentence_helper__revealed_l0_s0_e1");
        let match_tag = TagKey::from("__sentence_helper__consult_match_l0_s0_e1");
        let consult = Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
            PlayerFilter::You,
            crate::effects::consult_helpers::LibraryConsultMode::Reveal,
            ObjectFilter::default().without_type(CardType::Land),
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch,
            all_tag.clone(),
            match_tag.clone(),
        ));
        let boost = Effect::new(
            crate::effects::ApplyContinuousEffect::with_spec_runtime(
                ChooseSpec::Source,
                crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                    power: Value::ManaValueOf(Box::new(ChooseSpec::Tagged(match_tag)))
                        .with_surface_hint(ValueSurfaceHint::WhereXIs),
                    toughness: Value::Fixed(0),
                },
                Until::EndOfTurn,
            )
            .require_creature_target(),
        );
        let cleanup = Effect::new(
            crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
                all_tag,
                None,
                crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses,
                PlayerFilter::You,
            ),
        );

        let rendered = describe_consult_characteristic_boost_then_all_revealed_bottom(&[
            consult, boost, cleanup,
        ])
        .expect("consult/boost/cleanup bundle");
        assert!(rendered.contains("where X is that card's mana value"));
        assert!(
            rendered.contains("Put the revealed cards on the bottom of your library in any order")
        );
    }

    #[test]
    fn restores_cleanup_before_linked_variable_damage_reflexive() {
        let all_tag = TagKey::from("__sentence_helper__revealed_l0_s0_e1");
        let match_tag = TagKey::from("__sentence_helper__consult_match_l0_s0_e1");
        let consult = Effect::with_id(
            7,
            Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
                PlayerFilter::You,
                crate::effects::consult_helpers::LibraryConsultMode::Reveal,
                ObjectFilter::default().without_type(CardType::Land),
                crate::effects::ConsultTopOfLibraryStopRule::FirstMatch,
                all_tag.clone(),
                match_tag.clone(),
            )),
        );
        let reflexive = Effect::reflexive_trigger(
            crate::effect::EffectId(7),
            EffectPredicate::Happened,
            vec![Effect::deal_damage(
                Value::ManaValueOf(Box::new(ChooseSpec::Tagged(match_tag))),
                ChooseSpec::AnyTarget,
            )],
            vec![ChooseSpec::AnyTarget],
        );
        let cleanup = Effect::new(
            crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
                all_tag,
                None,
                crate::effects::consult_helpers::LibraryBottomOrder::Random,
                PlayerFilter::You,
            ),
        );

        assert_eq!(
            describe_consult_reflexive_damage_then_all_revealed_bottom(&[
                consult, reflexive, cleanup,
            ])
            .as_deref(),
            Some(
                "Reveal cards from the top of your library until you reveal a nonland card. Put the revealed cards on the bottom of your library in a random order. When you reveal a nonland card this way, this deals damage equal to that card's mana value to any target"
            )
        );
    }
}

#[cfg(test)]
mod reveal_set_optional_move_tests {
    use super::*;

    fn for_each_player(effect: Effect) -> Effect {
        Effect::new(crate::effects::ForPlayersEffect {
            filter: PlayerFilter::Any,
            effects: vec![effect],
            starting_with_controller: false,
            stop_after_first_happened: false,
        })
    }

    #[test]
    fn preserves_plural_revealed_collection_across_players() {
        let tag = TagKey::from("revealed_each_player");
        let reveal = for_each_player(Effect::new(crate::effects::RevealTopEffect::tagged(
            PlayerFilter::IteratedPlayer,
            tag.clone(),
        )));
        let may_move = Effect::with_id(
            7,
            Effect::new(crate::effects::MayEffect::new_for_player(
                vec![Effect::new(crate::effects::MoveToZoneEffect::new(
                    ChooseSpec::Tagged(tag),
                    Zone::Graveyard,
                    false,
                ))],
                PlayerFilter::You,
            )),
        );
        let draw = Effect::if_then(
            crate::effect::EffectId(7),
            EffectPredicate::DidNotHappen,
            vec![for_each_player(Effect::new(
                crate::effects::DrawCardsEffect::new(Value::Fixed(1), PlayerFilter::IteratedPlayer),
            ))],
        );

        assert_eq!(
            describe_each_player_reveal_set_may_move_else_draw(&[reveal, may_move, draw])
                .as_deref(),
            Some(
                "Each player reveals the top card of their library. You may put the revealed cards into their owners' graveyards. If you don't, each player draws a card"
            )
        );
    }
}

pub(crate) fn describe_cross_segment_death_replacement_bundle(
    effects: &[Effect],
) -> Option<String> {
    let filtered = effects.iter().collect::<Vec<_>>();
    if let Some(rendered) = describe_damage_and_die_replacement_bundle(&filtered) {
        return Some(rendered);
    }
    if let Some(rendered) = describe_compound_damage_regeneration_exile_bundle(&filtered) {
        return Some(rendered);
    }

    let (replacement_effect, prefix) = effects.split_last()?;
    let replacement =
        replacement_effect.downcast_ref::<crate::effects::RegisterZoneReplacementEffect>()?;
    // A bare filtered tag is not sufficient proof that the producer actually
    // captured every object in that filter. Require either the exact result tag
    // or the independently checkable secondary target of a fight effect.
    let has_proven_link = matches!(&replacement.target, ChooseSpec::Tagged(_))
        || prefix.iter().any(|producer| {
            structural_unwrap_render_wrappers(producer)
                .downcast_ref::<crate::effects::FightEffect>()
                .is_some_and(|fight| {
                    target_specs_select_same_objects(&fight.creature2, &replacement.target)
                })
        });
    if !has_proven_link {
        return None;
    }
    describe_tagged_die_exile_replacement_bundle(&filtered)
}

fn collect_cross_segment_consult_effects<'a>(
    effect: &'a Effect,
    visible: &mut Vec<&'a Effect>,
) -> bool {
    let effect = structural_unwrap_render_wrappers(effect);
    if let Some(schedule) = effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>() {
        if schedule.effects.segments.iter().any(|segment| {
            !segment.self_replacements.is_empty() || segment.default_effects.is_empty()
        }) {
            return false;
        }
        return schedule
            .effects
            .flattened_default_effects()
            .iter()
            .all(|nested| collect_cross_segment_consult_effects(nested, visible));
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return sequence
            .effects
            .iter()
            .all(|nested| collect_cross_segment_consult_effects(nested, visible));
    }
    visible.push(effect);
    true
}

fn effect_consumes_cross_segment_consult_tag(
    effect: &Effect,
    match_tag: &TagKey,
    all_tag: &TagKey,
) -> bool {
    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        return choose_spec_references_tagged_object(&move_to_zone.target, match_tag)
            || choose_spec_references_tagged_object(&move_to_zone.target, all_tag);
    }
    if let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>() {
        return for_each.tag == *match_tag || for_each.tag == *all_tag;
    }
    effect
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()
        .is_some_and(|bottom| {
            bottom.tag == *all_tag
                && bottom
                    .keep_tagged
                    .as_ref()
                    .is_none_or(|keep| keep == match_tag)
        })
}

/// Rejoin source-sentence segments only when an existing typed consult
/// renderer recognizes the complete flattened window. The unique consult and
/// exact result-tag dependency keep this presentation-only compaction from
/// merging independent library actions or crossing optional/control-flow
/// containers.
pub(crate) fn describe_cross_segment_consult_bundle(effects: &[Effect]) -> Option<String> {
    let mut visible = Vec::new();
    if !effects
        .iter()
        .all(|effect| collect_cross_segment_consult_effects(effect, &mut visible))
    {
        return None;
    }

    let consults = visible
        .iter()
        .enumerate()
        .filter_map(|(index, effect)| {
            effect
                .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()
                .map(|consult| (index, consult))
        })
        .collect::<Vec<_>>();
    let [(consult_index, consult)] = consults.as_slice() else {
        return None;
    };
    let consult_index = *consult_index;
    let consult = *consult;
    if !visible[consult_index + 1..].iter().any(|effect| {
        effect_consumes_cross_segment_consult_tag(effect, &consult.match_tag, &consult.all_tag)
    }) {
        return None;
    }

    if let Some(rendered) = describe_structural_multisentence_effect_list(effects) {
        return Some(rendered);
    }

    if let Some(rendered) = describe_choose_name_exile_top_consult_hand_rest_exile(&visible) {
        return Some(rendered);
    }

    let effect_refs = effects.iter().collect::<Vec<_>>();
    if let Some(rendered) = describe_consult_match_destination_alternative(&effect_refs) {
        return Some(rendered);
    }
    if let Some(rendered) = describe_consult_conditional_may_cast_remainder_bottom(&effect_refs) {
        return Some(rendered);
    }
    if let Some(rendered) =
        describe_exile_creatures_consult_that_many_battlefield_shuffle(&effect_refs)
    {
        return Some(rendered);
    }
    if let Some(rendered) = describe_consult_battlefield_attachment_remainder(effects) {
        return Some(rendered);
    }

    // Source-sentence segmentation can leave an authored setup action in the
    // same segment as the consult while wrapping the matched-card move and
    // remainder disposition in the following segment. Render the proven
    // consult suffix with its collection-aware helper, then retain the setup
    // prefix instead of falling back to singular generic pronouns.
    if effects.len() >= 3 {
        let suffix_start = effects.len() - 2;
        let suffix = effects[suffix_start..].iter().collect::<Vec<_>>();
        if let Some(rendered_suffix) = describe_consult_reveal_move_matches_then_bottom(&suffix) {
            let rendered_prefix = describe_effect_list(&effects[..suffix_start])
                .trim()
                .trim_end_matches('.')
                .to_string();
            if !rendered_prefix.is_empty() {
                return Some(format!("{rendered_prefix}. {rendered_suffix}"));
            }
        }
    }

    if visible.len() == 3 {
        if consult_index == 1
            && let Some(target) = visible[0].downcast_ref::<crate::effects::TargetOnlyEffect>()
            && target.chooser.is_none()
            && !target.explicit_declaration
            && choose_spec_player_filter(&target.target).as_ref() == Some(&consult.player)
            && let Some(rendered) =
                render_consult_reveal_put_all_revealed_into_graveyard(&[visible[1], visible[2]])
        {
            return Some(rendered);
        }
        if let Some(rendered) = describe_counted_consult_matches_to_graveyard_then_bottom(
            visible[0], visible[1], visible[2],
        ) {
            return Some(rendered);
        }
        if let Some(rendered) = render_consult_reveal_put_hand_then_bottom(&visible) {
            return Some(rendered);
        }
        if render_consult_reveal_put_hand_rest_graveyard(&visible).is_some() {
            if consult.player == PlayerFilter::You {
                let consult_text = describe_effect(visible[0])
                    .trim()
                    .trim_end_matches('.')
                    .strip_prefix("You ")
                    .map(capitalize_first)?;
                return Some(format!(
                    "{consult_text}. Put that card into your hand and all other cards revealed this way into your graveyard"
                ));
            }
            return render_consult_reveal_put_hand_rest_graveyard(&visible);
        }
        if render_consult_reveal_put_hand_rest_exile(&visible).is_some() {
            if consult.player == PlayerFilter::You {
                let consult_text = describe_effect(visible[0])
                    .trim()
                    .trim_end_matches('.')
                    .strip_prefix("You ")
                    .map(capitalize_first)?;
                return Some(format!(
                    "{consult_text}. Put that card into your hand and exile all other cards revealed this way"
                ));
            }
            return render_consult_reveal_put_hand_rest_exile(&visible);
        }
        if let Some(rendered) = describe_consult_reveal_put_battlefield_then_bottom(&visible) {
            return Some(rendered);
        }
    }
    if visible.len() == 4
        && consult_index == 1
        && let Some(rendered) = describe_target_opponent_consult_remainder_then_match(&visible)
    {
        return Some(rendered);
    }
    if visible.len() == 5
        && consult_index == 1
        && let Some(rendered) = describe_targeted_opponent_consult_may_cast_remainder(&visible)
    {
        return Some(rendered);
    }

    if effects.len() == 4
        && visible.len() == 4
        && consult_index == 1
        && visible[0]
            .downcast_ref::<crate::effects::ChooseCreatureTypeEffect>()
            .is_some()
        && visible[2]
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
            .is_some()
        && visible[3]
            .downcast_ref::<crate::effects::ShuffleLibraryEffect>()
            .is_some()
        && let Some(rendered) = describe_structural_multisentence_effect_list(effects)
    {
        return Some(rendered);
    }

    if effects.len() == 5 && visible.len() == 5 && consult_index == 0 {
        let exile = visible[1].downcast_ref::<crate::effects::MoveToZoneEffect>()?;
        let bottom =
            visible[4].downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
        if exile.zone == Zone::Exile
            && choose_spec_references_tagged_object(&exile.target, &consult.match_tag)
            && bottom.tag == consult.all_tag
            && bottom.keep_tagged.as_ref() == Some(&consult.match_tag)
            && bottom.player == consult.player
            && describe_exile_with_counters_then_gain_suspend(&effects[1..4]).is_some()
        {
            return describe_pre_clause_structural_effect_list(effects);
        }
    }

    None
}

fn describe_sequence_wrapped_search_two_split(effects: &[Effect]) -> Option<String> {
    let sequence = effects
        .first()?
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    let trailing = &effects[1..];
    if !matches!(
        (sequence.effects.len(), trailing.len()),
        (6 | 7, 0) | (5, 1 | 2)
    ) {
        return None;
    }

    // The existing matcher proves the search/reveal/exact split plus the
    // shuffle and optional scry suffix.  This bridge only restores the source
    // sentence boundary introduced by lowering, whether lowering wrapped the
    // complete procedure or left the shuffle/scry suffix outside. It cannot
    // make a partial or differently shaped sequence eligible for the compact
    // surface.
    let mut flattened = Vec::with_capacity(sequence.effects.len() + trailing.len());
    flattened.extend(sequence.effects.iter());
    flattened.extend(trailing.iter());
    describe_search_two_split_hand_graveyard_sequence(&flattened)
}

fn tagged_consult_match_moves_to_your_hand(effect: &Effect, match_tag: &TagKey) -> bool {
    let Some(move_to_zone) = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
    else {
        return false;
    };
    move_to_zone.zone == Zone::Hand
        && !move_to_zone.to_top
        && move_to_zone.verb_surface != ironsmith_core::MoveToZoneVerbSurface::Return
        && matches!(
            move_to_zone.target.base(),
            ChooseSpec::Tagged(tag) if tag == match_tag
        )
        && matches!(
            move_to_zone.actor_surface.as_ref(),
            None | Some(PlayerFilter::You)
        )
        && matches!(
            move_to_zone.destination_player_surface.as_ref(),
            None | Some(PlayerFilter::You)
        )
        && move_to_zone.destination_player_reference_surface.is_none()
}

fn tagged_consult_match_moves_to_battlefield(effect: &Effect, match_tag: &TagKey) -> bool {
    let Some(move_to_zone) = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
    else {
        return false;
    };
    move_to_zone.zone == Zone::Battlefield
        && !move_to_zone.to_top
        && move_to_zone.library_order.is_none()
        && !move_to_zone.target_plural_surface
        && matches!(
            move_to_zone.target.base(),
            ChooseSpec::Tagged(tag) if tag == match_tag
        )
        && matches!(
            move_to_zone.actor_surface.as_ref(),
            None | Some(PlayerFilter::You)
        )
        && move_to_zone.destination_player_surface.is_none()
        && move_to_zone.destination_player_reference_surface.is_none()
        && move_to_zone.exiled_with_source_surface.is_none()
        && move_to_zone.battlefield_controller == crate::effects::BattlefieldController::Preserve
        && !move_to_zone.controller_surface_explicit
        && move_to_zone.enters_with_counters.is_empty()
        && !move_to_zone.enters_tapped
        && !move_to_zone.enters_attacking
        && move_to_zone.attack_target_mode.is_none()
        && !move_to_zone.enters_face_down
        && !move_to_zone.transfer_exiled_with_source_links
}

fn is_single_result_consult(consult: &crate::effects::ConsultTopOfLibraryEffect) -> bool {
    consult.player == PlayerFilter::You
        && consult.max_exposed.is_none()
        && matches!(
            &consult.stop_rule,
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
                | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1))
        )
}

fn imperative_consult_text(effect: &Effect) -> Option<String> {
    let rendered = describe_effect(effect);
    let rendered = rendered.trim().trim_end_matches('.');
    let rendered = rendered
        .strip_prefix("you ")
        .or_else(|| rendered.strip_prefix("You "))
        .unwrap_or(rendered);
    (!rendered.is_empty()).then(|| capitalize_first(rendered))
}

fn consult_remainder_order_suffix(
    order: crate::effects::consult_helpers::LibraryBottomOrder,
) -> &'static str {
    match order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => " in a random order",
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => " in any order",
    }
}

/// Render the singular result of a library consult when the same tagged card
/// must go either to the battlefield or to its owner's hand. The linked
/// `WithIdEffect`/`IfEffect` pair proves that declining (or failing) the first
/// move selects the second destination; an optional tagged remainder proves
/// the exact looked-minus-result collection.
fn describe_consult_match_destination_alternative(effects: &[&Effect]) -> Option<String> {
    if let [consult_effect, optional_effect, remainder_effect] = effects {
        let consult = structural_unwrap_render_wrappers(consult_effect)
            .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
        let optional = structural_unwrap_render_wrappers(optional_effect)
            .downcast_ref::<crate::effects::MayEffect>()?;
        let [battlefield_effect] = optional.effects.as_slice() else {
            return None;
        };
        let remainder = structural_unwrap_render_wrappers(remainder_effect)
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )?;
        if is_single_result_consult(consult)
            && consult.mode == crate::effects::consult_helpers::LibraryConsultMode::Reveal
            && matches!(optional.decider.as_ref(), None | Some(PlayerFilter::You))
            && optional.fallback == crate::decision::FallbackStrategy::Decline
            && tagged_consult_match_moves_to_battlefield(battlefield_effect, &consult.match_tag)
            && remainder.tag == consult.all_tag
            && remainder.keep_tagged.as_ref() == Some(&consult.match_tag)
            && remainder.player == consult.player
            && matches!(
                remainder.surface,
                ironsmith_core::LibraryRemainderSurface::Rest
                    | ironsmith_core::LibraryRemainderSurface::RevealedCardsNotPutOntoBattlefield
            )
        {
            let consult_text = imperative_consult_text(consult_effect)?;
            let order = consult_remainder_order_suffix(remainder.order);
            return Some(format!(
                "{consult_text}. You may put that card onto the battlefield. Then put all cards revealed this way that weren't put onto the battlefield on the bottom of your library{order}"
            ));
        }
    }

    let (consult_effect, attempted_effect, fallback_effect, remainder_effect) = match effects {
        [consult, attempted, fallback] => (*consult, *attempted, *fallback, None),
        [consult, attempted, fallback, remainder] => {
            (*consult, *attempted, *fallback, Some(*remainder))
        }
        _ => return None,
    };

    let consult = structural_unwrap_render_wrappers(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if !is_single_result_consult(consult) {
        return None;
    }

    let attempted = attempted_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = attempted
        .effect
        .downcast_ref::<crate::effects::MayEffect>()?;
    let [battlefield_effect] = may.effects.as_slice() else {
        return None;
    };
    let fallback = fallback_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if !matches!(may.decider.as_ref(), None | Some(PlayerFilter::You))
        || may.fallback != crate::decision::FallbackStrategy::Decline
        || fallback.condition != attempted.id
        || !matches!(
            fallback.predicate,
            EffectPredicate::DidNotHappen | EffectPredicate::WasDeclined
        )
        || !fallback.else_.is_empty()
        || !tagged_consult_match_moves_to_battlefield(battlefield_effect, &consult.match_tag)
        || !matches!(fallback.then.as_slice(), [hand_effect]
            if tagged_consult_match_moves_to_your_hand(hand_effect, &consult.match_tag))
    {
        return None;
    }

    let consult_text = imperative_consult_text(consult_effect)?;
    let Some(remainder_effect) = remainder_effect else {
        return Some(format!(
            "{consult_text}. Put that card onto the battlefield or into your hand"
        ));
    };
    let remainder = structural_unwrap_render_wrappers(remainder_effect)
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if remainder.tag != consult.all_tag
        || remainder.keep_tagged.as_ref() != Some(&consult.match_tag)
        || remainder.player != consult.player
        || remainder.surface != ironsmith_core::LibraryRemainderSurface::Rest
    {
        return None;
    }
    let order = consult_remainder_order_suffix(remainder.order);
    Some(format!(
        "{consult_text}. You may put that card onto the battlefield. If you don't, put it into your hand. Put the rest on the bottom of your library{order}"
    ))
}

/// Render a revealed consult result whose destination is selected by a
/// condition, followed by the exact looked-minus-result remainder. The shared
/// consult tags prove both singular `it` references and permit the authored
/// bare `the rest` surface without naming an unrelated revealed collection.
fn describe_consult_conditional_destination_remainder(effects: &[&Effect]) -> Option<String> {
    let [consult_effect, conditional_effect, remainder_effect] = effects else {
        return None;
    };
    let consult = structural_unwrap_render_wrappers(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if !is_single_result_consult(consult)
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
    {
        return None;
    }
    let conditional = structural_unwrap_render_wrappers(conditional_effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    let [battlefield_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let [hand_effect] = conditional.if_false.as_slice() else {
        return None;
    };
    if conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf
        || !tagged_consult_match_moves_to_battlefield(battlefield_effect, &consult.match_tag)
        || !tagged_consult_match_moves_to_your_hand(hand_effect, &consult.match_tag)
    {
        return None;
    }
    let remainder = structural_unwrap_render_wrappers(remainder_effect)
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if remainder.tag != consult.all_tag
        || remainder.keep_tagged.as_ref() != Some(&consult.match_tag)
        || remainder.player != consult.player
        || remainder.surface != ironsmith_core::LibraryRemainderSurface::Rest
    {
        return None;
    }

    let consult_text = imperative_consult_text(consult_effect)?;
    let conditional_text = capitalize_first(
        describe_effect(conditional_effect)
            .trim()
            .trim_end_matches('.'),
    )
    .replace(", Put ", ", put ")
    .replace(". Otherwise, Put ", ". Otherwise, put ");
    if !conditional_text.starts_with("If ") || !conditional_text.contains(". Otherwise, ") {
        return None;
    }
    let order = consult_remainder_order_suffix(remainder.order);
    Some(format!(
        "{consult_text}. {conditional_text}. Put the rest on the bottom of your library{order}"
    ))
}

/// Render a conditional free-cast of the singular consult result followed by
/// disposal of every still-exiled member of the consult collection. Requiring
/// `keep_tagged: None` is what proves that the matched card joins the remainder
/// exactly when it was not cast.
fn describe_consult_conditional_may_cast_remainder_bottom(effects: &[&Effect]) -> Option<String> {
    let [consult_effect, conditional_effect, remainder_effect] = effects else {
        return None;
    };
    let consult = structural_unwrap_render_wrappers(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if !is_single_result_consult(consult)
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Exile
    {
        return None;
    }

    let conditional = structural_unwrap_render_wrappers(conditional_effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    let Condition::ValueComparison {
        left,
        operator,
        right,
    } = &conditional.condition
    else {
        return None;
    };
    let comparison = match operator {
        crate::effect::ValueComparisonOperator::LessThan => {
            format!("less than {}", describe_value(right))
        }
        crate::effect::ValueComparisonOperator::LessThanOrEqual => match right {
            Value::Fixed(n) => format!("{n} or less"),
            right => format!("less than or equal to {}", describe_value(right)),
        },
        _ => return None,
    };
    if conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf
        || !conditional.if_false.is_empty()
        || !matches!(
            left,
            Value::ManaValueOf(spec)
                if matches!(spec.as_ref().base(), ChooseSpec::Tagged(tag) if tag == &consult.match_tag)
        )
    {
        return None;
    }
    let [may_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let may = structural_unwrap_render_wrappers(may_effect)
        .downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast = structural_unwrap_render_wrappers(cast_effect)
        .downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if !matches!(may.decider.as_ref(), None | Some(PlayerFilter::You))
        || may.fallback != crate::decision::FallbackStrategy::Decline
        || cast.tag != consult.match_tag
        || cast.player != PlayerFilter::You
        || cast.allow_land
        || cast.as_copy
        || !cast.without_paying_mana_cost
        || cast.cost_reduction.is_some()
    {
        return None;
    }

    let remainder = structural_unwrap_render_wrappers(remainder_effect)
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if remainder.tag != consult.all_tag
        || remainder.keep_tagged.is_some()
        || remainder.player != consult.player
        || remainder.surface != ironsmith_core::LibraryRemainderSurface::Rest
    {
        return None;
    }
    let consult_text = imperative_consult_text(consult_effect)?;
    let order = consult_remainder_order_suffix(remainder.order);
    Some(format!(
        "{consult_text}. You may cast the exiled card without paying its mana cost if it's a spell with mana value {comparison}. Put the exiled cards not cast this way on the bottom of your library{order}"
    ))
}

/// Render an optional free cast of the singular card found by a consult, with
/// the same tagged card moving to its owner's hand on either failed condition
/// or declined cast. When the consult is explicitly from your library, the
/// tag edge proves that owner is you, so the source-equivalent destination is
/// "your hand" rather than the generic "its owner's hand".
fn describe_consult_exile_may_cast_else_your_hand(effects: &[Effect]) -> Option<String> {
    let [consult_effect, conditional_effect] = effects else {
        return None;
    };
    let consult = structural_unwrap_render_wrappers(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.player != PlayerFilter::You
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Exile
        || consult.max_exposed.is_some()
        || !matches!(
            &consult.stop_rule,
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
                | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1))
        )
    {
        return None;
    }

    let conditional = structural_unwrap_render_wrappers(conditional_effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    let Condition::ValueComparison {
        left,
        operator,
        right,
    } = &conditional.condition
    else {
        return None;
    };
    let comparison = match operator {
        crate::effect::ValueComparisonOperator::LessThan => {
            format!("less than {}", describe_value(right))
        }
        crate::effect::ValueComparisonOperator::LessThanOrEqual => match right {
            Value::Fixed(n) => format!("{n} or less"),
            right => format!("less than or equal to {}", describe_value(right)),
        },
        _ => return None,
    };
    if conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf
        || !matches!(
            left,
            Value::ManaValueOf(spec)
                if matches!(spec.as_ref().base(), ChooseSpec::Tagged(tag) if tag == &consult.match_tag)
        )
    {
        return None;
    }

    let [may_effect, declined_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let with_id = may_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast = structural_unwrap_render_wrappers(cast_effect)
        .downcast_ref::<crate::effects::CastTaggedEffect>()?;
    let declined = structural_unwrap_render_wrappers(declined_effect)
        .downcast_ref::<crate::effects::IfEffect>()?;
    if !matches!(may.decider.as_ref(), None | Some(PlayerFilter::You))
        || may.fallback != crate::decision::FallbackStrategy::Decline
        || cast.tag != consult.match_tag
        || cast.player != PlayerFilter::You
        || cast.allow_land
        || cast.as_copy
        || !cast.without_paying_mana_cost
        || cast.cost_reduction.is_some()
        || declined.condition != with_id.id
        || !matches!(
            declined.predicate,
            EffectPredicate::WasDeclined | EffectPredicate::DidNotHappen
        )
        || !declined.else_.is_empty()
        || !matches!(declined.then.as_slice(), [move_effect]
            if tagged_consult_match_moves_to_your_hand(move_effect, &consult.match_tag))
        || !matches!(conditional.if_false.as_slice(), [move_effect]
            if tagged_consult_match_moves_to_your_hand(move_effect, &consult.match_tag))
    {
        return None;
    }

    let consult_text =
        capitalize_first(describe_effect(consult_effect).trim().trim_end_matches('.'));
    Some(format!(
        "{consult_text}. You may cast that card without paying its mana cost if the spell's mana value is {comparison}. If you don't cast that card this way, put it into your hand"
    ))
}

pub(crate) fn describe_choose_exiled_card_then_play_without_paying(
    effects: &[Effect],
) -> Option<String> {
    let [choose_effect, grant_play_effect, grant_free_effect] = effects else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let grant_play = structural_unwrap_render_wrappers(grant_play_effect)
        .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    let grant_free = structural_unwrap_render_wrappers(grant_free_effect)
        .downcast_ref::<crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect>(
    )?;

    if choose.count.min != 1
        || choose.count.max != Some(1)
        || choose.count.dynamic_x
        || choose.count.up_to_x
        || choose.count.random
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || choose.chooser != PlayerFilter::You
        || !choose.additional_zones.is_empty()
        || choose.is_search
        || choose.reveal
        || choose.search_mode != ironsmith_core::SearchSelectionMode::Exact
        || choose.top_only
        || choose.bottom_only
        || choose.replace_tagged_objects
        || !choose.description.eq_ignore_ascii_case("choose")
        || grant_play.tag != choose.tag
        || grant_free.tag != choose.tag
        || grant_play.player != PlayerFilter::You
        || grant_free.player != PlayerFilter::You
        || grant_play.duration != crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
        || grant_free.duration != crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
        || !grant_play.allow_land
        || grant_play.mana_spend_mode != ironsmith_core::value_model::ManaSpendMode::Normal
        || grant_play.allow_any_color_for_cast
        || grant_play.while_on_top_of_library
        || grant_free.while_on_top_of_library
        || grant_free.zone != Some(Zone::Exile)
        || grant_play.filter.is_some()
        || grant_play.cast_pool_is_plural
    {
        return None;
    }

    let effective_zone = choose.zone.or(choose.filter.zone)?;
    if effective_zone != Zone::Exile {
        return None;
    }
    let mut filter = choose.filter.clone();
    filter.zone = None;
    let owner = filter.owner.take()?;
    let counter = match filter.with_counter.take()? {
        crate::filter::CounterConstraint::Typed(counter) => counter,
        crate::filter::CounterConstraint::Any
        | crate::filter::CounterConstraint::AtLeast { .. } => return None,
    };
    if filter != ObjectFilter::default() {
        return None;
    }
    let owner = match owner {
        PlayerFilter::Opponent => "an opponent owns",
        PlayerFilter::You => "you own",
        PlayerFilter::NotYou => "you don't own",
        PlayerFilter::Any => "a player owns",
        _ => return None,
    };
    let counter = with_indefinite_article(&format!("{} counter", counter.description()));
    Some(format!(
        "Choose an exiled card {owner} with {counter} on it. You may play it this turn without paying its mana cost"
    ))
}

/// Render the complete three-zone same-name extraction family from its typed
/// producer/consumer edges. Keeping this entry point independent of sentence
/// boundaries lets the resolution-program renderer reuse the same proof when
/// lowering has split the authored procedure across multiple segments.
pub(in crate::compiled_text) fn describe_same_name_three_zone_extraction(
    effects: &[Effect],
) -> Option<String> {
    let same_name_refs = effects.iter().collect::<Vec<_>>();
    if let Some(compact) = render_reveal_hand_choose_exile_each_same_name_shuffle(&same_name_refs) {
        return Some(compact);
    }
    if let Some(compact) = render_reveal_hand_choose_same_name_exile_shuffle(&same_name_refs) {
        return Some(compact);
    }
    if let Some(compact) = render_choose_name_search_same_name_exile_shuffle(&same_name_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_countered_spell_same_name_search_sequence(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_target_card_same_name_extraction(&same_name_refs) {
        return Some(compact);
    }
    if let Some(compact) =
        describe_exile_target_search_same_name_exile_shuffle_bundle(&same_name_refs)
    {
        return Some(compact);
    }
    None
}

/// Render a delayed combat set as the distributive instruction Oracle uses.
///
/// The tag producer is semantically significant: it freezes the set selected
/// by the combat-history filter so the following untap restriction applies to
/// those same permanents. Rendering the tap and restriction independently
/// loses that relationship and produces an incorrect plural subject followed
/// by an unrelated singular pronoun.
fn describe_tagged_blocked_set_tap_then_next_untap(effects: &[Effect]) -> Option<String> {
    let [tag_effect, tap_effect, cant_effect] = effects else {
        return None;
    };
    let tagged = structural_unwrap_render_wrappers(tag_effect)
        .downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    if tagged.zone.is_some() || !tagged.additional_zones.is_empty() {
        return None;
    }

    let blocker = tagged.filter.blocked_by.as_ref()?;
    let mut expected_filter = ObjectFilter::creature();
    expected_filter.blocked = true;
    expected_filter.blocked_by = Some(blocker.clone());
    if tagged.filter != expected_filter {
        return None;
    }

    let tap = structural_unwrap_render_wrappers(tap_effect)
        .downcast_ref::<crate::effects::TapEffect>()?;
    let ChooseSpec::All(tap_filter) = tap.target.base() else {
        return None;
    };
    if tap_filter != &tagged.filter {
        return None;
    }

    let cant = structural_unwrap_render_wrappers(cant_effect)
        .downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::Untap(restricted_filter) = &cant.restriction else {
        return None;
    };
    if cant.duration != Until::ControllersNextUntapStep
        || restricted_filter != &ObjectFilter::tagged(tagged.tag.clone())
    {
        return None;
    }

    let blocker = match blocker {
        crate::filter::ObjectRef::Target => "target creature",
        crate::filter::ObjectRef::Specific(_) => "that creature",
        crate::filter::ObjectRef::Tagged(tag) if tag.as_str() == "blocking" => {
            "the blocking creature"
        }
        crate::filter::ObjectRef::Tagged(_) => "one of those creatures",
    };
    Some(format!(
        "Tap each creature that was blocked by {blocker} this turn and it doesn't untap during its controller's next untap step"
    ))
}

pub(in crate::compiled_text) fn describe_explicit_target_then_coin_flip(
    effects: &[Effect],
) -> Option<(String, usize)> {
    let (target_effect, flip_effect, trailing_effects, nested_sequence) =
        if let [first_effect, trailing_effects @ ..] = effects
            && let Some(sequence) = first_effect.downcast_ref::<crate::effects::SequenceEffect>()
            && sequence.surface == ironsmith_core::SequenceSurface::CommaThen
            && let [target_effect, flip_effect] = sequence.effects.as_slice()
        {
            (target_effect, flip_effect, trailing_effects, true)
        } else {
            let [target_effect, flip_effect, trailing_effects @ ..] = effects else {
                return None;
            };
            (target_effect, flip_effect, trailing_effects, false)
        };
    let target = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let flip = structural_unwrap_render_wrappers(flip_effect)
        .downcast_ref::<crate::effects::FlipCoinEffect>()?;
    if !target.explicit_declaration
        || target.chooser.is_some()
        || flip.player != PlayerFilter::You
        || flip.forced_face.is_some()
        || flip.forced_winner.is_some()
        || flip.forced_loser.is_some()
    {
        return None;
    };
    let target_rendered = describe_effect(target_effect);
    let target_text = target_rendered.trim().trim_end_matches('.');
    // Keep the result-producing flip in the recursively rendered suffix. If
    // it is consumed here, later `IfEffect` branches lose their typed
    // antecedent and fall back to raw surfaces such as "effect #0 happened".
    let flip_and_results = if nested_sequence {
        let mut linked_results = Vec::with_capacity(trailing_effects.len() + 1);
        linked_results.push(flip_effect.clone());
        linked_results.extend_from_slice(trailing_effects);
        describe_effect_list(&linked_results)
    } else {
        describe_effect_list(&effects[1..])
    };
    Some((
        format!(
            "{target_text}, then {}",
            lowercase_first(flip_and_results.trim().trim_end_matches('.'))
        ),
        effects.len(),
    ))
}

/// Render a choice nested inside a conditional collection reference:
/// "that creature's controller gains control of one of those lands of their
/// choice and untaps it."
///
/// The choice tag is runtime identity, not display text.  Recognizing the
/// producer and both consumers together prevents the internal tagged-object
/// placeholder from leaking when the control and untap effects are rendered
/// independently.
pub(in crate::compiled_text) fn describe_condition_collection_choice_gain_control_then_untap(
    effects: &[Effect],
) -> Option<String> {
    let effects = if effects.first().is_some_and(|effect| {
        structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some_and(|tag| tag.tag.as_str() == "triggering")
    }) {
        &effects[1..]
    } else {
        effects
    };
    let [choose_effect, gain_effect, untap_effect] = effects else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.count.is_single()
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || choose.is_search
        || choose.filter.card_types.as_slice() != [CardType::Land]
        || choose.filter.with_counter.is_none()
        || !matches!(
            choose.chooser,
            PlayerFilter::DamagedPlayer | PlayerFilter::IteratedPlayer
        )
    {
        return None;
    }

    let gain = structural_unwrap_render_wrappers(gain_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if gain.until != Until::Forever
        || gain.condition.is_some()
        || gain.modification.is_some()
        || !gain.additional_modifications.is_empty()
        || gain.runtime_modifications.len() != 1
        || !matches!(
            gain.target_spec.as_ref().map(ChooseSpec::base),
            Some(ChooseSpec::Tagged(tag)) if tag == &choose.tag
        )
        || !matches!(
            gain.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(
                PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag))
            )] if tag.as_str() == "triggering"
        )
    {
        return None;
    }

    let untap = structural_unwrap_render_wrappers(untap_effect)
        .downcast_ref::<crate::effects::UntapEffect>()?;
    if !matches!(untap.target.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag) {
        return None;
    }

    Some(
        "That creature's controller gains control of one of those lands of their choice and untaps it"
            .to_string(),
    )
}

fn describe_repeated_explore_pair(effects: &[Effect]) -> Option<String> {
    let [first_effect, second_effect] = effects else {
        return None;
    };
    let first = structural_unwrap_render_wrappers(first_effect)
        .downcast_ref::<crate::effects::ExploreEffect>()?;
    let second = structural_unwrap_render_wrappers(second_effect)
        .downcast_ref::<crate::effects::ExploreEffect>()?;
    let repeats_first_result = effect_outer_tag(first_effect).is_some_and(|first_result_tag| {
        matches!(
            second.target.base(),
            ChooseSpec::Tagged(second_target_tag)
                if second_target_tag == first_result_tag
        )
    });
    if first.target != second.target && !repeats_first_result {
        return None;
    }

    let first_text = describe_effect(structural_unwrap_render_wrappers(first_effect));
    let first_text = first_text.trim().trim_end_matches('.');
    (!first_text.is_empty()).then(|| format!("{first_text}, then it explores again"))
}

/// Preserve a per-player reveal partition as one coordinated instruction.
///
/// The iterator, shared reveal tag, exhaustive permanent/nonpermanent
/// conditional, and owner-preserving destinations prove both halves consume
/// the same revealed set. Rejoining this typed shape avoids weakening the
/// authored "each player ... puts ... and puts the rest" relationship into
/// three unrelated imperative sentences.
fn describe_each_player_reveal_permanents_and_rest(effects: &[Effect]) -> Option<String> {
    let [for_players_effect] = effects else {
        return None;
    };
    let for_players = structural_unwrap_render_wrappers(for_players_effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::Any
        || for_players.starting_with_controller
        || for_players.stop_after_first_happened
    {
        return None;
    }

    let [look_effect, reveal_effect, partition_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let look = structural_unwrap_render_wrappers(look_effect)
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let reveal = structural_unwrap_render_wrappers(reveal_effect)
        .downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    let partition = structural_unwrap_render_wrappers(partition_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if look.player != PlayerFilter::IteratedPlayer
        || look.reveal
        || reveal.tag != look.tag
        || partition.tag != look.tag
    {
        return None;
    }

    let Value::Count(count_filter) = look.count.unhinted() else {
        return None;
    };
    let expected_count_filter = ObjectFilter {
        zone: Some(Zone::Battlefield),
        controller: Some(PlayerFilter::IteratedPlayer),
        card_types: ObjectFilter::permanent_card().card_types,
        excluded_card_types: vec![CardType::Land],
        ..ObjectFilter::default()
    };
    if count_filter != &expected_count_filter {
        return None;
    }

    let [conditional_effect] = partition.effects.as_slice() else {
        return None;
    };
    let conditional = structural_unwrap_render_wrappers(conditional_effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    let Condition::TaggedObjectMatches(tag, filter) = &conditional.condition else {
        return None;
    };
    if tag.as_str() != "__it__"
        || filter != &ObjectFilter::permanent_card()
        || conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf
    {
        return None;
    }
    let [battlefield_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let [graveyard_effect] = conditional.if_false.as_slice() else {
        return None;
    };
    let battlefield = structural_unwrap_render_wrappers(battlefield_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let graveyard = structural_unwrap_render_wrappers(graveyard_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;

    let mut expected_battlefield =
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Iterated, Zone::Battlefield, false);
    expected_battlefield.verb_surface = ironsmith_core::MoveToZoneVerbSurface::Put;
    expected_battlefield.battlefield_controller = crate::effects::BattlefieldController::Owner;
    expected_battlefield.controller_surface_explicit = true;
    let mut expected_graveyard =
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Iterated, Zone::Graveyard, false);
    expected_graveyard.verb_surface = ironsmith_core::MoveToZoneVerbSurface::Put;
    if battlefield != &expected_battlefield || graveyard != &expected_graveyard {
        return None;
    }

    Some(
        "Each player reveals a number of cards from the top of their library equal to the number of nonland permanents they control, puts all permanent cards they revealed this way onto the battlefield, and puts the rest into their graveyard"
            .to_string(),
    )
}

/// Fold a synthetic target declaration into a mass action whose executable
/// set is defined relative to that target's current combat. Keeping these as
/// two rendered actions produces the misleading internal surface "Choose
/// target creature" instead of the authored embedded target phrase.
fn describe_target_relative_combat_set_prefix(effects: &[Effect]) -> Option<(String, usize)> {
    let target_effect = effects.first()?;
    let target_only = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target_only.explicit_declaration || !target_only.target.is_target() {
        return None;
    }
    let target_description = describe_choose_spec(&target_only.target);

    let embed_target = |filter: &ObjectFilter| -> Option<String> {
        if !matches!(
            filter.in_combat_with,
            Some(crate::filter::ObjectRef::Target)
        ) {
            return None;
        }
        let description = describe_choose_spec(&ChooseSpec::All(filter.clone()));
        let description = description
            .strip_prefix("all ")
            .unwrap_or(&description)
            .to_string();
        Some(description.replacen("target creature", &target_description, 1))
    };

    let mut consumed = 2;
    let mut captured_filter = None;
    let mut set_effect = effects.get(1)?;
    if let Some(capture) = structural_unwrap_render_wrappers(set_effect)
        .downcast_ref::<crate::effects::TagMatchingObjectsEffect>()
    {
        if capture.zone.is_some()
            || !capture.additional_zones.is_empty()
            || !capture.source_tags.is_empty()
        {
            return None;
        }
        captured_filter = Some(&capture.filter);
        set_effect = effects.get(2)?;
        consumed = 3;
    }
    let set_effect = structural_unwrap_render_wrappers(set_effect);
    if let Some(tap) = set_effect.downcast_ref::<crate::effects::TapEffect>() {
        let ChooseSpec::All(filter) = tap.target.base() else {
            return None;
        };
        if !filter.blocking || captured_filter.is_some_and(|captured| captured != filter) {
            return None;
        }
        return Some((format!("Tap all {}", embed_target(filter)?), consumed));
    }
    if let Some(return_to_hand) = set_effect.downcast_ref::<crate::effects::ReturnToHandEffect>() {
        let ChooseSpec::All(filter) = return_to_hand.spec.base() else {
            return None;
        };
        if return_to_hand.actor_surface.is_some()
            || return_to_hand.destination_player_surface.is_some()
            || return_to_hand.exiled_with_source_surface.is_some()
        {
            return None;
        }
        return Some((
            format!("Return all {} to their owner's hand", embed_target(filter)?),
            consumed,
        ));
    }
    None
}

fn describe_owned_hand_or_graveyard_entry_with_counters(effects: &[Effect]) -> Option<String> {
    let (effects, optional) = match effects {
        [may_effect] => {
            let may = structural_unwrap_render_wrappers(may_effect)
                .downcast_ref::<crate::effects::MayEffect>()?;
            if !matches!(may.decider.as_ref(), None | Some(PlayerFilter::You))
                || may.fallback != crate::decision::FallbackStrategy::Decline
            {
                return None;
            }
            (may.effects.as_slice(), true)
        }
        effects => (effects, false),
    };
    let [choose_effect, move_effect] = effects else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::You
        || !choose.count.is_single()
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || choose.zone != Some(Zone::Hand)
        || choose.additional_zones.as_slice() != [Zone::Graveyard]
        || choose.is_search
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
        || choose.replace_tagged_objects
        || choose.remember_as_chosen_object
        || choose.filter.owner != Some(PlayerFilter::You)
    {
        return None;
    }
    let moved = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if moved.zone != Zone::Battlefield
        || moved.to_top
        || moved.library_order.is_some()
        || moved.target_plural_surface
        || !matches!(moved.target.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
        || !matches!(moved.actor_surface, None | Some(PlayerFilter::You))
        || moved.destination_player_surface.is_some()
        || moved.destination_player_reference_surface.is_some()
        || moved.exiled_with_source_surface.is_some()
        || moved.battlefield_controller != crate::effects::BattlefieldController::Preserve
        || moved.controller_surface_explicit
        || moved.enters_tapped
        || moved.enters_attacking
        || moved.attack_target_mode.is_some()
        || moved.enters_face_down
        || moved.transfer_exiled_with_source_links
    {
        return None;
    }
    let [counter] = moved.enters_with_counters.as_slice() else {
        return None;
    };
    let Value::Fixed(amount) = counter.amount.unhinted() else {
        return None;
    };
    if *amount <= 0
        || counter.condition.is_some()
        || counter.object_filter.is_some()
        || counter.surface != ironsmith_core::BattlefieldEntryCounterSurface::Inline
    {
        return None;
    }

    let mut display_filter = choose.filter.clone();
    display_filter.zone = None;
    display_filter.owner = None;
    let exact_land_count_bound = if let Some(crate::filter::Comparison::LessThanOrEqualExpr(limit)) =
        display_filter.mana_value.as_ref()
        && matches!(
            limit.unhinted(),
            Value::Count(filter)
                if filter.zone == Some(Zone::Battlefield)
                    && filter.controller == Some(PlayerFilter::You)
                    && filter.card_types == [CardType::Land]
        ) {
        let mut noun_filter = display_filter.clone();
        noun_filter.mana_value = None;
        noun_filter.description() == "creature card"
    } else {
        false
    };
    let object = if exact_land_count_bound {
        "a creature card with mana value less than or equal to the number of lands you control"
            .to_string()
    } else {
        with_indefinite_article(&display_filter.description())
    };
    let count = ironsmith_core::cardinal_word(*amount as u32).unwrap_or_else(|| amount.to_string());
    let counter_name = counter.counter_type.description();
    let counter_noun = if *amount == 1 { "counter" } else { "counters" };
    let prefix = if optional { "You may put" } else { "Put" };
    Some(format!(
        "{prefix} {object} onto the battlefield from your hand or graveyard with {count} {counter_name} {counter_noun} on it"
    ))
}

/// Render a looked-card battlefield choice whose optionality was authored as
/// a separate `MayEffect`. Keeping the exact-one inner choice distinguishes
/// "You may put one" from an ordinary bare "put up to one" selection.
fn describe_optional_looked_entry_with_counter_and_remainder(effects: &[Effect]) -> Option<String> {
    let (look_effect, choose_effect, move_each_effect, remainder_effect, wrapped_in_may) =
        match effects {
            [look_effect, may_effect, remainder_effect] => {
                let may = structural_unwrap_render_wrappers(may_effect)
                    .downcast_ref::<crate::effects::MayEffect>()?;
                if !matches!(may.decider.as_ref(), None | Some(PlayerFilter::You))
                    || may.fallback != crate::decision::FallbackStrategy::Decline
                {
                    return None;
                }
                let [choose_effect, move_each_effect] = may.effects.as_slice() else {
                    return None;
                };
                (
                    look_effect,
                    choose_effect,
                    move_each_effect,
                    remainder_effect,
                    true,
                )
            }
            [
                look_effect,
                choose_effect,
                move_each_effect,
                remainder_effect,
            ] => (
                look_effect,
                choose_effect,
                move_each_effect,
                remainder_effect,
                false,
            ),
            _ => return None,
        };
    let look = structural_unwrap_render_wrappers(look_effect)
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    if look.reveal || look.player != PlayerFilter::You {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let exact_optional_count = if wrapped_in_may {
        choose.count.is_single()
    } else {
        choose.count == ChoiceCount::up_to(1)
    };
    if choose.chooser != PlayerFilter::You
        || !exact_optional_count
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || choose.zone != Some(Zone::Library)
        || !choose.additional_zones.is_empty()
        || choose.is_search
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
        || choose.replace_tagged_objects
        || choose.remember_as_chosen_object
        || choose.filter.tagged_constraints.len() != 1
        || !choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == look.tag
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
    {
        return None;
    }
    let move_each = structural_unwrap_render_wrappers(move_each_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if move_each.tag != choose.tag || move_each.controller_at_last_blocked_by.is_some() {
        return None;
    }
    let (move_effect, separate_counter) = match move_each.effects.as_slice() {
        [move_effect] => (move_effect, None),
        [move_effect, counter_effect] => {
            let put = structural_unwrap_render_wrappers(counter_effect)
                .downcast_ref::<crate::effects::PutCountersEffect>()?;
            if !matches!(put.target.base(), ChooseSpec::Iterated)
                || put.target_count.is_some()
                || put.distributed
                || (!put.amount.has_surface_hint(
                    ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter,
                ) && !(!wrapped_in_may
                    && put.counter_type == CounterType::Shield
                    && put.amount.unhinted() == &Value::Fixed(1)))
            {
                return None;
            }
            (move_effect, Some((put.counter_type, &put.amount)))
        }
        _ => return None,
    };
    let moved = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if moved.zone != Zone::Battlefield
        || moved.to_top
        || moved.library_order.is_some()
        || moved.target_plural_surface
        || !matches!(moved.target.base(), ChooseSpec::Iterated)
        || moved.destination_player_surface.is_some()
        || moved.destination_player_reference_surface.is_some()
        || moved.exiled_with_source_surface.is_some()
        || moved.battlefield_controller != crate::effects::BattlefieldController::Preserve
        || moved.controller_surface_explicit
        || moved.enters_tapped
        || moved.enters_attacking
        || moved.attack_target_mode.is_some()
        || moved.enters_face_down
        || moved.transfer_exiled_with_source_links
    {
        return None;
    }
    let (counter_type, counter_amount) = if let Some(counter) = separate_counter {
        if !moved.enters_with_counters.is_empty() {
            return None;
        }
        counter
    } else {
        let [counter] = moved.enters_with_counters.as_slice() else {
            return None;
        };
        if counter.condition.is_some()
            || counter.object_filter.is_some()
            || counter.surface != ironsmith_core::BattlefieldEntryCounterSurface::Inline
        {
            return None;
        }
        (counter.counter_type, &counter.amount)
    };
    let remainder = structural_unwrap_render_wrappers(remainder_effect)
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if remainder.tag != look.tag
        || remainder.keep_tagged.as_ref() != Some(&choose.tag)
        || remainder.player != PlayerFilter::You
        || remainder.order != crate::effects::consult_helpers::LibraryBottomOrder::Random
    {
        return None;
    }

    let (count, noun, singular) = describe_look_count_and_noun(&look.count);
    if singular {
        return None;
    }
    let selection = describe_looked_battlefield_selection(choose)?;
    let counter = describe_put_counter_phrase(counter_amount, counter_type);
    Some(format!(
        "Look at the top {count} {noun} of your library. You may put {selection} from among them onto the battlefield with {counter} on it. Put the rest on the bottom of your library in a random order"
    ))
}

#[cfg(test)]
mod optional_looked_entry_with_counter_tests {
    use super::*;

    fn effects(counter_target: ChooseSpec) -> Vec<Effect> {
        let looked = crate::TagKey::from("looked");
        let selected = crate::TagKey::from("selected");
        let mut filter = ObjectFilter::permanent_card()
            .with_mana_value(crate::filter::Comparison::LessThanOrEqual(3))
            .in_zone(Zone::Library);
        filter
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: looked.clone(),
                relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            });
        let choose = crate::effects::ChooseObjectsEffect::new(
            filter,
            ChoiceCount::exactly(1),
            PlayerFilter::You,
            selected.clone(),
        )
        .in_zone(Zone::Library);
        let move_selected = Effect::new(crate::effects::ForEachTaggedEffect::new(
            selected.clone(),
            vec![
                Effect::new(crate::effects::MoveToZoneEffect::new(
                    ChooseSpec::Iterated,
                    Zone::Battlefield,
                    false,
                )),
                Effect::new(crate::effects::PutCountersEffect::new(
                    CounterType::Shield,
                    Value::Fixed(1).with_surface_hint(
                        ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter,
                    ),
                    counter_target,
                )),
            ],
        ));

        vec![
            Effect::new(crate::effects::LookAtTopCardsEffect::new(
                PlayerFilter::You,
                Value::Fixed(7),
                looked.clone(),
            )),
            Effect::new(crate::effects::MayEffect::new(vec![
                Effect::new(choose),
                move_selected,
            ])),
            Effect::new(
                crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
                    looked,
                    Some(selected),
                    crate::effects::consult_helpers::LibraryBottomOrder::Random,
                    PlayerFilter::You,
                ),
            ),
        ]
    }

    #[test]
    fn exact_optional_selection_keeps_may_and_inline_entry_counter_surface() {
        let effects = effects(ChooseSpec::Iterated);
        assert_eq!(
            describe_optional_looked_entry_with_counter_and_remainder(&effects).as_deref(),
            Some(
                "Look at the top seven cards of your library. You may put a permanent card with mana value 3 or less from among them onto the battlefield with a shield counter on it. Put the rest on the bottom of your library in a random order"
            )
        );
    }

    #[test]
    fn direct_up_to_one_selection_keeps_authored_may_surface() {
        let mut effects = effects(ChooseSpec::Iterated);
        let may_effect = effects.remove(1);
        let may = may_effect
            .downcast_ref::<crate::effects::MayEffect>()
            .expect("may wrapper");
        let [choose_effect, move_each_effect] = may.effects.as_slice() else {
            panic!("expected exact choice and move pair");
        };
        let mut choose = choose_effect
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            .expect("choose objects")
            .clone();
        choose.count = ChoiceCount::up_to(1);
        effects.insert(1, Effect::new(choose));
        effects.insert(2, move_each_effect.clone());

        assert_eq!(
            describe_optional_looked_entry_with_counter_and_remainder(&effects).as_deref(),
            Some(
                "Look at the top seven cards of your library. You may put a permanent card with mana value 3 or less from among them onto the battlefield with a shield counter on it. Put the rest on the bottom of your library in a random order"
            )
        );
    }

    #[test]
    fn counter_on_a_different_object_does_not_claim_the_compact_surface() {
        assert!(
            describe_optional_looked_entry_with_counter_and_remainder(&effects(ChooseSpec::Source))
                .is_none()
        );
    }
}

fn describe_destroyed_set_controller_life_by_mana_value(effects: &[Effect]) -> Option<String> {
    let [destroy_effect, reward_effect] = effects else {
        return None;
    };
    let destroyed_tag = effect_outer_tag(destroy_effect)?;
    let destroy = structural_unwrap_render_wrappers(destroy_effect)
        .downcast_ref::<crate::effects::DestroyNoRegenerationEffect>()?;
    let ChooseSpec::All(filter) = destroy.spec.base() else {
        return None;
    };
    if simple_filter_plural_noun(filter)?.as_str() != "artifacts" {
        return None;
    }
    let for_each = structural_unwrap_render_wrappers(reward_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if &for_each.tag != destroyed_tag || for_each.controller_at_last_blocked_by.is_some() {
        return None;
    }
    let [gain_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let gain = structural_unwrap_render_wrappers(gain_effect)
        .downcast_ref::<crate::effects::GainLifeEffect>()?;
    if !matches!(
        gain.player.base(),
        ChooseSpec::Player(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag)))
            if tag.as_str() == "__it__"
    ) || !matches!(
        gain.amount.unhinted(),
        Value::ManaValueOf(spec)
            if matches!(spec.base(), ChooseSpec::Iterated)
                || matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == "__it__")
    ) {
        return None;
    }

    Some(
        "Destroy all artifacts. They can't be regenerated. The controller of each of those artifacts gains life equal to its mana value"
            .to_string(),
    )
}

/// Keep an inline fixed counter plus a typed counter-kind choice on their one
/// shared target. A resolving `ChooseModeEffect` (explicit chooser) proves
/// this is an instruction-level choice rather than a printed modal ability.
pub(in crate::compiled_text) fn describe_fixed_counter_and_counter_choice_same_target(
    effects: &[Effect],
) -> Option<String> {
    let [fixed_effect, choice_effect] = effects else {
        return None;
    };
    let fixed = structural_unwrap_render_wrappers(fixed_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if fixed.amount.unhinted() != &Value::Fixed(1)
        || fixed.target_count.is_some()
        || fixed.distributed
    {
        return None;
    }

    let choice = structural_unwrap_render_wrappers(choice_effect)
        .downcast_ref::<crate::effects::ChooseModeEffect>()?;
    if choice.chooser != Some(PlayerFilter::You)
        || choice.min != Value::Fixed(1)
        || choice.max != Value::Fixed(1)
        || choice.choose_count != Value::Fixed(1)
        || choice.min_choose_count != Value::Fixed(1)
        || choice.allow_repeat
        || choice.random
        || choice.allow_repeated_modes
        || choice.spree
        || choice.tiered
        || !choice.common_prefix_effects.is_empty()
        || choice.common_suffix_effect_count != 0
        || !choice.mode_additional_mana_costs.is_empty()
        || choice.mode_point_costs.iter().any(|cost| *cost != 1)
        || choice.disallow_previously_chosen_modes
        || choice.disallow_previously_chosen_modes_this_turn
        || choice.distinct_player_targets_per_mode
        || choice.conditional_mode_range.is_some()
        || choice.modes.len() < 2
    {
        return None;
    }

    let mut names = Vec::with_capacity(choice.modes.len());
    for mode in &choice.modes {
        let [effect] = mode.effects.as_slice() else {
            return None;
        };
        let put = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::PutCountersEffect>()?;
        if put.amount.unhinted() != &Value::Fixed(1)
            || put.target != fixed.target
            || put.target_count.is_some()
            || put.distributed
        {
            return None;
        }
        names.push(put.counter_type.description().into_owned());
    }

    Some(format!(
        "Put {} and a counter from among {} on {}",
        describe_put_counter_phrase(&fixed.amount, fixed.counter_type),
        join_with_or(&names),
        describe_choose_spec(&fixed.target),
    ))
}

#[cfg(test)]
mod fixed_and_chosen_counter_tests {
    use super::*;

    fn choice(target: ChooseSpec) -> Effect {
        let modes = [
            CounterType::Flying,
            CounterType::FirstStrike,
            CounterType::Lifelink,
            CounterType::Vigilance,
        ]
        .into_iter()
        .map(|counter_type| crate::effect::EffectMode {
            source_text: format!("Put a {} counter on it", counter_type.description()),
            effects: vec![Effect::put_counters(counter_type, 1, target.clone())],
        })
        .collect();
        Effect::new(
            crate::effects::ChooseModeEffect::choose_one(modes).with_chooser(PlayerFilter::You),
        )
    }

    #[test]
    fn fixed_counter_and_typed_choice_keep_one_shared_target() {
        let target = ChooseSpec::tagged("targeted_0");
        let effects = vec![
            Effect::put_counters(CounterType::PlusOnePlusOne, 1, target.clone()),
            choice(target),
        ];
        assert_eq!(
            describe_fixed_counter_and_counter_choice_same_target(&effects),
            Some(
                "Put a +1/+1 counter and a counter from among flying, first strike, lifelink, or vigilance on it"
                    .to_string()
            )
        );
    }

    #[test]
    fn coordinated_fixed_counter_and_typed_choice_uses_the_same_compactor() {
        let target = ChooseSpec::tagged("targeted_0");
        let effect = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::put_counters(CounterType::PlusOnePlusOne, 1, target.clone()),
            choice(target),
        ]));
        assert_eq!(
            describe_effect(&effect),
            "Put a +1/+1 counter and a counter from among flying, first strike, lifelink, or vigilance on it"
        );
    }

    #[test]
    fn changed_choice_target_does_not_merge() {
        let effects = vec![
            Effect::put_counters(
                CounterType::PlusOnePlusOne,
                1,
                ChooseSpec::tagged("targeted_0"),
            ),
            choice(ChooseSpec::tagged("targeted_1")),
        ];
        assert_eq!(
            describe_fixed_counter_and_counter_choice_same_target(&effects),
            None
        );
    }
}

pub(crate) fn describe_effect_list(effects: &[Effect]) -> String {
    if let Some(compact) = describe_exile_all_from_same_target_players_hand_and_graveyard(effects) {
        return compact;
    }
    if let Some(compact) = describe_delegated_subset_with_hand_remainder(effects) {
        return compact;
    }
    if let Some(compact) = describe_declared_pool_then_delegated_partition_conditional(effects) {
        return compact;
    }
    if let Some(compact) = describe_delegated_collection_partition_moves(effects) {
        return compact;
    }
    if let Some(compact) = describe_delegated_subset_choice(effects) {
        return compact;
    }
    if let [effect] = effects
        && let Some(compact) = describe_delegated_collection_complement_move(effect)
    {
        return compact;
    }
    if let Some((compact, consumed)) = describe_selected_opponent_chosen_action(effects)
        && consumed == effects.len()
    {
        return compact;
    }
    if let [you_draw, controllers_draw] = effects
        && you_draw
            .downcast_ref::<crate::effects::DrawCardsEffect>()
            .is_some_and(|draw| draw.count == Value::Fixed(1) && draw.player == PlayerFilter::You)
        && let Some(for_each) =
            controllers_draw.downcast_ref::<crate::effects::ForEachControllerOfTaggedEffect>()
        && for_each.tag.as_str() == ironsmith_core::COMBAT_DAMAGE_GROUP_TAG
        && matches!(
            for_each.effects.as_slice(),
            [draw] if draw
                .downcast_ref::<crate::effects::DrawCardsEffect>()
                .is_some_and(|draw| {
                    draw.count == Value::Fixed(1)
                        && draw.player == PlayerFilter::IteratedPlayer
                })
        )
    {
        return "You and the controller of those creatures each draw a card".to_string();
    }
    let effect_refs = effects.iter().collect::<Vec<_>>();
    if let Some(compact) = describe_put_counters_then_goad(&effect_refs) {
        return compact;
    }
    if let Some(compact) = describe_target_must_be_blocked_same_tag(effects) {
        return compact;
    }
    if let Some(compact) = describe_optional_looked_entry_with_counter_and_remainder(effects) {
        return compact;
    }
    if let Some(compact) = describe_fixed_counter_and_counter_choice_same_target(effects) {
        return compact;
    }
    if let Some(compact) = describe_optional_target_player_mill(effects) {
        return compact;
    }
    if let Some(compact) = describe_owned_hand_or_graveyard_entry_with_counters(effects) {
        return compact;
    }
    if let Some(compact) = describe_destroyed_set_controller_life_by_mana_value(effects) {
        return compact;
    }
    if let Some(compact) = describe_source_exiled_return_partition(effects) {
        return compact;
    }
    if let Some(compact) = describe_each_opponent_optional_sacrifice_or_discard_then_damage(effects)
    {
        return compact;
    }
    if let Some(compact) = describe_target_mill_then_may_cast_from_exact_milled_set(effects) {
        return compact;
    }
    if let Some(compact) = describe_hybrid_named_vote_per_vote_sequence(effects) {
        return compact;
    }
    if let [target, grant, pump] = effects
        && let Some(compact) =
            describe_shared_declared_target_grant_then_pt_pump(target, grant, pump)
    {
        return compact;
    }
    if let [target, first, second] = effects
        && let Some(compact) = describe_shared_target_trample_mana_value_pump(target, first, second)
    {
        return compact;
    }
    if let Some(compact) = describe_tempting_offer_copy_spell_bundle(effects) {
        return compact;
    }
    if let Some(compact) = describe_historical_block_reanimation(effects) {
        return compact;
    }
    if let Some((prefix, consumed)) = describe_target_relative_combat_set_prefix(effects) {
        if consumed == effects.len() {
            return prefix;
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        );
    }
    // A source-sentence connective is explicit typed sequence provenance.
    // Honor it before structural compactors inspect or flatten the wrapped
    // children, or the authored leading "Then" would disappear again.
    if let [effect] = effects
        && effect
            .downcast_ref::<crate::effects::SequenceEffect>()
            .is_some_and(|sequence| {
                sequence.surface == ironsmith_core::SequenceSurface::SentenceLeadingThen
            })
    {
        return describe_effect(effect);
    }
    if let Some(compact) = describe_repeated_explore_pair(effects) {
        return compact;
    }
    if let Some(compact) = describe_each_player_reveal_permanents_and_rest(effects) {
        return compact;
    }
    if let Some(compact) = describe_condition_collection_choice_gain_control_then_untap(effects) {
        return compact;
    }
    if let Some(compact) = describe_copy_spell_with_characteristic_modifiers(effects) {
        return compact;
    }
    if let Some((prefix, consumed)) = describe_explicit_target_then_coin_flip(effects) {
        if consumed == effects.len() {
            return prefix;
        }
        let suffix = describe_effect_list(&effects[consumed..]);
        return format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        );
    }
    let direct_refs = effects.iter().collect::<Vec<_>>();
    if let [draw_effect, conditional_effect] = effects
        && let Some(draw) = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()
        && draw.player == PlayerFilter::You
        && let Some(conditional) =
            conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()
        && conditional.condition == Condition::YourTurn
        && conditional.if_false.is_empty()
    {
        let draw_text = capitalize_first(describe_effect(draw_effect).trim_end_matches('.'));
        let conditional_text = describe_effect(conditional_effect);
        return format!("{draw_text}. {conditional_text}");
    }
    if let [exile_effect, mutation_effect] = effects
        && let Some(exiled_tag) = tagged_exile_any_number_target_creatures(exile_effect)
        && let Some(for_each) =
            mutation_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()
        && for_each.tag.as_str() == exiled_tag
        && consult_reveal_put_battlefield_then_bottom_selection(for_each).is_some_and(|selection| {
            matches!(
                selection.as_str(),
                "creature" | "creature card" | "creature card in library"
            )
        })
    {
        return "Exile any number of target creatures controlled by different players. For each creature exiled this way, its controller reveals cards from the top of their library until they reveal a creature card, puts that card onto the battlefield, then puts the rest on the bottom of their library in a random order".to_string();
    }
    // Exact multi-effect procedures must run before broad producer/consumer
    // and per-effect rendering. Once those generic paths consume a prefix,
    // shared player, searched-card, and destination references are lost.
    if let Some(compact) = describe_search_two_split_hand_graveyard_sequence(&direct_refs) {
        return compact;
    }
    if let Some(compact) = describe_look_at_top_choose_battlefield_rest_graveyard(effects) {
        return compact;
    }
    if let Some(compact) = describe_reveal_hand_then_gain_for_that_players_hand(&direct_refs) {
        return compact;
    }
    if let Some(compact) = describe_target_player_look_top_may_move_that_card(&direct_refs) {
        return compact;
    }
    if let [target_effect, reveal_effect] = effects
        && let Some(compact) = describe_target_player_reveal_top(target_effect, reveal_effect)
    {
        return compact;
    }
    if let Some(compact) = describe_choose_x_permanents_create_x_copies(&direct_refs) {
        return compact;
    }
    if let [choose_effect, for_each_effect] = effects
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
    {
        if let Some(for_each) =
            for_each_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some(compact) = describe_choose_then_for_each_copy(choose, for_each)
        {
            return compact;
        }
        if let Some(for_each) = for_each_effect.downcast_ref::<crate::effects::ForEachObject>()
            && let Some(compact) = describe_choose_then_for_each_object_copy(choose, for_each)
        {
            return compact;
        }
    }
    if let Some(compact) = describe_tagged_blocked_set_tap_then_next_untap(effects) {
        return compact;
    }
    // Preserve exact producer IDs before broad discard/draw compaction turns
    // the dependency into the looser "that many cards" surface.
    if effects.first().is_some_and(|effect| {
        effect
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
            .is_some()
    }) && let Some(compact) = describe_id_backed_prior_action_count_consumer(effects)
    {
        return compact;
    }
    // Preserve a direct quantified discard actor before generic rendering
    // flattens `NotYou` to the singular description "a player other than
    // you". The paired outcome metric proves this is the all-participants
    // instruction represented by the producer.
    if let Some(compact) = describe_discard_then_draw_for_discarded(effects) {
        return compact;
    }
    if let [first, second] = effects
        && structural_unwrap_render_wrappers(first)
            .downcast_ref::<crate::effects::ForPlayersEffect>()
            .is_some_and(|for_players| for_players.filter == PlayerFilter::Opponent)
        && structural_unwrap_render_wrappers(second)
            .downcast_ref::<crate::effects::GainLifeEffect>()
            .is_some_and(|gain| gain.player == ChooseSpec::Player(PlayerFilter::You))
        && let Some(compact) = describe_joint_subject_pair(first, second)
    {
        return compact;
    }
    // "You and target opponent each create ..." — a synthetic target
    // declaration followed by the you/target halves of a joint action. The
    // declaration's target must be the one the second half acts for, so the
    // compact surface still names it.
    if let [declaration, first, second] = effects
        && let Some(target_only) = structural_unwrap_render_wrappers(declaration)
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
        && !target_only.explicit_declaration
        && unwrap_basic_tag_wrappers(second)
            .downcast_ref::<crate::effects::CreateTokenEffect>()
            .is_some_and(|create| create.controller_target.as_ref() == Some(&target_only.target))
        && let Some(compact) = describe_joint_subject_pair(first, second)
    {
        return compact;
    }
    // This exact producer/consumer triple is a single authored library
    // procedure. Recognize it before broad list renderers can consume the
    // consult, move, and remainder independently and lose their shared tags.
    if let Some(compact) = render_consult_reveal_put_hand_then_bottom(&direct_refs) {
        return compact;
    }
    if let Some(compact) = describe_relative_player_target_then_optional_consult(effects) {
        return compact;
    }
    if let Some(compact) = describe_mixed_target_collection_consult_damage(effects) {
        return compact;
    }
    if let Some(compact) = describe_relative_player_target_then_optional_search(effects) {
        return compact;
    }
    if let Some(compact) = describe_optional_gated_consult_partition(effects) {
        return compact;
    }
    if let Some(compact) = describe_land_or_nonland_chosen_kind_consult(effects) {
        return compact;
    }
    if let [consult, move_matches, bottom] = effects
        && let Some(compact) =
            describe_counted_consult_matches_to_graveyard_then_bottom(consult, move_matches, bottom)
    {
        return compact;
    }
    if let Some(compact) = describe_same_name_three_zone_extraction(effects) {
        return compact;
    }
    // Hidden-pile cloak/manifest programs carry enough typed tag and
    // face-down provenance to render as one authored procedure. Run this
    // exact structural matcher before broader list patterns can consume its
    // exile prefix and strand the manifest consumer as an unsupported tail.
    if let Some(compact) = describe_face_down_pile_then_manifest(effects) {
        return compact;
    }
    if let Some(compact) = describe_choose_exiled_card_then_play_without_paying(effects) {
        return compact;
    }
    let consult_refs = effects.iter().collect::<Vec<_>>();
    if let Some(compact) = describe_consult_match_destination_alternative(&consult_refs) {
        return compact;
    }
    if let Some(compact) = describe_consult_conditional_destination_remainder(&consult_refs) {
        return compact;
    }
    if let Some(compact) = describe_consult_conditional_may_cast_remainder_bottom(&consult_refs) {
        return compact;
    }
    if let Some(compact) = describe_target_player_source_control_transfer(effects) {
        return compact;
    }
    if let Some(compact) = describe_declared_target_player_draw_fanout(effects) {
        return compact;
    }
    if let Some(compact) = describe_declared_target_joint_draw(effects) {
        return compact;
    }
    if let [sequence_effect] = effects
        && let Some(sequence) = structural_unwrap_render_wrappers(sequence_effect)
            .downcast_ref::<crate::effects::SequenceEffect>()
        && let Some((compact, consumed)) =
            describe_repeated_targets_then_chosen_controller_partition(&sequence.effects)
        && consumed == sequence.effects.len()
    {
        return compact;
    }
    if let Some((prefix, consumed)) =
        describe_repeated_targets_then_chosen_controller_partition(effects)
    {
        if consumed == effects.len() {
            return prefix;
        }
        let suffix = describe_effect_list(&effects[consumed..]);
        return format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        );
    }
    if let Some((prefix, consumed)) = describe_participant_choose_then_untap_chosen(effects) {
        if consumed == effects.len() {
            return prefix;
        }
        let suffix = describe_effect_list(&effects[consumed..]);
        return format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        );
    }
    if let [sequence_effect] = effects
        && let Some(sequence) = structural_unwrap_render_wrappers(sequence_effect)
            .downcast_ref::<crate::effects::SequenceEffect>()
        && let Some((compact, consumed)) =
            describe_opponent_choose_then_return_chosen(&sequence.effects)
        && consumed == sequence.effects.len()
    {
        return compact;
    }
    if let Some((prefix, consumed)) = describe_opponent_choose_then_return_chosen(effects) {
        if consumed == effects.len() {
            return prefix;
        }
        let suffix = describe_effect_list(&effects[consumed..]);
        return format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        );
    }
    if let [sequence_effect] = effects
        && let Some(sequence) = structural_unwrap_render_wrappers(sequence_effect)
            .downcast_ref::<crate::effects::SequenceEffect>()
        && let Some((compact, consumed)) = describe_attributed_target_choice_pair(&sequence.effects)
        && consumed == sequence.effects.len()
    {
        return compact;
    }
    if let Some((prefix, consumed)) = describe_attributed_target_choice_pair(effects) {
        if consumed == effects.len() {
            return prefix;
        }
        let suffix = describe_effect_list(&effects[consumed..]);
        return format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        );
    }
    if let [sequence_effect] = effects
        && let Some(sequence) = structural_unwrap_render_wrappers(sequence_effect)
            .downcast_ref::<crate::effects::SequenceEffect>()
        && let Some((compact, consumed)) =
            describe_primary_then_opponent_chosen_same_action(&sequence.effects)
        && consumed == sequence.effects.len()
    {
        return compact;
    }
    if let Some((prefix, consumed)) = describe_primary_then_opponent_chosen_same_action(effects) {
        if consumed == effects.len() {
            return prefix;
        }
        let suffix = describe_effect_list(&effects[consumed..]);
        return format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        );
    }
    if let [sequence_effect] = effects
        && let Some(sequence) = structural_unwrap_render_wrappers(sequence_effect)
            .downcast_ref::<crate::effects::SequenceEffect>()
        && let Some((compact, consumed)) =
            describe_opponent_chosen_target_destroy_pair(&sequence.effects)
        && consumed == sequence.effects.len()
    {
        return compact;
    }
    if let Some((prefix, consumed)) = describe_opponent_chosen_target_destroy_pair(effects) {
        if consumed == effects.len() {
            return prefix;
        }
        let suffix = describe_effect_list(&effects[consumed..]);
        return format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        );
    }
    if let Some(compact) = describe_consult_exile_may_cast_else_your_hand(effects) {
        return compact;
    }
    if let Some(compact) = describe_sequence_wrapped_search_two_split(effects) {
        return compact;
    }
    if let Some(compact) = describe_hand_pipeline_then_leading_conditional(effects) {
        return compact;
    }
    if let Some((prefix, consumed)) = describe_hand_pipeline_prefix(effects) {
        if consumed == effects.len() {
            return prefix;
        }
        let suffix = describe_effect_list(&effects[consumed..]);
        return format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        );
    }
    if let Some(compact) = describe_sequence_wrapped_hand_pipeline(effects) {
        return compact;
    }
    if let Some((prefix, consumed)) = describe_untap_then_phase_out_until_source_leaves(effects) {
        if consumed == effects.len() {
            return prefix;
        }
        let suffix = describe_effect_list(&effects[consumed..]);
        return format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        );
    }
    if let Some((prefix, consumed)) = describe_damage_then_gain_life_this_way(effects) {
        if consumed == effects.len() {
            return prefix;
        }
        let suffix = describe_effect_list(&effects[consumed..]);
        return format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        );
    }
    if let Some(compact) = describe_each_player_reveal_filtered_token_then_pump_then_draw(effects) {
        return compact;
    }
    if let Some(compact) =
        describe_consult_reveal_triggering_creature_pump_then_move_revealed(effects)
    {
        return compact;
    }
    if let Some(compact) = describe_targeted_named_vote_conditional_sequence(effects) {
        return compact;
    }
    let leading_refs = effects.iter().collect::<Vec<_>>();
    if let Some(compact) = describe_council_vote_winners_exile(&leading_refs) {
        return compact;
    }
    if let Some(compact) = describe_attach_all_enchanting_target_to_same_controller(effects) {
        return compact;
    }
    if let Some(compact) = describe_targeted_attachment_instruction(effects) {
        return compact;
    }
    if effects.len() >= 2
        && target_only_pair_can_fold(effects, &effects[0])
        && let Some(prefix) = describe_redundant_target_only_pair(&effects[..2])
    {
        if effects.len() == 2 {
            return prefix;
        }
        let suffix = describe_effect_list(&effects[2..]);
        return format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        );
    }
    if let Some(compact) = describe_linked_counter_followup(effects) {
        return compact;
    }
    if let Some(compact) = describe_typed_counter_sentence_split(effects) {
        return compact;
    }
    if let Some(compact) = describe_optional_search_battlefield_partition_effects(effects) {
        return compact;
    }
    if let Some(compact) = describe_discard_redraw_mana_value_ladder(effects) {
        return compact;
    }
    if let Some(compact) = describe_look_hand_optional_exile_persistent_play_tax(effects) {
        return compact;
    }
    if let Some(compact) = describe_target_exile_persistent_owner_play_tax(effects) {
        return compact;
    }
    if let Some(compact) = describe_hidden_exile_partition_with_persistent_permission(effects) {
        return compact;
    }
    if let Some(compact) = describe_each_opponent_top_card_hidden_exile_permission(effects) {
        return compact;
    }
    if let Some(compact) = describe_exile_all_then_each_player_may_deploy_and_return_exiled(effects)
    {
        return compact;
    }
    if let Some(compact) = describe_exile_two_creatures_then_controller_consults(effects) {
        return compact;
    }
    if let Some(compact) = describe_exile_top_play_then_additional_land(effects) {
        return compact;
    }
    if let Some(compact) = describe_exile_top_choose_one_play_next_turn(effects) {
        return compact;
    }
    if let Some(compact) = describe_each_player_reveal_set_may_move_else_draw(effects) {
        return compact;
    }
    if let Some(compact) = describe_consult_characteristic_boost_then_all_revealed_bottom(effects) {
        return compact;
    }
    if let Some(compact) = describe_consult_reflexive_damage_then_all_revealed_bottom(effects) {
        return compact;
    }
    if let Some(compact) = describe_energy_payment_failure_fallback(effects) {
        return compact;
    }
    if let Some(compact) = describe_draw_then_additional_draw(effects) {
        return compact;
    }
    if let [first, second] = effects
        && let Some(compact) = describe_action_and_get_energy_pair(first, second)
    {
        return compact;
    }
    if let Some(compact) = describe_milled_creatures_returned_then_animated(effects) {
        return compact;
    }
    if let Some(compact) = describe_returned_object_set_to_enchantment(effects) {
        return compact;
    }
    if let Some(compact) = describe_returned_object_exact_types_with_quoted_ability(effects) {
        return compact;
    }
    if let Some(compact) = describe_returned_battlefield_object_then_animated(effects) {
        return compact;
    }
    if let Some(compact) = describe_bulk_battlefield_move_then_grant_decayed(effects) {
        return compact;
    }
    let same_name_refs = effects.iter().collect::<Vec<_>>();
    if let Some(compact) = describe_choose_name_reveal_hand_discard_named_bundle(&same_name_refs) {
        return compact;
    }
    if let Some(compact) = describe_same_name_reference_search_bundle(&same_name_refs) {
        return compact;
    }
    if let Some((compact, consumed)) = describe_linked_target_set_followup_prefix(effects)
        .or_else(|| describe_same_name_exile_then_investigate_prefix(effects))
        .or_else(|| describe_target_same_name_action_fanout_prefix(effects))
    {
        if consumed == effects.len() {
            return compact;
        }
        let suffix = describe_effect_list(&effects[consumed..]);
        return format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        );
    }
    // Exact typed multi-step bundles must run before the generic synthetic
    // target folds below. Those folds intentionally erase declarations that
    // have multiple consumers; doing that first loses the shared opponent and
    // revealed-card provenance needed by this life-payment sequence.
    if let Some(compact) = describe_pay_life_reveal_hand_choose_exile_effects(effects) {
        return compact;
    }
    // This public reveal/choice partition may contain a lowering-only player
    // target used as the chooser. Preserve the complete typed bundle before
    // the generic one-consumer target fold erases that declaration and leaves
    // the tagged-card move visible.
    let raw_effects = effects.iter().collect::<Vec<_>>();
    if let Some((compact, consumed)) = describe_revealed_top_choose_one_graveyard(&raw_effects) {
        if consumed == effects.len() {
            return compact;
        }
        let suffix = describe_effect_list(&effects[consumed..]);
        return format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        );
    }
    if let Some(compact) = describe_linked_exile_top_play_clause(effects) {
        return capitalize_first(&compact);
    }
    if let [first, second] = effects
        && let Some(compact) = describe_must_block_then_control_block_assignments(first, second)
    {
        return compact;
    }
    // A tagged target declaration followed by a strict condition on that
    // same target must retain its authored action-first surface. Run this
    // proof before the generic synthetic-target fold, which otherwise hides
    // the declaration and renders the conditional in leading-if order.
    if let Some(compact) = describe_tagged_target_then_conditional_action(effects) {
        return compact;
    }
    if let [target_effect, draw_effect, lose_effect] = effects
        && let Some(target_only) = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()
        && let Some(draw) = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()
        && let Some(lose) = lose_effect.downcast_ref::<crate::effects::LoseLifeEffect>()
        && let Some(compact) = describe_target_player_draw_then_lose_life(draw, target_only, lose)
    {
        return compact;
    }
    if let Some(compact) = describe_single_consumer_synthetic_target_fold(effects) {
        return compact;
    }
    if let Some(compact) = describe_multi_consumer_synthetic_target_declaration(effects) {
        return compact;
    }
    include!("effect_list/raw_patterns.rs");
    let preserve_target_only_players = effects.iter().any(|effect| {
        structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::ForPlayersEffect>()
            .is_some_and(|for_players| matches!(&for_players.filter, PlayerFilter::Target(_)))
    });
    let preserve_target_only_references = effects.iter().any(effect_references_target_player);
    let has_non_target_only = effects.iter().any(|effect| {
        effect
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
            .is_none()
    });
    let filtered = effects
        .iter()
        .filter(|effect| {
            if let Some(target_only) = structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                && !target_only.explicit_declaration
                && !(preserve_target_only_players
                    && choose_spec_is_player_choice(&target_only.target))
                && synthetic_target_has_single_consumer(effects, effect)
                && effects.iter().any(|candidate| {
                    !std::ptr::eq(*effect, candidate)
                        && action_consumes_implicit_target(candidate, &target_only.target)
                })
            {
                return false;
            }
            if structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some_and(|target_only| target_only.explicit_declaration)
            {
                return true;
            }
            if !(has_non_target_only
                && effect
                    .downcast_ref::<crate::effects::TargetOnlyEffect>()
                    .is_some())
            {
                return true;
            }

            if preserve_target_only_players
                && effect
                    .downcast_ref::<crate::effects::TargetOnlyEffect>()
                    .is_some_and(|target_only| {
                        matches!(
                            target_only.target,
                            ChooseSpec::Player(_) | ChooseSpec::WithCount(_, _)
                        )
                    })
            {
                return true;
            }

            if preserve_target_only_references
                && effect
                    .downcast_ref::<crate::effects::TargetOnlyEffect>()
                    .is_some_and(|target_only| choose_spec_is_player_choice(&target_only.target))
            {
                return true;
            }

            if effect
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some_and(|target_only| {
                    choose_spec_contains_hand_advantage_player_filter(&target_only.target)
                })
            {
                return true;
            }

            // A lowering-only declaration with multiple consumers carries
            // shared identity that cannot be reconstructed after filtering.
            // Keep it visible; zero-consumer bookkeeping retains the legacy
            // elision, while one-consumer declarations are handled above.
            if structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some()
                && synthetic_target_has_multiple_consumers(effects, effect)
            {
                return true;
            }

            false
        })
        .collect::<Vec<_>>();

    if let Some(compact) = describe_coordinated_controller_opponent_bundle(&filtered) {
        return compact;
    }

    include!("effect_list/filtered_patterns.rs");
    include!("effect_list/bundle_patterns.rs");
    if let [choose_effect, return_effect, counter_effect] = filtered.as_slice()
        && let Some(compact) = describe_choose_then_return_from_graveyard_with_counters(
            choose_effect,
            return_effect,
            counter_effect,
        )
    {
        return compact;
    }
    let mut parts = Vec::new();
    let mut idx = 0usize;
    while idx < filtered.len() {
        if idx + 2 < filtered.len()
            && let Some((compact, consumed)) =
                describe_selected_opponent_chosen_action(&filtered[idx..])
        {
            parts.push(compact);
            idx += consumed;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some((compact, consumed)) =
                describe_primary_then_opponent_chosen_same_action(&filtered[idx..])
        {
            parts.push(compact);
            idx += consumed;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) = describe_returned_battlefield_object_then_animated_pair(
                filtered[idx],
                filtered[idx + 1],
            )
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) =
                describe_opponent_chosen_target_action_join(filtered[idx], filtered[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) =
                describe_play_permission_then_free_cast_join(filtered[idx], filtered[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if let Some((compact, consumed)) =
            describe_leading_coordinated_graveyard_returns(&filtered[idx..])
        {
            parts.push(compact);
            idx += consumed;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) =
                describe_choose_then_return_from_graveyard(filtered[idx], filtered[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        include!("effect_list/loop_patterns_early.rs");
        include!("effect_list/loop_patterns_late.rs");
    }
    let text = parts
        .into_iter()
        .enumerate()
        .map(|(index, part)| {
            if index == 0 {
                if part.starts_with("target player ") {
                    capitalize_first(&part)
                } else {
                    part
                }
            } else {
                capitalize_first(&part)
            }
        })
        .collect::<Vec<_>>()
        .join(". ");
    if let Some(compact) = normalize_haunting_echoes_text(&text) {
        return compact;
    }
    cleanup_decompiled_text(&text)
}

pub(in crate::compiled_text) fn describe_bulk_battlefield_move_then_grant_decayed(
    effects: &[Effect],
) -> Option<String> {
    let [move_effect, grant_effect] = effects else {
        return None;
    };
    let moved_tag = effect_outer_tag(move_effect)?;
    let return_all = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()?;
    let apply = tagged_apply_continuous_effect(grant_effect)?;
    if !apply_continuous_is_forever_tagged(apply, moved_tag)
        || !apply_continuous_grants_decayed(apply)
    {
        return None;
    }

    Some(format!(
        "{}. They gain decayed",
        describe_return_all_to_battlefield_effect(return_all)
    ))
}

pub(in crate::compiled_text) fn describe_turn_start_hand_condition_effects(
    effects: &[Effect],
) -> Option<String> {
    let [first_effect, second_effect] = effects else {
        return None;
    };
    let first = first_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let second = second_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !first.if_false.is_empty()
        || !second.if_false.is_empty()
        || first.if_true.len() != 1
        || second.if_true.len() != 1
    {
        return None;
    }
    let Condition::PlayerCardsInHandAtTurnStartOrFewer {
        player: first_player,
        count: 0,
    } = &first.condition
    else {
        return None;
    };
    let Condition::PlayerCardsInHandAtTurnStartOrMore {
        player: second_player,
        count: 1,
    } = &second.condition
    else {
        return None;
    };
    if first_player != second_player {
        return None;
    }

    let first_text = lowercase_first(describe_effect(&first.if_true[0]).trim_end_matches('.'));
    let first_condition = lowercase_first(&describe_condition(&first.condition));
    let second_condition = lowercase_first(&describe_condition(&second.condition))
        .replace(" at the beginning of this turn", "");
    let second_text = lowercase_first(describe_effect(&second.if_true[0]).trim_end_matches('.'));
    Some(format!(
        "{first_text} if {first_condition}. If {second_condition}, {second_text}"
    ))
}

pub(super) fn describe_vote_with_received_vote_followups(effects: &[Effect]) -> Option<String> {
    let [first, rest @ ..] = effects else {
        return None;
    };
    first.downcast_ref::<crate::effects::VoteEffect>()?;
    if rest.is_empty() {
        return None;
    }
    let structurally_received_vote_followups = rest.iter().all(|effect| {
        if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() {
            return for_players.effects.len() == 1
                && for_players.effects[0]
                    .downcast_ref::<crate::effects::RepeatEffectsEffect>()
                    .is_some_and(|repeat| {
                        repeat.count == Value::PlayerVoteCount(PlayerFilter::IteratedPlayer)
                    });
        }
        effect
            .downcast_ref::<crate::effects::RepeatEffectsEffect>()
            .is_some_and(|repeat| matches!(repeat.count, Value::PlayerVoteCount(_)))
    });
    let rendered = effects
        .iter()
        .map(describe_effect)
        .filter(|text| !text.trim().is_empty())
        .collect::<Vec<_>>();
    let rendered_received_vote_followups = rendered
        .iter()
        .skip(1)
        .all(|text| text.starts_with("For each vote ") && text.contains(" received,"));
    if !structurally_received_vote_followups && !rendered_received_vote_followups {
        return None;
    }
    Some(rendered.join(". "))
}

pub(super) fn normalize_haunting_echoes_text(text: &str) -> Option<String> {
    let expected = concat!(
        "Exile all nonland cards in target player's graveyard or nonbasic cards in target player's graveyard. ",
        "For each card in exile, you search that player's library for any number with the same name as that object card. ",
        "For each tagged 'searched' object, Exile the tagged object 'searched'. ",
        "Target player shuffles"
    );
    if text == expected {
        return Some(
            "Exile all cards from target player's graveyard other than basic land cards. For each card exiled this way, search that player's library for all cards with the same name as that card and exile them. Then that player shuffles"
                .to_string(),
        );
    }
    None
}

pub(crate) fn describe_linked_graveyard_choices_then_may_return_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [first_choose_effect, second_choose_effect, may_effect] = filtered else {
        return None;
    };
    let first_choose = first_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let second_choose =
        second_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [move_effect] = may.effects.as_slice() else {
        return None;
    };
    let move_to_zone = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;

    if first_choose.is_search
        || second_choose.is_search
        || first_choose.replace_tagged_objects
        || second_choose.replace_tagged_objects
        || first_choose.tag != second_choose.tag
        || choose_exact_count(first_choose) != Some(1)
        || choose_exact_count(second_choose) != Some(1)
        || choose_primary_zone(first_choose) != Some(Zone::Graveyard)
        || choose_primary_zone(second_choose) != Some(Zone::Graveyard)
        || !matches!(
            &second_choose.chooser,
            PlayerFilter::AliasedOwnerOf(crate::filter::ObjectRef::Tagged(tag))
                | PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Tagged(tag))
                if tag == &first_choose.tag
        )
        || !move_to_battlefield_uses_chosen_tag(move_to_zone, first_choose.tag.as_str())
    {
        return None;
    }

    let describe_choose_clause = |choose: &crate::effects::ChooseObjectsEffect,
                                  capitalize_subject: bool| {
        let chooser = describe_player_filter(&choose.chooser);
        let chosen = describe_choose_selection(choose);
        let location = describe_choose_zone_location(choose, "graveyard");
        if chooser == "you" {
            return format!("Choose {chosen} {location}");
        }
        let subject = if capitalize_subject {
            capitalize_first(&chooser)
        } else {
            chooser.clone()
        };
        let choose_verb = player_verb(&chooser, "choose", "chooses");
        format!("{subject} {choose_verb} {chosen} {location}")
    };

    let first_clause = describe_choose_clause(first_choose, true);
    let second_clause = describe_choose_clause(second_choose, false);
    let tapped_suffix = if move_to_zone.enters_tapped {
        " tapped"
    } else {
        ""
    };
    let controller_suffix = match move_to_zone.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => " under their owners' control",
        crate::effects::BattlefieldController::You => " under your control",
    };
    let decider = may
        .decider
        .as_ref()
        .map(describe_player_filter)
        .unwrap_or_else(|| "you".to_string());
    let may_clause = if decider == "you" {
        format!("You may return those cards to the battlefield{tapped_suffix}{controller_suffix}")
    } else {
        let may_verb = player_verb(&decider, "may", "may");
        format!(
            "{} {may_verb} return those cards to the battlefield{tapped_suffix}{controller_suffix}",
            capitalize_first(&decider)
        )
    };

    Some(format!(
        "{first_clause}, then {second_clause}. {may_clause}"
    ))
}

pub(super) fn describe_graveyard_mana_ladder_return_clause_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [first_choose, second_choose, third_choose, return_effect] = filtered else {
        return None;
    };
    let chooses = [
        first_choose.downcast_ref::<crate::effects::ChooseObjectsEffect>()?,
        second_choose.downcast_ref::<crate::effects::ChooseObjectsEffect>()?,
        third_choose.downcast_ref::<crate::effects::ChooseObjectsEffect>()?,
    ];
    for (idx, choose) in chooses.iter().enumerate() {
        if choose.chooser != PlayerFilter::You
            || choose_exact_count(choose) != Some(1)
            || choose_primary_zone(choose) != Some(Zone::Graveyard)
            || choose.filter.owner != Some(PlayerFilter::You)
            || choose.filter.card_types != vec![CardType::Creature]
            || choose.filter.mana_value != Some(crate::filter::Comparison::Equal((idx + 1) as i32))
        {
            return None;
        }
    }
    let return_to_battlefield = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>(
    )?;
    if return_to_battlefield.tapped
        || (!matches!(&return_to_battlefield.target, ChooseSpec::Tagged(tag) if tag == &chooses[0].tag)
            && !matches!(&return_to_battlefield.target, ChooseSpec::Iterated))
    {
        return None;
    }
    Some(
        "Choose a creature card with mana value 1 in your graveyard, then do the same for creature cards with mana value 2 and 3. Return those cards to the battlefield."
            .to_string(),
    )
}

pub(super) fn describe_reveal_power_cards_for_mana_clause_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [choose_effect, reveal_effect, mana_effect] = filtered else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    let mana = mana_effect.downcast_ref::<crate::effects::AddScaledManaEffect>()?;
    if choose.chooser != PlayerFilter::You
        || choose.count.min != 0
        || choose.count.max.is_some()
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || reveal.tag != choose.tag
        || mana.player != PlayerFilter::You
        || mana.mana != vec![crate::mana::ManaSymbol::Green]
        || !matches!(&mana.amount, Value::Count(filter) if object_filter_has_tag(filter, &choose.tag))
    {
        return None;
    }
    let mut selection = choose.filter.clone();
    selection.zone = None;
    selection.owner = None;
    selection.controller = None;
    selection.tagged_constraints.clear();
    let mut selection = pluralize_noun_phrase(&selection.description());
    if let Some(rest) = selection.strip_prefix("creatures ") {
        selection = format!("creature cards {rest}");
    } else if selection == "creatures" {
        selection = "creature cards".to_string();
    } else if !selection.contains("card") {
        selection.push_str(" cards");
    }
    Some(format!(
        "Reveal any number of {selection} from your hand. Add {{G}} for each card revealed this way"
    ))
}

pub(in crate::compiled_text) fn describe_chosen_creatures_blessing_additional_combat_clause(
    effects: &[Effect],
) -> Option<String> {
    if let [
        target_effect,
        tag_matching_effect,
        untap_effect,
        for_each_effect,
        grant_effect,
        additional_effect,
        cant_effect,
    ] = effects
    {
        let targeted = target_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
        let target_only = targeted
            .effect
            .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
        let target_count = target_only.target.count();
        let tag_matching =
            tag_matching_effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
        let untap = untap_effect.downcast_ref::<crate::effects::UntapEffect>()?;
        let for_each = for_each_effect
            .downcast_ref::<crate::effects::ForEachObject>()
            .or_else(|| {
                for_each_effect
                    .downcast_ref::<crate::effects::TaggedEffect>()
                    .and_then(|tagged| {
                        tagged
                            .effect
                            .downcast_ref::<crate::effects::ForEachObject>()
                    })
            })?;
        let additional =
            additional_effect.downcast_ref::<crate::effects::AdditionalPhasesEffect>()?;
        let cant = cant_effect.downcast_ref::<crate::effects::CantEffect>()?;
        if target_count.min != 2
            || target_count.max != Some(2)
            || !describe_choose_spec(&target_only.target).contains("target creatures")
            || !object_filter_has_tag(&tag_matching.filter, &targeted.tag)
            || !matches!(
                &untap.target,
                ChooseSpec::All(filter)
                    if object_filter_has_tag(filter, &tag_matching.tag)
                        || object_filter_has_tag(filter, &targeted.tag)
            )
        {
            return None;
        }
        let blessing =
            describe_for_each_chosen_put_counters_then_gain_keywords(for_each, grant_effect)?;
        let combat =
            describe_additional_combat_then_chosen_attack_or_block_restriction(additional, cant)?;
        return Some(format!(
            "Choose two target creatures. Untap them. {blessing}. {combat}"
        ));
    }

    if let [
        target_effect,
        untap_effect,
        for_each_effect,
        grant_effect,
        additional_effect,
        cant_effect,
    ] = effects
        && let Some(targeted) = target_effect.downcast_ref::<crate::effects::TaggedEffect>()
        && let Some(target_only) = targeted
            .effect
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
        && let Some(untap) = untap_effect.downcast_ref::<crate::effects::UntapEffect>()
        && let Some(for_each) = for_each_effect
            .downcast_ref::<crate::effects::ForEachObject>()
            .or_else(|| {
                for_each_effect
                    .downcast_ref::<crate::effects::TaggedEffect>()
                    .and_then(|tagged| {
                        tagged
                            .effect
                            .downcast_ref::<crate::effects::ForEachObject>()
                    })
            })
        && let Some(additional) =
            additional_effect.downcast_ref::<crate::effects::AdditionalPhasesEffect>()
        && let Some(cant) = cant_effect.downcast_ref::<crate::effects::CantEffect>()
    {
        let target_count = target_only.target.count();
        if target_count.min != 2
            || target_count.max != Some(2)
            || !describe_choose_spec(&target_only.target).contains("target creatures")
            || !matches!(
                &untap.target,
                ChooseSpec::All(filter) if object_filter_has_tag(filter, &targeted.tag)
            )
        {
            // fall through to other six-effect shapes below
        } else if let Some(blessing) =
            describe_for_each_chosen_put_counters_then_gain_keywords(for_each, grant_effect)
            && let Some(combat) =
                describe_additional_combat_then_chosen_attack_or_block_restriction(additional, cant)
        {
            return Some(format!(
                "Choose two target creatures. Untap them. {blessing}. {combat}"
            ));
        }
    }

    let [
        choose_effect,
        untap_effect,
        for_each_effect,
        grant_effect,
        additional_effect,
        cant_effect,
    ] = effects
    else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let untap = untap_effect.downcast_ref::<crate::effects::UntapEffect>()?;
    let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachObject>()?;
    let additional = additional_effect.downcast_ref::<crate::effects::AdditionalPhasesEffect>()?;
    let cant = cant_effect.downcast_ref::<crate::effects::CantEffect>()?;
    if choose.chooser != PlayerFilter::You
        || choose.count.min != 2
        || choose.count.max != Some(2)
        || !choose.filter.card_types.contains(&CardType::Creature)
        || !matches!(&untap.target, ChooseSpec::Tagged(tag) if tag == &choose.tag)
    {
        return None;
    }
    let blessing =
        describe_for_each_chosen_put_counters_then_gain_keywords(for_each, grant_effect)?;
    let combat =
        describe_additional_combat_then_chosen_attack_or_block_restriction(additional, cant)?;
    Some(format!(
        "Choose two target creatures. Untap them. {blessing}. {combat}"
    ))
}

fn find_typed_linked_exile_top_play_boundary(effects: &[&Effect]) -> Option<(usize, usize)> {
    effects.iter().enumerate().find_map(|(exile_idx, effect)| {
        let exile_top = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
        let [moved_tag] = exile_top.moved_tags.as_slice() else {
            return None;
        };
        if !exile_top.accumulated_tags.is_empty() {
            return None;
        }

        // The first later grant for this tag owns the link. If it is qualified,
        // do not skip it and accidentally certify a different later grant.
        let (relative_grant_idx, grant_play) = effects[exile_idx + 1..]
            .iter()
            .enumerate()
            .find_map(|(relative_idx, candidate)| {
                structural_unwrap_render_wrappers(candidate)
                    .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
                    .filter(|grant_play| &grant_play.tag == moved_tag)
                    .map(|grant_play| (relative_idx, grant_play))
            })?;

        (grant_play.player == PlayerFilter::You
            && matches!(
                grant_play.duration,
                crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
                    | crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd
                    | crate::effects::GrantPlayTaggedDuration::UntilYourNextEndStep
                    | crate::effects::GrantPlayTaggedDuration::UntilSourceExilesAnother
                    | crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled
            )
            && !grant_play.while_on_top_of_library
            && grant_play.filter.is_none()
            && grant_play.allow_any_color_for_cast == grant_play.mana_spend_mode.allows_any_color())
        .then_some((exile_idx, exile_idx + 1 + relative_grant_idx))
    })
}

fn describe_linked_play_fragment(effects: &[Effect]) -> Option<String> {
    let rendered = match effects {
        [] => return Some(String::new()),
        [effect] => {
            let rendered = describe_effect(effect);
            if rendered.contains(". ") || rendered.contains(": ") {
                return None;
            }
            rendered
        }
        _ => describe_effect_clause_list(effects)?,
    };
    let rendered = rendered.trim().trim_end_matches('.');
    (!rendered.is_empty()).then(|| rendered.to_string())
}

fn describe_simple_linked_play_effect(effect: &Effect) -> Option<String> {
    let rendered = describe_effect(effect);
    let rendered = rendered.trim().trim_end_matches('.');
    (!rendered.is_empty() && !rendered.contains(". ") && !rendered.contains(": "))
        .then(|| rendered.to_string())
}

fn describe_create_token_and_exile_top_setup(
    effects: &[Effect],
    exile_idx: usize,
    grant_idx: usize,
    exile_clause: &str,
) -> Option<Vec<String>> {
    let create_idx = exile_idx.checked_sub(1)?;
    let create_token = structural_unwrap_render_wrappers(&effects[create_idx])
        .downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if create_token.controller != PlayerFilter::You {
        return None;
    }
    let create = describe_simple_linked_play_effect(&effects[create_idx])?;

    let mut sentences = Vec::new();
    if create_idx > 0 {
        let prefix = describe_linked_play_fragment(&effects[..create_idx])?;
        if !prefix.is_empty() {
            sentences.push(prefix);
        }
    }
    sentences.push(format!("{create} and {}", lowercase_first(exile_clause)));
    if exile_idx + 1 < grant_idx {
        let intervening = describe_linked_play_fragment(&effects[exile_idx + 1..grant_idx])?;
        if !intervening.is_empty() {
            sentences.push(capitalize_first(&intervening));
        }
    }
    Some(sentences)
}

fn describe_linked_exile_top_play_parts(
    exile_top: &crate::effects::ExileTopOfLibraryEffect,
    grant_play: &crate::effects::GrantPlayTaggedEffect,
    suppress_count_where_clause: bool,
    without_paying_mana_cost: bool,
) -> Option<(String, String)> {
    if grant_play.player != PlayerFilter::You {
        return None;
    }
    let (exile_clause, singular_count) =
        describe_exile_top_clause(exile_top, suppress_count_where_clause)?;
    if let Some(permission) =
        describe_temporary_tagged_permission_surface(grant_play, without_paying_mana_cost)
    {
        return Some((exile_clause, capitalize_first(&permission)));
    }
    let is_two_card_next_end_step_one_play_pool = !singular_count
        && exile_top.count == Value::Fixed(2)
        && grant_play.duration == crate::effects::GrantPlayTaggedDuration::UntilYourNextEndStep
        && grant_play.allow_land
        && grant_play.cast_pool_is_plural
        && grant_play.max_plays == Some(1);
    let cards_text = if singular_count {
        "that card"
    } else if is_two_card_next_end_step_one_play_pool {
        "one of those cards"
    } else {
        "those cards"
    };
    let verb = if grant_play.allow_land {
        "play"
    } else {
        "cast"
    };
    let spell_reference = if singular_count {
        "that spell"
    } else if grant_play.allow_land {
        "those spells"
    } else {
        "them"
    };
    let mana_suffix = grant_play
        .mana_spend_cast_clause(spell_reference)
        .map(|clause| format!(", and {clause}"))
        .unwrap_or_default();

    let permission = if let Some(counter_type) = grant_play.during_turns_counter_put_on_source {
        format!(
            "During any turn you put {} on this Saga, you may {verb} {cards_text}{mana_suffix}",
            with_indefinite_article(&format!("{} counter", counter_type.description()))
        )
    } else {
        match grant_play.duration {
            crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled => {
                let remaining = if singular_count {
                    "it remains exiled"
                } else {
                    "they remain exiled"
                };
                if !grant_play.allow_land && !singular_count {
                    format!(
                        "You may cast spells from among those cards for as long as {remaining}{mana_suffix}"
                    )
                } else {
                    format!("You may {verb} {cards_text} for as long as {remaining}{mana_suffix}")
                }
            }
            duration => {
                let duration_text = match duration {
                    crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn => "Until end of turn",
                    crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd => {
                        "Until the end of your next turn"
                    }
                    crate::effects::GrantPlayTaggedDuration::UntilYourNextEndStep => {
                        "Until your next end step"
                    }
                    _ => return None,
                };
                if !grant_play.allow_land && !singular_count {
                    format!(
                        "{duration_text}, you may cast spells from among those exiled cards{mana_suffix}"
                    )
                } else if duration == crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
                    && exile_top.player == PlayerFilter::You
                    && !grant_play.allow_any_color_for_cast
                {
                    format!("You may {verb} {cards_text} this turn")
                } else {
                    format!("{duration_text}, you may {verb} {cards_text}{mana_suffix}")
                }
            }
        }
    };
    Some((exile_clause, permission))
}

fn synthetic_target_prefix_for_linked_exile(
    effects: &[Effect],
    exile_idx: usize,
    exile_top: &crate::effects::ExileTopOfLibraryEffect,
) -> Option<usize> {
    let target_idx = exile_idx.checked_sub(1)?;
    let target_effect = &effects[target_idx];
    let target_only = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if !synthetic_target_has_single_consumer(effects, target_effect)
        || choose_spec_player_filter(&target_only.target)? != exile_top.player
    {
        return None;
    }
    Some(target_idx)
}

pub(in crate::compiled_text) fn describe_linked_exile_top_play_clause(
    effects: &[Effect],
) -> Option<String> {
    let refs = effects.iter().collect::<Vec<_>>();
    let (exile_idx, grant_idx) = find_typed_linked_exile_top_play_boundary(&refs)?;
    let exile_top = structural_unwrap_render_wrappers(&effects[exile_idx])
        .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    let grant_play = structural_unwrap_render_wrappers(&effects[grant_idx])
        .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    let consumes_free_cast = effects.get(grant_idx + 1).is_some_and(|effect| {
        structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect>()
            .is_some_and(|free_cast| {
                free_cast.tag == grant_play.tag
                    && free_cast.player == grant_play.player
                    && free_cast.duration == grant_play.duration
            })
    });
    let suppress_count_where_clause = exile_idx
        .checked_sub(1)
        .and_then(|previous_idx| {
            structural_unwrap_render_wrappers(&effects[previous_idx])
                .downcast_ref::<crate::effects::GrantNextSpellCostReductionEffect>()
        })
        .and_then(|reduction| reduction.generic_reduction.as_ref())
        .is_some_and(|reduction| reduction == &exile_top.count);
    let (exile_clause, permission) = describe_linked_exile_top_play_parts(
        exile_top,
        grant_play,
        suppress_count_where_clause,
        consumes_free_cast,
    )?;
    let synthetic_target_idx =
        synthetic_target_prefix_for_linked_exile(effects, exile_idx, exile_top);

    let mut sentences = if let Some(coordinated) =
        describe_create_token_and_exile_top_setup(effects, exile_idx, grant_idx, &exile_clause)
    {
        coordinated
    } else {
        let mut setup = Vec::new();
        let prefix_end = synthetic_target_idx.unwrap_or(exile_idx);
        let prefix = describe_linked_play_fragment(&effects[..prefix_end])?;
        if !prefix.is_empty() {
            setup.push(prefix);
        }
        setup.push(capitalize_first(&exile_clause));
        let intervening = describe_linked_play_fragment(&effects[exile_idx + 1..grant_idx])?;
        if !intervening.is_empty() {
            setup.push(capitalize_first(&intervening));
        }
        setup
    };
    sentences.push(permission);
    let suffix_start = grant_idx + 1 + usize::from(consumes_free_cast);
    if suffix_start < effects.len() {
        let suffix = describe_linked_play_fragment(&effects[suffix_start..])?;
        if !suffix.is_empty() {
            sentences.push(capitalize_first(&suffix));
        }
    }
    Some(cleanup_decompiled_text(&lowercase_first(
        &sentences.join(". "),
    )))
}

/// `describe_put_counters_then_grant_same_filter` already honours the authored
/// `CounterGrantSeparateSentence` hint, but only for an exact `[put, grant]` pair.
/// A third instruction after the grant ("... . You gain 2 life.") drops the whole
/// clause onto the comma-joining path, producing the run-on
/// "Put a +1/+1 counter on target creature, that creature gains first strike until
/// end of turn, then gain 2 life."
///
/// Keying off the same hint keeps the comma where oracle authored one — Olivia,
/// Mobilized for War's "put a +1/+1 counter on that creature, it gains haste until
/// end of turn" never carries it.
fn counter_placement_precedes_tagged_grant(effects: &[&Effect]) -> bool {
    effects.windows(2).any(|pair| {
        unwrap_basic_tag_wrappers(pair[0])
            .downcast_ref::<crate::effects::PutCountersEffect>()
            .is_some_and(|put| {
                put.amount
                    .has_surface_hint(ValueSurfaceHint::CounterGrantSeparateSentence)
            })
            && unwrap_basic_tag_wrappers(pair[1])
                .downcast_ref::<crate::effects::ApplyContinuousEffect>()
                .is_some()
    })
}

pub(super) fn clause_effects_have_typed_sentence_boundaries(effects: &[&Effect]) -> bool {
    if counter_placement_precedes_tagged_grant(effects) {
        return true;
    }
    match effects {
        [first, second] => {
            if let Some(add_mana) = unwrap_basic_tag_wrappers(first)
                .downcast_ref::<crate::effects::AddManaOfAnyColorEffect>()
                && add_mana.amount == Value::Fixed(1)
                && add_mana.player == PlayerFilter::You
                && !add_mana.distinct_colors
                && let Some(damage) = unwrap_basic_tag_wrappers(second)
                    .downcast_ref::<crate::effects::DealDamageEffect>()
                && matches!(damage.amount, Value::Fixed(_))
                && damage.target == ChooseSpec::SourceController
                && !damage.source_is_combat
                && !damage.unpreventable
            {
                return true;
            }

            if unwrap_basic_tag_wrappers(first)
                .downcast_ref::<crate::effects::AddManaOfAnyColorEffect>()
                .is_some()
                && let Some(cant) =
                    unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::CantEffect>()
                && matches!(&cant.restriction, crate::effect::Restriction::Untap(filter) if filter.source)
            {
                return true;
            }

            if let Some(tag) = effect_outer_tag(first)
                && unwrap_basic_tag_wrappers(first)
                    .downcast_ref::<crate::effects::ApplyContinuousEffect>()
                    .is_some()
                && let Some(remove) = unwrap_basic_tag_wrappers(second)
                    .downcast_ref::<crate::effects::RemoveUpToAnyCountersEffect>()
                && matches!(&remove.target, ChooseSpec::Tagged(found) if found == tag)
                && matches!(
                    &remove.max_count,
                    Value::CountersOn(spec, None)
                        if matches!(spec.as_ref(), ChooseSpec::Tagged(found) if found == tag)
                )
            {
                return true;
            }

            if let Some(tag) = effect_outer_tag(first)
                && let Some(return_all) = unwrap_basic_tag_wrappers(first)
                    .downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>(
                )
                && return_all.face_down
                && let Some(apply) = unwrap_basic_tag_wrappers(second)
                    .downcast_ref::<crate::effects::ApplyContinuousEffect>()
                && apply_continuous_targets_tag(apply, tag)
            {
                return true;
            }

            if let Some(tag) = effect_outer_tag(first)
                && unwrap_basic_tag_wrappers(first)
                    .downcast_ref::<crate::effects::ApplyContinuousEffect>()
                    .is_some()
                && let Some(untap) =
                    unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::UntapEffect>()
                && choose_spec_references_tagged_object(&untap.target, tag)
            {
                return true;
            }

            false
        }
        [reduction, exile_top, grant_play] => {
            reduction
                .downcast_ref::<crate::effects::GrantNextSpellCostReductionEffect>()
                .is_some()
                && exile_top
                    .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
                    .is_some()
                && grant_play
                    .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
                    .is_some()
        }
        [counter_effect, color_effect, subtype_effect, ability_effect] => {
            let Some(tag) = effect_outer_tag(counter_effect) else {
                return false;
            };
            if unwrap_basic_tag_wrappers(counter_effect)
                .downcast_ref::<crate::effects::PutCountersEffect>()
                .is_none()
            {
                return false;
            }
            let Some(color_apply) = tagged_apply_continuous_effect(color_effect) else {
                return false;
            };
            let Some(subtype_apply) = tagged_apply_continuous_effect(subtype_effect) else {
                return false;
            };
            let Some(ability_apply) = tagged_apply_continuous_effect(ability_effect) else {
                return false;
            };
            apply_continuous_targets_tag(color_apply, tag)
                && apply_continuous_targets_tag(subtype_apply, tag)
                && apply_continuous_targets_tag(ability_apply, tag)
                && color_apply.until == subtype_apply.until
                && color_apply.until == ability_apply.until
                && matches!(
                    &color_apply.modification,
                    Some(crate::continuous::Modification::SetColors(_))
                )
                && matches!(
                    &subtype_apply.modification,
                    Some(crate::continuous::Modification::AddSubtypes(_))
                )
                && matches!(
                    &ability_apply.modification,
                    Some(crate::continuous::Modification::AddAbility(_))
                )
        }
        _ => false,
    }
}

fn describe_optional_look_then_reveal_top_rest_bottom(effects: &[Effect]) -> Option<String> {
    let [with_id_effect, if_effect] = effects else {
        return None;
    };
    let with_id = with_id_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| *decider != PlayerFilter::You)
    {
        return None;
    }
    let [look_effect] = may.effects.as_slice() else {
        return None;
    };
    let look_at_top = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let if_effect = if_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != with_id.id
        || if_effect.predicate != EffectPredicate::Happened
        || !if_effect.else_.is_empty()
    {
        return None;
    }
    let [choose_effect, reveal_effect, move_effect, remainder_effect] = if_effect.then.as_slice()
    else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let reveal = reveal_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let (_, move_to_top) = for_each_tagged_for_compaction(move_effect)?;
    let remainder = remainder_effect
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if look_at_top.player != PlayerFilter::You
        || choose.chooser != PlayerFilter::You
        || choose.is_search
        || !choose_references_tag(choose, &look_at_top.tag)
        || !for_each_reveals_tag(reveal, choose.tag.as_str())
        || !for_each_moves_tag_to_library_top(move_to_top, choose.tag.as_str())
        || remainder.tag != look_at_top.tag
        || remainder.keep_tagged.as_ref() != Some(&choose.tag)
        || remainder.player != look_at_top.player
    {
        return None;
    }

    let selection = if choose.count.is_any_number() {
        format!(
            "any number of {}",
            describe_any_number_filter_from_looked_cards(look_at_top, choose)?
        )
    } else if choose.count == ChoiceCount::up_to(1) {
        let single = describe_choose_filter_from_looked_cards(look_at_top, choose)?;
        format!("up to one {}", strip_indefinite_article(&single))
    } else {
        describe_counted_choose_filter_from_looked_cards(look_at_top, choose)?
    };
    let owner = describe_possessive_player_filter(&look_at_top.player);
    let (count_text, noun, where_clause) =
        describe_top_count_noun_and_where_clause(&look_at_top.count);
    let selected_reference = if choose.count.max == Some(1) {
        "that card"
    } else {
        "those cards"
    };
    let selected_order = if choose.count.max == Some(1) {
        ""
    } else {
        " in any order"
    };
    let remainder_order = match remainder.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => " in a random order",
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => " in any order",
    };

    Some(format!(
        "You may look at the top {count_text} {noun} of {owner} library{where_clause}. If you do, reveal {selection} from among them, then put {selected_reference} on top of {owner} library{selected_order} and the rest on the bottom of {owner} library{remainder_order}"
    ))
}

fn describe_looked_hand_rest_bottom_clause(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    reveal_top: Option<&crate::effects::RevealTaggedEffect>,
    choose: &crate::effects::ChooseObjectsEffect,
    move_effect: &Effect,
    remainder: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
) -> Option<String> {
    if reveal_top.is_some_and(|reveal| reveal.tag != look_at_top.tag)
        || choose.is_search
        || !choose_references_tag(choose, &look_at_top.tag)
        || remainder.tag != look_at_top.tag
        || remainder.keep_tagged.as_ref() != Some(&choose.tag)
        || remainder.player != look_at_top.player
    {
        return None;
    }
    let (_, move_to_hand) = for_each_tagged_for_compaction(move_effect)?;
    if !for_each_moves_tag_to_hand(move_to_hand, choose.tag.as_str()) {
        return None;
    }

    let selection = if choose.count.is_any_number() {
        format!(
            "any number of {}",
            describe_any_number_filter_from_looked_cards(look_at_top, choose)?
        )
    } else {
        describe_counted_choose_filter_from_looked_cards(look_at_top, choose)?
    };
    let mandatory = choose.count.min > 0
        && choose.count.max == Some(choose.count.min)
        && !choose.count.dynamic_x
        && choose.search_mode != SearchSelectionMode::Optional;
    let actor = if mandatory {
        "Put".to_string()
    } else if choose.chooser == PlayerFilter::You {
        "You may put".to_string()
    } else {
        let chooser = capitalize_first(&describe_player_filter(&choose.chooser));
        format!("{chooser} may put")
    };
    let opener = if look_at_top.reveal || reveal_top.is_some() {
        "Reveal"
    } else {
        "Look at"
    };
    let owner = describe_possessive_player_filter(&look_at_top.player);
    let hand = describe_possessive_player_filter(&choose.chooser);
    let (count_text, noun, where_clause) =
        describe_top_count_noun_and_where_clause(&look_at_top.count);
    let order = match remainder.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => " in a random order",
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => " in any order",
    };
    let remainder_clause = match remainder.surface {
        ironsmith_core::LibraryRemainderSurface::SentenceLeadingThenRest => {
            format!(". Then put the rest on the bottom of {owner} library{order}")
        }
        ironsmith_core::LibraryRemainderSurface::Rest
        | ironsmith_core::LibraryRemainderSurface::RestBare => {
            format!(" and the rest on the bottom of {owner} library{order}")
        }
        _ => return None,
    };

    Some(format!(
        "{opener} the top {count_text} {noun} of {owner} library{where_clause}. {actor} {selection} from among them into {hand} hand{remainder_clause}"
    ))
}

fn shuffle_matches_looked_library(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    shuffle: &crate::effects::ShuffleLibraryEffect,
) -> bool {
    shuffle.player == look_at_top.player && shuffle.target_spec.is_none()
}

pub(super) fn describe_looked_battlefield_then_shuffle(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    move_effect: &Effect,
    shuffle: &crate::effects::ShuffleLibraryEffect,
    optionality_from_outer_may: bool,
    inline_comma_then: bool,
) -> Option<String> {
    if choose.is_search
        || !choose_references_tag(choose, &look_at_top.tag)
        || !shuffle_matches_looked_library(look_at_top, shuffle)
    {
        return None;
    }
    let (_, for_each) = for_each_tagged_for_compaction(move_effect)?;
    if for_each.tag != choose.tag || for_each.effects.len() != 1 {
        return None;
    }
    let move_to_zone = unwrap_basic_tag_wrappers(&for_each.effects[0])
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || !matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
    {
        return None;
    }

    let mut selection = describe_looked_battlefield_selection(choose)?;
    if optionality_from_outer_may && let Some(rest) = selection.strip_prefix("up to one ") {
        selection = with_indefinite_article(rest);
    }
    let actor = if choose.count.min == 0 {
        if choose.chooser == PlayerFilter::You {
            "You may put".to_string()
        } else {
            format!(
                "{} may put",
                capitalize_first(&describe_player_filter(&choose.chooser))
            )
        }
    } else {
        "Put".to_string()
    };
    let owner = describe_possessive_player_filter(&look_at_top.player);
    let (count_text, noun, where_clause) =
        describe_top_count_noun_and_where_clause(&look_at_top.count);
    let opener = if look_at_top.reveal {
        "Reveal"
    } else {
        "Look at"
    };
    let entry_state = describe_battlefield_entry_state_for_looked_move(move_to_zone);
    if inline_comma_then {
        if !choose.count.is_any_number() || choose.chooser != PlayerFilter::You {
            return None;
        }
        return Some(format!(
            "{opener} the top {count_text} {noun} of {owner} library{where_clause}, put {selection} from among them onto the battlefield{entry_state}, then shuffle"
        ));
    }
    Some(format!(
        "{opener} the top {count_text} {noun} of {owner} library{where_clause}. {actor} {selection} from among them onto the battlefield{entry_state}. Then shuffle"
    ))
}

fn describe_looked_same_name_permanent_battlefield_rest_bottom(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    comparison_set: &crate::effects::TagMatchingObjectsEffect,
    may: &crate::effects::MayEffect,
    remainder: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
) -> Option<String> {
    if look_at_top.reveal
        || look_at_top.player != PlayerFilter::You
        || comparison_set.zone != Some(Zone::Battlefield)
        || !comparison_set.additional_zones.is_empty()
        || comparison_set.filter != ObjectFilter::permanent()
        || remainder.tag != look_at_top.tag
        || remainder.player != PlayerFilter::You
        || !matches!(may.decider, None | Some(PlayerFilter::You))
    {
        return None;
    }
    let [choose_effect, move_effect] = may.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::You
        || choose.zone != Some(Zone::Library)
        || !choose.additional_zones.is_empty()
        || choose.count != ChoiceCount::exactly(1)
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || choose.is_search
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
        || choose.replace_tagged_objects
        || remainder.keep_tagged.as_ref() != Some(&choose.tag)
    {
        return None;
    }
    let has_looked_membership = choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == look_at_top.tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    });
    let has_name_comparison = choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == comparison_set.tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
    });
    let mut plain_choice = choose.filter.clone();
    plain_choice.zone = None;
    plain_choice.tagged_constraints.clear();
    if choose.filter.zone != Some(Zone::Library)
        || choose.filter.tagged_constraints.len() != 2
        || !has_looked_membership
        || !has_name_comparison
        || plain_choice != ObjectFilter::default()
    {
        return None;
    }
    let (_, for_each) = for_each_tagged_for_compaction(move_effect)?;
    let [move_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let move_to_zone = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if for_each.tag != choose.tag
        || move_to_zone.zone != Zone::Battlefield
        || move_to_zone.to_top
        || move_to_zone.library_order.is_some()
        || !matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
    {
        return None;
    }

    let (count_text, noun, where_clause) =
        describe_top_count_noun_and_where_clause(&look_at_top.count);
    let order = match remainder.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => " in a random order",
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => " in any order",
    };
    Some(format!(
        "Look at the top {count_text} {noun} of your library{where_clause}. You may put one of those cards onto the battlefield if it has the same name as a permanent. Put the rest on the bottom of your library{order}"
    ))
}

fn effect_reveals_looked_choice(effect: &Effect, tag: &crate::TagKey) -> bool {
    if let Some(reveal) =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::RevealTaggedEffect>()
    {
        return reveal.tag == *tag;
    }
    effect
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()
        .is_some_and(|for_each| for_each_reveals_tag(for_each, tag.as_str()))
}

fn effect_moves_looked_choice_to_hand(effect: &Effect, tag: &crate::TagKey) -> bool {
    if let Some(move_to_zone) =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::MoveToZoneEffect>()
    {
        return move_to_zone.zone == Zone::Hand
            && matches!(move_to_zone.target.base(), ChooseSpec::Tagged(found) if found == tag);
    }
    for_each_tagged_for_compaction(effect)
        .is_some_and(|(_, for_each)| for_each_moves_tag_to_hand(for_each, tag.as_str()))
}

fn describe_looked_reveal_hand_then_shuffle(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    reveal_effect: &Effect,
    move_effect: &Effect,
    shuffle: &crate::effects::ShuffleLibraryEffect,
) -> Option<String> {
    if look_at_top.reveal
        || choose.is_search
        || !choose_references_tag(choose, &look_at_top.tag)
        || !effect_reveals_looked_choice(reveal_effect, &choose.tag)
        || !effect_moves_looked_choice_to_hand(move_effect, &choose.tag)
        || !shuffle_matches_looked_library(look_at_top, shuffle)
    {
        return None;
    }
    let selection = describe_counted_choose_filter_from_looked_cards(look_at_top, choose)?;
    let (selection, where_clause) = selection
        .split_once(", where X is ")
        .map(|(head, basis)| (head.to_string(), format!(", where X is {basis}")))
        .unwrap_or((selection, String::new()));
    let reveal_actor = if choose.count.min == 0 {
        if choose.chooser == PlayerFilter::You {
            "You may reveal".to_string()
        } else {
            format!(
                "{} may reveal",
                capitalize_first(&describe_player_filter(&choose.chooser))
            )
        }
    } else {
        "Reveal".to_string()
    };
    let owner = describe_possessive_player_filter(&look_at_top.player);
    let hand = describe_possessive_player_filter(&choose.chooser);
    let (count_text, noun, look_where_clause) =
        describe_top_count_noun_and_where_clause(&look_at_top.count);
    Some(format!(
        "Look at the top {count_text} {noun} of {owner} library{look_where_clause}. {reveal_actor} {selection} from among them{where_clause}. Put the revealed cards into {hand} hand, then shuffle"
    ))
}

fn looked_granted_ability_text(effect: &Effect, chosen_tag: &crate::TagKey) -> Option<String> {
    let apply = unwrap_basic_tag_wrappers(effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if !matches!(apply.until, Until::Forever)
        || apply.condition.is_some()
        || !apply.runtime_modifications.is_empty()
        || !matches!(
            apply.target_spec.as_ref(),
            Some(spec) if choose_spec_references_tagged_object(spec, chosen_tag)
        )
    {
        return None;
    }
    let grants_ability = |modification: &crate::continuous::Modification| {
        matches!(
            modification,
            crate::continuous::Modification::AddAbility(_)
                | crate::continuous::Modification::AddAbilityGeneric(_)
        )
    };
    if !apply.modification.as_ref().is_some_and(grants_ability)
        || !apply.additional_modifications.iter().all(grants_ability)
    {
        return None;
    }
    let mut text = describe_effect(effect).trim_end_matches('.').to_string();
    for subject in ["That object", "That card", "The chosen card"] {
        if let Some(rest) = text.strip_prefix(subject) {
            text = format!("It{rest}");
            break;
        }
    }
    Some(capitalize_first(&text))
}

fn describe_looked_battlefield_grant_then_remainder(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    reveal_top: Option<&crate::effects::RevealTaggedEffect>,
    choose: &crate::effects::ChooseObjectsEffect,
    move_effect: &Effect,
    grant_effect: &Effect,
    remainder: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
) -> Option<String> {
    let base = describe_look_at_top_choose_battlefield_rest_bottom(
        look_at_top,
        reveal_top,
        choose,
        move_effect,
        remainder,
    )?;
    let grant = looked_granted_ability_text(grant_effect, &choose.tag)?;
    let (put_selected, remainder) = base.rsplit_once(". ")?;
    let remainder = if remainder.starts_with("Then ") {
        remainder.to_string()
    } else {
        format!("Then {}", lowercase_first(remainder))
    };
    Some(format!("{put_selected}. {grant}. {remainder}"))
}

fn describe_looked_battlefield_rest_then_reflexive(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    reveal_top: Option<&crate::effects::RevealTaggedEffect>,
    choose: &crate::effects::ChooseObjectsEffect,
    move_effect: &Effect,
    reflexive: &crate::effects::ReflexiveTriggerEffect,
    remainder: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
) -> Option<String> {
    let (with_id, for_each) = for_each_tagged_for_compaction(move_effect)?;
    let with_id = with_id?;
    if reflexive.condition != with_id.id || for_each.tag != choose.tag {
        return None;
    }
    let card_type = match &reflexive.predicate {
        EffectPredicate::AffectedObjectMatchesCardType {
            card_type,
            negated: false,
        } => *card_type,
        EffectPredicate::PriorEffectResult(surface)
            if surface.action == crate::effect::PriorEffectAction::PutOntoBattlefield
                && surface.actor == crate::effect::PriorEffectResultActor::Passive
                && surface.quantifier == crate::effect::PriorEffectResultQuantifier::One =>
        {
            let mut creature_card = ObjectFilter::default();
            creature_card.card_types.push(CardType::Creature);
            if surface.filter != creature_card {
                return None;
            }
            CardType::Creature
        }
        _ => return None,
    };
    let move_to_zone = for_each.effects.first().and_then(|effect| {
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::MoveToZoneEffect>()
    })?;
    if move_to_zone.zone != Zone::Battlefield
        || !matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
    {
        return None;
    }
    let base = describe_look_at_top_choose_battlefield_rest_bottom(
        look_at_top,
        reveal_top,
        choose,
        move_effect,
        remainder,
    )?;
    let type_word = describe_card_type_word_local(card_type);
    let mut triggered = lowercase_first(&describe_result_branch_effect_list(&reflexive.effects));
    let affected_subject = format!("a {type_word} ");
    if let Some(rest) = triggered.strip_prefix(&affected_subject) {
        triggered = format!("it {rest}");
    }
    Some(format!(
        "{base}. When a {type_word} is put onto the battlefield this way, {triggered}"
    ))
}

/// Looked-card compactors normally run from `describe_effect_list`, but
/// resolution programs prefer `describe_effect_clause_list`. Keep the shared
/// card-pool routing shapes at that earlier dispatch point so their tagged
/// implementation details do not leak into compiled rules text.
fn describe_looked_cards_clause_prefix(effects: &[Effect]) -> Option<(String, usize)> {
    let hidden_prefix = effects
        .iter()
        .take_while(|effect| {
            effect
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some()
                || effect
                    .downcast_ref::<crate::effects::TagTriggeringSourceEffect>()
                    .is_some()
                || effect
                    .downcast_ref::<crate::effects::TagTriggeringBlockersEffect>()
                    .is_some()
        })
        .count();
    let visible = effects.get(hidden_prefix..)?;

    fn optional_choice_and_move(
        effect: &Effect,
    ) -> Option<(crate::effects::ChooseObjectsEffect, &Effect)> {
        let may = effect.downcast_ref::<crate::effects::MayEffect>()?;
        let [choose_effect, move_effect] = may.effects.as_slice() else {
            return None;
        };
        let mut choose = choose_effect
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()?
            .clone();
        if may
            .decider
            .as_ref()
            .is_some_and(|decider| *decider != choose.chooser)
        {
            return None;
        }
        // The outer May is the optionality.  Existing looked-card renderers
        // derive "may put" from the choice count, so reflect that without
        // mutating the runtime effect.
        choose.count.min = 0;
        Some((choose, move_effect))
    }

    if let [look_effect, choose_effect, move_effect, rest_effect, ..] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some((_, move_chosen)) = for_each_tagged_for_compaction(move_effect)
        && let Some((_, rest)) = for_each_tagged_for_compaction(rest_effect)
    {
        if let Some(compact) = describe_look_at_top_then_put_into_hand_rest_graveyard(
            look_at_top,
            None,
            choose,
            None,
            move_chosen,
            rest,
        ) {
            return Some((compact, hidden_prefix + 4));
        }
        if let Some(compact) = describe_look_at_top_then_put_matching_to_zone_rest_hand(
            look_at_top,
            None,
            choose,
            move_chosen,
            rest,
        ) {
            return Some((compact, hidden_prefix + 4));
        }
    }

    if let [look_effect, may_effect, shuffle_effect, ..] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some((choose, move_effect)) = optional_choice_and_move(may_effect)
        && let Some(shuffle) = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && let Some(compact) = describe_looked_battlefield_then_shuffle(
            look_at_top,
            &choose,
            move_effect,
            shuffle,
            true,
            false,
        )
    {
        return Some((compact, hidden_prefix + 3));
    }

    if let [look_effect, may_effect, grant_effect, remainder_effect, ..] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some((choose, move_effect)) = optional_choice_and_move(may_effect)
        && let Some(remainder) = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )
        && let Some(compact) = describe_looked_battlefield_grant_then_remainder(
            look_at_top,
            None,
            &choose,
            move_effect,
            grant_effect,
            remainder,
        )
    {
        return Some((compact, hidden_prefix + 4));
    }

    if let [look_effect, may_effect, remainder_effect, ..] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some((choose, move_effect)) = optional_choice_and_move(may_effect)
        && let Some(remainder) = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )
    {
        if let Some(compact) = describe_looked_hand_rest_bottom_clause(
            look_at_top,
            None,
            &choose,
            move_effect,
            remainder,
        ) {
            return Some((compact, hidden_prefix + 3));
        }
        if let Some(compact) = describe_look_at_top_choose_battlefield_rest_bottom(
            look_at_top,
            None,
            &choose,
            move_effect,
            remainder,
        ) {
            return Some((compact, hidden_prefix + 3));
        }
    }

    if let [
        look_effect,
        comparison_effect,
        may_effect,
        remainder_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(comparison_set) =
            comparison_effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()
        && let Some(may) = may_effect.downcast_ref::<crate::effects::MayEffect>()
        && let Some(remainder) = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )
        && let Some(compact) = describe_looked_same_name_permanent_battlefield_rest_bottom(
            look_at_top,
            comparison_set,
            may,
            remainder,
        )
    {
        return Some((compact, hidden_prefix + 4));
    }

    if let [
        look_effect,
        choose_effect,
        reveal_effect,
        move_effect,
        shuffle_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(shuffle) = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && let Some(compact) = describe_looked_reveal_hand_then_shuffle(
            look_at_top,
            choose,
            reveal_effect,
            move_effect,
            shuffle,
        )
    {
        return Some((compact, hidden_prefix + 5));
    }

    if let [look_effect, choose_effect, move_effect, shuffle_effect, ..] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(shuffle) = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && let Some(compact) = describe_looked_battlefield_then_shuffle(
            look_at_top,
            choose,
            move_effect,
            shuffle,
            false,
            false,
        )
    {
        return Some((compact, hidden_prefix + 4));
    }

    if let [look_effect, may_effect, remainder_effect, ..] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(may) = may_effect.downcast_ref::<crate::effects::MayEffect>()
        && let Some(remainder) = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )
        && let Some(compact) = describe_looked_may_one_top_rest_bottom(look_at_top, may, remainder)
    {
        return Some((compact, hidden_prefix + 3));
    }

    if let [
        look_effect,
        choose_effect,
        move_effect,
        reflexive_effect,
        remainder_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(reflexive) =
            reflexive_effect.downcast_ref::<crate::effects::ReflexiveTriggerEffect>()
        && let Some(remainder) = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )
        && let Some(compact) = describe_looked_battlefield_rest_then_reflexive(
            look_at_top,
            None,
            choose,
            move_effect,
            reflexive,
            remainder,
        )
    {
        return Some((compact, hidden_prefix + 5));
    }

    if let [
        look_effect,
        reveal_top_effect,
        choose_effect,
        move_effect,
        reflexive_effect,
        remainder_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(reveal_top) =
            reveal_top_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(reflexive) =
            reflexive_effect.downcast_ref::<crate::effects::ReflexiveTriggerEffect>()
        && let Some(remainder) = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )
        && let Some(compact) = describe_looked_battlefield_rest_then_reflexive(
            look_at_top,
            Some(reveal_top),
            choose,
            move_effect,
            reflexive,
            remainder,
        )
    {
        return Some((compact, hidden_prefix + 6));
    }

    if let [
        look_effect,
        choose_effect,
        move_effect,
        grant_effect,
        remainder_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(remainder) = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )
        && let Some(compact) = describe_looked_battlefield_grant_then_remainder(
            look_at_top,
            None,
            choose,
            move_effect,
            grant_effect,
            remainder,
        )
    {
        return Some((compact, hidden_prefix + 5));
    }

    if let [
        look_effect,
        reveal_top_effect,
        choose_effect,
        move_effect,
        grant_effect,
        remainder_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(reveal_top) =
            reveal_top_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(remainder) = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )
        && let Some(compact) = describe_looked_battlefield_grant_then_remainder(
            look_at_top,
            Some(reveal_top),
            choose,
            move_effect,
            grant_effect,
            remainder,
        )
    {
        return Some((compact, hidden_prefix + 6));
    }

    if let [
        look_effect,
        reveal_top_effect,
        hand_choose_effect,
        hand_move_effect,
        matching_choose_effect,
        matching_move_effect,
        rest_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(reveal_top) =
            reveal_top_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()
        && let Some(hand_choose) =
            hand_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(matching_choose) =
            matching_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(rest) = rest_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()
        && let Some(compact) = describe_looked_one_hand_then_matching_to_zone_rest_graveyard(
            look_at_top,
            Some(reveal_top),
            hand_choose,
            hand_move_effect,
            matching_choose,
            matching_move_effect,
            rest,
        )
    {
        return Some((compact, hidden_prefix + 7));
    }
    if let [
        look_effect,
        hand_choose_effect,
        hand_move_effect,
        matching_choose_effect,
        matching_move_effect,
        rest_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(hand_choose) =
            hand_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(matching_choose) =
            matching_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(rest) = rest_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()
        && let Some(compact) = describe_looked_one_hand_then_matching_to_zone_rest_graveyard(
            look_at_top,
            None,
            hand_choose,
            hand_move_effect,
            matching_choose,
            matching_move_effect,
            rest,
        )
    {
        return Some((compact, hidden_prefix + 6));
    }

    if let [
        look_effect,
        battlefield_choose_effect,
        battlefield_move_effect,
        if_not_moved_effect,
        rest_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(battlefield_choose) =
            battlefield_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some((battlefield_move_id, battlefield_move)) =
            for_each_tagged_for_compaction(battlefield_move_effect)
        && let Some(if_not_moved) = if_not_moved_effect.downcast_ref::<crate::effects::IfEffect>()
        && let Some(rest) = rest_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()
        && let Some(compact) = describe_look_at_top_then_may_put_battlefield_else_hand_rest_bottom(
            look_at_top,
            battlefield_choose,
            battlefield_move_id,
            battlefield_move,
            if_not_moved,
            rest,
        )
    {
        return Some((compact, hidden_prefix + 5));
    }

    if let [
        look_effect,
        reveal_top_effect,
        choose_effect,
        move_effect,
        rest_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(reveal_top) =
            reveal_top_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(rest) = rest_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()
        && let Some((_, move_to_hand)) = for_each_tagged_for_compaction(move_effect)
        && let Some(compact) = describe_look_at_top_then_put_into_hand_rest_graveyard(
            look_at_top,
            Some(reveal_top),
            choose,
            None,
            move_to_hand,
            rest,
        )
    {
        return Some((compact, hidden_prefix + 5));
    }

    if let [
        look_effect,
        reveal_top_effect,
        choose_effect,
        move_effect,
        remainder_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(reveal_top) =
            reveal_top_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(remainder) = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )
    {
        if let Some(compact) = describe_looked_hand_rest_bottom_clause(
            look_at_top,
            Some(reveal_top),
            choose,
            move_effect,
            remainder,
        ) {
            return Some((compact, hidden_prefix + 5));
        }
        if let Some(compact) = describe_look_at_top_choose_battlefield_rest_bottom(
            look_at_top,
            Some(reveal_top),
            choose,
            move_effect,
            remainder,
        ) {
            return Some((compact, hidden_prefix + 5));
        }
        if let Some((_, move_chosen)) = for_each_tagged_for_compaction(move_effect)
            && let Some(compact) = describe_look_at_top_then_put_any_matching_to_zone_rest_bottom(
                look_at_top,
                Some(reveal_top),
                choose,
                move_chosen,
                remainder,
            )
        {
            return Some((compact, hidden_prefix + 5));
        }
    }

    if let [
        look_effect,
        choose_effect,
        reveal_effect,
        move_effect,
        remainder_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(reveal) = reveal_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()
        && let Some((_, move_chosen)) = for_each_tagged_for_compaction(move_effect)
        && let Some(remainder) = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )
    {
        if let Some(compact) = describe_look_at_top_then_reveal_put_on_top_rest_bottom(
            look_at_top,
            choose,
            reveal,
            move_chosen,
            remainder,
        ) {
            return Some((compact, hidden_prefix + 5));
        }
        if let Some(compact) = describe_look_at_top_then_reveal_put_into_hand_rest_bottom(
            look_at_top,
            choose,
            Some(reveal),
            move_chosen,
            remainder,
        ) {
            return Some((compact, hidden_prefix + 5));
        }
    }

    if let [
        look_effect,
        choose_effect,
        move_effect,
        remainder_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(remainder) = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )
    {
        if let Some(compact) =
            describe_looked_up_to_one_top_rest_bottom(look_at_top, choose, move_effect, remainder)
        {
            return Some((compact, hidden_prefix + 4));
        }
        if let Some(compact) = describe_looked_hand_rest_bottom_clause(
            look_at_top,
            None,
            choose,
            move_effect,
            remainder,
        ) {
            return Some((compact, hidden_prefix + 4));
        }
        if let Some(compact) = describe_look_at_top_choose_battlefield_rest_bottom(
            look_at_top,
            None,
            choose,
            move_effect,
            remainder,
        ) {
            return Some((compact, hidden_prefix + 4));
        }
        if let Some((_, move_chosen)) = for_each_tagged_for_compaction(move_effect)
            && let Some(compact) = describe_look_at_top_then_put_any_matching_to_zone_rest_bottom(
                look_at_top,
                None,
                choose,
                move_chosen,
                remainder,
            )
        {
            return Some((compact, hidden_prefix + 4));
        }
    }

    None
}

/// Render a complete looked-card procedure only when the existing structural
/// prefix matcher accounts for every visible effect. Resolution programs use
/// this stricter surface when source-sentence segmentation separates the
/// producer, optional selection, tagged follow-up, and exact complement.
pub(in crate::compiled_text) fn describe_complete_looked_cards_clause(
    effects: &[Effect],
) -> Option<String> {
    let (rendered, consumed) = describe_looked_cards_clause_prefix(effects)?;
    (consumed == effects.len()).then_some(rendered)
}

fn describe_sequence_wrapped_hand_pipeline(effects: &[Effect]) -> Option<String> {
    fn collect_visible_hand_pipeline_effects<'a>(
        effects: &'a [Effect],
        visible: &mut Vec<&'a Effect>,
    ) {
        for effect in effects {
            let effect = structural_unwrap_render_wrappers(effect);
            if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
                collect_visible_hand_pipeline_effects(&sequence.effects, visible);
                continue;
            }
            if effect
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some()
                || effect
                    .downcast_ref::<crate::effects::TagTriggeringSourceEffect>()
                    .is_some()
                || effect
                    .downcast_ref::<crate::effects::TagTriggeringBlockersEffect>()
                    .is_some()
                || effect
                    .downcast_ref::<crate::effects::TargetOnlyEffect>()
                    .is_some_and(|target| !target.explicit_declaration)
            {
                continue;
            }
            visible.push(effect);
        }
    }

    // Hand pipelines are frequently split by source-sentence coordination,
    // result-branch wrapping, or hidden trigger/target bookkeeping. Flatten
    // only presentation-neutral wrappers; the typed matcher below still has
    // to prove the complete look/reveal -> choice -> chosen-card action.
    let mut flattened = Vec::new();
    collect_visible_hand_pipeline_effects(effects, &mut flattened);

    let compact = describe_look_hand_choose_then_discard_or_exile(&flattened)
        .or_else(|| describe_reveal_hand_subset_choose_then_discard(&flattened))
        .or_else(|| describe_reveal_hand_choose_two_filters_then_discard(&flattened))
        .or_else(|| describe_look_hand_choose_then_discard(&flattened))?;

    let implicit_target = effects.iter().find_map(|effect| {
        structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
            .filter(|target| !target.explicit_declaration)
    });
    if let Some(target) = implicit_target
        && let Some(look) = flattened
            .first()
            .and_then(|effect| effect.downcast_ref::<crate::effects::LookAtHandEffect>())
    {
        let ordinary_subject = capitalize_first(&describe_choose_spec(&look.target));
        let target_subject = capitalize_first(&describe_choose_spec(&ChooseSpec::target(
            target.target.clone(),
        )));
        if let Some(rest) = compact.strip_prefix(&ordinary_subject) {
            return Some(format!("{target_subject}{rest}"));
        }
    }

    Some(compact)
}

/// Recognize a complete three-effect revealed-hand choice pipeline at the
/// start of a longer resolution program. The linked chooser, hand owner, and
/// chosen-card consumer are all proven by the existing complete-pipeline
/// matcher; anything after those three effects remains an independent source
/// sentence instead of being folded into the hand procedure.
fn describe_hand_pipeline_prefix(effects: &[Effect]) -> Option<(String, usize)> {
    let prefix = effects.get(..3)?;
    let compact = describe_sequence_wrapped_hand_pipeline(prefix)?;
    Some((compact, prefix.len()))
}

fn describe_hand_pipeline_then_leading_conditional(effects: &[Effect]) -> Option<String> {
    let (conditional_effect, pipeline_effects) = effects.split_last()?;
    let conditional = structural_unwrap_render_wrappers(conditional_effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    if conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf {
        return None;
    }
    let pipeline = describe_sequence_wrapped_hand_pipeline(pipeline_effects)?;
    let conditional_text = describe_effect(conditional_effect);
    let (condition, branch) = conditional_text.split_once(", ")?;
    if !condition.starts_with("If ") {
        return None;
    }
    let branch = branch
        .strip_prefix("you ")
        .or_else(|| branch.strip_prefix("You "))
        .map(normalize_you_verb_phrase)
        .unwrap_or_else(|| branch.to_string());
    Some(format!(
        "{}. {condition}, {branch}",
        pipeline.trim_end_matches('.')
    ))
}

pub(crate) fn describe_effect_clause_list(effects: &[Effect]) -> Option<String> {
    if let Some(compact) = describe_delegated_subset_with_hand_remainder(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_delegated_collection_partition_moves(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_delegated_subset_choice(effects) {
        return Some(lowercase_first(&compact));
    }
    if let [effect] = effects
        && let Some(compact) = describe_delegated_collection_complement_move(effect)
    {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_target_must_be_blocked_same_tag(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_put_counters_then_conditional_animation(effects) {
        return Some(lowercase_first(&compact));
    }
    if let [target, grant, pump] = effects
        && let Some(compact) =
            describe_shared_declared_target_grant_then_pt_pump(target, grant, pump)
    {
        return Some(lowercase_first(&compact));
    }
    if let [target, first, second] = effects
        && let Some(compact) = describe_shared_target_trample_mana_value_pump(target, first, second)
    {
        return Some(lowercase_first(&compact));
    }
    if let Some((prefix, consumed)) = describe_target_relative_combat_set_prefix(effects) {
        let prefix = lowercase_first(&prefix);
        if consumed == effects.len() {
            return Some(prefix);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return Some(format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }
    if let [permission, free_cast] = effects
        && let Some(compact) = describe_play_permission_then_free_cast_join(permission, free_cast)
    {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_repeated_explore_pair(effects) {
        return Some(lowercase_first(&compact));
    }
    if let [choose_effect, phase_out_effect] = effects
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseCardTypeEffect>()
        && let Some(phase_out) = phase_out_effect.downcast_ref::<crate::effects::PhaseOutEffect>()
        && let Some(compact) = describe_choose_type_then_phase_out(choose, phase_out)
    {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_copy_spell_with_characteristic_modifiers(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_discard_then_draw_for_discarded(effects) {
        return Some(compact);
    }
    let direct_refs = effects.iter().collect::<Vec<_>>();
    if let Some(compact) = describe_graveyard_creature_pile_exile_return_bundle(&direct_refs) {
        // This exact producer/disposition bundle has an authored sentence
        // boundary. Recognize it before clause fallback renders the selected
        // pile as an internal tag reference.
        return Some(compact);
    }
    if let Some(compact) = describe_prior_effect_dynamic_count_token_bundle(&direct_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some((compact, consumed)) = describe_looked_card_selected_partition(&direct_refs)
        && consumed == effects.len()
    {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_choose_x_permanents_create_x_copies(&direct_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_choose_exiled_card_then_play_without_paying(effects) {
        return Some(lowercase_first(&compact));
    }
    let consult_refs = effects.iter().collect::<Vec<_>>();
    if let Some(compact) = describe_consult_match_destination_alternative(&consult_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_consult_conditional_may_cast_remainder_bottom(&consult_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_consult_exile_may_cast_else_your_hand(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_sequence_wrapped_search_two_split(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_hand_pipeline_then_leading_conditional(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some((prefix, consumed)) = describe_hand_pipeline_prefix(effects) {
        let prefix = lowercase_first(&prefix);
        if consumed == effects.len() {
            return Some(prefix);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return Some(format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }
    if let Some(compact) = describe_sequence_wrapped_hand_pipeline(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = render_reveal_hand_choose_same_name_exile_shuffle(&direct_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = render_choose_name_search_same_name_exile_shuffle(&direct_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_countered_spell_same_name_search_sequence(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_target_card_same_name_extraction(&direct_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_exile_target_search_same_name_exile_shuffle_bundle(&direct_refs)
    {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_reveal_hand_optional_choice_discard_else_exile(&direct_refs) {
        return Some(lowercase_first(&compact));
    }
    if effects.len() < 2 {
        return None;
    }
    if let [with_id_effect, for_players_effect] = effects
        && let Some(with_id) = with_id_effect.downcast_ref::<crate::effects::WithIdEffect>()
        && let Some(for_players) =
            for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()
        && let Some(compact) = describe_with_id_then_for_players_if_didnt(with_id, for_players)
    {
        return Some(lowercase_first(&compact));
    }
    if let [look_effect, exile_effect, grant_effect] = effects
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(exile) = exile_effect.downcast_ref::<crate::effects::ExileEffect>()
        && let Some(grant) = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
        && let Some(compact) =
            describe_look_at_top_exile_face_down_then_play_while_exiled(look_at_top, exile, grant)
    {
        // Trigger/spell resolution enters through the clause renderer, whose
        // generic fallback joins three effects as "A, B, then C". Keep the
        // typed look/exile/permission bundle's authored boundary instead:
        // the exile follows the look, while the persistent permission is a
        // separate sentence.
        return Some(lowercase_first(&compact));
    }
    if let Some((prefix, consumed)) = describe_untap_then_phase_out_until_source_leaves(effects) {
        let prefix = lowercase_first(&prefix);
        if consumed == effects.len() {
            return Some(prefix);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return Some(format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }
    if let Some((prefix, consumed)) = describe_damage_then_gain_life_this_way(effects) {
        let prefix = lowercase_first(&prefix);
        if consumed == effects.len() {
            return Some(prefix);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return Some(format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }
    if let Some(compact) = describe_each_player_reveal_filtered_token_then_pump_then_draw(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) =
        describe_consult_reveal_triggering_creature_pump_then_move_revealed(effects)
    {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_attach_all_enchanting_target_to_same_controller(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_targeted_attachment_instruction(effects) {
        return Some(lowercase_first(&compact));
    }
    // Trigger and spell resolution normally enter through the clause-list
    // renderer. Preserve this typed shared-target procedure before the
    // synthetic-target folds split its payment, reveal, and selection into
    // unrelated instructions.
    if let Some(compact) = describe_pay_life_reveal_hand_choose_exile_effects(effects) {
        return Some(lowercase_first(&compact));
    }
    let raw_effects = effects.iter().collect::<Vec<_>>();
    if let Some((compact, consumed)) = describe_revealed_top_choose_one_graveyard(&raw_effects) {
        let compact = lowercase_first(&compact);
        if consumed == effects.len() {
            return Some(compact);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }
    if let Some(compact) = describe_linked_exile_top_play_clause(effects) {
        return Some(lowercase_first(&compact));
    }
    if let [first, second] = effects
        && let Some(compact) = describe_must_block_then_control_block_assignments(first, second)
    {
        return Some(lowercase_first(&compact));
    }
    if let [target_effect, draw_effect, lose_effect] = effects
        && let Some(target_only) = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()
        && let Some(draw) = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()
        && let Some(lose) = lose_effect.downcast_ref::<crate::effects::LoseLifeEffect>()
        && let Some(compact) = describe_target_player_draw_then_lose_life(draw, target_only, lose)
    {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_single_consumer_synthetic_target_fold(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_multi_consumer_synthetic_target_declaration(effects) {
        return Some(lowercase_first(&compact));
    }
    if target_only_pair_can_fold(effects, &effects[0])
        && let Some(compact) = describe_redundant_target_only_pair(&effects[..2])
    {
        if effects.len() == 2 {
            return Some(lowercase_first(&compact));
        }
        return describe_effect_clause_list(&effects[1..])
            .or_else(|| Some(lowercase_first(&describe_effect_list(&effects[1..]))));
    }
    if let Some(compact) = describe_linked_counter_followup(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_typed_counter_sentence_split(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_optional_search_battlefield_partition_effects(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_discard_redraw_mana_value_ladder(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_look_hand_optional_exile_persistent_play_tax(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_target_exile_persistent_owner_play_tax(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_hidden_exile_partition_with_persistent_permission(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_each_opponent_top_card_hidden_exile_permission(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_exile_all_then_each_player_may_deploy_and_return_exiled(effects)
    {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_exile_top_play_then_additional_land(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_exile_two_creatures_then_controller_consults(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_exile_top_then_search_to_hand_and_shuffle(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_two_target_players_each_search_to_top(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_search_reveal_nested_may_move_else_hand(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_bulk_battlefield_move_then_grant_decayed(effects) {
        return Some(lowercase_first(&compact));
    }
    // Trigger and spell resolution normally enters through the clause-list
    // renderer. Recognize the complete reveal-until partition before the
    // generic clause joiner exposes its internal tagged iterations as "for
    // each of those objects" / "unless it's a permanent" scaffolding.
    if effects.len() >= 3 {
        let consult_refs = effects[..3].iter().collect::<Vec<_>>();
        if let Some(compact) = render_consult_reveal_put_battlefield_rest_graveyard(&consult_refs) {
            let compact = lowercase_first(&compact);
            if effects.len() == 3 {
                return Some(compact);
            }
            let suffix = describe_effect_clause_list(&effects[3..])
                .unwrap_or_else(|| describe_effect_list(&effects[3..]));
            return Some(format!(
                "{}. {}",
                compact.trim_end_matches('.'),
                capitalize_first(suffix.trim_end_matches('.'))
            ));
        }
    }
    if let Some(compact) = describe_exile_top_choose_one_play_next_turn(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_each_player_reveal_set_may_move_else_draw(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_consult_characteristic_boost_then_all_revealed_bottom(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_consult_reflexive_damage_then_all_revealed_bottom(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_energy_payment_failure_fallback(effects) {
        return Some(lowercase_first(&compact));
    }
    if let [first, second] = effects
        && let Some(compact) = describe_action_and_get_energy_pair(first, second)
    {
        return Some(lowercase_first(&compact));
    }

    if let Some(compact) = describe_milled_creatures_returned_then_animated(effects) {
        return Some(lowercase_first(&compact));
    }

    // Spell and ability resolution prefers the clause-list renderer, so run
    // compound target-plus-linked-set prefixes here before the generic pair
    // renderer consumes only their first two visible effects. This preserves
    // the semantic union tag for follow-up clauses such as "those creatures",
    // "with that name", and event-count references.
    if let Some((compact, consumed)) = describe_linked_target_set_followup_prefix(effects)
        .or_else(|| describe_same_name_exile_then_investigate_prefix(effects))
        .or_else(|| describe_target_same_name_action_fanout_prefix(effects))
    {
        let compact = lowercase_first(&compact);
        if consumed == effects.len() {
            return Some(compact);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    if let Some(compact) = describe_returned_object_set_to_enchantment(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_returned_object_exact_types_with_quoted_ability(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_returned_battlefield_object_then_animated(effects) {
        return Some(lowercase_first(&compact));
    }

    // An authored blink clause is one compound instruction, while any
    // following effect is a new sentence. Preserve that boundary before the
    // generic clause joiner turns every visible effect into a comma chain.
    if effects.len() >= 2
        && let Some(tagged_exile) = effects[0].downcast_ref::<crate::effects::TaggedEffect>()
        && let Some(move_back) = unwrap_basic_tag_wrappers(&effects[1])
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
        && let Some(prefix) = describe_exile_then_return(tagged_exile, move_back)
    {
        let prefix = lowercase_first(&prefix);
        if effects.len() == 2 {
            return Some(prefix);
        }
        let suffix = describe_effect_clause_list(&effects[2..])
            .unwrap_or_else(|| describe_effect_list(&effects[2..]));
        let suffix = capitalize_first(suffix.trim_end_matches('.'));
        let suffix = suffix
            .strip_prefix("You ")
            .map(capitalize_first)
            .unwrap_or(suffix);
        return Some(format!("{}. {}", prefix.trim_end_matches('.'), suffix));
    }

    if let Some(compact) = describe_optional_look_then_reveal_top_rest_bottom(effects) {
        return Some(compact);
    }

    if let Some((compact, consumed)) = describe_typed_collection_selection_prefix(effects) {
        if consumed == effects.len() {
            return Some(compact);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        let suffix = normalize_imperative_you_clause(suffix.trim_end_matches('.'));
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(&suffix)
        ));
    }

    if let Some((compact, consumed)) = describe_looked_cards_clause_prefix(effects) {
        if consumed == effects.len() {
            return Some(compact);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    let early_refs = effects.iter().collect::<Vec<_>>();
    if let Some(compact) = describe_look_hand_choose_then_discard(&early_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_player_damage_then_same_player_discards(&early_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_target_player_sacrifice_then_gain_toughness(&early_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_reveal_hand_then_gain_for_that_players_hand(&early_refs) {
        return Some(lowercase_first(&compact));
    }
    // Preserve the typed revealed-card pool through its selection, movement,
    // and remainder disposition before the generic target-player reveal
    // prefix splits the program into unrelated clauses.
    if effects.len() >= 5
        && let Some(compact) =
            describe_target_player_reveal_top_may_put_matching_rest_bottom(&effects[..5])
    {
        let compact = lowercase_first(&compact);
        if effects.len() == 5 {
            return Some(compact);
        }
        let suffix = describe_effect_clause_list(&effects[5..])
            .unwrap_or_else(|| describe_effect_list(&effects[5..]));
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }
    if effects.len() >= 3
        && structural_unwrap_render_wrappers(&effects[0])
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
            .is_some()
        && let Some(observation_prefix) =
            describe_target_player_reveal_top(&effects[0], &effects[1])
    {
        let observed_refs = effects[1..].iter().collect::<Vec<_>>();
        if let Some((mut compact, consumed)) =
            describe_immediate_observation_conditionals(&observed_refs)
        {
            if let Some((_, remainder)) = compact.split_once(". ") {
                compact = format!("{observation_prefix}. {remainder}");
            }
            let consumed = consumed + 1;
            if consumed == effects.len() {
                return Some(lowercase_first(&compact));
            }
            let suffix = describe_effect_clause_list(&effects[consumed..])
                .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
            return Some(format!(
                "{}. {}",
                lowercase_first(compact.trim_end_matches('.')),
                capitalize_first(suffix.trim_end_matches('.'))
            ));
        }
    }
    if effects.len() >= 2
        && let Some(prefix) = describe_target_player_reveal_top(&effects[0], &effects[1])
    {
        if effects.len() == 2 {
            return Some(lowercase_first(&prefix));
        }
        let suffix = describe_effect_clause_list(&effects[2..])
            .unwrap_or_else(|| describe_effect_list(&effects[2..]));
        return Some(format!(
            "{}. {}",
            lowercase_first(&prefix),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    // Resolution programs prefer the clause-list renderer over
    // `describe_effect_list`, so structural prefixes that only live in the
    // latter never get a chance to run for ordinary spell and ability text.
    // Match the full control/untap/haste bundle before its two-effect prefix,
    // or the haste grant loses the shared object reference and conjunction.
    if effects.len() >= 3
        && let Some(bundle) = describe_gain_control_untap_haste_clause_structural(&effects[..3])
    {
        let bundle = lowercase_first(&bundle);
        if effects.len() == 3 {
            return Some(bundle);
        }
        let suffix = describe_effect_clause_list(&effects[3..])
            .unwrap_or_else(|| describe_effect_list(&effects[3..]));
        let suffix = normalize_imperative_you_clause(suffix.trim_end_matches('.'));
        return Some(format!(
            "{}. {}",
            bundle.trim_end_matches('.'),
            capitalize_first(&suffix)
        ));
    }
    if effects.len() >= 3
        && let Some(bundle) = describe_gain_control_untap_haste_structural(&effects[..3])
    {
        let bundle = lowercase_first(&bundle);
        if effects.len() == 3 {
            return Some(bundle);
        }
        let suffix = describe_effect_clause_list(&effects[3..])
            .unwrap_or_else(|| describe_effect_list(&effects[3..]));
        let suffix = normalize_imperative_you_clause(suffix.trim_end_matches('.'));
        return Some(format!(
            "{}. {}",
            bundle.trim_end_matches('.'),
            capitalize_first(&suffix)
        ));
    }

    // Keep the reusable two-effect control/untap recognizer at the real
    // dispatch point after longer structural bundles have declined.
    if effects.len() >= 2
        && let Some(prefix) = describe_gain_control_then_untap_structural(&effects[..2])
    {
        let prefix = lowercase_first(&prefix);
        if effects.len() == 2 {
            return Some(prefix);
        }
        let suffix = describe_effect_clause_list(&effects[2..])
            .unwrap_or_else(|| describe_effect_list(&effects[2..]));
        let suffix = normalize_imperative_you_clause(suffix.trim_end_matches('.'));
        return Some(format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(&suffix)
        ));
    }

    let bundle_refs = effects.iter().collect::<Vec<_>>();
    let visible_refs = bundle_refs
        .iter()
        .copied()
        .filter(|effect| {
            effect
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_none()
                && effect
                    .downcast_ref::<crate::effects::TagTriggeringSourceEffect>()
                    .is_none()
                && effect
                    .downcast_ref::<crate::effects::TagTriggeringBlockersEffect>()
                    .is_none()
        })
        .collect::<Vec<_>>();
    if let Some(compact) = describe_same_name_reference_search_bundle(&visible_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_single_hand_reveal_same_name_search(&visible_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_target_card_same_name_extraction(&visible_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_target_creature_damage_then_destroy_attached(&visible_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_destroy_target_creature_then_owner_gains(&visible_refs) {
        return Some(lowercase_first(&compact));
    }
    if let [first, second] = visible_refs.as_slice()
        && let Some(compact) = describe_target_continuous_fanout_pair(first, second)
            .or_else(|| describe_target_prevention_fanout_pair(first, second))
    {
        return Some(lowercase_first(&compact));
    }
    if let [first, second] = visible_refs.as_slice()
        && let Some(compact) = describe_target_creature_damage_fanout_pair(first, second)
    {
        return Some(lowercase_first(&compact));
    }
    if visible_refs.len() >= 2
        && let Some(compact) =
            describe_target_same_name_action_fanout_pair(visible_refs[0], visible_refs[1])
    {
        let compact = lowercase_first(&compact);
        if visible_refs.len() == 2 {
            return Some(compact);
        }
        let suffix = visible_refs[2..]
            .iter()
            .map(|effect| describe_effect(effect).trim_end_matches('.').to_string())
            .collect::<Vec<_>>()
            .join(". ");
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }
    if let Some(compact) = describe_look_hand_choose_then_discard(&visible_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_target_player_look_top_may_move_that_card(&visible_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_target_player_consult_exile_shuffle_may_cast(&visible_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_choose_name_reveal_discard_failure_draw_bundle(&visible_refs) {
        return Some(lowercase_first(&compact));
    }
    let search_sequence_refs = if matches!(
        visible_refs.first(),
        Some(effect) if effect.downcast_ref::<crate::effects::TargetOnlyEffect>().is_some()
    ) {
        &visible_refs[1..]
    } else {
        visible_refs.as_slice()
    };
    if let [sequence_effect, shuffle_effect] = search_sequence_refs
        && let Some(sequence) = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()
        && let Some(shuffle) = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && let Some(compact) = describe_search_sequence_then_shuffle(sequence, shuffle)
    {
        return Some(lowercase_first(&compact));
    }
    if visible_refs.len() >= 2
        && let Some(compact) = describe_source_exile_with_counters_pair(
            visible_refs[visible_refs.len() - 2],
            visible_refs[visible_refs.len() - 1],
        )
    {
        let compact = lowercase_first(&compact);
        if visible_refs.len() == 2 {
            return Some(compact);
        }
        let prefix_effects = &effects[..effects.len() - 2];
        let prefix = describe_effect_clause_list(prefix_effects)
            .unwrap_or_else(|| lowercase_first(&describe_effect_list(prefix_effects)));
        return Some(format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(&compact)
        ));
    }
    if let [choose_effect, move_effect, shuffle_effect, cast_effect] = visible_refs.as_slice()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(shuffle) = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && let Some(compact) =
            describe_search_choose_then_exile_and_cast(choose, move_effect, shuffle, cast_effect)
    {
        return Some(cleanup_decompiled_text(&lowercase_first(&compact)));
    }
    if let [choose_effect, cast_effect, shuffle_effect] = search_sequence_refs
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(shuffle) = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && let Some(compact) =
            describe_search_choose_then_cast_then_shuffle(choose, cast_effect, shuffle)
    {
        return Some(cleanup_decompiled_text(&lowercase_first(&compact)));
    }
    if let Some(compact) = describe_choose_each_basic_land_type_then_destroy(&visible_refs) {
        return Some(compact);
    }
    if let Some(compact) =
        describe_may_cast_target_graveyard_spell_then_exile_replacement(&visible_refs)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_hand_then_same_player_discards(&visible_refs) {
        return Some(compact);
    }
    if let Some((compact, consumed)) =
        describe_same_referenced_player_action_sequence(&visible_refs)
        && consumed == visible_refs.len()
    {
        return Some(lowercase_first(&compact));
    }
    if let [for_players_effect, destroy_effect] = visible_refs.as_slice()
        && let Some(for_players) =
            for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()
        && let Some(destroy) = unwrap_basic_tag_wrappers(destroy_effect)
            .downcast_ref::<crate::effects::DestroyEffect>()
        && let Some(compact) =
            describe_for_players_may_choose_then_destroy_chosen(for_players, destroy)
    {
        return Some(compact);
    }
    if let [choose_effect, reveal_effect, move_effect, shuffle_effect] = visible_refs.as_slice()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(reveal) = reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()
        && let Some(move_to_zone) = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
        && let Some(shuffle) = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && let Some(compact) =
            describe_search_choose_then_move(choose, Some(reveal), move_to_zone, Some(shuffle))
    {
        return Some(cleanup_decompiled_text(&lowercase_first(&compact)));
    }
    if let Some(compact) = describe_look_exile_one_rest_bottom_cast_else_hand(&bundle_refs) {
        return Some(compact);
    }
    if let [for_players_effect, look_effect, grant_effect] = bundle_refs.as_slice()
        && let Some(for_players) =
            for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()
        && let Some(look) = look_effect.downcast_ref::<crate::effects::LookAtObjectsEffect>()
        && let Some(grant) = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
        && let Some(compact) =
            describe_for_players_bottom_library_exile_then_look_cast(for_players, look, grant)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_exile_target_search_same_name_exile_shuffle_bundle(&bundle_refs)
    {
        return Some(compact);
    }
    let is_reference_search_bundle = match effects {
        [exile, for_each, shuffle] => {
            exile
                .downcast_ref::<crate::effects::TaggedEffect>()
                .is_some()
                && (for_each
                    .downcast_ref::<crate::effects::ForEachObject>()
                    .is_some()
                    || for_each
                        .downcast_ref::<crate::effects::ForEachTaggedEffect>()
                        .is_some())
                && shuffle
                    .downcast_ref::<crate::effects::ShuffleLibraryEffect>()
                    .is_some()
        }
        [look, choose, exile, for_each, shuffle] => {
            look.downcast_ref::<crate::effects::LookAtHandEffect>()
                .is_some()
                && choose
                    .downcast_ref::<crate::effects::ChooseObjectsEffect>()
                    .is_some()
                && exile
                    .downcast_ref::<crate::effects::MoveToZoneEffect>()
                    .is_some()
                && (for_each
                    .downcast_ref::<crate::effects::ForEachObject>()
                    .is_some()
                    || for_each
                        .downcast_ref::<crate::effects::ForEachTaggedEffect>()
                        .is_some())
                && shuffle
                    .downcast_ref::<crate::effects::ShuffleLibraryEffect>()
                    .is_some()
        }
        _ => false,
    };
    if is_reference_search_bundle {
        let compact = describe_effect_list(effects);
        if compact.starts_with(
            "Exile all cards from target player's graveyard other than basic land cards",
        ) || compact.starts_with(
            "Target opponent reveals their hand. Choose up to X nonland cards from it and exile them",
        ) {
            return Some(compact);
        }
    }
    if let Some(compact) = describe_reveal_hand_choose_graveyard_exile_bundle(&bundle_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_choose_name_reveal_hand_discard_named_bundle(&bundle_refs) {
        return Some(compact);
    }
    if let Some(reveal_line) =
        describe_choose_hand_then_reveal_chosen_pair(&effects[0], &effects[1])
    {
        if effects.len() == 2 {
            return Some(reveal_line);
        }
        let rest = describe_effect_clause_list(&effects[2..])
            .unwrap_or_else(|| describe_effect_list(&effects[2..]));
        if !rest.trim().is_empty() {
            return Some(format!(
                "{reveal_line}. {}",
                capitalize_first(rest.trim_end_matches('.'))
            ));
        }
        return Some(reveal_line);
    }

    if let [may_effect, shuffle_effect] = effects
        && let Some(may) = may_effect.downcast_ref::<crate::effects::MayEffect>()
        && may.decider.is_none()
        && let Some(shuffle) = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && let Some(compact) = describe_may_search_choose_for_each_with_shuffle(may, shuffle)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_destroy_all_groups_then_draw_for_destroyed(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_return_as_aura_with_granted_abilities(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_exile_source_and_target(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_target_permanent_shuffle_reveal_permanent_card(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_choose_color_target_and_shared_color_protection(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_target_and_shared_color_inline_ability_grant(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_look_reorder_then_may_shuffle(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_target_modifications_then_exile_top_play(effects) {
        return Some(compact);
    }

    let effect_refs = effects.iter().collect::<Vec<_>>();
    if effects.len() >= 4
        && let Some(look_at_top) = effects[0].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = effects[1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(remainder) =
            effects[3].downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()
        && let Some(compact) = describe_look_at_top_choose_battlefield_rest_bottom(
            look_at_top,
            None,
            choose,
            &effects[2],
            remainder,
        )
    {
        if effects.len() == 4 {
            return Some(compact);
        }
        let rest = describe_effect_clause_list(&effects[4..])
            .unwrap_or_else(|| describe_effect_list(&effects[4..]));
        return Some(format!(
            "{compact}. {}",
            capitalize_first(rest.trim_end_matches('.'))
        ));
    }
    if effects.len() >= 3
        && let Some(compact) =
            describe_for_players_choose_move_then_combined_characteristics(&effect_refs[..3])
    {
        if effects.len() == 3 {
            return Some(compact);
        }
        let rest = describe_effect_clause_list(&effects[3..])
            .unwrap_or_else(|| describe_effect_list(&effects[3..]));
        return Some(format!(
            "{compact}. {}",
            capitalize_first(rest.trim_end_matches('.'))
        ));
    }
    if effects.len() >= 5
        && let Some(compact) =
            describe_for_players_choose_move_then_characteristics(&effect_refs[..5])
    {
        if effects.len() == 5 {
            return Some(compact);
        }
        let rest = describe_effect_clause_list(&effects[5..])
            .unwrap_or_else(|| describe_effect_list(&effects[5..]));
        return Some(format!(
            "{compact}. {}",
            capitalize_first(rest.trim_end_matches('.'))
        ));
    }
    if effects.len() >= 3
        && let Some(compact) =
            describe_consult_may_cast_remainder_bottom_sequence(&effect_refs[..3])
    {
        if effects.len() == 3 {
            return Some(compact);
        }
        let rest = describe_effect_clause_list(&effects[3..])
            .unwrap_or_else(|| describe_effect_list(&effects[3..]));
        return Some(format!(
            "{compact}. {}",
            capitalize_first(rest.trim_end_matches('.'))
        ));
    }
    if effects.len() >= 3
        && let Some(compact) =
            describe_consult_exile_may_cast_rest_bottom_sequence(&effect_refs[..3])
    {
        if effects.len() == 3 {
            return Some(compact);
        }
        let rest = describe_effect_clause_list(&effects[3..])
            .unwrap_or_else(|| describe_effect_list(&effects[3..]));
        return Some(format!(
            "{compact}. {}",
            capitalize_first(rest.trim_end_matches('.'))
        ));
    }
    if effects.len() > 3
        && let Some(prefix) = describe_choose_top_exile_then_play_structural(&effects[..3])
    {
        let rest = describe_effect_clause_list(&effects[3..])
            .unwrap_or_else(|| describe_effect_list(&effects[3..]));
        return Some(format!(
            "{prefix}. Then {}",
            lowercase_first(rest.trim_end_matches('.'))
        ));
    }
    if let Some(compact) = describe_choose_top_exile_then_play_structural(effects) {
        return Some(compact);
    }
    if effects.len() > 3
        && let Some(suffix) =
            describe_choose_top_exile_then_play_structural(&effects[effects.len() - 3..])
    {
        let prefix = describe_effect_clause_list(&effects[..effects.len() - 3])
            .unwrap_or_else(|| describe_effect_list(&effects[..effects.len() - 3]));
        return Some(format!("{}. {suffix}", prefix.trim_end_matches('.')));
    }
    if effects.len() >= 3
        && let Some(exile_top) =
            effects[0].downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
        && let Some(choose) = effects[1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(grant_play) = effects[2].downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
        && let Some(prefix) = describe_exile_top_choose_one_then_play(exile_top, choose, grant_play)
    {
        if effects.len() == 3 {
            return Some(prefix);
        }
        let rest = describe_effect_clause_list(&effects[3..])
            .unwrap_or_else(|| describe_effect_list(&effects[3..]));
        return Some(format!(
            "{prefix}. Then {}",
            lowercase_first(rest.trim_end_matches('.'))
        ));
    }
    if let Some(compact) =
        describe_sacrifice_return_from_graveyard_then_exile_source_bundle(effects)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_chosen_creatures_blessing_additional_combat_clause(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_power_cards_for_mana_clause_bundle(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_gain_life_shuffle_source_and_graveyard(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_untap_triggering_then_remove_from_combat(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_remove_counter_then_no_counters_conditional(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_linked_graveyard_choices_then_may_return_bundle(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_graveyard_mana_ladder_return_clause_bundle(&effect_refs) {
        return Some(compact);
    }
    if let [first, second] = effects
        && let Some(compact) = describe_put_counters_then_untap_them(first, second)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_return_then_color_subtype_addition_compact(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_countered_spell_same_name_search_sequence(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_countered_spell_controller_consult_cast_shuffle(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_damage_each_then_tap_damaged_sequence(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_exile_source_and_attacking_nonflying_creature(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_exile_source_and_target(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_choose_two_tap_then_unattach_equipment_sequence(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_sacrifice_then_sacrificed_conditional_sequence(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_gain_control_create_token_attach_sequence(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_create_token_then_grant_same_tag(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_moved_object_haste_delayed_cleanup(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_pump_all_then_change_all_subtypes_same_filter(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_pump_all_then_grant_same_filter(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_put_counters_then_grant_same_filter(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_draw_count_then_grant_same_filter(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_continuous_choose_attach_sequence(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_return_each_subtype_card_from_your_graveyard(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_random_choose_then_destroy_rest(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_search_two_split_hand_graveyard_sequence(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_hand_choose_two_filters_then_discard(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_discard_reveal_hand_choose_discard_chosen(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_choose_color_reveal_hand_discard_that_color(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_target_player_choose_hand_top_library_any_order(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_hand_choose_then_library_placement(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_hand_then_gain_for_that_players_hand(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_hand_choose_graveyard_or_hand_exile(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_hand_choose_discard_then_scry(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_hand_choose_discard_then_adventure_move(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_hand_choose_gain_toughness_then_discard(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_look_hand_choose_then_discard_or_exile(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_life_lock_and_protection_from_everything(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_player_protection_from_everything_pair(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_choose_color_then_chosen_color_mana(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_power_damage_exchange_clause(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_tagged_for_each_then_apply_continuous(&effect_refs) {
        return Some(compact);
    }
    if effects.len() >= 4
        && let Some(look_at_top) = effects[0].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = effects[1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some((_, move_to_hand)) = for_each_tagged_for_compaction(&effects[2])
        && let Some((_, rest)) = for_each_tagged_for_compaction(&effects[3])
        && let Some(compact) = describe_look_at_top_then_put_into_hand_rest_graveyard(
            look_at_top,
            None,
            choose,
            None,
            move_to_hand,
            rest,
        )
    {
        if effects.len() == 4 {
            return Some(compact);
        }
        let rest = describe_effect_clause_list(&effects[4..])
            .unwrap_or_else(|| describe_effect_list(&effects[4..]));
        return Some(format!("{compact}. {}", capitalize_first(&rest)));
    }
    if effects.len() >= 4
        && let Some(look_at_top) = effects[0].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = effects[1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some((_, move_chosen)) = for_each_tagged_for_compaction(&effects[2])
        && let Some((_, rest)) = for_each_tagged_for_compaction(&effects[3])
        && let Some(compact) = describe_look_at_top_then_put_matching_to_zone_rest_hand(
            look_at_top,
            None,
            choose,
            move_chosen,
            rest,
        )
    {
        if effects.len() == 4 {
            return Some(compact);
        }
        let rest = describe_effect_clause_list(&effects[4..])
            .unwrap_or_else(|| describe_effect_list(&effects[4..]));
        return Some(format!("{compact}. {}", capitalize_first(&rest)));
    }
    if let Some(compact) = describe_choose_two_move_one_put_counters_on_other(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_choose_same_controller_sacrifice_one_return_other(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_choose_exiled_cards_exile_library_put_chosen_on_top(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_choose_two_sacrifice_one_return_other(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_choose_sacrifice_power_damage_each(effects) {
        return Some(compact);
    }
    if let [choose_effect, return_effect, counter_effect] = effects
        && let Some(compact) = describe_choose_then_return_from_graveyard_with_counters(
            choose_effect,
            return_effect,
            counter_effect,
        )
    {
        return Some(compact);
    }
    if let Some(compact) = describe_return_from_graveyard_with_counters(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_move_to_battlefield_with_additional_counters(effects) {
        return Some(compact);
    }
    if let [destroy_effect, target_effect, search_effect, shuffle_effect] = effects
        && let Some(compact) =
            describe_destroy_then_target_opponent_search_to_graveyard_then_shuffle(
                destroy_effect,
                target_effect,
                search_effect,
                shuffle_effect,
            )
    {
        return Some(compact);
    }
    if let [destroy_effect, search_effect, shuffle_effect] = effects
        && let Some(compact) =
            describe_destroy_then_search_target_opponent_to_graveyard_then_shuffle(
                destroy_effect,
                search_effect,
                shuffle_effect,
            )
    {
        return Some(compact);
    }

    if effects.len() >= 2
        && let Some(first) = describe_target_then_look_at_tagged_object(&effect_refs[..2])
    {
        if effects.len() == 2 {
            return Some(first);
        }
        let rest = describe_effect_clause_list(&effects[2..])
            .unwrap_or_else(|| lowercase_first(&describe_effect_list(&effects[2..])));
        return Some(format!("{first}. {rest}"));
    }

    if effects.len() >= 3
        && let Some(hand) = effects[0].downcast_ref::<crate::effects::LookAtHandEffect>()
        && !hand.reveal
        && hand.target == ChooseSpec::target_player()
        && let Some(top) = effects[1].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && !top.reveal
        && top.count == Value::Fixed(1)
        && top.player == PlayerFilter::target_player()
        && let Some(objects) = effects[2].downcast_ref::<crate::effects::LookAtObjectsEffect>()
        && objects.viewer == PlayerFilter::You
        && objects.subject == PlayerFilter::target_player()
        && objects.filter
            == ObjectFilter::creature()
                .face_down()
                .controlled_by(PlayerFilter::target_player())
    {
        let first = "look at target player's hand, the top card of that player's library, and any face-down creatures they control";
        if effects.len() == 3 {
            return Some(first.to_string());
        }
        let rest = describe_effect_clause_list(&effects[3..])
            .unwrap_or_else(|| lowercase_first(&describe_effect_list(&effects[3..])));
        return Some(format!("{first}. {rest}"));
    }

    if let Some(compact) = describe_structural_multisentence_effect_list(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_leading_selection_then_draw_sequence(effects) {
        return Some(compact);
    }

    // "you and that player each gain that much life" — joint-subject life
    // gain pair (see the matching compaction in describe_effect_list).
    if let [first, second] = effects
        && let Some(first_gain) =
            unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::GainLifeEffect>()
        && let Some(second_gain) =
            unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::GainLifeEffect>()
        && first_gain.amount == second_gain.amount
        && matches!(&first_gain.player, ChooseSpec::Player(PlayerFilter::You))
        && let ChooseSpec::Player(second_player) = &second_gain.player
        && *second_player != PlayerFilter::You
    {
        let other = match second_player {
            PlayerFilter::DamagedPlayer | PlayerFilter::TaggedPlayer(_) => {
                "that player".to_string()
            }
            other => describe_player_filter(other),
        };
        return Some(format!(
            "you and {other} each gain {}",
            describe_life_amount_phrase(&first_gain.amount)
        ));
    }

    if let Some(compact) = describe_linked_exile_top_play_clause(effects) {
        return Some(compact);
    }

    let compact = describe_effect_list(effects);
    let compact_trimmed = compact.trim();
    if compact_trimmed.starts_with("Exile the bottom card of ")
        && compact_trimmed.contains("For as long as those cards remain exiled")
    {
        return Some(cleanup_decompiled_text(&lowercase_first(
            compact_trimmed.trim_end_matches('.'),
        )));
    }
    if compact_trimmed
        == "Reveal the top card of your library and put that card into your hand. You lose life equal to that card's mana value"
    {
        return Some(cleanup_decompiled_text(&lowercase_first(compact_trimmed)));
    }
    if clause_effects_have_typed_sentence_boundaries(&visible_refs) {
        return Some(cleanup_decompiled_text(&lowercase_first(
            compact_trimmed.trim_end_matches('.'),
        )));
    }
    if !compact_trimmed.is_empty()
        && !compact_trimmed.contains(". ")
        && !compact_trimmed.contains(": ")
        && !compact_trimmed.starts_with("If ")
        && !compact_trimmed.starts_with("When ")
        && !compact_trimmed.starts_with("Whenever ")
        && !compact_trimmed.starts_with("At ")
        && !compact_trimmed.starts_with("Choose ")
    {
        let normalized = normalize_imperative_you_clause(compact_trimmed.trim_end_matches('.'));
        return Some(cleanup_decompiled_text(&lowercase_first(&normalized)));
    }
    if !compact_trimmed.is_empty()
        && compact_trimmed.contains(". That ")
        && compact_trimmed.contains(" in addition to its other colors and types")
        && !compact_trimmed.starts_with("If ")
        && !compact_trimmed.starts_with("When ")
        && !compact_trimmed.starts_with("Whenever ")
        && !compact_trimmed.starts_with("At ")
    {
        return Some(cleanup_decompiled_text(
            compact_trimmed.trim_end_matches('.'),
        ));
    }
    if !compact_trimmed.is_empty()
        && compact_trimmed.contains(" until ")
        && compact_trimmed.contains(". Put ")
        && compact_trimmed.contains(" and the rest on the bottom of ")
        && !compact_trimmed.starts_with("If ")
        && !compact_trimmed.starts_with("When ")
        && !compact_trimmed.starts_with("Whenever ")
        && !compact_trimmed.starts_with("At ")
    {
        return Some(cleanup_decompiled_text(&lowercase_first(
            compact_trimmed.trim_end_matches('.'),
        )));
    }

    if let Some(compact) = describe_roll_die_then_scry_result(effects) {
        return Some(compact);
    }

    // Per-effect rendering that surfaces internal tag scaffolding is never
    // oracle-faithful; when the compaction-aware multi-sentence render in
    // describe_effect_list avoided that scaffolding, bail so callers use it.
    let compact_has_scaffolding =
        compact_trimmed.contains("tagged cards") || compact_trimmed.contains("tagged '");
    let mut parts = Vec::with_capacity(effects.len());
    let mut effect_idx = 0usize;
    while effect_idx < effects.len() {
        let effect = &effects[effect_idx];
        if effect_idx + 2 < effects.len()
            && let Some((joint, consumed)) =
                describe_selected_opponent_chosen_action(&effects[effect_idx..])
        {
            parts.push(lowercase_first(joint.trim_end_matches('.')));
            effect_idx += consumed;
            continue;
        }
        if effect_idx + 2 < effects.len()
            && let Some((joint, consumed)) =
                describe_primary_then_opponent_chosen_same_action(&effects[effect_idx..])
        {
            parts.push(lowercase_first(joint.trim_end_matches('.')));
            effect_idx += consumed;
            continue;
        }
        if effect_idx + 1 < effects.len()
            && let Some(joint) = describe_returned_battlefield_object_then_animated_pair(
                effect,
                &effects[effect_idx + 1],
            )
        {
            parts.push(lowercase_first(joint.trim_end_matches('.')));
            effect_idx += 2;
            continue;
        }
        if effect_idx + 1 < effects.len()
            && let Some(joint) =
                describe_opponent_chosen_target_action_join(effect, &effects[effect_idx + 1])
        {
            parts.push(lowercase_first(joint.trim_end_matches('.')));
            effect_idx += 2;
            continue;
        }
        if effect_idx + 1 < effects.len()
            && let Some(joint) =
                describe_play_permission_then_free_cast_join(effect, &effects[effect_idx + 1])
        {
            parts.push(lowercase_first(joint.trim_end_matches('.')));
            effect_idx += 2;
            continue;
        }
        if effect_idx + 1 < effects.len()
            && let Some(joint) =
                describe_choose_then_return_from_graveyard(effect, &effects[effect_idx + 1])
        {
            let joint = joint
                .strip_prefix("you ")
                .map(normalize_you_verb_phrase)
                .unwrap_or(joint);
            parts.push(lowercase_first(joint.trim_end_matches('.')));
            effect_idx += 2;
            continue;
        }
        if effect_idx + 1 < effects.len()
            && let Some(choose) = structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(move_to_zone) = structural_unwrap_render_wrappers(&effects[effect_idx + 1])
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
            && let Some(joint) = describe_choose_then_move_to_graveyard(choose, move_to_zone)
        {
            parts.push(lowercase_first(joint.trim_end_matches('.')));
            effect_idx += 2;
            continue;
        }
        if effect_idx + 1 < effects.len()
            && let Some(choose) = structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(move_to_zone) = structural_unwrap_render_wrappers(&effects[effect_idx + 1])
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
            && let Some(joint) = describe_choose_then_move_to_battlefield(choose, move_to_zone)
        {
            let joint = joint
                .strip_prefix("you ")
                .map(normalize_you_verb_phrase)
                .unwrap_or(joint);
            parts.push(lowercase_first(joint.trim_end_matches('.')));
            effect_idx += 2;
            continue;
        }
        if effect_idx + 1 < effects.len()
            && let Some(joint) =
                describe_action_and_get_energy_pair(effect, &effects[effect_idx + 1])
        {
            parts.push(lowercase_first(&joint));
            effect_idx += 2;
            continue;
        }
        if effect_idx + 1 < effects.len()
            && let Some(joint) =
                describe_same_actor_gain_then_draw(effect, &effects[effect_idx + 1])
        {
            parts.push(lowercase_first(&joint));
            effect_idx += 2;
            continue;
        }
        if effect_idx + 1 < effects.len()
            && let Some(joint) =
                describe_same_actor_draw_then_gain(effect, &effects[effect_idx + 1])
        {
            parts.push(lowercase_first(&joint));
            effect_idx += 2;
            continue;
        }
        if effect_idx + 1 < effects.len()
            && let Some(joint) = describe_joint_subject_pair(effect, &effects[effect_idx + 1])
        {
            parts.push(lowercase_first(&joint));
            effect_idx += 2;
            continue;
        }
        if effect_idx + 1 < effects.len()
            && let Some(choose) = structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(sacrifice) = sacrifice_view_unwrapped(&effects[effect_idx + 1])
            && let Some(compact) = describe_choose_then_sacrifice(choose, sacrifice)
        {
            let compact = compact
                .strip_prefix("you ")
                .map(normalize_you_verb_phrase)
                .unwrap_or(compact);
            parts.push(lowercase_first(compact.trim_end_matches('.')));
            effect_idx += 2;
            continue;
        }
        let remaining = effects[effect_idx..].iter().collect::<Vec<_>>();
        if let Some((joint, consumed)) =
            describe_longest_conjoined_counter_or_draw_sequence(&remaining)
        {
            parts.push(lowercase_first(&joint));
            effect_idx += consumed;
            continue;
        }
        let rendered = describe_effect(effect);
        let trimmed = rendered.trim();
        if trimmed.is_empty()
            || trimmed.contains(". ")
            || trimmed.contains(": ")
            || trimmed.starts_with("If ")
            || trimmed.starts_with("When ")
            || trimmed.starts_with("Whenever ")
            || trimmed.starts_with("At ")
            || trimmed.starts_with("Choose ")
            || (!compact_has_scaffolding
                && (trimmed.contains("tagged cards") || trimmed.contains("tagged '")))
        {
            return None;
        }
        let normalized = normalize_imperative_you_clause(trimmed.trim_end_matches('.'));
        parts.push(lowercase_first(&normalized));
        effect_idx += 1;
    }

    let last = parts.pop()?;
    let body = if parts.is_empty() {
        last
    } else {
        format!("{}, then {last}", parts.join(", "))
    };
    Some(cleanup_decompiled_text(&body))
}

fn describe_exile_two_creatures_then_controller_consults(effects: &[Effect]) -> Option<String> {
    let [exile_effect, iteration_effect] = effects else {
        return None;
    };
    let exiled_tag =
        tagged_exile_exact_target_type(exile_effect, crate::types::CardType::Creature, 2)?;
    let for_each = iteration_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if for_each.tag.as_str() != exiled_tag && for_each.tag.as_str() != crate::tag::SOURCE_EXILED_TAG
    {
        return None;
    }
    if !matches!(
        consult_reveal_put_battlefield_then_shuffle_selection(for_each).as_deref(),
        Some("creature" | "creature card")
    ) {
        return None;
    }

    Some("Exile two target creatures. For each of those creatures, its controller reveals cards from the top of their library until they reveal a creature card, puts that card onto the battlefield, then shuffles the rest into their library".to_string())
}

fn describe_exile_top_then_search_to_hand_and_shuffle(effects: &[Effect]) -> Option<String> {
    let [exile_effect, search_effect, move_effect, shuffle_effect] = effects else {
        return None;
    };
    let exile = structural_unwrap_render_wrappers(exile_effect)
        .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    if exile.player != PlayerFilter::You {
        return None;
    }
    let search = structural_unwrap_render_wrappers(search_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !search.is_search
        || search.chooser != PlayerFilter::You
        || search.zone != Some(Zone::Library)
        || search.count.min != 1
        || search.count.max != Some(1)
    {
        return None;
    }
    let move_to_hand = downcast_search_split_move_to_zone(move_effect)?;
    if !search_split_move_to_zone_uses_tag(move_to_hand, search.tag.as_str(), Zone::Hand) {
        return None;
    }
    let shuffle = structural_unwrap_render_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if shuffle.player != PlayerFilter::You || shuffle.target_spec.is_some() {
        return None;
    }

    let exile_text = describe_effect(exile_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let search_text = lowercase_first(describe_effect(search_effect).trim().trim_end_matches('.'));
    Some(format!(
        "{exile_text}, then {search_text}. Put that card into your hand, then shuffle"
    ))
}

fn describe_two_target_players_each_search_to_top(effects: &[Effect]) -> Option<String> {
    let [target_effect, per_player_effect] = effects else {
        return None;
    };
    let target_only = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let ChooseSpec::WithCount(target, count) = &target_only.target else {
        return None;
    };
    if count.min != 2 || count.max != Some(2) {
        return None;
    }
    let ChooseSpec::Target(target) = target.as_ref() else {
        return None;
    };
    if !matches!(target.as_ref(), ChooseSpec::Player(PlayerFilter::Any)) {
        return None;
    }

    let for_players = structural_unwrap_render_wrappers(per_player_effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::target_player() || for_players.effects.len() != 1 {
        return None;
    }
    let search = structural_unwrap_render_wrappers(&for_players.effects[0])
        .downcast_ref::<crate::effects::SearchLibraryEffect>()?;
    if search.destination != Zone::Library
        || search.chooser != PlayerFilter::IteratedPlayer
        || search.player != PlayerFilter::IteratedPlayer
        || search.library_position_from_top != Some(Value::Fixed(1))
    {
        return None;
    }
    let rendered_search = describe_effect(&for_players.effects[0]);
    let action = rendered_search
        .trim()
        .trim_end_matches('.')
        .strip_prefix("That player ")
        .or_else(|| {
            rendered_search
                .trim()
                .trim_end_matches('.')
                .strip_prefix("that player ")
        })?;
    Some(format!("Choose two target players. Each of them {action}"))
}

fn describe_search_reveal_nested_may_move_else_hand(effects: &[Effect]) -> Option<String> {
    let [
        search_effect,
        reveal_effect,
        conditional_effect,
        shuffle_effect,
    ] = effects
    else {
        return None;
    };
    let search = structural_unwrap_render_wrappers(search_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !search.is_search
        || search.zone != Some(Zone::Library)
        || search.count.min != 1
        || search.count.max != Some(1)
    {
        return None;
    }
    let reveal = structural_unwrap_render_wrappers(reveal_effect)
        .downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    if reveal.tag != search.tag {
        return None;
    }
    let conditional = structural_unwrap_render_wrappers(conditional_effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    let [with_id_effect, declined_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let with_id = with_id_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = structural_unwrap_render_wrappers(&with_id.effect)
        .downcast_ref::<crate::effects::MayEffect>()?;
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| *decider != PlayerFilter::You)
    {
        return None;
    }
    let [battlefield_effect] = may.effects.as_slice() else {
        return None;
    };
    let battlefield_move = move_to_zone_from_effect(battlefield_effect)?;
    if battlefield_move.zone != Zone::Battlefield
        || !matches!(battlefield_move.target.base(), ChooseSpec::Tagged(tag) if tag == &search.tag)
    {
        return None;
    }
    let declined = structural_unwrap_render_wrappers(declined_effect)
        .downcast_ref::<crate::effects::IfEffect>()?;
    if declined.condition != with_id.id
        || declined.predicate != crate::effect::EffectPredicate::DidNotHappen
        || !declined.else_.is_empty()
    {
        return None;
    }
    let [declined_hand_effect] = declined.then.as_slice() else {
        return None;
    };
    let declined_hand = move_to_zone_from_effect(declined_hand_effect)?;
    let [otherwise_hand_effect] = conditional.if_false.as_slice() else {
        return None;
    };
    let otherwise_hand = move_to_zone_from_effect(otherwise_hand_effect)?;
    let is_searched_card_to_hand = |move_to_zone: &crate::effects::MoveToZoneEffect| {
        move_to_zone.zone == Zone::Hand
            && matches!(move_to_zone.target.base(), ChooseSpec::Tagged(tag) if tag == &search.tag)
    };
    if !is_searched_card_to_hand(declined_hand) || !is_searched_card_to_hand(otherwise_hand) {
        return None;
    }
    let shuffle = structural_unwrap_render_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if shuffle.player != PlayerFilter::You || shuffle.target_spec.is_some() {
        return None;
    }

    let rendered_search = describe_effect(search_effect);
    let search_text = rendered_search
        .trim()
        .trim_end_matches('.')
        .strip_prefix("You ")
        .or_else(|| {
            rendered_search
                .trim()
                .trim_end_matches('.')
                .strip_prefix("you ")
        })?
        .to_string();
    let condition = describe_condition(&conditional.condition);
    let tapped = if battlefield_move.enters_tapped {
        " tapped"
    } else {
        ""
    };
    Some(format!(
        "{}. You may put that card onto the battlefield{tapped} if {condition}. Otherwise, put that card into your hand. Then shuffle",
        capitalize_first(&format!("{search_text} and reveal it"))
    ))
}
