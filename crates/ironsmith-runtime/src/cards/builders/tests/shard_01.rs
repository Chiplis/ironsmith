#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::shard_07::*;
use super::shard_08::*;
use super::shard_09::*;
use super::shard_10::*;
use super::shard_11::*;
use super::shard_12::*;
use super::shard_13::*;
use super::shard_14::*;
use super::shard_15::*;
use super::shard_16::*;
use super::shard_17::*;
use super::shard_18::*;
use super::shard_19::*;
use super::shard_20::*;
use super::shard_21::*;
use super::shard_22::*;
use super::shard_23::*;
use super::*;

#[test]
pub(super) fn plague_of_vermin_runtime_starts_payment_rounds_with_spell_controller() {
    struct RecordingPayments {
        payments: HashMap<PlayerId, Vec<u32>>,
        prompted_players: Vec<PlayerId>,
    }

    impl crate::decision::DecisionMaker for RecordingPayments {
        fn decide_number(
            &mut self,
            _game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::NumberContext,
        ) -> u32 {
            self.prompted_players.push(ctx.player);
            self.payments
                .get_mut(&ctx.player)
                .and_then(|payments| payments.pop())
                .unwrap_or(ctx.min)
                .clamp(ctx.min, ctx.max)
        }
    }

    let def = parse_oracle_card_definition("Plague of Vermin");
    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    game.turn_store.turn_order = vec![charlie, bob, alice];
    let source = game.create_object_from_definition(&def, bob, Zone::Stack);
    let mut dm = RecordingPayments {
        payments: HashMap::from([
            (bob, vec![0, 1]),
            (alice, vec![0, 0]),
            (charlie, vec![0, 0]),
        ]),
        prompted_players: Vec::new(),
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, bob, &mut dm);
    let effects = &def
        .spell_effect
        .as_ref()
        .expect("Plague of Vermin should have spell effects")
        .segments[0]
        .default_effects;

    for effect in effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Plague of Vermin effect should resolve");
    }

    assert_eq!(
        dm.prompted_players,
        vec![bob, alice, charlie, bob, alice, charlie],
        "payment rounds should start with the spell controller and continue in turn order"
    );
}

#[test]
pub(super) fn commander_liara_portyr_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Commander Liara Portyr");
    let ability_debug = format!("{:#?}", def.abilities);
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        def.abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Triggered(_))),
        "Commander Liara Portyr should parse its attack trigger strictly"
    );
    assert!(
        ability_debug.contains("PlayersBeingAttacked")
            && ability_debug.contains("applies_to_all_matching_this_turn: true")
            && ability_debug.contains("GrantPlayTaggedEffect"),
        "expected Commander Liara Portyr to lower the dynamic exile-spell reduction and cast permission structurally, got {ability_debug}"
    );
    assert_eq!(
        rendered,
        "Whenever you attack, spells you cast from exile this turn cost {X} less to cast, where X is the number of players being attacked. Exile the top X cards of your library. Until end of turn, you may cast spells from among those exiled cards."
    );
}

#[test]
pub(super) fn order_of_succession_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Order of Succession");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let spell_debug = format!("{:#?}", def.spell_effect);

    assert!(
        def.spell_effect.is_some(),
        "Order of Succession should parse as a strict sorcery spell"
    );
    assert!(
        spell_debug.contains("ChooseNamedOptionEffect")
            && spell_debug.contains("DirectionalAdjacentPlayerControlEffect"),
        "expected Order of Succession to structurally choose left/right and apply directional adjacent-player control, got {spell_debug}"
    );
    assert_eq!(
        rendered,
        "You choose left or right. Starting with you and proceeding in the chosen direction, each player chooses a creature controlled by the next player in that direction. Each player gains control of the creature they chose.",
        "Order of Succession compiled text should preserve the full directional choice-control text"
    );
}

#[test]
pub(super) fn templar_knight_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Templar Knight");
    let rendered = unprocessed_compiled_lines(&def);
    let ability_debug = format!("{:#?}", def.abilities);

    assert_eq!(
        rendered,
        vec![
            "Vigilance".to_string(),
            "{W}, Tap five untapped attacking creatures you control named Templar Knight: Search your library for a legendary artifact card, put it onto the battlefield, then shuffle.".to_string(),
            "A deck can have any number of cards named Templar Knight.".to_string(),
        ],
        "Templar Knight should parse strictly and preserve its activation and deck-construction text"
    );
    assert!(
        def.abilities.iter().any(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => {
                static_ability.id() == StaticAbilityId::DeckConstructionRuleText
            }
            _ => false,
        }),
        "Templar Knight should lower its any-number deck rule to typed deck-construction text, got {ability_debug}"
    );
    assert!(
        ability_debug.contains("untapped: true")
            && ability_debug.contains("attacking: true")
            && ability_debug.contains("name: Some(")
            && ability_debug.contains("\"templar knight\"")
            && !ability_debug.contains("RuleFallbackText")
            && !ability_debug.contains("UnsupportedParserLine"),
        "Templar Knight should structurally model the named untapped attacking creature cost without parser fallbacks, got {ability_debug}"
    );
}

#[test]
pub(super) fn templar_knight_deck_construction_rule_has_no_game_runtime_effects() {
    let def = parse_oracle_card_definition("Templar Knight");
    let static_ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => {
                if static_ability.id() == StaticAbilityId::DeckConstructionRuleText {
                    Some(static_ability)
                } else {
                    None
                }
            }
            _ => None,
        })
        .expect("Templar Knight should have a typed deck-construction static ability");

    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    assert!(
        static_ability
            .generate_effects(source, alice, &game)
            .is_empty()
    );
    assert!(
        static_ability
            .generate_replacement_effect(source, alice)
            .is_none()
    );
    assert!(static_ability.pregame_action_kind().is_none());
}

#[test]
pub(super) fn mark_of_asylum_strict_parser_compiled_text_and_runtime_prevention_regression() {
    assert_oracle_card_parses_strict("Mark of Asylum");

    let def = parse_oracle_card_definition("Mark of Asylum");
    let rendered = unprocessed_compiled_lines(&def);
    assert_eq!(
        rendered,
        vec![
            "Prevent all noncombat damage that would be dealt to creatures you control."
                .to_string(),
        ],
        "Mark of Asylum should render its noncombat prevention clause exactly"
    );

    let static_ability = def
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Static(static_ability) = &ability.kind else {
                return None;
            };
            let is_mark_prevention = static_ability.id()
                == StaticAbilityId::PreventAllNoncombatDamageToPermanentsMatching;
            is_mark_prevention.then_some(static_ability)
        })
        .expect("Mark of Asylum should lower to filtered noncombat damage prevention");
    assert_eq!(
        static_ability.display(),
        "Prevent all noncombat damage that would be dealt to creatures you control."
    );

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let _mark = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let damage_source = CardBuilder::new(CardId::new(), "Damage Source")
        .card_types(vec![CardType::Artifact])
        .build();
    let source_id = game.create_object_from_card(&damage_source, bob, Zone::Battlefield);

    let protected = CardBuilder::new(CardId::new(), "Protected Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let protected_id = game.create_object_from_card(&protected, alice, Zone::Battlefield);

    let controlled_noncreature = CardBuilder::new(CardId::new(), "Controlled Noncreature")
        .card_types(vec![CardType::Artifact])
        .build();
    let controlled_noncreature_id =
        game.create_object_from_card(&controlled_noncreature, alice, Zone::Battlefield);

    let opponent_creature = CardBuilder::new(CardId::new(), "Opponent Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let opponent_creature_id =
        game.create_object_from_card(&opponent_creature, bob, Zone::Battlefield);

    let (noncombat_damage, noncombat_prevented) =
        crate::events::processing::process_damage_with_event(
            &mut game,
            source_id,
            crate::events::DamageTarget::Object(protected_id),
            3,
            false,
            crate::events::cause::EventCause::effect(),
        );
    assert_eq!(
        noncombat_damage, 0,
        "Mark of Asylum should prevent noncombat damage to creatures you control"
    );
    assert!(noncombat_prevented);

    let (combat_damage, combat_prevented) = crate::events::processing::process_damage_with_event(
        &mut game,
        source_id,
        crate::events::DamageTarget::Object(protected_id),
        3,
        true,
        crate::events::cause::EventCause::from_combat_damage(source_id, bob),
    );
    assert_eq!(
        combat_damage, 3,
        "Mark of Asylum should not prevent combat damage"
    );
    assert!(!combat_prevented);

    let (controlled_noncreature_damage, controlled_noncreature_prevented) =
        crate::events::processing::process_damage_with_event(
            &mut game,
            source_id,
            crate::events::DamageTarget::Object(controlled_noncreature_id),
            3,
            false,
            crate::events::cause::EventCause::effect(),
        );
    assert_eq!(
        controlled_noncreature_damage, 3,
        "Mark of Asylum should not prevent damage to noncreature permanents you control"
    );
    assert!(!controlled_noncreature_prevented);

    let (opponent_damage, opponent_prevented) =
        crate::events::processing::process_damage_with_event(
            &mut game,
            source_id,
            crate::events::DamageTarget::Object(opponent_creature_id),
            3,
            false,
            crate::events::cause::EventCause::effect(),
        );
    assert_eq!(
        opponent_damage, 3,
        "Mark of Asylum should not prevent damage to creatures you do not control"
    );
    assert!(!opponent_prevented);
}

#[test]
pub(super) fn templar_knight_activation_cost_filter_requires_untapped_attacking_named_creatures_you_control()
 {
    let def = parse_oracle_card_definition("Templar Knight");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Templar Knight should have an activated ability");
    let choose_cost = activated
        .mana_cost
        .costs()
        .iter()
        .filter_map(|cost| cost.effect_ref())
        .find_map(|effect| effect.downcast_ref::<ChooseObjectsEffect>())
        .expect("Templar Knight activation should choose creatures to tap as a cost");

    assert_eq!(choose_cost.count.min, 5);
    assert_eq!(choose_cost.count.max, Some(5));
    assert!(choose_cost.filter.untapped);
    assert!(choose_cost.filter.attacking);
    assert_eq!(choose_cost.filter.controller, Some(PlayerFilter::You));
    assert_eq!(choose_cost.filter.name.as_deref(), Some("templar knight"));

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let matching = (0..5)
        .map(|_| game.create_object_from_definition(&def, alice, Zone::Battlefield))
        .collect::<Vec<_>>();
    let nonattacking = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let tapped_attacker = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.tap(tapped_attacker);
    let bob_attacker = game.create_object_from_definition(&def, bob, Zone::Battlefield);
    let decoy_def = CardDefinitionBuilder::new(CardId::new(), "Templar Decoy")
        .card_types(vec![CardType::Creature])
        .build();
    let wrong_name_attacker =
        game.create_object_from_definition(&decoy_def, alice, Zone::Battlefield);

    let mut attackers = matching
        .iter()
        .chain([&tapped_attacker, &bob_attacker, &wrong_name_attacker])
        .map(|creature| crate::combat_state::AttackerInfo {
            creature: *creature,
            target: crate::combat_state::AttackTarget::Player(bob),
        })
        .collect::<Vec<_>>();
    attackers.push(crate::combat_state::AttackerInfo {
        creature: source,
        target: crate::combat_state::AttackTarget::Player(bob),
    });
    game.combat = Some(crate::combat_state::CombatState {
        attackers,
        ..Default::default()
    });
    let ctx = game.filter_context_for(alice, Some(source));

    for creature in matching {
        let object = game
            .object(creature)
            .expect("matching Templar should exist");
        assert!(
            choose_cost.filter.matches(object, &ctx, &game),
            "untapped attacking Templars you control should satisfy the activation cost filter"
        );
    }
    for (creature, reason) in [
        (nonattacking, "nonattacking"),
        (tapped_attacker, "tapped"),
        (bob_attacker, "opponent-controlled"),
        (wrong_name_attacker, "wrong-name"),
    ] {
        let object = game
            .object(creature)
            .expect("negative branch object should exist");
        assert!(
            !choose_cost.filter.matches(object, &ctx, &game),
            "{reason} creatures should not satisfy the Templar Knight activation cost filter"
        );
    }
}

#[test]
pub(super) fn thundermane_dragon_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Thundermane Dragon");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let ability_debug = format!("{:#?}", def.abilities);

    assert!(
        ability_debug.contains("PlayFrom") && ability_debug.contains("cast_this_way_grants"),
        "Thundermane Dragon should structurally grant top-library casting with a cast-this-way ability, got {ability_debug}"
    );
    assert!(
        rendered.contains("from the top of your library")
            && rendered.contains(
                "If you cast a creature spell this way, it gains haste until end of turn"
            ),
        "Thundermane Dragon compiled text should preserve the top-library cast and haste clause, got {rendered}"
    );
}

#[test]
pub(super) fn top_library_static_permissions_render_their_full_spell_domains() {
    for (card_name, expected_permission) in [
        (
            "Korlessa, Scale Singer",
            "You may cast Dragon spells from the top of your library",
        ),
        (
            "Mystic Forge",
            "You may cast artifact spells or colorless spells from the top of your library",
        ),
        (
            "Eladamri, Korvecdal",
            "You may cast creature spells from the top of your library",
        ),
        (
            "Traveling Chocobo",
            "You may play lands and cast Bird spells from the top of your library",
        ),
    ] {
        let def = parse_oracle_card_definition(card_name);
        let rendered = unprocessed_compiled_lines(&def).join("\n");
        assert!(
            rendered.contains(expected_permission),
            "{card_name} should render {expected_permission:?}, got {rendered}"
        );
    }
}

#[test]
pub(super) fn king_darien_xlviii_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("King Darien XLVIII");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let ability_debug = format!("{:#?}", def.abilities);

    assert_eq!(
        def.abilities.len(),
        3,
        "King Darien XLVIII should parse its static ability and two activated abilities, got {ability_debug}"
    );
    assert!(
        ability_debug.contains("Anthem")
            && ability_debug.contains("PutCountersEffect")
            && ability_debug.contains("SourceReference")
            && ability_debug.contains("CreateTokenEffect")
            && ability_debug.contains("SacrificeTargetEffect")
            && ability_debug.contains("Hexproof")
            && ability_debug.contains("Indestructible"),
        "King Darien XLVIII should structurally model all three abilities, got {ability_debug}"
    );
    assert!(
        rendered.contains("Other creatures you control get +1/+1."),
        "expected anthem text, got {rendered}"
    );
    assert!(
        rendered.contains(
            "{3}{G}{W}: Put a +1/+1 counter on King Darien and create a 1/1 white Soldier creature token."
        ),
        "expected self-counter plus Soldier creation text, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Sacrifice this creature: Creature tokens you control gain hexproof and indestructible until end of turn."
        ),
        "expected token keyword grant text, got {rendered}"
    );
}

