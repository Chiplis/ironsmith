//! Claude package 19 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::PlayerId;
use ironsmith::Zone;
use ironsmith::game_state::GameState;

/// Every effect reachable from a definition (spell, activated, triggered),
/// including nested children.
fn p19_effects(definition: &ironsmith::cards::CardDefinition) -> Vec<Effect> {
    fn walk(effect: &Effect, out: &mut Vec<Effect>) {
        out.push(effect.clone());
        effect.visit_child_effects(&mut |child| walk(child, out));
    }
    let mut out = Vec::new();
    if let Some(program) = definition.spell_effect.as_ref() {
        for effect in program.all_effects() { walk(effect, &mut out); }
    }
    for ability in &definition.abilities {
        match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated) => {
                for effect in activated.effects.all_effects() { walk(effect, &mut out); }
            }
            ironsmith::ability::AbilityKind::Triggered(triggered) => {
                for effect in triggered.effects.all_effects() { walk(effect, &mut out); }
            }
            _ => {}
        }
    }
    out
}

const REVEALING_WIND: &str = "Mana cost: {2}{G}\nType: Instant\nPrevent all combat damage that would be dealt this turn. You may look at each face-down creature that's attacking or blocking.";

/// CR 708.5: only a face-down permanent's controller may normally look at it;
/// Revealing Wind privately shows the caster every face-down attacker and
/// blocker, whoever controls it.
#[test]
fn revealing_wind_looks_at_each_face_down_attacking_or_blocking_creature() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use ironsmith::ids::{CardId, ObjectId};
    use ironsmith::types::CardType;
    let definition = compile_to_runtime_definition("Revealing Wind", REVEALING_WIND, false)
        .expect("Revealing Wind should compile");
    let look = p19_effects(&definition)
        .into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::LookAtObjectsEffect>().cloned())
        .expect("a look-at-objects instruction");
    assert_eq!(look.filter.face_down, Some(true));
    assert_eq!(look.filter.zone, Some(Zone::Battlefield));
    assert_eq!(look.filter.controller, None, "whoever controls them");
    assert!(!look.permit_while_exiled);

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let bear = |name: &str| CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    let hidden_attacker = game.create_object_from_card(&bear("Hidden attacker"), bob, Zone::Battlefield);
    let hidden_idle = game.create_object_from_card(&bear("Hidden idle"), bob, Zone::Battlefield);
    let open_attacker = game.create_object_from_card(&bear("Open attacker"), bob, Zone::Battlefield);
    game.set_face_down(hidden_attacker);
    game.set_face_down(hidden_idle);
    game.turn.phase = ironsmith::game_state::Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
    let mut combat = CombatState::default();
    combat.attackers.push(AttackerInfo { creature: hidden_attacker, target: AttackTarget::Player(alice) });
    combat.attackers.push(AttackerInfo { creature: open_attacker, target: AttackTarget::Player(alice) });
    game.combat = Some(combat);
    game.refresh_continuous_state().unwrap();

    #[derive(Default)]
    struct Views(Vec<(PlayerId, Vec<ObjectId>)>);
    impl ironsmith::decision::DecisionMaker for Views {
        fn view_cards(
            &mut self,
            _: &GameState,
            viewer: PlayerId,
            cards: &[ObjectId],
            _: &ironsmith::decisions::context::ViewCardsContext,
        ) {
            self.0.push((viewer, cards.to_vec()));
        }
    }
    let mut dm = Views::default();
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    {
        let mut ctx = ironsmith::effects::EffectContext::new(spell, alice, &mut dm);
        ironsmith::effects::execute_effect(&mut game, &Effect::new(look), &mut ctx).unwrap();
    }
    let seen: Vec<ObjectId> = dm.0.iter().filter(|(viewer, _)| *viewer == alice)
        .flat_map(|(_, cards)| cards.iter().copied()).collect();
    assert!(seen.contains(&hidden_attacker), "the face-down attacker is shown: {seen:?}");
    assert!(!seen.contains(&hidden_idle), "a face-down creature outside combat is not");
    assert!(!seen.contains(&open_attacker), "a face-up attacker needs no look");
}

const WALL_OF_SHADOWS: &str = "Mana cost: {1}{B}{B}\nType: Creature — Wall\nPower/Toughness: 0/1\nDefender (This creature can't attack.)\nPrevent all damage that would be dealt to this creature by creatures it's blocking.\nThis creature can't be the target of spells that can target only Walls or of abilities that can target only Walls.";

