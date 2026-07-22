#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_conditional_spell_cost_if_it_targets_compiles_target_filter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Conditional Cost Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "This spell costs {3} less to cast if it targets a tapped creature.\nDestroy target creature.",
        )
        .expect("conditional spell-cost clause should parse");

    let static_abilities = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        !static_abilities.is_empty(),
        "expected at least one static ability for conditional spell cost, got {static_abilities:?}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("if it targets tapped creature"),
        "expected tapped-target condition in rendered cost reduction, got {rendered}"
    );
    assert!(
        !rendered.contains("spells cost {3} less to cast"),
        "unconditional cost reduction text should not be rendered, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_conditional_spell_cost_if_you_ve_cast_instant_or_sorcery_this_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Storm Condition Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This spell costs {2} less to cast if you've cast an instant or sorcery spell this turn.\nFlying",
        )
        .expect("cast-history conditional spell-cost clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("if you've cast an instant or sorcery spell this turn"),
        "expected instant-or-sorcery cast-history condition in rendered text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_madness_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Madness Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Madness {1}{R}")
        .expect("madness keyword line should parse");

    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Madness { cost } => {
            assert_eq!(cost.to_oracle(), "{1}{R}");
        }
        other => panic!("expected madness alternative cast, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_devoid_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Devoid Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Devoid")
        .expect("devoid keyword line should parse");

    let has_make_colorless = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::MakeColorless
        )
    });
    assert!(
        has_make_colorless,
        "expected devoid to compile to a make-colorless static ability"
    );

    let devoid_ability = def
        .abilities
        .iter()
        .find(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id() == StaticAbilityId::MakeColorless
            )
        })
        .expect("expected to find devoid ability");
    assert!(
        devoid_ability.functions_in(&Zone::Hand)
            && devoid_ability.functions_in(&Zone::Library)
            && devoid_ability.functions_in(&Zone::Battlefield)
            && devoid_ability.functions_in(&Zone::Stack)
            && devoid_ability.functions_in(&Zone::Graveyard)
            && devoid_ability.functions_in(&Zone::Exile)
            && devoid_ability.functions_in(&Zone::Command),
        "devoid should function in all zones"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" | ");
    assert!(
        rendered.contains("Devoid"),
        "expected compiled text to include Devoid, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_landwalk_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Swampwalk Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Swampwalk")
        .expect("swampwalk keyword line should parse");

    let has_landwalk = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::Landwalk
        )
    });
    assert!(has_landwalk, "expected swampwalk to compile to Landwalk");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_nonbasic_landwalk_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Nonbasic Landwalk Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Nonbasic landwalk")
        .expect("nonbasic landwalk keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" | ");
    assert!(
        rendered.contains("Nonbasic landwalk"),
        "expected compiled text to preserve nonbasic landwalk, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_snow_landwalk_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Snow Forestwalk Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Snow forestwalk")
        .expect("snow forestwalk keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" | ");
    assert!(
        rendered.contains("Snow Forestwalk"),
        "expected compiled text to preserve snow forestwalk, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_keyword_bundle_compacts_landwalk_variants() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Keyword Bundle Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("First strike\nForestwalk\nVigilance")
        .expect("keyword bundle should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" | ");
    assert_eq!(
        rendered, "First strike, forestwalk, vigilance",
        "expected keyword markers to compact into one bundle, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_cant_be_blocked_by_more_than_one_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Max Blockers Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't be blocked by more than one creature.")
        .expect("max-blockers line should parse");

    let has_max_blockers = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::CantBeBlockedByMoreThan
        )
    });
    assert!(
        has_max_blockers,
        "expected max-blockers text to compile to static ability"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn rampaging_cyclops_parses_blocker_count_static_condition() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rampaging Cyclops")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text("This creature gets -2/-0 as long as two or more creatures are blocking it.")
        .expect("Rampaging Cyclops should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert_eq!(
        rendered, "This creature gets -2/-0 as long as two or more creatures are blocking it.",
        "expected Rampaging Cyclops compiled text to preserve its blocker-count condition"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("CountComparison") && debug.contains("BlockingSource"),
        "expected Rampaging Cyclops to lower to a structural blocking-source count condition, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_each_creature_cant_be_blocked_by_more_than_one_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Familiar Ground Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Each creature can't be blocked by more than one creature.")
        .expect("global max-blockers line should parse");

    let has_grant = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability) if static_ability.grants_abilities()
        )
    });
    assert!(
        has_grant,
        "expected Familiar Ground-style line to compile to an ability-granting static ability"
    );
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.starts_with("Each creature can't be blocked by more than 1 creature"),
        "expected the quantified grant subject and direct restriction surface, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_each_creature_with_counter_cant_be_blocked_by_more_than_one_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Counter Max Blockers Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Each creature you control with a +1/+1 counter on it can't be blocked by more than one creature.")
        .expect("filtered max-blockers line should parse");

    let has_grant = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability) if static_ability.grants_abilities()
        )
    });
    assert!(
        has_grant,
        "expected filtered max-blockers line to compile to an ability-granting static ability"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_each_creature_can_block_additional_creature_each_combat() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "High Ground Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Each creature can block an additional creature each combat.")
        .expect("global can-block-additional line should parse");

    let has_grant = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability) if static_ability.grants_abilities()
        )
    });
    assert!(
        has_grant,
        "expected High Ground-style line to compile to an ability-granting static ability"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_trigger_becomes_targeted_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Phantasmal Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature becomes the target of a spell or ability, sacrifice it.")
        .expect("becomes-targeted trigger should parse");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");
    assert!(
        triggered.trigger.display().contains("becomes the target"),
        "expected becomes-targeted trigger display, got {}",
        triggered.trigger.display()
    );
    let debug = format!("{:#?}", triggered.effects);
    assert!(
        debug.contains("SacrificeTargetEffect"),
        "expected direct sacrifice-target lowering for 'sacrifice it', got {debug}"
    );
    assert!(
        !debug.contains("ChooseObjectsEffect"),
        "unexpected chooser scaffolding for 'sacrifice it': {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_assign_damage_as_unblocked_with_this_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Thorn Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "You may have this creature assign its combat damage as though it weren't blocked.",
        )
        .expect("assign-as-unblocked wording with 'this creature' should parse");

    let has_assign_as_unblocked = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::MayAssignDamageAsUnblocked
        )
    });
    assert!(
        has_assign_as_unblocked,
        "expected static may-assign-damage-as-unblocked ability"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_assign_damage_as_unblocked_with_enchanted_creature_controller() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Indomitable Might Probe")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant creature\nEnchanted creature gets +3/+3.\nEnchanted creature's controller may have it assign its combat damage as though it weren't blocked.",
        )
        .expect("assign-as-unblocked wording with enchanted creature's controller should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("enchanted creature gets +3/+3"),
        "expected aura buff to be preserved, got {rendered}"
    );
    assert!(
        rendered.contains("assign its combat damage as though it weren't blocked"),
        "expected enchanted creature grant to include assign-as-unblocked text, got {rendered}"
    );

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("MayAssignDamageAsUnblocked"),
        "expected lowered aura effect to include MayAssignDamageAsUnblocked, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_first_spell_cost_modifier_marker_errors() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "First Spell Cost Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("The first creature spell you cast each turn costs {2} less to cast.")
        .expect_err("first-spell cost marker should fail parsing");
    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("unsupported first-spell cost modifier mechanic"),
        "expected explicit unsupported first-spell marker error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_other_anthem_subject_keeps_other() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Other Anthem Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Other Soldier creatures you control get +0/+1")
        .expect("parse other-anthem line");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" | ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("other soldier creatures you control get +0/+1"),
        "expected other-anthem text in compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_other_anthem_subject_rejects_temporary() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Other Anthem Reject Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Other Soldier creatures you control get +0/+1 until end of turn")
        .expect("parse temporary other-anthem line");
    assert!(
        def.abilities.is_empty(),
        "expected temporary other-anthem line to avoid static abilities, got {:?}",
        def.abilities
    );
    assert!(
        def.spell_effect.is_some(),
        "expected temporary other-anthem line to compile as a spell effect"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_peek_targets_player_hand() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Peek Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Look at target player's hand.\nDraw a card.")
        .expect("parse peek probe");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" | ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("look at target player's hand"),
        "expected target player wording in compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_peek_targets_opponent_hand() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Opponent Peek Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature enters, look at target opponent's hand.")
        .expect("parse look-at-opponent-hand trigger");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" | ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("look at target opponent's hand"),
        "expected target opponent wording in compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[derive(Debug)]
pub(super) struct SpyNetworkViewCall {
    pub(super) viewer: PlayerId,
    pub(super) subject: PlayerId,
    pub(super) zone: Zone,
    pub(super) cards: Vec<ObjectId>,
}

