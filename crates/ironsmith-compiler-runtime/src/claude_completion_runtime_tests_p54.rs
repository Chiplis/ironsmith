//! Claude package 54 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::ids::CardId;
use ironsmith::types::{CardType, Subtype};
use ironsmith::{PlayerId, Zone};

fn p54_compile(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    let (definition, trace) = ironsmith_compiler::parse_trace::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    definition.unwrap_or_else(|error| panic!("{name}: {error:?}\n{}", trace.render()))
}

/// Island Sanctuary: "instead you may skip that draw. If you do, ..." is an
/// optional replacement of each draw during your own draw step whose
/// replacement program is the follow-up (CR 614.1a), and the follow-up is a
/// player attack restriction that exempts flying and islandwalk creatures.
#[test]
fn island_sanctuary_optional_draw_step_skip_runs_its_attack_restriction() {
    let definition = p54_compile(
        "Island Sanctuary",
        "Mana cost: {1}{W}\nType: Enchantment\nIf you would draw a card during your draw step, instead you may skip that draw. If you do, until your next turn, you can't be attacked except by creatures with flying and/or islandwalk.",
    );
    let debug = format!("{definition:?}");
    assert!(
        definition.abilities.iter().any(|ability| matches!(&ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == ironsmith::static_abilities::StaticAbilityId::ConditionalDrawReplacement)),
        "{debug}"
    );
    assert!(debug.contains("DuringYourDrawStep"), "{debug}");
    assert!(debug.contains("optional: true"), "{debug}");
    assert!(debug.contains("AttackPlayer"), "{debug}");
    assert!(debug.contains("UntilYourNextTurn"), "{debug}");
    assert!(debug.contains("Flying") && debug.contains("Island"), "{debug}");
}

/// Kami of Jealous Thirst: the colored "This ability costs {4}{B} less ... if
/// you've drawn three or more cards this turn" reuses p42's `reduction_mana`
/// fold, so the conditional price is the printed cost minus {4}{B}.
#[test]
fn kami_of_jealous_thirst_colored_conditional_reduction_folds_into_printed_cost() {
    let definition = p54_compile(
        "Kami of Jealous Thirst",
        "Mana cost: {2}{B}\nType: Creature — Spirit\nPower/Toughness: 1/3\nDeathtouch\n{4}{B}: Each opponent loses 2 life and you gain 2 life. This ability costs {4}{B} less to activate if you've drawn three or more cards this turn. Activate only once each turn.",
    );
    let debug = format!("{definition:?}");
    assert!(
        definition.abilities.iter().any(|ability| matches!(ability.kind, AbilityKind::Activated(_))),
        "{debug}"
    );
    assert!(debug.contains("ActivatedAbilityCostReduction"), "{debug}");
    assert!(debug.contains("replacement_mana_cost: Some("), "{debug}");
    assert!(debug.contains("reduction_mana: None"), "{debug}");
    assert!(debug.contains("ThisAbility"), "{debug}");
    assert!(debug.contains("MaxActivationsPerTurn(1)") || debug.contains("OncePerTurn"), "{debug}");
}

/// Kaya, Spirits' Justice: "choose a creature card from among them" names the
/// exiled triggering group, so the choice is a zone-less tagged pool rather
/// than the chooser's hand; the -2 loops over the other players.
#[test]
fn kaya_spirits_justice_chooses_from_the_exiled_group_not_the_hand() {
    let definition = p54_compile(
        "Kaya, Spirits' Justice",
        "Mana cost: {2}{W}{B}\nType: Legendary Planeswalker — Kaya\nLoyalty: 3\nWhenever one or more creatures you control and/or creature cards in your graveyard are put into exile, you may choose a creature card from among them. Until end of turn, target token you control becomes a copy of it, except it has flying.\n+2: Surveil 2, then exile a card from a graveyard.\n+1: Create a 1/1 white and black Spirit creature token with flying.\n−2: Exile target creature you control. For each other player, exile up to one target creature that player controls.",
    );
    let debug = format!("{definition:?}");
    let start = debug.find("ChooseObjectsEffect").unwrap_or_else(|| panic!("{debug}"));
    let choose = &debug[start..];
    let end = choose.find("additional_zones").unwrap_or(choose.len());
    let choose = &choose[..end];
    assert!(!choose.contains("Hand"), "{choose}");
    assert!(choose.contains("IsTaggedObject"), "{choose}");
}

/// Ixhel: the conditional per-opponent exile accumulates one union across the
/// opponent loop, and the trailing permission names that union.
#[test]
fn ixhel_conditional_opponent_exile_exports_its_collection_to_the_permission() {
    let definition = p54_compile(
        "Ixhel, Scion of Atraxa",
        "Mana cost: {1}{W}{B}{G}\nType: Legendary Creature — Phyrexian Angel\nPower/Toughness: 3/5\nFlying, vigilance, toxic 2\nCorrupted — At the beginning of your end step, each opponent who has three or more poison counters exiles the top card of their library face down. You may look at and play those cards for as long as they remain exiled, and you may spend mana as though it were mana of any color to cast those spells.",
    );
    let debug = format!("{definition:?}");
    let exile = debug.find("ExileTopOfLibraryEffect").unwrap_or_else(|| panic!("{debug}"));
    let exile = &debug[exile..];
    let accumulated = exile.find("accumulated_tags: [").expect("accumulated tag list");
    let tag_start = accumulated + "accumulated_tags: [".len();
    let tag_text = &exile[tag_start..tag_start + exile[tag_start..].find(']').unwrap()];
    assert!(tag_text.contains("exiled"), "{exile}");
    assert!(exile[..accumulated].contains("moved_tags: []"), "{exile}");
    assert!(exile.contains("face_down: true"), "{exile}");
    let tag = tag_text.trim().trim_start_matches("TagKey(").trim_end_matches(')').trim();
    assert!(debug.matches(tag).count() >= 2, "permission names the union {tag}: {debug}");
}

