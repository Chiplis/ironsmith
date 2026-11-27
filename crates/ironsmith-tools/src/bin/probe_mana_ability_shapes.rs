//! Census of mana-ability shapes in the played card pool.
//!
//!   probe_mana_ability_shapes [format ...]   (default: every catalog format)
//!
//! Compiles every card any catalog deck plays, takes each activated or
//! triggered ability that produces mana, and groups them by structure: where
//! the ability functions, what its cost is made of, how its output is
//! described, and what else it does. The payment tiers treat sources by these
//! shapes, so the list says which shapes exist, how many cards have each, and
//! which ones a fast tier does not model.
use ironsmith::ability::AbilityKind;
use ironsmith::engine::prelude::Zone;
use ironsmith_tools::{compile_definition_from_payload, default_cards_path, normalize_lookup_name};
use std::collections::{BTreeMap, BTreeSet};

#[derive(serde::Deserialize)]
struct DeckEntry {
    name: String,
}
#[derive(serde::Deserialize)]
struct DeckFile {
    mainboard: Vec<DeckEntry>,
}

/// The leading identifier of a `Debug` rendering: the variant or type name.
fn head(text: &str) -> String {
    text.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect()
}

fn cost_kind(cost: &ironsmith::costs::Cost) -> String {
    if cost.requires_tap() {
        "tap".into()
    } else if let Some(mana) = cost.mana_cost_ref() {
        let typed = mana.pips().iter().flatten().any(|symbol| {
            !matches!(symbol, ironsmith::mana::ManaSymbol::Generic(_))
        });
        if typed { "mana(typed)".into() } else { "mana(generic)".into() }
    } else if cost.is_life_cost() {
        "life".into()
    } else if cost.is_sacrifice_self() {
        "sacrifice-self".into()
    } else if cost.is_sacrifice() {
        "sacrifice-other".into()
    } else if cost.is_discard() {
        "discard".into()
    } else if cost.is_exile_from_hand() {
        "exile-from-hand".into()
    } else if cost.is_remove_counters() {
        "remove-counters".into()
    } else {
        let display = cost.display();
        let lower = display.to_ascii_lowercase();
        if lower.starts_with("exile this") {
            "exile-self".into()
        } else if lower.contains("untap") {
            "untap".into()
        } else {
            format!("other({display})")
        }
    }
}

fn main() {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}

fn run() {
    let catalog = default_cards_path().parent().unwrap().join("web/ui/public/catalog");
    let mut formats: Vec<String> = std::env::args().skip(1).collect();
    if formats.is_empty() {
        formats = std::fs::read_dir(&catalog)
            .expect("catalog directory")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().join("details").is_dir())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        formats.sort();
    }
    let mut names = BTreeSet::new();
    for format in &formats {
        let Ok(entries) = std::fs::read_dir(catalog.join(format).join("details")) else {
            continue;
        };
        for path in entries.filter_map(|entry| entry.ok().map(|entry| entry.path())) {
            let Some(deck) = std::fs::read(&path)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<DeckFile>(&bytes).ok())
            else {
                continue;
            };
            names.extend(deck.mainboard.into_iter().map(|entry| entry.name));
        }
    }
    let names: Vec<String> = names.into_iter().collect();
    let payloads =
        ironsmith_tools::load_card_payloads_by_names(default_cards_path().to_str().unwrap(), &names)
            .expect("load card payloads");
    eprintln!("{} formats, {} distinct cards", formats.len(), names.len());

    // shape -> (ability count, example card names)
    let mut shapes: BTreeMap<String, (usize, BTreeSet<String>)> = BTreeMap::new();
    let mut compiled = 0usize;
    for name in &names {
        let Some(definition) = payloads
            .get(&normalize_lookup_name(name))
            .and_then(|payload| payload.first())
            .and_then(|payload| compile_definition_from_payload(payload).ok())
        else {
            continue;
        };
        compiled += 1;
        for ability in &definition.abilities {
            let zones = if ability.functional_zones == [Zone::Battlefield] {
                String::new()
            } else {
                format!(" zones={:?}", ability.functional_zones)
            };
            let shape = match &ability.kind {
                AbilityKind::Activated(activated) if activated.is_mana_ability() => {
                    let mut costs: Vec<String> = activated
                        .mana_cost
                        .as_all()
                        .map(|costs| costs.iter().map(cost_kind).collect())
                        .unwrap_or_else(|| vec!["alternatives".into()]);
                    costs.sort();
                    let mut production = Vec::new();
                    let mut side_effects = Vec::new();
                    if activated.mana_output.as_ref().is_some_and(|symbols| !symbols.is_empty()) {
                        production.push("Fixed".to_string());
                    }
                    for effect in activated.effects.iter() {
                        match effect.mana_production() {
                            Some(description) => production.push(head(&format!("{description:?}"))),
                            None => side_effects.push(head(&format!("{effect:?}"))),
                        }
                    }
                    production.sort();
                    production.dedup();
                    side_effects.sort();
                    side_effects.dedup();
                    let mut riders = Vec::new();
                    if let Some(condition) = &activated.activation_condition {
                        riders.push(format!("condition={}", head(&format!("{condition:?}"))));
                    }
                    if !activated.activation_restrictions.is_empty() {
                        riders.push("restrictions".into());
                    }
                    if !activated.additional_restrictions.is_empty() {
                        riders.push("additional-restrictions".into());
                    }
                    if !activated.choices.is_empty() {
                        riders.push("choices".into());
                    }
                    if !activated.mana_usage_restrictions.is_empty() {
                        riders.push("restricted-mana".into());
                    }
                    let timing = head(&format!("{:?}", activated.timing));
                    format!(
                        "activated{zones} cost=[{}] makes=[{}] also=[{}] timing={timing} {}",
                        costs.join("+"),
                        production.join(","),
                        side_effects.join(","),
                        riders.join(" "),
                    )
                }
                AbilityKind::Triggered(triggered)
                    if triggered
                        .effects
                        .all_effects()
                        .iter()
                        .any(|effect| effect.contains_mana_production()) =>
                {
                    format!("triggered{zones} (produces mana)")
                }
                _ => continue,
            };
            let entry = shapes.entry(shape).or_default();
            entry.0 += 1;
            if entry.1.len() < 4 {
                entry.1.insert(name.clone());
            }
        }
    }
    eprintln!("{compiled} compiled, {} shapes", shapes.len());
    let mut ordered: Vec<_> = shapes.into_iter().collect();
    ordered.sort_by(|left, right| right.1.0.cmp(&left.1.0));
    for (shape, (count, examples)) in ordered {
        println!(
            "{count:>5}  {shape}  e.g. {}",
            examples.into_iter().collect::<Vec<_>>().join("; ")
        );
    }
}
