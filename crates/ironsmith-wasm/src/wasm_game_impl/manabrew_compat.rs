use serde_json::{Value, json};

type JsonMap = serde_json::Map<String, Value>;

fn as_object(value: &Value) -> Option<&JsonMap> {
    value.as_object()
}

fn object_value(value: &Value) -> &JsonMap {
    static EMPTY: std::sync::OnceLock<JsonMap> = std::sync::OnceLock::new();
    value.as_object().unwrap_or_else(|| EMPTY.get_or_init(JsonMap::new))
}

fn get_any<'a>(object: &'a JsonMap, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| object.get(*key))
}

fn string_value(value: Option<&Value>, fallback: &str) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_string()
}

fn number_value(value: Option<&Value>, fallback: i64) -> i64 {
    value.and_then(Value::as_i64).unwrap_or(fallback)
}

fn bool_value(value: Option<&Value>, fallback: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(fallback)
}

fn array_value(value: Option<&Value>) -> &[Value] {
    value.and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn player_slot(value: Option<&Value>) -> String {
    format!("player-{}", number_value(value, 0).max(0))
}

fn card_id(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(value)) => format!("card-{value}"),
        Some(Value::Number(value)) => format!("card-{value}"),
        Some(value) if !value.is_null() => format!("card-{value}"),
        _ => "card-unknown".to_string(),
    }
}

fn stack_id(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(value)) => format!("stack-{value}"),
        Some(Value::Number(value)) => format!("stack-{value}"),
        Some(value) if !value.is_null() => format!("stack-{value}"),
        _ => "stack-unknown".to_string(),
    }
}

fn hidden_card(owner_id: &str, index: usize) -> Value {
    json!({
        "id": format!("hidden-{owner_id}-{index}"),
        "identity": { "name": "Hidden Card", "setCode": "", "cardNumber": "", "isToken": false },
        "controllerId": owner_id,
        "ownerId": owner_id,
        "zoneId": "hand",
        "color": "",
        "manaCost": "",
        "cmc": 0,
        "types": [],
        "subtypes": [],
        "supertypes": [],
        "power": null,
        "toughness": null,
        "text": "",
        "tapped": false,
        "isCrewed": false,
        "isAttacking": false,
        "keywords": [],
        "counters": {},
        "damage": 0,
        "summoningSick": false,
        "isCopy": false,
        "isDoubleFaced": false,
        "isTransformed": false,
        "isFaceDown": true,
        "isBestowed": false,
        "phasedOut": false,
        "exerted": false,
        "isRingBearer": false,
        "attachmentIds": [],
        "isMadnessExiled": false,
        "isPlotted": false,
        "isWarpExiled": false,
        "foil": false,
        "wouldDieInCombat": false
    })
}

fn identity(name: String, token: bool) -> Value {
    json!({ "name": name, "setCode": "", "cardNumber": "", "isToken": token })
}

fn split_power_toughness(value: Option<&Value>) -> (Value, Value) {
    let Some(raw) = value.and_then(Value::as_str) else {
        return (Value::Null, Value::Null);
    };
    let mut parts = raw.split('/');
    let power = parts.next().filter(|part| !part.is_empty());
    let toughness = parts.next().filter(|part| !part.is_empty());
    (
        power.map_or(Value::Null, |value| json!(value)),
        toughness.map_or(Value::Null, |value| json!(value)),
    )
}

fn counters(value: Option<&Value>) -> Value {
    let mut result = JsonMap::new();
    for entry in array_value(value) {
        let counter = object_value(entry);
        let kind = string_value(get_any(counter, &["kind"]), "");
        if !kind.is_empty() {
            result.insert(kind, json!(number_value(get_any(counter, &["amount"]), 0)));
        }
    }
    Value::Object(result)
}

fn mana_pool(value: Option<&Value>) -> Value {
    let pool = value.and_then(as_object);
    json!({
        "W": number_value(pool.and_then(|pool| get_any(pool, &["white"])), 0),
        "U": number_value(pool.and_then(|pool| get_any(pool, &["blue"])), 0),
        "B": number_value(pool.and_then(|pool| get_any(pool, &["black"])), 0),
        "R": number_value(pool.and_then(|pool| get_any(pool, &["red"])), 0),
        "G": number_value(pool.and_then(|pool| get_any(pool, &["green"])), 0),
        "C": number_value(pool.and_then(|pool| get_any(pool, &["colorless"])), 0),
    })
}

fn card_types(card: &JsonMap) -> Value {
    Value::Array(
        array_value(get_any(card, &["card_types", "cardTypes"]))
            .iter()
            .filter_map(Value::as_str)
            .map(|value| json!(value))
            .collect(),
    )
}

fn card_from_zone_card(value: &Value, owner_id: &str, controller_id: &str, zone_id: &str) -> Value {
    let card = object_value(value);
    let name = string_value(get_any(card, &["name"]), "Hidden Card");
    let (power, toughness) = split_power_toughness(get_any(card, &["power_toughness", "powerToughness"]));
    json!({
        "id": card_id(get_any(card, &["id", "stable_id", "stableId"])),
        "identity": identity(name, false),
        "controllerId": controller_id,
        "ownerId": owner_id,
        "zoneId": zone_id,
        "color": "",
        "manaCost": string_value(get_any(card, &["mana_cost", "manaCost"]), ""),
        "cmc": 0,
        "types": card_types(card),
        "subtypes": [],
        "supertypes": [],
        "power": power,
        "toughness": toughness,
        "text": string_value(get_any(card, &["oracle_text", "oracleText"]), ""),
        "tapped": false,
        "isCrewed": false,
        "isAttacking": false,
        "keywords": [],
        "counters": {},
        "damage": 0,
        "summoningSick": false,
        "isCopy": false,
        "isDoubleFaced": false,
        "isTransformed": false,
        "isFaceDown": false,
        "isBestowed": false,
        "phasedOut": false,
        "exerted": false,
        "isRingBearer": false,
        "attachmentIds": [],
        "isMadnessExiled": false,
        "isPlotted": false,
        "isWarpExiled": false,
        "foil": false,
        "wouldDieInCombat": false
    })
}

