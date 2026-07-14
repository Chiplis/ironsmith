//! Move an object to the top or bottom of its owner's library at a specified player's choice.

use crate::decision::DecisionMaker;
use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::EffectOutcome;
use crate::effects::helpers::{
    resolve_objects_for_effect, resolve_player_filter, resolve_tagged_object_id,
};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::processing::EventOutcome;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
use crate::zone::Zone;

use super::{apply_zone_change_with_additional_effects, maybe_prompt_for_split_result_order};

pub type MoveToLibraryTopOrBottomChoiceEffect =
    ironsmith_core::MoveToLibraryTopOrBottomChoiceEffect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LibraryPositionChoice {
    Top,
    Bottom,
}

fn choose_library_position(
    game: &GameState,
    decision_maker: &mut dyn DecisionMaker,
    chooser: PlayerId,
    source: ObjectId,
    object_name: &str,
) -> LibraryPositionChoice {
    let options = vec![
        SelectableOption::new(0, "Top of library"),
        SelectableOption::new(1, "Bottom of library"),
    ];
    let choice_ctx = SelectOptionsContext::new(
        chooser,
        Some(source),
        format!("Put {object_name} on the top or bottom of its owner's library"),
        options,
        1,
        1,
    );
    match decision_maker
        .decide_options(game, &choice_ctx)
        .into_iter()
        .next()
    {
        Some(1) => LibraryPositionChoice::Bottom,
        _ => LibraryPositionChoice::Top,
    }
}

fn position_library_objects(
    game: &mut GameState,
    object_ids: &[ObjectId],
    position: LibraryPositionChoice,
) {
    for &new_id in object_ids {
        let Some(owner) = game.object(new_id).map(|object| object.owner) else {
            continue;
        };
        match position {
            LibraryPositionChoice::Top => {
                game.move_library_card_to_top(owner, new_id, "card put on top of library")
            }
            LibraryPositionChoice::Bottom => {
                game.move_library_card_to_bottom(owner, new_id, "card put on bottom of library")
            }
        };
    }
}

