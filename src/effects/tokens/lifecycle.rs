//! Shared token lifecycle helpers.

#[cfg(test)]
use crate::ability::Ability;
use crate::effect::Effect;
use crate::effects::{EnterAttackingEffect, SacrificeTargetEffect, ScheduleDelayedTriggerEffect};
use crate::events::EnterBattlefieldEvent;
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget, execute_effect};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
#[cfg(test)]
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::triggers::{Trigger, TriggerEvent};
use crate::zone::Zone;

/// Entry-processing options for newly created tokens.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TokenEntryOptions {
    pub enters_tapped: bool,
    pub enters_attacking: bool,
}

impl TokenEntryOptions {
    pub fn new(enters_tapped: bool, enters_attacking: bool) -> Self {
        Self {
            enters_tapped,
            enters_attacking,
        }
    }
}

/// Apply common post-create entry processing for a token that entered the battlefield.
pub(crate) fn apply_token_battlefield_entry(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    token_id: ObjectId,
    _controller_id: PlayerId,
    _token_is_creature: bool,
    options: TokenEntryOptions,
    from_zone: Zone,
    enters_tapped: bool,
    events: &mut Vec<TriggerEvent>,
) -> Result<(), ExecutionError> {
    if enters_tapped && !game.is_tapped(token_id) {
        game.tap(token_id);
    }
    // Tokens always have summoning sickness.
    game.set_summoning_sick(token_id);

    // Zone-change events are queued by `move_object_with_etb_processing_with_dm`.
    // Emit the explicit ETB event here so enters-tapped/untapped triggers can match.
    let etb_event = if enters_tapped {
        TriggerEvent::new_with_provenance(
            EnterBattlefieldEvent::tapped(token_id, from_zone),
            ctx.provenance,
        )
    } else {
        TriggerEvent::new_with_provenance(
            EnterBattlefieldEvent::new(token_id, from_zone),
            ctx.provenance,
        )
    };
    events.push(etb_event);

    if options.enters_attacking {
        ctx.with_temp_targets(vec![ResolvedTarget::Object(token_id)], |ctx| {
            let enter_attacking = EnterAttackingEffect::new(ChooseSpec::AnyTarget);
            execute_effect(game, &Effect::new(enter_attacking), ctx).map(|_| ())
        })?;
    }

    Ok(())
}

/// Grant a sequence of static abilities to a created token.
#[cfg(test)]
pub(crate) fn grant_token_static_abilities(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    token_id: ObjectId,
    static_abilities: &[StaticAbility],
) -> Result<(), ExecutionError> {
    for static_ability in static_abilities {
        ctx.with_temp_targets(vec![ResolvedTarget::Object(token_id)], |ctx| {
            let grant_effect = crate::effects::GrantObjectAbilityEffect::new(
                Ability::static_ability(static_ability.clone()),
                ChooseSpec::AnyTarget,
            );
            execute_effect(game, &Effect::new(grant_effect), ctx).map(|_| ())
        })?;
    }

    Ok(())
}

/// Delayed-cleanup scheduling options for newly created tokens.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TokenCleanupOptions {
    pub exile_at_end_of_combat: bool,
    pub sacrifice_at_end_of_combat: bool,
    pub sacrifice_at_next_end_step: bool,
    pub exile_at_next_end_step: bool,
}

impl TokenCleanupOptions {
    pub fn new(
        exile_at_end_of_combat: bool,
        sacrifice_at_end_of_combat: bool,
        sacrifice_at_next_end_step: bool,
        exile_at_next_end_step: bool,
    ) -> Self {
        Self {
            exile_at_end_of_combat,
            sacrifice_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
        }
    }
}

/// Schedule configured delayed cleanup for a token.
pub(crate) fn schedule_token_cleanup(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    token_id: ObjectId,
    controller_id: PlayerId,
    options: TokenCleanupOptions,
) -> Result<(), ExecutionError> {
    if options.exile_at_end_of_combat {
        schedule_token_delayed_effect(
            game,
            ctx,
            token_id,
            controller_id,
            Trigger::end_of_combat(),
            vec![Effect::exile(ChooseSpec::SpecificObject(token_id))],
        )?;
    }

    if options.sacrifice_at_end_of_combat {
        schedule_token_delayed_effect(
            game,
            ctx,
            token_id,
            controller_id,
            Trigger::end_of_combat(),
            vec![Effect::new(SacrificeTargetEffect::new(
                ChooseSpec::SpecificObject(token_id),
            ))],
        )?;
    }

    if options.sacrifice_at_next_end_step {
        schedule_token_delayed_effect(
            game,
            ctx,
            token_id,
            controller_id,
            Trigger::beginning_of_end_step(PlayerFilter::Any),
            vec![Effect::new(SacrificeTargetEffect::new(
                ChooseSpec::SpecificObject(token_id),
            ))],
        )?;
    }

    if options.exile_at_next_end_step {
        schedule_token_delayed_effect(
            game,
            ctx,
            token_id,
            controller_id,
            Trigger::beginning_of_end_step(PlayerFilter::Any),
            vec![Effect::exile(ChooseSpec::SpecificObject(token_id))],
        )?;
    }

    Ok(())
}

