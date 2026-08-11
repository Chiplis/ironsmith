#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;
use crate::ability::ActivatedAbility;
use crate::decision::{AutoPassDecisionMaker, DecisionMaker, SelectFirstDecisionMaker};

fn activated_ability(definition: &CardDefinition) -> &ActivatedAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected an activated ability")
}

fn triggered_ability(definition: &CardDefinition) -> &TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected a triggered ability")
}

fn simple_spell(name: &str, card_type: CardType, mana_value: u8) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Generic(mana_value),
        ]]))
        .with_spell_effect(vec![Effect::draw(1)])
        .build()
}

fn resolve_chaos_wand(accept_cast: bool) -> (Zone, Zone) {
    let definition = parse_oracle_card_definition("Chaos Wand");
    let activated = activated_ability(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    // Library top is the last inserted card, so the creature is consulted
    // before the matching instant.
    let matching = game.create_object_from_definition(
        &simple_spell("Consulted Instant", CardType::Instant, 1),
        bob,
        Zone::Library,
    );
    let matching_stable = game.object(matching).expect("matching card").stable_id;
    let nonmatching = game.create_object_from_definition(
        &simple_spell("Consulted Creature", CardType::Creature, 1),
        bob,
        Zone::Library,
    );
    let nonmatching_stable = game
        .object(nonmatching)
        .expect("nonmatching card")
        .stable_id;

    let mut accept = SelectFirstDecisionMaker;
    let mut decline = AutoPassDecisionMaker;
    let decisions: &mut dyn DecisionMaker = if accept_cast {
        &mut accept
    } else {
        &mut decline
    };
    let target_spec = activated
        .choices
        .first()
        .cloned()
        .expect("Chaos Wand should declare target opponent");
    let assignments = vec![crate::game_state::TargetAssignment {
        spec: target_spec,
        range: 0..1,
    }];
    let mut context = crate::effects::ExecutionContext::new(source, alice, decisions)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)])
        .with_target_assignments(assignments.clone());
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        &activated.effects,
        None,
        &assignments,
    )
    .expect("Chaos Wand ability should resolve");

    let matching_zone = game
        .find_object_by_stable_id(matching_stable)
        .and_then(|id| game.object(id))
        .expect("matching card remains tracked")
        .zone;
    let nonmatching_zone = game
        .find_object_by_stable_id(nonmatching_stable)
        .and_then(|id| game.object(id))
        .expect("nonmatching card remains tracked")
        .zone;
    (matching_zone, nonmatching_zone)
}

#[test]
fn chaos_wand_keeps_targeted_consult_optional_cast_and_random_remainder() {
    let definition = parse_oracle_card_definition("Chaos Wand");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["Chaos Wand"],
        "{:#?}",
        activated_ability(&definition).effects,
    );

    let debug = format!("{:#?}", activated_ability(&definition).effects);
    assert!(debug.contains("ConsultTopOfLibraryEffect"), "{debug}");
    assert!(debug.contains("CastTaggedEffect"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnLibraryBottomEffect"),
        "{debug}"
    );
    assert!(debug.contains("order: Random"), "{debug}");
}

#[test]
fn chaos_wand_accepts_or_declines_without_stranding_the_consulted_remainder() {
    assert_eq!(resolve_chaos_wand(true), (Zone::Stack, Zone::Library));
    assert_eq!(resolve_chaos_wand(false), (Zone::Library, Zone::Library));
}