/// Wall of Shadows can't be chosen for a "target Wall" requirement, but a
/// requirement that can also target non-Walls ("target creature") may
/// choose it.
#[test]
fn wall_of_shadows_rejects_only_wall_target_requirements() {
    use ironsmith::target::{ChooseSpec, ObjectFilter};
    use ironsmith::types::Subtype;
    let definition = compile_to_runtime_definition("Wall of Shadows", WALL_OF_SHADOWS, false)
        .expect("Wall of Shadows should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("BeTargetedByRequirementsLimitedTo"), "{debug}");
    assert!(debug.contains("Wall"), "{debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let wall = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    game.update_cant_effects();
    let wall_target = |game: &GameState, filter: ObjectFilter| {
        ironsmith::targeting::compute_legal_targets(
            game,
            &ChooseSpec::Target(Box::new(ChooseSpec::Object(filter))),
            bob,
            None,
        )
        .contains(&ironsmith::game_state::Target::Object(wall))
    };
    assert!(
        !wall_target(&game, ObjectFilter::creature().with_subtype(Subtype::Wall)),
        "a requirement that can target only Walls can't choose it"
    );
    assert!(
        wall_target(&game, ObjectFilter::creature()),
        "a requirement that can also target other creatures can"
    );
}

const TORRENT_OF_LAVA: &str = "Mana cost: {X}{R}{R}\nType: Sorcery\nTorrent of Lava deals X damage to each creature without flying.\nAs long as Torrent of Lava is on the stack, each creature has \"{T}: Prevent the next 1 damage that would be dealt to this creature by Torrent of Lava this turn.\"";

/// CR 113.6: Torrent of Lava's static ability functions while the spell is
/// on the stack, granting each creature a tap ability whose shield only
/// stops damage from that exact spell (the granting object).
#[test]
fn torrent_of_lava_grants_a_shield_against_itself_while_on_the_stack() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::CardId;
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::types::CardType;
    let definition = compile_to_runtime_definition("Torrent of Lava", TORRENT_OF_LAVA, false)
        .expect("Torrent of Lava should compile");
    let grant = definition
        .abilities
        .iter()
        .find(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Static(_)))
        .expect("the stack-only static grant");
    assert_eq!(grant.functional_zones, vec![Zone::Stack], "{grant:?}");
    let debug = format!("{grant:?}");
    assert!(debug.contains("SourceIsInZone(Stack)"), "{debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let bear = CardBuilder::new(CardId::new(), "Bear").card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    let creature = game.create_object_from_card(&bear, bob, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    let granted_count = |game: &GameState| {
        game.current_characteristics(creature)
            .map(|chars| {
                chars.abilities.iter()
                    .filter(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
                    .count()
            })
            .unwrap_or(0)
    };
    assert_eq!(granted_count(&game), 0, "no grant while the card isn't a spell");

    let torrent = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.refresh_continuous_state().unwrap();
    let chars = game.current_characteristics(creature).expect("creature characteristics");
    let granted = chars.abilities.iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated) => Some(activated.clone()),
            _ => None,
        })
        .expect("each creature has the tap ability while Torrent is on the stack");
    fn find_prevent(effect: &Effect, out: &mut Option<ironsmith::effects::PreventDamageEffect>) {
        if let Some(prevent) = effect.downcast_ref::<ironsmith::effects::PreventDamageEffect>() {
            *out = Some(prevent.clone());
        }
        effect.visit_child_effects(&mut |child| find_prevent(child, out));
    }
    let mut prevent = None;
    for effect in granted.effects.all_effects() { find_prevent(effect, &mut prevent); }
    let prevent = prevent.expect("a finite prevention shield");
    let source_filter = prevent.damage_filter.from_source.as_ref().expect("source slot");
    assert_eq!(source_filter.tagged_constraints.len(), 1);
    assert_eq!(source_filter.tagged_constraints[0].tag.as_str(), ironsmith::tag::GRANTING_SOURCE_TAG);

    // Activating it binds the shield to the exact granting spell.
    let mut ctx = ironsmith::effects::EffectContext::new_default(creature, bob);
    ctx.set_tagged_objects(
        ironsmith::tag::GRANTING_SOURCE_TAG,
        vec![ObjectSnapshot::from_object_id(&game, torrent).unwrap()],
    );
    ironsmith::effects::execute_effect(&mut game, &Effect::new(prevent), &mut ctx).unwrap();
    drop(ctx);
    let shield = game.effect_store.prevention_effects.shields().last().expect("a shield");
    assert_eq!(shield.damage_filter.from_specific_source, Some(torrent));
    assert_eq!(shield.damage_filter.from_source, None);

    let _ = game.move_object_by_effect(torrent, Zone::Graveyard);
    game.refresh_continuous_state().unwrap();
    assert_eq!(granted_count(&game), 0, "the grant ends once Torrent leaves the stack");
}