fn schedule_token_delayed_effect(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    token_id: ObjectId,
    controller_id: PlayerId,
    trigger: Trigger,
    effects: Vec<Effect>,
) -> Result<(), ExecutionError> {
    let schedule = ScheduleDelayedTriggerEffect::new(
        trigger,
        effects,
        true,
        vec![token_id],
        PlayerFilter::Specific(controller_id),
    );
    let _ = execute_effect(game, &Effect::new(schedule), ctx)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::color::ColorSet;
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use crate::events::EventKind;
    use crate::executor::ExecutionContext;
    use crate::ids::PlayerId;
    use crate::object::Object;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_schedule_token_cleanup_no_flags_is_noop() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let token_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        schedule_token_cleanup(
            &mut game,
            &mut ctx,
            token_id,
            alice,
            TokenCleanupOptions::default(),
        )
        .unwrap();

        assert_eq!(game.delayed_triggers.len(), 0);
    }

    #[test]
    fn test_schedule_token_cleanup_exile_at_end_of_combat() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let token_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        schedule_token_cleanup(
            &mut game,
            &mut ctx,
            token_id,
            bob,
            TokenCleanupOptions::new(true, false, false, false),
        )
        .unwrap();

        assert_eq!(game.delayed_triggers.len(), 1);
        let delayed = &game.delayed_triggers[0];
        assert_eq!(
            delayed.trigger.display(),
            Trigger::end_of_combat().display()
        );
        assert!(delayed.one_shot);
        assert_eq!(delayed.target_objects, vec![token_id]);
        assert_eq!(delayed.controller, bob);
    }

    #[test]
    fn test_schedule_token_cleanup_all_flags() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let token_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        schedule_token_cleanup(
            &mut game,
            &mut ctx,
            token_id,
            alice,
            TokenCleanupOptions::new(true, true, true, true),
        )
        .unwrap();

        assert_eq!(game.delayed_triggers.len(), 4);
        let end_of_combat_display = Trigger::end_of_combat().display();
        let end_step_display = Trigger::beginning_of_end_step(PlayerFilter::Any).display();
        let end_of_combat_count = game
            .delayed_triggers
            .iter()
            .filter(|delayed| delayed.trigger.display() == end_of_combat_display)
            .count();
        let end_step_count = game
            .delayed_triggers
            .iter()
            .filter(|delayed| delayed.trigger.display() == end_step_display)
            .count();
        assert_eq!(end_of_combat_count, 2);
        assert_eq!(end_step_count, 2);
        for delayed in &game.delayed_triggers {
            assert!(delayed.one_shot);
            assert_eq!(delayed.target_objects, vec![token_id]);
            assert_eq!(delayed.controller, alice);
        }
    }

    #[test]
    fn test_apply_token_battlefield_entry_sets_flags_and_events() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let token_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let mut events = Vec::new();

        apply_token_battlefield_entry(
            &mut game,
            &mut ctx,
            token_id,
            alice,
            true,
            TokenEntryOptions::new(true, false),
            Zone::Command,
            true,
            &mut events,
        )
        .unwrap();

        assert!(game.is_tapped(token_id));
        assert!(game.is_summoning_sick(token_id));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind(), EventKind::EnterBattlefield);
        let etb = events[0]
            .downcast::<EnterBattlefieldEvent>()
            .expect("expected EnterBattlefieldEvent");
        assert!(etb.enters_tapped);
    }

    #[test]
    fn test_apply_token_battlefield_entry_enters_attacking() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let token_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let mut events = Vec::new();

        game.combat = Some(CombatState {
            attackers: vec![AttackerInfo {
                creature: source,
                target: AttackTarget::Player(bob),
            }],
            ..CombatState::default()
        });

        apply_token_battlefield_entry(
            &mut game,
            &mut ctx,
            token_id,
            alice,
            true,
            TokenEntryOptions::new(false, true),
            Zone::Command,
            false,
            &mut events,
        )
        .unwrap();

        let combat = game.combat.as_ref().expect("combat should exist");
        assert!(
            combat.attackers.iter().any(|attacker| {
                attacker.creature == token_id && attacker.target == AttackTarget::Player(bob)
            }),
            "token should enter attacking same target as source"
        );
    }

    #[test]
    fn test_grant_token_static_abilities() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let token_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let token = Object::new_token(
            token_id,
            alice,
            "Token".to_string(),
            vec![CardType::Creature],
            Vec::new(),
            Some(1),
            Some(1),
            ColorSet::default(),
        );
        game.add_object(token);

        grant_token_static_abilities(
            &mut game,
            &mut ctx,
            token_id,
            &[StaticAbility::haste(), StaticAbility::flying()],
        )
        .unwrap();

        let token = game.object(token_id).expect("token should exist");
        let has_haste = token.abilities.iter().any(|ability| {
            if let AbilityKind::Static(static_ability) = &ability.kind {
                static_ability.has_haste()
            } else {
                false
            }
        });
        let has_flying = token.abilities.iter().any(|ability| {
            if let AbilityKind::Static(static_ability) = &ability.kind {
                static_ability.has_flying()
            } else {
                false
            }
        });
        assert!(has_haste, "token should gain haste");
        assert!(has_flying, "token should gain flying");
    }
}