#[cfg(ironsmith_runtime_parser_tests)]
#[derive(Debug, Default)]
pub(super) struct SpyNetworkCaptureDm {
    pub(super) calls: Vec<SpyNetworkViewCall>,
    pub(super) order_calls: Vec<Vec<ObjectId>>,
    pub(super) reverse_order: bool,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl crate::decision::DecisionMaker for SpyNetworkCaptureDm {
    fn decide_order(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        let mut order = ctx.items.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        self.order_calls.push(order.clone());
        if self.reverse_order {
            order.reverse();
        }
        order
    }

    fn view_cards(
        &mut self,
        _game: &crate::game_state::GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        ctx: &crate::decisions::context::ViewCardsContext,
    ) {
        self.calls.push(SpyNetworkViewCall {
            viewer,
            subject: ctx.subject,
            zone: ctx.zone,
            cards: cards.to_vec(),
        });
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[derive(Debug)]
pub(super) struct SpyNetworkResolution {
    pub(super) game: crate::game_state::GameState,
    pub(super) dm: SpyNetworkCaptureDm,
    pub(super) bob_hand: ObjectId,
    pub(super) bob_library: ObjectId,
    pub(super) alice_library: Vec<ObjectId>,
    pub(super) face_up: ObjectId,
    pub(super) face_down: ObjectId,
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn spy_network_oracle_text() -> &'static str {
    concat!(
        "Look at target player's hand, the top card of that player's library, ",
        "and any face-down creatures they control. Look at the top four cards ",
        "of your library, then put them back in any order."
    )
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn spy_network_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(1), "Spy Network")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Instant])
        .parse_text(spy_network_oracle_text())
        .expect("Spy Network should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn spy_network_test_card(name: &str, card_types: Vec<CardType>) -> crate::card::Card {
    let is_creature = card_types.contains(&CardType::Creature);
    let mut builder = crate::card::CardBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(card_types);
    if is_creature {
        builder = builder.power_toughness(PowerToughness::fixed(2, 2));
    }
    builder.build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn spy_network_add_card(
    game: &mut crate::game_state::GameState,
    owner: PlayerId,
    zone: Zone,
    name: &str,
    card_types: Vec<CardType>,
) -> ObjectId {
    let card = spy_network_test_card(name, card_types);
    game.create_object_from_card(&card, owner, zone)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn spy_network_resolve(
    def: &CardDefinition,
    include_face_down: bool,
    reverse_order: bool,
) -> SpyNetworkResolution {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let bob_hand = spy_network_add_card(
        &mut game,
        bob,
        Zone::Hand,
        "Hand Card",
        vec![CardType::Instant],
    );
    let bob_library = spy_network_add_card(
        &mut game,
        bob,
        Zone::Library,
        "Library Card",
        vec![CardType::Sorcery],
    );
    let mut alice_library = Vec::new();
    for idx in 0..4 {
        alice_library.push(spy_network_add_card(
            &mut game,
            alice,
            Zone::Library,
            &format!("Alice Library Card {idx}"),
            vec![CardType::Instant],
        ));
    }
    let face_up = spy_network_add_card(
        &mut game,
        bob,
        Zone::Battlefield,
        "Face-Up Creature",
        vec![CardType::Creature],
    );
    let face_down = spy_network_add_card(
        &mut game,
        bob,
        Zone::Battlefield,
        "Face-Down Creature",
        vec![CardType::Creature],
    );
    if include_face_down {
        game.set_face_down(face_down);
    }

    let source = game.create_object_from_definition(def, alice, Zone::Stack);
    let mut dm = SpyNetworkCaptureDm {
        reverse_order,
        ..Default::default()
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)]);
    for effect in def
        .spell_effect
        .as_ref()
        .expect("Spy Network spell effects")
    {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Spy Network effect should resolve");
    }

    assert!(
        !dm.calls
            .iter()
            .any(|call| call.zone == Zone::Battlefield && call.cards.contains(&face_up)),
        "Spy Network should not show face-up creatures as part of the face-down clause"
    );
    SpyNetworkResolution {
        game,
        dm,
        bob_hand,
        bob_library,
        alice_library,
        face_up,
        face_down,
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn spy_network_parses_strictly_and_compiles_look_clauses() {
    let def = spy_network_definition();
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert_eq!(rendered, spy_network_oracle_text());

    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(debug.contains("LookAtHandEffect"), "{debug}");
    assert!(debug.contains("LookAtTopCardsEffect"), "{debug}");
    assert!(debug.contains("LookAtObjectsEffect"), "{debug}");
    assert!(debug.contains("ReorderLibraryTopEffect"), "{debug}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn spy_network_runtime_views_hand_library_and_face_down_creatures() {
    let def = spy_network_definition();
    let result = spy_network_resolve(&def, true, false);
    let dm = &result.dm;
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut alice_top_four = result.alice_library.clone();
    alice_top_four.reverse();

    assert!(
        dm.calls.iter().any(|call| call.viewer == alice
            && call.subject == bob
            && call.zone == Zone::Hand
            && call.cards == vec![result.bob_hand]),
        "Spy Network should show the target player's hand, got {:?}",
        dm.calls
    );
    assert!(
        dm.calls.iter().any(|call| call.viewer == alice
            && call.subject == bob
            && call.zone == Zone::Library
            && call.cards == vec![result.bob_library]),
        "Spy Network should show the top card of the target player's library, got {:?}",
        dm.calls
    );
    assert!(
        dm.calls.iter().any(|call| call.viewer == alice
            && call.subject == alice
            && call.zone == Zone::Library
            && call.cards == alice_top_four),
        "Spy Network should show the top four cards of your library, got {:?}",
        dm.calls
    );
    assert!(
        dm.calls.iter().any(|call| call.viewer == alice
            && call.subject == bob
            && call.zone == Zone::Battlefield
            && call.cards == vec![result.face_down]),
        "Spy Network should show target player's face-down creatures, got {:?}",
        dm.calls
    );
    assert!(
        !dm.calls
            .iter()
            .any(|call| call.cards.contains(&result.face_up)),
        "Spy Network should not show face-up creatures, got {:?}",
        dm.calls
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn spy_network_runtime_without_face_down_creatures_still_views_hand_and_library() {
    let def = spy_network_definition();
    let result = spy_network_resolve(&def, false, false);
    let dm = &result.dm;
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut alice_top_four = result.alice_library.clone();
    alice_top_four.reverse();

    assert!(
        dm.calls.iter().any(|call| call.viewer == alice
            && call.subject == bob
            && call.zone == Zone::Hand
            && call.cards == vec![result.bob_hand]),
        "Spy Network should still show the target player's hand when there are no face-down creatures, got {:?}",
        dm.calls
    );
    assert!(
        dm.calls.iter().any(|call| call.viewer == alice
            && call.subject == bob
            && call.zone == Zone::Library
            && call.cards == vec![result.bob_library]),
        "Spy Network should still show the target player's top library card when there are no face-down creatures, got {:?}",
        dm.calls
    );
    assert!(
        dm.calls.iter().any(|call| call.viewer == alice
            && call.subject == alice
            && call.zone == Zone::Library
            && call.cards == alice_top_four),
        "Spy Network should still show your top four library cards when there are no face-down creatures, got {:?}",
        dm.calls
    );
    assert!(
        !dm.calls.iter().any(|call| call.zone == Zone::Battlefield),
        "Spy Network should not create a battlefield view when no face-down creatures match, got {:?}",
        dm.calls
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn spy_network_runtime_reorders_your_top_four_library_cards() {
    let def = spy_network_definition();
    let result = spy_network_resolve(&def, false, true);
    let alice = PlayerId::from_index(0);
    let mut original_top_four = result.alice_library.clone();
    original_top_four.reverse();
    let mut expected_top_four = original_top_four.clone();
    expected_top_four.reverse();

    assert_eq!(
        result.dm.order_calls,
        vec![original_top_four],
        "Spy Network should ask you to reorder exactly the four looked-at library cards"
    );

    let actual_top_four = result
        .game
        .player(alice)
        .expect("Alice exists")
        .library
        .iter()
        .rev()
        .take(4)
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(
        actual_top_four, expected_top_four,
        "Spy Network should put the looked-at cards back in the chosen order"
    );
}

#[derive(Debug)]
pub(super) struct SmokeTellerViewCall {
    pub(super) viewer: PlayerId,
    pub(super) subject: PlayerId,
    pub(super) zone: Zone,
    pub(super) cards: Vec<ObjectId>,
}

#[derive(Debug, Default)]
pub(super) struct SmokeTellerCaptureDm {
    pub(super) calls: Vec<SmokeTellerViewCall>,
}

impl crate::decision::DecisionMaker for SmokeTellerCaptureDm {
    fn view_cards(
        &mut self,
        _game: &crate::game_state::GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        ctx: &crate::decisions::context::ViewCardsContext,
    ) {
        self.calls.push(SmokeTellerViewCall {
            viewer,
            subject: ctx.subject,
            zone: ctx.zone,
            cards: cards.to_vec(),
        });
    }
}

pub(super) fn smoke_teller_creature(name: &str) -> crate::card::Card {
    crate::card::CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

pub(super) fn smoke_teller_add_creature(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
    name: &str,
) -> ObjectId {
    let card = smoke_teller_creature(name);
    game.create_object_from_card(&card, controller, Zone::Battlefield)
}

pub(super) fn smoke_teller_activated_effects(def: &CardDefinition) -> &[Effect] {
    let ability = def
        .abilities
        .first()
        .expect("Smoke Teller activated ability");
    let AbilityKind::Activated(activated) = &ability.kind else {
        panic!("Smoke Teller should have an activated ability, got {ability:?}");
    };
    activated
        .effects
        .segments
        .first()
        .expect("Smoke Teller resolution segment")
        .default_effects
        .as_slice()
}

#[test]
pub(super) fn smoke_teller_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Smoke Teller");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert_eq!(rendered, "{1}{U}: Look at target face-down creature.");

    let debug = format!("{:#?}", smoke_teller_activated_effects(&def));
    assert!(debug.contains("TargetOnlyEffect"), "{debug}");
    assert!(debug.contains("LookAtObjectsEffect"), "{debug}");
}

#[test]
pub(super) fn smoke_teller_targets_and_views_only_the_selected_face_down_creature() {
    let def = parse_oracle_card_definition("Smoke Teller");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let face_down = smoke_teller_add_creature(&mut game, bob, "Face-Down Creature");
    let face_up = smoke_teller_add_creature(&mut game, bob, "Face-Up Creature");
    game.set_face_down(face_down);

    let choice = def
        .abilities
        .first()
        .and_then(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated.choices.first(),
            _ => None,
        })
        .expect("Smoke Teller should require a target");
    let ChooseSpec::Target(inner) = choice else {
        panic!("Smoke Teller should use a target object choice, got {choice:?}");
    };
    let ChooseSpec::Object(filter) = inner.as_ref() else {
        panic!("Smoke Teller should target a filtered object, got {choice:?}");
    };
    let filter_ctx = crate::filter::FilterContext::default();
    assert!(
        filter.matches(
            game.object(face_down).expect("face-down object"),
            &filter_ctx,
            &game
        ),
        "target filter should allow face-down creatures"
    );
    assert!(
        !filter.matches(
            game.object(face_up).expect("face-up object"),
            &filter_ctx,
            &game
        ),
        "target filter should reject face-up creatures"
    );

    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let mut dm = SmokeTellerCaptureDm::default();
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(face_down)]);
    for effect in smoke_teller_activated_effects(&def) {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Smoke Teller target and look effects should resolve");
    }

    assert_eq!(dm.calls.len(), 1, "expected one view call, got {dm:?}");
    assert_eq!(dm.calls[0].viewer, alice);
    assert_eq!(dm.calls[0].subject, alice);
    assert_eq!(dm.calls[0].zone, Zone::Battlefield);
    assert_eq!(dm.calls[0].cards, vec![face_down]);
    assert!(!dm.calls[0].cards.contains(&face_up));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_keeper_of_the_mind_target_condition_survives_rendering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Keeper of the Mind Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{U}, {T}: Choose target opponent who has at least two more cards in hand than you do as you activate this ability. Draw a card.",
        )
        .expect("parse Keeper of the Mind ability");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("CardsInHandAtLeastMoreThanYou")
            && debug.contains("base: Opponent")
            && debug.contains("TargetOnlyEffect")
            && debug.contains("DrawCardsEffect"),
        "expected a hand-size-gated opponent target followed by draw, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" | ")
        .to_ascii_lowercase();
    assert!(
        rendered
            .contains("choose target opponent who has at least two more cards in hand than you do")
            && rendered.contains("draw a card"),
        "expected Keeper target condition in compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_keeper_of_the_flame_life_target_condition_parses_and_renders() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Keeper of the Flame")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red], vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{R}, {T}: Choose target opponent who has more life than you do as you activate this ability. This creature deals 2 damage to that player.",
        )
        .expect("parse Keeper of the Flame ability");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("HasMoreLifeThanYou")
            && debug.contains("base: Opponent")
            && debug.contains("TargetOnlyEffect")
            && debug.contains("DealDamageEffect"),
        "expected a life-gated opponent target followed by damage, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" | ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "choose target opponent who has more life than you do as you activate this ability"
        ) && rendered.contains("this creature deals 2 damage to that player"),
        "expected Keeper of the Flame target condition and damage text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_untap_another_target_permanent_rendering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Untap Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Untap another target permanent.")
        .expect("parse untap probe");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" | ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("another target permanent"),
        "expected 'another target permanent' in compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_counter_unless_pays_rendering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Counter Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target spell unless its controller pays {1}.")
        .expect("parse counter probe");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" | ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("counter target spell unless its controller pays {1}"),
        "expected counter-unless-pays text in compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn frightful_delusion_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Frightful Delusion");
    let spell_debug = format!("{:#?}", def.spell_effect);
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        def.spell_effect.is_some(),
        "Frightful Delusion should parse as a strict spell"
    );
    assert!(
        spell_debug.contains("UnlessPaysEffect")
            && spell_debug.contains("CounterEffect")
            && spell_debug.contains("DiscardEffect")
            && spell_debug.contains("ControllerOf(")
            && !spell_debug.contains("IteratedPlayer"),
        "expected counter-unless-pay followed by target controller discard, got {spell_debug}"
    );
    assert!(
        rendered.contains("Counter target spell unless its controller pays {1}")
            && rendered.contains("That player discards a card"),
        "expected Frightful Delusion counter-unless and discard text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn thassas_intervention_strict_parser_and_twice_x_counter_text_regression() {
    assert_oracle_card_parses_strict("Thassa's Intervention");

    let def = parse_oracle_card_definition("Thassa's Intervention");
    let spell_debug = format!("{:#?}", def.spell_effect);
    let spell_compact_debug = format!("{:?}", def.spell_effect);
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        spell_debug.contains("ChooseModeEffect")
            && spell_debug.contains("UnlessPaysEffect")
            && spell_debug.contains("CounterEffect")
            && spell_compact_debug.contains("multiplier: Some(Fixed(2))"),
        "expected Thassa's Intervention to lower twice-X counter mode structurally, got {spell_debug}"
    );
    assert!(
        rendered.contains("Look at the top X cards of your library")
            && rendered.contains("Counter target spell unless its controller pays twice {X}"),
        "expected Thassa's Intervention compiled text to preserve both modes and twice-X payment, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_counter_unless_pays_and_life_rendering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mundungu Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Counter target spell unless its controller pays {1} and 1 life.")
        .expect("parse counter-unless-pay-and-life probe");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" | ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("counter target spell unless its controller pays {1} and 1 life"),
        "expected counter-unless-pay-and-life text in compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_counter_unless_pays_life_only_rendering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Counter Life Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target spell unless its controller pays 3 life.")
        .expect("parse counter-unless-pay-life probe");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" | ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("counter target spell unless its controller pays 3 life"),
        "expected counter-unless-pay-life text in compiled output, got {rendered}"
    );

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("unlesspayseffect") && debug.contains("loselifeeffect"),
        "expected counter-unless-pay-life to carry a checked total cost, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_counter_unless_non_cost_effect_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Counter Draw Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target spell unless its controller pays draw a card.")
        .expect_err("non-cost counter-unless payment should fail loudly");

    let err = format!("{err:?}").to_ascii_lowercase();
    assert!(
        err.contains("counter-unless") && (err.contains("cost") || err.contains("cost-executable")),
        "expected loud counter-unless cost error, got {err}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_counter_unless_pays_domain_rendering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Evasive Action Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Counter target spell unless its controller pays {1} for each basic land type among lands you control.",
        )
        .expect("parse domain counter-unless-pay probe");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" | ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "counter target spell unless its controller pays {1} for each basic land type among lands you control"
        ),
        "expected domain counter-unless-pay text in compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_generic_unless_pays_mana_rendering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Unless Mana Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Destroy target creature unless its controller pays {2}.")
        .expect("parse generic unless-pays mana probe");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" | ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("destroy target creature unless its controller pays {2}"),
        "expected generic unless-pays mana text in compiled output, got {rendered}"
    );

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("unlesspayseffect") && debug.contains("cost: totalcost"),
        "expected generic unless-pays to carry a total cost, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_generic_unless_pays_life_rendering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Unless Life Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Destroy target creature unless its controller pays 2 life.")
        .expect("parse generic unless-pays life probe");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" | ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("destroy target creature unless its controller pays 2 life"),
        "expected generic unless-pays life text in compiled output, got {rendered}"
    );

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("unlesspayseffect")
            && debug.contains("cost: totalcost")
            && debug.contains("loselifeeffect"),
        "expected generic unless-pays life to carry a checked total cost, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_generic_unless_sacrifice_rendering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Unless Sacrifice Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Destroy target creature unless its controller sacrifices a creature.")
        .expect("parse generic unless-sacrifice probe");

    let rendered_raw = unprocessed_compiled_lines(&def).join(" | ");
    let rendered = rendered_raw.to_ascii_lowercase();
    assert!(
        rendered.contains("destroy target creature unless its controller sacrifices"),
        "expected generic unless-sacrifice action text in compiled output, got {rendered_raw}"
    );
    assert!(
        !rendered.contains("choose exactly")
            && !rendered.contains("sacrifice_cost")
            && !rendered.contains("pay choose"),
        "unless-sacrifice rendering should hide internal choose/tag cost scaffolding, got {rendered_raw}"
    );

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("unlesspayseffect")
            && debug.contains("cost: totalcost")
            && debug.contains("sacrificeeffect"),
        "expected generic unless-sacrifice to carry a checked total cost, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_rogue_elephant_unless_sacrifice_keeps_oracle_like_cost_text() {
    let def = parse_oracle_card_definition("Rogue Elephant");

    let rendered_raw = unprocessed_compiled_lines(&def).join(" | ");
    let rendered = rendered_raw.to_ascii_lowercase();
    assert!(
        rendered.contains("sacrifice it unless you sacrifice a forest"),
        "expected Rogue Elephant to render the implicit-zone sacrifice cost naturally, got {rendered_raw}"
    );
    assert!(
        !rendered.contains("choose exactly")
            && !rendered.contains("sacrifice_cost")
            && !rendered.contains("pay choose"),
        "Rogue Elephant rendering should hide internal choose/tag cost scaffolding, got {rendered_raw}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_champion_keyword_preserves_sacrifice_unless_exile_semantics() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Champion Elemental Probe")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elemental])
        .parse_text("Trample\nChampion an Elemental")
        .expect("champion keyword should parse");

    let rendered_raw = unprocessed_compiled_lines(&def).join(" | ");
    assert!(
        rendered_raw.contains("Champion an Elemental"),
        "expected champion keyword rendering, got {rendered_raw}"
    );
    assert!(
        !rendered_raw.contains("exile another Elemental"),
        "champion keyword should not render as expanded exile-only text, got {rendered_raw}"
    );

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("UnlessActionEffect")
            && debug.contains("SacrificeTargetEffect")
            && debug.contains("ExileUntilEffect")
            && debug.contains("SourceLeavesBattlefield"),
        "champion must lower to sacrifice-unless plus linked exile-until-source-leaves, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn grant_keyword_to_wolves_uses_irregular_plural_and_lowercase_keyword() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Wolf Lord Probe")
        .parse_text("Wolves you control have deathtouch.")
        .expect("wolf subtype keyword grant should parse");

    let rendered_raw = unprocessed_compiled_lines(&def).join(" | ");
    assert!(
        rendered_raw.contains("Wolves you control have deathtouch"),
        "expected oracle-like wolf grant rendering, got {rendered_raw}"
    );
    assert!(
        !rendered_raw.contains("Wolfs") && !rendered_raw.contains("Deathtouch"),
        "wolf grants should use irregular plural and lowercase keyword, got {rendered_raw}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_generic_unless_non_cost_effect_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Unless Draw Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Destroy target creature unless its controller draws a card.")
        .expect_err("non-cost unless payment should fail loudly");

    let err = format!("{err:?}").to_ascii_lowercase();
    assert!(
        err.contains("cost") || err.contains("cost-executable"),
        "expected loud unless cost error, got {err}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_return_target_permanent_you_both_own_and_control_rendering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Obelisk Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text("{6}, {T}: Return target permanent you both own and control to your hand.")
        .expect("parse return-own-control probe");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" | ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("return target permanent you both own and control to your hand"),
        "expected own/control target restriction in compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_power_damage_exchange_rendering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Power Exchange Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{T}: This creature deals damage equal to its power to target creature. \
That creature deals damage equal to its power to this creature.",
        )
        .expect("parse power exchange probe");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" | ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("this creature deals damage equal to its power to target creature"),
        "expected first power damage clause, got {rendered}"
    );
    assert!(
        rendered.contains("that creature deals damage equal to its power to this creature"),
        "expected reciprocal power damage clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_prevent_all_combat_damage_from_target_rendering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Prevent Combat Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "{2}{W}: Prevent all combat damage that would be dealt by target creature this turn.",
        )
        .expect("parse prevent combat probe");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" | ")
        .to_ascii_lowercase();
    assert!(
        rendered
            .contains("prevent all combat damage that would be dealt by target creature this turn"),
        "expected prevent combat damage text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_guard_dogs_keeps_color_sharing_prevention_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Guard Dogs Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{2}{W}, {T}: Choose a permanent you control. Prevent all combat damage target creature would deal this turn if it shares a color with that permanent.")
        .expect("parse Guard Dogs");

    let ability_debug = format!("{:#?}", def.abilities);
    assert!(
        ability_debug.contains("ConditionalEffect")
            && ability_debug.contains("TargetMatches")
            && ability_debug.contains("SharesColorWithTagged")
            && (ability_debug.contains("PreventAllCombatDamageEffect")
                || ability_debug.contains("PreventAllCombatDamageFromEffect")),
        "expected conditional prevention lowering, got {ability_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" | ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("choose a permanent you control")
            && rendered.contains(
                "prevent all combat damage target creature would deal this turn if it shares a color with that permanent"
            )
            && !rendered.contains("if it matches creature"),
        "expected guard dogs color-sharing clause in compiled text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tsabos_assassin_parses_and_renders_most_common_color_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(44_300), "Tsabo's Assassin")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Phyrexian, Subtype::Zombie, Subtype::Assassin])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "{T}: Destroy target creature if it shares a color with the most common color among all permanents or a color tied for most common. A creature destroyed this way can't be regenerated.",
        )
        .expect("Tsabo's Assassin should parse strictly");

    let ability_debug = format!("{:#?}", def.abilities);
    assert!(
        ability_debug.contains("ConditionalEffect")
            && ability_debug.contains("SharesMostCommonPermanentColor")
            && ability_debug.contains("DestroyNoRegenerationEffect"),
        "expected conditional most-common-color destroy in Tsabo's Assassin, got {ability_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("destroy target creature if it shares a color with the most common color among all permanents or a color tied for most common")
            && (rendered.contains("can't be regenerated") || rendered.contains("cant be regenerated")),
        "expected Tsabo's Assassin compiled text to keep the most-common-color and regeneration clauses, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_heroism_parses_for_each_attacking_red_prevention_unless_pays() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Heroism")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Sacrifice a white creature: For each attacking red creature, prevent all combat \
             damage that would be dealt by that creature this turn unless its controller pays {2}{R}.",
        )
        .expect("Heroism should parse strictly");

    assert!(
        def.abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Activated(_))),
        "Heroism should compile to an activated ability"
    );

    let ability_debug = format!("{:#?}", def.abilities);
    assert!(
        ability_debug.contains("ForEachObject")
            && ability_debug.contains("attacking: true")
            && ability_debug.contains("UnlessPaysEffect")
            && ability_debug.contains("PreventAllCombatDamageEffect")
            && ability_debug.contains("ControllerOf")
            && ability_debug.contains("__it__"),
        "expected per-attacking-creature unless-pays prevention structure, got {ability_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("for each")
            && (rendered.contains("red attacking creature")
                || rendered.contains("attacking red creature"))
            && rendered.contains(
                "prevent all combat damage that would be dealt by that creature this turn"
            )
            && rendered.contains("unless its controller pays {2}{r}")
            && !rendered.contains("creature sources"),
        "expected Heroism prevention/unless text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_static_prevent_all_combat_damage_to_this_creature_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Everdawn Champion Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Prevent all combat damage that would be dealt to this creature.")
        .expect("parse static prevent-all-combat-damage to this creature");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&StaticAbilityId::PreventAllCombatDamageToSelf),
        "expected PreventAllCombatDamageToSelf ability id, got {ids:?}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("prevent all combat damage that would be dealt to this creature"),
        "expected static prevent-all-combat-damage text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_modal_choose_one_that_hasnt_been_chosen_sets_mode_memory() {
    let oracle = "{2}, {T}: Choose one that hasn't been chosen —\n\
• This artifact deals 2 damage to target creature.\n\
• Tap target creature.\n\
• Sacrifice this artifact. You gain 3 life.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Three Bowls Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text(oracle)
        .expect("parse choose-one-that-hasnt-been-chosen modal ability");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("disallow_previously_chosen_modes: true"),
        "expected modal memory flag in compiled ability, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("choose one that hasn't been chosen"),
        "expected modal heading to keep unchosen-mode clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_modal_choose_one_that_hasnt_been_chosen_this_turn_sets_turn_scope() {
    let oracle = "Whenever another creature you control enters, choose one that hasn't been chosen this turn —\n\
• Put a +1/+1 counter on this creature.\n\
• Create a tapped Treasure token.\n\
• You gain 2 life.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Gala Greeters Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .expect("parse this-turn choose-one-that-hasnt-been-chosen trigger");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("disallow_previously_chosen_modes_this_turn: true"),
        "expected per-turn modal memory flag in compiled ability, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("choose one that hasn't been chosen this turn"),
        "expected this-turn unchosen-mode clause in rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_keyword_marker_rejects_partial_trailing_clause() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Bad Unleash")
        .card_types(vec![CardType::Creature])
        .parse_text("Unleash while")
        .expect_err("trailing clause must not parse as standalone keyword");
    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("could not find verb in effect clause")
            || message.contains("unsupported line"),
        "expected strict parse failure for trailing keyword clause, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) const ILLICIT_AUCTION_ORACLE: &str = "Each player may bid life for control of target creature. You start the bidding with a bid of 0. In turn order, each player may top the high bid. The bidding ends if the high bid stands. The high bidder loses life equal to the high bid and gains control of the creature. (This effect lasts indefinitely.)";

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_illicit_auction_life_bid_for_control() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(16_449), "Illicit Auction")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(ILLICIT_AUCTION_ORACLE)
        .expect("Illicit Auction should parse strictly");

    let effects = def
        .spell_effect
        .as_ref()
        .expect("Illicit Auction should have spell effects")
        .flattened_default_effects();
    assert_eq!(
        effects.len(),
        1,
        "expected one bidding effect, got {effects:?}"
    );
    assert!(
        effects[0]
            .downcast_ref::<crate::effects::BidLifeEffect>()
            .is_some(),
        "Illicit Auction should lower to a life-bidding effect, got {:?}",
        effects[0]
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn compiled_text_illicit_auction_mentions_high_bidder_reward() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(16_449), "Illicit Auction")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(ILLICIT_AUCTION_ORACLE)
        .expect("Illicit Auction should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Each player may bid life for control of target creature")
            && rendered.contains("The high bidder loses life equal to the high bid")
            && rendered.contains("This effect lasts indefinitely"),
        "compiled text should preserve the life-bidding control clauses, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_level_up_tiers_render_semantics() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Level-Up Tiers Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Level up {W} ({W}: Put a level counter on this. Level up only as a sorcery.)\n\
LEVEL 2-6\n\
3/3\n\
First strike\n\
LEVEL 7+\n\
4/4\n\
Double strike",
        )
        .expect("parse level up tier block");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("level 2-6") && joined.contains("first strike"),
        "expected rendered level-2 tier details, got {joined}"
    );
    assert!(
        joined.contains("level 7+") && joined.contains("double strike"),
        "expected rendered level-7 tier details, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_kargan_dragonlord_level_tier_activation() {
    assert_oracle_card_parses_strict("Kargan Dragonlord");
    let def = parse_oracle_card_definition("Kargan Dragonlord");
    let debug = format!("{:?}", def.abilities);

    assert!(
        debug.contains("__ironsmith_level_range:8:+")
            && debug.contains("SourceHasCounterAtLeast")
            && debug.contains("ModifyPowerToughness"),
        "Kargan Dragonlord should lower its level-8 pump as a level-gated activated ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn compiled_text_kargan_dragonlord_keeps_pump_in_level_block() {
    let def = parse_oracle_card_definition("Kargan Dragonlord");
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains("Level up {R}")
            && rendered.contains(
                "Level 8+.\n8/8.\nFlying, trample\n{R}: This creature gets +1/+0 until end of turn."
            )
            && !rendered.contains("Activated ability"),
        "Kargan Dragonlord compiled text should render the level-up line and level-8 pump block, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_standalone_may_effect_does_not_emit_with_id_wrapper() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Slayer Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, you may destroy target Vampire, Werewolf, or Zombie.",
        )
        .expect("parse slayer-like triggered line");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");

    assert_eq!(triggered.choices.len(), 1, "expected one target choice");

    let debug = format!("{:?}", triggered.effects);
    assert!(
        debug.contains("MayEffect"),
        "expected optional may wrapper, got {debug}"
    );
    assert!(
        debug.contains("DestroyEffect"),
        "expected destroy effect, got {debug}"
    );
    assert_eq!(
        debug.matches("DestroyEffect").count(),
        1,
        "expected one destroy effect for the union target, got {debug}"
    );
    assert!(
        debug.contains("Vampire") && debug.contains("Werewolf") && debug.contains("Zombie"),
        "expected the single target filter to retain every subtype, got {debug}"
    );
    assert!(
        !debug.contains("WithIdEffect"),
        "standalone may should not be wrapped with WithId, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("destroy target vampire or werewolf or zombie"),
        "expected one oracle-like disjunctive target, got {rendered}"
    );
    assert!(
        !rendered.contains("destroy a werewolf") && !rendered.contains("destroy a zombie"),
        "expected disjunctive target not to split into follow-up destroys, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn search_reveal_conditional_that_card_preserves_condition_without_tag_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Loyal Inventor Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Vigilance\n\
             When this creature enters, you may search your library for an artifact card, reveal it, then shuffle. \
             Put that card into your hand if you control an Assassin. Otherwise, put that card on top of your library.",
        )
        .expect("parse loyal-inventor-like triggered line");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ConditionalEffect")
            && debug.contains("PlayerControls")
            && debug.contains("Assassin"),
        "expected conditional Assassin gate in lowered effects, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered
            .contains("you may search your library for an artifact card, reveal it, then shuffle"),
        "expected compact search/reveal/shuffle text, got {rendered}"
    );
    assert!(
        rendered.contains("if you control an assassin, put that card into your hand"),
        "expected the condition to govern the hand move, got {rendered}"
    );
    assert!(
        rendered.contains("otherwise, put that card on top of your library"),
        "expected otherwise branch to use that-card wording, got {rendered}"
    );
    assert!(
        !rendered.contains("tagged object") && !rendered.contains("tags it as"),
        "expected compiled text to hide internal tag plumbing, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn search_reveal_named_card_branch_moves_the_searched_card() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Nazahn Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, search your library for an Equipment card and reveal it. \
             If you reveal a card named Hammer of Nazahn this way, put it onto the battlefield. \
             Otherwise, put that card into your hand. Then shuffle.",
        )
        .expect("parse nazahn-like search branch");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("IfEffect")
            && debug.contains("PriorEffectResult")
            && debug.contains("hammer of nazahn"),
        "expected named searched-card conditional, got {debug}"
    );
    assert!(
        debug.contains("target: Tagged") && debug.contains("\"searched\""),
        "expected branch moves to target the searched card tag, got {debug}"
    );
    assert!(
        !debug.contains("target: Tagged(\n                                                            TagKey(\n                                                                \"triggering\""),
        "searched-card branch must not move the entering creature, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("search your library for an equipment card and reveal it"),
        "expected compact search/reveal text, got {rendered}"
    );
    assert!(
        rendered.contains(
            "if you reveal a card named hammer of nazahn this way, put it onto the battlefield"
        ),
        "expected named reveal branch, got {rendered}"
    );
    assert!(
        rendered.contains("otherwise, put that card into your hand")
            && rendered.contains("then shuffle"),
        "expected otherwise hand branch and final shuffle, got {rendered}"
    );
    assert!(
        !rendered.contains("tagged object") && !rendered.contains("tags it as"),
        "expected compiled text to hide internal tag plumbing, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn if_you_put_artifact_this_way_does_not_leak_tagged_object_markers() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Oviya Automech Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{G}, {T}: You may put a creature or Vehicle card from your hand onto the battlefield. \
             If you put an artifact onto the battlefield this way, put two +1/+1 counters on it.",
        )
        .expect("parse oviya-like activated line");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "if you put an artifact onto the battlefield this way, put two +1/+1 counters on it"
        ),
        "expected moved-tag conditional to render as put-this-way text, got {rendered}"
    );
    assert!(
        !rendered.contains("tagged object") && !rendered.contains("tagged '"),
        "expected compiled text to avoid internal tag references, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_if_you_do_still_wraps_antecedent_with_with_id() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "If You Do Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature enters, you may draw a card. If you do, discard a card.")
        .expect("parse if-you-do triggered line");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");

    assert_eq!(
        triggered.effects.len(),
        2,
        "expected antecedent and if-you-do follow-up"
    );

    let first_debug = format!("{:?}", triggered.effects[0]);
    assert!(
        first_debug.contains("WithIdEffect"),
        "if-you-do antecedent must store result id, got {first_debug}"
    );
    assert!(
        first_debug.contains("MayEffect"),
        "if-you-do antecedent should stay optional, got {first_debug}"
    );

    let second_debug = format!("{:?}", triggered.effects[1]);
    assert!(
        second_debug.contains("IfEffect"),
        "if-you-do follow-up must compile to IfEffect, got {second_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_each_player_who_did_this_way_compiles_to_per_player_if_result() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Kwain Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Each player may draw a card, then each player who drew a card this way gains 1 life.")
        .expect("parse each-player-who-did-this-way activated line");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");

    let debug = format!("{:?}", activated.effects);
    assert!(
        debug.contains("ForPlayersEffect"),
        "expected per-player iteration wrapper, got {debug}"
    );
    assert!(
        debug.contains("WithIdEffect") && debug.contains("MayEffect"),
        "expected optional antecedent to be tracked per player, got {debug}"
    );
    assert!(
        debug.contains("IfEffect") && debug.contains("GainLifeEffect"),
        "expected per-player follow-up gain-life conditional, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_for_each_opponent_who_does_merges_into_per_opponent_if_result() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tempting Contract Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of your upkeep, each opponent may create a Treasure token. For each opponent who does, you create a Treasure token.",
        )
        .expect("parse each-opponent-who-does trigger");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");

    let debug = format!("{:?}", triggered.effects);
    let for_players_count = debug.matches("ForPlayersEffect").count();
    assert_eq!(
        for_players_count, 1,
        "expected merged per-opponent wrapper, got {debug}"
    );
    assert!(
        debug.contains("IfEffect"),
        "expected merged follow-up to compile as IfEffect, got {debug}"
    );
    assert!(
        debug.contains("controller: IteratedPlayer"),
        "expected optional antecedent token controller to remain per-opponent, got {debug}"
    );
    assert!(
        debug.contains("controller: You"),
        "expected follow-up token creation to stay on you, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_for_each_opponent_who_does_binds_implicit_followup_to_you() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tempting Offer Variant")
        .parse_text(
            "Each opponent may create a Treasure token. For each opponent who does, create a Treasure token.",
        )
        .expect("parse each-opponent-who-does implicit follow-up");

    let spell_effects = def.spell_effect.as_ref().expect("spell effects");
    let debug = format!("{:?}", spell_effects);
    assert!(
        debug.contains("ForPlayersEffect"),
        "expected per-opponent wrapper, got {debug}"
    );
    assert!(
        debug.contains("IfEffect"),
        "expected follow-up to compile as IfEffect, got {debug}"
    );
    assert!(
        debug.contains("controller: You"),
        "expected implicit follow-up token creation to bind to you, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_for_each_player_who_does_binds_implicit_followup_to_you() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Group Offer Variant")
        .parse_text(
            "Each player may create a Treasure token. For each player who does, create a Treasure token.",
        )
        .expect("parse each-player-who-does implicit follow-up");

    let spell_effects = def.spell_effect.as_ref().expect("spell effects");
    let debug = format!("{:?}", spell_effects);
    assert!(
        debug.contains("ForPlayersEffect"),
        "expected per-player wrapper, got {debug}"
    );
    assert!(
        debug.contains("IfEffect"),
        "expected follow-up to compile as IfEffect, got {debug}"
    );
    assert!(
        debug.contains("controller: You"),
        "expected implicit follow-up token creation to bind to you, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_each_player_tagged_followups_collapse_into_single_for_players_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Duskmantle Seer Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("At the beginning of your upkeep, each player reveals the top card of their library, loses life equal to that card's mana value, then puts it into their hand.")
        .expect("parse each-player tagged followups trigger");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");

    let debug = format!("{:?}", triggered.effects);
    assert_eq!(
        debug.matches("ForPlayersEffect").count(),
        1,
        "expected a single per-player wrapper for tagged followups, got {debug}"
    );
    assert!(
        debug.contains("RevealTopEffect"),
        "expected reveal-top effect in per-player wrapper, got {debug}"
    );
    assert!(
        debug.contains("LoseLifeEffect"),
        "expected lose-life effect in per-player wrapper, got {debug}"
    );
    assert!(
        debug.contains("MoveToZoneEffect") && debug.contains("zone: Hand"),
        "expected move-to-hand effect in per-player wrapper, got {debug}"
    );
    assert!(
        debug.contains("target: Tagged") && !debug.contains("target: Source"),
        "expected the final 'it' to retain the revealed-card tag, got {debug}"
    );
    let rendered = compiled_text_lines(&def).join("\n");
    assert!(
        rendered.contains(
            "each player reveals the top card of their library, loses life equal to that card's mana value, then puts it into their hand"
        ) && !rendered.contains("puts this creature into each player's hand"),
        "expected the per-player reveal/life/move bundle to render the revealed card, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_trigger_without_comma() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "No Comma Trigger")
        .card_types(vec![CardType::Enchantment])
        .parse_text("At the beginning of the next end step draw a card.")
        .expect("parse trigger without comma");

    let has_triggered = def
        .abilities
        .iter()
        .any(|a| matches!(a.kind, AbilityKind::Triggered(_)));
    assert!(has_triggered, "expected triggered ability");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_state_triggered_sacrifice_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "State Trigger Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("When you control no Swamps, sacrifice this creature.")
        .expect("parse state-triggered sacrifice line");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");

    assert!(
        triggered
            .trigger
            .downcast_ref::<crate::triggers::StateTrigger>()
            .is_some(),
        "expected state-trigger matcher, got {:?}",
        triggered.trigger
    );
    assert!(
        format!("{:?}", triggered.effects).contains("SacrificeTargetEffect"),
        "expected sacrifice effect, got {:?}",
        triggered.effects
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_trigger_when_this_creature_is_turned_face_up() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Face-Up Trigger Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Morph {2}{U}\nWhen this creature is turned face up, draw a card.")
        .expect("parse turned-face-up trigger line");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");

    let effects_debug = format!("{:?}", triggered.effects);
    assert!(
        effects_debug.contains("DrawCardsEffect"),
        "expected draw effect from turned-face-up trigger, got {effects_debug}"
    );

    let compiled = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("turned face up"),
        "expected turned-face-up text in compiled output, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn morph_reveal_cost_cards_keep_reveal_as_the_cost() {
    for (name, color) in [
        ("Dragon's Eye Savants", "blue"),
        ("Ruthless Ripper", "black"),
        ("Temur Charger", "green"),
        ("Watcher of the Roost", "white"),
    ] {
        let def = parse_oracle_card_definition(name);
        let compiled = unprocessed_compiled_lines(&def).join(" ");
        let expected = format!("Morph—Reveal a {color} card from your hand");
        assert!(
            compiled.contains(&expected),
            "expected {name} to retain its reveal morph cost, got {compiled}"
        );
        assert!(
            !compiled.contains("Morph Exile") && !compiled.contains("Morph—Exile"),
            "{name} must not move the revealed card while paying morph: {compiled}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_kadenas_silencer_strict_parse_counter_all_opponent_abilities() {
    let def = parse_oracle_card_definition("Kadena's Silencer");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Kadena's Silencer should have a turned-face-up trigger");
    let effects_debug = format!("{:#?}", triggered.effects);
    assert!(
        effects_debug.contains("CounterEffect")
            && effects_debug.contains("Stack")
            && effects_debug.contains("Ability")
            && effects_debug.contains("Opponent"),
        "expected counter-all-opponent-abilities effect, got {effects_debug}"
    );

    let compiled = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("counter all abilities your opponents control"),
        "expected compiled text for countering all opponent abilities, got {compiled}"
    );
    assert!(
        compiled.contains("megamorph {1}{u}"),
        "expected megamorph text to remain present, got {compiled}"
    );
    assert!(
        !compiled.contains("unsupported parser line fallback"),
        "Kadena's Silencer should not rely on unsupported fallback: {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_trigger_when_face_down_permanent_is_turned_face_up() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sumala Trigger Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever a face-down permanent you control is turned face up, put a +1/+1 counter on it and a +1/+1 counter on this creature.",
        )
        .expect("parse filtered turned-face-up trigger line");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("PermanentTurnedFaceUpTrigger"),
        "expected filtered turned-face-up trigger matcher, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("unimplemented_trigger"),
        "expected no custom-trigger fallback for turned-face-up filter, got {abilities_debug}"
    );

    let compiled = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("whenever a face-down permanent you control is turned face up"),
        "expected turned-face-up trigger text to preserve face-down filter, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_experiment_twelve_strict_parse_regression() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Experiment Twelve")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Trample\nWhenever this creature or another creature you control is turned face up, put +1/+1 counters on that creature equal to its power.\nDisguise {4}{G}",
        )
        .expect("Experiment Twelve should parse");

    let compiled = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("put x +1/+1 counters on that creature, where x is its power"),
        "expected dynamic power-based counter clause in compiled output, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn aquamorph_entity_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Aquamorph Entity");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("ChoosePowerToughnessAsEntersOrTurnsFaceUp"),
        "expected structured P/T choice static ability, got {debug}"
    );
    assert!(
        debug.contains("Morph"),
        "expected morph ability, got {debug}"
    );

    let compiled = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains(
            "as this creature enters or is turned face up, it becomes your choice of 5/1 or 1/5"
        ),
        "expected Aquamorph Entity P/T-choice clause in compiled output, got {compiled}"
    );
    assert!(
        compiled.contains("morph {2}{u}"),
        "expected Aquamorph Entity morph clause in compiled output, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn primal_plasma_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Primal Plasma");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("ChoosePowerToughnessAsEntersOrTurnsFaceUp"),
        "expected structured P/T-choice static ability, got {debug}"
    );
    assert!(
        debug.contains("Flying") && debug.contains("Defender"),
        "expected keyword-granting P/T choices, got {debug}"
    );

    let compiled = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains(
            "as this creature enters, it becomes your choice of a 3/3 creature, a 2/2 creature with flying, or a 1/6 creature with defender"
        ),
        "expected Primal Plasma choice clause in compiled output, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_trigger_this_creature_enters_from_your_graveyard() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Phyrexian Dragon Engine")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters from your graveyard, you may discard your hand. If you do, draw three cards.",
        )
        .expect("parse enters-from-your-graveyard trigger");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("from: Specific(\n                                Graveyard")
            || debug.contains("from: Specific(Graveyard)"),
        "expected trigger origin zone to be graveyard, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_composed_anthems_keep_independent_land_conditions() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tek Variant")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .parse_text(
            "This creature gets +0/+2 as long as you control a Plains, has flying as long as you control an Island, gets +2/+0 as long as you control a Swamp, has first strike as long as you control a Mountain, and has trample as long as you control a Forest.",
        )
        .expect("composed anthems should parse with independent conditions");
    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("you control a plains")
            && abilities_debug.contains("you control an island")
            && abilities_debug.contains("you control a swamp")
            && abilities_debug.contains("you control a mountain")
            && abilities_debug.contains("you control a forest"),
        "expected independent land conditions for each composed anthem branch, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_granted_keyword_and_must_attack_keeps_both_parts() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Hellraiser Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Creatures you control have haste and attack each combat if able.")
        .expect_err("granted keyword + must-attack is currently unsupported");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported anthem subject"),
        "expected explicit unsupported anthem-subject error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_anger_graveyard_condition_with_land_control() {
    use crate::zone::Zone;

    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Anger Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Haste\nAs long as this card is in your graveyard and you control a Mountain, creatures you control have haste.",
        )
        .expect("anger-style graveyard + land-control condition should parse");

    let grant_from_graveyard = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::GrantObjectAbilityForFilter
        ) && ability.functional_zones.contains(&Zone::Graveyard)
            && !ability.functional_zones.contains(&Zone::Battlefield)
    });
    assert!(
        grant_from_graveyard,
        "expected anger-style grant ability to function from graveyard, got {:?}",
        def.abilities
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_quagmire_landwalk_as_though_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Quagmire")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Creatures with swampwalk can be blocked as though they didn't have swampwalk.")
        .expect("quagmire landwalk as-though clause should parse");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("as though they didn't have swampwalk"),
        "expected rendered as-though swampwalk override clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_exile_up_to_one_single_disjunction_stays_single_choice() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Scrollshift Variant")
            .parse_text(
                "Exile up to one target artifact, creature, or enchantment you control, then return it to the battlefield under its owner's control.",
            )
            .expect("parse single-disjunction exile");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    let debug = format!("{effects:?}");
    let choose_count = debug.matches("ChooseObjectsEffect").count();
    assert!(
        choose_count <= 1,
        "single disjunctive target should not fan out into per-type choices, got {choose_count} in {debug}"
    );
    assert!(
        debug.contains("ExileEffect") && debug.contains("MoveToZoneEffect"),
        "expected exile-then-return sequence, got {debug}"
    );
    assert!(
        debug.contains("card_types: [Artifact, Creature, Enchantment]")
            || debug.contains("card_types: [Artifact, Enchantment, Creature]")
            || debug.contains("card_types: [Creature, Artifact, Enchantment]")
            || debug.contains("card_types: [Creature, Enchantment, Artifact]")
            || debug.contains("card_types: [Enchantment, Artifact, Creature]")
            || debug.contains("card_types: [Enchantment, Creature, Artifact]"),
        "expected combined disjunctive type filter, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_exile_then_return_with_counter_keeps_counter_followup() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Planar Incision Variant")
        .parse_text(
            "Exile target artifact or creature, then return it to the battlefield under its owner's control with a +1/+1 counter on it.",
        )
        .expect("parse exile-then-return with counter");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("MoveToZoneEffect"),
        "expected return move-to-battlefield effect, got {debug}"
    );
    assert!(
        debug.contains("enters_with_counters") && debug.contains("PlusOnePlusOne"),
        "expected +1/+1 counter entry modifier on the returned object, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_shares_permanent_type_with_it_adds_tagged_constraint() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cloudstone Curio Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Whenever a nonartifact permanent you control enters, you may return another permanent you control that shares a permanent type with it to its owner's hand.",
        )
        .expect("parse shares-permanent-type clause");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("SharesPermanentType"),
        "expected tagged shares-card-type constraint for 'shares a permanent type with it', got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("shares a permanent type"),
        "expected rendered share-type restriction, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_unblocked_attacking_filter_sets_unblocked() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Throatseeker Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Unblocked attacking Ninjas you control have lifelink.")
        .expect("parse unblocked-attacking static filter");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("attacking: true"),
        "expected attacking filter flag, got {debug}"
    );
    assert!(
        debug.contains("unblocked: true"),
        "expected unblocked to map to unblocked filter flag, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_blocked_filter_sets_blocked() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Blocked Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Target blocked creature gains lifelink until end of turn.")
        .expect("parse blocked target filter");

    let debug = format!("{:#?}", def);
    assert!(
        debug.contains("blocked: true"),
        "expected blocked filter flag, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_lesser_mana_value_adds_tagged_lt_constraint() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Orah Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature or another Cleric you control dies, return target Cleric card with lesser mana value from your graveyard to the battlefield.",
        )
        .expect("parse lesser-mana-value tagged comparison");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("ManaValueLtTagged"),
        "expected lesser mana value relation against tagged object, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_this_or_another_creature_dies_is_not_this_dies_only() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Blood Artist Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.",
        )
        .expect("parse this-or-another creature dies trigger");

    let trigger_debug = match &def.abilities[0].kind {
        AbilityKind::Triggered(triggered) => format!("{:#?}", triggered.trigger),
        _ => panic!("expected triggered ability"),
    };
    let rendered = unprocessed_compiled_lines(&def).join(" | ");

    assert!(
        trigger_debug.contains("other: true"),
        "expected the another-creature branch to exclude the source, got {trigger_debug}"
    );
    assert!(
        rendered.contains("Whenever this creature or another creature dies"),
        "expected preserved this-or-another dies surface, got {rendered}\ntrigger={trigger_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_compile_this_or_another_graveyard_from_battlefield_surface() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Grave Pact Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever this enchantment or another nonland permanent you control is put into a graveyard from the battlefield, draw a card.",
        )
        .expect("parse this-or-another graveyard-from-battlefield trigger");

    let trigger_debug = match &def.abilities[0].kind {
        AbilityKind::Triggered(triggered) => format!("{:#?}", triggered.trigger),
        _ => panic!("expected triggered ability"),
    };
    let rendered = unprocessed_compiled_lines(&def).join(" | ");

    assert!(
        trigger_debug.contains("source: true") && trigger_debug.contains("other: true"),
        "expected distinct source and another-permanent trigger branches, got {trigger_debug}"
    );
    assert!(
        rendered.contains(
            "Whenever this enchantment or another nonland permanent you control is put into a graveyard from the battlefield"
        ),
        "expected preserved graveyard-from-battlefield surface, got {rendered}\ntrigger={trigger_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_compile_this_or_another_ally_enters_trigger_surface() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ally Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature or another Ally you control enters, you may put a +1/+1 counter on this creature.",
        )
        .expect("parse ally enter trigger");

    let rendered = unprocessed_compiled_lines(&def).join(" | ");
    let trigger_debug = match &def.abilities[0].kind {
        AbilityKind::Triggered(triggered) => format!("{:#?}", triggered.trigger),
        _ => panic!("expected triggered ability"),
    };
    assert!(
        rendered.contains("Whenever this creature or another Ally you control enters"),
        "expected ally enter trigger surface, got {rendered}\ntrigger={trigger_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_compile_this_or_another_ally_enters_team_buff_surface() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ally Team Buff Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature or another Ally you control enters, creatures you control get +1/+1 until end of turn.",
        )
        .expect("parse ally team buff trigger");

    let rendered = unprocessed_compiled_lines(&def).join(" | ");
    assert!(
        rendered.contains("Whenever this creature or another Ally you control enters")
            && rendered.contains("+1/+1 until end of turn"),
        "expected ally team-buff trigger surface, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_equal_or_lesser_mana_value_adds_tagged_lte_constraint() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Jailbreak Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Return target permanent card in an opponent's graveyard to the battlefield under their control. When that permanent enters, return up to one target permanent card with equal or lesser mana value from your graveyard to the battlefield.",
        )
        .expect("parse equal-or-lesser mana value tagged comparison");

    let debug = format!("{:#?}", def);
    assert!(
        debug.contains("ManaValueLteTagged"),
        "expected equal-or-lesser mana value relation against tagged object, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_render_multiple_cycling_variants_preserves_variant_names() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cycling Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Mountaincycling {2}, forestcycling {2}.")
        .expect("parse multiple cycling variants");

    let lines = unprocessed_compiled_lines(&def);
    assert!(
        lines.iter().any(|line| line.contains("Mountaincycling")),
        "expected mountaincycling keyword in render, got {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line.contains("Forestcycling")),
        "expected forestcycling keyword in render, got {lines:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_render_multiple_cycling_variants_with_reminder_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cycling Variant Reminder")
        .card_types(vec![CardType::Creature])
        .parse_text("Mountaincycling {2}, forestcycling {2} ({2}, Discard this card: Search your library for a Mountain or Forest card, reveal it, put it into your hand, then shuffle.)")
        .expect("parse cycling variants with reminder");

    let lines = unprocessed_compiled_lines(&def);
    assert!(
        lines
            .iter()
            .any(|line| line.contains("Mountaincycling {2}")),
        "expected mountaincycling keyword in render, got {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line.contains("Forestcycling {2}")),
        "expected forestcycling keyword in render, got {lines:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_multiple_cycling_variants_merges_search_filter_subtypes() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cycling Variant Filter")
        .card_types(vec![CardType::Creature])
        .parse_text("Mountaincycling {2}, forestcycling {2} ({2}, Discard this card: Search your library for a Mountain or Forest card, reveal it, put it into your hand, then shuffle.)")
        .expect("parse cycling variants with reminder");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("subtypes: [Mountain, Forest]")
            || debug.contains("subtypes: [Forest, Mountain]"),
        "expected merged mountain/forest cycling search filter, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_render_cycling_includes_cost() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cycling Cost Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Cycling {2}{U} ({2}{U}, Discard this card: Draw a card.)")
        .expect("parse cycling with cost");

    let lines = unprocessed_compiled_lines(&def);
    assert!(
        lines.iter().any(|line| line.contains("Cycling {2}{U}")),
        "expected cycling cost in render, got {lines:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_render_basic_landcycling_as_keyword_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Basic Landcycling Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Target player loses 4 life and you gain 4 life.\nBasic landcycling {1}{B} ({1}{B}, Discard this card: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.)",
        )
        .expect("parse basic landcycling line");

    let lines = unprocessed_compiled_lines(&def);
    assert!(
        lines
            .iter()
            .any(|line| line.contains("Basic landcycling {1}{B}")),
        "expected basic landcycling keyword in render, got {lines:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_cycling_pay_life_keeps_keyword_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Street Wraith Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Cycling—Pay 2 life. ({2}, Discard this card: Draw a card.)")
        .expect("parse life-cycling line");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Cycling—Pay 2 life"),
        "expected rendered life-cycling keyword, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("LoseLifeEffect")
            && debug.contains("DiscardEffect")
            && debug.contains("EmitKeywordActionEffect")
            && debug.contains("DrawCardsEffect"),
        "expected life-cycling to remain a discard+draw activated ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_cycle_this_card_trigger_compiles() {
    use crate::zone::Zone;

    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cycling Trigger Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Cycling {2}.\nWhenever you cycle this card, draw a card.")
        .expect("parse cycling trigger variant");

    let has_trigger = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Triggered(t) if t.trigger.display() == "Whenever you cycle this card"
        ) && ability.functional_zones.contains(&Zone::Graveyard)
    });
    assert!(
        has_trigger,
        "expected source-specific cycling trigger that functions in graveyard, got {:?}",
        def.abilities
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_crystalline_resonance_becomes_copy_until_your_next_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Crystalline Resonance")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever you cycle a card, you may have this enchantment become a copy of another target permanent until your next turn, except it has this ability.",
        )
        .expect("Crystalline Resonance should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        (rendered.contains("becomes a copy of another target permanent")
            || rendered.contains("become a copy of another target permanent"))
            && rendered.contains("until your next turn")
            && rendered.contains("except it has this ability"),
        "expected the copy duration and preserved ability to survive rendering, got {rendered}"
    );

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("ApplyContinuousEffect")
            && debug.contains("YourNextTurn")
            && debug.contains("preserve_source_abilities: true")
            && !debug.contains("CopySpellEffect"),
        "expected a copy-permanent lowering with a preserved source ability and next-turn duration, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_sarkhan_becomes_copy_with_name_and_legendary_exception() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sarkhan, Soul Aflame")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Red],
        ]))
        .supertypes(vec![crate::types::Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Shaman])
        .power_toughness(PowerToughness::fixed(2, 4))
        .parse_text(
            "Dragon spells you cast cost {1} less to cast.\nWhenever a Dragon you control enters, you may have Sarkhan become a copy of it until end of turn, except its name is Sarkhan, Soul Aflame and it's legendary in addition to its other types.",
        )
        .expect("Sarkhan copy exception should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "Whenever a Dragon you control enters, you may have Sarkhan become a copy of it until end of turn, except its name is Sarkhan, Soul Aflame and it's legendary in addition to its other types."
        ),
        "expected Sarkhan's source surface, copy name, and legendary exception to survive rendering, got {rendered}"
    );

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("name_override")
            && debug.contains("Sarkhan, Soul Aflame")
            && debug.contains("add_supertypes")
            && debug.contains("Legendary"),
        "expected Sarkhan lowering to carry modeled copy exceptions, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_treasure_nabber_keeps_next_turn_end_control_duration() {
    let def = parse_oracle_card_definition("Treasure Nabber");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Whenever an opponent taps an artifact for mana")
            && rendered.contains("gain control of that artifact until the end of your next turn"),
        "expected Treasure Nabber to preserve the next-turn-end control duration, got {rendered}"
    );

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("ChangeControllerToEffectController") && debug.contains("YourNextTurnEnd"),
        "expected Treasure Nabber to lower as a control-change duration through your next turn end, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_valiant_rescuer_keeps_another_card_cycle_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Valiant Rescuer")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(3, 1))
        .parse_text(
            "Whenever you cycle another card for the first time each turn, create a 1/1 white Human Soldier creature token.\nCycling {2} ({2}, Discard this card: Draw a card.)",
        )
        .expect("Valiant Rescuer should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Whenever you cycle another card") && rendered.contains("Cycling {2}"),
        "expected another-card cycling trigger to survive rendering, got {rendered}"
    );

    let debug = format!("{:#?}", def.abilities);
    let has_once_each_turn_cap = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Triggered(triggered)
                if triggered.intervening_if == Some(crate::ConditionExpr::FirstTimeThisTurn)
        )
    });
    assert!(
        debug.contains("source_filter: Some")
            && debug.contains("other: true")
            && has_once_each_turn_cap,
        "expected reusable another-card cycle trigger lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_commander_recursion_trigger_uses_graveyard_zone_and_commander_filter() {
    use crate::zone::Zone;

    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Commander Recursion Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Whenever your commander enters or attacks, you may pay {2}. If you do, return this card from your graveyard to your hand.",
        )
        .expect("parse commander recursion trigger");

    let ability = def
        .abilities
        .iter()
        .find(|ability| matches!(&ability.kind, AbilityKind::Triggered(_)))
        .expect("expected triggered ability");

    assert!(
        ability.functional_zones.contains(&Zone::Graveyard)
            && !ability.functional_zones.contains(&Zone::Battlefield),
        "expected trigger to function from graveyard only, got {:?}",
        ability.functional_zones
    );

    let trigger_debug = match &ability.kind {
        AbilityKind::Triggered(triggered) => format!("{:?}", triggered.trigger),
        _ => unreachable!("checked triggered ability above"),
    };
    assert!(
        trigger_debug.contains("AttacksTrigger") && !trigger_debug.contains("ThisAttacksTrigger"),
        "expected shared-subject attack branch, got {trigger_debug}"
    );
    let compact = trigger_debug.split_whitespace().collect::<String>();
    assert!(
        compact.contains("is_commander:true") && compact.contains("owner:Some(You"),
        "expected your-commander ownership filter on both branches, got {trigger_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_resize_strictly_parses_and_renders_recover() {
    assert_oracle_card_parses_strict("Resize");
    let def = parse_oracle_card_definition("Resize");

    let rendered = unprocessed_compiled_lines(&def);
    assert_eq!(
        rendered,
        vec![
            "Target creature gets +3/+3 until end of turn.".to_string(),
            "Recover {1}{G}.".to_string(),
        ],
        "expected Resize to render its spell effect and compact recover keyword line"
    );

    let recover = def
        .abilities
        .iter()
        .find(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Triggered(triggered)
                    if matches!(
                        triggered.presentation_label.as_ref(),
                        Some(crate::ability::PresentationLabel::Keyword(
                            crate::ability::PresentationKeyword::Recover(cost)
                        )) if cost == "{1}{G}"
                    )
            )
        })
        .expect("Resize should lower recover to a triggered ability");
    assert_eq!(
        recover.functional_zones,
        vec![Zone::Graveyard],
        "recover should function only from the graveyard"
    );

    let debug = format!("{recover:#?}");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(
        compact.contains("ZoneChangeTrigger")
            && compact.contains("from:Specific(Battlefield)")
            && compact.contains("to:Specific(Graveyard)")
            && compact.contains("owner:Some(You")
            && debug.contains("SourceIsInZone")
            && debug.contains("PayManaEffect")
            && debug.contains("ReturnFromGraveyardToHandEffect")
            && debug.contains("ExileEffect")
            && debug.contains("DidNotHappen"),
        "expected Resize recover to structurally model payment, return, and otherwise-exile branches, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_bridge_from_below_compiles_graveyard_triggers() {
    use crate::zone::Zone;

    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bridge from Below")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever a nontoken creature is put into your graveyard from the battlefield, if this card is in your graveyard, create a 2/2 black Zombie creature token.\nWhen a creature is put into an opponent's graveyard from the battlefield, if this card is in your graveyard, exile this card.",
        )
        .expect("Bridge from Below should parse without unsupported fallback");

    assert_eq!(def.abilities.len(), 2, "expected two triggered abilities");
    assert!(
        def.abilities.iter().all(|ability| {
            matches!(&ability.kind, AbilityKind::Triggered(_))
                && ability.functional_zones == vec![Zone::Graveyard]
        }),
        "expected both Bridge triggers to function from graveyard, got {:?}",
        def.abilities
    );

    let triggers = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(triggers.len(), 2, "expected two triggered abilities");
    assert_eq!(
        triggers[0].intervening_if,
        Some(crate::ConditionExpr::SourceIsInZone(Zone::Graveyard)),
        "expected Bridge token trigger to recheck that the source is still in the graveyard"
    );
    let first_trigger_debug = format!("{:?}", triggers[0].trigger);
    assert!(
        first_trigger_debug.contains("from: Specific(Battlefield)")
            && first_trigger_debug.contains("owner: Some(You)")
            && first_trigger_debug.contains("nontoken: true"),
        "expected first Bridge trigger to watch your creature dying from the battlefield, got {first_trigger_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("create a 2/2 black zombie creature token")
            && rendered.contains("exile this")
            && rendered.contains("an opponent")
            && rendered.contains("is put into an opponent's graveyard from the battlefield"),
        "expected both Bridge abilities in compiled text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_return_from_graveyard_keeps_with_cycling_filter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sacred Excavation Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Return up to two target cards with cycling from your graveyard to your hand.")
        .expect("parse return with cycling filter");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("with cycling from your graveyard"),
        "expected rendered target filter to keep with-cycling qualifier, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_same_name_destroy_fans_out_to_all_other_matching_objects() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Same Name Destroy Variant")
        .parse_text(
            "Destroy target artifact and all other artifacts with the same name as that artifact.",
        )
        .expect("parse same-name destroy sentence");

    let effects = def.spell_effect.expect("expected spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("DestroyEffect") && debug.matches("DestroyEffect").count() >= 2,
        "expected target destroy plus fanout destroy, got {debug}"
    );
    assert!(
        debug.contains("SameNameAsTagged"),
        "expected same-name tagged relation in fanout filter, got {debug}"
    );
    assert!(
        debug.contains("IsNotTaggedObject"),
        "expected all-other exclusion relation in fanout filter, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_same_name_exile_with_that_player_controls_keeps_controller_link() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Same Name Exile Variant")
            .parse_text(
                "Exile target creature an opponent controls with mana value 2 or less and all other creatures that player controls with the same name as that creature.",
            )
            .expect("parse same-name exile sentence");

    let effects = def.spell_effect.expect("expected spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("MoveToZoneEffect") && debug.contains("ExileEffect"),
        "expected target exile plus fanout exile-all effect, got {debug}"
    );
    assert!(
        debug.contains("SameNameAsTagged"),
        "expected same-name tagged relation in fanout filter, got {debug}"
    );
    assert!(
        debug.contains("SameControllerAsTagged"),
        "expected same-controller tagged relation in fanout filter, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_legions_end_style_reveal_and_exile_keeps_same_name_hand_graveyard_bundle() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Legion's End Variant")
        .parse_text(
            "Exile target creature an opponent controls with mana value 2 or less and all other creatures that player controls with the same name as that creature. Then that player reveals their hand and exiles all cards with that name from their hand and graveyard.",
        )
        .expect("parse legion's end style reveal+exile sentence");

    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("lookathandeffect"),
        "expected reveal-hand effect for that player, got {debug}"
    );
    assert!(
        debug.contains("samenameastagged"),
        "expected same-name tagged relation in hand/graveyard exile filter, got {debug}"
    );
    assert!(
        debug.contains("zone: some(hand)") && debug.contains("zone: some(graveyard)"),
        "expected hand and graveyard zones in follow-up exile filter, got {debug}"
    );
    assert!(
        debug.contains("controllerof"),
        "expected 'that player' to resolve through controller-of tagged context, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_same_name_fanout_requires_full_reference_tail() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Broken Same Name Variant")
        .parse_text("Destroy target artifact and all other artifacts with the same name as.")
        .expect_err("same-name clause without full tail should fail");
    let message = format!("{err:?}");
    assert!(
        message.contains("same-name"),
        "expected actionable same-name parse error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_same_name_target_gets_fans_out_to_tagged_filter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Same Name Gets Variant")
            .parse_text(
                "Target creature and all other creatures with the same name as that creature get -3/-3 until end of turn.",
            )
            .expect("parse same-name gets sentence");

    let effects = def.spell_effect.expect("expected spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.matches("ApplyContinuousEffect").count() >= 2
            && debug.contains("runtime_modifications: [ModifyPowerToughness"),
        "expected target and fanout continuous runtime modifications, got {debug}"
    );
    assert!(
        debug.contains("SameNameAsTagged") && debug.contains("IsNotTaggedObject"),
        "expected same-name all-other relations in fanout filter, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_equipped_gets_and_has_activated_grant_as_static_abilities() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Equip Activated Grant Variant")
            .parse_text(
                "Equip {1}\nEquipped creature gets +0/+3 and has \"{2}, {T}: Target player mills three cards.\"",
            )
            .expect("parse equipped activated grant line");

    assert!(
        def.spell_effect.is_none(),
        "equipped activated grant must not compile as one-shot spell effects"
    );

    let mut has_anthem = false;
    let mut has_attached_grant = false;
    for ability in &def.abilities {
        if let AbilityKind::Static(static_ability) = &ability.kind {
            if static_ability.id() == crate::static_abilities::StaticAbilityId::Anthem {
                has_anthem = true;
            }
            if static_ability.id() == crate::static_abilities::StaticAbilityId::AttachedAbilityGrant
            {
                has_attached_grant = true;
            }
        }
    }
    assert!(has_anthem, "expected equipped anthem static ability");
    assert!(
        has_attached_grant,
        "expected attached activated-ability grant static ability"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_song_of_the_dryads_type_transform_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Song of the Dryads")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text("Enchant permanent\nEnchanted permanent is a colorless Forest land.")
        .expect("song-style attached transform should parse");

    let compiled = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("enchanted permanent is a colorless forest land"),
        "expected land type-setting text, got {compiled}"
    );
    assert!(
        compiled.contains("forest"),
        "expected forest subtype text, got {compiled}"
    );
    assert!(
        compiled.contains("colorless"),
        "expected colorless text, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_imprisoned_in_the_moon_type_transform_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Imprisoned in the Moon")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant creature, land, or planeswalker\nEnchanted permanent is a colorless land with \"{T}: Add {C}\" and loses all other card types and abilities.",
        )
        .expect("imprisoned-style attached transform should parse");

    let compiled = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("colorless"),
        "expected colorless text, got {compiled}"
    );
    assert!(
        compiled.contains("{t}: add {c}") || compiled.contains("{t}: add c"),
        "expected granted mana ability text, got {compiled}"
    );
    assert!(
        compiled.contains("loses all other card types and abilities")
            || compiled.contains("lose all abilities"),
        "expected ability-loss text, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_swift_reconfiguration_vehicle_transform_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Swift Reconfiguration")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Flash\nEnchant creature or Vehicle\nEnchanted permanent is a Vehicle artifact with crew 5 and it loses all other card types.",
        )
        .expect("swift-reconfiguration-style transform should parse");

    let compiled = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("vehicle"),
        "expected vehicle text, got {compiled}"
    );
    assert!(
        compiled.contains("crew 5"),
        "expected crew keyword text, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_spider_man_no_more_transform_strict_and_compiled_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Spider-Man No More")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant creature\nEnchanted creature is a Citizen with base power and toughness 1/1. It has defender and loses all other abilities.",
        )
        .expect("Spider-Man No More should parse strictly");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        static_ids.contains(&StaticAbilityId::SetBasePowerToughnessForFilter),
        "expected base power/toughness static ability, got {static_ids:?}"
    );
    assert!(
        static_ids.contains(&StaticAbilityId::RemoveAllAbilitiesForFilter),
        "expected ability-removal static ability, got {static_ids:?}"
    );
    assert!(
        static_ids.contains(&StaticAbilityId::AttachedAbilityGrant),
        "expected attached defender grant, got {static_ids:?}"
    );
    assert!(
        static_ids.contains(&StaticAbilityId::SetCreatureSubtypes),
        "expected Citizen to replace other creature types, got {static_ids:?}"
    );

    let compiled = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        compiled.contains("Enchanted creature is a Citizen with base power and toughness 1/1."),
        "expected compact transform wording, got {compiled}"
    );
    assert!(
        compiled.contains("It has defender and loses all other abilities."),
        "expected defender-plus-other-abilities wording, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_ensoul_artifact_style_transform_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ensoul Artifact")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant artifact\nEnchanted artifact is a creature with base power and toughness 5/5 in addition to its other types.",
        )
        .expect("ensoul-artifact-style transform should parse");

    let compiled = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("enchanted artifact is a creature")
            || compiled
                .contains("enchanted artifact has base power and toughness 5/5 and is a creature"),
        "expected creature type-setting text, got {compiled}"
    );
    assert!(
        compiled.contains("base power and toughness 5/5"),
        "expected base power/toughness text, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_equipped_activated_grant_with_unattach_cost_compiles() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Equip Unattach Grant Variant")
        .parse_text(
            "Equip {5}\nEquipped creature gets +2/+1 and has \"{T}, Unattach this source: Destroy target creature.\"",
        )
        .expect("equipped unattach activated cost should compile");
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("attachedabilitygrant") && debug.contains("unattachobjectseffect"),
        "expected equipped unattach grant to compile as an attached activated ability, got {debug}"
    );
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("unattach") && rendered.contains("destroy target creature"),
        "expected rendered equipped ability to preserve unattach destroy clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn carry_away_strict_parser_and_text_regression() {
    let def = parse_oracle_card_definition("Carry Away");
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("unattachobjectseffect") && debug.contains("controlattachedpermanent"),
        "Carry Away should lower to unattach effect plus attached control static ability, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("unattach enchanted equipment"),
        "Carry Away compiled text should preserve the unattach enchanted Equipment clause, got {rendered}"
    );
    assert!(
        rendered.contains("you control enchanted equipment"),
        "Carry Away compiled text should preserve the control-attached Equipment clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn carry_away_runtime_setup(
    equipment_starts_attached: bool,
) -> (
    crate::game_state::GameState,
    PlayerId,
    PlayerId,
    ObjectId,
    ObjectId,
    ObjectId,
) {
    let carry_away = parse_oracle_card_definition("Carry Away");
    let equipment = CardDefinitionBuilder::new(CardId::new(), "Carry Away Test Equipment")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .build();
    let creature = CardDefinitionBuilder::new(CardId::new(), "Carry Away Test Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let creature_id = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
    let equipment_id = game.create_object_from_definition(&equipment, bob, Zone::Battlefield);
    if equipment_starts_attached {
        assert!(game.attach_object_to_target(
            equipment_id,
            crate::object::AttachmentTarget::Object(creature_id),
        ));
    }
    let carry_away_id = game.create_object_from_definition(&carry_away, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(
        carry_away_id,
        crate::object::AttachmentTarget::Object(equipment_id),
    ));
    game.mark_continuous_state_dirty();
    game.refresh_continuous_state();

    (game, alice, bob, carry_away_id, equipment_id, creature_id)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_carry_away_enter_trigger(
    game: &mut crate::game_state::GameState,
    carry_away_id: ObjectId,
) {
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(carry_away_id)
            .expect("Carry Away should exist on battlefield"),
        game,
    );
    let enters_event = crate::events::RawEvent::new(
        crate::events::ZoneChangeEvent::with_cause(
            carry_away_id,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    for entry in crate::triggers::check_triggers(game, &enters_event)
        .into_iter()
        .filter(|entry| entry.source == carry_away_id)
    {
        trigger_queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(game, &mut trigger_queue)
        .expect("Carry Away enters trigger should go on the stack");
    assert_eq!(
        game.stack.len(),
        1,
        "Carry Away entering should create exactly one unattach trigger"
    );
    crate::game_loop::resolve_stack_entry(game)
        .expect("Carry Away unattach trigger should resolve");
    game.mark_continuous_state_dirty();
    game.refresh_continuous_state();
}