fn permanent_card(value: &Value, owner_id: &str) -> Value {
    let permanent = object_value(value);
    let name = string_value(get_any(permanent, &["name"]), "Permanent");
    let (power, toughness) =
        split_power_toughness(get_any(permanent, &["power_toughness", "powerToughness"]));
    let lane = string_value(get_any(permanent, &["lane"]), "").to_lowercase();
    json!({
        "id": card_id(get_any(permanent, &["id", "stable_id", "stableId"])),
        "identity": identity(name, bool_value(get_any(permanent, &["token"]), false)),
        "controllerId": owner_id,
        "ownerId": owner_id,
        "zoneId": "battlefield",
        "color": "",
        "manaCost": string_value(get_any(permanent, &["mana_cost", "manaCost"]), ""),
        "cmc": 0,
        "types": if lane.contains("land") { json!(["Land"]) } else { json!([]) },
        "subtypes": [],
        "supertypes": [],
        "power": power,
        "toughness": toughness,
        "text": string_value(get_any(permanent, &["oracle_text", "oracleText"]), ""),
        "tapped": bool_value(get_any(permanent, &["tapped"]), false),
        "isCrewed": false,
        "isAttacking": false,
        "keywords": [],
        "counters": counters(get_any(permanent, &["counters"])),
        "damage": 0,
        "summoningSick": false,
        "isCopy": false,
        "isDoubleFaced": false,
        "isTransformed": false,
        "isFaceDown": false,
        "isBestowed": false,
        "phasedOut": false,
        "exerted": false,
        "isRingBearer": false,
        "attachmentIds": [],
        "isMadnessExiled": false,
        "isPlotted": false,
        "isWarpExiled": false,
        "foil": false,
        "wouldDieInCombat": false
    })
}

fn target_refs(value: &Value) -> Vec<Value> {
    let target = object_value(value);
    match string_value(get_any(target, &["kind"]), "").as_str() {
        "player" => vec![json!({ "kind": "player", "id": player_slot(get_any(target, &["player"])) })],
        "object" => vec![json!({ "kind": "card", "id": card_id(get_any(target, &["object"])) })],
        _ => Vec::new(),
    }
}

fn stack_object(value: &Value) -> Value {
    let stack = object_value(value);
    let id = stack_id(get_any(stack, &["id", "stable_id", "stableId"]));
    let controller_id = player_slot(get_any(stack, &["controller"]));
    let name = string_value(get_any(stack, &["name"]), "Stack object");
    let targets: Vec<Value> = array_value(get_any(stack, &["targets"]))
        .iter()
        .flat_map(target_refs)
        .collect();
    json!({
        "id": id,
        "sourceId": card_id(get_any(stack, &[
            "inspect_object_id",
            "inspectObjectId",
            "source_stable_id",
            "sourceStableId",
            "stable_id",
            "stableId",
            "id",
        ])),
        "controllerId": controller_id,
        "identity": identity(name, false),
        "text": string_value(get_any(stack, &["ability_text", "abilityText", "effect_text", "effectText"]), ""),
        "isPermanentSpell": false,
        "isCasting": false,
        "targets": targets,
    })
}

fn normalize_step(snapshot: &JsonMap) -> String {
    let raw = string_value(
        get_any(snapshot, &["step"]),
        &string_value(get_any(snapshot, &["phase"]), "main1"),
    );
    let normalized = raw.to_lowercase().split_whitespace().collect::<Vec<_>>().join("_");
    if normalized.contains("precombat") {
        "main1".to_string()
    } else if normalized.contains("postcombat") {
        "main2".to_string()
    } else if normalized.contains("combat") {
        "combat".to_string()
    } else if normalized.contains("draw") {
        "draw".to_string()
    } else if normalized.contains("upkeep") {
        "upkeep".to_string()
    } else if normalized.contains("end") {
        "end".to_string()
    } else if normalized.is_empty() {
        "main1".to_string()
    } else {
        normalized
    }
}

fn game_over(snapshot: &JsonMap) -> (bool, Value) {
    let raw = get_any(snapshot, &["game_over", "gameOver"]).and_then(as_object);
    let kind = raw
        .and_then(|raw| get_any(raw, &["kind"]))
        .and_then(Value::as_str)
        .unwrap_or_default();
    match kind {
        "" => (false, Value::Null),
        "winner" => (
            true,
            json!(player_slot(raw.and_then(|raw| get_any(raw, &["player"])))),
        ),
        "remaining" => {
            let remaining = raw
                .and_then(|raw| get_any(raw, &["players"]))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            (
                remaining.len() <= 1,
                if remaining.len() == 1 {
                    json!(player_slot(remaining.first()))
                } else {
                    Value::Null
                },
            )
        }
        _ => (true, Value::Null),
    }
}

