//! Claude package 58 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::ability::AbilityKind;
use ironsmith::ids::CardId;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::types::CardType;
use ironsmith::{PlayerId, Zone};

const P58_MANA_SCULPT: &str = "Mana cost: {1}{U}{U}\nType: Instant\nCounter target spell. If you control a Wizard, add an amount of {C} equal to the amount of mana spent to cast that spell at the beginning of your next main phase.";
const P58_MALAMET: &str = "Mana cost: {G}\nType: Sorcery\nChoose target creature you control and target creature you don't control. If the creature you control entered this turn, put a +1/+1 counter on it. Then those creatures fight each other.";
const P58_SAMUT: &str = "Mana cost: {1}{R}{G}\nType: Legendary Creature — Human Warrior Cleric\nPower/Toughness: 2/3\nFirst strike, vigilance, haste\nWhenever a creature you control deals combat damage to a player, if that creature entered this turn, draw a card.";
const P58_MAGES_CONTEST: &str = "Mana cost: {1}{R}{R}\nType: Instant\nYou and target spell's controller bid life. You start the bidding with a bid of 1. In turn order, each player may top the high bid. The bidding ends if the high bid stands. The high bidder loses life equal to the high bid. If you win the bidding, counter that spell.";
const P58_LIM_DULS_VAULT: &str = "Mana cost: {U}{B}\nType: Instant\nLook at the top five cards of your library. As many times as you choose, you may pay 1 life, put those cards on the bottom of your library in any order, then look at the top five cards of your library. Then shuffle and put the last cards you looked at this way on top in any order.";
const P58_LOST_IN_THOUGHT: &str = "Mana cost: {1}{U}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature can't attack or block, and its activated abilities can't be activated. Its controller may exile three cards from their graveyard for that player to ignore this effect until end of turn.";
const P58_MARSHLAND_BLOODCASTER: &str = "Mana cost: {4}{B}\nType: Creature — Vampire Warlock\nPower/Toughness: 3/5\nFlying\n{1}{B}, {T}: Rather than pay the mana cost of the next spell you cast this turn, you may pay life equal to that spell's mana value.";
const P58_MALCOLM: &str = "Mana cost: {2}{U}\nType: Legendary Creature — Siren Pirate\nPower/Toughness: 2/2\nFlying\nWhenever one or more Pirates you control deal damage to your opponents, you create a Treasure token for each opponent dealt damage. (It's an artifact with \"{T}, Sacrifice this token: Add one mana of any color.\")\nPartner (You can have two commanders if both have partner.)";

fn p58_spell_debug(definition: &ironsmith::cards::CardDefinition) -> String {
    format!("{:?}", definition.spell_effect.as_ref().expect("spell effect"))
}

#[test]
fn mana_sculpt_checks_wizard_on_resolution_and_delays_only_the_mana() {
    let definition = compile_to_runtime_definition("Mana Sculpt", P58_MANA_SCULPT, false)
        .expect("Mana Sculpt should compile");
    let debug = p58_spell_debug(&definition);
    let condition = debug.find("ConditionalEffect").expect("a resolution-time Wizard check");
    let schedule = debug
        .find("ScheduleDelayedTriggerEffect")
        .expect("the mana is added by a delayed trigger");
    assert!(condition < schedule, "the Wizard check wraps the schedule: {debug}");
    assert!(
        !debug[schedule..].contains("Wizard"),
        "the delayed trigger itself must not re-check for a Wizard: {debug}"
    );
    assert!(debug[schedule..].contains("AddScaledManaEffect"), "{debug}");
    assert!(debug.contains("ManaSpentToCast"), "{debug}");
}

