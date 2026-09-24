//! Planar Nexus: "This land is every nonbasic land type. {T}: Add {C}.
//! {1}, {T}: Add one mana of any color."
use ironsmith::ability::AbilityKind;
use ironsmith::target::ObjectFilter;
use ironsmith::{GameState, PlayerId, Subtype, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Planar Nexus",
    )
    .unwrap()
    .remove(0)
}

#[test]
fn strict_snapshot_and_full_quality_gate() {
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(
        snapshot.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(
        snapshot.parse_error.is_none() && !snapshot.parse_lossy && !snapshot.has_unimplemented,
        "{snapshot:#?}"
    );
    assert!(snapshot.similarity_score >= 0.99, "{snapshot:#?}");
}

fn battlefield_nexus() -> (GameState, ironsmith::ObjectId) {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let nexus = game.create_object_from_definition(&def, PlayerId::from_index(0), Zone::Battlefield);
    (game, nexus)
}

#[test]
fn has_every_nonbasic_land_type_and_no_basic_type() {
    let (game, nexus) = battlefield_nexus();
    let subtypes = game.calculated_subtypes(nexus);
    for nonbasic in Subtype::nonbasic_land_types() {
        assert!(subtypes.contains(nonbasic), "missing {nonbasic:?}: {subtypes:?}");
    }
    assert!(
        subtypes.iter().all(|subtype| !subtype.is_basic_land_type()),
        "no basic land types: {subtypes:?}"
    );
}

#[test]
fn counts_for_land_type_filters_such_as_locus_desert_gate_and_urzas() {
    let (game, nexus) = battlefield_nexus();
    let alice = PlayerId::from_index(0);
    let ctx = ironsmith::effects::EffectContext::new_default(nexus, alice);
    let count = |subtype: Subtype| {
        ironsmith::effects::helpers::resolve_value(
            &game,
            &ironsmith::Value::Count(ObjectFilter::land().with_subtype(subtype)),
            &ctx,
        )
        .unwrap()
    };
    for subtype in [Subtype::Locus, Subtype::Desert, Subtype::Gate, Subtype::Urzas, Subtype::Cave] {
        assert_eq!(count(subtype), 1, "{subtype:?} land count should include Planar Nexus");
    }
    assert_eq!(count(Subtype::Forest), 0);
}

#[test]
fn has_colorless_and_filtered_any_color_mana_abilities() {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let mana_abilities: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) if activated.is_mana_ability() => Some(activated),
            _ => None,
        })
        .collect();
    assert_eq!(mana_abilities.len(), 2, "{mana_abilities:#?}");
    let debug = format!("{mana_abilities:#?}");
    assert!(debug.contains("Colorless"), "{{T}}: Add {{C}}");
    assert!(debug.contains("AddManaOfAnyColorEffect"), "{{1}}, {{T}}: any color");
}
