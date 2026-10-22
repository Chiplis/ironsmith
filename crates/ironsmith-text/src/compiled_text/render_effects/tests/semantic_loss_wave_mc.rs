//! Semantic-loss fix wave (2026-10-21), package MC: real-text compile and
//! runtime checks for the behaviour each fix restored.

use super::*;

fn compile(name: &str, card_types: Vec<CardType>, text: &str) -> crate::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(card_types)
        .parse_text(text)
        .unwrap()
}

/// Whitespace-free pretty Debug of a definition, so nested typed facts can be
/// matched without depending on indentation.
fn compact_debug(definition: &crate::CardDefinition) -> String {
    format!("{definition:#?}")
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect()
}

fn lines(definition: &crate::CardDefinition) -> String {
    crate::compiled_text::compiled_text_lines(definition).join("\n")
}

// ---------------------------------------------------------------------------
// Push // Pull: "from a single graveyard" targets share one owner's graveyard.
// ---------------------------------------------------------------------------

fn single_graveyard_creature_targets() -> ChooseSpec {
    let mut filter = ObjectFilter::creature().in_zone(Zone::Graveyard);
    filter.single_graveyard = true;
    ChooseSpec::WithCount(
        Box::new(ChooseSpec::Target(Box::new(ChooseSpec::Object(filter)))),
        crate::effect::ChoiceCount::up_to(2),
    )
}

fn creature_card() -> crate::card::Card {
    crate::card::CardBuilder::new(crate::ids::CardId::new(), "Graveyard Bear")
        .card_types(vec![CardType::Creature])
        .build()
}

#[test]
fn single_graveyard_target_sets_are_grouped_by_owner() {
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let card = creature_card();
    let a1 = game.create_object_from_card(&card, alice, Zone::Graveyard);
    let a2 = game.create_object_from_card(&card, alice, Zone::Graveyard);
    let b1 = game.create_object_from_card(&card, bob, Zone::Graveyard);
    let legal = [a1, a2, b1].map(crate::game_state::Target::Object);
    let sets = crate::targeting::legal_target_sets_for_spec(
        &game,
        &single_graveyard_creature_targets(),
        &legal,
    );
    // Before the fix no constraint was reported and any mix was legal.
    assert_eq!(sets.len(), 2, "{sets:?}");
    for set in &sets {
        let owners = set
            .iter()
            .map(|target| match target {
                crate::game_state::Target::Object(id) => game.object(*id).unwrap().owner,
                other => panic!("unexpected target {other:?}"),
            })
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(owners.len(), 1, "mixed graveyards in {set:?}");
    }
    assert!(sets.iter().any(|set| set.contains(&crate::game_state::Target::Object(a1))
        && set.contains(&crate::game_state::Target::Object(a2))));
}

#[test]
fn two_single_graveyard_targets_need_two_cards_in_one_graveyard() {
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let card = creature_card();
    let a1 = game.create_object_from_card(&card, alice, Zone::Graveyard);
    let b1 = game.create_object_from_card(&card, bob, Zone::Graveyard);
    let legal = [a1, b1].map(crate::game_state::Target::Object);
    let spec = single_graveyard_creature_targets();
    assert!(!crate::targeting::has_enough_legal_targets_for_spec(&game, &spec, &legal, 2));
    assert!(crate::targeting::has_enough_legal_targets_for_spec(&game, &spec, &legal, 1));
}

#[test]
fn pull_keeps_and_renders_the_single_graveyard_restriction() {
    let definition = compile(
        "Pull Probe",
        vec![CardType::Sorcery],
        "Put up to two target creature cards from a single graveyard onto the battlefield under your control.",
    );
    assert!(compact_debug(&definition).contains("single_graveyard:true"));
    assert!(lines(&definition).contains("from a single graveyard"), "{}", lines(&definition));
}

// ---------------------------------------------------------------------------
// Nahiri, the Unforgiving: "attacks a player" is any player, not an opponent
// of the ability's controller.
// ---------------------------------------------------------------------------

#[test]
fn attacks_a_player_requirement_names_any_player() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Nahiri Probe")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(5)
        .parse_text("+1: Until your next turn, up to one target creature attacks a player each combat if able.")
        .unwrap();
    let debug = compact_debug(&definition);
    let start = debug.find("MustAttackPlayer{").expect("must-attack requirement");
    let block = &debug[start..];
    let player_at = block.find(",player:").expect("required player");
    let player = &block[player_at..player_at + 20];
    assert!(player.starts_with(",player:Any"), "{player}");
    assert!(lines(&definition).contains("attacks a player each combat if able"));
}

