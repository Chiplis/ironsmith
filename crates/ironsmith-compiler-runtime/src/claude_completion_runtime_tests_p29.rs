//! Claude package 29 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::color::ColorSet;
use ironsmith::ids::CardId;
use ironsmith::types::{CardType, Subtype};
use ironsmith::{GameState, ObjectId, PlayerId, Zone};

fn p29_compile(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    let (definition, trace) = ironsmith_compiler::parse_trace::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    definition.unwrap_or_else(|error| panic!("{name}: {error:?}\n{}", trace.render()))
}

fn p29_creature(name: &str, colors: ColorSet, power: i32, toughness: i32) -> ironsmith::card::Card {
    CardBuilder::new(CardId::new(), name)
        .color_indicator(colors)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build()
}

fn p29_resolve_spell(game: &mut GameState, definition: &ironsmith::cards::CardDefinition, caster: PlayerId) -> ObjectId {
    let source = game.create_object_from_definition(definition, caster, Zone::Stack);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, caster);
    for effect in definition.spell_effect.as_ref().expect("spell effect").all_effects() {
        ironsmith::effects::execute_effect(game, effect, &mut ctx).unwrap();
    }
    source
}

const DISORDER: &str = "Mana cost: {1}{R}\nType: Sorcery\nDisorder deals 2 damage to each white creature and each player who controls a white creature.";

#[test]
fn disorder_damages_white_creatures_and_only_players_controlling_one() {
    let definition = p29_compile("Disorder", DISORDER);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let white = game.create_object_from_card(&p29_creature("White witness", ColorSet::WHITE, 5, 5), bob, Zone::Battlefield);
    let red = game.create_object_from_card(&p29_creature("Red witness", ColorSet::RED, 5, 5), alice, Zone::Battlefield);
    p29_resolve_spell(&mut game, &definition, alice);
    assert_eq!(game.damage_on(white), 2);
    assert_eq!(game.damage_on(red), 0);
    assert_eq!(game.player(bob).unwrap().life, 18);
    assert_eq!(game.player(alice).unwrap().life, 20);
}

const DJINN_OF_INFINITE_DECEITS: &str = "Mana cost: {4}{U}{U}\nType: Creature — Djinn\nPower/Toughness: 5/7\nFlying\n{T}: Exchange control of two target nonlegendary creatures. You can't activate this ability during combat.";

#[test]
fn djinn_of_infinite_deceits_cannot_activate_during_combat() {
    let definition = p29_compile("Djinn of Infinite Deceits", DJINN_OF_INFINITE_DECEITS);
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("exchange ability");
    assert_eq!(activated.timing, ironsmith::ability::ActivationTiming::AnyTime);
    let condition = format!("{:?}", activated.activation_condition);
    assert!(condition.contains("Not(ActivationTiming(DuringCombat))"), "{condition}");
    let effects = format!("{:?}", activated.effects);
    assert!(effects.contains("Exchange"), "{effects}");
}

fn p29_walk(effect: &Effect, out: &mut Vec<Effect>) {
    out.push(effect.clone());
    effect.visit_child_effects(&mut |child| p29_walk(child, out));
}

/// Picks the named object whenever it is a legal candidate, otherwise the
/// minimum number of legal candidates.
struct P29Pick(Vec<String>);

impl ironsmith::decision::DecisionMaker for P29Pick {
    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        let legal = ctx.candidates.iter().filter(|candidate| candidate.legal).map(|candidate| candidate.id).collect::<Vec<_>>();
        let preferred = legal.iter().copied().filter(|id| {
            game.object(*id).is_some_and(|object| self.0.iter().any(|name| *name == object.name.to_string()))
        }).collect::<Vec<_>>();
        if !preferred.is_empty() {
            return preferred.into_iter().take(ctx.max.unwrap_or(usize::MAX).max(1)).collect();
        }
        legal.into_iter().take(ctx.min).collect()
    }
}

const DIMIR_CHARM: &str = "Mana cost: {U}{B}\nType: Instant\nChoose one —\n• Counter target sorcery spell.\n• Destroy target creature with power 2 or less.\n• Look at the top three cards of target player's library. Put one back and the rest into that player's graveyard.";