#[test]
pub(super) fn king_darien_xlviii_anthem_buffs_only_your_other_creatures_runtime() {
    let def = parse_oracle_card_definition("King Darien XLVIII");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let alice_creature = CardDefinitionBuilder::new(CardId::from_raw(92_010), "Alice Recruit")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let bob_creature = CardDefinitionBuilder::new(CardId::from_raw(92_011), "Bob Recruit")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let alice_id = game.create_object_from_definition(&alice_creature, alice, Zone::Battlefield);
    let bob_id = game.create_object_from_definition(&bob_creature, bob, Zone::Battlefield);

    let alice_chars = game
        .calculated_characteristics(alice_id)
        .expect("Alice creature should have characteristics");
    assert_eq!(
        (alice_chars.power, alice_chars.toughness),
        (Some(2), Some(2)),
        "King Darien XLVIII should give other creatures you control +1/+1"
    );

    let bob_chars = game
        .calculated_characteristics(bob_id)
        .expect("Bob creature should have characteristics");
    assert_eq!(
        (bob_chars.power, bob_chars.toughness),
        (Some(1), Some(1)),
        "King Darien XLVIII should not buff opponents' creatures"
    );
}

#[test]
pub(super) fn knight_of_new_alara_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Knight of New Alara");
    let def = parse_oracle_card_definition("Knight of New Alara");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    let abilities_debug = format!("{:#?}", def.abilities);

    assert!(
        rendered_lower
            .contains("other multicolored creatures you control get +1/+1 for each of its colors"),
        "Knight of New Alara should render the affected-creature color-count anthem, got {rendered}"
    );
    assert!(
        abilities_debug.contains("ColorsOfAffected")
            && !abilities_debug.contains("UnsupportedParserLine")
            && !abilities_debug.contains("RuleFallbackText"),
        "Knight of New Alara should structurally count each affected creature's colors, got {abilities_debug}"
    );
}

#[test]
pub(super) fn knight_of_new_alara_counts_each_affected_creatures_colors_runtime() {
    fn colored_creature_def(
        name: &str,
        colors: crate::color::ColorSet,
        power: i32,
        toughness: i32,
    ) -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .color_indicator(colors)
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build()
    }

    let knight_oracle = oracle_text_by_name()
        .get("Knight of New Alara")
        .expect("missing oracle text for Knight of New Alara")
        .clone();
    let knight = CardDefinitionBuilder::new(CardId::new(), "Knight of New Alara")
        .card_types(vec![CardType::Creature])
        .color_indicator(crate::color::ColorSet::GREEN.union(crate::color::ColorSet::WHITE))
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(knight_oracle)
        .expect("Knight of New Alara should parse for runtime regression");
    let two_color = colored_creature_def(
        "Alice Two-Color Creature",
        crate::color::ColorSet::WHITE.union(crate::color::ColorSet::BLUE),
        2,
        2,
    );
    let three_color = colored_creature_def(
        "Alice Three-Color Creature",
        crate::color::ColorSet::WHITE
            .union(crate::color::ColorSet::BLUE)
            .union(crate::color::ColorSet::BLACK),
        1,
        1,
    );
    let one_color = colored_creature_def(
        "Alice One-Color Creature",
        crate::color::ColorSet::GREEN,
        2,
        2,
    );
    let bob_two_color = colored_creature_def(
        "Bob Two-Color Creature",
        crate::color::ColorSet::RED.union(crate::color::ColorSet::GREEN),
        2,
        2,
    );
    let color_granter = CardDefinitionBuilder::new(CardId::new(), "Alice Color Granter")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(
            crate::static_abilities::StaticAbility::add_colors(
                ObjectFilter::creature()
                    .you_control()
                    .named("Alice Two-Color Creature"),
                crate::color::ColorSet::RED,
            ),
        ))
        .build();

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let knight_id = game.create_object_from_definition(&knight, alice, Zone::Battlefield);
    let two_color_id = game.create_object_from_definition(&two_color, alice, Zone::Battlefield);
    let three_color_id = game.create_object_from_definition(&three_color, alice, Zone::Battlefield);
    let one_color_id = game.create_object_from_definition(&one_color, alice, Zone::Battlefield);
    let bob_two_color_id =
        game.create_object_from_definition(&bob_two_color, bob, Zone::Battlefield);
    game.create_object_from_definition(&color_granter, alice, Zone::Battlefield);

    assert_eq!(
        (
            game.calculated_power(two_color_id),
            game.calculated_toughness(two_color_id)
        ),
        (Some(5), Some(5)),
        "a two-color creature Alice controls that gains a third color should get +3/+3"
    );
    assert_eq!(
        (
            game.calculated_power(three_color_id),
            game.calculated_toughness(three_color_id)
        ),
        (Some(4), Some(4)),
        "a three-color creature Alice controls should get +3/+3"
    );
    assert_eq!(
        (
            game.calculated_power(one_color_id),
            game.calculated_toughness(one_color_id)
        ),
        (Some(2), Some(2)),
        "a monocolored creature should not match the multicolored subject"
    );
    assert_eq!(
        (
            game.calculated_power(bob_two_color_id),
            game.calculated_toughness(bob_two_color_id)
        ),
        (Some(2), Some(2)),
        "an opponent's multicolored creature should not be affected"
    );
    assert_eq!(
        (
            game.calculated_power(knight_id),
            game.calculated_toughness(knight_id)
        ),
        (Some(2), Some(2)),
        "Knight of New Alara should not buff itself"
    );
}

#[test]
pub(super) fn king_darien_xlviii_mana_ability_puts_counter_on_self_and_creates_soldier_runtime() {
    let def = parse_oracle_card_definition("King Darien XLVIII");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Activated(activated) = &ability.kind else {
                return None;
            };
            format!("{:?}", activated.effects)
                .contains("CreateTokenEffect")
                .then_some(activated)
        })
        .expect("King Darien XLVIII should have its mana activated ability");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let king_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(king_id, alice, &mut dm);
    for effect in activated.effects.flattened_default_effects() {
        effect
            .0
            .execute(&mut game, &mut ctx)
            .expect("King Darien XLVIII mana ability effect should resolve");
    }

    assert_eq!(
        game.object(king_id).and_then(|object| object
            .counters
            .get(&crate::object::CounterType::PlusOnePlusOne)
            .copied()),
        Some(1),
        "the mana ability should put a +1/+1 counter on King Darien XLVIII"
    );
    assert!(
        game.battlefield.iter().any(|id| {
            game.object(*id).is_some_and(|object| {
                object.name == "Soldier" && object.kind == crate::object::ObjectKind::Token
            })
        }),
        "the mana ability should create a Soldier creature token"
    );
}

#[test]
pub(super) fn king_darien_xlviii_sacrifice_ability_grants_keywords_only_to_your_tokens_runtime() {
    let def = parse_oracle_card_definition("King Darien XLVIII");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Activated(activated) = &ability.kind else {
                return None;
            };
            format!("{:?}", activated.effects)
                .contains("ApplyContinuousEffect")
                .then_some(activated)
        })
        .expect("King Darien XLVIII should have its sacrifice activated ability");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let king_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let token_def = CardDefinitionBuilder::new(CardId::from_raw(92_012), "Alice Token")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let alice_token = game.create_object_from_definition(&token_def, alice, Zone::Battlefield);
    game.object_mut(alice_token)
        .expect("Alice token should exist")
        .kind = crate::object::ObjectKind::Token;
    let bob_token = game.create_object_from_definition(&token_def, bob, Zone::Battlefield);
    game.object_mut(bob_token)
        .expect("Bob token should exist")
        .kind = crate::object::ObjectKind::Token;
    let nontoken_def = CardDefinitionBuilder::new(CardId::from_raw(92_013), "Alice Nontoken")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let alice_nontoken =
        game.create_object_from_definition(&nontoken_def, alice, Zone::Battlefield);

    let sacrifice_cost = activated
        .mana_cost
        .costs()
        .first()
        .expect("sacrifice ability should have a sacrifice cost");
    let mut cost_dm = crate::decision::AutoPassDecisionMaker;
    let mut cost_ctx = crate::costs::CostContext::new(king_id, alice, &mut cost_dm);
    sacrifice_cost
        .pay(&mut game, &mut cost_ctx)
        .expect("King Darien XLVIII sacrifice cost should be payable");
    assert!(
        !game.battlefield.contains(&king_id)
            && game.player(alice).is_some_and(|player| {
                player.graveyard.iter().any(|id| {
                    game.object(*id)
                        .is_some_and(|object| object.name == "King Darien XLVIII")
                })
            }),
        "paying the sacrifice cost should move King Darien XLVIII from the battlefield to the graveyard"
    );

    let mut effect_dm = crate::decision::AutoPassDecisionMaker;
    let mut effect_ctx = crate::effects::ExecutionContext::new(king_id, alice, &mut effect_dm);
    for effect in activated.effects.flattened_default_effects() {
        effect
            .0
            .execute(&mut game, &mut effect_ctx)
            .expect("King Darien XLVIII sacrifice ability effect should resolve");
    }

    assert!(
        game.object_has_static_ability_id(alice_token, StaticAbilityId::Hexproof)
            && game.object_has_static_ability_id(alice_token, StaticAbilityId::Indestructible),
        "your creature token should gain hexproof and indestructible"
    );
    assert!(
        !game.object_has_static_ability_id(alice_nontoken, StaticAbilityId::Hexproof)
            && !game.object_has_static_ability_id(alice_nontoken, StaticAbilityId::Indestructible),
        "your nontoken creature should not gain the token-only keywords"
    );
    assert!(
        !game.object_has_static_ability_id(bob_token, StaticAbilityId::Hexproof)
            && !game.object_has_static_ability_id(bob_token, StaticAbilityId::Indestructible),
        "opponents' creature tokens should not gain the keywords"
    );
}

#[test]
pub(super) fn stoic_sphinx_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Stoic Sphinx");
    let ability_debug = format!("{:#?}", def.abilities);
    let rendered = canonical_compiled_lines(&def).join("\n");

    assert!(
        ability_debug.contains("Hexproof")
            && ability_debug.contains("Not")
            && ability_debug.contains("PlayerCastSpellsThisTurnOrMore"),
        "Stoic Sphinx should model conditional hexproof using the player-cast-spell condition, got {ability_debug}"
    );
    assert!(
        rendered
            .contains("This creature has hexproof as long as you haven't cast a spell this turn."),
        "Stoic Sphinx compiled text should preserve the haven't-cast-a-spell static clause, got {rendered}"
    );
}

#[test]
pub(super) fn targeted_theft_cards_keep_control_untap_and_haste_sentence_boundaries() {
    let cases = [
        ("Bloody Betrayal", "It gains haste until end of turn."),
        ("Bond of Passion", "It gains haste until end of turn."),
        ("Caught Red-Handed", "It gains haste until end of turn."),
        (
            "Involuntary Employment",
            "It gains haste until end of turn.",
        ),
        ("Portent of Betrayal", "It gains haste until end of turn."),
        ("Traitorous Greed", "It gains haste until end of turn."),
        ("Furnace Reins", "Until end of turn, it gains haste and"),
        ("Lose Calm", "It gains haste and menace until end of turn."),
        (
            "Shackles of Treachery",
            "Until end of turn, it gains haste and",
        ),
    ];

    for (name, haste_fragment) in cases {
        let rendered = canonical_compiled_lines(&parse_oracle_card_definition(name)).join("\n");
        assert!(
            rendered.contains(
                "Gain control of target creature until end of turn. Untap that creature."
            ),
            "{name} should keep the targeted control and untap instructions as separate sentences, got {rendered}"
        );
        assert!(
            rendered.contains(haste_fragment),
            "{name} should retain its haste grant after the untap instruction, got {rendered}"
        );
        assert!(
            !rendered.contains("until end of turn, untap that creature"),
            "{name} should not merge the control and untap instructions, got {rendered}"
        );
    }
}

#[test]
pub(super) fn additional_draw_cards_preserve_the_oracle_modifier_in_compiled_text() {
    let cases = [
        (
            "ED-E, Lonesome Eyebot",
            "draw an additional card for each quest counter on ed-e",
        ),
        ("Font of Mythos", "draws two additional cards"),
        ("Grafted Skullcap", "draw an additional card"),
        ("Heightened Awareness", "draw an additional card"),
        ("Howling Mine", "draws an additional card"),
        ("Kami of the Crescent Moon", "draws an additional card"),
        ("Lord Skitter's Blessing", "draw an additional card"),
        ("Lord Windgrace", "draw an additional card"),
        ("Monastery Siege", "draw an additional card"),
        ("Righteous Authority", "draws an additional card"),
        ("Sylvan Library", "draw two additional cards"),
        ("Well of Ideas", "draws an additional card"),
    ];

    for (name, expected) in cases {
        let rendered = canonical_compiled_lines(&parse_oracle_card_definition(name))
            .join("\n")
            .to_ascii_lowercase();
        assert!(
            rendered.contains(expected),
            "{name} should preserve its additional-draw surface, got {rendered}"
        );
    }

    let well = canonical_compiled_lines(&parse_oracle_card_definition("Well of Ideas"))
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        well.contains("draw two additional cards"),
        "Well of Ideas should preserve both additional-draw counts, got {well}"
    );
}

