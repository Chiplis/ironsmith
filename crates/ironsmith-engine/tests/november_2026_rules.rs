//! Regression scenarios for the November 6, 2026 Comprehensive Rules audit.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::condition_eval::evaluate_condition_resolution;
use ironsmith::decisions::context::SelectOptionsContext;
use ironsmith::effect::Value;
use ironsmith::effects::{ChooseCreatureTypeEffect, FlipCoinEffect, RollDiceChooseResultEffect};
use ironsmith::types::SubtypeFamily;
use ironsmith::{
    AbilityKind, CardId, CardType, CounterType, DecisionMaker, EffectContext, EffectExecutor,
    GameState, ObjectId, Phase, PlayerFilter, PlayerId, PowerToughness, Zone,
};

const A: PlayerId = PlayerId(0);
fn game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.turn.active_player = A;
    g.turn.priority_player = Some(A);
    g.turn.turn_number = 2;
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    g
}
fn creature(g: &mut GameState, name: &str, p: i32, t: i32) -> ObjectId {
    let d = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(p, t))
        .build();
    g.create_object_from_definition(&d, A, Zone::Battlefield)
}

fn card(route: &str) -> ironsmith::CardDefinition {
    let (name, block) = match route {
        "abyssal-harvester" => (
            "Abyssal Harvester",
            "Mana cost: {1}{B}{B}\nType: Creature — Demon Warlock\nFirst printed set: Foundations\nPower/Toughness: 3/2\n{T}: Exile target creature card from a graveyard that was put there this turn. Create a token that's a copy of it, except it's a Nightmare in addition to its other types. Then exile all other Nightmare tokens you control.",
        ),
        "fiery-bombardment" => (
            "Fiery Bombardment",
            "Mana cost: {1}{R}\nType: Enchantment\nFirst printed set: Eventide\nChroma — {2}, Sacrifice a creature: This enchantment deals damage to any target equal to the number of red mana symbols in the sacrificed creature's mana cost.",
        ),
        "graf-rats" => (
            "Graf Rats",
            "Mana cost: {1}{B}\nType: Creature — Rat\nFirst printed set: Eldritch Moon\nPower/Toughness: 2/1\nAt the beginning of combat on your turn, if you both own and control this creature and a creature named Midnight Scavengers, exile them, then meld them into Chittering Host.",
        ),
        "grizzly-bears" => (
            "Grizzly Bears",
            "Mana cost: {1}{G}\nType: Creature — Bear\nFirst printed set: Limited Edition Alpha\nPower/Toughness: 2/2",
        ),
        "island" => (
            "Island",
            "Type: Basic Land — Island\nFirst printed set: Limited Edition Alpha\n({T}: Add {U}.)",
        ),
        "ixidron" => (
            "Ixidron",
            "Mana cost: {3}{U}{U}\nType: Creature — Illusion\nFirst printed set: Time Spiral\nPower/Toughness: */*\nAs this creature enters, turn all other nontoken creatures face down. (They're 2/2 creatures.)\nIxidron's power and toughness are each equal to the number of face-down creatures on the battlefield.",
        ),
        "kira-nerys-firebrand" => (
            "Kira Nerys, Firebrand",
            "Mana cost: {1}{R}{W}\nType: Legendary Creature — Bajoran Soldier\nFirst printed set: Star Trek\nPower/Toughness: 4/3\nTrample\nIf a creature you control would enter while you control an attacking creature, instead it enters tapped and attacking.\nWhenever another creature you control enters, it gets +1/+1 until end of turn.",
        ),
        "krark-s-thumb" => (
            "Krark's Thumb",
            "Mana cost: {2}\nType: Legendary Artifact\nFirst printed set: Mirrodin\nIf you would flip a coin, instead flip two coins and ignore one.",
        ),
        "martial-coup" => (
            "Martial Coup",
            "Mana cost: {X}{W}{W}\nType: Sorcery\nFirst printed set: Conflux\nCreate X 1/1 white Soldier creature tokens. If X is 5 or more, destroy all other creatures.",
        ),
        "mirror-box" => (
            "Mirror Box",
            "Mana cost: {3}\nType: Artifact\nFirst printed set: Kamigawa: Neon Dynasty\nThe \"legend rule\" doesn't apply to permanents you control.\nEach legendary creature you control gets +1/+1.\nEach nontoken creature you control gets +1/+1 for each other creature you control with the same name as that creature.",
        ),
        "ral-zarek" => (
            "Ral Zarek",
            "Mana cost: {2}{U}{R}\nType: Legendary Planeswalker — Ral\nFirst printed set: Dragon's Maze\nLoyalty: 4\n+1: Tap target permanent, then untap another target permanent.\n−2: Ral Zarek deals 3 damage to any target.\n−7: Flip five coins. Take an extra turn after this one for each coin that comes up heads.",
        ),
        "stampeding-elk-herd" => (
            "Stampeding Elk Herd",
            "Mana cost: {3}{G}{G}\nType: Creature — Elk\nFirst printed set: Dragons of Tarkir\nPower/Toughness: 5/5\nFormidable — Whenever this creature attacks, if creatures you control have total power 8 or greater, creatures you control gain trample until end of turn.",
        ),
        "white-sun-s-twilight" => (
            "White Sun's Twilight",
            "Mana cost: {X}{W}{W}\nType: Sorcery\nFirst printed set: Phyrexia: All Will Be One\nYou gain X life. Create X 1/1 colorless Phyrexian Mite artifact creature tokens with toxic 1 and \"This token can't block.\" If X is 5 or more, destroy all other creatures. (Players dealt combat damage by a creature with toxic 1 also get a poison counter.)",
        ),
        "zethi-arcane-blademaster" => (
            "Zethi, Arcane Blademaster",
            "Mana cost: {1}{W}{U}\nType: Legendary Creature — Human Soldier\nFirst printed set: Secret Lair Drop\nPower/Toughness: 3/3\nMultikicker {W/U}\nWhen Zethi, Arcane Blademaster enters, exile up to X target instant cards from your graveyard, where X is the number of times Zethi was kicked. Put a kick counter on each of them.\nWhenever Zethi attacks, copy each exiled card you own with a kick counter on it. You may cast the copies.",
        ),
        _ => panic!("unknown regression fixture {route}"),
    };
    ironsmith_compiler_runtime::compile_to_runtime_definition(name, block, false).unwrap()
}

