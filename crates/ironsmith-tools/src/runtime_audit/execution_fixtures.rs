use super::create_fixture_object;
use ironsmith::card::PowerToughness;
use ironsmith::cards::CardDefinition;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::events::*;
use ironsmith::game_state::{Phase, Step};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::CounterType;
use ironsmith::snapshot::ObjectSnapshot;
use ironsmith::triggers::TriggerEvent;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Subtype, Zone};

pub const SEED: u64 = 0x4952_4f4e_534d_4954;

pub struct Seed {
    pub game: GameState,
    pub source: ObjectId,
    creatures: Vec<ObjectId>,
    lands: Vec<ObjectId>,
    spells: Vec<ObjectId>,
    hands: Vec<ObjectId>,
}

pub struct Fixture {
    pub name: String,
    pub event: TriggerEvent,
    active: PlayerId,
    phase: Phase,
    step: Option<Step>,
}

impl Fixture {
    pub fn prepare(&self, game: &mut GameState) {
        game.turn.active_player = self.active;
        game.turn.phase = self.phase;
        game.turn.step = self.step;
        // Synthetic historical events are not fed back through the event
        // producers. Their limitations remain explicit in every observation.
        game.effect_store.pending_trigger_events.clear();
    }
}

pub fn seed(definition: &CardDefinition, source_zone: Zone) -> Seed {
    let mut game = GameState::new(
        vec![
            "Audit controller".into(),
            "Audit opponent".into(),
            "Audit active opponent".into(),
        ],
        20,
    );
    game.set_random_seed(SEED);
    game.turn.active_player = PlayerId::from_index(2);
    let source = create_fixture_object(&mut game, definition, PlayerId::from_index(0), source_zone);
    let creature = CardDefinitionBuilder::new(CardId::new(), "Audit Human Soldier")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(3, 3))
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .build();
    let land = CardDefinitionBuilder::new(CardId::new(), "Audit Plains")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Plains])
        .build();
    let artifact = CardDefinitionBuilder::new(CardId::new(), "Audit Artifact")
        .card_types(vec![CardType::Artifact])
        .build();
    let enchantment = CardDefinitionBuilder::new(CardId::new(), "Audit Enchantment")
        .card_types(vec![CardType::Enchantment])
        .build();
    let spell = CardDefinitionBuilder::new(CardId::new(), "Audit Instant")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .build();
    let mut creatures = Vec::new();
    let mut lands = Vec::new();
    let mut spells = Vec::new();
    let mut hands = Vec::new();
    for index in 0..3 {
        let player = PlayerId::from_index(index);
        creatures.push(create_fixture_object(
            &mut game,
            &creature,
            player,
            Zone::Battlefield,
        ));
        lands.push(create_fixture_object(
            &mut game,
            &land,
            player,
            Zone::Battlefield,
        ));
        create_fixture_object(&mut game, &artifact, player, Zone::Battlefield);
        create_fixture_object(&mut game, &enchantment, player, Zone::Battlefield);
        spells.push(create_fixture_object(
            &mut game,
            &spell,
            player,
            Zone::Stack,
        ));
        hands.push(create_fixture_object(
            &mut game,
            &creature,
            player,
            Zone::Hand,
        ));
        create_fixture_object(&mut game, &land, player, Zone::Hand);
        for card in [&creature, &land, &spell] {
            create_fixture_object(&mut game, card, player, Zone::Graveyard);
            for _ in 0..4 {
                create_fixture_object(&mut game, card, player, Zone::Library);
            }
        }
        for symbol in [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
            ManaSymbol::Colorless,
        ] {
            game.player_mut(player).unwrap().mana_pool.add(symbol, 5);
        }
    }
    game.effect_store.pending_trigger_events.clear();
    Seed {
        game,
        source,
        creatures,
        lands,
        spells,
        hands,
    }
}

fn snapshot(seed: &Seed, object: ObjectId) -> Option<ObjectSnapshot> {
    seed.game
        .object(object)
        .map(|object| ObjectSnapshot::from_object(object, &seed.game))
}