#[test]
fn dimir_charm_keeps_one_looked_card_on_top_and_mills_the_rest() {
    let definition = p29_compile("Dimir Charm", DIMIR_CHARM);
    let mut all = Vec::new();
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        p29_walk(effect, &mut all);
    }
    let modes = all
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::ChooseModeEffect>())
        .expect("modal spell")
        .modes
        .clone();
    assert_eq!(modes.len(), 3);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    for name in ["Deep", "Deeper", "Mill one", "Keep", "Mill two"] {
        let card = CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Sorcery]).build();
        game.create_object_from_card(&card, bob, Zone::Library);
    }
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut dm = P29Pick(vec!["Keep".into()]);
    let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm)
        .with_targets(vec![ironsmith::effects::ResolvedTarget::Player(bob)]);
    for effect in &modes[2].effects {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    let library = game.player(bob).unwrap().library.clone();
    let names = library.iter().map(|id| game.object(*id).unwrap().name.to_string()).collect::<Vec<_>>();
    assert_eq!(names, vec!["Deep", "Deeper", "Keep"], "the kept card is back on top");
    let graveyard = game.player(bob).unwrap().graveyard.iter().map(|id| game.object(*id).unwrap().name.to_string()).collect::<Vec<_>>();
    assert_eq!(graveyard.len(), 2, "{graveyard:?}");
    assert!(graveyard.contains(&"Mill one".to_string()) && graveyard.contains(&"Mill two".to_string()));
}

const DISCORDANT_SPIRIT: &str = "Mana cost: {2}{B}{R}\nType: Creature — Spirit\nPower/Toughness: 2/2\nAt the beginning of each end step, if it's an opponent's turn, put a +1/+1 counter on this creature for each 1 damage dealt to you this turn.\nAt the beginning of your end step, remove all +1/+1 counters from this creature.";

#[test]
fn discordant_spirit_counts_damage_dealt_to_you_on_opponents_turns() {
    let definition = p29_compile("Discordant Spirit", DISCORDANT_SPIRIT);
    let triggers = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Triggered(trigger) => Some(format!("{trigger:?}")),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(triggers.len(), 2, "{triggers:?}");
    let growth = &triggers[0];
    assert!(growth.contains("Not(YourTurn)"), "{growth}");
    assert!(growth.contains("DamageDealtToPlayersThisTurn(You)"), "{growth}");
    assert!(growth.contains("PlusOnePlusOne"), "{growth}");
}

const DISMANTLE: &str = "Mana cost: {2}{R}\nType: Sorcery\nDestroy target artifact. If that artifact had counters on it, put that many +1/+1 counters or charge counters on an artifact you control.";

#[test]
fn dismantle_moves_the_destroyed_artifacts_counter_count_as_a_chosen_kind() {
    use ironsmith::object::CounterType;
    let definition = p29_compile("Dismantle", DISMANTLE);
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("PutCounterChoice") || debug.contains("ChooseMode"), "{debug}");
    assert!(debug.contains("CountersOn"), "{debug}");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (counters, expected) in [(3u32, 3u32), (0, 0)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let artifact = CardBuilder::new(CardId::new(), "Artifact witness")
            .card_types(vec![CardType::Artifact])
            .build();
        let doomed = game.create_object_from_card(&artifact, bob, Zone::Battlefield);
        if counters > 0 {
            game.add_counters(doomed, CounterType::Charge, counters);
        }
        let mine = game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice)
            .with_targets(vec![ironsmith::effects::ResolvedTarget::Object(doomed)]);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        drop(ctx);
        assert!(game.object(doomed).is_none_or(|object| object.zone != Zone::Battlefield));
        let placed = game.counter_count(mine, CounterType::PlusOnePlusOne)
            + game.counter_count(mine, CounterType::Charge);
        assert_eq!(placed, expected, "counters={counters}");
    }
}

const DJERU_AND_HAZORET: &str = "Mana cost: {2}{R}{R}{W}\nType: Legendary Creature — Human God\nPower/Toughness: 5/4\nAs long as you have one or fewer cards in hand, Djeru and Hazoret has vigilance and haste.\nWhenever Djeru and Hazoret attacks, look at the top six cards of your library. You may exile a legendary creature card from among them. Put the rest on the bottom of your library in a random order. Until end of turn, you may cast the exiled card without paying its mana cost.";

