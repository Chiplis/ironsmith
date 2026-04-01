//! Meld effect implementation.

use crate::combat_state::{AttackerInfo, get_attack_target};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::zones::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_with_options,
};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::game_state::MeldComponentState;
use crate::object::ObjectKind;
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeldEffect {
    pub result_name: String,
    pub enters_tapped: bool,
    pub enters_attacking: bool,
}

impl MeldEffect {
    pub fn new(result_name: impl Into<String>) -> Self {
        Self {
            result_name: result_name.into(),
            enters_tapped: false,
            enters_attacking: false,
        }
    }

    pub fn enters_tapped(mut self, enters_tapped: bool) -> Self {
        self.enters_tapped = enters_tapped;
        self
    }

    pub fn enters_attacking(mut self, enters_attacking: bool) -> Self {
        self.enters_attacking = enters_attacking;
        self
    }
}

fn current_source_id(game: &GameState, ctx: &ExecutionContext) -> Option<crate::ids::ObjectId> {
    if game.object(ctx.source).is_some() {
        return Some(ctx.source);
    }
    ctx.source_snapshot
        .as_ref()
        .and_then(|snapshot| game.find_object_by_stable_id(snapshot.stable_id))
}

impl EffectExecutor for MeldEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(source_id) = current_source_id(game, ctx) else {
            return Ok(EffectOutcome::resolved());
        };
        let Some(source) = game.object(source_id).cloned() else {
            return Ok(EffectOutcome::resolved());
        };
        if source.owner != ctx.controller
            || source.controller != ctx.controller
            || source.kind != ObjectKind::Card
        {
            return Ok(EffectOutcome::resolved());
        }

        let Some(counterpart_name) = crate::cards::meld_counterpart_name(&source.name) else {
            return Ok(EffectOutcome::resolved());
        };

        let counterpart_id = match source.zone {
            Zone::Battlefield => game.battlefield.iter().copied().find(|&candidate_id| {
                game.object(candidate_id).is_some_and(|candidate| {
                    candidate_id != source_id
                        && candidate.owner == ctx.controller
                        && candidate.controller == ctx.controller
                        && candidate.name.eq_ignore_ascii_case(counterpart_name)
                })
            }),
            Zone::Exile => game.exile.iter().copied().find(|&candidate_id| {
                game.object(candidate_id).is_some_and(|candidate| {
                    candidate_id != source_id
                        && candidate.owner == ctx.controller
                        && candidate.controller == ctx.controller
                        && candidate.name.eq_ignore_ascii_case(counterpart_name)
                })
            }),
            _ => None,
        };
        let Some(counterpart_id) = counterpart_id else {
            return Ok(EffectOutcome::resolved());
        };

        let source_attack_target = if self.enters_attacking {
            game.combat
                .as_ref()
                .and_then(|combat| get_attack_target(combat, source_id).cloned())
        } else {
            None
        };

        let (source_exile_id, counterpart_exile_id) = if source.zone == Zone::Battlefield {
            let Some(new_source_id) = game.move_object_by_effect(source_id, Zone::Exile) else {
                return Ok(EffectOutcome::resolved());
            };
            let Some(new_counterpart_id) = game.move_object_by_effect(counterpart_id, Zone::Exile)
            else {
                return Ok(EffectOutcome::resolved());
            };
            (new_source_id, new_counterpart_id)
        } else {
            (source_id, counterpart_id)
        };

        let Some(exiled_source) = game.object(source_exile_id) else {
            return Ok(EffectOutcome::resolved());
        };
        let Some(exiled_counterpart) = game.object(counterpart_exile_id) else {
            return Ok(EffectOutcome::resolved());
        };
        if exiled_source.zone != Zone::Exile
            || exiled_counterpart.zone != Zone::Exile
            || exiled_source.kind != ObjectKind::Card
            || exiled_counterpart.kind != ObjectKind::Card
        {
            return Ok(EffectOutcome::resolved());
        }

        let Some(result_def) =
            crate::cards::linked_face_definition_by_name_or_id(Some(&self.result_name), None)
        else {
            return Ok(EffectOutcome::resolved());
        };

        let meld_components = vec![
            MeldComponentState {
                stable_id: exiled_source.stable_id,
                owner: exiled_source.owner,
                name: exiled_source.name.clone(),
            },
            MeldComponentState {
                stable_id: exiled_counterpart.stable_id,
                owner: exiled_counterpart.owner,
                name: exiled_counterpart.name.clone(),
            },
        ];

        let melded_id =
            game.create_object_from_definition(&result_def, ctx.controller, Zone::Command);
        match move_to_battlefield_with_options(
            game,
            ctx,
            melded_id,
            BattlefieldEntryOptions::specific(ctx.controller, self.enters_tapped),
        ) {
            BattlefieldEntryOutcome::Moved(new_id) => {
                game.set_melded_permanent(new_id, meld_components);
                game.remove_object(source_exile_id);
                game.remove_object(counterpart_exile_id);
                if let Some(target) = source_attack_target
                    && let Some(combat) = game.combat.as_mut()
                {
                    combat.attackers.push(AttackerInfo {
                        creature: new_id,
                        target,
                    });
                }
                Ok(EffectOutcome::with_objects(vec![new_id]))
            }
            BattlefieldEntryOutcome::Prevented => {
                game.remove_object(melded_id);
                Ok(EffectOutcome::resolved())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::cards::{CardDefinition, register_runtime_custom_card};
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use crate::decision::DecisionMaker;
    use crate::executor::ExecutionContext;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::{Object, ObjectKind};
    use crate::snapshot::ObjectSnapshot;
    use crate::target::ChooseSpec;
    use crate::types::{CardType, Subtype};

    fn card_definition(name: &str, types: Vec<CardType>, pt: Option<(i32, i32)>) -> CardDefinition {
        let mut builder = CardBuilder::new(CardId::new(), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(types)
            .oracle_text("");
        if let Some((power, toughness)) = pt {
            builder = builder
                .subtypes(vec![Subtype::Rat])
                .power_toughness(PowerToughness::fixed(power, toughness));
        }
        CardDefinition::new(builder.build())
    }

    fn register_test_meld_cards() {
        register_runtime_custom_card(card_definition(
            "Graf Rats",
            vec![CardType::Creature],
            Some((2, 1)),
        ));
        register_runtime_custom_card(card_definition(
            "Midnight Scavengers",
            vec![CardType::Creature],
            Some((3, 3)),
        ));
        register_runtime_custom_card(card_definition(
            "Chittering Host",
            vec![CardType::Creature],
            Some((5, 6)),
        ));
    }

    fn create_test_melded_permanent(game: &mut GameState, owner: PlayerId) -> ObjectId {
        let source = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Graf Rats"), None)
                .expect("source definition"),
            owner,
            Zone::Exile,
        );
        game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Midnight Scavengers"), None)
                .expect("counterpart definition"),
            owner,
            Zone::Exile,
        );

        let mut ctx = ExecutionContext::new_default(source, owner);
        MeldEffect::new("Chittering Host")
            .execute(game, &mut ctx)
            .expect("meld should resolve")
            .first_output_object()
            .expect("meld should produce a result object")
    }

    fn names_for_ids(game: &GameState, ids: &[ObjectId]) -> Vec<String> {
        ids.iter()
            .map(|id| game.object(*id).expect("object exists").name.clone())
            .collect()
    }

    fn library_top_to_bottom_names(game: &GameState, player: PlayerId) -> Vec<String> {
        game.player(player)
            .expect("player exists")
            .library
            .iter()
            .rev()
            .map(|id| {
                game.object(*id)
                    .expect("library object exists")
                    .name
                    .clone()
            })
            .collect()
    }

    #[derive(Default)]
    struct ReverseOrderDecisionMaker {
        prompts: Vec<String>,
    }

    impl DecisionMaker for ReverseOrderDecisionMaker {
        fn decide_order(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::OrderContext,
        ) -> Vec<ObjectId> {
            self.prompts.push(ctx.description.clone());
            let mut ids = ctx.items.iter().map(|(id, _)| *id).collect::<Vec<_>>();
            ids.reverse();
            ids
        }
    }

    #[test]
    fn meld_effect_creates_result_from_exiled_pair() {
        register_test_meld_cards();
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let source = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Graf Rats"), None)
                .expect("source definition"),
            alice,
            Zone::Exile,
        );
        let counterpart = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Midnight Scavengers"), None)
                .expect("counterpart definition"),
            alice,
            Zone::Exile,
        );

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = MeldEffect::new("Chittering Host")
            .execute(&mut game, &mut ctx)
            .expect("meld should resolve");

        let created = outcome.output_objects();
        assert!(!created.is_empty(), "meld should produce a result object");
        assert_eq!(created.len(), 1);
        let result = game.object(created[0]).expect("meld result should exist");
        assert_eq!(result.zone, Zone::Battlefield);
        assert_eq!(result.name, "Chittering Host");
        assert!(
            game.object(source).is_none(),
            "source card should be consumed"
        );
        assert!(
            game.object(counterpart).is_none(),
            "counterpart card should be consumed"
        );
    }

    #[test]
    fn meld_effect_leaves_objects_exiled_when_pair_is_invalid() {
        register_test_meld_cards();
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let source = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Graf Rats"), None)
                .expect("source definition"),
            alice,
            Zone::Exile,
        );

        let token_id = game.new_object_id();
        let token_card = CardBuilder::new(CardId::new(), "Midnight Scavengers")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Rat])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build();
        let mut token = Object::from_card(token_id, &token_card, alice, Zone::Exile);
        token.kind = ObjectKind::Token;
        game.add_object(token);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = MeldEffect::new("Chittering Host")
            .execute(&mut game, &mut ctx)
            .expect("meld should resolve");

        assert!(
            outcome
                .affected_objects()
                .is_none_or(|objects| objects.is_empty())
        );
        assert_eq!(
            game.object(source).expect("source should remain").zone,
            Zone::Exile
        );
        assert!(game.battlefield.iter().copied().all(|id| {
            game.object(id)
                .is_none_or(|obj| obj.name != "Chittering Host")
        }));
    }

    #[test]
    fn meld_effect_can_enter_tapped_and_attacking() {
        register_test_meld_cards();
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source_battlefield = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Graf Rats"), None)
                .expect("source definition"),
            alice,
            Zone::Battlefield,
        );
        let counterpart_battlefield = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Midnight Scavengers"), None)
                .expect("counterpart definition"),
            alice,
            Zone::Battlefield,
        );
        let source_snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(source_battlefield).expect("source exists"),
            &game,
        );
        game.combat = Some(CombatState {
            attackers: vec![
                AttackerInfo {
                    creature: source_battlefield,
                    target: AttackTarget::Player(bob),
                },
                AttackerInfo {
                    creature: counterpart_battlefield,
                    target: AttackTarget::Player(bob),
                },
            ],
            blockers: Default::default(),
            damage_assignment_order: Default::default(),
        });

        let mut ctx = ExecutionContext::new_default(source_battlefield, alice)
            .with_source_snapshot(source_snapshot);
        let outcome = MeldEffect::new("Chittering Host")
            .enters_tapped(true)
            .enters_attacking(true)
            .execute(&mut game, &mut ctx)
            .expect("meld should resolve");

        let result_id = outcome
            .first_output_object()
            .expect("meld should produce a result object");
        assert!(game.is_tapped(result_id), "meld result should enter tapped");
        let attackers = &game.combat.as_ref().expect("combat should exist").attackers;
        assert!(attackers.iter().any(|info| {
            info.creature == result_id && info.target == AttackTarget::Player(bob)
        }));
    }

    #[test]
    fn melded_permanent_leaves_battlefield_as_two_front_cards() {
        register_test_meld_cards();
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let source = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Graf Rats"), None)
                .expect("source definition"),
            alice,
            Zone::Exile,
        );
        let counterpart = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Midnight Scavengers"), None)
                .expect("counterpart definition"),
            alice,
            Zone::Exile,
        );
        let source_stable = game.object(source).expect("source exists").stable_id;
        let counterpart_stable = game
            .object(counterpart)
            .expect("counterpart exists")
            .stable_id;

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = MeldEffect::new("Chittering Host")
            .execute(&mut game, &mut ctx)
            .expect("meld should resolve");
        let melded_id = outcome
            .first_output_object()
            .expect("meld should produce a result object");

        let first_graveyard_id = game
            .move_object_by_effect(melded_id, Zone::Graveyard)
            .expect("melded permanent should move");
        let mut moved_ids = game.take_zone_change_results(melded_id);
        if moved_ids.is_empty() {
            moved_ids.push(first_graveyard_id);
        }

        assert_eq!(moved_ids.len(), 2, "meld should split into two cards");
        let moved_names: Vec<_> = moved_ids
            .iter()
            .map(|&id| game.object(id).expect("moved card exists").name.clone())
            .collect();
        assert!(moved_names.contains(&"Graf Rats".to_string()));
        assert!(moved_names.contains(&"Midnight Scavengers".to_string()));
        assert!(
            moved_ids.iter().any(|&id| game
                .object(id)
                .is_some_and(|obj| obj.stable_id == source_stable)),
            "one split card should preserve Graf Rats stable identity"
        );
        assert!(
            moved_ids.iter().any(|&id| game
                .object(id)
                .is_some_and(|obj| obj.stable_id == counterpart_stable)),
            "one split card should preserve Midnight Scavengers stable identity"
        );
        assert!(
            game.battlefield.iter().all(|&id| game
                .object(id)
                .is_none_or(|obj| obj.name != "Chittering Host")),
            "meld result should leave the battlefield"
        );
    }

    #[test]
    fn melded_permanent_zone_change_event_includes_split_result_cards() {
        register_test_meld_cards();
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let source = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Graf Rats"), None)
                .expect("source definition"),
            alice,
            Zone::Exile,
        );
        let _counterpart = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Midnight Scavengers"), None)
                .expect("counterpart definition"),
            alice,
            Zone::Exile,
        );

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = MeldEffect::new("Chittering Host")
            .execute(&mut game, &mut ctx)
            .expect("meld should resolve");
        let melded_id = outcome
            .first_output_object()
            .expect("meld should produce a result object");

        game.move_object_by_effect(melded_id, Zone::Graveyard)
            .expect("melded permanent should move");
        let pending = game.take_pending_trigger_events();
        let zone_change = pending
            .iter()
            .filter_map(|event| event.downcast::<crate::events::ZoneChangeEvent>())
            .find(|event| event.from == Zone::Battlefield && event.to == Zone::Graveyard)
            .expect("zone change event should be queued");

        assert_eq!(zone_change.objects, vec![melded_id]);
        assert_eq!(zone_change.result_objects.len(), 2);
        assert_eq!(zone_change.from, Zone::Battlefield);
        assert_eq!(zone_change.to, Zone::Graveyard);
    }

    #[test]
    fn destroy_uses_order_prompt_for_split_graveyard_cards() {
        register_test_meld_cards();
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let melded_id = create_test_melded_permanent(&mut game, alice);
        let source = game.new_object_id();

        let mut dm = ReverseOrderDecisionMaker::default();
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect =
            crate::effects::zones::DestroyEffect::with_spec(ChooseSpec::SpecificObject(melded_id));
        effect
            .execute(&mut game, &mut ctx)
            .expect("destroy should resolve");

        let graveyard_names =
            names_for_ids(&game, &game.player(alice).expect("alice exists").graveyard);
        assert_eq!(
            graveyard_names,
            vec!["Midnight Scavengers".to_string(), "Graf Rats".to_string()],
            "graveyard order should follow the chooser's ordering prompt",
        );
        assert!(
            dm.prompts
                .iter()
                .any(|prompt| prompt.contains("split cards in the graveyard")),
            "destroying a melded permanent should prompt for graveyard order",
        );
    }

    #[test]
    fn exile_uses_order_prompt_for_split_exile_cards() {
        register_test_meld_cards();
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let melded_id = create_test_melded_permanent(&mut game, alice);
        let source = game.new_object_id();

        let mut dm = ReverseOrderDecisionMaker::default();
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        crate::effects::zones::ExileEffect::specific(melded_id)
            .execute(&mut game, &mut ctx)
            .expect("exile should resolve");

        let exile_names = names_for_ids(&game, &game.exile);
        assert_eq!(
            exile_names,
            vec!["Midnight Scavengers".to_string(), "Graf Rats".to_string()],
            "exile order should follow the chooser's ordering prompt",
        );
        assert!(
            dm.prompts
                .iter()
                .any(|prompt| prompt.contains("split cards in exile")),
            "exiling a melded permanent should prompt for exile order",
        );
    }

    #[test]
    fn move_to_library_uses_order_prompt_for_split_library_cards() {
        register_test_meld_cards();
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let melded_id = create_test_melded_permanent(&mut game, alice);
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Library Bottom")
                .card_types(vec![CardType::Sorcery])
                .build(),
            alice,
            Zone::Library,
        );
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Existing Top")
                .card_types(vec![CardType::Instant])
                .build(),
            alice,
            Zone::Library,
        );
        let source = game.new_object_id();

        let mut dm = ReverseOrderDecisionMaker::default();
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        crate::effects::zones::MoveToZoneEffect::to_top_of_library(ChooseSpec::SpecificObject(
            melded_id,
        ))
        .execute(&mut game, &mut ctx)
        .expect("move to library should resolve");

        let library_names = library_top_to_bottom_names(&game, alice);
        assert_eq!(
            library_names,
            vec![
                "Graf Rats".to_string(),
                "Midnight Scavengers".to_string(),
                "Existing Top".to_string(),
                "Library Bottom".to_string(),
            ],
            "library top-to-bottom order should follow the chooser's ordering prompt",
        );
        assert!(
            dm.prompts
                .iter()
                .any(|prompt| prompt.contains("top card among them")),
            "moving a melded permanent into a library should prompt for library order",
        );
    }
}
