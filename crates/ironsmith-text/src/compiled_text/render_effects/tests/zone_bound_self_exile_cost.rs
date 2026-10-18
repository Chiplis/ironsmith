use super::*;

fn definition(text: &str, card_type: CardType) -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Zone Cost Fixture")
        .card_types(vec![card_type])
        .parse_text(text)
        .expect("zone-bound cost compiles")
}

#[test]
fn zone_bound_self_exile_cost_preserves_origin_for_mana_and_nonmana_abilities() {
    for card_type in [
        CardType::Creature,
        CardType::Artifact,
        CardType::Enchantment,
    ] {
        for zone in ["hand", "graveyard"] {
            for prefix in ["", "{3}, "] {
                for effect in ["Draw a card.", "Add {C}{C}."] {
                    let text = format!("{prefix}Exile this card from your {zone}: {effect}");
                    let card = definition(&text, card_type);
                    assert_eq!(
                        crate::compiled_text::compiled_text_lines(&card).join("\n"),
                        text
                    );
                }
            }
        }
    }
}

#[test]
fn zone_bound_self_exile_cost_land_grants_keep_the_cost_and_duration() {
    for (cost, mana) in [
        ("{3}", "{C}{C}"),
        ("{2}", "{U}, {B}, or {R}"),
        ("{2}", "{B}, {R}, or {G}"),
        ("{2}", "{R}, {G}, or {W}"),
        ("{2}", "{W}, {U}, or {B}"),
        ("{2}", "{G}, {W}, or {U}"),
    ] {
        let text = format!(
            "{cost}, Exile this card from your hand: Target land gains \"{{T}}: Add {mana}\" until this card is cast from exile. You may cast this card for as long as it remains exiled."
        );
        let card = definition(&text, CardType::Creature);
        let rendered = crate::compiled_text::compiled_text_lines(&card).join("\n");
        assert!(
            rendered.starts_with(&format!("{cost}, Exile this card from your hand:")),
            "{rendered}"
        );
        assert!(
            rendered.contains(&format!(
                "Target land gains \"{{T}}: Add {mana}\" until this card is cast from exile."
            )),
            "{rendered}"
        );
        assert!(
            rendered.contains("for as long as it remains exiled"),
            "{rendered}"
        );
    }
}

#[test]
fn zone_bound_self_exile_cost_requires_a_single_origin_and_a_source_cost() {
    let mut card = definition(
        "Exile this card from your hand: Draw a card.",
        CardType::Creature,
    );
    let ability = card
        .abilities
        .iter_mut()
        .find(|a| matches!(a.kind, AbilityKind::Activated(_)))
        .unwrap();
    let raw = Some("Exile this source".to_string());
    for zones in [
        vec![Zone::Battlefield],
        vec![Zone::Hand, Zone::Graveyard],
        vec![Zone::Battlefield, Zone::Graveyard],
        vec![],
    ] {
        ability.functional_zones = zones;
        assert_eq!(
            normalize_zone_bound_self_exile_cost(raw.clone(), ability),
            raw
        );
    }
    ability.functional_zones = vec![Zone::Hand];
    let AbilityKind::Activated(activated) = &mut ability.kind else {
        unreachable!()
    };
    activated.mana_cost = crate::cost::TotalCost::free();
    assert_eq!(
        normalize_zone_bound_self_exile_cost(raw.clone(), ability),
        raw
    );
}

#[test]
fn zone_bound_self_exile_cost_is_idempotent_and_preserves_quotes() {
    let card = definition(
        "{3}, Exile this card from your hand: Draw a card.",
        CardType::Creature,
    );
    let ability = card
        .abilities
        .iter()
        .find(|a| matches!(a.kind, AbilityKind::Activated(_)))
        .unwrap();
    let raw = Some("{3}, Exile this source, Reveal a card with \"Exile this source\"".to_string());
    let once = normalize_zone_bound_self_exile_cost(raw, ability);
    assert_eq!(
        once.as_deref(),
        Some("{3}, Exile this card from your hand, Reveal a card with \"Exile this source\"")
    );
    assert_eq!(
        normalize_zone_bound_self_exile_cost(once.clone(), ability),
        once
    );
}

#[test]
fn zone_bound_self_exile_cost_does_not_change_other_card_or_battlefield_costs() {
    for text in [
        "Exile this creature: Draw a card.",
        "{2}, Exile a card from your hand: Draw a card.",
        "{2}, Exile a card from your graveyard: Draw a card.",
        "Discard this card: Draw a card.",
    ] {
        let card = definition(text, CardType::Creature);
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&card).join("\n"),
            text
        );
    }
}

#[test]
fn zone_bound_self_exile_cost_accepts_wrapped_and_alternative_costs() {
    let mut card = definition(
        "Exile this card from your hand: Draw a card.",
        CardType::Creature,
    );
    let ability = card
        .abilities
        .iter_mut()
        .find(|a| matches!(a.kind, AbilityKind::Activated(_)))
        .unwrap();
    let AbilityKind::Activated(activated) = &mut ability.kind else {
        unreachable!()
    };
    let original = activated.mana_cost.clone();
    let wrapped = original
        .try_map(|cost| {
            crate::costs::Cost::try_effect(Effect::with_id(17, cost.effect_ref().unwrap().clone()))
        })
        .unwrap();
    activated.mana_cost = crate::cost::TotalCost::one_of(vec![wrapped.clone(), wrapped]);
    assert_eq!(
        normalize_zone_bound_self_exile_cost(
            Some("Exile this source or exile this source".into()),
            ability
        )
        .as_deref(),
        Some("Exile this card from your hand or exile this card from your hand")
    );
}
