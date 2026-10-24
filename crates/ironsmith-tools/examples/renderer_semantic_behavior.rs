//! Boundary scenarios for the confirmed card-type/card-count mismatch.
use ironsmith::{
    cards::builders::CardDefinitionBuilder,
    game_state::GameState,
    ids::{CardId, PlayerId},
    types::CardType as T,
    zone::Zone,
};
use serde_json::json;
fn main() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}
fn run() {
    other_cases();
    tagged_case();
    other_tagged_cases();
    attachment_case();
    let ps =
        ironsmith_tools::load_card_payloads_by_names("cards.json", &["Keen-Eyed Curator".into()])
            .unwrap();
    let def = ironsmith_tools::compile_runtime_definition_from_payload(
        ps.values().next().unwrap().first().unwrap(),
    )
    .unwrap();
    let cases = vec![
        (
            "four cards, three types",
            vec![
                vec![T::Creature],
                vec![T::Creature],
                vec![T::Artifact],
                vec![T::Land],
            ],
            true,
            false,
        ),
        (
            "four cards, one type",
            vec![vec![T::Creature]; 4],
            true,
            false,
        ),
        (
            "three cards, four types",
            vec![
                vec![T::Artifact, T::Creature],
                vec![T::Enchantment],
                vec![T::Land],
            ],
            true,
            true,
        ),
        (
            "four cards, four types",
            vec![
                vec![T::Creature],
                vec![T::Artifact],
                vec![T::Enchantment],
                vec![T::Land],
            ],
            true,
            true,
        ),
        (
            "four unrelated exiles",
            vec![
                vec![T::Creature],
                vec![T::Artifact],
                vec![T::Enchantment],
                vec![T::Land],
            ],
            false,
            false,
        ),
    ];
    for (label, types, linked, expected) in cases {
        let mut g = GameState::new(vec!["A".into(), "B".into()], 20);
        let p = PlayerId::from_index(0);
        let source = g.create_object_from_definition(&def, p, Zone::Battlefield);
        for (i, ts) in types.into_iter().enumerate() {
            let witness = CardDefinitionBuilder::new(CardId::new(), format!("Witness {i}"))
                .card_types(ts)
                .build();
            let id = g.create_object_from_definition(&witness, p, Zone::Exile);
            if linked {
                g.add_exiled_with_source_link(source, id);
            }
        }
        g.refresh_continuous_state().unwrap();
        let actual = g.calculated_power(source);
        let trample = g.object_has_static_ability_id(
            source,
            ironsmith::static_abilities::StaticAbilityId::Trample,
        );
        println!(
            "{}",
            json!({"scenario":label,"expected_power":if expected{7}else{3},"actual_power":actual,"expected_trample":expected,"actual_trample":trample,"matches_expected":actual==Some(if expected{7}else{3})&&trample==expected})
        );
    }
}

