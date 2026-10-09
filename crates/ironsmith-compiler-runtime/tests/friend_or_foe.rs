//! "For each player, choose friend or foe. Each friend ... Each foe ..."
//! (Battlebond). The controller designates every player on resolution
//! (CR 608.2d); each group instruction iterates its tagged players.
//! Source-authored, deliberately unrun.
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::BooleanContext;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::{CardId, CardType, GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

const VIRTUS: &str = "Mana cost: {2}{B}\nType: Sorcery\nFor each player, choose friend or foe. Each friend returns a creature card from their graveyard to their hand. Each foe sacrifices a creature of their choice.";
const PIRS_WHIM: &str = "Mana cost: {3}{G}\nType: Sorcery\nFor each player, choose friend or foe. Each friend searches their library for a land card, puts it onto the battlefield tapped, then shuffles. Each foe sacrifices an artifact or enchantment of their choice.";
const ZNDRSPLTS_JUDGMENT: &str = "Mana cost: {4}{U}\nType: Sorcery\nFor each player, choose friend or foe. Each friend creates a token that's a copy of a creature they control. Each foe returns a creature they control to its owner's hand.";

fn routes(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_artifact(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    [direct, decoded]
}

#[test]
fn friend_or_foe_spells_choose_groups_then_iterate_them() {
    for (name, text) in [
        ("Virtus's Maneuver", VIRTUS),
        ("Pir's Whim", PIRS_WHIM),
        ("Zndrsplt's Judgment", ZNDRSPLTS_JUDGMENT),
    ] {
        for definition in routes(name, text) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let text = format!("{:?}", definition.spell_effect);
            assert!(text.contains("ChoosePlayerOptionEffect"), "{name}: {text}");
            assert!(text.contains("__player_option_choice__:friend"), "{name}: {text}");
            assert!(text.contains("__player_option_choice__:foe"), "{name}: {text}");
            assert!(text.matches("ForPlayersEffect").count() >= 2, "{name}: {text}");
        }
    }
}

/// Alice is a friend, Bob is a foe.
struct AliceFriend;
impl DecisionMaker for AliceFriend {
    fn decide_options(&mut self, _game: &GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
        vec![if ctx.description.contains("Alice") { 0 } else { 1 }]
    }

    fn decide_boolean(&mut self, _game: &GameState, ctx: &BooleanContext) -> bool {
        ctx.description.contains("Alice")
    }
}

#[test]
fn virtus_returns_for_friends_and_sacrifices_for_foes() {
    for definition in routes("Virtus's Maneuver", VIRTUS) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let bear = CardDefinitionBuilder::new(CardId::new(), "Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let alice_dead = game.create_object_from_definition(&bear, A, Zone::Graveyard);
        let bob_dead = game.create_object_from_definition(&bear, B, Zone::Graveyard);
        let alice_live = game.create_object_from_definition(&bear, A, Zone::Battlefield);
        let bob_live = game.create_object_from_definition(&bear, B, Zone::Battlefield);
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        let mut dm = AliceFriend;
        let mut ctx = EffectContext::new(source, A, &mut dm);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects_owned() {
            execute_effect(&mut game, &effect, &mut ctx).unwrap();
        }
        assert!(game.object(alice_dead).is_none(), "the friend's creature card left the graveyard");
        assert!(game.player(A).unwrap().hand.len() >= 1);
        assert!(game.object(bob_dead).is_some(), "the foe returns nothing");
        assert!(game.object(alice_live).is_some(), "the friend sacrifices nothing");
        assert!(game.object(bob_live).is_none(), "the foe sacrificed its creature");
    }
}

const KHORVATHS_FURY: &str = "Mana cost: {4}{R}\nType: Sorcery\nFor each player, choose friend or foe. Each friend discards all cards from their hand, then draws that many cards plus one. Khorvath's Fury deals damage to each foe equal to the number of cards in their hand.";
const REGNAS_SANCTION: &str = "Mana cost: {3}{W}\nType: Sorcery\nFor each player, choose friend or foe. Each friend puts a +1/+1 counter on each creature they control. Each foe chooses one untapped creature they control, then taps the rest.";

#[test]
fn khorvath_and_regna_iterate_both_tagged_groups() {
    for (name, text) in [
        ("Khorvath's Fury", KHORVATHS_FURY),
        ("Regna's Sanction", REGNAS_SANCTION),
    ] {
        for definition in routes(name, text) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let text = format!("{:?}", definition.spell_effect);
            assert!(text.contains("ChoosePlayerOptionEffect"), "{name}: {text}");
            assert!(text.matches("ForPlayersEffect").count() >= 2, "{name}: {text}");
        }
    }
    for definition in routes("Khorvath's Fury", KHORVATHS_FURY) {
        let text = format!("{:?}", definition.spell_effect);
        assert!(text.contains("DiscardHand") || text.contains("Discard"), "{text}");
        assert!(text.contains("CardsInHand(IteratedPlayer)"), "{text}");
    }
    for definition in routes("Regna's Sanction", REGNAS_SANCTION) {
        let text = format!("{:?}", definition.spell_effect);
        assert!(text.contains("ChooseObjectsEffect"), "{text}");
        assert!(text.contains("TapEffect"), "{text}");
        assert!(!text.contains("SacrificeEffect"), "the rest are tapped, not sacrificed: {text}");
    }
}

