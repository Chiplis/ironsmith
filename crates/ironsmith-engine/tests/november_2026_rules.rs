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
        "altar-golem" => (
            "Altar Golem",
            "Mana cost: {7}\nType: Artifact Creature — Golem\nFirst printed set: Eventide\nPower/Toughness: */*\nTrample\nAltar Golem's power and toughness are each equal to the number of creatures on the battlefield.\nThis creature doesn't untap during your untap step.\nTap five untapped creatures you control: Untap this creature.",
        ),
        "bayou-groff" => (
            "Bayou Groff",
            "Mana cost: {1}{G}\nType: Creature — Plant Dog\nFirst printed set: Strixhaven: School of Mages\nPower/Toughness: 5/4\nAs an additional cost to cast this spell, sacrifice a creature or pay {3}.",
        ),
        "borg-cube" => (
            "Borg Cube",
            "Mana cost: {5}{W}{B}\nType: Artifact — Spacecraft\nFirst printed set: Star Trek\nPower/Toughness: 6/6\nAffinity for artifact creatures\nWhenever this Spacecraft enters or attacks, assimilate a creature card exiled with it. If you can't, exile up to one target creature. (To assimilate a card, put it onto the battlefield under your control with a +1/+1 counter. It's a Borg artifact creature and loses all other creature types.)\nStation\n12+ | Artifact creatures you control have flying.",
        ),
        "contentious-plan" => (
            "Contentious Plan",
            "Mana cost: {1}{U}\nType: Sorcery\nFirst printed set: War of the Spark\nProliferate. (Choose any number of permanents and/or players, then give each another counter of each kind already there.)\nDraw a card.",
        ),
        "discerning-financier" => (
            "Discerning Financier",
            "Mana cost: {2}{W}\nType: Creature — Human Noble\nFirst printed set: Wilds of Eldraine\nPower/Toughness: 2/3\nAt the beginning of your upkeep, if an opponent controls more lands than you, create a Treasure token. (It's an artifact with \"{T}, Sacrifice this token: Add one mana of any color.\")\n{2}{W}: Choose another player. That player gains control of target Treasure you control. You draw a card.",
        ),
        "divination" => (
            "Divination",
            "Mana cost: {2}{U}\nType: Sorcery\nFirst printed set: Magic 2010\nDraw two cards.",
        ),
        "frankenstein-s-monster" => (
            "Frankenstein's Monster",
            "Mana cost: {X}{B}{B}\nType: Creature — Zombie\nFirst printed set: The Dark\nPower/Toughness: 0/1\nAs this creature enters, exile X creature cards from your graveyard. If you can't, put this creature into its owner's graveyard instead of onto the battlefield. For each creature card exiled this way, this creature enters with a +2/+0, +1/+1, or +0/+2 counter on it.",
        ),
        "humility" => (
            "Humility",
            "Mana cost: {2}{W}{W}\nType: Enchantment\nFirst printed set: Tempest\nAll creatures lose all abilities and have base power and toughness 1/1.",
        ),
        "muraganda-petroglyphs" => (
            "Muraganda Petroglyphs",
            "Mana cost: {3}{G}\nType: Enchantment\nFirst printed set: Future Sight\nCreatures with no abilities get +2/+2.",
        ),
        "pain-s-reward" => (
            "Pain's Reward",
            "Mana cost: {2}{B}\nType: Sorcery\nFirst printed set: Saviors of Kamigawa\nEach player may bid life. You start the bidding with a bid of any number. In turn order, each player may top the high bid. The bidding ends if the high bid stands. The high bidder loses life equal to the high bid and draws four cards.",
        ),
        "past-in-flames" => (
            "Past in Flames",
            "Mana cost: {3}{R}\nType: Sorcery\nFirst printed set: Innistrad\nEach instant and sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.\nFlashback {4}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)",
        ),
        "phyrexian-revoker" => (
            "Phyrexian Revoker",
            "Mana cost: {2}\nType: Artifact Creature — Phyrexian Horror\nFirst printed set: Mirrodin Besieged\nPower/Toughness: 2/1\nAs this creature enters, choose a nonland card name.\nActivated abilities of sources with the chosen name can't be activated.",
        ),
        "pithing-needle" => (
            "Pithing Needle",
            "Mana cost: {1}\nType: Artifact\nFirst printed set: Saviors of Kamigawa\nAs this artifact enters, choose a card name.\nActivated abilities of sources with the chosen name can't be activated unless they're mana abilities.",
        ),
        "plea-for-power" => (
            "Plea for Power",
            "Mana cost: {3}{U}\nType: Sorcery\nFirst printed set: Conspiracy\nWill of the council — Starting with you, each player votes for time or knowledge. If time gets more votes, take an extra turn after this one. If knowledge gets more votes or the vote is tied, draw three cards.",
        ),
        "reanimate" => (
            "Reanimate",
            "Mana cost: {B}\nType: Sorcery\nFirst printed set: Tempest\nPut target creature card from a graveyard onto the battlefield under your control. You lose life equal to that card's mana value.",
        ),
        "simian-spirit-guide" => (
            "Simian Spirit Guide",
            "Mana cost: {2}{R}\nType: Creature — Ape Spirit\nFirst printed set: Planar Chaos\nPower/Toughness: 2/2\nExile this card from your hand: Add {R}.",
        ),
        "street-wraith" => (
            "Street Wraith",
            "Mana cost: {3}{B}{B}\nType: Creature — Wraith\nFirst printed set: Future Sight\nPower/Toughness: 3/4\nSwampwalk (This creature can't be blocked as long as defending player controls a Swamp.)\nCycling—Pay 2 life. (Pay 2 life, Discard this card: Draw a card.)",
        ),
        "think-twice" => (
            "Think Twice",
            "Mana cost: {1}{U}\nType: Instant\nFirst printed set: Time Spiral\nDraw a card.\nFlashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)",
        ),
        "underworld-breach" => (
            "Underworld Breach",
            "Mana cost: {1}{R}\nType: Enchantment\nFirst printed set: Theros Beyond Death Promos\nEach nonland card in your graveyard has escape. The escape cost is equal to the card's mana cost plus exile three other cards from your graveyard. (You may cast cards from your graveyard for their escape cost.)\nAt the beginning of the end step, sacrifice this enchantment.",
        ),
        "yixlid-jailer" => (
            "Yixlid Jailer",
            "Mana cost: {1}{B}\nType: Creature — Zombie Wizard\nFirst printed set: Future Sight\nPower/Toughness: 2/1\nCards in graveyards lose all abilities.",
        ),
        "biotransference" => (
            "Biotransference",
            "Mana cost: {2}{B}{B}\nType: Enchantment\nFirst printed set: Warhammer 40,000 Commander\nCreatures you control are artifacts in addition to their other types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.\nWhenever you cast an artifact spell, you lose 1 life and create a 2/2 black Necron Warrior artifact creature token.",
        ),
        "chromatic-orrery" => (
            "Chromatic Orrery",
            "Mana cost: {7}\nType: Legendary Artifact\nFirst printed set: Core Set 2021\nYou may spend mana as though it were mana of any color.\n{T}: Add {C}{C}{C}{C}{C}.\n{5}, {T}: Draw a card for each color among permanents you control.",
        ),
        "chrome-mox" => (
            "Chrome Mox",
            "Mana cost: {0}\nType: Artifact\nFirst printed set: Mirrodin\nImprint — When this artifact enters, you may exile a nonartifact, nonland card from your hand.\n{T}: Add one mana of any of the exiled card's colors.",
        ),
        "dusk-feaster" => (
            "Dusk Feaster",
            "Mana cost: {5}{B}{B}\nType: Creature — Vampire\nFirst printed set: Eldritch Moon\nPower/Toughness: 4/5\nDelirium — This spell costs {2} less to cast if there are four or more card types among cards in your graveyard.\nFlying",
        ),
        "elemental-resonance" => (
            "Elemental Resonance",
            "Mana cost: {2}{G}{G}\nType: Enchantment — Aura\nFirst printed set: Dissension\nEnchant permanent\nAt the beginning of your first main phase, add mana equal to enchanted permanent's mana cost. (Mana cost includes color. If a mana symbol has multiple colors, choose one.)",
        ),
        "faeburrow-elder" => (
            "Faeburrow Elder",
            "Mana cost: {1}{G}{W}\nType: Creature — Treefolk Druid\nFirst printed set: Throne of Eldraine\nPower/Toughness: 0/0\nVigilance\nThis creature gets +1/+1 for each color among permanents you control.\n{T}: For each color among permanents you control, add one mana of that color.",
        ),
        "forest" => (
            "Forest",
            "Type: Basic Land — Forest\nFirst printed set: Limited Edition Alpha\n({T}: Add {G}.)",
        ),
        "hill-giant" => (
            "Hill Giant",
            "Mana cost: {3}{R}\nType: Creature — Giant\nFirst printed set: Limited Edition Alpha\nPower/Toughness: 3/3",
        ),
        "lunar-insight" => (
            "Lunar Insight",
            "Mana cost: {2}{U}\nType: Sorcery\nFirst printed set: Foundations\nDraw a card for each different mana value among nonland permanents you control.",
        ),
        "mirrorweave" => (
            "Mirrorweave",
            "Mana cost: {2}{W/U}{W/U}\nType: Instant\nFirst printed set: Shadowmoor\nEach other creature becomes a copy of target nonlegendary creature until end of turn.",
        ),
        "mycosynth-lattice" => (
            "Mycosynth Lattice",
            "Mana cost: {6}\nType: Artifact\nFirst printed set: Darksteel\nAll permanents are artifacts in addition to their other types.\nAll cards that aren't on the battlefield, spells, and permanents are colorless.\nPlayers may spend mana as though it were mana of any color.",
        ),
        "nighthawk-scavenger" => (
            "Nighthawk Scavenger",
            "Mana cost: {1}{B}{B}\nType: Creature — Vampire Rogue\nFirst printed set: Zendikar Rising Promos\nPower/Toughness: 1+*/3\nFlying, deathtouch, lifelink\nNighthawk Scavenger's power is equal to 1 plus the number of card types among cards in your opponents' graveyards.",
        ),
        "painter-s-servant" => (
            "Painter's Servant",
            "Mana cost: {2}\nType: Artifact Creature — Scarecrow\nFirst printed set: Shadowmoor\nPower/Toughness: 1/3\nAs this creature enters, choose a color.\nAll cards that aren't on the battlefield, spells, and permanents are the chosen color in addition to their other colors.",
        ),
        "pit-of-offerings" => (
            "Pit of Offerings",
            "Type: Land — Cave\nFirst printed set: The Lost Caverns of Ixalan\nThis land enters tapped.\nWhen this land enters, exile up to three target cards from graveyards.\n{T}: Add {C}.\n{T}: Add one mana of any of the exiled cards' colors.",
        ),
        "reverse-the-sands" => (
            "Reverse the Sands",
            "Mana cost: {6}{W}{W}\nType: Sorcery\nFirst printed set: Champions of Kamigawa\nRedistribute any number of players' life totals. (Each of those players gets one life total back.)",
        ),
        "savannah-lions" => (
            "Savannah Lions",
            "Mana cost: {W}\nType: Creature — Cat\nFirst printed set: Limited Edition Alpha\nPower/Toughness: 2/1",
        ),
        "tarmogoyf" => (
            "Tarmogoyf",
            "Mana cost: {1}{G}\nType: Creature — Lhurgoyf\nFirst printed set: Future Sight\nPower/Toughness: */1+*\nTarmogoyf's power is equal to the number of card types among cards in all graveyards and its toughness is equal to that number plus 1.",
        ),
        "dark-ritual" => (
            "Dark Ritual",
            "Mana cost: {B}\nType: Instant\nFirst printed set: Limited Edition Alpha\nAdd {B}{B}{B}.",
        ),
        "opt" => (
            "Opt",
            "Mana cost: {U}\nType: Instant\nFirst printed set: Invasion\nScry 1. (Look at the top card of your library. You may put that card on the bottom.)\nDraw a card.",
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

fn put(game: &mut GameState, route: &str, owner: PlayerId, zone: Zone) -> ObjectId {
    game.create_object_from_definition(&card(route), owner, zone)
}

fn resolve_spell(
    game: &mut GameState,
    route: &str,
    targets: Vec<ironsmith::Target>,
    dm: &mut impl DecisionMaker,
) {
    let spell = put(game, route, A, Zone::Stack);
    let mut entry = ironsmith::StackEntry::new(spell, A);
    entry.targets = targets;
    game.push_to_stack(entry);
    ironsmith::game_loop::resolve_stack_entry_with(game, dm).unwrap();
}

fn can_cast(game: &GameState, card: ObjectId) -> bool {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    compute_legal_actions(game, A).unwrap().iter().any(
        |action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == card),
    )
}