#[test]
fn malamet_battle_glyph_tests_the_first_target_for_entering_this_turn() {
    let definition = compile_to_runtime_definition("Malamet Battle Glyph", P58_MALAMET, false)
        .expect("Malamet Battle Glyph should compile");
    let debug = p58_spell_debug(&definition);
    assert!(debug.contains("entered_battlefield_this_turn: true"), "{debug}");
    let condition = debug.find("condition:").expect("a resolution-time condition");
    let if_true = debug[condition..].find("if_true").map(|offset| condition + offset).unwrap();
    let tested = &debug[condition..if_true];
    assert!(
        tested.contains("TaggedObjectMatches") && !tested.contains("targeted_1"),
        "the condition reads the creature you control, not the newest target: {tested}"
    );
    let counters = debug.find("PutCountersEffect").expect("a +1/+1 counter");
    let counters_end = debug[counters..]
        .find("FightEffect")
        .map_or(debug.len(), |offset| counters + offset);
    assert!(
        !debug[counters..counters_end].contains("targeted_1"),
        "'it' is the creature you control: {debug}"
    );
    assert!(debug.contains("FightEffect"), "{debug}");
}

#[test]
fn demonstrative_entered_this_turn_keeps_the_entry_history() {
    let definition = compile_to_runtime_definition("Samut, Vizier of Naktamun", P58_SAMUT, false)
        .expect("Samut should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(
        debug.contains("entered_battlefield_this_turn: true"),
        "'that creature entered this turn' must not degrade to 'is a permanent': {debug}"
    );
}

struct P58Bidder {
    bob_tops: bool,
    asked: Vec<PlayerId>,
}
impl ironsmith::decision::DecisionMaker for P58Bidder {
    fn decide_boolean(
        &mut self,
        _: &ironsmith::GameState,
        ctx: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        self.asked.push(ctx.player);
        ctx.player == PlayerId::from_index(1)
            && self.bob_tops
            && self.asked.iter().filter(|player| **player == ctx.player).count() == 1
    }
    fn decide_number(
        &mut self,
        _: &ironsmith::GameState,
        ctx: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        ctx.min
    }
}

#[test]
fn mages_contest_two_bidder_auction_counters_only_when_controller_wins() {
    let definition = compile_to_runtime_definition("Mages' Contest", P58_MAGES_CONTEST, false)
        .expect("Mages' Contest should compile");
    let debug = p58_spell_debug(&definition);
    assert!(debug.contains("BidLifeEffect"), "{debug}");
    assert!(debug.contains("controller_must_win: true"), "{debug}");
    assert!(debug.contains("opponent: Some("), "{debug}");
    assert!(debug.contains("Fixed(1)"), "the opening bid is 1: {debug}");
    assert!(debug.contains("CounterEffect"), "{debug}");

    let (alice, bob, carol) = (PlayerId::from_index(0), PlayerId::from_index(1), PlayerId::from_index(2));
    for bob_tops in [false, true] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let mut bid = ironsmith::effects::BidLifeEffect::untargeted(
            ironsmith::effects::LifeBidStart::Fixed(1),
            vec![Effect::gain_life(10)],
        );
        bid.opponent = Some(ironsmith::target::PlayerFilter::Specific(bob));
        bid.controller_must_win = true;
        let mut bidder = P58Bidder { bob_tops, asked: Vec::new() };
        let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut bidder);
        ironsmith::effects::execute_effect(&mut game, &Effect::new(bid), &mut ctx).unwrap();
        drop(ctx);
        assert!(!bidder.asked.contains(&carol), "only you and that player bid: {:?}", bidder.asked);
        if bob_tops {
            assert_eq!(game.player(bob).unwrap().life, 18, "Bob bid 2 and pays it");
            assert_eq!(game.player(alice).unwrap().life, 20, "no reward when you lose");
        } else {
            assert_eq!(game.player(alice).unwrap().life, 29, "you pay 1 and get the reward");
            assert_eq!(game.player(bob).unwrap().life, 20);
        }
    }
}

