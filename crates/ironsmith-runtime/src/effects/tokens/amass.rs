//! Amass keyword action implementation.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::color::ColorSet;
use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::helpers::normalize_object_selection;
use crate::effects::{CreateTokenEffect, EffectExecutor, PutCountersEffect};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::object::CounterType;
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;
use crate::types::{CardType, Subtype};

/// Effect that performs the amass keyword action.
///
/// If you control no Army creature, creates a 0/0 black `[Subtype] Army` token,
/// then chooses an Army creature you control and puts N +1/+1 counters on it.
/// For amass with a subtype (e.g., "amass Orcs"), that Army also gains the
/// subtype in addition to its other types if it doesn't already have it.
#[derive(Debug, Clone, PartialEq)]
pub struct AmassEffect {
    /// Optional explicit subtype for amass variants (e.g., Orc).
    /// `None` represents classic "amass N", which defaults to Zombie.
    pub subtype: Option<Subtype>,
    /// Number of +1/+1 counters to put on the chosen Army creature.
    pub amount: u32,
}

impl AmassEffect {
    /// Create a new amass effect.
    pub fn new(subtype: Option<Subtype>, amount: u32) -> Self {
        Self { subtype, amount }
    }

    fn token_subtype(&self) -> Subtype {
        self.subtype.unwrap_or(Subtype::Zombie)
    }
}

fn army_creature_candidates(game: &GameState, controller: PlayerId) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|&id| {
            game.object(id).is_some_and(|obj| {
                obj.controller == controller
                    && game.object_has_card_type(id, CardType::Creature)
                    && game.calculated_subtypes(id).contains(&Subtype::Army)
            })
        })
        .collect()
}

fn army_token_definition(subtype: Subtype) -> CardDefinition {
    let name = format!("{subtype} Army");
    CardDefinitionBuilder::new(CardId::new(), &name)
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![subtype, Subtype::Army])
        .color_indicator(ColorSet::BLACK)
        .power_toughness(PowerToughness::fixed(0, 0))
        .build()
}

impl EffectExecutor for AmassEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let amass_subtype = self.token_subtype();
        let mut outcomes = Vec::new();

        let mut army_candidates = army_creature_candidates(game, ctx.controller);
        if army_candidates.is_empty() {
            let create_outcome = CreateTokenEffect::you(army_token_definition(amass_subtype), 1)
                .execute(game, ctx)?;
            outcomes.push(create_outcome);
            army_candidates = army_creature_candidates(game, ctx.controller);
        }

        if army_candidates.is_empty() {
            let action_event = TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(
                    KeywordActionKind::Amass,
                    ctx.controller,
                    ctx.source,
                    self.amount,
                ),
                ctx.provenance,
            );
            return Ok(EffectOutcome::aggregate(outcomes).with_event(action_event));
        }

        let chosen_army = if army_candidates.len() == 1 {
            army_candidates[0]
        } else {
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                "Choose an Army creature you control for amass",
                army_candidates.clone(),
                1,
                Some(1),
            );
            let chosen = make_decision(
                game,
                ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                spec,
            );
            let selected = normalize_object_selection(chosen, &army_candidates, 1);
            selected.first().copied().unwrap_or(army_candidates[0])
        };

        // "Amass <Subtype>" causes the chosen Army creature to become that subtype
        // in addition to its other types if it doesn't already have it.
        if !game
            .calculated_subtypes(chosen_army)
            .contains(&amass_subtype)
            && let Some(obj) = game.object_mut(chosen_army)
            && !obj.subtypes.contains(&amass_subtype)
        {
            obj.subtypes.push(amass_subtype);
        }

        let counters_outcome = PutCountersEffect::new(
            CounterType::PlusOnePlusOne,
            self.amount,
            ChooseSpec::SpecificObject(chosen_army),
        )
        .execute(game, ctx)?;
        outcomes.push(counters_outcome);

        let action_event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(
                KeywordActionKind::Amass,
                ctx.controller,
                ctx.source,
                self.amount,
            ),
            ctx.provenance,
        );
        Ok(EffectOutcome::aggregate(outcomes)
            .with_execution_fact(ExecutionFact::ChosenObjects(vec![chosen_army]))
            .with_event(action_event))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effect::{Effect, Value};
    use crate::events::DamageEvent;
    use crate::executor::{ResolvedTarget, execute_effect};
    use crate::game_event::DamageTarget;
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

        let army = game.object(existing).expect("existing army should exist");
        assert!(
            army.subtypes.contains(&Subtype::Orc),
            "expected existing Army to gain Orc subtype"
        );
        assert!(
            army.subtypes.contains(&Subtype::Zombie),
            "expected existing Army to keep prior creature subtype"
        );
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
