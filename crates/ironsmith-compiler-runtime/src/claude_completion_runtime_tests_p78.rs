//! Claude package 78 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/claude2/BRIEF.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::ids::CardId;
use ironsmith::target::ChooseSpec;
use ironsmith::types::CardType;
use ironsmith::{PlayerId, Zone};

fn p78_visit(effect: &Effect, f: &mut dyn FnMut(&Effect)) {
    f(effect);
    effect.visit_child_effects(&mut |child| p78_visit(child, f));
}

/// Top-level effects of every triggered/activated ability and the spell.
fn p78_top_effects(definition: &ironsmith::cards::CardDefinition) -> Vec<Effect> {
    let mut out = Vec::new();
    for ability in &definition.abilities {
        match &ability.kind {
            ironsmith::ability::AbilityKind::Triggered(trigger) => {
                out.extend(trigger.effects.all_effects().into_iter().cloned());
            }
            ironsmith::ability::AbilityKind::Activated(activated) => {
                out.extend(activated.effects.all_effects().into_iter().cloned());
            }
            _ => {}
        }
    }
    if let Some(program) = definition.spell_effect.as_ref() {
        out.extend(program.all_effects().into_iter().cloned());
    }
    out
}

fn p78_all_effects(definition: &ironsmith::cards::CardDefinition) -> Vec<Effect> {
    let mut out = Vec::new();
    for effect in p78_top_effects(definition) {
        p78_visit(&effect, &mut |effect| out.push(effect.clone()));
    }
    out
}

/// Myth Unbound: the cost reduction applies to commander spells only, not to
/// every spell you cast (the command-zone trigger also carries is_commander,
/// so check the cost-reduction payload itself).
#[test]
fn myth_unbound_reduces_only_commander_spells() {
    let definition = compile_to_runtime_definition("Myth Unbound",
        "Mana cost: {2}{G}\nType: Enchantment\nYour commander costs {1} less to cast for each time it's been cast from the command zone this game.\nWhenever your commander is put into the command zone from anywhere, draw a card.", false)
        .expect("Myth Unbound should compile");
    let debug = format!("{:?}", definition.abilities);
    let start = debug.find("CostReduction(CostReduction").expect("commander cost reduction");
    let rest = &debug[start..];
    let filter = &rest[..rest.find("amount:").expect("reduction amount")];
    assert!(filter.contains("is_commander: true"), "only commander spells: {filter}");
    assert!(rest.contains("CommanderCastCount"), "{rest}");
}

/// The Deck of Many Things: "When that creature dies, its owner loses the
/// game" belongs to the 20 row and watches the returned creature.
#[test]
fn deck_of_many_things_dies_watcher_stays_on_the_twenty_row() {
    let definition = compile_to_runtime_definition("The Deck of Many Things",
        "Mana cost: {5}\nType: Legendary Artifact\n{2}, {T}: Roll a d20 and subtract the number of cards in your hand. If the result is 0 or less, discard your hand.\n1—9 | Return a card at random from your graveyard to your hand.\n10—19 | Draw two cards.\n20 | Put a creature card from any graveyard onto the battlefield under your control. When that creature dies, its owner loses the game.", false)
        .expect("The Deck of Many Things should compile");
    let top = p78_top_effects(&definition);
    assert!(top.iter().all(|effect| effect
        .downcast_ref::<ironsmith::effects::ScheduleDelayedTriggerEffect>()
        .is_none()), "the watcher is not registered on every activation: {top:?}");
    let row = top.iter().map(|effect| format!("{effect:?}"))
        .find(|debug| debug.contains("Equal(20)"))
        .expect("the 20 row");
    assert!(row.contains("ScheduleDelayedTriggerEffect"), "{row}");
    assert!(row.contains("LoseTheGameEffect"), "{row}");
    assert!(row.contains("target_tag: Some("), "the watcher names the returned creature: {row}");
}