fn game_view_from_snapshot(snapshot_value: &Value) -> Value {
    let snapshot = object_value(snapshot_value);
    let players_source = array_value(get_any(snapshot, &["players"]));
    let players: Vec<Value> = players_source
        .iter()
        .map(|player_value| {
            let player = object_value(player_value);
            let id = player_slot(get_any(player, &["id"]));
            let hand_cards: Vec<Value> = array_value(get_any(player, &["hand_cards", "handCards"]))
                .iter()
                .map(|card| card_from_zone_card(card, &id, &id, "hand"))
                .collect();
            let hand_size = number_value(
                get_any(player, &["hand_size", "handSize"]),
                hand_cards.len() as i64,
            ) as usize;
            let visible_hand = !hand_cards.is_empty()
                || bool_value(get_any(player, &["can_view_hand", "canViewHand"]), false);
            json!({
                "id": id,
                "name": string_value(get_any(player, &["name"]), &id),
                "isHuman": true,
                "life": number_value(get_any(player, &["life"]), 20),
                "poison": 0,
                "hand": if visible_hand {
                    Value::Array(hand_cards)
                } else {
                    Value::Array((0..hand_size).map(|index| hidden_card(&id, index)).collect())
                },
                "graveyard": Value::Array(array_value(get_any(player, &["graveyard_cards", "graveyardCards"]))
                    .iter()
                    .map(|card| card_from_zone_card(card, &id, &id, "graveyard"))
                    .collect()),
                "exile": Value::Array(array_value(get_any(player, &["exile_cards", "exileCards"]))
                    .iter()
                    .map(|card| card_from_zone_card(card, &id, &id, "exile"))
                    .collect()),
                "commandZone": Value::Array(array_value(get_any(player, &["command_cards", "commandCards"]))
                    .iter()
                    .map(|card| card_from_zone_card(card, &id, &id, "command"))
                    .collect()),
                "libraryCount": number_value(get_any(player, &["library_size", "librarySize"]), 0),
                "manaPool": mana_pool(get_any(player, &["mana_pool", "manaPool"])),
                "commanderDamage": {},
                "energyCounters": 0,
                "radiationCounters": 0,
                "hasCityBlessing": false,
                "ringLevel": 0,
                "speed": 0,
                "experienceCounters": 0,
                "ticketCounters": 0,
            })
        })
        .collect();
    let battlefield: Vec<Value> = players_source
        .iter()
        .flat_map(|player_value| {
            let player = object_value(player_value);
            let owner_id = player_slot(get_any(player, &["id"]));
            array_value(get_any(player, &["battlefield"]))
                .iter()
                .map(move |permanent| permanent_card(permanent, &owner_id))
        })
        .collect();
    let (over, winner_id) = game_over(snapshot);
    json!({
        "gameId": format!(
            "ironsmith-{}",
            get_any(snapshot, &["snapshot_id", "snapshotId"]).map_or_else(|| "game".to_string(), Value::to_string)
        ),
        "turn": number_value(get_any(snapshot, &["turn_number", "turnNumber"]), 1),
        "step": normalize_step(snapshot),
        "combatAssignments": [],
        "activePlayerId": player_slot(get_any(snapshot, &["active_player", "activePlayer"])),
        "priorityPlayerId": player_slot(
            get_any(snapshot, &["priority_player", "priorityPlayer"])
                .or_else(|| get_any(snapshot, &["active_player", "activePlayer"]))
        ),
        "players": players,
        "battlefield": battlefield,
        "stack": Value::Array(array_value(get_any(snapshot, &["stack_objects", "stackObjects"]))
            .iter()
            .map(stack_object)
            .collect()),
        "gameOver": over,
        "winnerId": winner_id,
        "concededPlayerIds": [],
        "monarchId": null,
        "initiativeHolderId": null,
    })
}

fn state_from_snapshot(snapshot: &Value) -> Value {
    json!({ "gameView": game_view_from_snapshot(snapshot) })
}

fn redact_private_state(state: &Value) -> Value {
    let mut state = state.clone();
    let Some(players) = state
        .get_mut("gameView")
        .and_then(|game_view| game_view.get_mut("players"))
        .and_then(Value::as_array_mut)
    else {
        return state;
    };
    for player in players {
        let Some(player_object) = player.as_object_mut() else {
            continue;
        };
        let id = player_object
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("player-0")
            .to_string();
        let count = player_object
            .get("hand")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        player_object.insert(
            "hand".to_string(),
            Value::Array((0..count).map(|index| hidden_card(&id, index)).collect()),
        );
    }
    state
}

fn presentation(title: &str, description: Option<String>, source_card_id: Option<String>) -> Value {
    json!({
        "title": title,
        "description": description,
        "sourceCardId": source_card_id,
        "targets": [],
    })
}

fn source_card_id(decision: &JsonMap) -> Option<String> {
    get_any(decision, &["source_id", "sourceId"]).map(|value| card_id(Some(value)))
}

fn legal_options(decision: &JsonMap) -> Vec<&Value> {
    array_value(get_any(decision, &["options"]))
        .iter()
        .filter(|option| bool_value(as_object(option).and_then(|option| get_any(option, &["legal"])), true))
        .collect()
}

fn option_description(option: &Value) -> String {
    let option = object_value(option);
    string_value(
        get_any(option, &["description"]),
        &get_any(option, &["index"])
            .map_or_else(String::new, Value::to_string),
    )
}

fn color_letter(label: &str) -> String {
    match label {
        "White" => "W",
        "Blue" => "U",
        "Black" => "B",
        "Red" => "R",
        "Green" => "G",
        "Colorless" => "C",
        value => value,
    }
    .to_string()
}

fn prompt_binding(prompt_id: &str, player_slot: &str, decision_kind: &str) -> Value {
    json!({
        "promptId": prompt_id,
        "playerSlot": player_slot,
        "decisionKind": decision_kind,
        "actionRefs": {},
        "targetKinds": {},
        "optionIndices": {},
    })
}

