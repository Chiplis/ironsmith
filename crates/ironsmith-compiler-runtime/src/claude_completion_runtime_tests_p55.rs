//! Claude package 55 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::PlayerId;
use ironsmith::Zone;
use ironsmith::game_state::GameState;

/// Every effect reachable from a definition (spell, activated, triggered),
/// including nested children.
fn p55_effects(definition: &ironsmith::cards::CardDefinition) -> Vec<Effect> {
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

fn p55_static_ids(
    definition: &ironsmith::cards::CardDefinition,
) -> Vec<ironsmith::static_abilities::StaticAbilityId> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect()
}

const AGENT_OF_ACQUISITIONS: &str = "Mana cost: {2}\nType: Artifact Creature — Construct\nPower/Toughness: 2/2\nDraft this card face up.\nInstead of drafting a card from a booster pack, you may draft each card in that booster pack, one at a time. If you do, turn this card face down and you can't draft cards for the rest of this draft round. (You may look at booster packs passed to you.)";
const ARCHDEMON_OF_PALIANO: &str = "Mana cost: {2}{B}{B}\nType: Creature — Demon\nPower/Toughness: 5/4\nDraft this card face up.\nAs long as this card is face up during the draft, you can't look at booster packs and must draft cards at random. After you draft three cards this way, turn this card face down. (You may look at cards as you draft them.)\nFlying";

/// Draft-only pick rules have no in-game effect (CR 905): both lines are kept
/// as draft rule text the Conspiracy draft reads.
#[test]
fn p55_draft_only_pick_rules_compile_as_draft_rule_text() {
    use ironsmith::static_abilities::StaticAbilityId;
    for (name, text) in [
        ("Agent of Acquisitions", AGENT_OF_ACQUISITIONS),
        ("Archdemon of Paliano", ARCHDEMON_OF_PALIANO),
    ] {
        let definition = compile_to_runtime_definition(name, text, false)
            .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"));
        let ids = p55_static_ids(&definition);
        assert_eq!(
            ids.iter().filter(|id| **id == StaticAbilityId::DraftRuleText).count(),
            2,
            "{name}: {ids:?}"
        );
    }
}

const ARCANE_SAVANT: &str = "Mana cost: {3}{U}{U}\nType: Creature — Human Wizard\nPower/Toughness: 3/4\nBefore you shuffle your deck to start the game, you may reveal this card from your deck and exile an instant or sorcery card you drafted that isn't in your deck.\nWhen this creature enters, copy a card you exiled with cards named Arcane Savant. You may cast the copy without paying its mana cost.";

/// Arcane Savant exiles one drafted instant or sorcery before the opening
/// shuffle; its ETB chooses one card exiled with cards named Arcane Savant
/// and offers a free cast of its copy.
#[test]
fn p55_arcane_savant_exiles_a_drafted_spell_and_casts_a_copy() {
    use ironsmith::types::CardType;
    let definition = compile_to_runtime_definition("Arcane Savant", ARCANE_SAVANT, false)
        .expect("Arcane Savant should compile");
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
    assert_eq!(spec.card_types, vec![CardType::Instant, CardType::Sorcery]);
    assert_eq!((spec.minimum, spec.maximum), (1, Some(1)));
    let choose = p55_effects(&definition)
        .into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>().cloned())
        .expect("the choice of exiled card");
    assert_eq!(choose.filter.zone, Some(Zone::Exile));
    assert_eq!(
        choose.filter.exiled_with_cards_named.as_deref().map(str::to_ascii_lowercase).as_deref(),
        Some("arcane savant")
    );
    let debug = format!("{definition:?}");
    assert!(debug.contains("without_paying_mana_cost: true") || debug.contains("WithoutPaying"), "{debug}");
}

const ANRAKYR_THE_TRAVELLER: &str = "Mana cost: {4}{B}\nType: Legendary Artifact Creature — Necron\nPower/Toughness: 4/4\nLord of the Pyrrhian Legions — Whenever Anrakyr the Traveller attacks, you may cast an artifact spell from your hand or graveyard by paying life equal to its mana value rather than paying its mana cost.";