/// The Notary Hobbits: "except the tokens aren't legendary" removes the
/// supertype (CR 707.9b) instead of adding it.
#[test]
fn notary_hobbits_token_copies_are_not_legendary() {
    let definition = compile_to_runtime_definition("The Notary Hobbits",
        "Mana cost: {3}{G}{G}\nType: Legendary Creature — Halfling Advisor\nPower/Toughness: 1/1\nWhen The Notary Hobbits enter, if they're not a token, create two tokens that are copies of them, except the tokens aren't legendary.\n{T}: Add {C} for each Halfling you control.", false)
        .expect("The Notary Hobbits should compile");
    let copy = p78_all_effects(&definition).iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::CreateTokenCopyEffect>().cloned())
        .expect("token copies");
    assert!(copy.removed_supertypes.contains(&ironsmith::types::Supertype::Legendary), "{copy:?}");
    assert!(!copy.added_supertypes.contains(&ironsmith::types::Supertype::Legendary), "{copy:?}");
}

/// The Twelfth Doctor: demonstrate goes only to the first spell cast from
/// anywhere other than your hand each turn.
#[test]
fn twelfth_doctor_grants_demonstrate_only_to_the_first_non_hand_spell() {
    let definition = compile_to_runtime_definition("The Twelfth Doctor",
        "Mana cost: {3}{U}{R}\nType: Legendary Creature — Time Lord Doctor\nPower/Toughness: 4/4\nThe first spell you cast from anywhere other than your hand each turn has demonstrate. (When you cast that spell, you may copy it. If you do, choose an opponent to also copy it. A copy of a permanent spell becomes a token.)\nWhenever you copy a spell, put a +1/+1 counter on The Twelfth Doctor.", false)
        .expect("The Twelfth Doctor should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("GrantSpellKeyword"), "{debug}");
    assert!(debug.contains("first_spell_cast_each_turn: true"), "{debug}");
    assert!(debug.contains("excluded_cast_origin_zone: Some(Hand)"), "{debug}");
}

/// Volo: the Journal token has hexproof, and the draw ability announces its
/// target permanent (the count reads it).
#[test]
fn volo_journal_has_hexproof_and_the_draw_ability_declares_its_target() {
    let definition = compile_to_runtime_definition("Volo, Itinerant Scholar",
        "Mana cost: {2}{U}\nType: Legendary Creature — Human Wizard\nPower/Toughness: 2/3\nWhen Volo enters, create Volo's Journal, a legendary colorless Book artifact token with hexproof and \"Whenever you cast a creature spell, note one of its creature types that hasn't been noted for Volo's Journal.\"\n{2}, {T}: Draw a card for each creature type noted for target permanent you control named Volo's Journal.\nChoose a Background", false)
        .expect("Volo should compile");
    let create = p78_all_effects(&definition).iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::CreateTokenEffect>().cloned())
        .expect("the Journal");
    let token = format!("{:?}", create.token.abilities);
    assert!(token.contains("Hexproof"), "{token}");
    assert!(token.contains("NoteTriggeringCreatureTypeEffect"), "{token}");
    let draw = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(activated) => Some(activated),
        _ => None,
    }).expect("draw ability");
    assert!(!draw.choices.is_empty(), "the Journal is a target: {draw:?}");
    assert!(format!("{:?}", draw.effects).contains("TargetOnlyEffect"), "{draw:?}");
}