fn priority_prompt(decision: &JsonMap, prompt_id: &str) -> Value {
    let for_player = player_slot(get_any(decision, &["player"]));
    let actions = array_value(get_any(decision, &["actions"]));
    let mut action_refs = JsonMap::new();
    let options: Vec<Value> = actions
        .iter()
        .enumerate()
        .map(|(index, action)| {
            let action = object_value(action);
            action_refs.insert(
                index.to_string(),
                normalize_priority_action_ref(get_any(action, &["action_ref", "actionRef"]).cloned())
                    .unwrap_or(Value::Null),
            );
            json!(string_value(get_any(action, &["label"]), &format!("Action {}", index + 1)))
        })
        .collect();
    let mut binding = prompt_binding(prompt_id, &for_player, "priority");
    binding["actionRefs"] = Value::Object(action_refs);
    json!({
        "forPlayer": for_player,
        "prompt": {
            "promptId": prompt_id,
            "decidingPlayerId": for_player,
            "input": {
                "type": "chooseFromSelection",
                "presentation": presentation(
                    "Choose action",
                    Some(string_value(get_any(decision, &["description"]), "")).filter(|value| !value.is_empty()),
                    None,
                ),
                "options": options,
                "minChoices": if actions.is_empty() { 0 } else { 1 },
                "maxChoices": if actions.is_empty() { 0 } else { 1 },
            }
        },
        "binding": binding,
    })
}

fn number_prompt(decision: &JsonMap, prompt_id: &str) -> Value {
    let for_player = player_slot(get_any(decision, &["player"]));
    let source_card_id = source_card_id(decision);
    json!({
        "forPlayer": for_player,
        "prompt": {
            "promptId": prompt_id,
            "decidingPlayerId": for_player,
            "sourceCardId": source_card_id,
            "input": {
                "type": "chooseNumber",
                "presentation": presentation(
                    "Choose a number",
                    Some(string_value(get_any(decision, &["description"]), "")),
                    source_card_id,
                ),
                "min": number_value(get_any(decision, &["min"]), 0),
                "max": number_value(get_any(decision, &["max"]), 0),
            }
        },
        "binding": prompt_binding(prompt_id, &for_player, "number"),
    })
}

fn options_prompt(decision: &JsonMap, prompt_id: &str) -> Value {
    let for_player = player_slot(get_any(decision, &["player"]));
    let source_card_id = source_card_id(decision);
    let options = legal_options(decision);
    let labels: Vec<String> = options.iter().map(|option| option_description(option)).collect();
    let mut option_indices = JsonMap::new();
    for (index, option) in options.iter().enumerate() {
        let option = object_value(option);
        option_indices.insert(
            index.to_string(),
            json!(number_value(get_any(option, &["index"]), index as i64)),
        );
    }
    let color_labels = ["White", "Blue", "Black", "Red", "Green", "Colorless", "W", "U", "B", "R", "G", "C"];
    let mut binding = prompt_binding(prompt_id, &for_player, "select_options");
    if !labels.is_empty() && labels.iter().all(|label| color_labels.contains(&label.as_str())) {
        for (index, label) in labels.iter().enumerate() {
            let option = object_value(options[index]);
            option_indices.insert(
                color_letter(label),
                json!(number_value(get_any(option, &["index"]), index as i64)),
            );
        }
        binding["optionIndices"] = Value::Object(option_indices);
        return json!({
            "forPlayer": for_player,
            "prompt": {
                "promptId": prompt_id,
                "decidingPlayerId": for_player,
                "sourceCardId": source_card_id,
                "input": {
                    "type": "chooseColor",
                    "validColors": labels.iter().map(|label| color_letter(label)).collect::<Vec<_>>(),
                    "amount": number_value(get_any(decision, &["max"]), 1),
                    "repeatAllowed": true,
                }
            },
            "binding": binding,
        });
    }
    binding["optionIndices"] = Value::Object(option_indices);
    if labels.len() == 2
        && number_value(get_any(decision, &["min"]), 0) == 1
        && number_value(get_any(decision, &["max"]), 0) == 1
    {
        return json!({
            "forPlayer": for_player,
            "prompt": {
                "promptId": prompt_id,
                "decidingPlayerId": for_player,
                "sourceCardId": source_card_id,
                "input": {
                    "type": "chooseBoolean",
                    "presentation": presentation("Choose", Some(string_value(get_any(decision, &["description"]), "")), source_card_id),
                    "confirmLabel": labels[0],
                    "denyLabel": labels[1],
                }
            },
            "binding": binding,
        });
    }
    json!({
        "forPlayer": for_player,
        "prompt": {
            "promptId": prompt_id,
            "decidingPlayerId": for_player,
            "sourceCardId": source_card_id,
            "input": {
                "type": "chooseFromSelection",
                "presentation": presentation("Choose", Some(string_value(get_any(decision, &["description"]), "")), source_card_id),
                "options": labels,
                "minChoices": number_value(get_any(decision, &["min"]), 0),
                "maxChoices": number_value(get_any(decision, &["max"]), options.len() as i64),
            }
        },
        "binding": binding,
    })
}

fn object_choice_card(value: &Value, owner_id: &str) -> Value {
    let candidate = object_value(value);
    let default_controller = json!(parse_player_index(owner_id));
    let controller = get_any(candidate, &["object_controller", "objectController"])
        .unwrap_or(&default_controller);
    json!({
        "id": card_id(get_any(candidate, &["id", "stable_id", "stableId"])),
        "identity": identity(string_value(get_any(candidate, &["name"]), "Card"), false),
        "controllerId": player_slot(Some(controller)),
        "ownerId": owner_id,
        "zoneId": "choice",
        "color": "",
        "manaCost": "",
        "cmc": 0,
        "types": [],
        "subtypes": [],
        "supertypes": [],
        "power": null,
        "toughness": null,
        "text": "",
        "tapped": false,
        "isCrewed": false,
        "isAttacking": false,
        "keywords": [],
        "counters": {},
        "damage": 0,
        "summoningSick": false,
        "isCopy": false,
        "isDoubleFaced": false,
        "isTransformed": false,
        "isFaceDown": false,
        "isBestowed": false,
        "phasedOut": false,
        "exerted": false,
        "isRingBearer": false,
        "attachmentIds": [],
        "isMadnessExiled": false,
        "isPlotted": false,
        "isWarpExiled": false,
        "foil": false,
        "wouldDieInCombat": false
    })
}