#[test]
fn regna_counters_friends_and_taps_all_but_one_foe_creature() {
    for definition in routes("Regna's Sanction", REGNAS_SANCTION) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let bear = CardDefinitionBuilder::new(CardId::new(), "Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let ally = game.create_object_from_definition(&bear, A, Zone::Battlefield);
        let foe_one = game.create_object_from_definition(&bear, B, Zone::Battlefield);
        let foe_two = game.create_object_from_definition(&bear, B, Zone::Battlefield);
        let foe_three = game.create_object_from_definition(&bear, B, Zone::Battlefield);
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        let mut dm = AliceFriend;
        let mut ctx = EffectContext::new(source, A, &mut dm);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects_owned() {
            execute_effect(&mut game, &effect, &mut ctx).unwrap();
        }
        assert_eq!(game.counter_count(ally, ironsmith::CounterType::PlusOnePlusOne), 1);
        assert!(!game.is_tapped(ally), "friends are not tapped");
        let untapped = [foe_one, foe_two, foe_three]
            .into_iter()
            .filter(|id| !game.is_tapped(*id))
            .count();
        assert_eq!(untapped, 1, "the foe keeps exactly its chosen creature untapped");
        for id in [foe_one, foe_two, foe_three] {
            assert_eq!(game.counter_count(id, ironsmith::CounterType::PlusOnePlusOne), 0);
        }
    }
}

#[test]
fn khorvath_refills_friends_and_burns_foes_by_hand_size() {
    for definition in routes("Khorvath's Fury", KHORVATHS_FURY) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let filler = CardDefinitionBuilder::new(CardId::new(), "Filler")
            .card_types(vec![CardType::Sorcery])
            .build();
        for _ in 0..2 {
            game.create_object_from_definition(&filler, A, Zone::Hand);
        }
        for _ in 0..3 {
            game.create_object_from_definition(&filler, B, Zone::Hand);
        }
        for _ in 0..10 {
            game.create_object_from_definition(&filler, A, Zone::Library);
            game.create_object_from_definition(&filler, B, Zone::Library);
        }
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        let mut dm = AliceFriend;
        let mut ctx = EffectContext::new(source, A, &mut dm);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects_owned() {
            execute_effect(&mut game, &effect, &mut ctx).unwrap();
        }
        assert_eq!(game.player(A).unwrap().hand.len(), 3, "discard two, draw two plus one");
        assert_eq!(game.player(A).unwrap().life, 20);
        assert_eq!(game.player(B).unwrap().hand.len(), 3, "foes keep their hand");
        assert_eq!(game.player(B).unwrap().life, 17, "three cards in hand -> 3 damage");
    }
}