/// Ochre Jelly: the delayed token copy enters with half the counters the
/// dead creature had; nothing is put on the dead creature itself.
#[test]
fn ochre_jelly_split_token_enters_with_half_the_counters() {
    let definition = compile_to_runtime_definition("Ochre Jelly",
        "Mana cost: {X}{G}\nType: Creature — Ooze\nPower/Toughness: 0/0\nTrample\nThis creature enters with X +1/+1 counters on it.\nSplit — When this creature dies, if it had two or more +1/+1 counters on it, create a token that's a copy of it at the beginning of the next end step. The token enters with half that many +1/+1 counters on it, rounded down.", false)
        .expect("Ochre Jelly should compile");
    let top = p78_top_effects(&definition);
    assert!(top.iter().all(|effect| effect.downcast_ref::<ironsmith::effects::PutCountersEffect>().is_none()),
        "no immediate counters on the dead creature: {top:?}");
    let copy = p78_all_effects(&definition).iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::CreateTokenCopyEffect>().cloned())
        .expect("delayed token copy");
    assert!(!copy.entry_counters.is_empty(), "{copy:?}");
    assert!(format!("{:?}", copy.entry_counters).contains("HalfRoundedDown"), "{copy:?}");
}

/// Noctis: "It can't be blocked that combat" starts with the delayed return
/// and names the returned permanent.
#[test]
fn noctis_cant_be_blocked_rides_inside_the_delayed_return() {
    let definition = compile_to_runtime_definition("Noctis, Heir Apparent",
        "Mana cost: {W}{U}{B}\nType: Legendary Creature — Human Noble\nPower/Toughness: 2/3\nWhenever a creature you control enters during combat, you may attach target Equipment you control to target creature you control.\nWarp-Strike — {3}: Exile Noctis. Return it to the battlefield under its owner's control tapped and attacking at the beginning of that player's next declare attackers step. It can't be blocked that combat.", false)
        .expect("Noctis should compile");
    let top = p78_top_effects(&definition);
    for effect in &top {
        let debug = format!("{effect:?}");
        if effect.downcast_ref::<ironsmith::effects::ScheduleDelayedTriggerEffect>().is_some() {
            assert!(debug.contains("BeBlocked"), "{debug}");
        } else {
            assert!(!debug.contains("BeBlocked"), "not applied at activation: {debug}");
        }
    }
}

/// "deals combat damage during your turn" qualifies the event with your turn
/// instead of dropping it (Quilled Greatwurm; The Aetherspark's grant shares
/// the reader).
#[test]
fn combat_damage_during_your_turn_qualifies_the_trigger() {
    let definition = compile_to_runtime_definition("Quilled Greatwurm",
        "Mana cost: {4}{G}{G}\nType: Creature — Wurm\nPower/Toughness: 7/7\nTrample\nWhenever a creature you control deals combat damage during your turn, put that many +1/+1 counters on it. (It must survive to get the counters.)\nYou may cast this card from your graveyard by removing six counters from among creatures you control in addition to paying its other costs.", false)
        .expect("Quilled Greatwurm should compile");
    let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => Some(trigger),
        _ => None,
    }).expect("combat damage trigger");
    let debug = format!("{:?}", trigger.trigger);
    assert!(debug.contains("DealsDamage") || debug.contains("CombatDamage"), "{debug}");
    assert!(debug.contains("YourTurn"), "during your turn: {debug}");
}

/// "a modal spell" / "a spell using teamwork" keep their marker on the
/// spell-cast trigger filter.
#[test]
fn modal_and_teamwork_spell_triggers_keep_their_marker() {
    let riku = compile_to_runtime_definition("Riku of Many Paths",
        "Mana cost: {G}{U}{R}\nType: Legendary Creature — Human Wizard\nPower/Toughness: 3/3\nWhenever you cast a modal spell, choose up to X, where X is the number of times you chose a mode for that spell —\n• Exile the top card of your library. Until the end of your next turn, you may play it.\n• Put a +1/+1 counter on Riku. It gains trample until end of turn.\n• Create a 1/1 blue Bird creature token with flying.", false)
        .expect("Riku should compile");
    let debug = format!("{:?}", riku.abilities);
    assert!(debug.contains("SpellCastTrigger"), "{debug}");
    assert!(debug.contains("\"modal\""), "{debug}");
    let assistant = compile_to_runtime_definition("Virtual Assistant",
        "Mana cost: {1}{U}\nType: Artifact Creature — Illusion Advisor\nPower/Toughness: 3/3\nDefender\nWhenever you cast a spell using teamwork, create a 1/1 colorless Robot Hero artifact creature token with flying.", false)
        .expect("Virtual Assistant should compile");
    let debug = format!("{:?}", assistant.abilities);
    assert!(debug.contains("\"teamwork\""), "{debug}");
}