fn objects_prompt(decision: &JsonMap, prompt_id: &str) -> Value {
    let for_player = player_slot(get_any(decision, &["player"]));
    let source_card_id = source_card_id(decision);
    let candidates: Vec<&Value> = array_value(get_any(decision, &["candidates"]))
        .iter()
        .filter(|candidate| bool_value(as_object(candidate).and_then(|candidate| get_any(candidate, &["legal"])), true))
        .collect();
    let cards: Vec<Value> = candidates
        .iter()
        .map(|candidate| object_choice_card(candidate, &for_player))
        .collect();
    let reorder = string_value(get_any(decision, &["selection_identity", "selectionIdentity"]), "").contains("order");
    let decision_kind = if reorder { "reorder_objects" } else { "select_objects" };
    json!({
        "forPlayer": for_player,
        "prompt": {
            "promptId": prompt_id,
            "decidingPlayerId": for_player,
            "sourceCardId": source_card_id,
            "input": if reorder {
                json!({
                    "type": "reorderCards",
                    "presentation": presentation("Reorder cards", Some(string_value(get_any(decision, &["description"]), "")), source_card_id),
                    "cards": cards,
                    "targetLabel": "top",
                    "topOfDeck": true,
                })
            } else {
                json!({
                    "type": "chooseCards",
                    "presentation": presentation("Choose cards", Some(string_value(get_any(decision, &["description"]), "")), source_card_id),
                    "cards": cards,
                    "min": number_value(get_any(decision, &["min"]), 0),
                    "max": number_value(get_any(decision, &["max"]), candidates.len() as i64),
                })
            }
        },
        "binding": prompt_binding(prompt_id, &for_player, decision_kind),
    })
}

fn target_ref(value: &Value) -> Option<Value> {
    let target = object_value(value);
    match string_value(get_any(target, &["kind"]), "").as_str() {
        "player" => Some(json!({ "kind": "player", "id": player_slot(get_any(target, &["player"])) })),
        "object" => Some(json!({ "kind": "card", "id": card_id(get_any(target, &["object"])) })),
        _ => None,
    }
}

fn targets_prompt(decision: &JsonMap, prompt_id: &str) -> Value {
    let for_player = player_slot(get_any(decision, &["player"]));
    let source_card_id = source_card_id(decision);
    let requirements: Vec<&JsonMap> = array_value(get_any(decision, &["requirements"]))
        .iter()
        .filter_map(as_object)
        .collect();
    let mut target_kinds = JsonMap::new();
    let candidates: Vec<Value> = requirements
        .iter()
        .flat_map(|requirement| array_value(get_any(requirement, &["legal_targets", "legalTargets"])))
        .filter_map(|target| {
            let reference = target_ref(target)?;
            if let Some(id) = reference.get("id").and_then(Value::as_str) {
                let kind = if reference.get("kind").and_then(Value::as_str) == Some("player") {
                    "player"
                } else {
                    "object"
                };
                target_kinds.insert(id.to_string(), json!(kind));
            }
            Some(reference)
        })
        .collect();
    let min_targets: i64 = requirements
        .iter()
        .map(|req| number_value(get_any(req, &["min_targets", "minTargets"]), 0))
        .sum();
    let max_targets: i64 = requirements
        .iter()
        .map(|req| number_value(get_any(req, &["max_targets", "maxTargets"]), number_value(get_any(req, &["min_targets", "minTargets"]), 0)))
        .sum();
    let mut binding = prompt_binding(prompt_id, &for_player, "targets");
    binding["targetKinds"] = Value::Object(target_kinds);
    json!({
        "forPlayer": for_player,
        "prompt": {
            "promptId": prompt_id,
            "decidingPlayerId": for_player,
            "sourceCardId": source_card_id,
            "input": {
                "type": "chooseBoardTargets",
                "candidates": candidates,
                "hostile": true,
                "intent": "hostile",
                "minTargets": min_targets,
                "maxTargets": max_targets,
                "chosenTargets": 0,
                "label": string_value(get_any(decision, &["context"]), "Choose targets"),
            }
        },
        "binding": binding,
    })
}

