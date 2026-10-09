//! cf8/p07: temporary landwalk of a land type chosen on resolution
//! ("choose a land type. <creature> gains landwalk of the chosen type until
//! end of turn"). The type is chosen for the resolving source and locked
//! into the granted landwalk as the grant resolves (CR 702.14a, 611.2c).
//! Source-authored, deliberately unrun.
#[path = "p07_support/mod.rs"]
mod support;

use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::SelectOptionsContext;
use ironsmith::effects::{ChooseLandTypeEffect, EffectContext, ResolvedTarget, execute_effect};
use ironsmith::game_state::Phase;
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

const FIXTURE: &str = include_str!("../../../fixtures/chosen_type_landwalk_grants.json.fixture");
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

/// Chooses the named option in a land-type prompt.
struct ChooseNamed(&'static str);

impl DecisionMaker for ChooseNamed {
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if let Some(option) = ctx.options.iter().find(|option| option.description == self.0) {
            return vec![option.index];
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
}

fn permanent(game: &mut GameState, controller: PlayerId, name: &str, body: &str) -> ironsmith::ObjectId {
    let definition = compile_to_runtime_definition(name, body, false).unwrap();
    game.create_object_from_definition(&definition, controller, Zone::Battlefield)
}

#[test]
fn illusionary_presence_gains_landwalk_of_the_type_chosen_each_upkeep() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Illusionary Presence");
    assert_eq!(row["oracle_id"], "8f319b31-efb0-4f48-a2a3-1f308e7b1dda");
    for definition in support::definitions(row) {
        let triggers = support::triggered(&definition);
        let upkeep = triggers
            .iter()
            .find(|trigger| format!("{:?}", trigger.effects).contains("ChooseLandType"))
            .expect("upkeep choice trigger");
        let effects = support::triggered_effects(upkeep);
        let choices = support::find::<ChooseLandTypeEffect>(&effects);
        assert_eq!(choices.len(), 1);
        assert!(!choices[0].exclude_basic && !choices[0].basic_only);
        let debug = format!("{effects:?}");
        assert!(debug.contains("ChosenType { snow: false }"), "{debug}");
        assert!(debug.contains("EndOfTurn"), "{debug}");

        // Gameplay: choosing Swamp makes it unblockable by a player who
        // controls a Swamp, and only that type.
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = A;
        game.turn.phase = Phase::Beginning;
        let presence = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let blocker = permanent(
            &mut game,
            B,
            "Witness Bear",
            "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2",
        );
        let mut dm = ChooseNamed("Swamp");
        let mut ctx = EffectContext::new(presence, A, &mut dm);
        for effect in upkeep.effects.all_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        game.refresh_continuous_state().unwrap();
        assert!(ironsmith::rules::combat::can_block(
            game.object(presence).unwrap(),
            game.object(blocker).unwrap(),
            &game
        ));
        permanent(&mut game, B, "Witness Swamp", "Type: Basic Land — Swamp");
        game.refresh_continuous_state().unwrap();
        assert!(!ironsmith::rules::combat::can_block(
            game.object(presence).unwrap(),
            game.object(blocker).unwrap(),
            &game
        ));
    }
}

#[test]
fn barbarian_guides_grants_snow_landwalk_of_the_chosen_type_to_the_target() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Barbarian Guides");
    assert_eq!(row["oracle_id"], "ef8c324a-582b-401e-add1-1985c5596908");
    for definition in support::definitions(row) {
        let activated = support::activated(&definition);
        assert_eq!(activated.len(), 1);
        let effects = support::activated_effects(activated[0]);
        let debug = format!("{effects:?}");
        assert!(debug.contains("ChooseLandTypeEffect"), "{debug}");
        assert!(debug.contains("ChosenType { snow: true }"), "{debug}");
        assert!(debug.contains("EndOfTurn"), "{debug}");

        // Gameplay: the target needs a snow land of the chosen type.
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = A;
        game.turn.phase = Phase::FirstMain;
        let guides = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let raider = permanent(
            &mut game,
            A,
            "Witness Raider",
            "Mana cost: {1}{R}\nType: Creature — Human\nPower/Toughness: 2/2",
        );
        let blocker = permanent(
            &mut game,
            B,
            "Witness Bear",
            "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2",
        );
        permanent(&mut game, B, "Witness Forest", "Type: Basic Land — Forest");
        let mut dm = ChooseNamed("Forest");
        let mut ctx = EffectContext::new(guides, A, &mut dm)
            .with_targets(vec![ResolvedTarget::Object(raider)]);
        for effect in activated[0].effects.all_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        game.refresh_continuous_state().unwrap();
        // A non-snow Forest doesn't satisfy snow forestwalk.
        assert!(ironsmith::rules::combat::can_block(
            game.object(raider).unwrap(),
            game.object(blocker).unwrap(),
            &game
        ));
        permanent(&mut game, B, "Witness Snow Forest", "Type: Basic Snow Land — Forest");
        game.refresh_continuous_state().unwrap();
        assert!(!ironsmith::rules::combat::can_block(
            game.object(raider).unwrap(),
            game.object(blocker).unwrap(),
            &game
        ));
    }
}