impl EffectExecutor for MoveToLibraryTopOrBottomChoiceEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let moves_source = matches!(self.target.base(), ChooseSpec::Source);
        let mut object_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
        if let ChooseSpec::Tagged(tag) = &self.target {
            if let Some(tagged) = ctx.get_tagged_all(tag) {
                for (idx, snapshot) in tagged.iter().enumerate() {
                    if idx < object_ids.len()
                        && game.object(object_ids[idx]).is_none()
                        && let Some(resolved) = resolve_tagged_object_id(game, snapshot)
                    {
                        object_ids[idx] = resolved;
                    }
                }
            }
        }
        if object_ids.is_empty() {
            return Ok(EffectOutcome::target_invalid());
        }

        let mut moved_ids = Vec::new();
        let mut affected_ids = Vec::new();
        let mut any_replaced = false;
        let mut moved_source_lki = None;

        for object_id in object_ids {
            let Some(obj) = game.object(object_id) else {
                continue;
            };
            let stable_id = obj.stable_id;
            let from_zone = obj.zone;
            let chooser = match &self.chooser {
                Some(chooser) => resolve_player_filter(game, chooser, ctx)?,
                None => obj.owner,
            };
            let object_name = obj.name.to_string();
            let pre_snapshot =
                ObjectSnapshot::from_object_with_calculated_characteristics(obj, game);
            let source_lki_before_move = if moves_source && object_id == ctx.source {
                Some(pre_snapshot.clone())
            } else {
                None
            };

            let choice = choose_library_position(
                game,
                &mut ctx.decision_maker,
                chooser,
                ctx.source,
                &object_name,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }

            let additional_effects = ctx.additional_replacement_effects_snapshot();
            let result = apply_zone_change_with_additional_effects(
                game,
                object_id,
                from_zone,
                Zone::Library,
                ctx.cause.clone(),
                &mut ctx.decision_maker,
                &additional_effects,
            );

            match result {
                EventOutcome::Prevented => return Ok(EffectOutcome::prevented()),
                EventOutcome::Proceed(mut result) => {
                    if result.new_object_ids.is_empty() {
                        continue;
                    }
                    ctx.refresh_target_snapshot(pre_snapshot.clone());
                    if let Some(snapshot) = source_lki_before_move.clone() {
                        moved_source_lki = Some(snapshot);
                    }

                    if result.final_zone == Zone::Exile {
                        for &new_id in &result.new_object_ids {
                            game.add_exiled_with_source_link(ctx.source, new_id);
                        }
                    } else if result.final_zone == Zone::Library {
                        position_library_objects(game, &result.new_object_ids, choice);
                        if from_zone == Zone::Battlefield {
                            maybe_prompt_for_split_result_order(
                                game,
                                &mut ctx.decision_maker,
                                result.final_zone,
                                &ctx.cause,
                                &mut result,
                            );
                            game.record_zone_change_results(
                                object_id,
                                result.new_object_ids.clone(),
                            );
                        }
                    }

                    affected_ids.extend(result.new_object_ids.iter().copied());
                    moved_ids.extend(result.new_object_ids.iter().copied());
                }
                EventOutcome::Replaced => {
                    any_replaced = true;
                    if let Some(result_id) = game.find_object_by_stable_id(stable_id) {
                        affected_ids.push(result_id);
                    }
                }
                EventOutcome::NotApplicable => {}
            }
        }

        if moves_source && let Some(new_source_id) = moved_ids.first().copied() {
            ctx.source = new_source_id;
        }
        if let Some(snapshot) = moved_source_lki {
            ctx.refresh_source_snapshot(snapshot);
        }

        if !moved_ids.is_empty() {
            return Ok(EffectOutcome::with_objects(moved_ids).with_affected_objects(affected_ids));
        }
        if any_replaced {
            return Ok(EffectOutcome::replaced().with_affected_objects(affected_ids));
        }
        Ok(EffectOutcome::target_invalid())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target to move to a chosen library position"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::CardType;

    struct BottomChoiceDm {
        chooser: Option<PlayerId>,
    }

    impl DecisionMaker for BottomChoiceDm {
        fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            self.chooser = Some(ctx.player);
            vec![1]
        }
    }

    fn test_card(id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Creature])
            .build()
    }

    #[test]
    fn owner_chooses_bottom_for_target_creature() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature =
            game.create_object_from_card(&test_card(1, "Choice Target"), bob, Zone::Battlefield);
        let existing_library_card =
            game.create_object_from_card(&test_card(2, "Existing Top"), bob, Zone::Library);

        let source = game.new_object_id();
        let mut dm = BottomChoiceDm { chooser: None };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let effect =
            MoveToLibraryTopOrBottomChoiceEffect::new(ChooseSpec::SpecificObject(creature));

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("top-or-bottom move should execute");
        let moved_id = outcome
            .objects()
            .and_then(|objects| objects.first())
            .copied();

        assert_eq!(dm.chooser, Some(bob));
        assert_ne!(
            moved_id,
            Some(creature),
            "zone changes should create a new object id"
        );
        assert_eq!(game.players[1].library.first().copied(), moved_id);
        assert_eq!(
            game.players[1].library.last().copied(),
            Some(existing_library_card)
        );
    }

    #[test]
    fn explicit_you_chooser_uses_effect_controller() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature =
            game.create_object_from_card(&test_card(3, "Choice Target"), bob, Zone::Battlefield);

        let source = game.new_object_id();
        let mut dm = BottomChoiceDm { chooser: None };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let effect =
            MoveToLibraryTopOrBottomChoiceEffect::new(ChooseSpec::SpecificObject(creature))
                .with_chooser(crate::target::PlayerFilter::You);

        effect
            .execute(&mut game, &mut ctx)
            .expect("controller-chosen top-or-bottom move should execute");

        assert_eq!(dm.chooser, Some(alice));
    }
}
