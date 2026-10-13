//! Shared constrained contributor selection, independent of the action paid.
use crate::effects::ExecutionContext;
use crate::{
    ability::AbilityKind,
    game_state::GameState,
    ids::{ObjectId, PlayerId},
    static_abilities::StaticAbilityId,
};

pub(super) fn candidates(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
    allowed: impl Fn(ObjectId) -> bool,
) -> Vec<ObjectId> {
    candidates_with_source_policy(game, source, controller, false, allowed)
}

/// Crew/Saddle exclude their own permanent; a spell's contributor cost need
/// not exclude its source explicitly. All contributors must be present and untapped.
pub(super) fn candidates_with_source_policy(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
    allow_source: bool,
    allowed: impl Fn(ObjectId) -> bool,
) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|&id| {
            (allow_source || id != source)
                && game
                    .object(id)
                    .is_some_and(|object| game.controller_of(object) == controller)
                && game.current_is_creature(id)
                && !game.is_tapped(id)
                && !game.is_phased_out(id)
                && allowed(id)
        })
        .collect()
}

#[derive(Clone, Copy)]
pub(super) enum ContributionRequirement { AtLeast(u32), Exactly(usize) }

/// Return the selected contributors and whether their captured contribution
/// meets the threshold. The caller owns alternative payment and completion.
pub(super) fn select_contributors(
    game: &GameState,
    ctx: &mut ExecutionContext,
    candidates: Vec<ObjectId>,
    required: u32,
    prompt: &str,
    value: impl Fn(ObjectId) -> i32,
) -> Option<(Vec<ObjectId>, bool)> {
    select_contributors_with_requirement(game, ctx, candidates,
        ContributionRequirement::AtLeast(required), prompt, value)
}

pub(super) fn select_contributors_with_requirement(
    game: &GameState,
    ctx: &mut ExecutionContext,
    mut candidates: Vec<ObjectId>,
    requirement: ContributionRequirement,
    prompt: &str,
    value: impl Fn(ObjectId) -> i32,
) -> Option<(Vec<ObjectId>, bool)> {
    let (minimum, maximum) = match requirement {
        ContributionRequirement::AtLeast(required) => {
            candidates.sort_by_key(|id| std::cmp::Reverse(value(*id)));
            (usize::from(required != 0), candidates.len())
        }
        ContributionRequirement::Exactly(count) => (count, count),
    };
    let spec = crate::decisions::specs::ChooseObjectsSpec::new(
        ctx.source,
        prompt,
        candidates.clone(),
        minimum,
        Some(maximum),
    );
    let mut chosen = crate::decisions::make_decision(
        game,
        ctx.decision_maker,
        ctx.controller,
        Some(ctx.source),
        spec,
    );
    if ctx.decision_maker.awaiting_choice() {
        return None;
    }
    chosen.retain(|id| candidates.contains(id));
    chosen.sort();
    chosen.dedup();
    if let ContributionRequirement::Exactly(count) = requirement {
        // Preserve exact-cost fallback: fill an underspecified answer, but an
        // overspecified answer does not authorize tapping more contributors.
        for id in candidates {
            if chosen.len() >= count { break; }
            if !chosen.contains(&id) { chosen.push(id); }
        }
        let sufficient = chosen.len() == count;
        return Some((chosen, sufficient));
    }
    let ContributionRequirement::AtLeast(required) = requirement else { unreachable!() };
    let mut total: i64 = chosen.iter().map(|id| i64::from(value(*id))).sum();
    for id in candidates {
        if total >= i64::from(required) {
            break;
        }
        if !chosen.contains(&id) && value(id) > 0 {
            chosen.push(id);
            total += i64::from(value(id));
        }
    }
    Some((chosen, total >= i64::from(required)))
}

pub(super) fn object_power(game: &GameState, object_id: ObjectId) -> i32 {
    game.calculated_characteristics(object_id)
        .and_then(|calc| calc.power)
        .or_else(|| game.object(object_id).and_then(|obj| obj.power()))
        .unwrap_or(0)
}

fn object_toughness(game: &GameState, object_id: ObjectId) -> i32 {
    game.calculated_characteristics(object_id)
        .and_then(|calc| calc.toughness)
        .or_else(|| game.object(object_id).and_then(|obj| obj.toughness()))
        .unwrap_or(0)
}

pub(super) fn keyword_marker_texts(game: &GameState, object_id: ObjectId) -> Vec<String> {
    let abilities = game.current_abilities(object_id).unwrap_or_else(|| {
        game.object(object_id)
            .map(|obj| obj.abilities_vec())
            .unwrap_or_default()
    });
    abilities
        .into_iter()
        .filter_map(|ability| match ability.kind {
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::KeywordMarker =>
            {
                Some(static_ability.display().to_ascii_lowercase())
            }
            _ => None,
        })
        .collect()
}