/// Inquisitive Glimmer: "Unlock costs you pay cost {1} less." is a
/// special-action cost modifier (CR 709.5c) read by the unlock payment.
#[test]
fn inquisitive_glimmer_reduces_unlock_costs_you_pay() {
    let definition = p54_compile(
        "Inquisitive Glimmer",
        "Mana cost: {W}{U}\nType: Enchantment Creature — Fox Glimmer\nPower/Toughness: 2/3\nEnchantment spells you cast cost {1} less to cast.\nUnlock costs you pay cost {1} less.",
    );
    assert!(
        definition.abilities.iter().any(|ability| matches!(&ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.compiled_model().is_some_and(|model| matches!(
                    model.payload,
                    ironsmith_core::StaticAbilityPayload::UnlockCostReduction { generic_reduction: 1 }
                )))),
        "{:?}",
        definition.abilities
    );
}

/// Karn's Sylex: players can't pay life to cast spells or to activate
/// non-mana abilities, but mana abilities stay payable with life (CR 605).
#[test]
fn karns_sylex_bans_life_payments_except_for_mana_abilities() {
    use ironsmith::costs::PaymentReason;
    let definition = p54_compile(
        "Karn's Sylex",
        "Mana cost: {3}\nType: Legendary Artifact\nKarn's Sylex enters tapped.\nPlayers can't pay life to cast spells or to activate abilities that aren't mana abilities.\n{X}, {T}, Exile Karn's Sylex: Destroy each nonland permanent with mana value X or less. Activate only as a sorcery.",
    );
    let debug = format!("{definition:?}");
    assert!(debug.contains("exempt_mana_abilities: true"), "{debug}");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    for player in [alice, bob] {
        assert!(!game.can_pay_life_with_reason(player, 2, PaymentReason::CastSpell));
        assert!(!game.can_pay_life_with_reason(player, 2, PaymentReason::ActivateAbility));
        assert!(game.can_pay_life_with_reason(player, 2, PaymentReason::ActivateManaAbility));
        assert!(game.can_pay_life_with_reason(player, 2, PaymentReason::Effect));
    }
}

/// Irencrag Feat: "You can cast only one more spell this turn" is a cast
/// limit counted from resolution, so Irencrag Feat itself does not use up
/// the one more spell.
#[test]
fn irencrag_feat_allows_exactly_one_more_spell_after_it_resolves() {
    let definition = p54_compile(
        "Irencrag Feat",
        "Mana cost: {1}{R}{R}{R}\nType: Sorcery\nAdd seven {R}. You can cast only one more spell this turn.",
    );
    let debug = format!("{definition:?}");
    assert!(debug.contains("CastMoreThanNSpellsEachTurn"), "{debug}");
    assert!(debug.contains("beyond_spells_already_cast: true"), "{debug}");
    assert!(debug.contains("maximum: 1"), "{debug}");
}

/// Kaya, Geist Hunter: the -2 registers a resolved, until-end-of-turn token
/// doubling replacement (the temporary form of Parallel Lives).
#[test]
fn kaya_geist_hunter_registers_until_end_of_turn_token_doubling() {
    let definition = p54_compile(
        "Kaya, Geist Hunter",
        "Mana cost: {1}{W}{B}\nType: Legendary Planeswalker — Kaya\nLoyalty: 3\n+1: Creatures you control gain deathtouch until end of turn. Put a +1/+1 counter on up to one target creature token you control.\n−2: Until end of turn, if one or more tokens would be created under your control, twice that many of those tokens are created instead.\n−6: Exile all cards from all graveyards, then create a 1/1 white Spirit creature token with flying for each card exiled this way.",
    );
    let register = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .flat_map(|activated| activated.effects.all_effects())
        .find_map(|effect| {
            effect.downcast_ref::<ironsmith::effects::RegisterTokenCreationMultiplierEffect>()
        })
        .cloned()
        .unwrap_or_else(|| panic!("{:?}", definition.abilities));
    assert_eq!(register.factor, 2);
    assert_eq!(register.controller, ironsmith::target::PlayerFilter::You);
    assert_eq!(register.mode, ironsmith::effects::ReplacementApplyMode::UntilEndOfTurn);
}

/// Kang the Conqueror: "During that turn, power-up abilities can't be
/// activated" binds a keyword-scoped activation ban to the extra turn the
/// same ability created.
#[test]
fn kang_the_conqueror_bans_power_up_activations_during_the_extra_turn() {
    let definition = p54_compile(
        "Kang the Conqueror",
        "Mana cost: {2}{U}{U}\nType: Legendary Creature — Human Villain\nPower/Toughness: 4/4\nFlying\nPower-up — {5}{U}{U}{U}: Put a +1/+1 counter on Kang. Take an extra turn after this one. During that turn, power-up abilities can't be activated. (Activate each power-up ability only once. Reduce the cost by his mana cost if he entered this turn.)",
    );
    let cant = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .flat_map(|activated| activated.effects.all_effects())
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::CantEffect>())
        .cloned()
        .unwrap_or_else(|| panic!("{:?}", definition.abilities));
    assert_eq!(cant.start, ironsmith::effect::RestrictionStart::CreatedExtraTurn);
    assert!(matches!(
        cant.restriction,
        ironsmith_core::Restriction::ActivateKeywordAbilitiesOf {
            keyword: ironsmith_core::ActivatedAbilityKeyword::PowerUp,
            ..
        }
    ), "{cant:?}");
}
