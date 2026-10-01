//! Optional choices retain their grammatical actor through compilation and trigger resolution.

use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, TargetsContext};
use ironsmith::events::{DamageEvent, DamageTarget};
use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Target;
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{CardDefinition, CardId, GameState, ObjectId, PlayerId, Zone};

const ALICE: PlayerId = PlayerId(0);
const BOB: PlayerId = PlayerId(1);

struct Choices {
    accept: bool,
    players: Vec<PlayerId>,
}

impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
        self.players.push(ctx.player);
        self.accept
    }

    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        SelectFirstDecisionMaker.decide_targets(game, ctx)
    }
}

fn compile(name: &str, text: &str) -> CardDefinition {
    ironsmith_registry::compile_builder_to_runtime_definition(
        ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name),
        text.to_owned(),
        false,
    )
    .unwrap_or_else(|error| panic!("{name}: {error:?}"))
}

fn resolve_damage_trigger(game: &mut GameState, source: ObjectId, choices: &mut Choices) {
    let event = TriggerEvent::new_with_provenance(
        DamageEvent::with_cause(
            source,
            DamageTarget::Player(BOB),
            3,
            true,
            ironsmith::events::cause::EventCause::effect(),
        ),
        ironsmith::provenance::ProvNodeId::default(),
    );
    let mut queue = TriggerQueue::new();
    for trigger in check_triggers(game, &event) {
        queue.add(trigger);
    }
    assert_eq!(queue.entries.len(), 1);
    put_triggers_on_stack_with_dm(game, &mut queue, choices).unwrap();
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.stack[0].controller, ALICE);
    resolve_stack_entry_with(game, choices).unwrap();
}

#[test]
fn graveyard_cast_may_belongs_to_controller_on_accept_and_decline() {
    for (name, text, graveyard_owner) in [
        (
            "Seifer Almasy",
            "Mana cost: {3}{R}\nType: Legendary Creature — Human Knight\nPower/Toughness: 3/4\nWhenever a creature you control attacks alone, it gains double strike until end of turn.\nFire Cross — Whenever Seifer Almasy deals combat damage to a player, you may cast target instant or sorcery card with mana value 3 or less from your graveyard without paying its mana cost. If that spell would be put into your graveyard, exile it instead.",
            ALICE,
        ),
        (
            "Efreet Flamepainter",
            "Mana cost: {3}{R}\nType: Creature — Efreet Shaman\nPower/Toughness: 1/4\nDouble strike\nWhenever this creature deals combat damage to a player, you may cast target instant or sorcery card from your graveyard without paying its mana cost. If that spell would be put into your graveyard, exile it instead.",
            ALICE,
        ),
        (
            "Opponent Graveyard Caster",
            "Type: Creature\nPower/Toughness: 3/4\nWhenever this creature deals combat damage to a player, you may cast target instant or sorcery card from that player's graveyard without paying its mana cost. If that spell would be put into a graveyard, exile it instead.",
            BOB,
        ),
    ] {
        let definition = compile(name, text);
        let spell = compile(
            "Life Spell",
            "Mana cost: {3}\nType: Sorcery\nYou gain 2 life.",
        );
        for accept in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&definition, ALICE, Zone::Battlefield);
            game.create_object_from_definition(&spell, graveyard_owner, Zone::Graveyard);
            let mut choices = Choices {
                accept,
                players: Vec::new(),
            };
            resolve_damage_trigger(&mut game, source, &mut choices);
            assert_eq!(choices.players, vec![ALICE], "{name}, accept={accept}");
            if accept {
                assert_eq!(game.stack.len(), 1, "cast without any mana available");
                assert_eq!(game.stack[0].controller, ALICE);
                resolve_stack_entry_with(&mut game, &mut choices).unwrap();
                assert_eq!(game.player(ALICE).unwrap().life, 22);
                assert!(
                    game.exile
                        .iter()
                        .any(|id| game.object(*id).unwrap().name == "Life Spell")
                );
                assert!(game.player(graveyard_owner).unwrap().graveyard.is_empty());
            } else {
                assert!(game.stack.is_empty());
                assert_eq!(game.player(graveyard_owner).unwrap().graveyard.len(), 1);
                assert_eq!(game.player(ALICE).unwrap().life, 20);
            }
        }
    }
}

#[test]
fn explicit_and_per_player_may_choices_keep_their_actors() {
    for (effect, expected_players, expected_life) in [
        ("you may gain 2 life.", vec![ALICE], [22, 20]),
        ("that player may gain 2 life.", vec![BOB], [20, 22]),
        ("each player may gain 2 life.", vec![ALICE, BOB], [22, 22]),
    ] {
        let definition = compile(
            "Choice Source",
            &format!(
                "Type: Creature\nPower/Toughness: 2/2\nWhenever this creature deals combat damage to a player, {effect}"
            ),
        );
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, ALICE, Zone::Battlefield);
        let mut choices = Choices {
            accept: true,
            players: Vec::new(),
        };
        resolve_damage_trigger(&mut game, source, &mut choices);
        assert_eq!(choices.players, expected_players, "{effect}");
        assert_eq!(
            [
                game.player(ALICE).unwrap().life,
                game.player(BOB).unwrap().life
            ],
            expected_life,
            "{effect}"
        );
    }
}