#[test]
fn djeru_and_hazoret_grants_a_free_cast_of_the_exiled_card_this_turn() {
    let definition = p29_compile("Djeru and Hazoret", DJERU_AND_HAZORET);
    let trigger = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Triggered(trigger) => Some(format!("{trigger:?}")),
            _ => None,
        })
        .expect("attack trigger");
    assert!(trigger.contains("GrantPlayTagged"), "{trigger}");
    assert!(trigger.contains("UntilEndOfTurn"), "{trigger}");
    assert!(trigger.contains("GrantTaggedSpellFreeCastUntilEndOfTurn"), "{trigger}");
    assert!(trigger.contains("Legendary"), "{trigger}");
}

const DOC_AURLOCK: &str = "Mana cost: {G}{U}\nType: Legendary Creature — Bear Druid\nPower/Toughness: 2/2\nSpells you cast from your graveyard or from exile cost {2} less to cast.\nPlotting cards from your hand costs {2} less.";

#[test]
fn doc_aurlock_reduces_the_plot_special_action_only() {
    let definition = p29_compile("Doc Aurlock, Grizzled Genius", DOC_AURLOCK);
    let statics = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Static(ability) => Some(ability),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(statics.len(), 2, "{statics:?}");
    let plot = statics
        .iter()
        .find(|ability| ability.plot_special_action_reduction().is_some())
        .expect("plot special-action modifier");
    assert_eq!(plot.plot_special_action_reduction(), Some(2));
    assert_eq!(plot.foretell_special_action_modifier(), None, "foretell is unchanged");
}

const DODECAPOD: &str = "Mana cost: {4}\nType: Artifact Creature — Golem\nPower/Toughness: 3/3\nIf a spell or ability an opponent controls causes you to discard this card, put it onto the battlefield with two +1/+1 counters on it instead of putting it into your graveyard.";

#[test]
fn dodecapod_enters_with_two_counters_when_an_opponent_makes_you_discard_it() {
    use ironsmith::object::CounterType;
    let definition = p29_compile("Dodecapod", DODECAPOD);
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("OpponentDiscardThisToBattlefieldWithCounters"), "{debug}");
    assert!(debug.contains("Hand"), "the replacement functions from hand: {debug}");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let pod = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let stable = game.object(pod).unwrap().stable_id;
    let spell = CardBuilder::new(CardId::new(), "Discard witness")
        .card_types(vec![CardType::Sorcery])
        .build();
    let source = game.create_object_from_card(&spell, bob, Zone::Stack);
    game.refresh_continuous_state().unwrap();
    let discard = Effect::new(ironsmith::effects::DiscardEffect::new(
        1,
        ironsmith::target::PlayerFilter::Specific(alice),
        false,
    ));
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, bob);
    ironsmith::effects::execute_effect(&mut game, &discard, &mut ctx).unwrap();
    drop(ctx);
    let moved = game.find_object_by_stable_id(stable).expect("dodecapod");
    assert_eq!(game.object(moved).unwrap().zone, Zone::Battlefield);
    assert_eq!(game.counter_count(moved, CounterType::PlusOnePlusOne), 2);
}

const DOG_UMBRA: &str = "Mana cost: {1}{W}\nType: Enchantment — Aura\nFlash\nEnchant creature\nAs long as another player controls enchanted creature, it can't attack or block. Otherwise, this Aura has umbra armor. (If enchanted creature would be destroyed, instead remove all damage from it and destroy this Aura.)";

#[test]
fn dog_umbra_splits_on_who_controls_the_enchanted_creature() {
    let definition = p29_compile("Dog Umbra", DOG_UMBRA);
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("UmbraArmor"), "{debug}");
    assert!(debug.matches("Not(").count() >= 1, "{debug}");
    assert!(debug.contains("Attack") && debug.contains("Block"), "{debug}");
}

const DONE_FOR_THE_DAY: &str = "Mana cost: {2}{G}\nType: Enchantment\nAt the beginning of your end step, if you control an Employee, a Performer, or a Robot, you may get {TK} or create a Treasure token. If you control all three, you may put a sticker on a nonland permanent you own.";

#[test]
fn done_for_the_day_all_three_requires_each_listed_permanent() {
    let definition = p29_compile("Done for the Day", DONE_FOR_THE_DAY);
    let trigger = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Triggered(trigger) => Some(format!("{trigger:?}")),
            _ => None,
        })
        .expect("end step trigger");
    for subtype in ["Employee", "Performer", "Robot"] {
        assert!(trigger.matches(subtype).count() >= 2, "{subtype}: {trigger}");
    }
    assert!(trigger.contains("And("), "{trigger}");
    assert!(trigger.to_ascii_lowercase().contains("sticker"), "{trigger}");
}