#[test]
fn grim_servant_keeps_devotion_as_the_search_mana_value_limit() {
    let definition = parse_oracle_card_definition("Grim Servant");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["Grim Servant"],
        "{:#?}",
        triggered_ability(&definition).effects,
    );

    let mut search_filter = None;
    for effect in triggered_ability(&definition)
        .effects
        .flattened_default_effects()
    {
        effect.visit_child_effects(&mut |child| {
            if let Some(choose) = child.downcast_ref::<crate::effects::ChooseObjectsEffect>()
                && choose.is_search
            {
                search_filter = Some(choose.filter.clone());
            }
            if let Some(search) = child.downcast_ref::<crate::effects::SearchLibraryEffect>() {
                search_filter = Some(search.filter.clone());
            }
        });
        if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && choose.is_search
        {
            search_filter = Some(choose.filter.clone());
        }
        if let Some(search) = effect.downcast_ref::<crate::effects::SearchLibraryEffect>() {
            search_filter = Some(search.filter.clone());
        }
    }
    let filter = search_filter.expect("typed library search");
    assert!(matches!(
        filter.mana_value,
        Some(crate::filter::Comparison::LessThanOrEqualExpr(limit))
            if matches!(limit.unhinted(), crate::effect::Value::Devotion {
                player: PlayerFilter::You,
                color: crate::color::Color::Black,
            })
    ));
}

struct ChooseDevotionLegalCard {
    legal: ObjectId,
    illegal: ObjectId,
}

impl DecisionMaker for ChooseDevotionLegalCard {
    fn decide_objects(
        &mut self,
        _game: &crate::GameState,
        context: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        let legal = context
            .candidates
            .iter()
            .find(|candidate| candidate.id == self.legal)
            .expect("mana-value-three card should be offered");
        assert!(legal.legal);
        assert!(
            context
                .candidates
                .iter()
                .find(|candidate| candidate.id == self.illegal)
                .is_none_or(|candidate| !candidate.legal),
            "mana-value-four card must exceed devotion three"
        );
        vec![self.legal]
    }
}

#[test]
fn grim_servant_search_uses_live_devotion_when_selecting_a_card() {
    let definition = parse_oracle_card_definition("Grim Servant");
    let triggered = triggered_ability(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let devotion_permanent = CardDefinitionBuilder::new(CardId::new(), "Three Black Devotion")
        .card_types(vec![CardType::Enchantment])
        .mana_cost(crate::mana::ManaCost::from_pips(vec![
            vec![crate::mana::ManaSymbol::Black],
            vec![crate::mana::ManaSymbol::Black],
            vec![crate::mana::ManaSymbol::Black],
        ]))
        .build();
    game.create_object_from_definition(&devotion_permanent, alice, Zone::Battlefield);
    let legal = game.create_object_from_definition(
        &simple_spell("Mana Value Three", CardType::Sorcery, 3),
        alice,
        Zone::Library,
    );
    let legal_stable = game.object(legal).expect("legal card").stable_id;
    let illegal = game.create_object_from_definition(
        &simple_spell("Mana Value Four", CardType::Sorcery, 4),
        alice,
        Zone::Library,
    );
    let illegal_stable = game.object(illegal).expect("illegal card").stable_id;

    let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
        game.object(source).expect("Grim Servant exists"),
        &game,
    );
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            source,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut decisions = ChooseDevotionLegalCard { legal, illegal };
    let mut context = crate::effects::ExecutionContext::new(source, alice, &mut decisions)
        .with_triggering_event(event);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Grim Servant trigger should resolve");

    assert_eq!(
        game.find_object_by_stable_id(legal_stable)
            .and_then(|id| game.object(id))
            .expect("legal card remains tracked")
            .zone,
        Zone::Hand
    );
    assert_eq!(
        game.find_object_by_stable_id(illegal_stable)
            .and_then(|id| game.object(id))
            .expect("illegal card remains tracked")
            .zone,
        Zone::Library,
    );
}

#[test]
fn commander_liara_public_payload_already_keeps_one_shared_attacked_player_value() {
    let definition = parse_oracle_card_definition("Commander Liara Portyr");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["Commander Liara Portyr"],
        "{definition:#?}"
    );
    let triggered = triggered_ability(&definition);
    let typed = format!("{:#?}", triggered.effects);
    assert!(
        typed.contains("GrantNextSpellCostReductionEffect")
            && typed.contains("ExileTopOfLibraryEffect")
            && typed.matches("PlayersBeingAttacked").count() >= 2,
        "the reduction and exile producer must retain one shared dynamic basis: {typed}"
    );
}