fn power_bonus_from_marker(marker: &str, crew: bool) -> Option<i32> {
    let prefixes = [
        "this creature crews vehicles as though its power were ",
        "this creature saddles mounts and crews vehicles as though its power were ",
        "this token saddles mounts and crews vehicles as though its power were ",
    ];
    prefixes
        .iter()
        .skip(if crew { 0 } else { 1 })
        .find_map(|prefix| {
            marker
                .strip_prefix(prefix)
                .and_then(|rest| rest.strip_suffix(" greater."))
                .and_then(|amount| amount.parse::<i32>().ok())
        })
}

pub(super) fn contribution_value(game: &GameState, object_id: ObjectId, crew: bool) -> i32 {
    let markers = keyword_marker_texts(game, object_id);
    let use_toughness = markers.iter().any(|marker| {
        let marker = marker.trim_end_matches('.');
        (crew && marker == "this creature crews vehicles using its toughness rather than its power")
            || marker
                == "this creature saddles mounts and crews vehicles using its toughness rather than its power"
    });
    let base = if use_toughness {
        object_toughness(game, object_id)
    } else {
        object_power(game, object_id)
    };
    base + markers
        .iter()
        .filter_map(|marker| power_bonus_from_marker(marker, crew))
        .sum::<i32>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::EffectExecutor;
    use crate::{
        card::{CardBuilder, PowerToughness},
        ids::CardId,
        types::CardType,
        zone::Zone,
    };
    fn creature(game: &mut GameState, power: i32, toughness: i32) -> ObjectId {
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Contributor")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(power, toughness))
                .build(),
            PlayerId(0),
            Zone::Battlefield,
        )
    }
    #[test]
    fn shared_metric_respects_mechanic_specific_markers_and_cost_validation() {
        for saddle in [false, true] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let source = creature(&mut game, 10, 10);
            let payer = creature(&mut game, 1, 4);
            for text in [
                "This creature saddles mounts and crews vehicles using its toughness rather than its power.",
                "This creature crews Vehicles as though its power were 2 greater.",
            ] {
                game.object_mut(payer).unwrap().abilities_mut().push(
                    crate::ability::Ability::static_ability(
                        crate::static_abilities::StaticAbility::keyword_marker(text),
                    ),
                );
            }
            assert_eq!(
                contribution_value(&game, payer, !saddle),
                if saddle { 4 } else { 6 }
            );
            let mut ctx = ExecutionContext::new_default(source, PlayerId(0));
            if saddle {
                let effect = super::super::SaddleCostEffect::new(4);
                effect
                    .can_execute_as_cost(&game, source, PlayerId(0))
                    .unwrap();
                effect.execute_with_outputs(&mut game, &mut ctx).unwrap();
            } else {
                let effect = super::super::CrewCostEffect::new(6);
                effect
                    .can_execute_as_cost(&game, source, PlayerId(0))
                    .unwrap();
                effect.execute_with_outputs(&mut game, &mut ctx).unwrap();
            }
            assert!(game.is_tapped(payer));
            assert!(!game.is_tapped(source));
        }
    }
    #[test]
    fn candidate_policy_keeps_crew_prohibition_out_of_saddle() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let source = creature(&mut game, 3, 3);
        let payer = creature(&mut game, 3, 3);
        game.effect_store.cant_effects.cant_crew.insert(payer);
        assert!(
            candidates(&game, source, PlayerId(0), |id| !game
                .effect_store
                .cant_effects
                .cant_crew
                .contains(&id))
            .is_empty()
        );
        assert_eq!(
            candidates(&game, source, PlayerId(0), |_| true),
            vec![payer]
        );
    }
    #[test]
    fn exact_contributors_filter_invalid_answers_deduplicate_and_fill_without_overpaying() {
        struct Answer(Vec<ObjectId>);
        impl crate::decision::DecisionMaker for Answer {
            fn decide_objects(&mut self, _: &GameState, _: &crate::decisions::context::SelectObjectsContext) -> Vec<ObjectId> { self.0.clone() }
        }
        let game = crate::tests::test_helpers::setup_two_player_game();
        let eligible = vec![ObjectId(1), ObjectId(2), ObjectId(3)];
        for (answer, expected, sufficient) in [
            (vec![ObjectId(99), ObjectId(2), ObjectId(2)], vec![ObjectId(2), ObjectId(1)], true),
            (eligible.clone(), eligible.clone(), false),
            (vec![], vec![ObjectId(1), ObjectId(2)], true),
        ] {
            let mut dm = Answer(answer);
            let mut ctx = ExecutionContext::new(ObjectId(90), PlayerId(0), &mut dm);
            assert_eq!(select_contributors_with_requirement(&game, &mut ctx, eligible.clone(),
                ContributionRequirement::Exactly(2), "Contribute", |_| 50), Some((expected, sufficient)));
        }
    }

}
