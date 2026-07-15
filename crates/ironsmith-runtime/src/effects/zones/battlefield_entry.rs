use crate::effects::ExecutionContext;
use crate::effects::ExecutionError;
use crate::effects::helpers::resolve_value;
use crate::events::EnterBattlefieldEvent;
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::provenance::ProvNodeId;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

/// Controller policy when an object enters the battlefield.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattlefieldEntryController {
    Preserve,
    Owner,
    Specific(PlayerId),
}

/// Config for moving an object to the battlefield through ETB processing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BattlefieldEntryOptions {
    pub controller: BattlefieldEntryController,
    pub tapped: bool,
    pub initial_counters: Vec<(crate::object::CounterType, u32)>,
}

impl BattlefieldEntryOptions {
    pub(crate) fn preserve(tapped: bool) -> Self {
        Self {
            controller: BattlefieldEntryController::Preserve,
            tapped,
            initial_counters: Vec::new(),
        }
    }

    pub(crate) fn owner(tapped: bool) -> Self {
        Self {
            controller: BattlefieldEntryController::Owner,
            tapped,
            initial_counters: Vec::new(),
        }
    }

    pub(crate) fn specific(controller: PlayerId, tapped: bool) -> Self {
        Self {
            controller: BattlefieldEntryController::Specific(controller),
            tapped,
            initial_counters: Vec::new(),
        }
    }

    pub(crate) fn with_initial_counters(
        mut self,
        counters: Vec<(crate::object::CounterType, u32)>,
    ) -> Self {
        self.initial_counters = counters;
        self
    }
}

/// Resolve authored one-shot entry-counter metadata before the object changes
/// zones. The resulting concrete counters are passed into ETB replacement
/// processing as part of the original enter event.
pub(crate) fn resolve_battlefield_entry_counters(
    game: &GameState,
    ctx: &ExecutionContext,
    object_id: ObjectId,
    specs: &[ironsmith_core::BattlefieldEntryCounterSpec],
) -> Result<Vec<(crate::object::CounterType, u32)>, ExecutionError> {
    let mut counters = Vec::new();
    for spec in specs {
        if let Some(condition) = &spec.condition
            && !crate::condition_eval::evaluate_condition_resolution(game, condition, ctx)?
        {
            continue;
        }
        if let Some(filter) = &spec.object_filter {
            let Some(object) = game.object(object_id) else {
                continue;
            };
            let filter_ctx = ctx.filter_context(game);
            if !filter.matches(object, &filter_ctx, game) {
                continue;
            }
        }
        let amount = resolve_value(game, &spec.amount, ctx)?.max(0) as u32;
        if amount > 0 {
            counters.push((spec.counter_type, amount));
        }
    }
    Ok(counters)
}

/// Result for a move-to-battlefield attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattlefieldEntryOutcome {
    Moved(ObjectId),
    Prevented,
}

