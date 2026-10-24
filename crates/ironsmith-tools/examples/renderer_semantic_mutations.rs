//! Diagnostic mutations: preserve wording, alter executable fields in memory.
use ironsmith::ability::AbilityKind;
use ironsmith_core::{AmountModifierSpec, StaticAbilityPayload as P};
use serde_json::json;
fn gate(c: &mut ironsmith::ConditionExpr) -> bool {
    match c {
        ironsmith::ConditionExpr::CountComparison { comparison, .. } => {
            *comparison = ironsmith_core::Comparison::GreaterThanOrEqual(9);
            true
        }
        ironsmith::ConditionExpr::ValueComparison { right, .. } => {
            *right = ironsmith_core::Value::Fixed(9);
            true
        }
        _ => false,
    }
}
fn audit(def: &ironsmith::CardDefinition) -> Vec<String> {
    ironsmith::compiled_text::semantic_audit_lines(def)
}
fn main() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}
fn run() {
    let names: Vec<String> = [
        "Keen-Eyed Curator",
        "Crawlspace",
        "Bruvac the Grandiloquent",
        "Suppression Field",
        "Castle Ardenvale",
        "Angelic Destiny",
        "Rule of Law",
        "Sphere of Resistance",
        "Divine Presence",
        "Kenessos, Priest of Thassa",
        "Augur of Autumn",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let ps = ironsmith_tools::load_card_payloads_by_names("cards.json", &names).unwrap();
    for p in ps.values().flatten() {
        let def = ironsmith_tools::compile_runtime_definition_from_payload(p).unwrap();
        let before = ironsmith::compiled_text::compiled_text_lines(&def);
        let audit_before = audit(&def);
        if p.name == "Keen-Eyed Curator" {
            let mut changed = def.clone();
            for a in &mut changed.abilities {
                let AbilityKind::Static(s) = &a.kind else {
                    continue;
                };
                let Some(original) = s.compiled_model() else {
                    continue;
                };
                let mut m = original.clone();
                match &mut m.payload {
                    P::Anthem(x) => {
                        if let Some(c) = &mut x.condition {
                            gate(c);
                        }
                    }
                    P::GrantObjectAbilityForFilter(x) => {
                        if let Some(c) = &mut x.condition {
                            gate(c);
                        }
                    }
                    _ => continue,
                }
                a.kind =
                    AbilityKind::Static(ironsmith::static_abilities::StaticAbility::from_model(m));
            }
            let after = ironsmith::compiled_text::compiled_text_lines(&changed);
            let audit_after = audit(&changed);
            println!(
                "{}",
                json!({"name":p.name,"mutation":"both executable thresholds changed to >=9","hidden":before==after,"audit_hidden":audit_before==audit_after,"before":before,"after":after,"audit_before":audit_before,"audit_after":audit_after})
            );
        }
        for (i, a) in def.abilities.iter().enumerate() {
            let AbilityKind::Static(s) = &a.kind else {
                continue;
            };
            let Some(original) = s.compiled_model() else {
                continue;
            };
            let mut model = original.clone();
            let what = match &mut model.payload {
                P::Anthem(anthem) => {
                    if let Some(condition) = &mut anthem.condition {
                        if gate(condition) {
                            "count comparison changed to >=9"
                        } else {
                            continue;
                        }
                    } else {
                        anthem.power = ironsmith_core::AnthemValue::Fixed(9);
                        "power bonus changed to 9"
                    }
                }
                P::GrantObjectAbilityForFilter(grant) => {
                    if let Some(condition) = &mut grant.condition {
                        if gate(condition) {
                            "count comparison changed to >=9"
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }
                P::EntersTappedUnlessCondition { condition, .. } => {
                    if gate(condition) {
                        "entry gate changed to >=9"
                    } else {
                        continue;
                    }
                }
                P::EventAmountReplacement { modifier, .. } => {
                    *modifier = AmountModifierSpec::SetTo(9);
                    "event modifier changed to set amount to 9"
                }
                P::MaxCreaturesCanAttackYouEachCombat(n) => {
                    *n = 9;
                    "maximum attackers changed to 9"
                }
                P::CostIncrease(c) => {
                    c.amount = ironsmith_core::Value::Fixed(9);
                    "cost increase changed to 9"
                }
                P::ActivatedAbilityCostIncrease { increase, .. } => {
                    *increase =
                        ironsmith_core::TotalCost::mana(ironsmith::mana::ManaCost::from_symbols(
                            vec![ironsmith::mana::ManaSymbol::Generic(9)],
                        ));
                    "activation cost increase changed to 9"
                }
                P::Grants(grant) if grant.filtered_zone_surface.is_some() => {
                    grant.zone = ironsmith::zone::Zone::Graveyard;
                    "permission zone changed to graveyard"
                }
                _ => continue,
            };
            let mutated_debug = format!("{:?}", model.payload);
            let mut changed = def.clone();
            changed.abilities[i].kind = AbilityKind::Static(
                ironsmith::static_abilities::StaticAbility::from_model(model),
            );
            let after = ironsmith::compiled_text::compiled_text_lines(&changed);
            let audit_after = audit(&changed);
            println!(
                "{}",
                json!({"name":p.name,"ability":i,"mutation":what,"hidden":before==after,"audit_hidden":audit_before==audit_after,"before":before,"after":after,"audit_before":audit_before,"audit_after":audit_after,"original_payload":format!("{:?}",original.payload),"mutated_payload":mutated_debug})
            );
        }
    }
}