pub(super) fn record_stoic_sphinx_spell_cast_event(
    game: &mut crate::game_state::GameState,
    caster: PlayerId,
    raw_id: u32,
) {
    let spell = CardBuilder::new(CardId::from_raw(raw_id), "Recorded Spell")
        .card_types(vec![CardType::Instant])
        .build();
    let spell_id = game.create_object_from_card(&spell, caster, Zone::Stack);
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(spell_id)
            .expect("recorded spell should exist on the stack"),
        game,
    );
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new_with_snapshot(
            spell_id,
            caster,
            Zone::Hand,
            snapshot.clone(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.turn_store
        .turn_history
        .record_event(&event, Some(snapshot), None);
}

#[test]
pub(super) fn stoic_sphinx_hexproof_tracks_whether_controller_cast_a_spell_this_turn() {
    let def = parse_oracle_card_definition("Stoic Sphinx");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let sphinx_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    assert!(
        game.object_has_static_ability_id(sphinx_id, StaticAbilityId::Hexproof),
        "Stoic Sphinx should have hexproof before its controller casts a spell this turn"
    );

    record_stoic_sphinx_spell_cast_event(&mut game, bob, 92_102);
    assert!(
        game.object_has_static_ability_id(sphinx_id, StaticAbilityId::Hexproof),
        "an opponent casting a spell should not turn off Stoic Sphinx's hexproof"
    );

    record_stoic_sphinx_spell_cast_event(&mut game, alice, 92_103);
    assert!(
        !game.object_has_static_ability_id(sphinx_id, StaticAbilityId::Hexproof),
        "Stoic Sphinx should lose hexproof after its controller has cast a spell this turn"
    );

    game.turn_store.turn_history.clear_for_new_turn();
    assert!(
        game.object_has_static_ability_id(sphinx_id, StaticAbilityId::Hexproof),
        "Stoic Sphinx should regain hexproof after turn-scoped spell-cast history clears"
    );
}

#[test]
pub(super) fn ashad_the_lone_cyberman_parses_and_renders_casualty_grant() {
    let def = parse_oracle_card_definition("Ashad, the Lone Cyberman");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("first nonlegendary artifact spell you cast each turn"),
        "Ashad should render the first nonlegendary artifact spell grant, got {rendered}"
    );
    assert!(
        rendered.contains("casualty 2"),
        "Ashad should render the granted casualty value, got {rendered}"
    );
}

pub(super) fn setup_ashad_casualty_game() -> (crate::game_state::GameState, PlayerId, PlayerId) {
    let ashad = parse_oracle_card_definition("Ashad, the Lone Cyberman");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.create_object_from_definition(&ashad, alice, Zone::Battlefield);
    (game, alice, bob)
}

pub(super) fn record_ashad_spell_cast(
    game: &mut crate::game_state::GameState,
    caster: PlayerId,
    raw_id: u32,
    name: &str,
    card_types: Vec<CardType>,
    supertypes: Vec<Supertype>,
) -> ObjectId {
    let mut builder = CardBuilder::new(CardId::from_raw(raw_id), name).card_types(card_types);
    if !supertypes.is_empty() {
        builder = builder.supertypes(supertypes);
    }
    let spell = builder.build();
    let spell_id = game.create_object_from_card(&spell, caster, Zone::Stack);
    game.push_to_stack(crate::game_state::StackEntry::new(spell_id, caster));
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(spell_id)
            .expect("recorded Ashad test spell should exist on the stack"),
        game,
    );
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new_with_snapshot(
            spell_id,
            caster,
            Zone::Hand,
            snapshot.clone(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.turn_store
        .turn_history
        .record_event(&event, Some(snapshot), None);
    spell_id
}

pub(super) fn spell_has_ashad_casualty_two(
    game: &crate::game_state::GameState,
    spell: ObjectId,
) -> bool {
    game.current_abilities(spell)
        .unwrap_or_default()
        .iter()
        .any(|ability| {
            let AbilityKind::Triggered(triggered) = &ability.kind else {
                return false;
            };
            if triggered
                .trigger
                .downcast_ref::<crate::triggers::YouCastThisSpellTrigger>()
                .is_none()
            {
                return false;
            }
            let [effect] = triggered.effects.flattened_default_effects() else {
                return false;
            };
            let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() else {
                return false;
            };
            let [sacrifice, copy, choose_targets] = may.effects.as_slice() else {
                return false;
            };
            let Some(sacrifice) = sacrifice.downcast_ref::<crate::effects::SacrificeEffect>()
            else {
                return false;
            };
            if sacrifice.player != PlayerFilter::You
                || sacrifice.count != crate::effect::Value::Fixed(1)
                || sacrifice.filter.power != Some(crate::filter::Comparison::GreaterThanOrEqual(2))
                || !sacrifice.filter.card_types.contains(&CardType::Creature)
            {
                return false;
            }
            let Some(copy) = copy.downcast_ref::<crate::effects::WithIdEffect>() else {
                return false;
            };
            if copy
                .effect
                .downcast_ref::<crate::effects::CopySpellEffect>()
                .is_none()
            {
                return false;
            }
            choose_targets
                .downcast_ref::<crate::effects::ChooseNewTargetsEffect>()
                .is_some_and(|choose| choose.from_effect == copy.id && choose.may)
        })
}

#[test]
pub(super) fn ashad_grants_casualty_to_first_nonlegendary_artifact_spell_you_cast() {
    let (mut game, alice, bob) = setup_ashad_casualty_game();

    record_ashad_spell_cast(
        &mut game,
        bob,
        93_200,
        "Bob's First Spell",
        vec![CardType::Instant],
        vec![],
    );
    let first_alice_artifact = record_ashad_spell_cast(
        &mut game,
        alice,
        93_201,
        "Alice's First Artifact",
        vec![CardType::Artifact],
        vec![],
    );
    assert!(
        spell_has_ashad_casualty_two(&game, first_alice_artifact),
        "Ashad should grant casualty 2 to the first nonlegendary artifact spell Alice casts even if Bob cast a spell first; abilities: {:#?}",
        game.current_abilities(first_alice_artifact)
    );

    let second_alice_artifact = record_ashad_spell_cast(
        &mut game,
        alice,
        93_202,
        "Alice's Second Artifact",
        vec![CardType::Artifact],
        vec![],
    );
    assert!(
        !spell_has_ashad_casualty_two(&game, second_alice_artifact),
        "Ashad should not grant casualty 2 to Alice's second spell that turn"
    );
}

#[test]
pub(super) fn ashad_ignores_prior_spells_that_do_not_match_the_granted_subject() {
    let (mut nonartifact_game, alice, _) = setup_ashad_casualty_game();
    let prior_nonartifact = record_ashad_spell_cast(
        &mut nonartifact_game,
        alice,
        93_203,
        "Prior Nonartifact Spell",
        vec![CardType::Instant],
        vec![],
    );
    assert!(
        !spell_has_ashad_casualty_two(&nonartifact_game, prior_nonartifact),
        "Ashad should not grant casualty 2 to a nonartifact spell"
    );
    let first_matching_artifact = record_ashad_spell_cast(
        &mut nonartifact_game,
        alice,
        93_204,
        "First Matching Artifact",
        vec![CardType::Artifact],
        vec![],
    );
    assert!(
        spell_has_ashad_casualty_two(&nonartifact_game, first_matching_artifact),
        "Ashad should grant casualty 2 to the first nonlegendary artifact spell even after a prior nonartifact spell"
    );

    let (mut legendary_game, alice, _) = setup_ashad_casualty_game();
    let prior_legendary_artifact = record_ashad_spell_cast(
        &mut legendary_game,
        alice,
        93_205,
        "Prior Legendary Artifact",
        vec![CardType::Artifact],
        vec![Supertype::Legendary],
    );
    assert!(
        !spell_has_ashad_casualty_two(&legendary_game, prior_legendary_artifact),
        "Ashad should not grant casualty 2 to a legendary artifact spell"
    );
    let first_nonlegendary_artifact = record_ashad_spell_cast(
        &mut legendary_game,
        alice,
        93_206,
        "First Nonlegendary Artifact",
        vec![CardType::Artifact],
        vec![],
    );
    assert!(
        spell_has_ashad_casualty_two(&legendary_game, first_nonlegendary_artifact),
        "Ashad should grant casualty 2 to the first nonlegendary artifact spell even after a prior legendary artifact spell"
    );
}

#[test]
pub(super) fn ashad_does_not_grant_casualty_to_legendary_or_nonartifact_first_spells() {
    let (mut legendary_game, alice, _) = setup_ashad_casualty_game();
    let legendary_artifact = record_ashad_spell_cast(
        &mut legendary_game,
        alice,
        93_207,
        "Legendary Artifact Probe",
        vec![CardType::Artifact],
        vec![Supertype::Legendary],
    );
    assert!(
        !spell_has_ashad_casualty_two(&legendary_game, legendary_artifact),
        "Ashad should not grant casualty 2 to a legendary artifact spell"
    );

    let (mut nonartifact_game, alice, _) = setup_ashad_casualty_game();
    let nonartifact_spell = record_ashad_spell_cast(
        &mut nonartifact_game,
        alice,
        93_208,
        "Nonartifact Spell Probe",
        vec![CardType::Instant],
        vec![],
    );
    assert!(
        !spell_has_ashad_casualty_two(&nonartifact_game, nonartifact_spell),
        "Ashad should not grant casualty 2 to a nonartifact spell"
    );
}

pub(super) fn alacrian_armory_trigger(def: &CardDefinition) -> &crate::ability::TriggeredAbility {
    def.abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Triggered(triggered) = &ability.kind else {
                return None;
            };
            Some(triggered)
        })
        .expect("Alacrian Armory should have a beginning-of-combat triggered ability")
}

pub(super) fn resolve_alacrian_armory_trigger_for_target(
    game: &mut crate::game_state::GameState,
    armory_id: ObjectId,
    controller: PlayerId,
    target: ObjectId,
    triggered: &crate::ability::TriggeredAbility,
) {
    let mut ctx = crate::effects::ExecutionContext::new_default(armory_id, controller)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: triggered
                .choices
                .first()
                .expect("Alacrian Armory should declare an optional target")
                .clone(),
            range: 0..1,
        }]);
    ctx.snapshot_targets(game);

    for effect in triggered.effects.flattened_default_effects() {
        crate::effects::execute_effect(game, effect, &mut ctx)
            .expect("Alacrian Armory combat trigger effect should resolve");
    }
}

#[derive(Default)]
pub(super) struct ChooseTopOfLibraryReplacement;

impl crate::decision::DecisionMaker for ChooseTopOfLibraryReplacement {
    fn decide_options(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        ctx.options
            .iter()
            .find(|option| option.description == "Top of library")
            .map(|option| vec![option.index])
            .unwrap_or_default()
    }
}

pub(super) fn whirlpool_whelm_mana_value_card(name: &str, mana_value: u8) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mana_value,
        )]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build()
}

pub(super) fn whirlpool_whelm_game(
    controller_top_mana_value: u8,
    opponent_top_mana_value: u8,
) -> (
    crate::game_state::GameState,
    CardDefinition,
    PlayerId,
    PlayerId,
    ObjectId,
    ObjectId,
) {
    let def = parse_oracle_card_definition("Whirlpool Whelm");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let target_def = whirlpool_whelm_mana_value_card("Bob's Target Creature", 2);
    let target = game.create_object_from_definition(&target_def, bob, Zone::Battlefield);

    let alice_top = whirlpool_whelm_mana_value_card("Alice Clash Card", controller_top_mana_value);
    let bob_top = whirlpool_whelm_mana_value_card("Bob Clash Card", opponent_top_mana_value);
    game.create_object_from_definition(&alice_top, alice, Zone::Library);
    game.create_object_from_definition(&bob_top, bob, Zone::Library);

    (game, def, alice, bob, source, target)
}

pub(super) fn resolve_whirlpool_whelm<D: crate::decision::DecisionMaker>(
    game: &mut crate::game_state::GameState,
    def: &CardDefinition,
    controller: PlayerId,
    source: ObjectId,
    target: ObjectId,
    decision_maker: &mut D,
) {
    let mut ctx = crate::effects::ExecutionContext::new(source, controller, decision_maker)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
    ctx.snapshot_targets(game);

    for effect in def
        .spell_effect
        .as_ref()
        .expect("Whirlpool Whelm should have a spell effect")
        .flattened_default_effects()
    {
        crate::effects::execute_effect(game, effect, &mut ctx)
            .expect("Whirlpool Whelm spell effect should resolve");
    }
}

#[test]
pub(super) fn whirlpool_whelm_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Whirlpool Whelm");
    let def = parse_oracle_card_definition("Whirlpool Whelm");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let debug = format!("{:#?}", def.spell_effect);

    assert!(
        rendered
            .contains("Clash with an opponent, then return target creature to its owner's hand")
            && rendered.contains(
                "If you win, you may put that creature on top of its owner's library instead"
            ),
        "Whirlpool Whelm should preserve its clash replacement wording, got {rendered}"
    );
    assert!(
        debug.contains("ClashEffect")
            && debug.contains("LocalRewriteEffect")
            && debug.contains("RegisterZoneReplacementEffect")
            && debug.contains("replacement_zone: Library"),
        "Whirlpool Whelm should structurally lower the win branch as an optional local zone replacement, got {debug}"
    );
}