/// One artifact card chosen from the hand/graveyard pool is cast with a life
/// payment replacing its mana cost (CR 118.9).
#[test]
fn p55_anrakyr_casts_an_artifact_from_hand_or_graveyard_for_life() {
    let definition =
        compile_to_runtime_definition("Anrakyr the Traveller", ANRAKYR_THE_TRAVELLER, false)
            .expect("Anrakyr should compile");
    let effects = p55_effects(&definition);
    let choose = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>())
        .expect("the choice of card to cast");
    let zones: Vec<_> = choose.zone.into_iter().chain(choose.additional_zones.iter().copied()).chain(choose.filter.any_of.iter().filter_map(|branch| branch.zone)).collect();
    assert!(zones.contains(&Zone::Hand) && zones.contains(&Zone::Graveyard), "{choose:?}");
    let cast = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::CastTaggedEffect>())
        .expect("the cast of the chosen card");
    assert!(!cast.without_paying_mana_cost);
    assert!(cast.alternative_cost.is_some(), "{cast:?}");
}

const ANTAGONISM: &str = "Mana cost: {3}{R}\nType: Enchantment\nAt the beginning of each player's end step, this enchantment deals 2 damage to that player unless one of their opponents was dealt damage this turn.";

/// The damage is skipped when any opponent of the active player was dealt
/// damage this turn.
#[test]
fn p55_antagonism_damage_is_gated_on_the_active_players_opponents() {
    let definition = compile_to_runtime_definition("Antagonism", ANTAGONISM, false)
        .expect("Antagonism should compile");
    let debug = format!("{definition:?}");
    assert!(
        debug.contains("DamageDealtToPlayersThisTurn(OpponentOf(Active))"),
        "{debug}"
    );
    assert!(debug.contains("DealDamage"), "{debug}");
}

const AVENGE: &str = "Mana cost: {4}{W}{W}\nType: Sorcery\nThis spell costs {2} less to cast if a player attacked you during their last turn.\nDestroy all creatures. You gain 1 life for each creature destroyed this way.";