const VOLATILE_CHIMERA: &str = "Mana cost: {2}{R}\nType: Creature — Elemental Chimera\nPower/Toughness: 3/3\nBefore you shuffle your deck to start the game, you may reveal this card from your deck and exile three or more creature cards you drafted that aren't in your deck.\n{1}{R}: Choose a card at random you exiled with cards named Volatile Chimera. This creature becomes a copy of that card, except it has this ability.";

/// CR 100.4a: in a drafted game the cards drafted but not in the deck are
/// the sideboard. Volatile Chimera's pregame action exiles three or more of
/// those creature cards, recorded for every card named Volatile Chimera, and
/// its activated ability chooses among them at random.
#[test]
fn volatile_chimera_exiles_drafted_creatures_before_the_shuffle() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::CardId;
    use ironsmith::target::ChooseSpec;
    use ironsmith::types::CardType;
    let definition = compile_to_runtime_definition("Volatile Chimera", VOLATILE_CHIMERA, false)
        .expect("Volatile Chimera should compile");
    let pregame = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Static(static_ability) => {
                static_ability.pregame_action_kind()
            }
            _ => None,
        })
        .expect("a pregame action");
    let ironsmith::static_abilities::PregameActionKind::ExileDraftedCardsBeforeShuffle(spec) =
        pregame
    else {
        panic!("{pregame:?}");
    };
    assert_eq!(spec.card_types, vec![CardType::Creature]);
    assert_eq!(spec.minimum, 3);
    let choose = p19_effects(&definition)
        .into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>().cloned())
        .expect("the random choice");
    assert!(choose.count.is_random());
    assert_eq!(
        choose.filter.exiled_with_cards_named.as_deref().map(str::to_ascii_lowercase).as_deref(),
        Some("volatile chimera")
    );
    let debug = format!("{definition:?}");
    assert!(debug.contains("BecomeCopy") || debug.contains("Copy"), "{debug}");

    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.enable_conspiracy(Vec::new()).unwrap();
    let chimera = game.create_object_from_definition(&definition, alice, Zone::Library);
    let ability_index = definition
        .abilities
        .iter()
        .position(|ability| matches!(&ability.kind,
            ironsmith::ability::AbilityKind::Static(s) if s.pregame_action_kind().is_some()))
        .unwrap();
    let creature = |name: &str| CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    let drafted = ["Drafted A", "Drafted B", "Drafted C"]
        .map(|name| game.create_object_from_card(&creature(name), alice, Zone::OutsideGame));
    let instant = CardBuilder::new(CardId::new(), "Drafted Instant").card_types(vec![CardType::Instant]).build();
    let instant = game.create_object_from_card(&instant, alice, Zone::OutsideGame);

    assert!(game.exile_drafted_cards_before_shuffle(alice, chimera, ability_index, &drafted[..2]).is_err(),
        "fewer than three cards");
    assert!(game.exile_drafted_cards_before_shuffle(alice, chimera, ability_index,
        &[drafted[0], drafted[1], instant]).is_err(), "only creature cards");
    let exiled = game.exile_drafted_cards_before_shuffle(alice, chimera, ability_index, &drafted)
        .expect("three drafted creature cards");
    assert_eq!(exiled.len(), 3);
    for id in &exiled {
        assert_eq!(game.object(*id).unwrap().zone, Zone::Exile);
        assert!(game.card_exiled_with_cards_named(alice, "Volatile Chimera", *id));
    }
    let candidates = ironsmith::targeting::compute_legal_targets(
        &game,
        &ChooseSpec::Target(Box::new(ChooseSpec::Object(choose.filter.clone()))),
        alice,
        None,
    );
    for id in &exiled {
        assert!(candidates.contains(&ironsmith::game_state::Target::Object(*id)), "{candidates:?}");
    }
    assert_eq!(candidates.len(), 3, "only the cards exiled with Volatile Chimera");
}