fn attackers_prompt(decision: &JsonMap, prompt_id: &str) -> Value {
    let for_player = player_slot(get_any(decision, &["player"]));
    let mut target_map = JsonMap::new();
    let attackers: Vec<Value> = array_value(get_any(decision, &["attacker_options", "attackerOptions"]))
        .iter()
        .map(|option_value| {
            let option = object_value(option_value);
            let valid_target_ids: Vec<Value> = array_value(get_any(option, &["valid_targets", "validTargets"]))
                .iter()
                .map(|target_value| {
                    let target = object_value(target_value);
                    let kind = string_value(get_any(target, &["kind"]), "");
                    let id = if kind == "player" {
                        player_slot(get_any(target, &["player"]))
                    } else {
                        card_id(get_any(target, &["object"]))
                    };
                    target_map.insert(
                        id.clone(),
                        json!({ "id": id, "label": string_value(get_any(target, &["name"]), ""), "kind": if kind == "player" { "player" } else { "planeswalker" } }),
                    );
                    json!(id)
                })
                .collect();
            json!({
                "attackerId": card_id(get_any(option, &["creature"])),
                "validTargetIds": valid_target_ids,
                "mustAttack": bool_value(get_any(option, &["must_attack", "mustAttack"]), false),
            })
        })
        .collect();
    let mut target_kinds = JsonMap::new();
    for (id, target) in &target_map {
        let kind = if target.get("kind").and_then(Value::as_str) == Some("player") {
            "player"
        } else {
            "planeswalker"
        };
        target_kinds.insert(id.clone(), json!(kind));
    }
    let mut binding = prompt_binding(prompt_id, &for_player, "attackers");
    binding["targetKinds"] = Value::Object(target_kinds);
    json!({
        "forPlayer": for_player,
        "prompt": {
            "promptId": prompt_id,
            "decidingPlayerId": for_player,
            "input": {
                "type": "chooseAttackers",
                "attackers": attackers,
                "attackTargets": Value::Array(target_map.values().cloned().collect()),
            }
        },
        "binding": binding,
    })
}

fn blockers_prompt(decision: &JsonMap, prompt_id: &str) -> Value {
    let for_player = player_slot(get_any(decision, &["player"]));
    let mut blockers = Vec::new();
    let attackers: Vec<Value> = array_value(get_any(decision, &["blocker_options", "blockerOptions"]))
        .iter()
        .map(|option_value| {
            let option = object_value(option_value);
            let valid_blocker_ids: Vec<Value> = array_value(get_any(option, &["valid_blockers", "validBlockers"]))
                .iter()
                .map(|blocker_value| {
                    let id = card_id(get_any(object_value(blocker_value), &["id"]));
                    blockers.push(json!(id));
                    json!(id)
                })
                .collect();
            json!({
                "attackerId": card_id(get_any(option, &["attacker"])),
                "validBlockerIds": valid_blocker_ids,
                "minBlockers": number_value(get_any(option, &["min_blockers", "minBlockers"]), 0),
                "maxBlockers": null,
                "mustBeBlocked": number_value(get_any(option, &["min_blockers", "minBlockers"]), 0) > 0,
            })
        })
        .collect();
    blockers.sort_by_key(Value::to_string);
    blockers.dedup_by_key(|value| value.to_string());
    json!({
        "forPlayer": for_player,
        "prompt": {
            "promptId": prompt_id,
            "decidingPlayerId": for_player,
            "input": {
                "type": "chooseBlockers",
                "attackers": attackers,
                "availableBlockerIds": blockers,
            }
        },
        "binding": prompt_binding(prompt_id, &for_player, "blockers"),
    })
}

fn manabrew_prompt_from_snapshot(snapshot: &Value, prompt_id: &str) -> Value {
    let decision = snapshot
        .get("decision")
        .and_then(as_object)
        .cloned()
        .unwrap_or_default();
    let kind = string_value(get_any(&decision, &["kind"]), "");
    match kind.as_str() {
        "" => Value::Null,
        "text_input" => json!({
            "forPlayer": player_slot(get_any(&decision, &["player"])),
            "message": string_value(
                get_any(&decision, &["description"]),
                "Ironsmith text input prompts are not supported in Manabrew yet.",
            ),
        }),
        "priority" => priority_prompt(&decision, prompt_id),
        "number" => number_prompt(&decision, prompt_id),
        "select_options" => options_prompt(&decision, prompt_id),
        "select_objects" => objects_prompt(&decision, prompt_id),
        "targets" => targets_prompt(&decision, prompt_id),
        "attackers" => attackers_prompt(&decision, prompt_id),
        "blockers" => blockers_prompt(&decision, prompt_id),
        _ => json!({
            "forPlayer": player_slot(get_any(&decision, &["player"])),
            "message": format!("Ironsmith decision kind is not supported in Manabrew yet: {kind}"),
        }),
    }
}

fn zero_payload_priority_ref_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "PassPriority" | "passPriority" | "pass_priority" => Some("pass_priority"),
        "KeepOpeningHand" | "keepOpeningHand" | "keep_opening_hand" => Some("keep_opening_hand"),
        "TakeMulligan" | "takeMulligan" | "take_mulligan" => Some("take_mulligan"),
        "ContinuePregame" | "continuePregame" | "continue_pregame" => Some("continue_pregame"),
        "BeginGame" | "beginGame" | "begin_game" => Some("begin_game"),
        _ => None,
    }
}

fn priority_action_ref_kind(action_ref: &Value) -> String {
    if let Some(kind) = action_ref.as_str() {
        return zero_payload_priority_ref_kind(kind).unwrap_or("").to_string();
    }
    let kind = as_object(action_ref)
        .and_then(|object| get_any(object, &["kind"]))
        .and_then(Value::as_str)
        .unwrap_or_default();
    zero_payload_priority_ref_kind(kind).unwrap_or(kind).to_string()
}

fn normalize_priority_action_ref(action_ref: Option<Value>) -> Option<Value> {
    let action_ref = action_ref?;
    let kind = priority_action_ref_kind(&action_ref);
    if kind.is_empty() {
        return Some(action_ref);
    }
    if action_ref.is_string() {
        return Some(json!({ "kind": kind }));
    }
    let mut object = action_ref.as_object().cloned().unwrap_or_default();
    object.insert("kind".to_string(), json!(kind));
    Some(Value::Object(object))
}

