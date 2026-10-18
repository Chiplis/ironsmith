//! Claude package 75 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/claude2/BRIEF.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

/// Every effect reachable from a definition (spell, activated, triggered),
/// including nested children such as reflexive and loop bodies.
fn p75_effects(definition: &ironsmith::cards::CardDefinition) -> Vec<Effect> {
    fn walk(effect: &Effect, out: &mut Vec<Effect>) {
        out.push(effect.clone());
        effect.visit_child_effects(&mut |child| walk(child, out));
    }
    let mut out = Vec::new();
    if let Some(program) = definition.spell_effect.as_ref() {
        for effect in program.all_effects() {
            walk(effect, &mut out);
        }
    }
    for ability in &definition.abilities {
        match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated) => {
                for effect in activated.effects.all_effects() {
                    walk(effect, &mut out);
                }
            }
            ironsmith::ability::AbilityKind::Triggered(triggered) => {
                for effect in triggered.effects.all_effects() {
                    walk(effect, &mut out);
                }
            }
            _ => {}
        }
    }
    out
}

const BABA_LYSAGA: &str = "Mana cost: {1}{B}{G}\nType: Legendary Creature — Human Warlock\nPower/Toughness: 3/3\n{T}, Sacrifice up to three permanents: If there were three or more card types among the sacrificed permanents, each opponent loses 3 life, you gain 3 life, and you draw three cards.";

/// The card-type gate reads the permanents actually sacrificed as the
/// activation cost, not an effect tag a cost payment never publishes.
#[test]
fn p75_baba_lysaga_card_types_read_the_sacrifice_cost_result() {
    let definition = compile_to_runtime_definition("Baba Lysaga, Night Witch", BABA_LYSAGA, false)
        .expect("Baba Lysaga should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("CardTypesAmong"), "{debug}");
    assert!(
        debug.contains("__original_sacrifice_cost_0"),
        "the gate must name the sacrifice-cost result: {debug}"
    );
    assert!(
        !debug.contains("TagKey(\"sacrificed_0\")"),
        "no cost payment publishes sacrificed_0: {debug}"
    );
}

const CITY_IN_A_BOTTLE: &str = "Mana cost: {2}\nType: Artifact\nWhenever one or more other nontoken permanents with a name originally printed in the Arabian Nights expansion are on the battlefield, their controllers sacrifice them.\nPlayers can't cast spells or play lands with a name originally printed in the Arabian Nights expansion.";

/// The state trigger and its sacrifice only cover Arabian Nights names; they
/// must never sweep every other nontoken permanent.
#[test]
fn p75_city_in_a_bottle_state_trigger_keeps_the_original_printing_set() {
    let definition = compile_to_runtime_definition("City in a Bottle", CITY_IN_A_BOTTLE, false)
        .expect("City in a Bottle should compile");
    let triggered = definition
        .abilities
        .iter()
        .find(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Triggered(_)))
        .expect("the state trigger");
    let debug = format!("{triggered:?}");
    // The state condition, the sacrificed set and its count all carry it.
    assert!(
        debug.matches("name_originally_printed_in_set: Some(\"Arabian Nights\")").count() >= 2,
        "{debug}"
    );
    assert!(
        !debug.contains("name_originally_printed_in_set: None"),
        "no Arabian Nights filter may lose the set: {debug}"
    );
}

const BEHEMOTH_OF_VAULT_0: &str = "Mana cost: {6}\nType: Artifact Creature — Robot\nPower/Toughness: 6/6\nTrample\nWhen this creature enters, you get {E}{E}{E}{E} (four energy counters).\nWhen this creature dies, you may pay an amount of {E} equal to target nonland permanent's mana value. When you do, destroy that permanent.";

/// "destroy that permanent" names the targeted nonland permanent, not the
/// Behemoth whose death triggered the ability.
#[test]
fn p75_behemoth_destroys_the_energy_amount_target() {
    let definition = compile_to_runtime_definition("Behemoth of Vault 0", BEHEMOTH_OF_VAULT_0, false)
        .expect("Behemoth should compile");
    let destroy = p75_effects(&definition)
        .into_iter()
        .map(|effect| format!("{effect:?}"))
        .find(|debug| debug.starts_with("Effect(DestroyEffect"))
        .expect("the reflexive destroy");
    assert!(destroy.contains("targeted_"), "{destroy}");
    assert!(!destroy.contains("\"triggering\""), "{destroy}");
}

const ASMORANOMARDICADAISTINACULDACAR: &str = "Type: Legendary Creature — Human Wizard\nPower/Toughness: 3/3\nAs long as you've discarded a card this turn, you may pay {B/R} to cast this spell.\nWhen Asmoranomardicadaistinaculdacar enters, you may search your library for a card named The Underworld Cookbook, reveal it, put it into your hand, then shuffle.\nSacrifice two Foods: Target creature deals 6 damage to itself.";

