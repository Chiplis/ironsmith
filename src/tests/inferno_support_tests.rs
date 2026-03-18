use std::collections::HashMap;
use std::sync::OnceLock;

use crate::cards::{CardDefinitionBuilder, generated_definition_has_unimplemented_content};
use crate::ids::CardId;

#[derive(serde::Deserialize)]
struct OracleCardFaceJson {
    name: String,
    mana_cost: Option<String>,
    type_line: Option<String>,
    oracle_text: Option<String>,
    power: Option<String>,
    toughness: Option<String>,
    loyalty: Option<String>,
    defense: Option<String>,
}

#[derive(serde::Deserialize)]
struct OracleCardJson {
    name: String,
    mana_cost: Option<String>,
    type_line: Option<String>,
    oracle_text: Option<String>,
    power: Option<String>,
    toughness: Option<String>,
    loyalty: Option<String>,
    defense: Option<String>,
    card_faces: Option<Vec<OracleCardFaceJson>>,
    lang: Option<String>,
}

#[derive(Clone)]
struct OracleCardInfo {
    parse_input: String,
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn build_parse_input(
    mana_cost: Option<String>,
    type_line: Option<String>,
    oracle_text: String,
    power: Option<String>,
    toughness: Option<String>,
    loyalty: Option<String>,
    defense: Option<String>,
) -> String {
    let mut lines = Vec::new();
    if let Some(mana_cost) = mana_cost {
        lines.push(format!("Mana cost: {mana_cost}"));
    }
    if let Some(type_line) = type_line {
        lines.push(format!("Type: {type_line}"));
    }
    if let (Some(power), Some(toughness)) = (power, toughness) {
        lines.push(format!("Power/Toughness: {power}/{toughness}"));
    }
    if let Some(loyalty) = loyalty {
        lines.push(format!("Loyalty: {loyalty}"));
    }
    if let Some(defense) = defense {
        lines.push(format!("Defense: {defense}"));
    }
    lines.push(oracle_text);
    lines.join("\n")
}

fn inferno_card_info_by_name() -> &'static HashMap<String, OracleCardInfo> {
    static ORACLE_BY_NAME: OnceLock<HashMap<String, OracleCardInfo>> = OnceLock::new();
    ORACLE_BY_NAME.get_or_init(|| {
        let raw =
            std::fs::read_to_string("cards.json").expect("read cards.json for Inferno regressions");
        let cards: Vec<OracleCardJson> =
            serde_json::from_str(&raw).expect("parse cards.json for Inferno regressions");

        let mut out = HashMap::new();
        for card in cards {
            if card.lang.as_deref().unwrap_or("en") != "en" {
                continue;
            }
            if card.type_line.as_deref().map(str::trim) == Some("Card") {
                continue;
            }

            let root_text = non_empty(card.oracle_text.clone());
            let mut face_entries = Vec::new();
            if let Some(faces) = card.card_faces {
                for face in faces {
                    let Some(face_text) = non_empty(face.oracle_text) else {
                        continue;
                    };
                    face_entries.push((
                        face.name,
                        OracleCardInfo {
                            parse_input: build_parse_input(
                                non_empty(face.mana_cost),
                                non_empty(face.type_line),
                                face_text,
                                non_empty(face.power),
                                non_empty(face.toughness),
                                non_empty(face.loyalty),
                                non_empty(face.defense),
                            ),
                        },
                    ));
                }
            }

            let primary_info = root_text
                .clone()
                .map(|oracle_text| OracleCardInfo {
                    parse_input: build_parse_input(
                        non_empty(card.mana_cost.clone()),
                        non_empty(card.type_line.clone()),
                        oracle_text,
                        non_empty(card.power.clone()),
                        non_empty(card.toughness.clone()),
                        non_empty(card.loyalty.clone()),
                        non_empty(card.defense.clone()),
                    ),
                })
                .or_else(|| face_entries.first().map(|(_, info)| info.clone()));

            if let Some(primary_info) = primary_info {
                out.entry(card.name).or_insert(primary_info);
            }

            for (face_name, info) in face_entries {
                out.entry(face_name).or_insert(info);
            }
        }

        out
    })
}

const INFERNO_CARDS: &[&str] = &[
    "Acolyte of the Inferno",
    "Calamity, Galloping Inferno",
    "Chandra, Awakened Inferno",
    "Clive, Ifrit's Dominant // Ifrit, Warden of Inferno",
    "Collective Inferno",
    "Impossible Inferno",
    "Inferno",
    "Inferno Elemental",
    "Inferno Fist",
    "Inferno Hellion",
    "Inferno Jet",
    "Inferno of the Star Mounts",
    "Inferno Project",
    "Inferno Titan",
    "Inferno Trap",
    "Invasion of Regatha // Disciples of the Inferno",
    "Jaya's Immolating Inferno",
    "Living Inferno",
    "Molten Man, Inferno Incarnate",
    "Twinferno",
    "Unleash the Inferno",
];

#[test]
fn scryfall_inferno_cards_parse_without_unsupported_markers() {
    let infos = inferno_card_info_by_name();
    let mut failures = Vec::new();

    for &name in INFERNO_CARDS {
        let Some(info) = infos.get(name) else {
            failures.push(format!("{name}: missing oracle data in cards.json"));
            continue;
        };

        let definition = match CardDefinitionBuilder::new(CardId::new(), name)
            .from_text_with_metadata(info.parse_input.clone())
        {
            Ok(definition) => definition,
            Err(err) => {
                failures.push(format!("{name}: parse error {err:?}"));
                continue;
            }
        };

        if generated_definition_has_unimplemented_content(&definition) {
            failures.push(format!(
                "{name}: contains unimplemented or unsupported markers"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "Inferno support regressions:\n{}",
        failures.join("\n")
    );
}