#[test]
pub(super) fn whirlpool_whelm_win_branch_can_put_target_on_owners_library() {
    let (mut game, def, alice, bob, source, target) = whirlpool_whelm_game(5, 1);
    let stable_id = game
        .object(target)
        .expect("target should start on the battlefield")
        .stable_id;
    let mut decision_maker = ChooseTopOfLibraryReplacement;

    resolve_whirlpool_whelm(&mut game, &def, alice, source, target, &mut decision_maker);

    let moved = game
        .find_object_by_stable_id(stable_id)
        .expect("target should still exist after Whirlpool Whelm resolves");
    assert_eq!(
        game.object(moved).expect("moved target should exist").zone,
        Zone::Library,
        "winning the clash and choosing the replacement should put the target into its owner's library"
    );
    assert_eq!(
        game.player(bob)
            .expect("Bob should exist")
            .library
            .last()
            .copied(),
        Some(moved),
        "the replaced destination should be the top of the target owner's library"
    );
}

#[test]
pub(super) fn whirlpool_whelm_win_branch_can_decline_replacement_to_return_target() {
    let (mut game, def, alice, bob, source, target) = whirlpool_whelm_game(5, 1);
    let stable_id = game
        .object(target)
        .expect("target should start on the battlefield")
        .stable_id;
    let mut decision_maker = crate::decision::AutoPassDecisionMaker;

    resolve_whirlpool_whelm(&mut game, &def, alice, source, target, &mut decision_maker);

    let moved = game
        .find_object_by_stable_id(stable_id)
        .expect("target should still exist after Whirlpool Whelm resolves");
    assert_eq!(
        game.object(moved).expect("moved target should exist").zone,
        Zone::Hand,
        "declining the optional win replacement should return the target to its owner's hand"
    );
    assert!(
        game.player(bob)
            .expect("Bob should exist")
            .hand
            .contains(&moved),
        "the declined replacement should leave the target in its owner's hand"
    );
}

#[test]
pub(super) fn whirlpool_whelm_lost_clash_returns_target_without_replacement() {
    let (mut game, def, alice, bob, source, target) = whirlpool_whelm_game(1, 5);
    let stable_id = game
        .object(target)
        .expect("target should start on the battlefield")
        .stable_id;
    let mut decision_maker = ChooseTopOfLibraryReplacement;

    resolve_whirlpool_whelm(&mut game, &def, alice, source, target, &mut decision_maker);

    let moved = game
        .find_object_by_stable_id(stable_id)
        .expect("target should still exist after Whirlpool Whelm resolves");
    assert_eq!(
        game.object(moved).expect("moved target should exist").zone,
        Zone::Hand,
        "losing the clash should return the target without offering the library replacement"
    );
    assert!(
        game.player(bob)
            .expect("Bob should exist")
            .hand
            .contains(&moved),
        "the lost-clash branch should put the target into its owner's hand"
    );
}

pub(super) fn lumengrid_augur_activated_ability(
    def: &CardDefinition,
) -> &crate::ability::ActivatedAbility {
    def.abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Activated(activated) = &ability.kind else {
                return None;
            };
            Some(activated)
        })
        .expect("Lumengrid Augur should have an activated ability")
}

pub(super) struct LumengridAugurDiscardDecisionMaker {
    pub(super) card_to_discard: ObjectId,
}

impl crate::decision::DecisionMaker for LumengridAugurDiscardDecisionMaker {
    fn decide_objects(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if ctx
            .candidates
            .iter()
            .any(|candidate| candidate.id == self.card_to_discard && candidate.legal)
        {
            vec![self.card_to_discard]
        } else {
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(ctx.min)
                .collect()
        }
    }
}

pub(super) fn lumengrid_augur_test_card(name: &str, card_types: Vec<CardType>) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(card_types)
        .build()
}

pub(super) fn resolve_lumengrid_augur_discarding(
    discarded_card_types: Vec<CardType>,
) -> (crate::game_state::GameState, ObjectId, PlayerId) {
    let def = parse_oracle_card_definition("Lumengrid Augur");
    let activated = lumengrid_augur_activated_ability(&def);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.tap(source);

    let drawn = lumengrid_augur_test_card("Bob Drawn Card", vec![CardType::Creature]);
    game.create_object_from_definition(&drawn, bob, Zone::Library);
    let discard_card = lumengrid_augur_test_card("Bob Discarded Card", discarded_card_types);
    let discarded = game.create_object_from_definition(&discard_card, bob, Zone::Hand);
    let kept = lumengrid_augur_test_card("Bob Kept Card", vec![CardType::Creature]);
    game.create_object_from_definition(&kept, bob, Zone::Hand);

    let mut dm = LumengridAugurDiscardDecisionMaker {
        card_to_discard: discarded,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: activated
                .choices
                .first()
                .expect("Lumengrid Augur should target a player")
                .clone(),
            range: 0..1,
        }]);

    for effect in activated.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Lumengrid Augur activated ability effect should resolve");
    }

    (game, source, bob)
}

#[test]
pub(super) fn lumengrid_augur_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Lumengrid Augur");
    let def = parse_oracle_card_definition("Lumengrid Augur");
    let activated = lumengrid_augur_activated_ability(&def);
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let debug = format!("{:#?}", activated);
    let cost_debug = format!("{:?}", activated.mana_cost);

    assert!(
        activated.has_tap_cost() && cost_debug.contains("Generic(1)"),
        "Lumengrid Augur should keep its {{1}}, {{T}} activation cost, got {cost_debug}"
    );
    assert!(
        format!("{:?}", activated.choices).contains("Player"),
        "Lumengrid Augur should target a player, got {:?}",
        activated.choices
    );
    assert_eq!(
        rendered,
        "{1}, {T}: Target player draws a card, then discards a card. If that player discards an artifact card this way, untap this creature.",
        "Lumengrid Augur should render its artifact-discard untap branch"
    );
    assert!(
        debug.contains("DrawCardsEffect")
            && debug.contains("DiscardEffect")
            && debug.contains("PlayerTaggedObjectMatches")
            && debug.contains("Artifact")
            && debug.contains("UntapEffect"),
        "Lumengrid Augur should structurally lower draw-discard plus artifact-discard conditional untap, got {debug}"
    );
}

#[test]
pub(super) fn lumengrid_augur_untaps_when_target_player_discards_artifact_card() {
    let (game, source, bob) = resolve_lumengrid_augur_discarding(vec![CardType::Artifact]);

    assert!(
        !game.is_tapped(source),
        "Lumengrid Augur should untap when the target player discards an artifact card this way"
    );
    assert!(
        game.player(bob)
            .is_some_and(|player| player.graveyard.iter().any(|id| game
                .object(*id)
                .is_some_and(|obj| obj.name == "Bob Discarded Card"))),
        "the chosen artifact card should be discarded to the target player's graveyard"
    );
    assert!(
        game.player(bob)
            .is_some_and(|player| player.hand.iter().any(|id| game
                .object(*id)
                .is_some_and(|obj| obj.name == "Bob Drawn Card"))),
        "the target player should draw before discarding"
    );
}

#[test]
pub(super) fn lumengrid_augur_stays_tapped_when_target_player_discards_nonartifact_card() {
    let (game, source, bob) = resolve_lumengrid_augur_discarding(vec![CardType::Creature]);

    assert!(
        game.is_tapped(source),
        "Lumengrid Augur should stay tapped when the target player discards a nonartifact card this way"
    );
    assert!(
        game.player(bob)
            .is_some_and(|player| player.graveyard.iter().any(|id| game
                .object(*id)
                .is_some_and(|obj| obj.name == "Bob Discarded Card"))),
        "the chosen nonartifact card should still be discarded"
    );
}

#[test]
pub(super) fn alacrian_armory_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Alacrian Armory");
    let def = parse_oracle_card_definition("Alacrian Armory");
    let triggered = alacrian_armory_trigger(&def);
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let debug = format!("{:#?}", triggered);
    let choice_debug = format!("{:?}", triggered.choices);

    assert!(
        triggered.choices.len() == 1
            && choice_debug.contains("min: 0")
            && choice_debug.contains("max: Some(1)")
            && choice_debug.contains("Mount")
            && choice_debug.contains("Vehicle"),
        "Alacrian Armory should choose up to one target Mount or Vehicle, got {choice_debug}"
    );
    assert!(
        debug.contains("BecomeSaddledUntilEotEffect")
            && debug.contains("AddCardTypes")
            && debug.contains("TaggedObjectMatches"),
        "expected structural Mount/Vehicle conditional become effects, got {debug}"
    );
    assert!(
        rendered.contains("Until end of turn, that permanent becomes saddled if it's a Mount and becomes an artifact creature if it's a Vehicle"),
        "expected Alacrian Armory conditional become text to render oracle-like, got {rendered}"
    );
}

#[test]
pub(super) fn alacrian_armory_mount_branch_becomes_saddled_runtime() {
    let def = parse_oracle_card_definition("Alacrian Armory");
    let triggered = alacrian_armory_trigger(&def);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let armory_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let mount = CardDefinitionBuilder::new(CardId::from_raw(92_030), "Test Mount")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Mount])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let mount_id = game.create_object_from_definition(&mount, alice, Zone::Battlefield);

    resolve_alacrian_armory_trigger_for_target(&mut game, armory_id, alice, mount_id, triggered);

    assert!(
        game.is_saddled(mount_id),
        "a Mount target should become saddled until end of turn"
    );
    assert!(
        !game.current_has_card_type(mount_id, CardType::Artifact),
        "a non-Vehicle Mount should not gain artifact from the Vehicle branch"
    );
}

#[test]
pub(super) fn alacrian_armory_vehicle_branch_becomes_artifact_creature_runtime() {
    let def = parse_oracle_card_definition("Alacrian Armory");
    let triggered = alacrian_armory_trigger(&def);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let armory_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let vehicle = CardDefinitionBuilder::new(CardId::from_raw(92_031), "Test Vehicle")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Vehicle])
        .build();
    let vehicle_id = game.create_object_from_definition(&vehicle, alice, Zone::Battlefield);

    assert!(!game.current_has_card_type(vehicle_id, CardType::Artifact));
    assert!(!game.current_is_creature(vehicle_id));

    resolve_alacrian_armory_trigger_for_target(&mut game, armory_id, alice, vehicle_id, triggered);

    assert!(
        game.current_has_card_type(vehicle_id, CardType::Artifact)
            && game.current_is_creature(vehicle_id),
        "a Vehicle target should become an artifact creature until end of turn"
    );
    assert!(
        !game.is_saddled(vehicle_id),
        "a non-Mount Vehicle should not become saddled from the Mount branch"
    );
}

#[test]
pub(super) fn alacrian_armory_mount_vehicle_target_gets_both_branches_runtime() {
    let def = parse_oracle_card_definition("Alacrian Armory");
    let triggered = alacrian_armory_trigger(&def);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let armory_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let mount_vehicle = CardDefinitionBuilder::new(CardId::from_raw(92_032), "Test Mount Vehicle")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Mount, Subtype::Vehicle])
        .build();
    let target_id = game.create_object_from_definition(&mount_vehicle, alice, Zone::Battlefield);

    resolve_alacrian_armory_trigger_for_target(&mut game, armory_id, alice, target_id, triggered);

    assert!(
        game.is_saddled(target_id),
        "a Mount Vehicle should become saddled"
    );
    assert!(
        game.current_has_card_type(target_id, CardType::Artifact)
            && game.current_is_creature(target_id),
        "a Mount Vehicle should also become an artifact creature"
    );

    crate::turn::execute_cleanup_step(&mut game);
    assert!(
        !game.current_is_creature(target_id),
        "artifact creature effect should expire at end of turn"
    );

    game.next_turn();

    assert!(
        !game.is_saddled(target_id),
        "saddled state should expire by the next turn"
    );
}

#[test]
pub(super) fn vampire_socialite_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Vampire Socialite");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let ability_debug = format!("{:#?}", def.abilities);

    assert!(
        ability_debug.contains("OpponentLostLifeThisTurn")
            && ability_debug.contains("EnterWithCountersForFilter")
            && ability_debug.contains("intervening_if: Some"),
        "Vampire Socialite should structurally keep both opponent-life-loss gates, got {ability_debug}"
    );
    assert!(
        rendered.contains("When this creature enters, if an opponent lost life this turn, put a +1/+1 counter on each other Vampire you control."),
        "expected Vampire Socialite ETB intervening-if text, got {rendered}"
    );
    assert!(
        rendered.contains("As long as an opponent lost life this turn, each other Vampire you control enters with an additional +1/+1 counter on it."),
        "expected Vampire Socialite static conditional ETB-counter text, got {rendered}"
    );
}

pub(super) fn stage_vampire_socialite_opponent_life_loss(game: &mut crate::game_state::GameState) {
    let bob = PlayerId::from_index(1);
    let life_loss = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::life::LifeLossEvent::from_effect(bob, 1),
        crate::provenance::ProvNodeId::default(),
    );
    game.stage_turn_history_event(&life_loss);
}

#[test]
pub(super) fn vampire_socialite_etb_trigger_condition_and_counter_effect_runtime() {
    let def = parse_oracle_card_definition("Vampire Socialite");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Triggered(triggered) = &ability.kind else {
                return None;
            };
            format!("{:?}", triggered.effects)
                .contains("PutCountersEffect")
                .then_some(triggered)
        })
        .expect("Vampire Socialite should have a counter-placing ETB trigger");

    assert_eq!(
        triggered.intervening_if,
        Some(crate::effect::Condition::OpponentLostLifeThisTurn),
        "Vampire Socialite ETB trigger should be gated by opponent life loss"
    );

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let socialite_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let other_vampire = CardDefinitionBuilder::new(CardId::from_raw(92_001), "Bloodhall Trainee")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Vampire])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let other_id = game.create_object_from_definition(&other_vampire, alice, Zone::Battlefield);

    assert!(
        !crate::condition_eval::evaluate_condition_cast_time(
            &game,
            &crate::effect::Condition::OpponentLostLifeThisTurn,
            alice,
            socialite_id,
        ),
        "Vampire Socialite ETB condition should be false before an opponent loses life"
    );
    stage_vampire_socialite_opponent_life_loss(&mut game);
    assert!(
        crate::condition_eval::evaluate_condition_cast_time(
            &game,
            &crate::effect::Condition::OpponentLostLifeThisTurn,
            alice,
            socialite_id,
        ),
        "Vampire Socialite ETB condition should be true after an opponent loses life"
    );

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(socialite_id, alice, &mut dm);
    for effect in triggered.effects.flattened_default_effects() {
        effect
            .0
            .execute(&mut game, &mut ctx)
            .expect("Vampire Socialite ETB counter effect should resolve");
    }

    assert_eq!(
        game.object(other_id).and_then(|object| object
            .counters
            .get(&crate::object::CounterType::PlusOnePlusOne)
            .copied()),
        Some(1),
        "Vampire Socialite ETB trigger should put a +1/+1 counter on another Vampire"
    );
    assert_eq!(
        game.object(socialite_id).and_then(|object| object
            .counters
            .get(&crate::object::CounterType::PlusOnePlusOne)
            .copied()),
        None,
        "Vampire Socialite ETB trigger should not put a counter on itself"
    );
}