fn p58_look_tags(debug: &str) -> Vec<String> {
    debug
        .match_indices("LookAtTopCardsEffect")
        .filter_map(|(start, _)| {
            let rest = &debug[start..];
            let tag = rest.find("tag: TagKey(\"")? + "tag: TagKey(\"".len();
            let end = rest[tag..].find('"')?;
            Some(rest[tag..tag + end].to_string())
        })
        .collect()
}

#[test]
fn lim_duls_vault_repeats_paid_looks_and_keeps_the_last_five_on_top() {
    let definition = compile_to_runtime_definition("Lim-Dûl's Vault", P58_LIM_DULS_VAULT, false)
        .expect("Lim-Dûl's Vault should compile");
    let debug = p58_spell_debug(&definition);
    let tags = p58_look_tags(&debug);
    assert!(tags.len() >= 2, "an opening look and a look each round: {debug}");
    assert!(tags.iter().all(|tag| tag == &tags[0]), "every look re-tags one group: {tags:?}");
    assert!(debug.contains("PayLifeEffect"), "{debug}");
    assert!(debug.contains("MayEffect"), "each round is optional: {debug}");
    let may = debug.find("MayEffect").unwrap();
    let round_look = debug[may..].find("LookAtTopCardsEffect").map(|offset| may + offset);
    assert!(round_look.is_some(), "the round's look is part of the optional round: {debug}");
    assert!(debug.contains("Shuffle"), "{debug}");
    assert!(debug.matches(&tags[0]).count() >= 3, "the bottom move and final top move use the looked group: {debug}");
}

#[test]
fn lost_in_thought_controller_exiles_three_graveyard_cards_to_ignore_the_aura() {
    let definition = compile_to_runtime_definition("Lost in Thought", P58_LOST_IN_THOUGHT, false)
        .expect("Lost in Thought should compile");
    let ids: Vec<_> = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => Some(ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&StaticAbilityId::AttachedControllerMayPayCostToIgnoreSourceEffectUntilEndOfTurn),
        "{ids:?}"
    );
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.matches("RuleRestriction").count() >= 2, "{debug}");

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let aura = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let index = game
        .object(aura)
        .unwrap()
        .abilities
        .iter()
        .position(|ability| matches!(&ability.kind, AbilityKind::Static(ability)
            if ability.id() == StaticAbilityId::AttachedControllerMayPayCostToIgnoreSourceEffectUntilEndOfTurn))
        .expect("the special-action permission");
    let display = ironsmith::special_actions::ignore_attached_restriction_cost_display(&game, aura, index);
    assert!(display.to_lowercase().contains("exile"), "{display}");
    assert!(display.to_lowercase().contains("graveyard"), "{display}");
}

#[test]
fn marshland_bloodcaster_grants_next_spell_a_life_alternative_cost() {
    let definition =
        compile_to_runtime_definition("Marshland Bloodcaster", P58_MARSHLAND_BLOODCASTER, false)
            .expect("Marshland Bloodcaster should compile");
    fn walk(effect: &Effect, out: &mut Vec<Effect>) {
        out.push(effect.clone());
        effect.visit_child_effects(&mut |child| walk(child, out));
    }
    let mut effects = Vec::new();
    for ability in &definition.abilities {
        if let AbilityKind::Activated(activated) = &ability.kind {
            for effect in activated.effects.all_effects() {
                walk(effect, &mut effects);
            }
        }
    }
    let grant = effects
        .iter()
        .find_map(|effect| {
            effect
                .downcast_ref::<ironsmith::effects::GrantNextSpellCostReductionEffect>()
                .cloned()
        })
        .expect("a one-shot next-spell grant");
    assert!(grant.without_paying_mana_cost && grant.pay_life_equal_to_mana_value, "{grant:?}");
    assert!(!grant.applies_to_all_matching_this_turn, "{grant:?}");
}