/// Rith: only an opponent's creature or planeswalker dealt excess damage
/// counts.
#[test]
fn rith_excess_damage_history_requires_an_opponents_permanent() {
    let definition = compile_to_runtime_definition("Rith, Liberated Primeval",
        "Mana cost: {2}{R}{G}{W}\nType: Legendary Creature — Dragon\nPower/Toughness: 5/5\nFlying, ward {2}\nOther Dragons you control have ward {2}.\nAt the beginning of your end step, if a creature or planeswalker an opponent controlled was dealt excess damage this turn, create a 4/4 red Dragon creature token with flying.", false)
        .expect("Rith should compile");
    let debug = format!("{:?}", definition.abilities);
    let start = debug.find("DealtExcessDamage").expect("excess damage history");
    let history = &debug[start..(start + 1500).min(debug.len())];
    assert!(history.contains("Opponent"), "{history}");
}

/// Shirei: "if Shirei is still on the battlefield" is checked when the
/// delayed return resolves.
#[test]
fn shirei_checks_its_presence_when_the_delayed_return_resolves() {
    let definition = compile_to_runtime_definition("Shirei, Shizo's Caretaker",
        "Mana cost: {4}{B}\nType: Legendary Creature — Spirit\nPower/Toughness: 2/2\nWhenever a creature with power 1 or less is put into your graveyard from the battlefield, you may return that card to the battlefield at the beginning of the next end step if Shirei is still on the battlefield.", false)
        .expect("Shirei should compile");
    let schedule = p78_all_effects(&definition).iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::ScheduleDelayedTriggerEffect>().cloned())
        .expect("delayed return");
    let body = format!("{:?}", schedule.effects);
    assert!(body.contains("SourceIsInZone"), "checked at the end step: {body}");
}

/// Silkguard: Auras, Equipment and modified creatures, not "Aura creatures".
#[test]
fn silkguard_hexproof_reaches_auras_equipment_and_modified_creatures() {
    let definition = compile_to_runtime_definition("Silkguard",
        "Mana cost: {X}{G}\nType: Instant\nPut a +1/+1 counter on each of up to X target creatures you control.\nAuras, Equipment, and modified creatures you control gain hexproof until end of turn. (Equipment, Auras you control, and counters are modifications.)", false)
        .expect("Silkguard should compile");
    let debug = format!("{:?}", definition.spell_effect);
    let aura = debug.find("subtypes: [Aura]").expect("Aura branch");
    let branch_start = debug[..aura].rfind("ObjectFilter {").expect("Aura branch filter");
    assert!(debug[branch_start..aura].contains("card_types: []"), "Auras aren't creatures: {debug}");
    assert!(debug.contains("subtypes: [Equipment]"), "{debug}");
    assert!(debug.contains("modified: true"), "{debug}");
}