#[test]
pub(super) fn vampire_socialite_static_replacement_requires_opponent_life_loss() {
    fn entering_vampire_definition() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::from_raw(92_002), "Falkenrath Recruit")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Vampire])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    let def = parse_oracle_card_definition("Vampire Socialite");
    let alice = PlayerId::from_index(0);

    let mut inactive_game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    inactive_game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let inactive_entering = inactive_game.create_object_from_definition(
        &entering_vampire_definition(),
        alice,
        Zone::Hand,
    );
    let inactive_result = inactive_game
        .move_object_with_etb_processing(inactive_entering, Zone::Battlefield)
        .expect("inactive Vampire should enter");
    assert_eq!(
        inactive_game
            .object(inactive_result.new_id)
            .and_then(|object| object
                .counters
                .get(&crate::object::CounterType::PlusOnePlusOne)
                .copied()),
        None,
        "Vampire Socialite static replacement should not add a counter before opponent life loss"
    );

    let mut active_game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    active_game.create_object_from_definition(&def, alice, Zone::Battlefield);
    stage_vampire_socialite_opponent_life_loss(&mut active_game);
    let active_entering = active_game.create_object_from_definition(
        &entering_vampire_definition(),
        alice,
        Zone::Hand,
    );
    let active_result = active_game
        .move_object_with_etb_processing(active_entering, Zone::Battlefield)
        .expect("active Vampire should enter");
    assert_eq!(
        active_game
            .object(active_result.new_id)
            .and_then(|object| object
                .counters
                .get(&crate::object::CounterType::PlusOnePlusOne)
                .copied()),
        Some(1),
        "Vampire Socialite static replacement should add a counter after opponent life loss"
    );
}

#[test]
pub(super) fn jadar_ghoulcaller_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Jadar, Ghoulcaller of Nephalia");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Triggered(triggered) = &ability.kind else {
                return None;
            };
            format!("{:?}", triggered.effects)
                .contains("CreateTokenEffect")
                .then_some(triggered)
        })
        .expect("Jadar should have a token-creating end-step trigger");
    let condition_debug = format!("{:?}", triggered.intervening_if);

    assert!(
        condition_debug.contains("PlayerControls")
            && condition_debug.contains("Not")
            && condition_debug.contains("decayed"),
        "Jadar should structurally keep the no-creatures-with-decayed gate, got {condition_debug}"
    );
    assert!(
        rendered.contains(
            "At the beginning of your end step, if you control no creatures with decayed, create a 2/2 black Zombie creature token with decayed."
        ),
        "expected Jadar compiled text to preserve the full condition and decayed token creation, got {rendered}"
    );
}

#[test]
pub(super) fn ruin_raider_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Ruin Raider");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let ability_debug = format!("{:#?}", def.abilities);

    assert!(
        ability_debug.contains("BeginningOfEndStepTrigger")
            && ability_debug.contains("AttackedThisTurn")
            && ability_debug.contains("RevealTopEffect")
            && ability_debug.contains("MoveToZoneEffect")
            && ability_debug.contains("LoseLifeEffect")
            && ability_debug.contains("ManaValueOf"),
        "Ruin Raider should structurally keep the raid trigger, top-card reveal, hand move, and mana-value life loss, got {ability_debug}"
    );
    assert!(
        rendered.contains(
            "Raid — At the beginning of your end step, if you attacked this turn, reveal the top card of your library and put that card into your hand. You lose life equal to the card's mana value."
        ) || rendered.contains(
            "Raid — At the beginning of your end step, if you attacked this turn, reveal the top card of your library and put that card into your hand. You lose life equal to that card's mana value."
        ),
        "expected Ruin Raider compiled text to preserve the reveal-to-hand and mana-value life-loss clause, got {rendered}"
    );
}

pub(super) fn jadar_end_step_event(player: PlayerId) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfEndStepEvent::new(player),
        crate::provenance::ProvNodeId::default(),
    )
}

pub(super) fn jadar_decayed_creature_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(92_003), "Decayed Zombie")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie])
        .power_toughness(PowerToughness::fixed(2, 2))
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::keyword_marker("decayed"),
        ))
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::cant_block(),
        ))
        .build()
}

#[test]
pub(super) fn jadar_end_step_creates_decayed_zombie_when_you_control_none() {
    let def = parse_oracle_card_definition("Jadar, Ghoulcaller of Nephalia");
    let alice = PlayerId::from_index(0);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let jadar_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.turn.active_player = alice;

    let event = jadar_end_step_event(alice);
    let triggers = crate::triggers::check_triggers(&game, &event);
    assert_eq!(
        triggers
            .iter()
            .filter(|entry| entry.source == jadar_id)
            .count(),
        1,
        "Jadar should trigger when you control no creatures with decayed"
    );

    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    for trigger in triggers {
        trigger_queue.add(trigger);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Jadar trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game).expect("Jadar trigger should resolve");

    let zombie_tokens = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id).map(|object| (id, object)))
        .filter(|(_, object)| {
            matches!(object.kind, crate::object::ObjectKind::Token)
                && object.subtypes.contains(&Subtype::Zombie)
                && game.controller_of(object) == alice
        })
        .collect::<Vec<_>>();
    assert_eq!(
        zombie_tokens.len(),
        1,
        "Jadar should create one Zombie token"
    );

    let (token_id, token) = zombie_tokens[0];
    assert_eq!(game.current_power(token_id), Some(2));
    assert_eq!(game.current_toughness(token_id), Some(2));
    assert_eq!(token.colors(), ColorSet::from(Color::Black));
    assert!(
        game.object_has_static_ability_id(token_id, StaticAbilityId::CantBlock),
        "Jadar's token should have decayed's can't-block ability"
    );
    let token_abilities = format!("{:?}", token.abilities);
    assert!(
        token_abilities.contains("KeywordMarker") && token_abilities.contains("decayed"),
        "Jadar's token should carry a decayed marker for future no-decayed checks, got {token_abilities}"
    );
    assert!(
        token
            .abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Triggered(_))),
        "Jadar's token should have decayed's sacrifice trigger"
    );
}

#[test]
pub(super) fn jadar_end_step_does_not_trigger_while_you_control_decayed_creature() {
    let def = parse_oracle_card_definition("Jadar, Ghoulcaller of Nephalia");
    let alice = PlayerId::from_index(0);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let jadar_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.create_object_from_definition(
        &jadar_decayed_creature_definition(),
        alice,
        Zone::Battlefield,
    );
    game.turn.active_player = alice;

    let event = jadar_end_step_event(alice);
    let triggers = crate::triggers::check_triggers(&game, &event);
    assert_eq!(
        triggers
            .iter()
            .filter(|entry| entry.source == jadar_id)
            .count(),
        0,
        "Jadar should not queue its intervening-if trigger while you control a decayed creature"
    );
}

#[test]
pub(super) fn jadar_end_step_trigger_does_not_create_token_if_condition_fails_on_resolution() {
    let def = parse_oracle_card_definition("Jadar, Ghoulcaller of Nephalia");
    let alice = PlayerId::from_index(0);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let jadar_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.turn.active_player = alice;

    let event = jadar_end_step_event(alice);
    let triggers = crate::triggers::check_triggers(&game, &event);
    assert_eq!(
        triggers
            .iter()
            .filter(|entry| entry.source == jadar_id)
            .count(),
        1,
        "Jadar should initially queue when you control no decayed creatures"
    );

    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    for trigger in triggers {
        trigger_queue.add(trigger);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Jadar trigger should go on the stack");
    game.create_object_from_definition(
        &jadar_decayed_creature_definition(),
        alice,
        Zone::Battlefield,
    );
    crate::game_loop::resolve_stack_entry(&mut game).expect("Jadar trigger should resolve");

    let jadar_created_tokens = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id))
        .filter(|object| {
            matches!(object.kind, crate::object::ObjectKind::Token)
                && object.subtypes.contains(&Subtype::Zombie)
                && game.controller_of(object) == alice
        })
        .count();
    assert_eq!(
        jadar_created_tokens, 0,
        "Jadar should not create a token if the decayed-creature condition is false on resolution"
    );
}

#[test]
pub(super) fn party_dude_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Party Dude");
    let ability_debug = format!("{:#?}", def.abilities);
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert!(
        ability_debug.contains("PlayersAttackedTrigger")
            || ability_debug.contains("PlayersAttackedOneOrMore"),
        "Party Dude should parse the opponent-attacked trigger strictly, got {ability_debug}"
    );
    assert!(
        ability_debug.contains("WithCount")
            && ability_debug.contains("ChoiceCount")
            && ability_debug.contains("Hand")
            && ability_debug.contains("WhereXIs"),
        "Party Dude should structurally keep the hand-size X pump and up-to-one target, got {ability_debug}"
    );
    assert!(
        rendered.contains("{1}{G}: Level 2")
            && rendered.contains("{4}{G}: Level 3")
            && rendered.contains("Whenever one or more of your opponents are attacked")
            && rendered.contains("up to one target attacking creature gets +X/+X until end of turn, where X is the number of cards in your hand"),
        "expected Party Dude compiled text to preserve class level and opponent-attacked pump clauses, got {rendered}"
    );
}

#[test]
pub(super) fn case_of_the_shattered_pact_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Case of the Shattered Pact");
    let ability_debug = format!("{:#?}", def.abilities);
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert!(
        ability_debug.contains("SearchLibraryEffect")
            && ability_debug.contains("CaseToSolve")
            && ability_debug.contains("CaseSolved")
            && ability_debug.contains("SolveCaseEffect")
            && !ability_debug.contains("PutCountersEffect"),
        "Case of the Shattered Pact should strictly parse its ETB search, solve trigger, and solved trigger labels, got {ability_debug}"
    );
    assert!(
        rendered.contains("When this Case enters, search your library for a basic land card, reveal it, put it into your hand, then shuffle.")
            && rendered.contains("To solve — There are five colors among permanents you control.")
            && rendered.contains("Solved — At the beginning of combat on your turn, target creature you control gains flying, double strike, and vigilance until end of turn."),
        "expected Case of the Shattered Pact compiled text to preserve its Case clauses, got {rendered}"
    );
}

#[test]
pub(super) fn rootpath_purifier_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Rootpath Purifier");
    let ability_debug = format!("{:#?}", def.abilities);
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        def.abilities.iter().any(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => {
                static_ability.id() == StaticAbilityId::AddSupertypes
            }
            _ => false,
        }),
        "Rootpath Purifier should parse its basic-supertype static ability strictly, got {ability_debug}"
    );
    assert!(
        ability_debug.contains("AddSupertypes")
            && ability_debug.contains("Basic")
            && ability_debug.contains("Library")
            && ability_debug.contains("Battlefield"),
        "Rootpath Purifier should structurally add Basic to battlefield lands and library land cards, got {ability_debug}"
    );
    assert!(
        rendered.contains("Lands you control")
            && rendered.contains("land cards in your library")
            && rendered.contains("are basic"),
        "Rootpath Purifier compiled text should cover the full basic-supertype clause, got {rendered}"
    );
}

#[test]
pub(super) fn rootpath_purifier_makes_only_your_lands_and_library_land_cards_basic() {
    let rootpath = parse_oracle_card_definition("Rootpath Purifier");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);

    let alice_battlefield_land = CardBuilder::new(CardId::new(), "Alice Battlefield Land")
        .card_types(vec![CardType::Land])
        .build();
    let bob_battlefield_land = CardBuilder::new(CardId::new(), "Bob Battlefield Land")
        .card_types(vec![CardType::Land])
        .build();
    let alice_library_land = CardBuilder::new(CardId::new(), "Alice Library Land")
        .card_types(vec![CardType::Land])
        .build();
    let alice_library_spell = CardBuilder::new(CardId::new(), "Alice Library Spell")
        .card_types(vec![CardType::Instant])
        .build();
    let bob_library_land = CardBuilder::new(CardId::new(), "Bob Library Land")
        .card_types(vec![CardType::Land])
        .build();

    game.create_object_from_definition(&rootpath, alice, Zone::Battlefield);
    let alice_battlefield_land_id =
        game.create_object_from_card(&alice_battlefield_land, alice, Zone::Battlefield);
    let bob_battlefield_land_id =
        game.create_object_from_card(&bob_battlefield_land, bob, Zone::Battlefield);
    let alice_library_land_id =
        game.create_object_from_card(&alice_library_land, alice, Zone::Library);
    let alice_library_spell_id =
        game.create_object_from_card(&alice_library_spell, alice, Zone::Library);
    let bob_library_land_id = game.create_object_from_card(&bob_library_land, bob, Zone::Library);

    let is_basic = |game: &crate::game_state::GameState, id| {
        game.current_characteristics(id)
            .expect("object should have current characteristics")
            .supertypes
            .contains(&Supertype::Basic)
    };

    assert!(
        is_basic(&game, alice_battlefield_land_id),
        "Rootpath Purifier should make lands you control basic"
    );
    assert!(
        is_basic(&game, alice_library_land_id),
        "Rootpath Purifier should make land cards in your library basic"
    );
    assert!(
        !is_basic(&game, bob_battlefield_land_id),
        "Rootpath Purifier should not make lands opponents control basic"
    );
    assert!(
        !is_basic(&game, alice_library_spell_id),
        "Rootpath Purifier should not make nonland cards in your library basic"
    );
    assert!(
        !is_basic(&game, bob_library_land_id),
        "Rootpath Purifier should not make land cards in opponents' libraries basic"
    );
}