#[test]
fn malcolm_creates_one_treasure_per_damaged_opponent_occurrence() {
    let definition = compile_to_runtime_definition("Malcolm, Keen-Eyed Navigator", P58_MALCOLM, false)
        .expect("Malcolm should compile");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("the Pirate damage trigger");
    let trigger = format!("{:?}", triggered.trigger);
    assert!(trigger.contains("Pirate") && trigger.contains("Opponent"), "{trigger}");
    let effects = format!("{:?}", triggered.effects);
    assert!(effects.contains("CountPlayers(DamagedPlayer)"), "{effects}");
    assert!(effects.contains("Treasure"), "{effects}");
}

const P58_LONG_LIST: &str = "Mana cost: {G}\nType: Enchantment — Saga\n(As this Saga enters and after your draw step, add a lore counter. Sacrifice after VI.)\nI, II, III, IV, V, VI — Note a creature type that hasn't been noted for this Saga. When you next cast a creature spell of that type this turn, that creature enters with an additional +1/+1 counter on it.";

#[test]
fn long_list_of_the_ents_notes_a_fresh_creature_type_each_chapter() {
    let definition = compile_to_runtime_definition("Long List of the Ents", P58_LONG_LIST, false)
        .expect("Long List of the Ents should compile");
    fn walk(effect: &Effect, out: &mut Vec<Effect>) {
        out.push(effect.clone());
        effect.visit_child_effects(&mut |child| walk(child, out));
    }
    let mut effects = Vec::new();
    for ability in &definition.abilities {
        if let AbilityKind::Triggered(triggered) = &ability.kind {
            for effect in triggered.effects.all_effects() {
                walk(effect, &mut effects);
            }
        }
    }
    let note = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::ChooseCreatureTypeEffect>().cloned())
        .expect("the chapter notes a creature type");
    assert!(note.note_for_source, "{note:?}");
    let debug = format!("{effects:?}");
    assert!(debug.contains("ScheduleDelayedTriggerEffect"), "{debug}");
    assert!(debug.contains("chosen_creature_type: true"), "'of that type' reads the noted type: {debug}");

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let saga = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    for _ in 0..2 {
        let mut ctx = ironsmith::effects::EffectContext::new_default(saga, alice);
        ironsmith::effects::execute_effect(&mut game, &Effect::new(note.clone()), &mut ctx).unwrap();
    }
    let noted = game.noted_creature_types_for_source(saga).to_vec();
    assert_eq!(noted.len(), 2, "each chapter notes a type not yet noted: {noted:?}");
    assert_ne!(noted[0], noted[1], "{noted:?}");
    assert_eq!(game.chosen_subtype(saga), Some(noted[1]), "'that type' is the latest noted type");
}

const P58_MARALEN: &str = "Mana cost: {2}{B}{G}{U}\nType: Legendary Creature — Elf Faerie Noble\nPower/Toughness: 4/5\nFlying\nWhenever Maralen or another Elf or Faerie you control enters, exile the top two cards of target opponent's library.\nOnce each turn, you may cast a spell with mana value less than or equal to the number of Elves and Faeries you control from among cards exiled with Maralen this turn without paying its mana cost.";

#[test]
fn maralen_once_each_turn_free_casts_from_cards_it_exiled_this_turn() {
    let definition = compile_to_runtime_definition("Maralen, Fae Ascendant", P58_MARALEN, false)
        .expect("Maralen should compile");
    let statics = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => Some(format!("{ability:?}")),
            _ => None,
        })
        .collect::<Vec<_>>();
    let grant = statics
        .iter()
        .find(|debug| debug.contains("OnceEachTurn"))
        .unwrap_or_else(|| panic!("a once-each-turn permission: {statics:#?}"));
    assert!(grant.contains("__source_exiled_by_you_this_turn__"), "only cards exiled this turn: {grant}");
    assert!(grant.contains("Exile"), "{grant}");
    assert!(grant.contains("mana_value: Some("), "the dynamic mana-value cap: {grant}");
    assert!(grant.contains("Elf") && grant.contains("Faerie"), "{grant}");
    assert!(grant.contains("without paying") || grant.contains("free"), "{grant}");
}