fn attachment_case() {
    let ps = ironsmith_tools::load_card_payloads_by_names("cards.json", &["Brass Knuckles".into()])
        .unwrap();
    let def = ironsmith_tools::compile_runtime_definition_from_payload(
        ps.values().next().unwrap().first().unwrap(),
    )
    .unwrap();
    for second in [false, true] {
        let mut g = GameState::new(vec!["A".into(), "B".into()], 20);
        let p = PlayerId::from_index(0);
        let source = g.create_object_from_definition(&def, p, Zone::Battlefield);
        let host = CardDefinitionBuilder::new(CardId::new(), "Host")
            .card_types(vec![T::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
            .build();
        let host = g.create_object_from_definition(&host, p, Zone::Battlefield);
        assert!(
            g.attach_object_to_target(source, ironsmith::object::AttachmentTarget::Object(host))
        );
        if second {
            let eq = CardDefinitionBuilder::new(CardId::new(), "Other equipment")
                .card_types(vec![T::Artifact])
                .subtypes(vec![ironsmith::types::Subtype::Equipment])
                .build();
            let eq = g.create_object_from_definition(&eq, p, Zone::Battlefield);
            assert!(
                g.attach_object_to_target(eq, ironsmith::object::AttachmentTarget::Object(host))
            );
        }
        g.refresh_continuous_state().unwrap();
        let actual = g.object_has_static_ability_id(
            host,
            ironsmith::static_abilities::StaticAbilityId::DoubleStrike,
        );
        println!(
            "{}",
            json!({"name":"Brass Knuckles","scenario":if second{"host with two equipment"}else{"host with only Brass Knuckles"},"expected_double_strike":second,"actual_double_strike":actual,"matches_expected":actual==second})
        );
    }
}

fn other_tagged_cases() {
    use ironsmith::ability::AbilityKind;
    fn find(e: &ironsmith::Effect, out: &mut Vec<ironsmith::ConditionExpr>) {
        if let Some(c) = e.downcast_ref::<ironsmith::effects::ConditionalEffect>() {
            if format!("{:?}", c.condition).contains("CountComparison")
                && format!("{:?}", c.condition).contains("__it__")
            {
                out.push(c.condition.clone());
            }
        }
        e.visit_child_effects(&mut |child| find(child, out));
    }
    let names: Vec<String> = [
        "Bazaar of Wonders",
        "Candles of Leng",
        "Spellweaver Helix",
        "Guardian Project",
        "Retraced Image",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let ps = ironsmith_tools::load_card_payloads_by_names("cards.json", &names).unwrap();
    for payload in ps.values().flatten() {
        let def = ironsmith_tools::compile_runtime_definition_from_payload(payload).unwrap();
        let mut conditions = vec![];
        if let Some(program) = &def.spell_effect {
            for e in program.all_effects() {
                find(e, &mut conditions);
            }
        }
        for a in &def.abilities {
            let effects = match &a.kind {
                AbilityKind::Activated(a) => a.effects.all_effects(),
                AbilityKind::Triggered(a) => {
                    if let Some(c) = &a.intervening_if {
                        if format!("{c:?}").contains("CountComparison")
                            && format!("{c:?}").contains("__it__")
                        {
                            conditions.push(c.clone());
                        }
                    }
                    a.effects.all_effects()
                }
                _ => vec![],
            };
            for e in effects {
                find(e, &mut conditions);
            }
        }
        for condition in conditions {
            for same in [true, false] {
                let mut g = GameState::new(vec!["A".into(), "B".into()], 20);
                let p = PlayerId::from_index(0);
                let source = g.create_object_from_definition(&def, p, Zone::Battlefield);
                let ty = if ["Guardian Project", "Retraced Image"].contains(&payload.name.as_str())
                {
                    T::Creature
                } else {
                    T::Sorcery
                };
                let w = CardDefinitionBuilder::new(CardId::new(), "Matching Spell")
                    .card_types(vec![ty])
                    .build();
                let zone = if payload.name == "Spellweaver Helix" {
                    Zone::Exile
                } else if payload.name == "Retraced Image" {
                    Zone::Battlefield
                } else {
                    Zone::Graveyard
                };
                let reference = g.create_object_from_definition(&w, p, zone);
                if zone == Zone::Exile {
                    g.add_exiled_with_source_link(source, reference);
                }
                let tested = CardDefinitionBuilder::new(
                    CardId::new(),
                    if same {
                        "Matching Spell"
                    } else {
                        "Other Spell"
                    },
                )
                .card_types(vec![ty])
                .build();
                let tested_zone = if payload.name == "Guardian Project" {
                    Zone::Battlefield
                } else if payload.name == "Retraced Image" {
                    Zone::Hand
                } else {
                    Zone::Stack
                };
                let tested = g.create_object_from_definition(&tested, p, tested_zone);
                let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
                let mut ctx = ironsmith::effects::EffectContext::new(source, p, &mut dm);
                ctx.tag_object(
                    ironsmith::TagKey::from("__it__"),
                    ironsmith::snapshot::ObjectSnapshot::from_object(g.object(tested).unwrap(), &g),
                );
                let actual =
                    ironsmith::condition_eval::evaluate_condition_resolution(&g, &condition, &ctx)
                        .unwrap();
                let expected = if payload.name == "Guardian Project" {
                    !same
                } else {
                    same
                };
                println!(
                    "{}",
                    json!({"name":payload.name,"scenario":if same{"matching names, bound __it__"}else{"different names, bound __it__"},"expected_condition":expected,"actual_condition":actual,"matches_expected":actual==expected,"condition":format!("{condition:?}")})
                );
            }
        }
    }
}

fn tagged_case() {
    use ironsmith::ability::AbilityKind;
    let p = PlayerId::from_index(0);
    let ps =
        ironsmith_tools::load_card_payloads_by_names("cards.json", &["Jester's Scepter".into()])
            .unwrap();
    let def = ironsmith_tools::compile_runtime_definition_from_payload(
        ps.values().next().unwrap().first().unwrap(),
    )
    .unwrap();
    let condition = def
        .abilities
        .iter()
        .find_map(|a| {
            let AbilityKind::Activated(a) = &a.kind else {
                return None;
            };
            a.effects.all_effects().into_iter().find_map(|e| {
                e.downcast_ref::<ironsmith::effects::ConditionalEffect>()
                    .map(|c| c.condition.clone())
            })
        })
        .unwrap();
    for same in [true, false] {
        let mut g = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = g.create_object_from_definition(&def, p, Zone::Battlefield);
        let paid = CardDefinitionBuilder::new(CardId::new(), "Matching Spell")
            .card_types(vec![T::Instant])
            .build();
        let spell = CardDefinitionBuilder::new(
            CardId::new(),
            if same {
                "Matching Spell"
            } else {
                "Other Spell"
            },
        )
        .card_types(vec![T::Instant])
        .build();
        let paid = g.create_object_from_definition(&paid, p, Zone::Graveyard);
        let spell = g.create_object_from_definition(&spell, p, Zone::Stack);
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut ctx = ironsmith::effects::EffectContext::new(source, p, &mut dm);
        ctx.tag_object(
            ironsmith::TagKey::from("__it__"),
            ironsmith::snapshot::ObjectSnapshot::from_object(g.object(spell).unwrap(), &g),
        );
        ctx.tag_object(
            ironsmith::TagKey::from("zone_cost_0"),
            ironsmith::snapshot::ObjectSnapshot::from_object(g.object(paid).unwrap(), &g),
        );
        let actual =
            ironsmith::condition_eval::evaluate_condition_resolution(&g, &condition, &ctx).unwrap();
        println!(
            "{}",
            json!({"name":"Jester's Scepter","scenario":if same{"matching spell and paid card names"}else{"different spell and paid card names"},"expected_condition":same,"actual_condition":actual,"matches_expected":actual==same})
        );
    }
}

fn other_cases() {
    use ironsmith::ability::AbilityKind;
    use ironsmith_core::StaticAbilityPayload as P;
    let ps = ironsmith_tools::load_card_payloads_by_names(
        "cards.json",
        &["Augur of Autumn".into(), "Earth Rumble Wrestlers".into()],
    )
    .unwrap();
    for payload in ps.values().flatten() {
        let def = ironsmith_tools::compile_runtime_definition_from_payload(payload).unwrap();
        let scenarios = if payload.name == "Augur of Autumn" {
            vec![
                (
                    "three creatures all power 2",
                    vec![(vec![T::Creature], 2), (vec![T::Creature], 2)],
                    false,
                ),
                (
                    "three different powers",
                    vec![(vec![T::Creature], 1), (vec![T::Creature], 3)],
                    true,
                ),
            ]
        } else {
            vec![
                (
                    "old land creature, no entries this turn",
                    vec![(vec![T::Land, T::Creature], 2)],
                    true,
                ),
                (
                    "old ordinary creature, no entries this turn",
                    vec![(vec![T::Creature], 2)],
                    false,
                ),
            ]
        };
        for (scenario, witnesses, expected) in scenarios {
            let mut g = GameState::new(vec!["A".into(), "B".into()], 20);
            let p = PlayerId::from_index(0);
            let source = g.create_object_from_definition(&def, p, Zone::Battlefield);
            for (i, (types, power)) in witnesses.into_iter().enumerate() {
                let w = CardDefinitionBuilder::new(CardId::new(), format!("Witness {i}"))
                    .card_types(types)
                    .power_toughness(ironsmith::card::PowerToughness::fixed(power, 3))
                    .build();
                g.create_object_from_definition(&w, p, Zone::Battlefield);
            }
            g.turn_store.turn_history = Default::default();
            g.refresh_continuous_state().unwrap();
            let condition = def
                .abilities
                .iter()
                .find_map(|a| {
                    let AbilityKind::Static(s) = &a.kind else {
                        return None;
                    };
                    match &s.compiled_model()?.payload {
                        P::Anthem(a) => a.condition.as_ref(),
                        P::Conditional { condition, .. } => Some(condition),
                        _ => None,
                    }
                })
                .unwrap();
            let context = ironsmith::condition_eval::ExternalEvaluationContext {
                source,
                controller: p,
                filter_source: Some(source),
                ..Default::default()
            };
            let actual =
                ironsmith::condition_eval::evaluate_condition_external(&g, condition, &context);
            println!(
                "{}",
                json!({"name":payload.name,"scenario":scenario,"expected_condition":expected,"actual_condition":actual,"matches_expected":actual==expected,"power":g.calculated_power(source),"trample":g.object_has_static_ability_id(source,ironsmith::static_abilities::StaticAbilityId::Trample)})
            );
        }
    }
}
