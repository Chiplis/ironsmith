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
            assert!(text.contains("ChooseFriendsOrFoesEffect"), "{name}: {text}");
            assert!(text.contains("\"friends\""), "{name}: {text}");
            assert!(text.contains("\"foes\""), "{name}: {text}");
            assert!(text.matches("ForEachTaggedPlayerEffect").count() >= 2, "{name}: {text}");
        }
    }
}

/// Alice is a friend, Bob is a foe.
struct AliceFriend;
impl DecisionMaker for AliceFriend {
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