fn pass_priority_action_ref(binding: &JsonMap) -> Value {
    let pass_like = [
        "pass_priority",
        "keep_opening_hand",
        "continue_pregame",
        "begin_game",
    ];
    binding
        .get("actionRefs")
        .and_then(Value::as_object)
        .and_then(|action_refs| {
            action_refs.values().find_map(|action_ref| {
                let normalized = normalize_priority_action_ref(Some(action_ref.clone()))?;
                pass_like
                    .contains(&priority_action_ref_kind(&normalized).as_str())
                    .then_some(normalized)
            })
        })
        .unwrap_or_else(|| json!({ "kind": "pass_priority" }))
}

fn bound_priority_action_ref(binding: &JsonMap, action_id: &str) -> Result<Value, JsValue> {
    if priority_action_ref_kind(&json!(action_id)) == "pass_priority" {
        return Ok(pass_priority_action_ref(binding));
    }
    let action_refs = binding
        .get("actionRefs")
        .and_then(Value::as_object)
        .ok_or_else(|| JsValue::from_str("Ironsmith priority binding has no action refs"))?;
    let action_ref = action_refs
        .get(action_id)
        .ok_or_else(|| JsValue::from_str(&format!("Unknown Ironsmith priority action id: {action_id}")))?;
    normalize_priority_action_ref(Some(action_ref.clone())).ok_or_else(|| {
        JsValue::from_str(&format!(
            "Unsupported Ironsmith priority action ref for {action_id}"
        ))
    })
}