#[test]
fn graveyard_casting_abilities_can_be_lost_and_regained_in_timestamp_order() {
    let opponent = PlayerId(1);
    for donor in ["underworld-breach", "past-in-flames"] {
        for grant_is_newer in [false, true] {
            let mut g = game();
            let printed = put(&mut g, "think-twice", A, Zone::Graveyard);
            let granted = put(&mut g, "divination", A, Zone::Graveyard);
            for _ in 0..4 {
                put(&mut g, "bayou-groff", A, Zone::Graveyard);
            }
            g.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ironsmith::ManaSymbol::Blue, 5);
            assert!(can_cast(&g, printed));
            assert!(!can_cast(&g, granted));
            let apply_grant = |g: &mut GameState| {
                if donor == "past-in-flames" {
                    resolve_spell(
                        g,
                        donor,
                        vec![],
                        &mut ironsmith::decision::SelectFirstDecisionMaker,
                    );
                } else {
                    put(g, donor, A, Zone::Battlefield);
                }
            };
            if !grant_is_newer {
                apply_grant(&mut g);
            }
            let jailer = put(&mut g, "yixlid-jailer", opponent, Zone::Battlefield);
            assert!(!can_cast(&g, printed), "printed flashback under Jailer");
            if grant_is_newer {
                apply_grant(&mut g);
            }
            assert_eq!(can_cast(&g, granted), grant_is_newer, "{donor}");
            g.phase_out(jailer);
            assert!(can_cast(&g, printed));
            assert!(can_cast(&g, granted));
        }
    }
}