#[derive(Default)]
struct Menu(Vec<String>);
impl DecisionMaker for Menu {
    fn decide_options(&mut self, _: &GameState, options: &SelectOptionsContext) -> Vec<usize> {
        self.0.extend(
            options
                .options
                .iter()
                .filter(|o| o.legal)
                .map(|o| o.description.clone()),
        );
        vec![options.options[0].index]
    }
}
#[test]
fn formidable_counts_negative_power() {
    let mut g = game();
    let herd = g.create_object_from_definition(&card("stampeding-elk-herd"), A, Zone::Battlefield);
    creature(&mut g, "Another attacker", 3, 3);
    let weak = creature(&mut g, "Weakened creature", -2, 3);
    assert_eq!(g.current_power(weak), Some(-2));
    let ctx = EffectContext::new_default(herd, A);
    let abilities = g.current_abilities(herd).unwrap();
    let condition = abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => t.intervening_if.as_ref(),
            _ => None,
        })
        .unwrap();
    assert!(
        !evaluate_condition_resolution(&g, condition, &ctx).unwrap(),
        "5 + 3 - 2 = 6, below eight"
    );
}

#[test]
fn berserkers_frenzy_ignores_lower_die() {
    let mut g = game();
    let s = g.new_object_id();
    g.force_next_die_roll(3);
    g.force_next_die_roll(18);
    let out = RollDiceChooseResultEffect::new(PlayerFilter::You, 2, 20)
        .with_ignore_lower(true)
        .execute(&mut g, &mut EffectContext::new_default(s, A))
        .unwrap();
    assert_eq!(out.as_count(), Some(18));
    assert!(
        !out.execution_facts
            .iter()
            .any(|fact| matches!(fact, ironsmith::effect::ExecutionFact::OtherNumber(_)))
    );
    assert_eq!(
        g.turn_store.turn_history.completed_die_roll_count(A),
        1,
        "only the 18 counts"
    );
}