fn option_indices_from_colors(output: &JsonMap, binding: &JsonMap) -> Vec<Value> {
    let chosen_colors = output
        .get("output")
        .and_then(|output| output.get("chosenColors"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let option_indices = binding
        .get("optionIndices")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    chosen_colors
        .iter()
        .flat_map(|(color, amount)| {
            let amount = amount.as_u64().unwrap_or(0);
            (0..amount).map({
                let value = option_indices.get(color).cloned().unwrap_or_else(|| json!(0));
                move |_| value.clone()
            })
        })
        .collect()
}

fn parse_object_id(id: &str) -> u64 {
    id.chars()
        .rev()
        .take_while(|char| char.is_ascii_digit())
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

fn parse_player_index(id: &str) -> u64 {
    id.strip_prefix("player-")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

fn manabrew_command_from_values(output: &JsonMap, binding: &JsonMap) -> Result<Value, JsValue> {
    match output.get("type").and_then(Value::as_str).unwrap_or_default() {
        "chooseAction" => {
            let inner = output.get("output").and_then(as_object).cloned().unwrap_or_default();
            match inner.get("type").and_then(Value::as_str).unwrap_or_default() {
                "concede" => Ok(json!({ "type": "forfeit_player" })),
                "pass" => Ok(json!({ "type": "priority_action", "action_ref": pass_priority_action_ref(binding) })),
                "act" => Ok(json!({
                    "type": "priority_action",
                    "action_ref": bound_priority_action_ref(binding, &string_value(inner.get("actionId"), ""))?,
                })),
                _ => Err(JsValue::from_str("unsupported chooseAction output")),
            }
        }
        "payManaCost" => {
            let inner = output.get("output").and_then(as_object).cloned().unwrap_or_default();
            if inner.get("type").and_then(Value::as_str) == Some("cancel") {
                return Ok(json!({ "type": "priority_action", "action_ref": pass_priority_action_ref(binding) }));
            }
            if inner.get("type").and_then(Value::as_str) == Some("act") {
                return Ok(json!({
                    "type": "priority_action",
                    "action_ref": bound_priority_action_ref(binding, &string_value(inner.get("actionId"), ""))?,
                }));
            }
            let first = binding
                .get("actionRefs")
                .and_then(Value::as_object)
                .and_then(|refs| refs.keys().next().cloned());
            Ok(json!({
                "type": "priority_action",
                "action_ref": if let Some(first) = first {
                    bound_priority_action_ref(binding, &first)?
                } else {
                    pass_priority_action_ref(binding)
                },
            }))
        }
        "chooseNumber" => Ok(json!({
            "type": "number_choice",
            "value": output
                .get("output")
                .and_then(|output| output.get("chosenNumber"))
                .cloned()
                .unwrap_or_else(|| json!(0)),
        })),
        "chooseBoolean" => {
            let value = output
                .get("output")
                .and_then(|output| output.get("value"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let key = if value { "0" } else { "1" };
            let fallback = if value { 0 } else { 1 };
            let index = binding
                .get("optionIndices")
                .and_then(Value::as_object)
                .and_then(|indices| indices.get(key))
                .cloned()
                .unwrap_or_else(|| json!(fallback));
            Ok(json!({ "type": "select_options", "option_indices": [index] }))
        }
        "chooseFromSelection" => {
            let chosen_indices = output
                .get("output")
                .and_then(|output| output.get("chosenIndices"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if binding.get("decisionKind").and_then(Value::as_str) == Some("priority") {
                let action_ref = if let Some(first) = chosen_indices.first().and_then(Value::as_u64) {
                    bound_priority_action_ref(binding, &first.to_string())?
                } else {
                    pass_priority_action_ref(binding)
                };
                return Ok(json!({ "type": "priority_action", "action_ref": action_ref }));
            }
            let option_indices = binding
                .get("optionIndices")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            Ok(json!({
                "type": "select_options",
                "option_indices": chosen_indices
                    .iter()
                    .map(|index| {
                        let key = index.as_u64().unwrap_or(0).to_string();
                        option_indices.get(&key).cloned().unwrap_or_else(|| index.clone())
                    })
                    .collect::<Vec<_>>(),
            }))
        }
        "chooseColor" => Ok(json!({
            "type": "select_options",
            "option_indices": option_indices_from_colors(output, binding),
        })),
        "chooseCards" => Ok(json!({
            "type": "select_objects",
            "object_ids": output
                .get("output")
                .and_then(|output| output.get("chosenCardIds"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|id| json!(parse_object_id(id.as_str().unwrap_or_default())))
                .collect::<Vec<_>>(),
        })),
        "reorderCards" => Ok(json!({
            "type": "select_objects",
            "object_ids": output
                .get("output")
                .and_then(|output| output.get("orderedCardIds"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|id| json!(parse_object_id(id.as_str().unwrap_or_default())))
                .collect::<Vec<_>>(),
        })),
        "chooseBoardTargets" => Ok(json!({
            "type": "select_targets",
            "targets": output
                .get("output")
                .and_then(|output| output.get("chosen"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|target| {
                    let target = object_value(target);
                    let id = string_value(get_any(target, &["id"]), "");
                    if string_value(get_any(target, &["kind"]), "") == "player" {
                        json!({ "kind": "player", "player": parse_player_index(&id) })
                    } else {
                        json!({ "kind": "object", "object": parse_object_id(&id) })
                    }
                })
                .collect::<Vec<_>>(),
        })),
        "chooseAttackers" => {
            let target_kinds = binding
                .get("targetKinds")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            Ok(json!({
                "type": "declare_attackers",
                "declarations": output
                    .get("output")
                    .and_then(|output| output.get("assignments"))
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .map(|assignment| {
                        let assignment = object_value(assignment);
                        let target_id = string_value(get_any(assignment, &["targetId"]), "");
                        let target_kind = target_kinds.get(&target_id).and_then(Value::as_str);
                        json!({
                            "creature": parse_object_id(&string_value(get_any(assignment, &["attackerId"]), "")),
                            "target": if target_kind == Some("player") {
                                json!({ "kind": "player", "player": parse_player_index(&target_id) })
                            } else {
                                json!({ "kind": "planeswalker", "object": parse_object_id(&target_id) })
                            },
                        })
                    })
                    .collect::<Vec<_>>(),
            }))
        }
        "chooseBlockers" => Ok(json!({
            "type": "declare_blockers",
            "declarations": output
                .get("output")
                .and_then(|output| output.get("assignments"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|assignment| {
                    let assignment = object_value(assignment);
                    json!({
                        "blocker": parse_object_id(&string_value(get_any(assignment, &["blockerId"]), "")),
                        "blocking": parse_object_id(&string_value(get_any(assignment, &["attackerId"]), "")),
                    })
                })
                .collect::<Vec<_>>(),
        })),
        other => Err(JsValue::from_str(&format!(
            "Unsupported Ironsmith Manabrew prompt output: {other}"
        ))),
    }
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(js_name = manabrewView)]
    pub fn manabrew_view(&mut self, prompt_id: String) -> Result<JsValue, JsValue> {
        let snapshot_js = self.ui_state()?;
        let snapshot: Value = serde_wasm_bindgen::from_value(snapshot_js)
            .map_err(|error| JsValue::from_str(&format!("failed to decode Ironsmith UI state: {error}")))?;
        let view = json!({
            "state": state_from_snapshot(&snapshot),
            "promptResult": manabrew_prompt_from_snapshot(&snapshot, &prompt_id),
        });
        serde_wasm_bindgen::to_value(&view)
            .map_err(|error| JsValue::from_str(&format!("failed to encode Manabrew view: {error}")))
    }

    #[wasm_bindgen(js_name = manabrewPublicState)]
    pub fn manabrew_public_state(&mut self) -> Result<JsValue, JsValue> {
        let snapshot_js = self.ui_state()?;
        let snapshot: Value = serde_wasm_bindgen::from_value(snapshot_js)
            .map_err(|error| JsValue::from_str(&format!("failed to decode Ironsmith UI state: {error}")))?;
        serde_wasm_bindgen::to_value(&redact_private_state(&state_from_snapshot(&snapshot)))
            .map_err(|error| JsValue::from_str(&format!("failed to encode Manabrew public state: {error}")))
    }

    #[wasm_bindgen(js_name = manabrewPrompt)]
    pub fn manabrew_prompt(&mut self, prompt_id: String) -> Result<JsValue, JsValue> {
        let snapshot_js = self.ui_state()?;
        let snapshot: Value = serde_wasm_bindgen::from_value(snapshot_js)
            .map_err(|error| JsValue::from_str(&format!("failed to decode Ironsmith UI state: {error}")))?;
        serde_wasm_bindgen::to_value(&manabrew_prompt_from_snapshot(&snapshot, &prompt_id))
            .map_err(|error| JsValue::from_str(&format!("failed to encode Manabrew prompt: {error}")))
    }

    #[wasm_bindgen(js_name = manabrewCommandFromPromptOutput)]
    pub fn manabrew_command_from_prompt_output(
        &mut self,
        output: JsValue,
        binding: JsValue,
    ) -> Result<JsValue, JsValue> {
        let output: Value = serde_wasm_bindgen::from_value(output)
            .map_err(|error| JsValue::from_str(&format!("invalid Manabrew prompt output: {error}")))?;
        let binding: Value = serde_wasm_bindgen::from_value(binding)
            .map_err(|error| JsValue::from_str(&format!("invalid Ironsmith prompt binding: {error}")))?;
        let output = output
            .as_object()
            .ok_or_else(|| JsValue::from_str("Manabrew prompt output must be an object"))?;
        let binding = binding
            .as_object()
            .ok_or_else(|| JsValue::from_str("Ironsmith prompt binding must be an object"))?;
        serde_wasm_bindgen::to_value(&manabrew_command_from_values(output, binding)?)
            .map_err(|error| JsValue::from_str(&format!("failed to encode Ironsmith command: {error}")))
    }
}