pub(super) fn kin_tree_nurturer_endure_effect(def: &CardDefinition) -> &Effect {
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Kin-Tree Nurturer should compile an enters trigger");
    let effects = triggered.effects.flattened_default_effects();
    let [effect] = effects else {
        panic!("Kin-Tree Nurturer should have exactly one endure effect, got {effects:#?}");
    };
    effect
}

pub(super) fn pious_kitsune_upkeep_effects(def: &CardDefinition) -> &[Effect] {
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Pious Kitsune should compile an upkeep trigger");
    triggered.effects.flattened_default_effects()
}

pub(super) fn pious_kitsune_life_activated_ability(
    def: &CardDefinition,
) -> &crate::ability::ActivatedAbility {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Pious Kitsune should compile a life-gain activated ability")
}

#[test]
pub(super) fn pious_kitsune_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Pious Kitsune");
    let ability_debug = format!("{:#?}", def.abilities);
    let compiled = unprocessed_compiled_lines(&def);
    let rendered = compiled.join("\n");
    let oracle = oracle_text_by_name()
        .get("Pious Kitsune")
        .expect("Pious Kitsune oracle text")
        .clone();
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        crate::semantic_compare::compare_semantics_scored(
            &oracle,
            &compiled,
            crate::semantic_compare::report_embedding_config(),
        );

    assert!(
        ability_debug.contains("ValueComparison")
            && ability_debug.contains("eight-and-a-half-tails")
            && ability_debug.contains("CountersOnSource")
            && ability_debug.contains("\"devotion\"")
            && ability_debug.contains("RemoveCountersEffect"),
        "Pious Kitsune should structurally keep the named-creature condition, devotion-counter life scaling, and activated counter cost, got {ability_debug}"
    );
    assert!(
        rendered.contains("if a creature named eight-and-a-half-tails is on the battlefield")
            && rendered.contains("gain 1 life for each devotion counter on this creature")
            && rendered.contains("Remove a devotion counter from this creature"),
        "expected Pious Kitsune compiled text to preserve named-creature condition and devotion counter clauses, got {rendered}"
    );
    assert!(
        similarity >= 0.99 && !mismatch,
        "expected Pious Kitsune semantic comparison to clear target, score={similarity}, mismatch={mismatch}, compiled={compiled:?}"
    );
}

#[test]
pub(super) fn pious_kitsune_upkeep_gains_life_when_named_creature_is_on_battlefield() {
    let def = parse_oracle_card_definition("Pious Kitsune");
    let effects = pious_kitsune_upkeep_effects(&def);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.add_counters(source, CounterType::Named("devotion"), 1)
        .expect("Pious Kitsune should accept devotion counters");
    let named_creature = CardDefinitionBuilder::new(CardId::new(), "Eight-and-a-Half-Tails")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_definition(&named_creature, bob, Zone::Battlefield);

    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    for effect in effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Pious Kitsune upkeep effect should resolve");
    }

    assert_eq!(
        game.counter_count(source, CounterType::Named("devotion")),
        2,
        "upkeep trigger should put a devotion counter on Pious Kitsune first"
    );
    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        22,
        "named creature on any battlefield should enable life gain equal to devotion counters"
    );
}

#[test]
pub(super) fn pious_kitsune_upkeep_skips_life_gain_without_named_creature() {
    let def = parse_oracle_card_definition("Pious Kitsune");
    let effects = pious_kitsune_upkeep_effects(&def);
    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.add_counters(source, CounterType::Named("devotion"), 1)
        .expect("Pious Kitsune should accept devotion counters");

    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    for effect in effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Pious Kitsune upkeep effect should resolve");
    }

    assert_eq!(
        game.counter_count(source, CounterType::Named("devotion")),
        2,
        "upkeep trigger should put a devotion counter even when condition is false"
    );
    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        20,
        "life-gain branch should not happen without Eight-and-a-Half-Tails on the battlefield"
    );
}

#[test]
pub(super) fn pious_kitsune_activated_ability_removes_devotion_counter_and_gains_life() {
    let def = parse_oracle_card_definition("Pious Kitsune");
    let activated = pious_kitsune_life_activated_ability(&def);
    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.remove_summoning_sickness(source);
    game.add_counters(source, CounterType::Named("devotion"), 1)
        .expect("Pious Kitsune should accept devotion counters");

    crate::cost::can_pay_cost(&game, source, alice, &activated.mana_cost).expect(
        "Pious Kitsune activation should be payable with an untapped source and a devotion counter",
    );
    let mut dm = crate::decision::AutoPassDecisionMaker::default();
    crate::special_actions::pay_total_cost_with_choice(
        &mut game,
        alice,
        source,
        &activated.mana_cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut dm,
    )
    .expect("Pious Kitsune activation cost should be paid");

    assert!(
        game.is_tapped(source),
        "activation cost should tap Pious Kitsune"
    );
    assert_eq!(
        game.counter_count(source, CounterType::Named("devotion")),
        0,
        "activation cost should remove one devotion counter"
    );

    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    for effect in activated.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Pious Kitsune activated ability effect should resolve");
    }

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        21,
        "activated ability should gain 1 life after its counter-removal cost is paid"
    );
}

#[test]
pub(super) fn tromp_the_domains_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Tromp the Domains");
    let effect_debug = format!("{:#?}", def.spell_effect);
    let compiled = unprocessed_compiled_lines(&def);
    let rendered = compiled.join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        effect_debug.contains("ApplyContinuousEffect")
            && effect_debug.contains("AddAbility")
            && effect_debug.contains("Trample")
            && effect_debug.contains("ModifyPowerToughness")
            && effect_debug.contains("BasicLandTypesAmong"),
        "Tromp the Domains should structurally grant trample and scale P/T by domain, got {effect_debug}"
    );
    assert!(
        rendered_lower.contains("creatures you control")
            && rendered_lower.contains("gain trample")
            && rendered_lower
                .contains("get +1/+1 for each basic land type among lands you control"),
        "expected Tromp the Domains compiled text to preserve the domain P/T clause, got {rendered}"
    );
}

pub(super) fn bumi_modal_effect(def: &CardDefinition) -> &ChooseModeEffect {
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Bumi, King of Three Trials should compile an enters trigger");
    triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<ChooseModeEffect>())
        .expect("Bumi, King of Three Trials should compile one modal choice effect")
}

pub(super) fn bumi_game_with_lessons(
    lesson_count: usize,
) -> (
    CardDefinition,
    crate::game_state::GameState,
    PlayerId,
    PlayerId,
    ObjectId,
) {
    let def = parse_oracle_card_definition("Bumi, King of Three Trials");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let lesson = CardDefinitionBuilder::new(CardId::new(), "Bumi Test Lesson")
        .card_types(vec![CardType::Sorcery])
        .subtypes(vec![Subtype::Lesson])
        .build();

    for _ in 0..lesson_count {
        game.create_object_from_definition(&lesson, alice, Zone::Graveyard);
    }

    (def, game, alice, bob, source)
}

pub(super) fn bumi_earthbend_target_assignment() -> crate::game_state::TargetAssignment {
    crate::game_state::TargetAssignment {
        spec: ChooseSpec::target(ChooseSpec::Object(
            crate::filter::ObjectFilter::land().you_control(),
        )),
        range: 0..1,
    }
}

#[test]
pub(super) fn parse_oracle_bumi_king_of_three_trials_strict_parser_text_and_structure_regression() {
    assert_oracle_card_parses_strict("Bumi, King of Three Trials");

    let def = parse_oracle_card_definition("Bumi, King of Three Trials");
    let modal = bumi_modal_effect(&def);
    let modal_debug = format!("{modal:#?}");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let canonical = canonical_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains(
            "When Bumi enters, choose up to X, where X is the number of Lesson cards in your graveyard"
        ),
        "expected Bumi compiled text to keep the Lesson-based modal X clause, got {rendered}"
    );
    assert!(
        rendered.contains("Target player scries 3") && rendered.contains("Earthbend 3"),
        "expected Bumi compiled text to keep the targeted scry and earthbend modes, got {rendered}"
    );
    assert!(
        canonical.contains(
            "When Bumi enters, choose up to X, where X is the number of Lesson cards in your graveyard —\n•"
        ) && !canonical.contains("—.\n•"),
        "Bumi canonical compiled text should not add a period after the modal-header dash, got {canonical}"
    );
    assert_eq!(modal.min_choose_count, Value::Fixed(0));
    assert_eq!(modal.modes.len(), 3);
    assert!(
        modal_debug.contains("WhereXIs")
            && modal_debug.contains("Lesson")
            && modal_debug.contains("TargetOnlyEffect")
            && modal_debug.contains("ScryEffect")
            && modal_debug.contains("EarthbendEffect"),
        "Bumi should structurally model dynamic Lesson mode count, targeted scry, and earthbend, got {modal_debug}"
    );
}

#[test]
pub(super) fn bumi_zero_lesson_graveyard_cannot_apply_selected_counter_mode() {
    let (def, mut game, alice, _bob, source) = bumi_game_with_lessons(0);
    let modal = bumi_modal_effect(&def).clone();
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice)
        .with_chosen_modes(Some(vec![0]));

    modal
        .execute(&mut game, &mut ctx)
        .expect("Bumi with zero Lesson cards should resolve without choosing modes");

    assert_eq!(
        game.counter_count(source, CounterType::PlusOnePlusOne),
        0,
        "zero Lesson cards in graveyard should make Bumi choose up to zero modes"
    );
}

#[test]
pub(super) fn bumi_counter_mode_uses_lesson_based_modal_bound() {
    let (def, mut game, alice, _bob, source) = bumi_game_with_lessons(1);
    let modal = bumi_modal_effect(&def).clone();
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice)
        .with_chosen_modes(Some(vec![0]));

    modal
        .execute(&mut game, &mut ctx)
        .expect("Bumi should allow one mode with one Lesson card in graveyard");

    assert_eq!(
        game.counter_count(source, CounterType::PlusOnePlusOne),
        3,
        "counter mode should put three +1/+1 counters on Bumi"
    );
}

#[test]
pub(super) fn bumi_target_player_scry_mode_targets_the_chosen_player() {
    let (def, mut game, alice, bob, source) = bumi_game_with_lessons(1);
    let modal = bumi_modal_effect(&def).clone();
    let library_card = CardDefinitionBuilder::new(CardId::new(), "Bumi Scry Card")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    game.create_object_from_definition(&library_card, bob, Zone::Library);
    game.create_object_from_definition(&library_card, bob, Zone::Library);
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice)
        .with_chosen_modes(Some(vec![1]))
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: ChooseSpec::target_player(),
            range: 0..1,
        }]);

    let outcome = modal
        .execute(&mut game, &mut ctx)
        .expect("Bumi targeted scry mode should resolve");

    let scry_event = outcome
        .events_of_type::<crate::events::KeywordActionEvent>()
        .find(|event| event.action == crate::events::KeywordActionKind::Scry)
        .expect("targeted scry mode should emit a scry keyword action event");
    assert_eq!(scry_event.player, bob);
    assert_eq!(
        scry_event.amount, 2,
        "targeted scry should apply to Bob's two-card library"
    );
}

#[test]
pub(super) fn bumi_earthbend_mode_targets_a_land_you_control() {
    let (def, mut game, alice, _bob, source) = bumi_game_with_lessons(1);
    let modal = bumi_modal_effect(&def).clone();
    let land = CardDefinitionBuilder::new(CardId::new(), "Bumi Test Land")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Forest])
        .build();
    let land_id = game.create_object_from_definition(&land, alice, Zone::Battlefield);
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice)
        .with_chosen_modes(Some(vec![2]))
        .with_targets(vec![crate::effects::ResolvedTarget::Object(land_id)])
        .with_target_assignments(vec![bumi_earthbend_target_assignment()]);

    modal
        .execute(&mut game, &mut ctx)
        .expect("Bumi earthbend mode should resolve with a controlled land target");

    assert!(
        game.current_is_creature(land_id),
        "earthbend should make the targeted land a creature"
    );
    assert_eq!(game.counter_count(land_id, CounterType::PlusOnePlusOne), 3);
    assert_eq!(game.calculated_power(land_id), Some(3));
    assert_eq!(game.calculated_toughness(land_id), Some(3));
}

#[test]
pub(super) fn bumi_earthbend_mode_is_illegal_without_a_controlled_land_target() {
    let (def, mut game, alice, _bob, source) = bumi_game_with_lessons(1);
    let modal = bumi_modal_effect(&def).clone();
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice)
        .with_chosen_modes(Some(vec![2]));

    let err = modal
        .execute(&mut game, &mut ctx)
        .expect_err("Bumi earthbend mode should be illegal with no land you control");
    assert!(
        format!("{err:?}").contains("Selected mode is not legal"),
        "expected earthbend target legality failure, got {err:?}"
    );
}