#[test]
fn named_sources_in_hand_obey_mana_and_nonmana_prohibitions() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    let mut g = game();
    let street = put(&mut g, "street-wraith", A, Zone::Hand);
    let guide = put(&mut g, "simian-spirit-guide", A, Zone::Hand);
    let needle = put(&mut g, "pithing-needle", PlayerId(1), Zone::Battlefield);
    g.set_chosen_named_option(needle, "Street Wraith".into());
    let actions = compute_legal_actions(&g, A).unwrap();
    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == street))
    );
    g.set_chosen_named_option(needle, "Simian Spirit Guide".into());
    let actions = compute_legal_actions(&g, A).unwrap();
    assert!(
        actions.iter().any(
            |a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == guide)
        )
    );
    let revoker = put(&mut g, "phyrexian-revoker", PlayerId(1), Zone::Battlefield);
    g.set_chosen_named_option(revoker, "Simian Spirit Guide".into());
    assert!(
        !compute_legal_actions(&g, A).unwrap().iter().any(
            |a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == guide)
        )
    );
}

#[test]
fn a_creatures_additional_casting_cost_counts_as_an_ability_until_removed() {
    let mut g = game();
    let groff = put(&mut g, "bayou-groff", A, Zone::Battlefield);
    put(&mut g, "muraganda-petroglyphs", A, Zone::Battlefield);
    assert_eq!(g.current_power(groff), Some(5));
    put(&mut g, "humility", A, Zone::Battlefield);
    assert_eq!(g.current_power(groff), Some(3));
}

