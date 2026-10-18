//! Claude package 30 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p30_definition(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"))
}

fn p30_compact(definition: &ironsmith::cards::CardDefinition) -> String {
    format!("{definition:#?}").split_whitespace().collect::<String>()
}

#[test]
fn everything_pizza_each_of_your_opponents_discards() {
    let definition = p30_definition(
        "Everything Pizza",
        "Mana cost: {2}\nType: Artifact — Food\nWhen this artifact enters, search your library for a basic land card, reveal it, put it into your hand, then shuffle.\n{2}{W}{U}{B}{R}{G}, {T}, Sacrifice this artifact: Target player gains 3 life and draws a card. Each of your opponents discards a card. This artifact deals 3 damage to any target. Put three +1/+1 counters on up to one target creature.",
    );
    let compact = p30_compact(&definition);
    let fanout = compact
        .split_once("ForPlayersEffect{filter:Opponent")
        .map(|(_, tail)| tail)
        .unwrap_or_else(|| panic!("each opponent fan-out: {compact}"));
    assert!(fanout.contains("DiscardEffect"), "{compact}");
    assert!(compact.contains("DealDamageEffect") || compact.contains("DamageEffect"), "{compact}");
}

#[test]
fn elvish_healer_green_creature_replaces_the_one_point_shield_with_two() {
    let definition = p30_definition(
        "Elvish Healer",
        "Mana cost: {2}{W}\nType: Creature — Elf Cleric\nPower/Toughness: 1/2\n{T}: Prevent the next 1 damage that would be dealt to any target this turn. If it's a green creature, prevent the next 2 damage instead.",
    );
    let compact = p30_compact(&definition);
    assert!(compact.contains("PreventDamageEffect{amount:Fixed(1"), "{compact}");
    assert!(compact.contains("PreventDamageEffect{amount:Fixed(2"), "{compact}");
    assert!(compact.contains("self_replacements:[SelfReplacement"), "the larger shield replaces the default: {compact}");
    assert!(compact.contains("colors:Some(ColorSet(16"), "the green-creature test: {compact}");
}

#[test]
fn errantry_enchanted_creature_can_only_attack_alone() {
    let definition = p30_definition(
        "Errantry",
        "Mana cost: {1}{R}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature gets +3/+0 and can only attack alone.",
    );
    let compact = p30_compact(&definition);
    assert!(compact.contains("AttackOnlyAlone"), "{compact}");
    assert!(!compact.contains("AttackAlone("), "not the can't-attack-alone restriction: {compact}");
}

#[test]
fn erdwal_illuminator_investigates_an_additional_time() {
    let definition = p30_definition(
        "Erdwal Illuminator",
        "Mana cost: {1}{U}\nType: Creature — Spirit\nPower/Toughness: 1/3\nFlying\nWhenever you investigate for the first time each turn, investigate an additional time.",
    );
    let compact = p30_compact(&definition);
    let investigate = compact
        .split_once("InvestigateEffect{")
        .map(|(_, tail)| tail)
        .unwrap_or_else(|| panic!("investigate effect: {compact}"));
    assert!(investigate.contains("Fixed(1"), "exactly one more investigation: {compact}");
    assert!(compact.contains("Investigate"), "{compact}");
}

#[test]
fn essence_reliquary_returns_the_target_and_your_auras_attached_to_it() {
    let definition = p30_definition(
        "Essence Reliquary",
        "Mana cost: {2}{W}\nType: Artifact\n{T}: Return another target permanent you control and all Auras you control attached to it to their owner's hand. Activate only during your turn.",
    );
    let compact = p30_compact(&definition);
    assert!(compact.matches("ReturnToHandEffect").count() >= 2, "{compact}");
    let auras = compact
        .split_once("spec:All(")
        .map(|(_, tail)| tail)
        .unwrap_or_else(|| panic!("all Auras return: {compact}"));
    assert!(auras.contains("Aura"), "{compact}");
    assert!(auras.contains("controller:Some(You"), "only Auras you control: {compact}");
    assert!(
        auras.contains("AttachedToTaggedObject") || auras.contains("attached_to_object:Some"),
        "only Auras attached to the returned permanent: {compact}"
    );
}