pub fn events(seed: &Seed) -> Vec<Fixture> {
    let mut result = Vec::new();
    for index in 0..3 {
        let player = PlayerId::from_index(index);
        let active = PlayerId::from_index(2);
        let mut add = |label: &str, event: Box<dyn GameEventType>| {
            result.push(Fixture {
                name: format!("{label}/player_{index}/seed_{SEED}"),
                event: TriggerEvent::from_boxed(event, Default::default()),
                active,
                phase: Phase::FirstMain,
                step: None,
            });
        };
        for amount in [1, 3] {
            add(
                &format!("life_loss_{amount}"),
                Box::new(LifeLossEvent::from_effect(player, amount)),
            );
            add(
                &format!("damage_life_loss_{amount}"),
                Box::new(LifeLossEvent::new(player, amount, true)),
            );
        }
        add("life_gain", Box::new(LifeGainEvent::new(player, 3)));
        add(
            "draw_first",
            Box::new(CardsDrawnEvent::single(
                player,
                seed.hands[index as usize],
                true,
            )),
        );
        add(
            "draw_later",
            Box::new(CardsDrawnEvent::single(
                player,
                seed.hands[index as usize],
                false,
            )),
        );
        add(
            "discard",
            Box::new(
                CardDiscardedEvent::new(player, seed.hands[index as usize])
                    .with_snapshot(snapshot(seed, seed.hands[index as usize]).unwrap()),
            ),
        );
        add(
            "reveal",
            Box::new(CardRevealedEvent::new(
                player,
                seed.hands[index as usize],
                Zone::Hand,
                Some(seed.source),
                snapshot(seed, seed.hands[index as usize]),
            )),
        );
        add(
            "spell_cast",
            Box::new(SpellCastEvent::new_with_snapshot(
                seed.spells[index as usize],
                player,
                Zone::Hand,
                snapshot(seed, seed.spells[index as usize]).unwrap(),
            )),
        );
        add(
            "spell_countered",
            Box::new(SpellCounteredEvent::new(
                seed.spells[index as usize],
                player,
                snapshot(seed, seed.spells[index as usize]),
            )),
        );
        add(
            "land_played",
            Box::new(LandPlayedEvent::new(
                seed.lands[index as usize],
                player,
                Zone::Hand,
            )),
        );
        add(
            "library_search",
            Box::new(SearchLibraryEvent::new(player, Some(player))),
        );
        add(
            "library_shuffle",
            Box::new(ShuffleLibraryEvent::new(player, EventCause::effect())),
        );
        add(
            "gift",
            Box::new(GiftGivenEvent::new(
                player,
                PlayerId::from_index((index + 1) % 3),
                seed.source,
            )),
        );
        add(
            "die_roll",
            Box::new(other::DieRolledEvent::new(player, seed.source, 4, 6)),
        );
        for won in [false, true] {
            add(
                if won { "coin_win" } else { "coin_loss" },
                Box::new(CoinFlippedEvent {
                    player,
                    source: seed.source,
                    face: ironsmith::CoinFace::Heads,
                    call: Some(if won {
                        ironsmith::CoinFace::Heads
                    } else {
                        ironsmith::CoinFace::Tails
                    }),
                    winner: won.then_some(player),
                    loser: (!won).then_some(player),
                }),
            );
        }
        for combat in [false, true] {
            add(
                if combat {
                    "combat_damage_to_player"
                } else {
                    "noncombat_damage_to_player"
                },
                Box::new(DamageEvent::with_cause(
                    seed.source,
                    DamageTarget::Player(player),
                    3,
                    combat,
                    EventCause::effect(),
                )),
            );
            add(
                if combat {
                    "combat_damage_to_source"
                } else {
                    "noncombat_damage_to_source"
                },
                Box::new(
                    DamageEvent::with_cause(
                        seed.creatures[index as usize],
                        DamageTarget::Object(seed.source),
                        1,
                        combat,
                        EventCause::effect(),
                    )
                    .with_target_snapshot(snapshot(seed, seed.source).unwrap()),
                ),
            );
        }
        for action in [
            KeywordActionKind::Scry,
            KeywordActionKind::Surveil,
            KeywordActionKind::Cycle,
            KeywordActionKind::Investigate,
            KeywordActionKind::Explore,
            KeywordActionKind::Connive,
            KeywordActionKind::Proliferate,
            KeywordActionKind::RingTemptsYou,
            KeywordActionKind::Exert,
            KeywordActionKind::Expend,
        ] {
            add(
                &format!("keyword_{action:?}"),
                Box::new(
                    KeywordActionEvent::new(action, player, seed.source, 3)
                        .with_snapshot(snapshot(seed, seed.source)),
                ),
            );
        }
        // Beginning-of-step events identify their own active player. Put the
        // state in the corresponding phase for turn-qualified matchers.
        for (label, event, phase, step) in [
            (
                "upkeep",
                Box::new(BeginningOfUpkeepEvent::new(player)) as Box<dyn GameEventType>,
                Phase::Beginning,
                Some(Step::Upkeep),
            ),
            (
                "draw_step",
                Box::new(BeginningOfDrawStepEvent::new(player)),
                Phase::Beginning,
                Some(Step::Draw),
            ),
            (
                "first_main",
                Box::new(BeginningOfPrecombatMainPhaseEvent::new(player)),
                Phase::FirstMain,
                None,
            ),
            (
                "combat_begin",
                Box::new(BeginningOfCombatEvent::new(player)),
                Phase::Combat,
                Some(Step::BeginCombat),
            ),
            (
                "second_main",
                Box::new(BeginningOfPostcombatMainPhaseEvent::new(player)),
                Phase::NextMain,
                None,
            ),
            (
                "end_step",
                Box::new(BeginningOfEndStepEvent::new(player)),
                Phase::Ending,
                Some(Step::End),
            ),
        ] {
            result.push(Fixture {
                name: format!("{label}/player_{index}/seed_{SEED}"),
                event: TriggerEvent::from_boxed(event, Default::default()),
                active: player,
                phase,
                step,
            });
        }
    }
    for (label, object) in [
        ("source", seed.source),
        ("own_creature", seed.creatures[0]),
        ("opponent_creature", seed.creatures[1]),
        ("own_land", seed.lands[0]),
    ] {
        let mut add = |name: &str, event: Box<dyn GameEventType>| {
            result.push(Fixture {
                name: format!("{name}/{label}/seed_{SEED}"),
                event: TriggerEvent::from_boxed(event, Default::default()),
                active: PlayerId::from_index(2),
                phase: Phase::FirstMain,
                step: None,
            });
        };
        add(
            "enter_battlefield",
            Box::new(EnterBattlefieldEvent::new(object, Zone::Hand)),
        );
        add(
            "zone_enter_battlefield",
            Box::new(ZoneChangeEvent::with_cause(
                object,
                Zone::Hand,
                Zone::Battlefield,
                EventCause::effect(),
                snapshot(seed, object),
            )),
        );
        add(
            "dies",
            Box::new(ZoneChangeEvent::with_cause(
                object,
                Zone::Battlefield,
                Zone::Graveyard,
                EventCause::effect(),
                snapshot(seed, object),
            )),
        );
        add(
            "exiled",
            Box::new(ZoneChangeEvent::with_cause(
                object,
                Zone::Battlefield,
                Zone::Exile,
                EventCause::effect(),
                snapshot(seed, object),
            )),
        );
        add(
            "sacrifice",
            Box::new(
                SacrificeEvent::new(object, Some(seed.source))
                    .with_snapshot(snapshot(seed, object), seed.game.current_controller(object)),
            ),
        );
        add("tapped", Box::new(PermanentTappedEvent::new(object)));
        add("untapped", Box::new(PermanentUntappedEvent::new(object)));
        add(
            "counter",
            Box::new(
                CounterPlacedEvent::new(object, CounterType::PlusOnePlusOne, 1)
                    .with_previous_count(0),
            ),
        );
        add(
            "lore_counter",
            Box::new(CounterPlacedEvent::new(object, CounterType::Lore, 1).with_previous_count(0)),
        );
        add(
            "targeted",
            Box::new(BecomesTargetedEvent::new(
                object,
                seed.spells[1],
                PlayerId::from_index(1),
                false,
            )),
        );
        add(
            "attack",
            Box::new(
                CreatureAttackedEvent::with_total_attackers(
                    object,
                    AttackEventTarget::Player(PlayerId::from_index(1)),
                    1,
                )
                .with_declared_attackers(
                    vec![ironsmith::combat_state::AttackerInfo {
                        creature: object,
                        target: ironsmith::combat_state::AttackTarget::Player(
                            PlayerId::from_index(1),
                        ),
                    }]
                    .into(),
                ),
            ),
        );
        add(
            "turned_face_up",
            Box::new(TurnedFaceUpEvent::new(
                object,
                seed.game.current_controller(object).unwrap(),
            ).with_snapshot(seed.game.object(object).map(|object|
                ObjectSnapshot::from_object_with_calculated_characteristics(object, &seed.game)))),
        );
    }
    result
}
