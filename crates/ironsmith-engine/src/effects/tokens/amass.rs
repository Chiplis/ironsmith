//! Amass keyword action implementation.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::color::ColorSet;
use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::helpers::resolve_value;
use crate::effects::{CreateTokenEffect, EffectExecutor, PutCountersEffect};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::object::CounterType;
use crate::target::ChooseSpec;
use crate::types::{CardType, Subtype};

/// Effect that performs the amass keyword action.
///
/// If you control no Army creature, creates a 0/0 black `[Subtype] Army` token,
/// then chooses an Army creature you control and puts N +1/+1 counters on it.
/// For amass with a subtype (e.g., "amass Orcs"), that Army also gains the
/// subtype in addition to its other types if it doesn't already have it.
pub type AmassEffect = ironsmith_core::AmassEffect;

fn amass_token_subtype(effect: &AmassEffect) -> Subtype {
    effect.subtype.unwrap_or(Subtype::Zombie)
}

fn army_creature_candidates(game: &GameState, controller: PlayerId) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        // CR 702.26b: a phased-out Army is treated as though it doesn't exist.
        .filter(|&id| !game.is_phased_out(id))
        .filter(|&id| {
            game.object(id).is_some_and(|obj| {
                game.controller_of(obj) == controller
                    && game.object_has_card_type(id, CardType::Creature)
                    && game.calculated_subtypes(id).contains(&Subtype::Army)
            })
        })
        .collect()
}

fn army_token_definition(subtype: Subtype) -> Result<CardDefinition, ExecutionError> {
    army_token_blueprint(subtype, CardId::new())
}

fn army_token_blueprint(subtype: Subtype, card_id: CardId) -> Result<CardDefinition, ExecutionError> {
    let subtypes = vec![subtype, Subtype::Army];
    let name = ironsmith_core::subtype_derived_token_name(&subtypes).ok_or_else(||
        ExecutionError::IncompleteEvidence("amass token name requires canonical subtype spellings".into()))?;
    Ok(CardDefinitionBuilder::new(card_id, &name)
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(subtypes)
        .color_indicator(ColorSet::BLACK)
        .power_toughness(PowerToughness::fixed(0, 0))
        .build())
}