#[test]
fn expand_the_sphere_proliferates_the_land_shortfall() {
    let definition = p30_definition(
        "Expand the Sphere",
        "Mana cost: {3}{G}\nType: Sorcery\nLook at the top six cards of your library. Put up to two land cards from among them onto the battlefield tapped and the rest on the bottom of your library in a random order. If you put fewer than two lands onto the battlefield this way, proliferate a number of times equal to the difference. (Choose any number of permanents and/or players, then give each another counter of each kind already there.)",
    );
    let compact = p30_compact(&definition);
    assert!(compact.contains("LessThan"), "the shortfall gate compares the land count: {compact}");
    assert!(compact.contains("PutOntoBattlefield"), "counts lands actually put this way: {compact}");
    let proliferate = compact
        .split_once("ProliferateEffect{")
        .map(|(_, tail)| tail)
        .unwrap_or_else(|| panic!("proliferate: {compact}"));
    assert!(!proliferate.starts_with("count:Fixed(1)"), "repeats by the difference: {compact}");
    assert!(!compact.contains("PendingComparisonDifference"), "difference must bind: {compact}");
}

#[test]
fn esoteric_duplicator_paid_copy_is_created_at_the_next_end_step() {
    let definition = p30_definition(
        "Esoteric Duplicator",
        "Mana cost: {2}{U}\nType: Artifact — Clue\nWhenever you sacrifice this artifact or another artifact, you may pay {2}. If you do, at the beginning of the next end step, create a token that's a copy of that artifact.\n{2}, Sacrifice this artifact: Draw a card.",
    );
    let compact = p30_compact(&definition);
    let delayed = compact
        .split_once("ScheduleDelayedTriggerEffect")
        .map(|(_, tail)| tail)
        .unwrap_or_else(|| panic!("delayed copy: {compact}"));
    assert!(delayed.contains("EndStepTrigger"), "{compact}");
    assert!(delayed.contains("CreateTokenCopyEffect"), "{compact}");
    assert!(compact.contains("SacrificesTrigger") || compact.contains("Sacrifice"), "{compact}");
    assert!(compact.contains("Generic(2"), "the optional {{2}} payment: {compact}");
}

#[test]
fn etrata_the_silencer_loss_checks_owned_exiled_hit_counter_cards() {
    let definition = p30_definition(
        "Etrata, the Silencer",
        "Mana cost: {2}{U}{B}\nType: Legendary Creature — Vampire Assassin\nPower/Toughness: 3/5\nEtrata can't be blocked.\nWhenever Etrata deals combat damage to a player, exile target creature that player controls and put a hit counter on that card. That player loses the game if they own three or more exiled cards with hit counters on them. Etrata's owner shuffles Etrata into their library.",
    );
    let compact = p30_compact(&definition);
    let gate = compact
        .split_once("PlayerHasAtLeast{")
        .map(|(_, tail)| tail)
        .unwrap_or_else(|| panic!("owned exiled-card count gate: {compact}"));
    assert!(gate.contains("Exile"), "{compact}");
    assert!(gate.contains("Hit"), "{compact}");
    assert!(gate.contains("count:3"), "{compact}");
    assert!(compact.contains("LoseTheGameEffect"), "{compact}");
    assert!(compact.contains("ShuffleIntoLibrary") || compact.contains("Shuffle"), "{compact}");
}

#[test]
fn eventides_shadow_draws_and_loses_life_for_each_removed_counter() {
    let definition = p30_definition(
        "Eventide's Shadow",
        "Mana cost: {6}{B}{B}{B}\nType: Sorcery\nRemove any number of counters from among permanents on the battlefield. You draw cards and lose life equal to the number of counters removed this way.",
    );
    let compact = p30_compact(&definition);
    assert!(compact.contains("RemoveUpToAnyCountersEffect"), "{compact}");
    assert!(compact.contains("Draw"), "{compact}");
    assert!(compact.contains("LoseLife"), "{compact}");
    let draw_tail = compact
        .split_once("LoseLife")
        .map(|(_, tail)| tail)
        .unwrap_or_default();
    assert!(
        !draw_tail.starts_with("Effect{amount:Fixed(1)"),
        "the life loss shares the removed-counter amount: {compact}"
    );
}

#[test]
fn elspeth_conquers_death_taxes_opponents_noncreature_spells_until_your_next_turn() {
    let definition = p30_definition(
        "Elspeth Conquers Death",
        "Mana cost: {3}{W}{W}\nType: Enchantment — Saga\n(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)\nI — Exile target permanent an opponent controls with mana value 3 or greater.\nII — Noncreature spells your opponents cast cost {2} more to cast until your next turn.\nIII — Return target creature or planeswalker card from your graveyard to the battlefield. Put a +1/+1 counter or a loyalty counter on it.",
    );
    let compact = p30_compact(&definition);
    let tax = compact
        .split_once("GrantNextSpellCostReductionEffect{")
        .map(|(_, tail)| tail)
        .unwrap_or_else(|| panic!("temporary spell tax: {compact}"));
    assert!(tax.starts_with("player:Opponent"), "taxes the opponents' spells: {compact}");
    assert!(tax.contains("increases_cost:true"), "{compact}");
    assert!(tax.contains("duration:YourNextTurn"), "{compact}");
    assert!(tax.contains("applies_to_all_matching_this_turn:true"), "{compact}");
    assert!(tax.contains("Creature"), "noncreature restriction kept: {compact}");
}