#[test]
fn station_power_and_toughness_are_available_only_while_stationed() {
    let mut g = game();
    for zone in [
        Zone::Library,
        Zone::Hand,
        Zone::Stack,
        Zone::Graveyard,
        Zone::Exile,
        Zone::Battlefield,
    ] {
        let id = put(&mut g, "borg-cube", A, zone);
        assert_eq!(
            (g.current_power(id), g.current_toughness(id)),
            (None, None),
            "{zone:?}"
        );
        if zone == Zone::Battlefield {
            g.add_counters(id, CounterType::Charge, 12);
            assert_eq!(
                (g.current_power(id), g.current_toughness(id)),
                (Some(6), Some(6))
            );
        }
    }
}

#[test]
fn reanimate_uses_zero_for_an_uncast_creatures_entry_x() {
    let mut g = game();
    let target = put(&mut g, "frankenstein-s-monster", A, Zone::Graveyard);
    resolve_spell(
        &mut g,
        "reanimate",
        vec![ironsmith::Target::Object(target)],
        &mut ironsmith::decision::SelectFirstDecisionMaker,
    );
    assert_eq!(g.player(A).unwrap().life, 18);
    let permanent = g
        .battlefield
        .iter()
        .copied()
        .find(|id| g.object(*id).unwrap().name.as_ref() == "Frankenstein's Monster")
        .unwrap();
    assert_eq!(
        (g.current_power(permanent), g.current_toughness(permanent)),
        (Some(0), Some(1))
    );
}