#[test]
pub(super) fn kin_tree_nurturer_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Kin-Tree Nurturer");
    let ability_debug = format!("{:#?}", def.abilities);
    let compiled = unprocessed_compiled_lines(&def);
    let rendered = compiled.join("\n");
    let oracle = oracle_text_by_name()
        .get("Kin-Tree Nurturer")
        .expect("Kin-Tree Nurturer oracle text")
        .clone();
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        crate::semantic_compare::compare_semantics_scored(
            &oracle,
            &compiled,
            Some(crate::semantic_compare::EmbeddingConfig {
                dims: 384,
                mismatch_threshold: 0.99,
            }),
        );

    assert!(
        ability_debug.contains("ChooseModeEffect")
            && ability_debug.contains("PutCountersEffect")
            && ability_debug.contains("CreateTokenEffect")
            && ability_debug.contains("Spirit"),
        "Kin-Tree Nurturer should structurally lower endure to counter-or-Spirit modes, got {ability_debug}"
    );
    assert!(
        rendered.contains("Lifelink")
            && rendered.contains("When this creature enters, it endures 1."),
        "expected Kin-Tree Nurturer compiled text to preserve lifelink and endure, got {rendered}"
    );
    assert!(
        similarity >= 0.99 && !mismatch,
        "expected Kin-Tree Nurturer semantic comparison to clear target, score={similarity}, mismatch={mismatch}, compiled={compiled:?}"
    );
}

#[test]
pub(super) fn typed_leaf_regressions_preserve_endure_counter_counts_and_plural_phasing() {
    for name in [
        "Sinkhole Surveyor",
        "Hamza, Guardian of Arashin",
        "Time and Tide",
    ] {
        assert_oracle_card_parses_strict(name);
    }

    let sinkhole = parse_oracle_card_definition("Sinkhole Surveyor");
    let sinkhole_lines = unprocessed_compiled_lines(&sinkhole);
    assert!(
        sinkhole_lines.iter().any(|line| line
            == "Whenever this creature attacks, lose 1 life and this creature endures 1."),
        "source-reference surface hints must not expand endure into a modal block: {sinkhole_lines:?}"
    );

    let hamza = parse_oracle_card_definition("Hamza, Guardian of Arashin");
    let hamza_lines = unprocessed_compiled_lines(&hamza);
    let hamza_debug = format!("{:#?}", hamza.abilities);
    assert_eq!(
        hamza_lines,
        vec![
            "This spell costs {1} less to cast for each creature you control with a +1/+1 counter on it.".to_string(),
            "Creature spells you cast cost {1} less to cast for each creature you control with a +1/+1 counter on it.".to_string(),
        ]
    );
    assert!(
        hamza_debug.matches("amount: Count(").count() >= 2
            && hamza_debug.matches("with_counter: Some(").count() >= 2,
        "both Hamza reductions must retain their typed countered-creature counts: {hamza_debug}"
    );

    let time_and_tide = parse_oracle_card_definition("Time and Tide");
    let time_and_tide_lines = unprocessed_compiled_lines(&time_and_tide);
    assert_eq!(
        time_and_tide_lines,
        vec![
            "Simultaneously, all phased-out creatures phase in and all creatures with phasing phase out."
                .to_string(),
        ],
        "the comma-bearing simultaneous prefix must preserve both plural phase subjects"
    );
}

#[test]
pub(super) fn kin_tree_nurturer_endure_counter_mode_puts_counter_on_it() {
    let def = parse_oracle_card_definition("Kin-Tree Nurturer");
    let effect = kin_tree_nurturer_endure_effect(&def);
    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice)
        .with_chosen_modes(Some(vec![0]));

    crate::effects::execute_effect(&mut game, effect, &mut ctx)
        .expect("Kin-Tree Nurturer endure counter mode should resolve");

    assert_eq!(
        game.counter_count(source, CounterType::PlusOnePlusOne),
        1,
        "counter mode should put one +1/+1 counter on Kin-Tree Nurturer"
    );
    assert!(
        game.objects_in_zone(Zone::Battlefield)
            .into_iter()
            .filter(|&id| id != source)
            .filter_map(|id| game.object(id))
            .all(|object| object.name != "Spirit"),
        "counter mode should not create the Spirit token branch"
    );
}

#[test]
pub(super) fn kin_tree_nurturer_endure_token_mode_creates_white_spirit() {
    let def = parse_oracle_card_definition("Kin-Tree Nurturer");
    let effect = kin_tree_nurturer_endure_effect(&def);
    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice)
        .with_chosen_modes(Some(vec![1]));

    crate::effects::execute_effect(&mut game, effect, &mut ctx)
        .expect("Kin-Tree Nurturer endure token mode should resolve");

    assert_eq!(
        game.counter_count(source, CounterType::PlusOnePlusOne),
        0,
        "token mode should not put the counter branch on Kin-Tree Nurturer"
    );
    let spirits = game
        .objects_in_zone(Zone::Battlefield)
        .into_iter()
        .filter(|&id| id != source)
        .filter_map(|id| game.object(id))
        .filter(|object| {
            object.name == "Spirit"
                && object.card_types == [CardType::Creature]
                && object.subtypes == [Subtype::Spirit]
                && object.color_override == Some(crate::color::ColorSet::WHITE)
                && object.base_power == Some(crate::card::PtValue::Fixed(1))
                && object.base_toughness == Some(crate::card::PtValue::Fixed(1))
                && game.controller_of(object) == alice
        })
        .count();
    assert_eq!(spirits, 1, "token mode should create one 1/1 white Spirit");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_day_of_the_moon_goads_creatures_with_chosen_name() {
    let def = parse_oracle_card_definition("Day of the Moon");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Day of the Moon should compile to a saga chapter trigger");
    let effects = &triggered.effects.segments[0].default_effects;

    let choose_name = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ChooseCardNameEffect>())
        .expect("Day of the Moon should choose a card name before goading");
    assert_eq!(choose_name.tag.as_str(), "__chosen_name__");

    let goad = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::GoadEffect>())
        .expect("Day of the Moon should goad the matching creatures");
    let ChooseSpec::All(filter) = &goad.target else {
        panic!("expected Day of the Moon to goad all matching creatures, got {goad:#?}");
    };
    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert!(
        filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == "__chosen_name__"
                && matches!(
                    constraint.relation,
                    crate::filter::TaggedOpbjectRelation::SameNameAsTagged
                )
        }),
        "goad target should match the just-chosen name, got {filter:#?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_robe_of_the_archmagi_compiled_text_includes_class_equip_clause() {
    let def = parse_oracle_card_definition("Robe of the Archmagi");
    let lines = canonical_compiled_lines(&def);

    assert!(
        lines
            .iter()
            .any(|line| line == "Whenever equipped creature deals combat damage to a player, you draw that many cards."),
        "expected Robe trigger text in compiled output, got {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line == "Equip {4}"),
        "expected base equip line in compiled output, got {lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "Equip Shaman or Warlock or Wizard {1}"),
        "expected class-qualified equip line in compiled output, got {lines:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn day_of_the_moon_chapter_resolution_goads_only_chosen_name() {
    struct ChooseMemnite;

    impl crate::decision::DecisionMaker for ChooseMemnite {
        fn decide_text(
            &mut self,
            _game: &crate::game_state::GameState,
            _ctx: &crate::decisions::context::TextInputContext,
        ) -> String {
            "Memnite".to_string()
        }
    }

    let def = parse_oracle_card_definition("Day of the Moon");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Day of the Moon should compile to a saga chapter trigger");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let memnite = CardDefinitionBuilder::new(CardId::from_raw(91_001), "Memnite")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let vanguard = CardDefinitionBuilder::new(CardId::from_raw(91_002), "Elite Vanguard")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 1))
        .build();
    let memnite_id = game.create_object_from_definition(&memnite, bob, Zone::Battlefield);
    let vanguard_id = game.create_object_from_definition(&vanguard, bob, Zone::Battlefield);

    let mut dm = ChooseMemnite;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &triggered.effects,
        None,
        &[],
    )
    .expect("chapter ability should resolve");

    assert!(game.is_goaded(memnite_id));
    assert!(!game.is_goaded(vanguard_id));

    game.next_turn();
    let combat = crate::combat_state::CombatState::default();
    let decision = crate::game_loop::get_declare_attackers_decision(&game, &combat);
    let crate::decisions::context::DecisionContext::Attackers(attackers) = decision else {
        panic!("expected attackers decision");
    };
    let memnite_option = attackers
        .attacker_options
        .iter()
        .find(|option| option.creature == memnite_id)
        .expect("Memnite should be able to attack Bob's combat");
    assert!(
        memnite_option.must_attack,
        "chosen-name goad should make Memnite a required attacker"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_public_enemy_oracle_and_compiled_text() {
    let def = parse_oracle_card_definition("Public Enemy");
    let rendered = canonical_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains("Enchant creature"),
        "Public Enemy should keep its Aura restriction, got {rendered}"
    );
    assert!(
        rendered
            .contains("All creatures attack enchanted creature's controller each combat if able."),
        "Public Enemy should render the required attack-player clause, got {rendered}"
    );
    assert!(
        rendered.contains("When enchanted creature dies, draw a card.")
            || rendered.contains("Whenever enchanted creature dies, draw a card."),
        "Public Enemy should keep its death trigger, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn public_enemy_requires_legal_attackers_to_attack_enchanted_creatures_controller() {
    let public_enemy = parse_oracle_card_definition("Public Enemy");
    let creature = CardDefinitionBuilder::new(CardId::from_raw(91_101), "Grizzly Bears")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );

    let enchanted_creature =
        game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    let aura = game.create_object_from_definition(&public_enemy, bob, Zone::Battlefield);
    assert!(game.attach_object_to_target(
        aura,
        crate::object::AttachmentTarget::Object(enchanted_creature),
    ));
    let attacker = game.create_object_from_definition(&creature, charlie, Zone::Battlefield);
    game.remove_summoning_sickness(attacker);
    game.turn.active_player = charlie;

    let combat = crate::combat_state::CombatState::default();
    let options = crate::decision::compute_legal_attackers(&game, &combat);
    let attacker_option = options
        .iter()
        .find(|option| option.creature == attacker)
        .expect("Charlie's creature should be attack-capable");
    assert!(
        attacker_option.must_attack,
        "Public Enemy should make creatures attack the enchanted creature's controller if able"
    );
    assert_eq!(
        attacker_option.valid_targets,
        vec![crate::combat_state::AttackTarget::Player(alice)],
        "Public Enemy should require attacking the enchanted creature's controller, not another player"
    );

    let mut combat = crate::combat_state::CombatState::default();
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    let wrong_target = [crate::AttackerDeclaration {
        creature: attacker,
        target: crate::combat_state::AttackTarget::Player(bob),
    }];
    assert!(
        crate::game_loop::apply_attacker_declarations(
            &mut game,
            &mut combat,
            &mut trigger_queue,
            &wrong_target,
        )
        .is_err(),
        "attacking a different player should be illegal while the enchanted creature's controller is attackable"
    );

    let correct_target = [crate::AttackerDeclaration {
        creature: attacker,
        target: crate::combat_state::AttackTarget::Player(alice),
    }];
    crate::game_loop::apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &correct_target,
    )
    .expect("attacking the enchanted creature's controller should be legal");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn public_enemy_does_not_force_attack_when_enchanted_creatures_controller_cant_be_attacked()
 {
    let public_enemy = parse_oracle_card_definition("Public Enemy");
    let creature = CardDefinitionBuilder::new(CardId::from_raw(91_102), "Grizzly Bears")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );

    let enchanted_creature =
        game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    let aura = game.create_object_from_definition(&public_enemy, bob, Zone::Battlefield);
    assert!(game.attach_object_to_target(
        aura,
        crate::object::AttachmentTarget::Object(enchanted_creature),
    ));
    let attacker = game.create_object_from_definition(&creature, charlie, Zone::Battlefield);
    game.remove_summoning_sickness(attacker);
    game.effect_store
        .cant_effects
        .add_cant_attack_defenders(attacker, [alice]);
    game.turn.active_player = charlie;

    let combat = crate::combat_state::CombatState::default();
    let options = crate::decision::compute_legal_attackers(&game, &combat);
    let attacker_option = options
        .iter()
        .find(|option| option.creature == attacker)
        .expect("Charlie's creature should still be able to attack someone else");
    assert!(
        !attacker_option.must_attack,
        "Public Enemy should not force an attack when its required player is not attackable"
    );
    assert!(
        !attacker_option
            .valid_targets
            .contains(&crate::combat_state::AttackTarget::Player(alice)),
        "the restricted enchanted creature's controller should not be a legal target"
    );
    assert!(
        attacker_option
            .valid_targets
            .contains(&crate::combat_state::AttackTarget::Player(bob)),
        "the creature may still attack other legal players"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_shiny_impetus_oracle_and_compiled_text() {
    let def = parse_oracle_card_definition("Shiny Impetus");
    let rendered_lines = canonical_compiled_lines(&def);
    let rendered = rendered_lines.join("\n");
    let ability_debug = format!("{:#?}", def.abilities);

    assert_eq!(
        rendered_lines,
        vec![
            "Enchant creature".to_string(),
            "Enchanted creature gets +2/+2 and is goaded.".to_string(),
            "Whenever enchanted creature attacks, create a Treasure token.".to_string(),
        ],
        "Shiny Impetus should keep its exact compiled oracle shape, got {rendered}"
    );
    assert!(
        ability_debug.contains("Anthem")
            && ability_debug.contains("AttachedGoadedBySourceController")
            && ability_debug.contains("CreateTokenEffect"),
        "Shiny Impetus should structurally model anthem, goad, and Treasure trigger, got {ability_debug}"
    );
}

#[test]
pub(super) fn ordeal_of_erebos_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Ordeal of Erebos");
    let rendered = canonical_compiled_lines(&def).join("\n");
    let ability_debug = format!("{:#?}", def.abilities);

    assert!(
        rendered.contains("Enchant creature"),
        "Ordeal of Erebos should keep its Aura restriction, got {rendered}"
    );
    assert!(
        rendered.contains("Whenever enchanted creature attacks, put a +1/+1 counter on it"),
        "Ordeal of Erebos should render the enchanted-creature attack trigger, got {rendered}"
    );
    assert!(
        rendered.contains("+1/+1 counters on it"),
        "Ordeal of Erebos should render the counter threshold clause, got {rendered}"
    );
    assert!(
        rendered.contains("target player discards two cards"),
        "Ordeal of Erebos should render its sacrifice trigger discard branch, got {rendered}"
    );
    assert!(
        ability_debug.contains("PutCountersEffect")
            && ability_debug.contains("ValueComparison")
            && ability_debug.contains("CountersOn")
            && ability_debug.contains("GreaterThanOrEqual")
            && ability_debug.contains("SacrificeTargetEffect")
            && ability_debug.contains("PlayerSacrificesTrigger")
            && ability_debug.contains("DiscardEffect"),
        "Ordeal of Erebos should structurally model counters, threshold sacrifice, and discard trigger, got {ability_debug}"
    );
}