#[test]
fn berserkers_frenzy_roller_breaks_tie() {
    let mut g = game();
    let s = g.new_object_id();
    g.force_next_die_roll(8);
    g.force_next_die_roll(8);
    let mut dm = Menu::default();
    RollDiceChooseResultEffect::new(PlayerFilter::You, 2, 20)
        .with_ignore_lower(true)
        .execute(&mut g, &mut EffectContext::new(s, A, &mut dm))
        .unwrap();
    assert!(
        !dm.0.is_empty(),
        "two eights require the roller to select which to ignore"
    );
}

#[test]
fn neverwinter_hydra_rolls_the_chosen_number_of_dice() {
    let mut g = game();
    let s = g.new_object_id();
    RollDiceChooseResultEffect::new(PlayerFilter::You, 4097, 6)
        .with_total(None)
        .execute(&mut g, &mut EffectContext::new_default(s, A))
        .unwrap();
    assert_eq!(g.turn_store.turn_history.completed_die_roll_count(A), 4097);
}

#[test]
fn all_requested_coin_flips_take_place() {
    let mut g = game();
    let s = g.new_object_id();
    let mut e = FlipCoinEffect::face_only(PlayerFilter::You);
    e.count = 4097;
    e.execute(&mut g, &mut EffectContext::new_default(s, A))
        .unwrap();
    assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), 4097);
}

#[test]
fn same_name_token_cap_resets_for_each_creation_event() {
    let mut game = game();
    let source = game.new_object_id();
    let cat = CardDefinitionBuilder::new(CardId::new(), "Cat")
        .token()
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let outcome = ironsmith::effects::CreateTokenEffect::you(cat.clone(), 513)
        .execute(&mut game, &mut EffectContext::new_default(source, A))
        .unwrap();
    assert_eq!(outcome.result_objects().unwrap().len(), 512);
    assert_eq!(game.battlefield.len(), 512);
    ironsmith::effects::CreateTokenEffect::you(cat, 1)
        .execute(&mut game, &mut EffectContext::new_default(source, A))
        .unwrap();
    assert_eq!(game.battlefield.len(), 513);
}

#[test]
fn each_exalted_counter_grants_an_ability() {
    let mut g = game();
    let s = creature(&mut g, "Recipient", 1, 1);
    let kind = CounterType::Named("exalted".into());
    g.add_counters(s, kind, 4097).unwrap();
    assert_eq!(g.counter_count(s, kind), 4097);
    let triggers = g
        .current_abilities(s)
        .unwrap()
        .into_iter()
        .filter(|a| matches!(a.kind, AbilityKind::Triggered(_)))
        .count();
    assert_eq!(triggers, 4097);
}

#[test]
fn oversized_ability_counter_placement_fails_without_changing_the_recipient() {
    let mut g = game();
    let source = creature(&mut g, "Recipient", 1, 1);
    let kind = CounterType::Named("exalted".into());
    g.add_counters(source, kind, 1).unwrap();
    let effect = ironsmith::Effect::put_counters_on_source(kind, i32::MAX);
    let error = ironsmith::effects::execute_effect(
        &mut g,
        &effect,
        &mut EffectContext::new_default(source, A),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ironsmith::effects::ExecutionError::ResourceLimitExceeded { .. }
    ));
    assert_eq!(g.counter_count(source, kind), 1);
}