#[test]
fn another_player_excludes_the_chooser_before_transferring_control() {
    let mut g = game();
    let financier = put(&mut g, "discerning-financier", A, Zone::Battlefield);
    let treasure = CardDefinitionBuilder::new(CardId::new(), "Treasure")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![ironsmith::Subtype::Treasure])
        .build();
    let t = g.create_object_from_definition(&treasure, A, Zone::Battlefield);
    put(&mut g, "divination", A, Zone::Library);
    let program = g
        .current_abilities(financier)
        .unwrap()
        .into_iter()
        .find_map(|a| match a.kind {
            AbilityKind::Activated(a) => Some(a.effects),
            _ => None,
        })
        .unwrap();
    let mut entry = ironsmith::StackEntry::ability(financier, A, program);
    entry.targets = vec![ironsmith::Target::Object(t)];
    g.push_to_stack(entry);
    ironsmith::game_loop::resolve_stack_entry_with(
        &mut g,
        &mut ironsmith::decision::SelectFirstDecisionMaker,
    )
    .unwrap();
    assert_eq!(g.current_controller(t), Some(PlayerId(1)));
    assert_eq!(g.player(A).unwrap().hand.len(), 1);
}

#[derive(Default)]
struct MultiplayerDecisions {
    voters: Vec<PlayerId>,
    bidders: Vec<PlayerId>,
}

impl DecisionMaker for MultiplayerDecisions {
    fn decide_options(&mut self, _: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        self.voters.push(c.player);
        vec![c.options.iter().find(|o| o.legal).unwrap().index]
    }
    fn decide_boolean(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        self.bidders.push(c.player);
        false
    }
    fn decide_proliferate(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::ProliferateContext,
    ) -> ironsmith::decisions::specs::ProliferateResponse {
        ironsmith::decisions::specs::ProliferateResponse {
            permanents: c.eligible_permanents.iter().map(|p| p.0).collect(),
            players: c.eligible_players.iter().map(|p| p.0).collect(),
        }
    }
}

#[test]
fn multiplayer_choices_and_characteristics_respect_range_and_turn_direction() {
    let mut g = GameState::new(
        ["A", "B", "C", "D", "E"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        20,
    );
    let seats: Vec<_> = (0..5).map(PlayerId).collect();
    g.enable_limited_range_of_influence(seats.clone(), vec![1; 5])
        .unwrap();
    g.turn.active_player = A;
    g.turn_store.turn_order = seats.iter().copied().rev().collect();
    let c = PlayerId(2);
    let golem = put(&mut g, "altar-golem", A, Zone::Battlefield);
    let power = g.current_power(golem);
    let distant = put(&mut g, "bayou-groff", c, Zone::Battlefield);
    assert_eq!(g.current_power(golem), power);
    g.add_counters(distant, CounterType::PlusOnePlusOne, 1);
    g.player_mut(c).unwrap().poison_counters = 1;
    g.player_mut(A).unwrap().poison_counters = 1;
    for _ in 0..9 {
        put(&mut g, "divination", A, Zone::Library);
    }
    let mut dm = MultiplayerDecisions::default();
    resolve_spell(&mut g, "contentious-plan", vec![], &mut dm);
    assert_eq!(g.player(A).unwrap().poison_counters, 2);
    assert_eq!(g.player(c).unwrap().poison_counters, 1);
    assert_eq!(g.current_power(distant), Some(6));
    resolve_spell(&mut g, "plea-for-power", vec![], &mut dm);
    assert_eq!(dm.voters, [A, PlayerId(4), PlayerId(1)]);
    resolve_spell(&mut g, "pain-s-reward", vec![], &mut dm);
    assert!(dm.bidders.contains(&PlayerId(1)));
    assert!(!dm.bidders.contains(&c) && !dm.bidders.contains(&PlayerId(3)));
}

#[path = "november_2026_rules/characteristic_interactions.rs"]
mod characteristic_interactions;