#[test]
fn any_player_requirement_includes_the_ability_controller() {
    // Restriction::MustAttackPlayer resolves its player filter relative to
    // the ability's controller; `Any` keeps that controller attackable by
    // an opponent's creature, which `Opponent` excluded.
    use crate::filter::PlayerFilterExt as _;
    let game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let ctx = game.filter_context_for(alice, None);
    assert!(PlayerFilter::Any.matches_player(alice, &ctx));
    assert!(!PlayerFilter::Opponent.matches_player(alice, &ctx));
}

// ---------------------------------------------------------------------------
// On Serra's Wings: "is legendary" adds the supertype instead of filtering.
// ---------------------------------------------------------------------------

const SERRAS_WINGS: &str = "Enchant creature\nEnchanted creature is legendary, gets +1/+1, and has flying, vigilance, and lifelink.";

fn serras_wings() -> crate::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "On Serra's Wings")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(SERRAS_WINGS)
        .unwrap()
}

#[test]
fn attached_is_legendary_adds_the_supertype() {
    let debug = format!("{:#?}", serras_wings());
    assert!(debug.contains("add supertypes"), "{debug}");
}

#[test]
fn attached_is_legendary_does_not_gate_the_bonus_on_legendary() {
    let debug = format!("{:#?}", serras_wings());
    for line in debug.lines().filter(|line| {
        line.contains("payload: \"Anthem(") || line.contains("payload: \"GrantObjectAbilityForFilter(")
    }) {
        assert!(!line.contains("supertypes: [Legendary]"), "{line}");
    }
}

#[test]
fn attached_is_legendary_renders_all_three_predicates() {
    assert_eq!(lines(&serras_wings()), SERRAS_WINGS);
}

// ---------------------------------------------------------------------------
// Rakdos, Lord of Riots: a static per-life-lost reduction, not a spell effect.
// ---------------------------------------------------------------------------

const RAKDOS_REDUCTION: &str = "Creature spells you cast cost {1} less to cast for each 1 life your opponents have lost this turn.";

#[test]
fn per_life_lost_reduction_is_a_static_not_a_spell_effect() {
    let definition = compile("Rakdos Probe", vec![CardType::Creature], RAKDOS_REDUCTION);
    assert!(definition.spell_effect.is_none(), "became a spell effect");
    assert!(
        definition
            .abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Static(_))),
        "no static modifier"
    );
}

#[test]
fn per_life_lost_reduction_scales_with_opponent_life_lost() {
    let definition = compile("Rakdos Probe", vec![CardType::Creature], RAKDOS_REDUCTION);
    assert!(compact_debug(&definition).contains("LifeLostThisTurn(Opponent"));
    assert_eq!(lines(&definition), RAKDOS_REDUCTION);
}

// ---------------------------------------------------------------------------
// Sands of Time: both sets are snapshotted before either action applies.
// ---------------------------------------------------------------------------

#[test]
fn player_subject_simultaneous_untap_tap_snapshots_both_sets() {
    let definition = compile(
        "Sands Probe",
        vec![CardType::Artifact],
        "At the beginning of each player's upkeep, that player simultaneously untaps each tapped artifact, creature, and land they control and taps each untapped artifact, creature, and land they control.",
    );
    let debug = compact_debug(&definition);
    assert_eq!(debug.matches("TagMatchingObjectsEffect").count(), 2, "{debug}");
    assert!(debug.contains("simultaneous_0") && debug.contains("simultaneous_1"));
}

// ---------------------------------------------------------------------------
// Sonar Strike: a three-adjective combat-state list is one disjunction.
// ---------------------------------------------------------------------------

#[test]
fn serial_combat_state_list_keeps_every_arm() {
    let definition = compile(
        "Sonar Probe",
        vec![CardType::Instant],
        "Sonar Probe deals 4 damage to target attacking, blocking, or tapped creature.",
    );
    let debug = compact_debug(&definition);
    for state in ["attacking:true", "blocking:true", "tapped:true"] {
        assert_eq!(debug.matches(state).count(), 1, "{state}: {debug}");
    }
    assert!(debug.contains("any_of:["));
}

// ---------------------------------------------------------------------------
// Scampire: "with a sticker on it" restricts the target to stickered cards.
// ---------------------------------------------------------------------------

#[test]
fn plain_sticker_requirement_restricts_the_target() {
    let definition = compile(
        "Graveyard Recursion Probe",
        vec![CardType::Creature],
        "{3}{B}: Return target creature card with a sticker on it from your graveyard to the battlefield.",
    );
    assert!(compact_debug(&definition).contains("sticker:Some(Sticker"));
    assert!(lines(&definition).contains("with a sticker on it"));
}

// ---------------------------------------------------------------------------
// Raph & Mikey: the revealed creature enters tapped and attacking.
// ---------------------------------------------------------------------------