/// Move an object to the battlefield with ETB replacement processing and policy hooks.
pub(crate) fn move_to_battlefield_with_options(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    object_id: ObjectId,
    options: BattlefieldEntryOptions,
) -> BattlefieldEntryOutcome {
    let old_zone = game.object(object_id).map(|obj| obj.zone);
    let entering_controller = match options.controller {
        BattlefieldEntryController::Specific(controller) => Some(controller),
        BattlefieldEntryController::Preserve | BattlefieldEntryController::Owner => None,
    };
    let Some(result) = game
        .move_object_with_etb_processing_with_initial_counters_and_controller_with_dm(
            object_id,
            Zone::Battlefield,
            options.initial_counters.clone(),
            entering_controller,
            &mut ctx.decision_maker,
        )
    else {
        return BattlefieldEntryOutcome::Prevented;
    };

    let new_id = result.new_id;

    if game
        .object(new_id)
        .is_none_or(|obj| obj.zone != Zone::Battlefield)
    {
        return BattlefieldEntryOutcome::Prevented;
    }

    game.add_battlefield_put_with_source_link(ctx.source, new_id);

    let enters_tapped = result.enters_tapped || options.tapped;
    if options.tapped && !result.enters_tapped {
        game.tap(new_id);
    }

    if let Some(from_zone) = old_zone {
        let event = if enters_tapped {
            TriggerEvent::new_with_provenance(
                EnterBattlefieldEvent::tapped(new_id, from_zone),
                ProvNodeId::default(),
            )
        } else {
            TriggerEvent::new_with_provenance(
                EnterBattlefieldEvent::new(new_id, from_zone),
                ProvNodeId::default(),
            )
        };
        game.queue_trigger_event(ctx.provenance, event);
    }

    BattlefieldEntryOutcome::Moved(new_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::SelectObjectsContext;
    use crate::ids::CardId;
    use crate::object::{AttachmentTarget, AuraAttachmentFilter, Object};
    use crate::target::ObjectFilter;
    use crate::types::{CardType, Subtype};

    fn create_creature(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.add_object(Object::from_card(id, &card, owner, Zone::Battlefield));
        id
    }

    fn create_hand_aura(
        game: &mut GameState,
        name: &str,
        owner: PlayerId,
        filter: ObjectFilter,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .build();
        let mut object = Object::from_card(id, &card, owner, Zone::Hand);
        object.aura_attach_filter = Some(AuraAttachmentFilter::from(filter).into());
        game.add_object(object);
        id
    }

    struct ChooseObjectDm {
        desired: ObjectId,
        chooser: Option<PlayerId>,
    }

    impl DecisionMaker for ChooseObjectDm {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.chooser = Some(ctx.player);
            ctx.candidates
                .iter()
                .any(|candidate| candidate.id == self.desired)
                .then_some(vec![self.desired])
                .unwrap_or_default()
        }
    }

    #[test]
    fn nonspell_aura_entry_uses_the_entering_controller_to_choose_attachment() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let _alice_creature = create_creature(&mut game, "Alice Bear", alice);
        let bob_creature = create_creature(&mut game, "Bob Bear", bob);
        let aura = create_hand_aura(
            &mut game,
            "Borrowed Blessing",
            alice,
            ObjectFilter::creature().you_control(),
        );
        let mut dm = ChooseObjectDm {
            desired: bob_creature,
            chooser: None,
        };

        let outcome = {
            let mut ctx = ExecutionContext::new(ObjectId::from_raw(9000), bob, &mut dm);
            move_to_battlefield_with_options(
                &mut game,
                &mut ctx,
                aura,
                BattlefieldEntryOptions::specific(bob, false),
            )
        };

        let BattlefieldEntryOutcome::Moved(new_id) = outcome else {
            panic!("Aura should enter attached");
        };
        assert_eq!(dm.chooser, Some(bob));
        assert_eq!(game.current_controller(new_id), Some(bob));
        assert_eq!(
            game.object(new_id).and_then(|object| object.attached_to),
            Some(AttachmentTarget::Object(bob_creature))
        );
    }

    #[test]
    fn nonspell_aura_with_no_legal_attachment_remains_in_its_current_zone() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let aura = create_hand_aura(
            &mut game,
            "Lonely Blessing",
            alice,
            ObjectFilter::creature(),
        );
        let mut ctx = ExecutionContext::new_default(ObjectId::from_raw(9001), alice);

        let outcome = move_to_battlefield_with_options(
            &mut game,
            &mut ctx,
            aura,
            BattlefieldEntryOptions::specific(alice, false),
        );

        assert_eq!(outcome, BattlefieldEntryOutcome::Prevented);
        assert_eq!(
            game.object(aura).map(|object| object.zone),
            Some(Zone::Hand)
        );
        assert!(
            game.player(alice)
                .is_some_and(|player| player.hand.contains(&aura))
        );
        assert!(game.battlefield.iter().all(|id| *id != aura));
        assert!(
            game.player(alice)
                .is_some_and(|player| player.graveyard.is_empty())
        );
    }
}