#[test]
fn evercoat_ursine_has_two_hideaway_instances_and_plays_from_the_linked_exile() {
    let definition = p30_definition(
        "Evercoat Ursine",
        "Mana cost: {5}{G}{G}\nType: Creature — Elemental Bear\nPower/Toughness: 8/8\nTrample\nHideaway 3, hideaway 3 (When this creature enters, look at the top three cards of your library, exile one face down, then put the rest on the bottom in a random order. Then do it again.)\nWhenever this creature deals combat damage to a player, if there are cards exiled with it, you may play one of them without paying its mana cost.",
    );
    let compact = p30_compact(&definition);
    assert_eq!(
        compact.matches("LookAtTopCardsEffect").count(),
        2,
        "one look-and-exile per hideaway instance: {compact}"
    );
    assert_eq!(compact.matches("ZoneChangeTrigger").count(), 2, "{compact}");
    assert!(!compact.contains("KeywordFallbackText"), "{compact}");
    assert!(compact.contains("ThisDealsCombatDamageToPlayerTrigger"), "{compact}");
}

#[test]
fn ethereal_valkyrie_foretells_the_exiled_card_for_its_mana_cost_minus_two() {
    use ironsmith::card::CardBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::{PlayerId, Zone};
    let definition = p30_definition(
        "Ethereal Valkyrie",
        "Mana cost: {4}{W}{U}\nType: Creature — Spirit Angel\nPower/Toughness: 4/4\nFlying\nWhenever this creature enters or attacks, draw a card, then exile a card from your hand face down. It becomes foretold. Its foretell cost is its mana cost reduced by {2}. (On a later turn, you may cast it for its foretell cost, even if this creature has left the battlefield.)",
    );
    let compact = p30_compact(&definition);
    let foretold = compact
        .split_once("BecomeForetoldEffect{")
        .map(|(_, tail)| tail)
        .unwrap_or_else(|| panic!("foretold designation: {compact}"));
    assert!(
        foretold[..foretold.len().min(1500)].contains("foretell_cost_reduction:Some(2"),
        "{compact}"
    );

    // The granted foretell cost is the card's mana cost with two less generic.
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let card = CardBuilder::new(CardId::new(), "Exiled foretell subject")
        .card_types(vec![ironsmith::types::CardType::Sorcery])
        .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![
            ironsmith::ManaSymbol::Generic(3),
            ironsmith::ManaSymbol::White,
        ]))
        .build();
    let exiled = game.create_object_from_card(&card, alice, Zone::Exile);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let effect = Effect::new(
        ironsmith::effects::BecomeForetoldEffect::new(ironsmith::target::ChooseSpec::SpecificObject(exiled))
            .with_foretell_cost_reduction(Some(2)),
    );
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    ironsmith::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
    drop(ctx);
    assert!(game.is_foretold(exiled));
    let cost = game
        .object(exiled)
        .unwrap()
        .alternative_casts
        .iter()
        .find_map(|method| match method {
            ironsmith::alternative_cast::AlternativeCastingMethod::Foretell { cost } => {
                Some(cost.clone())
            }
            _ => None,
        })
        .expect("granted foretell cost");
    assert_eq!(cost.mana_value(), 2, "{{3}}{{W}} reduced by {{2}} is {{1}}{{W}}: {cost:?}");
}

#[test]
fn etrata_deadly_fugitive_grants_face_down_creatures_a_self_face_up_ability() {
    let definition = p30_definition(
        "Etrata, Deadly Fugitive",
        "Mana cost: {U}{B}\nType: Legendary Creature — Vampire Assassin\nPower/Toughness: 2/2\nDeathtouch\nFace-down creatures you control have \"{2}{U}{B}: Turn this creature face up. If you can't, exile it, then you may cast the exiled card without paying its mana cost.\"\nWhenever an Assassin you control deals combat damage to an opponent, cloak the top card of that player's library.",
    );
    let compact = p30_compact(&definition);
    let face_up = compact
        .split_once("TurnFaceUpEffect{")
        .map(|(_, tail)| tail)
        .unwrap_or_else(|| panic!("granted face-up ability: {compact}"));
    assert!(face_up.starts_with("target:Source"), "turns the creature with the ability: {compact}");
    assert!(compact.contains("face_down:Some(true"), "granted to face-down creatures: {compact}");
    assert!(compact.contains("ManifestTopCardOfLibraryEffect") && compact.contains("cloak:true"), "{compact}");
}