/// "Target creature deals 6 damage to itself": the targeted creature is both
/// the source and the recipient; the ability's own source is not dealt damage.
#[test]
fn p75_target_creature_deals_damage_to_itself() {
    let definition = compile_to_runtime_definition(
        "Asmoranomardicadaistinaculdacar",
        ASMORANOMARDICADAISTINACULDACAR,
        false,
    )
    .expect("Asmoranomardicadaistinaculdacar should compile");
    let activated = definition
        .abilities
        .iter()
        .find(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
        .expect("the Food ability");
    let debug = format!("{activated:?}");
    assert!(debug.contains("DealDamageEffect"), "{debug}");
    assert!(
        !debug.contains("damage_recipient_source"),
        "the recipient is the targeted creature, not this creature: {debug}"
    );
}

const ASHLING_THE_LIMITLESS: &str = "Mana cost: {2}{R}\nType: Legendary Creature — Elemental Sorcerer\nPower/Toughness: 3/3\nElemental permanent spells you cast from your hand gain evoke {4} as you cast them. (If you cast a spell for its evoke cost, it's sacrificed when it enters.)\nWhenever you sacrifice a nontoken Elemental, create a token that's a copy of it. The token gains haste until end of turn. At the beginning of your next end step, sacrifice it unless you pay {W}{U}{B}{R}{G}.";

/// The granted evoke rider sacrifices the evoked permanent itself, never a
/// permanent of the controller's choice.
#[test]
fn p75_ashling_evoke_rider_sacrifices_the_evoked_permanent() {
    let definition = compile_to_runtime_definition("Ashling, the Limitless", ASHLING_THE_LIMITLESS, false)
        .expect("Ashling should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("Evoke"), "{debug}");
    assert!(debug.contains("permanent_this_way_grants"), "{debug}");
    assert!(
        !debug.contains("sacrificed_0"),
        "the rider must not choose an arbitrary permanent to sacrifice: {debug}"
    );
}

const DANCE_WITH_CALAMITY: &str = "Mana cost: {7}{R}\nType: Sorcery\nShuffle your library. As many times as you choose, you may exile the top card of your library. If the total mana value of the cards exiled this way is 13 or less, you may cast any number of spells from among those cards without paying their mana costs.";

/// Each chosen exiled card is cast in turn: the loop body casts the iterated
/// card, not the first card of the accumulated exile group.
#[test]
fn p75_dance_with_calamity_casts_each_chosen_card() {
    let definition = compile_to_runtime_definition("Dance with Calamity", DANCE_WITH_CALAMITY, false)
        .expect("Dance with Calamity should compile");
    let casts = p75_effects(&definition)
        .into_iter()
        .filter_map(|effect| effect.downcast_ref::<ironsmith::effects::CastTaggedEffect>().cloned())
        .collect::<Vec<_>>();
    assert!(!casts.is_empty(), "a cast of the chosen cards");
    for cast in &casts {
        assert_ne!(cast.tag.as_str(), "exiled_as_many_times_as_chosen", "{cast:?}");
        assert!(cast.without_paying_mana_cost, "{cast:?}");
    }
    let debug = format!("{definition:?}");
    assert!(debug.contains("exiled_as_many_times_as_chosen"), "the group still gates and bounds the choice");
}

const ARCHANGEL_OF_STRIFE: &str = "Mana cost: {5}{W}{W}\nType: Creature — Angel\nPower/Toughness: 6/6\nFlying\nAs this creature enters, each player chooses war or peace.\nCreatures controlled by players who chose war get +3/+0.\nCreatures controlled by players who chose peace get +0/+3.";

/// Each anthem applies only to creatures whose controller chose its option.
#[test]
fn p75_archangel_of_strife_anthems_follow_each_players_choice() {
    let definition = compile_to_runtime_definition("Archangel of Strife", ARCHANGEL_OF_STRIFE, false)
        .expect("Archangel of Strife should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("each_player: true"), "{debug}");
    assert!(debug.contains("__player_option_choice__:war"), "{debug}");
    assert!(debug.contains("__player_option_choice__:peace"), "{debug}");
}

/// The rendered Archangel anthems name the choosing players, never an
/// internal tag or "that player".
#[test]
fn p75_archangel_of_strife_anthems_render_the_choosing_players() {
    let definition = compile_to_runtime_definition("Archangel of Strife", ARCHANGEL_OF_STRIFE, false)
        .expect("Archangel of Strife should compile");
    let text = ironsmith_text::compiled_text_lines(&definition).join("\n").to_ascii_lowercase();
    assert!(!text.contains("__player_option_choice__"), "{text}");
    assert!(text.contains("players who chose war"), "{text}");
    assert!(text.contains("players who chose peace"), "{text}");
}

const CHAOTIC_TRANSFORMATION: &str = "Mana cost: {5}{R}\nType: Sorcery\nExile up to one target artifact, up to one target creature, up to one target enchantment, up to one target planeswalker, and/or up to one target land. For each permanent exiled this way, its controller reveals cards from the top of their library until they reveal a card that shares a card type with it, puts that card onto the battlefield, then shuffles.";

/// Five independent "up to one target" slots, one per listed type.
#[test]
fn p75_chaotic_transformation_has_five_independent_target_slots() {
    let definition = compile_to_runtime_definition("Chaotic Transformation", CHAOTIC_TRANSFORMATION, false)
        .expect("Chaotic Transformation should compile");
    let exiles = p75_effects(&definition)
        .into_iter()
        .map(|effect| format!("{effect:?}"))
        .filter(|debug| debug.starts_with("Effect(ExileEffect"))
        .collect::<Vec<_>>();
    assert_eq!(exiles.len(), 5, "{exiles:#?}");
    for exile in &exiles {
        let types = ["Artifact", "Creature", "Enchantment", "Planeswalker", "Land"]
            .iter()
            .filter(|ty| exile.contains(&format!("card_types: [{ty}]")))
            .count();
        assert_eq!(types, 1, "each slot names exactly one type: {exile}");
    }
}

const BREECHES_THE_BLASTMAKER: &str = "Mana cost: {1}{U}{R}\nType: Legendary Creature — Goblin Pirate\nPower/Toughness: 3/3\nMenace\nWhenever you cast your second spell each turn, you may sacrifice an artifact. If you do, flip a coin. When you win the flip, copy that spell. You may choose new targets for the copy. When you lose the flip, Breeches deals damage equal to that spell's mana value to any target.";

/// The flip happens only if an artifact was sacrificed; each flip result is a
/// reflexive trigger of that flip, and the losing branch reads the original
/// spell's mana value.
#[test]
fn p75_breeches_flip_results_bind_to_the_flip() {
    let definition = compile_to_runtime_definition("Breeches, the Blastmaker", BREECHES_THE_BLASTMAKER, false)
        .expect("Breeches should compile");
    let effects = p75_effects(&definition);
    let debugs = effects.iter().map(|effect| format!("{effect:?}")).collect::<Vec<_>>();
    // The flip sits inside the sacrifice's "if you do" branch.
    let gate = debugs
        .iter()
        .find(|debug| debug.starts_with("Effect(IfEffect") && debug.contains("FlipCoinEffect"))
        .expect("the flip is gated on the sacrifice");
    assert!(gate.contains("ReflexiveTriggerEffect"), "flip results live in the branch: {gate}");
    // Every reflexive is conditioned on the flip, never on the sacrifice.
    let flip_id = debugs
        .iter()
        .find_map(|debug| {
            let rest = debug.strip_prefix("Effect(WithIdEffect { id: EffectId(")?;
            let (id, tail) = rest.split_once(')')?;
            tail.starts_with(", effect: Effect(FlipCoinEffect").then(|| id.to_string())
        })
        .expect("the flip exports its result");
    let reflexives = debugs
        .iter()
        .filter(|debug| debug.starts_with("Effect(ReflexiveTriggerEffect"))
        .collect::<Vec<_>>();
    assert_eq!(reflexives.len(), 2, "{reflexives:#?}");
    for reflexive in &reflexives {
        assert!(
            reflexive.contains(&format!("condition: EffectId({flip_id})")),
            "{reflexive}"
        );
    }
    let damage = debugs
        .iter()
        .find(|debug| debug.starts_with("Effect(DealDamageEffect"))
        .expect("the losing branch's damage");
    assert!(!damage.contains("__copied_stack_object__"), "{damage}");
}

const BARON_HELMUT_ZEMO: &str = "Mana cost: {B}{B}{B}\nType: Legendary Creature — Human Noble Villain\nPower/Toughness: 3/3\nWhenever you cast a black spell from your hand, Baron Helmut Zemo connives.\nBoast — Exile any number of black cards from your graveyard with fifteen or more black mana symbols among their mana costs: Copy those exiled cards. You may cast up to three of the copies without paying their mana costs. (Activate only if this creature attacked this turn and only once each turn.)";

/// The boast cost requires the exiled set to total fifteen black symbols.
#[test]
fn p75_baron_zemo_boast_cost_needs_fifteen_black_symbols() {
    let definition = compile_to_runtime_definition("Baron Helmut Zemo", BARON_HELMUT_ZEMO, false)
        .expect("Baron Helmut Zemo should compile");
    let activated = definition
        .abilities
        .iter()
        .find(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
        .expect("the boast ability");
    let debug = format!("{activated:?}");
    assert!(debug.contains("ManaSymbolsOfColor(Black)"), "{debug}");
    assert!(debug.contains("minimum: Some(Fixed(15))"), "{debug}");

    let mut cost = ironsmith::mana::ManaCost::from_pips(vec![
        vec![ironsmith::mana::ManaSymbol::Black],
        vec![ironsmith::mana::ManaSymbol::Black, ironsmith::mana::ManaSymbol::Red],
        vec![ironsmith::mana::ManaSymbol::Generic(2)],
    ]);
    assert_eq!(
        ironsmith::targeting::mana_symbols_of_color(&cost, ironsmith::color::Color::Black),
        2
    );
    cost = ironsmith::mana::ManaCost::from_pips(vec![vec![ironsmith::mana::ManaSymbol::Red]]);
    assert_eq!(
        ironsmith::targeting::mana_symbols_of_color(&cost, ironsmith::color::Color::Black),
        0
    );
}