#[test]
fn giant_slug_chooses_a_basic_land_type_at_its_next_upkeep() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Giant Slug");
    assert_eq!(row["oracle_id"], "1ce7f356-1f9b-44dc-9b05-f7b1ecc5d755");
    for definition in support::definitions(row) {
        let activated = support::activated(&definition);
        assert_eq!(activated.len(), 1);
        let effects = support::activated_effects(activated[0]);
        let choices = support::find::<ChooseLandTypeEffect>(&effects);
        assert_eq!(choices.len(), 1, "{effects:?}");
        assert!(choices[0].basic_only);
        let debug = format!("{effects:?}");
        assert!(debug.contains("Delayed") || debug.contains("delayed"), "{debug}");
        assert!(debug.contains("ChosenType { snow: false }"), "{debug}");
        assert!(debug.contains("EndOfTurn"), "{debug}");
    }
}

#[test]
fn excavator_grants_one_landwalk_per_land_type_of_the_sacrificed_land() {
    use ironsmith::snapshot::ObjectSnapshot;
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Excavator");
    assert_eq!(row["oracle_id"], "2d5ebcd7-07c2-412e-b0d9-54e3a88895fd");
    for definition in support::definitions(row) {
        let activated = support::activated(&definition);
        assert_eq!(activated.len(), 1);
        let debug = format!("{:?}", support::activated_effects(activated[0]));
        assert!(debug.contains("SacrificedLandTypes"), "{debug}");
        assert!(debug.contains("EndOfTurn"), "{debug}");

        // Gameplay: the sacrificed land was a Swamp, so the target gains
        // swampwalk; a Forest controlled by the defender doesn't stop blocks.
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = A;
        game.turn.phase = Phase::FirstMain;
        let excavator = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let swamp = permanent(&mut game, A, "Witness Swamp", "Type: Basic Land — Swamp");
        let swamp_snapshot = ObjectSnapshot::from_object(game.object(swamp).unwrap(), &game);
        let raider = permanent(
            &mut game,
            A,
            "Witness Raider",
            "Mana cost: {1}{R}\nType: Creature — Human\nPower/Toughness: 2/2",
        );
        let blocker = permanent(
            &mut game,
            B,
            "Witness Bear",
            "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2",
        );
        permanent(&mut game, B, "Witness Forest", "Type: Basic Land — Forest");
        let mut tagged = std::collections::HashMap::new();
        tagged.insert(ironsmith::TagKey::from("sacrifice_cost_0"), vec![swamp_snapshot.clone()]);
        tagged.insert(
            ironsmith::TagKey::from("__original_sacrifice_cost_0"),
            vec![swamp_snapshot],
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(excavator, A, &mut dm)
            .with_targets(vec![ResolvedTarget::Object(raider)])
            .with_tagged_objects(tagged);
        for effect in activated[0].effects.all_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        game.refresh_continuous_state().unwrap();
        assert!(ironsmith::rules::combat::can_block(
            game.object(raider).unwrap(),
            game.object(blocker).unwrap(),
            &game
        ));
        permanent(&mut game, B, "Witness Swamp B", "Type: Basic Land — Swamp");
        game.refresh_continuous_state().unwrap();
        assert!(!ironsmith::rules::combat::can_block(
            game.object(raider).unwrap(),
            game.object(blocker).unwrap(),
            &game
        ));
    }
}