#[test]
fn everlasting_torment_makes_all_damage_to_creatures_minus_one_counters() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::ids::CardId;
    use ironsmith::object::CounterType;
    use ironsmith::{PlayerId, Zone};
    let definition = p30_definition(
        "Everlasting Torment",
        "Mana cost: {2}{B/R}\nType: Enchantment\nPlayers can't gain life.\nDamage can't be prevented.\nAll damage is dealt as though its source had wither. (A source with wither deals damage to creatures in the form of -1/-1 counters.)",
    );
    let compact = p30_compact(&definition);
    assert!(compact.contains("AllDamageAsThoughWither"), "{compact}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let bear = CardBuilder::new(CardId::new(), "Wither test bear")
        .card_types(vec![ironsmith::types::CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let creature = game.create_object_from_card(&bear, bob, Zone::Battlefield);
    let source = game.create_object_from_card(&bear, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    let mut ctx = EffectContext::new_default(source, alice);
    execute_effect(
        &mut game,
        &Effect::deal_damage(2, ironsmith::target::ChooseSpec::SpecificObject(creature)),
        &mut ctx,
    )
    .unwrap();
    drop(ctx);
    assert_eq!(game.counter_count(creature, CounterType::MinusOneMinusOne), 2);
    assert_eq!(game.damage_on(creature), 0, "wither damage is not marked");
}

#[test]
fn embiggen_pumps_by_the_targets_supertypes_card_types_and_subtypes() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::ids::CardId;
    use ironsmith::types::{CardType, Subtype, Supertype};
    use ironsmith::{PlayerId, Zone};
    let definition = p30_definition(
        "Embiggen",
        "Mana cost: {G}\nType: Instant\nUntil end of turn, target non-Brushwagg creature gets +1/+1 for each supertype, card type, and subtype it has.",
    );
    let compact = p30_compact(&definition);
    assert!(compact.contains("TypeCountOf("), "{compact}");
    assert!(compact.contains("Brushwagg"), "non-Brushwagg target restriction: {compact}");

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let golem = CardBuilder::new(CardId::new(), "Legendary artifact golem wizard")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Golem, Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let creature = game.create_object_from_card(&golem, alice, Zone::Battlefield);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut ctx = EffectContext::new_default(source, alice)
        .with_targets(vec![ResolvedTarget::Object(creature)]);
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    game.refresh_continuous_state().unwrap();
    // Legendary + artifact + creature + Golem + Wizard = five.
    assert_eq!(game.calculated_power(creature), Some(7));
    assert_eq!(game.calculated_toughness(creature), Some(7));
}

#[test]
fn endless_detour_owner_of_a_serial_target_list_chooses_top_or_bottom() {
    let definition = p30_definition(
        "Endless Detour",
        "Mana cost: {G}{W}{U}\nType: Instant\nThe owner of target spell, nonland permanent, or card in a graveyard puts it on their choice of the top or bottom of their library.",
    );
    let compact = p30_compact(&definition);
    let moved = compact
        .split_once("MoveToLibraryTopOrBottomChoiceEffect{")
        .map(|(_, tail)| tail)
        .unwrap_or_else(|| panic!("owner's top-or-bottom choice: {compact}"));
    assert!(moved.contains("Stack"), "spells are legal targets: {compact}");
    assert!(moved.contains("Graveyard"), "graveyard cards are legal targets: {compact}");
    assert!(moved.contains("top_position:0"), "{compact}");
}

#[test]
fn enlightened_confidant_returns_a_surveilled_card_within_the_life_gained() {
    let definition = p30_definition(
        "Enlightened Confidant",
        "Mana cost: {1}{W}\nType: Creature — Kor Cleric\nPower/Toughness: 2/2\nLifelink\nAt the beginning of your end step, if you gained life this turn, surveil 1. If you put a card with mana value less than or equal to the amount of life you gained this turn into your graveyard this way, put that card into your hand.",
    );
    let compact = p30_compact(&definition);
    assert!(compact.contains("Surveil"), "{compact}");
    assert!(compact.contains("LessThanOrEqualExpr"), "mana value at most the life gained: {compact}");
    assert!(compact.contains("Hand"), "the card returns to hand: {compact}");
}