/// The discount reads each other player's own most recent turn.
#[test]
fn p55_avenge_costs_less_after_a_player_attacked_you_during_their_last_turn() {
    let definition = compile_to_runtime_definition("Avenge", AVENGE, false)
        .expect("Avenge should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("OpponentAttackedControllerDuringTheirLastTurn"), "{debug}");
    assert!(definition.spell_effect.is_some());
}

const ARCADE_GANNON: &str = "Mana cost: {2}{W}{U}\nType: Legendary Creature — Human Doctor\nPower/Toughness: 1/3\n{T}: Draw a card, then discard a card. Put a quest counter on Arcade Gannon.\nFor Auld Lang Syne — Once during each of your turns, you may cast an artifact or Human spell from your graveyard with mana value less than or equal to the number of quest counters on Arcade Gannon.";

/// The once-per-turn graveyard permission carries a dynamic mana-value cap
/// read from the source's quest counters.
#[test]
fn p55_arcade_gannon_graveyard_cast_is_capped_by_quest_counters() {
    let definition = compile_to_runtime_definition("Arcade Gannon", ARCADE_GANNON, false)
        .expect("Arcade Gannon should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("OnceDuringEachOfYourTurns"), "{debug}");
    assert!(debug.contains("LessThanOrEqualExpr"), "{debug}");
    assert!(debug.contains("Quest"), "{debug}");
    assert!(debug.contains("Human"), "{debug}");
}

const ASHNOD_THE_UNCARING: &str = "Mana cost: {2}{U}{B}{R}\nType: Legendary Creature — Human Artificer\nPower/Toughness: 3/3\nDeathtouch\nWhenever you activate an ability of an artifact or creature that isn't a mana ability, if one or more permanents were sacrificed to activate it, you may copy that ability. You may choose new targets for the copy. (Sacrificing an artifact for mana to activate an ability doesn't count.)";

/// The intervening if reads the sacrifices recorded on the activation event.
#[test]
fn p55_ashnod_copies_only_activations_paid_with_a_sacrifice() {
    let definition = compile_to_runtime_definition("Ashnod the Uncaring", ASHNOD_THE_UNCARING, false)
        .expect("Ashnod should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("PermanentsSacrificedToActivate"), "{debug}");
    let activation = ironsmith::events::AbilityActivatedEvent::new(
        ironsmith::ids::ObjectId::from_raw(1),
        PlayerId::from_index(0),
        false,
    )
    .with_permanents_sacrificed_to_activate(2);
    assert_eq!(activation.permanents_sacrificed_to_activate, 2);
}

const ASMORANOMARDICADAISTINACULDACAR: &str = "Type: Legendary Creature — Human Wizard\nPower/Toughness: 3/3\nAs long as you've discarded a card this turn, you may pay {B/R} to cast this spell.\nWhen Asmoranomardicadaistinaculdacar enters, you may search your library for a card named The Underworld Cookbook, reveal it, put it into your hand, then shuffle.\nSacrifice two Foods: Target creature deals 6 damage to itself.";

/// "you may pay <cost> to cast this spell" under an "as long as" condition is
/// a conditional alternative cost (CR 118.9).
#[test]
fn p55_asmoranomardicadaistinaculdacar_has_a_conditional_alternative_cost() {
    let definition = compile_to_runtime_definition(
        "Asmoranomardicadaistinaculdacar",
        ASMORANOMARDICADAISTINACULDACAR,
        false,
    )
    .expect("Asmoranomardicadaistinaculdacar should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("Parsed alternative cost"), "{debug}");
    assert!(debug.to_ascii_lowercase().contains("discard"), "{debug}");
}

const ATEMSIS_ALL_SEEING: &str = "Mana cost: {3}{U}{U}{U}\nType: Legendary Creature — Sphinx\nPower/Toughness: 4/5\nFlying\n{2}{U}, {T}: Draw two cards, then discard a card.\nWhenever Atemsis deals damage to an opponent, you may reveal your hand. If cards with at least six different mana values are revealed this way, that player loses the game.";

/// The loss is gated on distinct mana values among the revealed cards.
#[test]
fn p55_atemsis_counts_distinct_mana_values_revealed_this_way() {
    let definition = compile_to_runtime_definition("Atemsis, All-Seeing", ATEMSIS_ALL_SEEING, false)
        .expect("Atemsis should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("DistinctManaValues"), "{debug}");
    assert!(debug.contains("LoseTheGame"), "{debug}");
}

const AZOG_MORIAS_RUIN: &str = "Mana cost: {2}{B}\nType: Legendary Creature — Goblin Soldier\nPower/Toughness: 3/2\nWhen Azog enters, destroy up to one other target creature. Its controller amasses Goblins X, where X is that creature's power. If you controlled that creature, draw a card. (To amass Goblins X, that player puts X +1/+1 counters on an Army they control. It's also a Goblin. If they don't control an Army, they create a 0/0 black Goblin Army creature token first.)";

/// The destroyed creature's controller amasses, not Azog's controller.
#[test]
fn p55_azog_has_the_destroyed_creatures_controller_amass() {
    let definition = compile_to_runtime_definition("Azog, Moria's Ruin", AZOG_MORIAS_RUIN, false)
        .expect("Azog should compile");
    let amass = p55_effects(&definition)
        .into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::AmassEffect>().cloned())
        .expect("an amass instruction");
    assert!(amass.player.is_some(), "{amass:?}");
    assert_eq!(amass.subtype, Some(ironsmith::types::Subtype::Goblin));
}

const AZULA_CUNNING_USURPER: &str = "Mana cost: {2}{U}{B}{B}\nType: Legendary Creature — Human Noble Rogue\nPower/Toughness: 4/4\nFirebending 2 (Whenever this creature attacks, add {R}{R}. This mana lasts until end of combat.)\nWhen Azula enters, target opponent exiles a nontoken creature they control, then they exile a nonland card from their graveyard.\nDuring your turn, you may cast cards exiled with Azula and you may cast them as though they had flash. Mana of any type can be spent to cast those spells.";

/// The linked-exile cast permission is limited to your turns, has flash
/// timing, and lets any type of mana pay.
#[test]
fn p55_azula_casts_linked_exiled_cards_with_flash_and_any_mana() {
    let definition = compile_to_runtime_definition("Azula, Cunning Usurper", AZULA_CUNNING_USURPER, false)
        .expect("Azula should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("DuringYourTurns"), "{debug}");
    assert!(debug.contains("instant_timing: true"), "{debug}");
    assert!(debug.contains("AnyType"), "{debug}");
}

const AZURE_BEASTBINDER: &str = "Mana cost: {1}{U}\nType: Creature — Rat Rogue\nPower/Toughness: 1/3\nVigilance\nThis creature can't be blocked by creatures with power 2 or greater.\nWhenever this creature attacks, up to one target artifact, creature, or planeswalker an opponent controls loses all abilities until your next turn. If it's a creature, it also has base power and toughness 2/2 until your next turn.";

/// The serial type list after "target" stays one target selector.
#[test]
fn p55_azure_beastbinder_targets_an_artifact_creature_or_planeswalker() {
    let definition = compile_to_runtime_definition("Azure Beastbinder", AZURE_BEASTBINDER, false)
        .expect("Azure Beastbinder should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("Planeswalker"), "{debug}");
    assert!(debug.contains("Artifact"), "{debug}");
    assert!(debug.contains("RemoveAllAbilities") || debug.contains("LoseAllAbilities"), "{debug}");
}

const BABA_LYSAGA_NIGHT_WITCH: &str = "Mana cost: {1}{B}{G}\nType: Legendary Creature — Human Warlock\nPower/Toughness: 3/3\n{T}, Sacrifice up to three permanents: If there were three or more card types among the sacrificed permanents, each opponent loses 3 life, you gain 3 life, and you draw three cards.";

/// The gate counts card types among the permanents sacrificed as the cost.
#[test]
fn p55_baba_lysaga_gates_on_card_types_among_sacrificed_permanents() {
    let definition = compile_to_runtime_definition("Baba Lysaga, Night Witch", BABA_LYSAGA_NIGHT_WITCH, false)
        .expect("Baba Lysaga should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("CardTypesAmong"), "{debug}");
    assert!(debug.contains("Fixed(3)"), "{debug}");
}

const BARON_HELMUT_ZEMO: &str = "Mana cost: {B}{B}{B}\nType: Legendary Creature — Human Noble Villain\nPower/Toughness: 3/3\nWhenever you cast a black spell from your hand, Baron Helmut Zemo connives.\nBoast — Exile any number of black cards from your graveyard with fifteen or more black mana symbols among their mana costs: Copy those exiled cards. You may cast up to three of the copies without paying their mana costs. (Activate only if this creature attacked this turn and only once each turn.)";

/// Up to three copies of the cost-exiled cards are cast for free.
#[test]
fn p55_baron_zemo_casts_up_to_three_copies_of_the_cost_exiled_cards() {
    let definition = compile_to_runtime_definition("Baron Helmut Zemo", BARON_HELMUT_ZEMO, false)
        .expect("Baron Helmut Zemo should compile");
    let effects = p55_effects(&definition);
    let choose = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>())
        .expect("the choice of exiled cards");
    assert_eq!(choose.filter.zone, Some(Zone::Exile));
    let cast = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::CastTaggedEffect>())
        .expect("the cast of each chosen copy");
    assert!(cast.as_copy && cast.without_paying_mana_cost, "{cast:?}");
}

const BANON_THE_RETURNERS_LEADER: &str = "Mana cost: {R}{W}\nType: Legendary Creature — Human Cleric Rebel\nPower/Toughness: 3/3\nPray — Once during each of your turns, you may cast a creature spell from among cards in your graveyard that were put there from anywhere other than the battlefield this turn.\nWhenever you attack, you may pay {1} and discard a card. If you do, draw a card.";

/// The once-per-turn graveyard permission only covers cards that reached the
/// graveyard this turn from somewhere other than the battlefield.
#[test]
fn p55_banon_casts_creatures_put_into_the_graveyard_from_elsewhere_this_turn() {
    let definition = compile_to_runtime_definition("Banon, the Returners' Leader", BANON_THE_RETURNERS_LEADER, false)
        .expect("Banon should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("entered_graveyard_not_from_battlefield_this_turn: true"), "{debug}");
    assert!(debug.contains("OnceDuringEachOfYourTurns"), "{debug}");
}

const ARBORIA: &str = "Mana cost: {2}{G}{G}\nType: World Enchantment\nCreatures can't attack a player unless that player cast a spell or put a nontoken permanent onto the battlefield during their last turn.";

/// Arboria is a global attack restriction; a player with no history for
/// their last turn (no turn taken yet) can't be attacked while it is out.
#[test]
fn p55_arboria_is_a_global_inactive_player_attack_restriction() {
    use ironsmith::static_abilities::StaticAbilityId;
    let definition = compile_to_runtime_definition("Arboria", ARBORIA, false)
        .expect("Arboria should compile");
    assert!(
        p55_static_ids(&definition)
            .contains(&StaticAbilityId::CreaturesCantAttackPlayersInactiveLastTurn),
        "{definition:?}"
    );
    let game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    assert!(!ironsmith::static_abilities::player_was_active_during_their_last_turn(
        &game,
        PlayerId::from_index(1)
    ));
}

const BARBARIAN_BULLY: &str = "Mana cost: {2}{R}\nType: Creature — Human Barbarian\nPower/Toughness: 2/2\nDiscard a card at random: This creature gets +2/+2 until end of turn unless a player has this creature deal 4 damage to them. Activate only once each turn.";

/// Any player may take 4 damage from the Bully to stop the pump.
#[test]
fn p55_barbarian_bully_pump_is_an_any_player_punisher() {
    let definition = compile_to_runtime_definition("Barbarian Bully", BARBARIAN_BULLY, false)
        .expect("Barbarian Bully should compile");
    let unless = p55_effects(&definition)
        .into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::UnlessActionEffect>().cloned())
        .expect("an unless-action choice");
    assert_eq!(unless.player, ironsmith::target::PlayerFilter::Any);
    let debug = format!("{unless:?}");
    assert!(debug.contains("Fixed(4)"), "{debug}");
}

const ASHLING_THE_LIMITLESS: &str = "Mana cost: {2}{R}\nType: Legendary Creature — Elemental Sorcerer\nPower/Toughness: 3/3\nElemental permanent spells you cast from your hand gain evoke {4} as you cast them. (If you cast a spell for its evoke cost, it's sacrificed when it enters.)\nWhenever you sacrifice a nontoken Elemental, create a token that's a copy of it. The token gains haste until end of turn. At the beginning of your next end step, sacrifice it unless you pay {W}{U}{B}{R}{G}.";

/// Granted evoke is a hand alternative cost named Evoke whose permanents
/// carry the evoke sacrifice trigger.
#[test]
fn p55_ashling_grants_evoke_to_elemental_hand_spells() {
    let definition = compile_to_runtime_definition("Ashling, the Limitless", ASHLING_THE_LIMITLESS, false)
        .expect("Ashling should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("Evoke"), "{debug}");
    assert!(debug.contains("permanent_this_way_grants"), "{debug}");
    assert!(debug.contains("Elemental"), "{debug}");
}
