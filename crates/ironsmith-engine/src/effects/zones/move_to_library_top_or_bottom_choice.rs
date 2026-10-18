//! Move an object to the top or bottom of its owner's library at a specified player's choice.

use crate::decision::DecisionMaker;
use crate::decisions::context::SelectableOption;
use crate::effect::EffectOutcome;
use crate::effects::CompletedEffectOutputs;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_tagged_object_id};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
use crate::zone::Zone;

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
    top_position: u32,
) -> LibraryPositionChoice {
    let top_label = if top_position == 0 {
        "Top of library".to_string()
    } else {
        format!(
            "{} from the top of library",
            ordinal_position(top_position + 1)
        )
    };
    use crate::effects::composition::scoped_choice::{SingleChoicePolicy, select_typed_menu};
    select_typed_menu(
        game,
        decision_maker,
        chooser,
        source,
        &format!("Put {object_name} on the top or bottom of its owner's library"),
        &[
            (LibraryPositionChoice::Top, top_label),
            (LibraryPositionChoice::Bottom, "Bottom of library".into()),
        ],
        SingleChoicePolicy::FirstOrDefault,
        |i, (_, label)| SelectableOption::new(i, label.clone()),
    )
    .expect("first-option policy cannot fail")
    .map(|(position, _)| position)
    .unwrap_or(LibraryPositionChoice::Top) // Caller checks suspension before moving anything.
}

fn ordinal_position(position: u32) -> String {
    match position {
        2 => "Second".to_string(),
        3 => "Third".to_string(),
        n => format!("{n}th"),
    }
}

impl EffectExecutor for MoveToLibraryTopOrBottomChoiceEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let result = crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let moves_source = matches!(self.target.base(), ChooseSpec::Source);
                let mut object_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
                if let ChooseSpec::Tagged(tag) = &self.target
                    && let Some(tagged) = ctx.get_tagged_all(tag)
                {
                    for (idx, snapshot) in tagged.iter().enumerate() {
                        if idx < object_ids.len()
                            && game.object(object_ids[idx]).is_none()
                            && let Some(resolved) = resolve_tagged_object_id(game, ctx, snapshot)
                        {
                            object_ids[idx] = resolved;
                        }
                    }
                }
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                if object_ids.is_empty() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::target_invalid(),
                    ));
                }

                let mut choices = std::collections::HashMap::new();
                let mut moves = Vec::new();
                for object_id in object_ids {
                    let Some(obj) = game.object(object_id) else {
                        continue;
                    };
                    let from_zone = obj.zone;
                    let chooser = match &self.chooser {
                        Some(chooser) => crate::effects::helpers::resolve_player_filter_as_chooser(
                            game, chooser, ctx,
                        )?,
                        None => obj.owner,
                    };
                    let object_name = obj.name.to_string();
                    let pre_snapshot =
                        ObjectSnapshot::from_object_with_calculated_characteristics(obj, game);
                    let choice = choose_library_position(
                        game,
                        &mut ctx.decision_maker,
                        chooser,
                        ctx.source,
                        &object_name,
                        self.top_position,
                    );
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }

                    choices.insert(
                        object_id,
                        (
                            match choice {
                                LibraryPositionChoice::Top if self.top_position > 0 => {
                                    crate::effects::cards::LibraryCardPosition::NthFromTop(
                                        self.top_position as usize + 1,
                                    )
                                }
                                LibraryPositionChoice::Top => {
                                    crate::effects::cards::LibraryCardPosition::Top
                                }
                                LibraryPositionChoice::Bottom => {
                                    crate::effects::cards::LibraryCardPosition::Bottom
                                }
                            },
                            pre_snapshot.clone(),
                        ),
                    );
                    moves.push(super::PreparedZoneMove::capture(
                        game,
                        object_id,
                        from_zone,
                        Zone::Library,
                        ctx.cause.clone(),
                        Some(pre_snapshot),
                    ));
                }
                super::execute_zone_moves_with_outputs(game, ctx, moves, |game, ctx, receipts| {
                    super::positioned_move::complete_positioned_moves(
                        game,
                        ctx,
                        receipts,
                        &choices,
                        if moves_source {
                            super::positioned_move::SourceCompletion::Rebind
                        } else {
                            super::positioned_move::SourceCompletion::Unchanged
                        },
                    )
                })
            },
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        result
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
    #[cfg(test)]
    use crate::decisions::context::SelectOptionsContext;
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

    #[test]
    fn heterogeneous_zone_union_moves_each_legal_domain_to_owners_library() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let spell_card = CardBuilder::new(CardId::from_raw(10), "Detoured Spell")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
            .card_types(vec![CardType::Instant])
            .build();
        let graveyard_card = CardBuilder::new(CardId::from_raw(11), "Detoured Graveyard Land")
            .card_types(vec![CardType::Land])
            .build();
        let permanent = game.create_object_from_card(
            &test_card(12, "Detoured Permanent"),
            bob,
            Zone::Battlefield,
        );
        let spell = game.create_object_from_card(&spell_card, bob, Zone::Stack);
        game.push_to_stack(crate::game_state::StackEntry::new(spell, bob));
        let graveyard_land = game.create_object_from_card(&graveyard_card, bob, Zone::Graveyard);
        let existing_library_card =
            game.create_object_from_card(&test_card(13, "Existing Top"), bob, Zone::Library);

        let target_filter = crate::target::ObjectFilter {
            any_of: vec![
                crate::target::ObjectFilter::spell(),
                crate::target::ObjectFilter::nonland_permanent(),
                crate::target::ObjectFilter::default().in_zone(Zone::Graveyard),
            ],
            ..crate::target::ObjectFilter::default()
        };
        let target_spec = ChooseSpec::target(ChooseSpec::Object(target_filter));
        let effect = MoveToLibraryTopOrBottomChoiceEffect::new(target_spec);
        let source = game.new_object_id();
        let mut dm = BottomChoiceDm { chooser: None };

        let mut moved = Vec::new();
        for target in [spell, permanent, graveyard_land] {
            let outcome = {
                let mut ctx = ExecutionContext::new(source, alice, &mut dm)
                    .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
                effect
                    .execute(&mut game, &mut ctx)
                    .expect("each Endless Detour target domain should resolve")
            };
            let moved_id = outcome
                .objects()
                .and_then(|objects| objects.first())
                .copied()
                .expect("the targeted object should move to its owner's library");
            moved.push(moved_id);
            assert_eq!(dm.chooser, Some(bob));
            assert_eq!(game.players[1].library.first().copied(), Some(moved_id));
        }

        assert!(moved.iter().all(|id| {
            game.object(*id)
                .is_some_and(|object| object.zone == Zone::Library)
        }));
        assert_eq!(
            game.players[1].library.last().copied(),
            Some(existing_library_card)
        );
    }
}