#[test]
fn admitting_a_counter_ability_preflights_existing_counters() {
    let mut g = game();
    let source = creature(&mut g, "Recipient", 1, 1);
    let kind = CounterType::Named("fixture".into());
    g.add_counters(source, kind, 100_000).unwrap();
    let error = g
        .register_named_ability_counter(kind, vec![ironsmith::ability::flying()])
        .unwrap_err();
    assert!(error.contains("ability counter occurrences"));
    assert_eq!(g.counter_count(source, kind), 100_000);
    assert!(g.current_abilities(source).unwrap().is_empty());
}

#[test]
fn a_sacrificed_creature_with_no_mana_cost_has_zero_red_symbols() {
    let mut g = game();
    let source = g.create_object_from_definition(&card("fiery-bombardment"), A, Zone::Battlefield);
    let token = creature(&mut g, "Costless creature", 1, 1);
    let mut ctx = EffectContext::new_default(source, A);
    ctx.tag_object(
        "__original_sacrifice_cost_0",
        ironsmith::UnifiedObjectSnapshot::from_object_id(&g, token).unwrap(),
    );
    g.move_object_by_effect(token, Zone::Graveyard);
    let v = Value::ManaSymbolsInManaCostOf {
        spec: Box::new(ironsmith::ChooseSpec::tagged("__original_sacrifice_cost_0")),
        color: ironsmith::Color::Red,
    };
    assert_eq!(
        ironsmith::effects::resolve_value(&g, &v, &ctx).map_err(|e| format!("{e:?}")),
        Ok(0)
    );
}

#[test]
fn five_coin_flips_with_eleven_thumbs_keep_five_results() {
    let mut g = game();
    g.create_object_from_definition(&card("mirror-box"), A, Zone::Battlefield);
    let thumb = card("krark-s-thumb");
    for _ in 0..11 {
        g.create_object_from_definition(&thumb, A, Zone::Battlefield);
    }
    let s = g.create_object_from_definition(&card("ral-zarek"), A, Zone::Battlefield);
    let mut effect = FlipCoinEffect::face_only(PlayerFilter::You);
    effect.count = 5;
    effect
        .execute(&mut g, &mut EffectContext::new_default(s, A))
        .unwrap();
    assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), 5);
}

#[test]
fn an_interchangeable_printing_must_meet_a_card_name_choices_type_restriction() {
    struct ChoosePrinting;
    impl DecisionMaker for ChoosePrinting {
        fn decide_text(
            &mut self,
            _: &GameState,
            _: &ironsmith::decisions::context::TextInputContext,
        ) -> String {
            "Chun-Li, Countless Kicks".into()
        }
    }
    for (filter, expected) in [
        (ironsmith::ObjectFilter::artifact(), false),
        (ironsmith::ObjectFilter::creature(), true),
    ] {
        let mut g = game();
        let source = creature(&mut g, "Chooser", 1, 1);
        let effect = ironsmith::effects::ChooseCardNameEffect::new(
            PlayerFilter::You,
            Some(filter),
            "chosen",
        );
        effect
            .execute(
                &mut g,
                &mut EffectContext::new(source, A, &mut ChoosePrinting),
            )
            .unwrap();
        assert_eq!(g.chosen_named_option(source).is_some(), expected);
    }
}

#[test]
fn interchangeable_legendary_printings_invoke_the_legend_rule() {
    let mut g = game();
    let mut d = card("zethi-arcane-blademaster");
    g.create_object_from_definition(&d, A, Zone::Battlefield);
    d.card.name = "Chun-Li, Countless Kicks".to_string();
    g.create_object_from_definition(&d, A, Zone::Battlefield);
    assert_eq!(
        ironsmith::rules::state_based::get_legend_rule_specs(&g).len(),
        1
    );
}