#[test]
fn consult_match_enters_tapped_and_attacking() {
    let definition = compile(
        "Consult Probe",
        vec![CardType::Creature],
        "Whenever this creature attacks, reveal cards from the top of your library until you reveal a creature card. Put that card onto the battlefield tapped and attacking and the rest on the bottom of your library in a random order.",
    );
    let debug = compact_debug(&definition);
    assert!(debug.contains("enters_tapped:true") && debug.contains("enters_attacking:true"));
    assert!(lines(&definition).contains("onto the battlefield tapped and attacking"));
}

// ---------------------------------------------------------------------------
// Sentry Oak: an optional clash's follow-up requires winning the clash.
// ---------------------------------------------------------------------------

fn sentry_oak() -> crate::CardDefinition {
    compile(
        "Sentry Probe",
        vec![CardType::Creature],
        "At the beginning of combat on your turn, you may clash with an opponent. If you win, this creature gets +2/+0 until end of turn.",
    )
}

#[test]
fn optional_clash_win_reads_the_clash_result() {
    let debug = compact_debug(&sentry_oak());
    assert!(!debug.contains("predicate:Happened"), "{debug}");
    assert!(debug.contains("predicate:Value(GreaterThan(0"), "{debug}");
}

#[test]
fn optional_clash_win_renders_if_you_win() {
    assert!(lines(&sentry_oak()).contains("If you win,"), "{}", lines(&sentry_oak()));
}

// ---------------------------------------------------------------------------
// Nelly Borca: you and the controller of those creatures each draw.
// ---------------------------------------------------------------------------

#[test]
fn joint_draw_includes_the_controller_of_the_triggering_creatures() {
    let definition = compile(
        "Nelly Probe",
        vec![CardType::Creature],
        "Whenever one or more creatures an opponent controls deal combat damage to one or more of your opponents, you and the controller of those creatures each draw a card.",
    );
    let debug = compact_debug(&definition);
    assert_eq!(debug.matches("DrawCardsEffect").count(), 2, "{debug}");
    assert!(debug.contains("player:ControllerOf(Tagged(TagKey(\"triggering\""), "{debug}");
}

// ---------------------------------------------------------------------------
// Radiant Destiny: the conditional vigilance goes to the bonus recipients.
// ---------------------------------------------------------------------------

const RADIANT: &str = "As this enchantment enters, choose a creature type.\nCreatures you control of the chosen type get +1/+1. As long as you have the city's blessing, they also have vigilance.";

#[test]
fn conditional_they_grant_reaches_the_previous_recipients() {
    let definition = compile("Radiant Probe", vec![CardType::Enchantment], RADIANT);
    let debug = format!("{definition:#?}");
    let grant = debug
        .lines()
        .find(|line| line.contains("payload: \"GrantObjectAbilityForFilter(") && line.contains("Vigilance"))
        .expect("vigilance grant");
    assert!(!grant.contains("source: true"), "granted to the source: {grant}");
    assert!(grant.contains("chosen_creature_type: true"), "{grant}");
    assert!(grant.contains("PlayerHasCitysBlessing"), "{grant}");
}

#[test]
fn conditional_they_grant_renders_as_a_continuation() {
    let definition = compile("Radiant Probe", vec![CardType::Enchantment], RADIANT);
    assert_eq!(lines(&definition), RADIANT);
}

// ---------------------------------------------------------------------------
// Maelstrom of the Spirit Dragon: "an Omen spell" is a spell subtype arm.
// ---------------------------------------------------------------------------

#[test]
fn omen_spell_arm_survives_in_the_mana_restriction() {
    let definition = compile(
        "Maelstrom Probe",
        vec![CardType::Land],
        "{T}: Add one mana of any color. Spend this mana only to cast a Dragon spell or an Omen spell.",
    );
    let debug = compact_debug(&definition);
    let restriction = &debug[debug.find("mana_usage_restrictions").expect("restriction")..];
    assert!(restriction.contains("Dragon,Omen"), "{restriction}");
    assert!(lines(&definition).contains("Dragon or Omen spell"));
}

// ---------------------------------------------------------------------------
// Pyrrhic Strike: "choose both instead" requires both modes.
// ---------------------------------------------------------------------------

#[test]
fn paid_cost_choose_both_requires_both_modes() {
    let definition = compile(
        "Pyrrhic Probe",
        vec![CardType::Instant],
        "As an additional cost to cast this spell, you may blight 2.\nChoose one. If this spell's additional cost was paid, choose both instead.\n• Destroy target artifact or enchantment.\n• Destroy target creature with mana value 3 or greater.",
    );
    let debug = compact_debug(&definition);
    assert!(debug.contains("min_modes:Fixed(2,)"), "{debug}");
    assert!(debug.contains("max_modes:Fixed(2,)"), "{debug}");
}