impl EffectExecutor for AmassEffect {
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&crate::effect::Effect)) {
        if let Ok(token) = army_token_blueprint(amass_token_subtype(self), CardId::from_raw(0)) {
            visitor(&crate::effect::Effect::new(CreateTokenEffect::you(token, 1)));
        }
        visitor(&crate::effect::Effect::new(PutCountersEffect::new(
            CounterType::PlusOnePlusOne, self.amount.clone(),
            ChooseSpec::Object(crate::target::ObjectFilter::creature().with_subtype(Subtype::Army)))));
        visitor(&crate::effect::Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::Object(crate::target::ObjectFilter::creature().with_subtype(Subtype::Army)),
            crate::continuous::Modification::AddSubtypes(vec![amass_token_subtype(self)]),
            crate::effect::Until::Forever)));
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        // Another player amasses: that player chooses or creates the Army
        // under their own control (CR 701.47a).
        if let Some(player) = &self.player {
            let player = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let mut own = self.clone();
            own.player = None;
            if player != ctx.controller {
                return crate::effects::composition::execute_with_source::with_actor_binding(
                    ctx,
                    player,
                    None,
                    |ctx| own.execute_with_outputs(game, ctx),
                );
            }
            return own.execute_with_outputs(game, ctx);
        }
        super::lifecycle::execute_token_instruction_with_pending_value(
            game,
            ctx,
            || {
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::with_objects(
                    Vec::new(),
                ))
            },
            |game, ctx| {
                let amass_subtype = amass_token_subtype(self);
                let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;
                let (chosen, children) = crate::effects::composition::ensure_chosen_object(
                    game, ctx, "Choose an Army creature you control for amass",
                    army_creature_candidates,
                    |game, ctx| {
                        let token = army_token_definition(amass_subtype)?;
                        let roles = ironsmith_core::TokenTextRoles::rules_implied(
                            ironsmith_core::TokenNameTextRole::SubtypeDerived, token.abilities.len());
                        CreateTokenEffect::you(token, 1).with_text_roles(roles)
                            .execute_child_with_outputs(game, ctx)
                    },
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::resolved()));
                }
                let mut outcomes = children.iter().map(|child| child.outcome.clone()).collect::<Vec<_>>();
                let mut outputs = crate::effects::composition::aggregate_children(children);
                let Some(chosen_army) = chosen else {
                    if self.descriptive { return Ok(outputs); }
                    return crate::effects::composition::complete_keyword_action_with_outputs(
                        game, ctx, outputs,
                        KeywordActionEvent::new(KeywordActionKind::Amass, ctx.controller, ctx.source, amount));
                };

                // CR 701.47a places counters before adding the amass subtype. Counter
                // replacements inspect the Army's characteristics at that earlier step.
                let counters_outcome = PutCountersEffect::new(
                    CounterType::PlusOnePlusOne,
                    amount,
                    ChooseSpec::SpecificObject(chosen_army),
                )
                .execute_child_with_outputs(game, ctx)?;
                outcomes.push(counters_outcome.outcome.clone());
                outputs.retain_owned_child(counters_outcome);
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::resolved(),
                    ));
                }

                // "Amass <Subtype>" causes the chosen Army creature to become that subtype
                // in addition to its other types if it doesn't already have it. That is
                // a type-changing (layer 4) effect, not a copiable value (CR 701.47a).
                if !game
                    .calculated_subtypes(chosen_army)
                    .contains(&amass_subtype)
                {
                    let become_subtype = crate::effects::ApplyContinuousEffect::with_spec(
                        ChooseSpec::SpecificObject(chosen_army),
                        crate::continuous::Modification::AddSubtypes(vec![amass_subtype]),
                        crate::effect::Until::Forever,
                    );
                    let child = become_subtype.execute_child_with_outputs(game, ctx)?;
                    outcomes.push(child.outcome.clone());
                    outputs.retain_owned_child(child);
                }

                let outputs = outputs.project_aggregate(EffectOutcome::aggregate(outcomes)
                    .with_execution_fact(ExecutionFact::ChosenObjects(vec![chosen_army])));
                if self.descriptive { return Ok(outputs); }
                crate::effects::composition::complete_keyword_action_with_outputs(
                    game,
                    ctx,
                    outputs,
                    KeywordActionEvent::new(
                        KeywordActionKind::Amass,
                        ctx.controller,
                        ctx.source,
                        amount,
                    ),
                )
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effect::{Effect, Value};
    use crate::effects::{ResolvedTarget, execute_effect};
    use crate::events::DamageEvent;
    use crate::events::DamageTarget;
    use crate::tag::TagKey;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn add_army_creature(
        game: &mut GameState,
        controller: PlayerId,
        subtypes: Vec<Subtype>,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), "Army Test Creature")
            .card_types(vec![CardType::Creature])
            .subtypes(subtypes)
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    #[test]
    fn native_amass_token_blueprints_use_canonical_rules_names_without_added_abilities() {
        for (subtype, expected) in [(Subtype::Goblin, "Goblin Army Token"), (Subtype::Orc, "Orc Army Token"),
            (Subtype::Zombie, "Zombie Army Token")] {
            let token = army_token_definition(subtype).unwrap();
            assert_eq!(token.card.name, expected);
            assert_eq!(token.card.subtypes, vec![subtype, Subtype::Army]);
            assert!(token.abilities.is_empty());
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let source = game.new_object_id();
            AmassEffect::new(Some(subtype), 3).execute(&mut game,
                &mut ExecutionContext::new_default(source, alice)).unwrap();
            let army = army_creature_candidates(&game, alice)[0];
            assert_eq!(game.object(army).unwrap().name, expected);
            assert_eq!(game.current_power(army), Some(3));
            assert_eq!(game.current_toughness(army), Some(3));
            assert_eq!(game.current_colors(army), Some(ColorSet::BLACK));
        }
    }

    #[test]
    fn unsupported_native_amass_subtype_name_fails_before_creating_any_token() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let next = game.next_object_id_counter();
        let result = AmassEffect::new(Some(Subtype::Ajani), 3).execute(&mut game,
            &mut ExecutionContext::new_default(source, alice));
        assert!(matches!(result, Err(ExecutionError::IncompleteEvidence(_))));
        assert!(game.battlefield.is_empty());
        assert_eq!(game.next_object_id_counter(), next);
        assert!(game.take_pending_trigger_events().is_empty());
    }

    #[test]
    fn amass_creates_zombie_army_when_none_exists() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AmassEffect::new(None, 2);
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("amass should resolve");

        let armies: Vec<ObjectId> = army_creature_candidates(&game, alice);
        assert_eq!(armies.len(), 1, "expected exactly one Army creature");
        let army = game.object(armies[0]).expect("army should exist");
        assert_eq!(army.name, "Zombie Army Token");
        assert!(
            army.subtypes.contains(&Subtype::Zombie),
            "expected classic amass to create Zombie Army"
        );
        assert_eq!(
            army.counters
                .get(&CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0),
            2,
            "expected two +1/+1 counters on created Army"
        );
        assert!(
            outcome.events.iter().any(|event| {
                event
                    .downcast::<KeywordActionEvent>()
                    .is_some_and(|action| action.action == KeywordActionKind::Amass)
            }),
            "expected keyword action event for amass"
        );
    }

    #[test]
    fn amass_orcs_adds_orc_subtype_to_existing_army() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let existing = add_army_creature(&mut game, alice, vec![Subtype::Zombie, Subtype::Army]);
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AmassEffect::new(Some(Subtype::Orc), 1);
        let _ = effect
            .execute(&mut game, &mut ctx)
            .expect("amass orcs should resolve");

        // CR 701.47a: becoming an Orc is a type-changing effect (layer 4),
        // not a change to the Army's copiable subtypes.
        let subtypes = game.calculated_subtypes(existing);
        assert!(
            subtypes.contains(&Subtype::Orc),
            "expected existing Army to gain Orc subtype"
        );
        assert!(
            subtypes.contains(&Subtype::Zombie),
            "expected existing Army to keep prior creature subtype"
        );
        let army = game.object(existing).expect("existing army should exist");
        assert_eq!(army.name, "Army Test Creature", "amass preserves the existing object's name");
        assert!(!army.subtypes.contains(&Subtype::Orc), "not a copiable value");
        assert_eq!(
            army.counters
                .get(&CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0),
            1,
            "expected one +1/+1 counter on chosen Army"
        );
        assert_eq!(
            army_creature_candidates(&game, alice).len(),
            1,
            "amass should not create a new Army when one already exists"
        );
    }

    #[test]
    fn tagged_amass_preserves_the_chosen_army_for_followup_provenance() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let first_army = add_army_creature(&mut game, alice, vec![Subtype::Zombie, Subtype::Army]);
        let second_army = add_army_creature(&mut game, alice, vec![Subtype::Orc, Subtype::Army]);
        game.add_counters_with_source(
            second_army,
            CounterType::PlusOnePlusOne,
            5,
            Some(source),
            Some(alice),
        );
        let victim = add_army_creature(&mut game, bob, vec![Subtype::Goblin, Subtype::Warrior]);

        let mut ctx = ExecutionContext::new_default(source, alice);

        let amass = Effect::amass(Some(Subtype::Orc), 2).tag("amassed");
        execute_effect(&mut game, &amass, &mut ctx).expect("amass should resolve");

        let tagged = ctx
            .get_tagged("amassed")
            .expect("chosen Army should be tagged");
        assert_eq!(
            tagged.object_id, first_army,
            "amass should tag the specifically chosen Army, not an arbitrary Army"
        );

        let followup = Effect::new(crate::effects::ExecuteWithSourceEffect::new(
            ChooseSpec::Tagged(TagKey::from("amassed")),
            Effect::deal_damage(
                Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from("amassed")))),
                ChooseSpec::AnyTarget,
            ),
        ));
        let outcome = ctx
            .with_temp_targets(vec![ResolvedTarget::Object(victim)], |ctx| {
                execute_effect(&mut game, &followup, ctx)
            })
            .expect("follow-up damage should work");
        let events_debug = format!("{:?}", outcome.events);

        assert!(
            outcome.events.iter().any(|event| {
                event.downcast::<DamageEvent>().is_some_and(|damage| {
                    damage.source == first_army
                        && damage.amount == 3
                        && matches!(damage.target, DamageTarget::Object(id) if id == victim)
                })
            }),
            "expected damage from the chosen Army, got {events_debug}"
        );
    }

    #[test]
    fn tagged_amass_with_zero_counters_still_tags_the_chosen_army() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let first_army = add_army_creature(&mut game, alice, vec![Subtype::Zombie, Subtype::Army]);
        let second_army = add_army_creature(&mut game, alice, vec![Subtype::Orc, Subtype::Army]);
        game.add_counters_with_source(
            second_army,
            CounterType::PlusOnePlusOne,
            4,
            Some(source),
            Some(alice),
        );
        let victim = add_army_creature(&mut game, bob, vec![Subtype::Goblin, Subtype::Warrior]);

        let initial_first_counters = game
            .object(first_army)
            .expect("first Army should exist")
            .counters
            .get(&CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0);

        let mut ctx = ExecutionContext::new_default(source, alice);

        let amass = Effect::amass(Some(Subtype::Orc), 0).tag("amassed");
        execute_effect(&mut game, &amass, &mut ctx).expect("zero-counter amass should resolve");

        let tagged = ctx
            .get_tagged("amassed")
            .expect("chosen Army should still be tagged");
        assert_eq!(
            tagged.object_id, first_army,
            "amass should remember the chosen Army even if no counters were added"
        );
        assert_eq!(
            game.object(first_army)
                .expect("first Army should still exist")
                .counters
                .get(&CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0),
            initial_first_counters,
            "zero-counter amass should not add counters to the chosen Army"
        );

        let followup = Effect::new(crate::effects::ExecuteWithSourceEffect::new(
            ChooseSpec::Tagged(TagKey::from("amassed")),
            Effect::deal_damage(
                Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from("amassed")))),
                ChooseSpec::AnyTarget,
            ),
        ));
        let outcome = ctx
            .with_temp_targets(vec![ResolvedTarget::Object(victim)], |ctx| {
                execute_effect(&mut game, &followup, ctx)
            })
            .expect("follow-up damage should use the chosen Army");
        let events_debug = format!("{:?}", outcome.events);

        assert!(
            outcome.events.iter().any(|event| {
                event.downcast::<DamageEvent>().is_some_and(|damage| {
                    damage.source == first_army
                        && damage.amount == 1
                        && matches!(damage.target, DamageTarget::Object(id) if id == victim)
                })
            }),
            "expected damage from the chosen zero-counter Army, got {events_debug}"
        );
    }
}