/// Silver Surfer: the per-object "if a land enters this way" check reads the
/// card being returned (still in exile), so only the land enters tapped.
#[test]
fn silver_surfer_delayed_return_taps_only_the_land() {
    use ironsmith::snapshot::ObjectSnapshot;
    let definition = compile_to_runtime_definition("Silver Surfer, Cosmic Voyager",
        "Mana cost: {4}{U}{U}\nType: Legendary Creature — Alien Hero\nPower/Toughness: 5/5\nFlash\nFlying\nWhen Silver Surfer enters, exile any number of other target permanents you control. Return those cards to the battlefield under their owner's control at the beginning of the next end step. If a land enters this way, it enters tapped.", false)
        .expect("Silver Surfer should compile");
    let move_back = p78_all_effects(&definition).iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::MoveToZoneEffect>()
            .filter(|move_back| move_back.enters_tapped_if.is_some()).cloned())
        .expect("delayed return with a per-object tapped entry");
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let land = CardBuilder::new(CardId::new(), "Exiled land").card_types(vec![CardType::Land]).build();
    let creature = CardBuilder::new(CardId::new(), "Exiled creature")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(1, 1)).build();
    let land_id = game.create_object_from_card(&land, alice, Zone::Exile);
    let creature_id = game.create_object_from_card(&creature, alice, Zone::Exile);
    let surfer = CardBuilder::new(CardId::new(), "Surfer")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build();
    let source = game.create_object_from_card(&surfer, alice, Zone::Battlefield);
    let mut effect = move_back.clone();
    effect.target = ChooseSpec::Tagged("p78_returned".into());
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    ctx.set_tagged_objects("p78_returned", vec![
        ObjectSnapshot::from_object_id(&game, land_id).unwrap(),
        ObjectSnapshot::from_object_id(&game, creature_id).unwrap(),
    ]);
    ironsmith::effects::execute_effect(&mut game, &Effect::new(effect), &mut ctx).unwrap();
    let returned: Vec<_> = game.battlefield.iter().copied().filter(|id| *id != source).collect();
    assert_eq!(returned.len(), 2, "both cards return together");
    for id in returned {
        let is_land = game.current_card_types(id).unwrap().contains(&CardType::Land);
        assert_eq!(game.is_tapped(id), is_land, "land tapped, creature untapped");
    }
}

/// Tromokratis: hexproof is a static of the permanent, lost while attacking
/// or blocking (never a spell effect).
#[test]
fn tromokratis_hexproof_is_a_static_unless_attacking_or_blocking() {
    let definition = compile_to_runtime_definition("Tromokratis",
        "Mana cost: {5}{U}{U}\nType: Legendary Creature — Kraken\nPower/Toughness: 8/8\nTromokratis has hexproof unless it's attacking or blocking.\nTromokratis can't be blocked unless all creatures defending player controls block it. (If any creature that player controls doesn't block this creature, it can't be blocked.)", false)
        .expect("Tromokratis should compile");
    assert!(definition.spell_effect.is_none(), "no spell effect: {:?}", definition.spell_effect);
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("Hexproof"), "{debug}");
    assert!(debug.contains("CantBeBlockedUnlessAllDefendingCreaturesBlock"), "{debug}");
}

/// Whippoorwill: damage to the target can't be prevented or redirected this
/// turn — a this-turn prohibition on that creature, not a permanent grant.
#[test]
fn whippoorwill_unpreventable_damage_is_a_this_turn_prohibition_on_the_target() {
    let definition = compile_to_runtime_definition("Whippoorwill",
        "Mana cost: {G}\nType: Creature — Bird\nPower/Toughness: 1/1\n{G}{G}, {T}: Target creature can't be regenerated this turn. Damage that would be dealt to that creature this turn can't be prevented or dealt instead to another permanent or player. When the creature dies this turn, exile the creature.", false)
        .expect("Whippoorwill should compile");
    let debugs: Vec<String> = p78_top_effects(&definition).iter().map(|effect| format!("{effect:?}")).collect();
    assert!(debugs.iter().all(|debug| !(debug.contains("ApplyContinuousEffect") && debug.contains("PreventDamageTo"))),
        "not a granted static: {debugs:?}");
    let prohibition = debugs.iter().find(|debug| debug.contains("PreventDamageTo"))
        .expect("the damage prohibition");
    assert!(prohibition.contains("or_redirected: true"), "{prohibition}");
    assert!(prohibition.contains("EndOfTurn"), "this turn only: {prohibition}");
    assert!(!prohibition.contains("__it__"), "bound to the target: {prohibition}");
}