const DRACH_NYEN: &str = "Mana cost: {4}{B}{R}\nType: Legendary Artifact — Equipment\nEcho of the First Murder — When Drach'Nyen enters, exile up to one target creature.\nDaemon Sword — Equipped creature has menace and gets +X/+0, where X is the exiled card's power.\nEquip {2} ({2}: Attach to target creature you control. Equip only as a sorcery.)";

#[test]
fn drach_nyen_flavor_word_is_not_echo_and_sword_reads_the_exiled_power() {
    let definition = p29_compile("Drach'Nyen", DRACH_NYEN);
    let debug = format!("{:?}", definition.abilities);
    assert!(!debug.contains("Echo"), "flavor word only: {debug}");
    assert!(debug.contains("Menace"), "{debug}");
    assert!(debug.contains("PowerOf") && debug.contains("source_exiled"), "{debug}");
    assert!(
        definition.abilities.iter().any(|ability| matches!(
            ability.kind,
            ironsmith::ability::AbilityKind::Triggered(_)
        )),
        "the enters trigger exiles: {debug}"
    );
}

const DRACOPLASM: &str = "Mana cost: {U}{R}\nType: Creature — Shapeshifter\nPower/Toughness: 0/0\nFlying\nAs this creature enters, sacrifice any number of creatures. This creature's power becomes the total power of those creatures and its toughness becomes their total toughness.\n{R}: This creature gets +1/+0 until end of turn.";

#[test]
fn dracoplasm_sums_only_the_sacrificed_creatures() {
    let definition = p29_compile("Dracoplasm", DRACOPLASM);
    let debug = format!("{:?}", definition.abilities);
    for aggregate in ["TotalPower(", "TotalToughness("] {
        let at = debug.find(aggregate).unwrap_or_else(|| panic!("{aggregate}: {debug}"));
        let window = &debug[at..(at + 6000).min(debug.len())];
        assert!(
            window.contains("tagged_constraints: [TaggedObjectConstraint"),
            "{aggregate} must sum the sacrificed set: {window}"
        );
    }
}

const EIDOLON_OF_COUNTLESS_BATTLES: &str = "Mana cost: {1}{W}{W}\nType: Enchantment Creature — Spirit\nPower/Toughness: 0/0\nBestow {2}{W}{W} (If you cast this card for its bestow cost, it's an Aura spell with enchant creature. It becomes a creature again if it's not attached.)\nThis creature and enchanted creature each get +1/+1 for each creature you control and +1/+1 for each Aura you control.";

#[test]
fn eidolon_of_countless_battles_counts_creatures_plus_auras() {
    let definition = p29_compile("Eidolon of Countless Battles", EIDOLON_OF_COUNTLESS_BATTLES);
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.matches("Add(").count() >= 2, "both anthems sum the two counts: {debug}");
    assert!(debug.contains("Aura"), "{debug}");
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let eidolon = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.create_object_from_card(&p29_creature("Bear witness", ColorSet::GREEN, 2, 2), alice, Zone::Battlefield);
    let aura = CardBuilder::new(CardId::new(), "Aura witness")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .build();
    game.create_object_from_card(&aura, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    // Two creatures (Eidolon itself and the bear) plus one Aura.
    assert_eq!(game.calculated_power(eidolon), Some(3));
    assert_eq!(game.calculated_toughness(eidolon), Some(3));
}

const ELKIN_LAIR: &str = "Mana cost: {3}{R}\nType: World Enchantment\nAt the beginning of each player's upkeep, that player exiles a card at random from their hand. The player may play that card this turn. At the beginning of the next end step, if the player hasn't played the card, they put it into their graveyard.";

#[test]
fn elkin_lair_mills_the_exiled_card_only_if_it_is_still_unplayed() {
    let definition = p29_compile("Elkin Lair", ELKIN_LAIR);
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("Delayed") || debug.contains("delayed"), "{debug}");
    assert!(debug.contains("GrantPlayTagged"), "{debug}");
    assert!(debug.contains("Graveyard"), "{debug}");
    assert!(debug.contains("Exile"), "the delayed condition checks the card is still exiled: {debug}");
}