#[test]
fn five_token_board_wipes_spare_the_new_creatures() {
    let mut wrong = vec![];
    for (route, subtype) in [
        ("martial-coup", ironsmith::Subtype::Soldier),
        ("white-sun-s-twilight", ironsmith::Subtype::Mite),
    ] {
        for x in [4, 5] {
            let mut g = game();
            let old = creature(&mut g, "Old creature", 2, 2);
            let id = g.create_object_from_definition(&card(route), A, Zone::Stack);
            let mut entry = ironsmith::StackEntry::new(id, A);
            g.object_mut(id).unwrap().x_value = Some(x as u32);
            entry.x_value = Some(x);
            g.push_to_stack(entry);
            ironsmith::game_loop::resolve_stack_entry(&mut g).unwrap();
            assert_eq!(g.battlefield.contains(&old), x < 5);
            let soldiers = g
                .battlefield
                .iter()
                .filter(|&&s| g.current_has_subtype(s, subtype))
                .count();
            if soldiers != x as usize {
                wrong.push(format!("{route} at X={x}: {soldiers} surviving tokens"));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join(", "));
}

#[test]
fn abyssal_harvester_keeps_the_new_nightmare() {
    let mut g = game();
    let d = card("abyssal-harvester");
    let harvester = g.create_object_from_definition(&d, A, Zone::Battlefield);
    let program = d
        .abilities
        .into_iter()
        .find_map(|a| match a.kind {
            AbilityKind::Activated(a) => Some(a.effects),
            _ => None,
        })
        .unwrap();
    let mut previous_nightmare = None;
    for name in ["First Bear", "Second Bear"] {
        let bear = creature(&mut g, name, 2, 2);
        let grave = g.move_object_by_effect(bear, Zone::Graveyard).unwrap();
        let mut entry = ironsmith::StackEntry::ability(harvester, A, program.clone());
        entry.targets.push(ironsmith::Target::Object(grave));
        g.push_to_stack(entry);
        ironsmith::game_loop::resolve_stack_entry(&mut g).unwrap();
        assert!(
            g.exile
                .iter()
                .any(|&id| g.object(id).is_some_and(|o| o.name == name))
        );
        let copies: Vec<_> = g
            .battlefield
            .iter()
            .copied()
            .filter(|&id| g.current_has_subtype(id, ironsmith::Subtype::Nightmare))
            .collect();
        assert_eq!(
            copies.len(),
            1,
            "each activation leaves exactly its own Nightmare"
        );
        assert_eq!(g.object(copies[0]).unwrap().name, name);
        assert_ne!(previous_nightmare.replace(copies[0]), Some(copies[0]));
    }
}

#[test]
fn ixidron_cannot_turn_graf_rats_face_down() {
    let mut g = game();
    let rats = g.create_object_from_definition(&card("graf-rats"), A, Zone::Battlefield);
    let bear = g.create_object_from_definition(&card("grizzly-bears"), A, Zone::Battlefield);
    let id = g.create_object_from_definition(&card("ixidron"), A, Zone::Stack);
    g.push_to_stack(ironsmith::StackEntry::new(id, A));
    ironsmith::game_loop::resolve_stack_entry(&mut g).unwrap();
    assert!(g.is_face_down(bear), "ordinary creature turns face down");
    assert!(
        !g.is_face_down(rats),
        "face-up Graf Rats must survive Ixidron's instruction face up"
    );
}

#[test]
fn creature_type_choices_are_independent_of_an_opponents_library() {
    fn menu(g: &mut GameState, id: ObjectId) -> Vec<String> {
        let mut dm = Menu::default();
        ChooseCreatureTypeEffect::for_family(PlayerFilter::You, SubtypeFamily::Creature)
            .execute(g, &mut EffectContext::new(id, A, &mut dm))
            .unwrap();
        dm.0
    }
    let mut a = game();
    let id = creature(&mut a, "Source", 1, 1);
    let mut b = a.clone();
    a.create_object_from_definition(&card("island"), PlayerId(1), Zone::Library);
    b.create_object_from_definition(&card("kira-nerys-firebrand"), PlayerId(1), Zone::Library);
    let first = menu(&mut a, id);
    let second = menu(&mut b, id);
    assert_eq!(
        first.contains(&"Bajoran".to_string()),
        second.contains(&"Bajoran".to_string()),
        "substituting an unrevealed Kira for an Island changes Alice's menu"
    );
}
