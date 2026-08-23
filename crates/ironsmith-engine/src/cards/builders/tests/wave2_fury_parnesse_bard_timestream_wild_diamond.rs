#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn vanilla_creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build()
}

#[test]
fn fury_keeps_evoke_surface_and_hides_only_its_exact_sacrifice_helper() {
    let definition = parse_oracle_card_definition("Fury");
    let lines = canonical_compiled_lines(&definition);
    assert!(lines.contains(&"Evoke—Exile a red card from your hand.".to_string()));
    assert!(
        !lines.iter().any(|line| line.contains("rather than pay")),
        "evoke must render as its keyword cost: {lines:#?}"
    );
    assert!(
        !lines
            .iter()
            .any(|line| line.contains("evoke cost was paid")),
        "the executable evoke helper is not a separately authored line: {lines:#?}"
    );
    assert_eq!(
        definition
            .alternative_casts
            .iter()
            .filter(|method| method.is_composed_cost() && method.name() == "Evoke")
            .count(),
        1
    );
}

#[test]
fn parnesse_keeps_target_opponent_as_copier_and_retargeting_decider() {
    let definition = parse_oracle_card_definition("Parnesse, the Subtle Brush");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Whenever you or a permanent you control becomes the target of a spell or ability an opponent controls, counter that spell or ability unless that player pays 4 life.",
            "Whenever you copy a spell, up to one target opponent may also copy that spell. They may choose new targets for that copy.",
        ]
    );

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .downcast_ref::<crate::triggers::SpellCopiedTrigger>()
                    .is_some() =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Parnesse should retain the spell-copied trigger");
    let [_, target_effect, may_effect] = triggered.effects.flattened_default_effects() else {
        panic!("expected triggering tag, opponent target, and optional copy: {triggered:#?}");
    };
    let target = target_effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
        .expect("the opponent must remain an announced target");
    assert!(target.target.is_target());
    assert_eq!(target.target.count().min, 0);
    assert_eq!(target.target.count().max, Some(1));
    assert!(matches!(
        target.target.base(),
        ChooseSpec::Player(PlayerFilter::Opponent)
    ));

    let may = may_effect
        .downcast_ref::<crate::effects::MayEffect>()
        .expect("the selected opponent must receive the optional copy");
    assert_eq!(may.decider, Some(PlayerFilter::target_opponent()));
    let [copy_effect, retarget_effect] = may.effects.as_slice() else {
        panic!("copy and retarget must stay in the same opponent-scoped offer: {may:#?}");
    };
    let tagged = copy_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("the created stack copy must remain tagged");
    let with_id = tagged
        .effect
        .downcast_ref::<crate::effects::WithIdEffect>()
        .expect("the retarget instruction must reference the copy result");
    let copy = with_id
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()
        .expect("expected a typed spell-copy effect");
    assert_eq!(copy.copier, PlayerFilter::target_opponent());
    let retarget = retarget_effect
        .downcast_ref::<crate::effects::ChooseNewTargetsEffect>()
        .expect("expected typed copied-spell retargeting");
    assert_eq!(retarget.from_effect, with_id.id);
    assert_eq!(retarget.chooser, Some(PlayerFilter::target_opponent()));
}

#[test]
fn bard_class_colored_reduction_keeps_its_limiting_sentence() {
    let definition = parse_oracle_card_definition("Bard Class");
    let lines = canonical_compiled_lines(&definition);
    assert!(lines.contains(
        &"Legendary spells you cast cost {R}{G} less to cast. This effect reduces only the amount of colored mana you pay."
            .to_string()
    ));

    let reduction = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => static_ability.cost_reduction_mana_cost(),
            _ => None,
        })
        .expect("Bard Class should retain its typed legendary-spell reduction");
    assert_eq!(reduction.filter.supertypes, [Supertype::Legendary]);
    assert_eq!(reduction.reduction.to_oracle(), "{R}{G}");
}

#[test]
fn timestream_navigator_uses_a_source_move_to_library_bottom_cost() {
    let definition = parse_oracle_card_definition("Timestream Navigator");
    let line = canonical_compiled_lines(&definition)
        .into_iter()
        .find(|line| line.contains("{2}{U}{U}"))
        .expect("Timestream Navigator should retain its activation");
    assert!(
        line.contains("Put this creature on the bottom of its owner's library"),
        "the activation must render the source-moving cost: {line}"
    );
    assert!(!line.contains("creature counter"), "{line}");

    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Timestream Navigator should have an activated ability");
    let move_source = activated
        .mana_cost
        .costs()
        .iter()
        .filter_map(|cost| cost.effect_ref())
        .find_map(|effect| effect.downcast_ref::<crate::effects::MoveToZoneEffect>())
        .expect("the bottom-library source move must remain an executable cost");
    assert_eq!(move_source.zone, Zone::Library);
    assert!(!move_source.to_top);
    assert!(matches!(move_source.target.base(), ChooseSpec::Source));
    assert_eq!(
        move_source.target.source_reference_surface(),
        Some(&SourceReferenceSurface::ThisPermanentType(
            "this creature".to_string()
        ))
    );
}

#[test]
fn wild_mammoth_resolves_only_the_unique_creature_control_leader() {
    let definition = parse_oracle_card_definition("Wild Mammoth");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "At the beginning of your upkeep, if a player controls more creatures than each other player, the player who controls the most creatures gains control of this creature."
        ]
    );
    let mut game = crate::GameState::new(
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
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.create_object_from_definition(&vanilla_creature("Bob One"), bob, Zone::Battlefield);
    game.create_object_from_definition(&vanilla_creature("Bob Two"), bob, Zone::Battlefield);
    game.create_object_from_definition(
        &vanilla_creature("Charlie One"),
        charlie,
        Zone::Battlefield,
    );
    let ctx = crate::effects::ExecutionContext::new_default(source, alice);
    let leader = PlayerFilter::ControlsMost {
        filter: Box::new(ObjectFilter::creature()),
    };
    assert_eq!(
        crate::effects::helpers::resolve_player_filter(&game, &leader, &ctx)
            .expect("Bob should be the unique creature-control leader"),
        bob
    );

    game.create_object_from_definition(
        &vanilla_creature("Charlie Two"),
        charlie,
        Zone::Battlefield,
    );
    assert!(
        crate::effects::helpers::resolve_player_filter(&game, &leader, &ctx).is_err(),
        "a tied-most set must not select an arbitrary controller"
    );
}

#[test]
fn diamond_kaleidoscope_keeps_prism_on_both_token_and_sacrifice_filter() {
    let definition = parse_oracle_card_definition("Diamond Kaleidoscope");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "{3}, {T}: Create a 0/1 colorless Prism artifact creature token.",
            "Sacrifice a Prism token: Add one mana of any color.",
        ]
    );
    let activated = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(activated.len(), 2);
    let create = activated[0]
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::CreateTokenEffect>())
        .expect("the first activation should create a typed token");
    assert!(create.token.card.subtypes.contains(&Subtype::Prism));

    let mut sacrifice_cost = activated[1]
        .mana_cost
        .costs()
        .iter()
        .filter_map(|cost| cost.effect_ref())
        .next()
        .expect("the second activation should retain a typed sacrifice cost");
    while let Some(inner) = sacrifice_cost.transparent_child_effect() {
        sacrifice_cost = inner;
    }
    let sacrifice = sacrifice_cost
        .downcast_ref::<crate::effects::SacrificeEffect>()
        .expect("the second activation cost should lower to a sacrifice effect");
    assert_eq!(sacrifice.filter.subtypes, [Subtype::Prism]);
    assert!(sacrifice.filter.token);
}