#[test]
pub(super) fn unbound_flourishing_strict_parser_compiled_text_and_model_regression() {
    let text = "Mana cost: {2}{G}\n\
Type: Enchantment\n\
Whenever you cast a permanent spell with a mana cost that contains {X}, double the value of X.\n\
Whenever you cast an instant or sorcery spell or activate an ability, if that spell's mana cost or that ability's activation cost contains {X}, copy that spell or ability. You may choose new targets for the copy.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(88_101), "Unbound Flourishing")
        .parse_text(text)
        .expect("Unbound Flourishing should strict-parse");

    let rendered = compiled_text_lines(&def).join("\n");
    assert_eq!(
        rendered,
        "Whenever you cast a permanent spell with a mana cost that contains {X}, double the value of X.\nWhenever you cast an instant or sorcery spell or activate an ability, if that spell's mana cost or that ability's activation cost contains {X}, copy that spell or ability. You may choose new targets for the copy."
    );

    let debug = format!("{def:#?}");
    assert!(debug.contains("ScaleXValueEffect"), "{debug}");
    assert!(debug.contains("target: Tagged"), "{debug}");
    assert!(debug.contains("\"triggering\""), "{debug}");
    assert!(debug.contains("CopySpellEffect"), "{debug}");
    assert!(debug.contains("AbilityActivatedTrigger"), "{debug}");
    assert!(debug.contains("has_x_in_cost: true"), "{debug}");
}

#[test]
pub(super) fn haunting_wind_or_trigger_preserves_without_tap_cost_condition() {
    let oracle = "Whenever an artifact becomes tapped or a player activates an artifact's ability without {T} in its activation cost, this enchantment deals 1 damage to that artifact's controller.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Haunting Wind")
        .card_types(vec![CardType::Enchantment])
        .parse_text(oracle)
        .expect("Haunting Wind trigger should parse");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert_eq!(rendered, oracle);

    let debug = format!("{def:#?}");
    assert!(debug.contains("OrTrigger"), "{debug}");
    assert!(debug.contains("PermanentBecomesTappedTrigger"), "{debug}");
    assert!(debug.contains("AbilityActivatedTrigger"), "{debug}");
    let compact_debug: String = debug.chars().filter(|ch| !ch.is_whitespace()).collect();
    assert!(
        compact_debug.contains("activation_cost_has_tap:Some(false"),
        "{debug}"
    );
}

#[test]
pub(super) fn haunting_wind_runtime_trigger_matches_tapped_artifact_or_no_tap_cost_ability_only() {
    let oracle = "Whenever an artifact becomes tapped or a player activates an artifact's ability without {T} in its activation cost, this enchantment deals 1 damage to that artifact's controller.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Haunting Wind")
        .card_types(vec![CardType::Enchantment])
        .parse_text(oracle)
        .expect("Haunting Wind trigger should parse");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let haunting_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let artifact_card = CardBuilder::new(CardId::new(), "Artifact Source")
        .card_types(vec![CardType::Artifact])
        .build();
    let artifact_id = game.create_object_from_card(&artifact_card, bob, Zone::Battlefield);

    let no_tap_cost_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::AbilityActivatedEvent::new(artifact_id, bob, false)
            .with_activation_cost_has_tap(false),
        crate::provenance::ProvNodeId::default(),
    );
    let no_tap_matches = crate::triggers::check_triggers(&game, &no_tap_cost_event)
        .into_iter()
        .filter(|entry| entry.source == haunting_id)
        .count();
    assert_eq!(
        no_tap_matches, 1,
        "expected no-tap-cost artifact ability to trigger"
    );

    let tap_cost_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::AbilityActivatedEvent::new(artifact_id, bob, false)
            .with_activation_cost_has_tap(true),
        crate::provenance::ProvNodeId::default(),
    );
    let tap_cost_matches = crate::triggers::check_triggers(&game, &tap_cost_event)
        .into_iter()
        .filter(|entry| entry.source == haunting_id)
        .count();
    assert_eq!(
        tap_cost_matches, 0,
        "artifact abilities with {{T}} in their activation cost should not satisfy the ability branch"
    );

    let tapped_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::PermanentTappedEvent::new(artifact_id),
        crate::provenance::ProvNodeId::default(),
    );
    let tapped_matches = crate::triggers::check_triggers(&game, &tapped_event)
        .into_iter()
        .filter(|entry| entry.source == haunting_id)
        .count();
    assert_eq!(
        tapped_matches, 1,
        "expected artifact tapped branch to trigger"
    );
}

pub(super) struct OrdealTargetBob {
    pub(super) bob: PlayerId,
}

impl crate::decision::DecisionMaker for OrdealTargetBob {
    fn decide_targets(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<crate::game_state::Target> {
        ctx.requirements
            .iter()
            .filter_map(|requirement| {
                requirement
                    .legal_targets
                    .iter()
                    .find_map(|target| match target {
                        crate::game_state::Target::Player(player) if *player == self.bob => {
                            Some(target.clone())
                        }
                        _ => None,
                    })
            })
            .take(1)
            .collect()
    }
}

pub(super) fn ordeal_of_erebos_game(
    starting_counters: u32,
    bob_hand_size: u32,
) -> (
    crate::game_state::GameState,
    PlayerId,
    PlayerId,
    ObjectId,
    ObjectId,
) {
    let ordeal = parse_oracle_card_definition("Ordeal of Erebos");
    let creature = CardDefinitionBuilder::new(CardId::from_raw(91_124), "Grizzly Bears")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let discard_card = CardDefinitionBuilder::new(CardId::from_raw(91_125), "Discard Fodder")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );

    let enchanted_creature = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
    let aura = game.create_object_from_definition(&ordeal, alice, Zone::Battlefield);
    assert!(
        game.object(aura)
            .is_some_and(|object| object.subtypes.contains(&Subtype::Aura)
                && object.aura_attach_filter.is_some()),
        "parsed Ordeal should enter the runtime scenario as an Aura with an enchant restriction"
    );
    assert!(
        game.calculated_subtypes(aura).contains(&Subtype::Aura),
        "calculated Ordeal characteristics should preserve the Aura subtype before attachment"
    );
    assert!(game.attach_object_to_target(
        aura,
        crate::object::AttachmentTarget::Object(enchanted_creature),
    ));
    game.add_counters(
        enchanted_creature,
        CounterType::PlusOnePlusOne,
        starting_counters,
    );
    for _ in 0..bob_hand_size {
        game.create_object_from_definition(&discard_card, bob, Zone::Hand);
    }
    game.remove_summoning_sickness(enchanted_creature);
    game.turn.active_player = bob;
    assert_eq!(charlie, PlayerId::from_index(2));

    (game, alice, bob, aura, enchanted_creature)
}

pub(super) fn resolve_ordeal_attack_trigger(
    game: &mut crate::game_state::GameState,
    enchanted_creature: ObjectId,
    decision_maker: &mut impl crate::decision::DecisionMaker,
) {
    let charlie = PlayerId::from_index(2);
    let mut combat = crate::combat_state::CombatState::default();
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    let attack = [crate::AttackerDeclaration {
        creature: enchanted_creature,
        target: crate::combat_state::AttackTarget::Player(charlie),
    }];
    crate::game_loop::apply_attacker_declarations(game, &mut combat, &mut trigger_queue, &attack)
        .expect("enchanted creature should be able to attack");
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Ordeal of Erebos should queue exactly one attack trigger"
    );
    crate::game_loop::put_triggers_on_stack(game, &mut trigger_queue)
        .expect("Ordeal of Erebos attack trigger should go on the stack");
    game.turn.priority_player = Some(game.turn.active_player);
    crate::game_loop::run_priority_loop_with(game, &mut trigger_queue, decision_maker)
        .expect("Ordeal of Erebos attack trigger should resolve through priority");
}

#[test]
pub(super) fn ordeal_of_erebos_below_threshold_keeps_aura_attached() {
    let (mut game, _alice, bob, aura, enchanted_creature) = ordeal_of_erebos_game(1, 2);
    let bob_hand_before = game.player(bob).expect("Bob should exist").hand.len();
    let mut dm = OrdealTargetBob { bob };

    resolve_ordeal_attack_trigger(&mut game, enchanted_creature, &mut dm);

    assert_eq!(
        game.counter_count(enchanted_creature, CounterType::PlusOnePlusOne),
        2,
        "attack trigger should put one +1/+1 counter on the enchanted creature"
    );
    assert_eq!(
        game.object(aura).expect("Aura should remain").zone,
        Zone::Battlefield,
        "Aura should remain on the battlefield below the three-counter threshold"
    );
    assert_eq!(
        game.object(aura).and_then(|object| object.attached_to),
        Some(crate::object::AttachmentTarget::Object(enchanted_creature)),
        "Aura should stay attached below the threshold"
    );
    assert_eq!(
        game.player(bob).expect("Bob should exist").hand.len(),
        bob_hand_before,
        "below-threshold attack should not trigger the discard branch"
    );
}

#[test]
pub(super) fn ordeal_of_erebos_threshold_sacrifices_aura_and_discards_two() {
    let (mut game, _alice, bob, aura, enchanted_creature) = ordeal_of_erebos_game(2, 3);
    let mut dm = OrdealTargetBob { bob };

    resolve_ordeal_attack_trigger(&mut game, enchanted_creature, &mut dm);

    assert_eq!(
        game.counter_count(enchanted_creature, CounterType::PlusOnePlusOne),
        3,
        "attack trigger should put the third +1/+1 counter on the enchanted creature"
    );
    assert!(
        !game.battlefield.contains(&aura)
            && game.objects_in_zone(Zone::Graveyard).into_iter().any(|id| {
                game.object(id)
                    .is_some_and(|object| object.name == "Ordeal of Erebos")
            }),
        "Aura should be sacrificed once the enchanted creature has three +1/+1 counters"
    );

    assert_eq!(
        game.player(bob).expect("Bob should exist").hand.len(),
        1,
        "target player should discard exactly two cards"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_equipment_attached_goaded_anthem_preserves_equipped_subject() {
    let def = parse_oracle_card_definition("Bloodthirsty Blade");
    let rendered_lines = canonical_compiled_lines(&def);
    let rendered = rendered_lines.join("\n");
    let ability_debug = format!("{:#?}", def.abilities);

    assert!(
        rendered_lines
            .iter()
            .any(|line| line == "Equipped creature gets +2/+0 and is goaded."),
        "Bloodthirsty Blade should keep the Equipment subject in compiled text, got {rendered}"
    );
    assert!(
        !rendered.contains("Enchanted creature is goaded"),
        "Equipment goad text should not render as enchanted, got {rendered}"
    );
    assert!(
        ability_debug.contains("Anthem")
            && ability_debug.contains("AttachedGoadedBySourceController"),
        "Bloodthirsty Blade should structurally model equipment anthem and goad, got {ability_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn shiny_impetus_buffs_and_goads_enchanted_creature_away_from_aura_controller() {
    let shiny_impetus = parse_oracle_card_definition("Shiny Impetus");
    let creature = CardDefinitionBuilder::new(CardId::from_raw(91_120), "Grizzly Bears")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );

    let enchanted_creature = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
    let aura = game.create_object_from_definition(&shiny_impetus, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(
        aura,
        crate::object::AttachmentTarget::Object(enchanted_creature),
    ));
    game.remove_summoning_sickness(enchanted_creature);
    game.turn.active_player = bob;

    assert_eq!(game.current_power(enchanted_creature), Some(4));
    assert_eq!(game.current_toughness(enchanted_creature), Some(4));
    assert!(
        game.is_goaded(enchanted_creature),
        "Shiny Impetus should make the enchanted creature goaded"
    );

    let combat = crate::combat_state::CombatState::default();
    let options = crate::decision::compute_legal_attackers(&game, &combat);
    let attacker_option = options
        .iter()
        .find(|option| option.creature == enchanted_creature)
        .expect("enchanted creature should be attack-capable");
    assert!(
        attacker_option.must_attack,
        "goaded enchanted creature should be required to attack"
    );
    assert!(
        !attacker_option
            .valid_targets
            .contains(&crate::combat_state::AttackTarget::Player(alice)),
        "goaded creature should not attack the Aura controller while another player is attackable"
    );
    assert!(
        attacker_option
            .valid_targets
            .contains(&crate::combat_state::AttackTarget::Player(charlie)),
        "goaded creature should attack a non-goading player when able"
    );

    let mut combat = crate::combat_state::CombatState::default();
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    let wrong_target = [crate::AttackerDeclaration {
        creature: enchanted_creature,
        target: crate::combat_state::AttackTarget::Player(alice),
    }];
    assert!(
        crate::game_loop::apply_attacker_declarations(
            &mut game,
            &mut combat,
            &mut trigger_queue,
            &wrong_target,
        )
        .is_err(),
        "attacking the Aura controller should be illegal while a non-goading player is attackable"
    );

    let correct_target = [crate::AttackerDeclaration {
        creature: enchanted_creature,
        target: crate::combat_state::AttackTarget::Player(charlie),
    }];
    let mut combat = crate::combat_state::CombatState::default();
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &correct_target,
    )
    .expect("attacking a non-goading player should be legal");
}
