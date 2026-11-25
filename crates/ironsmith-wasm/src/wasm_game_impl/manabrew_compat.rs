use std::collections::BTreeMap;

use manabrew_protocol::game::{
    CardDto, CardIdentity, CardView, CombatAssignmentDto, DayTime, GameViewDto, Mana, ManaColor,
    PlayerCounterKind, PlayerDto, PlayerStatus, RoomDoor, StackObjectDto, StepKind, ZoneDto,
    ZoneKind,
};
use manabrew_protocol::prompts::choose_attackers::AttackerOptionDto;
use manabrew_protocol::prompts::choose_blockers::BlockableAttackerDto;
use manabrew_protocol::prompts::common::{
    ActivatableAbilityInfo, AttackAssignment, AttackTargetDto, AttackTargetKind, AvailableAction,
    AvailableActionKind, BlockAssignment, PaymentAction, PaymentActionKind, PaymentResourceKind,
    AlternativeCostKind,
    PlayCardMode, PromptPresentation, TargetKind, TargetRef,
};
use manabrew_protocol::prompts::scry::ScryDestination;
use manabrew_protocol::prompts::{
    ChooseActionInput, ChooseActionOutput, ChooseAttackersInput, ChooseAttackersOutput,
    ChooseBlockersInput, ChooseBlockersOutput, ChooseBoardTargetsInput, ChooseBoardTargetsOutput,
    ChooseBooleanInput, ChooseBooleanOutput, ChooseCardsInput, ChooseCardsOutput, ChooseColorInput,
    ChooseColorOutput, ChooseCombatDamageAssignmentInput, ChooseCombatDamageAssignmentOutput,
    ChooseFromSelectionInput, ChooseFromSelectionOutput, ChooseNumberInput,
    ChooseNumberOutput, MulliganInput, MulliganOutput, MulliganPutBackInput,
    MulliganPutBackOutput, PayManaCostInput, PayManaCostOutput,
    PromptInput, PromptOutput, ReorderInput, ReorderItem, ReorderOutput, ResponseViolation,
    ScryInput, ScryOutput, SelectionOption,
};
use manabrew_protocol::transport::{
    AgentPrompt, DirectiveInput, ProtocolError, ProtocolErrorCode, StateUpdate,
};
use serde_json::Value;

type JsonMap = serde_json::Map<String, Value>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManabrewMatchConfigInput {
    /// Optional: the published Manabrew adapter does not declare a version.
    /// A declared version must match the adapter's.
    #[serde(default)]
    protocol_version: Option<u32>,
    player_names: Vec<String>,
    starting_life: i32,
    #[serde(default)]
    seed: Option<u64>,
    #[serde(default)]
    game_id: Option<String>,
    #[serde(default)]
    human_players: Vec<bool>,
    #[serde(default)]
    bot_players: Vec<u8>,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    decks: Vec<Value>,
    #[serde(default)]
    commander_names: Vec<Option<String>>,
    #[serde(default)]
    opening_hand_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ManabrewViewResult {
    protocol_version: u32,
    state: StateUpdate,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<AgentPrompt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ProtocolError>,
    /// Verified matches only: the single legal answer to a decision the player
    /// is not asked about. The host submits it through the signed transcript.
    #[serde(skip_serializing_if = "Option::is_none")]
    forced_command: Option<Value>,
}

/// What a Manabrew response means for the engine, without applying it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
enum ManabrewTranslation {
    /// Submit this command (a `UiCommand`) through the host's action pipeline.
    Dispatch { command: Value },
    /// Take back the action chain the prompt belongs to.
    Cancel,
    /// A multi-step prompt moved on locally; nothing to submit.
    Continue { view: ManabrewViewResult },
    /// The response was refused; the view carries the error.
    Rejected { view: ManabrewViewResult },
}

enum ManabrewResponseAction {
    Dispatch(UiCommand),
    /// Take back the action chain this prompt belongs to.
    Cancel,
    Continue {
        input: PromptInput,
        binding: ManabrewPromptBinding,
    },
}

/// The adapter's protocol version: the major of the pinned `manabrew-protocol`
/// crate, whose DTO shapes this adapter emits and accepts.
fn manabrew_protocol_major() -> u32 {
    manabrew_protocol::VERSION
        .split('.')
        .next()
        .and_then(|major| major.parse().ok())
        .expect("manabrew-protocol has a numeric major version")
}

fn validate_manabrew_protocol_version(version: Option<u32>) -> Result<(), String> {
    let expected = manabrew_protocol_major();
    match version {
        None => Ok(()),
        Some(version) if version == expected => Ok(()),
        Some(version) => Err(format!(
            "Manabrew protocol version mismatch: expected {expected}, received {version}"
        )),
    }
}

const MANABREW_TEXT_OPTIONS_PER_PROMPT: usize = 100;

fn object_value(value: &Value) -> &JsonMap {
    static EMPTY: std::sync::OnceLock<JsonMap> = std::sync::OnceLock::new();
    value
        .as_object()
        .unwrap_or_else(|| EMPTY.get_or_init(JsonMap::new))
}

fn array_value(value: Option<&Value>) -> &[Value] {
    value
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn first_value<'a>(object: &'a JsonMap, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| object.get(*key))
}

fn scalar_string(value: Option<&Value>) -> Option<String> {
    let value = value?;
    let text = match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        _ => return None,
    };
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

fn manabrew_format(value: Option<&str>) -> MatchFormatInput {
    match value.unwrap_or_default().to_ascii_lowercase().as_str() {
        "ante" => MatchFormatInput::Ante,
        "brawl" => MatchFormatInput::Brawl,
        "commander" | "oathbreaker" => MatchFormatInput::Commander,
        "paupercommander" | "pauper_commander" | "pdh" => MatchFormatInput::PauperCommander,
        _ => MatchFormatInput::Normal,
    }
}

fn manabrew_card_name(card: &Value) -> Option<String> {
    let card = object_value(card);
    let name = card
        .get("identity")
        .and_then(Value::as_object)
        .and_then(|identity| identity.get("name"))
        .and_then(Value::as_str)
        .or_else(|| card.get("name").and_then(Value::as_str))
        .unwrap_or_default()
        .trim();
    (!name.is_empty()).then(|| name.to_string())
}

fn manabrew_deck_section_names(deck: &Value, section: &str) -> Vec<String> {
    array_value(object_value(deck).get(section))
        .iter()
        .filter_map(manabrew_card_name)
        .collect()
}

fn manabrew_deck_commander_names(deck: &Value, fallback: Option<&str>) -> Vec<String> {
    let commanders = manabrew_deck_section_names(deck, "commanders");
    if !commanders.is_empty() {
        return commanders;
    }
    fallback
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| vec![value.to_string()])
        .unwrap_or_default()
}

fn manabrew_all_deck_cards(deck: &Value) -> Vec<&Value> {
    let deck = object_value(deck);
    let mut cards = Vec::new();
    for section in [
        "cards",
        "sideboard",
        "commanders",
        "attractions",
        "contraptions",
        "schemes",
        "planes",
        "maybeboard",
        "tokens",
    ] {
        cards.extend(array_value(deck.get(section)));
    }
    if let Some(companion) = deck.get("companion") {
        cards.push(companion);
    }
    cards
}

fn manabrew_string_array(card: &JsonMap, key: &str) -> Vec<String> {
    array_value(card.get(key))
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn manabrew_type_line(card: &JsonMap) -> String {
    if let Some(type_line) = scalar_string(first_value(card, &["typeLine", "type_line"])) {
        return type_line;
    }
    let mut front = manabrew_string_array(card, "supertypes");
    front.extend(manabrew_string_array(card, "types"));
    let front = front.join(" ");
    let subtypes = manabrew_string_array(card, "subtypes").join(" ");
    match (front.is_empty(), subtypes.is_empty()) {
        (true, true) => "Card".to_string(),
        (true, false) => subtypes,
        (false, true) => front,
        (false, false) => format!("{front} — {subtypes}"),
    }
}

/// Printed loyalty (planeswalkers) and defense (battles) by face name. The
/// Manabrew deck protocol carries neither, and they cannot be derived from a
/// card's type line or rules text, so the adapter ships them. Regenerate with
/// `scripts/generate_printed_loyalty.py`.
static MANABREW_PRINTED_STATS: &str = include_str!("printed_loyalty.tsv");

#[derive(Clone, Copy, PartialEq, Eq)]
enum ManabrewPrintedStat {
    Loyalty,
    Defense,
}

fn manabrew_printed_stat(name: &str, stat: ManabrewPrintedStat) -> Option<&'static str> {
    static TABLE: std::sync::OnceLock<HashMap<(String, bool), &'static str>> =
        std::sync::OnceLock::new();
    TABLE
        .get_or_init(|| {
            MANABREW_PRINTED_STATS
                .lines()
                .filter_map(|line| {
                    let mut fields = line.split('\t');
                    let name = fields.next()?.trim();
                    let kind = fields.next()?.trim();
                    let value = fields.next()?.trim();
                    (!name.is_empty() && !value.is_empty())
                        .then(|| ((name.to_lowercase(), kind == "D"), value))
                })
                .collect()
        })
        .get(&(
            name.trim().to_lowercase(),
            stat == ManabrewPrintedStat::Defense,
        ))
        .copied()
}

fn manabrew_type_line_has(card: &JsonMap, card_type: &str) -> bool {
    manabrew_type_line(card)
        .split(|character: char| !character.is_ascii_alphabetic())
        .any(|word| word.eq_ignore_ascii_case(card_type))
}

/// The printed stat a card of this type needs: what the host supplied, or
/// else the adapter's table. `None` when the card is not of that type.
fn manabrew_required_stat(
    card: &Value,
    stat: ManabrewPrintedStat,
) -> Option<Result<String, String>> {
    let object = object_value(card);
    let (card_type, field) = match stat {
        ManabrewPrintedStat::Loyalty => ("Planeswalker", "loyalty"),
        ManabrewPrintedStat::Defense => ("Battle", "defense"),
    };
    let supplied = scalar_string(object.get(field));
    if !manabrew_type_line_has(object, card_type) {
        return supplied.map(Ok);
    }
    let name = manabrew_card_name(card).unwrap_or_default();
    Some(
        supplied
            .or_else(|| manabrew_printed_stat(&name, stat).map(str::to_string))
            .ok_or_else(|| {
                format!(
                    "{name} is a {} with no printed {field}; supply `{field}` on the deck card",
                    card_type.to_lowercase()
                )
            }),
    )
}

/// Every face of a deck card that would enter without its printed loyalty or
/// defense, as (face name, reason).
fn manabrew_missing_printed_stats(card: &Value) -> Vec<String> {
    let object = object_value(card);
    let mut faces: Vec<&Value> = manabrew_card_faces(card).iter().take(2).collect();
    if faces.len() < 2 {
        faces = vec![card];
        if let Some(back) = first_value(object, &["backFace", "back_face"]) {
            faces.push(back);
        }
    }
    faces
        .into_iter()
        .flat_map(|face| {
            [ManabrewPrintedStat::Loyalty, ManabrewPrintedStat::Defense]
                .into_iter()
                .filter_map(move |stat| manabrew_required_stat(face, stat)?.err())
        })
        .collect()
}

fn manabrew_card_source_block(card_value: &Value) -> String {
    let card = object_value(card_value);
    let mut lines = Vec::new();
    // A two-part card that shares one image (split, adventure, prepare, flip)
    // reaches the adapter as its front half, yet Scryfall-derived hosts leave
    // the card-level cost joined ("{1}{R} // {1}{U}"). This block describes
    // one face, so it takes that face's half.
    if let Some(mana_cost) = scalar_string(first_value(card, &["manaCost", "mana_cost"]))
        && let Some(mana_cost) = mana_cost
            .split("//")
            .map(str::trim)
            .next()
            .filter(|cost| !cost.is_empty())
    {
        lines.push(format!("Mana cost: {mana_cost}"));
    }
    lines.push(format!("Type: {}", manabrew_type_line(card)));
    let power = scalar_string(card.get("power"));
    let toughness = scalar_string(card.get("toughness"));
    if let (Some(power), Some(toughness)) = (power, toughness) {
        lines.push(format!("Power/Toughness: {power}/{toughness}"));
    }
    if let Some(Ok(loyalty)) = manabrew_required_stat(card_value, ManabrewPrintedStat::Loyalty)
    {
        lines.push(format!("Loyalty: {loyalty}"));
    }
    if let Some(Ok(defense)) = manabrew_required_stat(card_value, ManabrewPrintedStat::Defense)
    {
        lines.push(format!("Defense: {defense}"));
    }
    if let Some(oracle_text) =
        scalar_string(first_value(card, &["text", "oracleText", "oracle_text"]))
    {
        lines.push(oracle_text);
    }
    lines.join("\n")
}

fn manabrew_card_faces(card: &Value) -> &[Value] {
    let card = object_value(card);
    array_value(first_value(card, &["cardFaces", "card_faces", "faces"]))
}

fn manabrew_card_source(card: &Value) -> Option<ExternalCardSourceFile> {
    let deck_name = manabrew_card_name(card)?;
    let card_object = object_value(card);
    let faces = manabrew_card_faces(card);
    let mut linked_faces = faces
        .iter()
        .take(2)
        .filter_map(|face| {
            Some(ExternalCardFaceSource {
                name: manabrew_card_name(face)?,
                block: manabrew_card_source_block(face),
                score: Some(1.0),
            })
        })
        .collect::<Vec<_>>();
    // Protocol `CardRulesSummary::back_face`: a transform or modal double-faced
    // card whose front face is the deck card itself.
    if linked_faces.len() < 2
        && let Some(back) = first_value(card_object, &["backFace", "back_face"])
        && let Some(back_name) = manabrew_card_name(back)
    {
        let front_name = deck_name
            .split(" // ")
            .next()
            .unwrap_or(&deck_name)
            .trim()
            .to_string();
        linked_faces = vec![
            ExternalCardFaceSource {
                name: front_name,
                block: manabrew_card_source_block(card),
                score: Some(1.0),
            },
            ExternalCardFaceSource {
                name: back_name,
                block: manabrew_card_source_block(back),
                score: Some(1.0),
            },
        ];
    }

    if linked_faces.len() == 2 {
        let front_name = linked_faces[0].name.clone();
        let combined_name =
            scalar_string(first_value(card_object, &["combinedName", "combined_name"]))
                .or_else(|| deck_name.contains(" // ").then(|| deck_name.clone()))
                .unwrap_or_else(|| format!("{} // {}", linked_faces[0].name, linked_faces[1].name));
        // Scryfall layout names. Adventure and modal double-faced cards are
        // transform-like groups (the engine tells an Adventure by its subtype);
        // aftermath and Room cards are printed with the split layout.
        let layout = scalar_string(card_object.get("layout")).unwrap_or_default();
        let layout = match layout.to_ascii_lowercase().as_str() {
            "split" | "aftermath" | "room" => "split",
            "prepare" => "prepare",
            "flip" => "flip",
            "transform" => "transform",
            _ => "transform_like",
        };
        let has_fuse = first_value(card_object, &["hasFuse", "has_fuse"])
            .and_then(Value::as_bool)
            .unwrap_or_else(|| {
                manabrew_string_array(card_object, "keywords")
                    .iter()
                    .any(|keyword| keyword.eq_ignore_ascii_case("fuse"))
                    || linked_faces.iter().any(|face| {
                        face.block
                            .lines()
                            .any(|line| line.trim_start().starts_with("Fuse ("))
                    })
            });
        let mut aliases = Vec::new();
        for alias in [&deck_name, &combined_name] {
            if !alias.eq_ignore_ascii_case(&front_name)
                && !aliases
                    .iter()
                    .any(|entry: &ExternalCardAliasSource| entry.alias.eq_ignore_ascii_case(alias))
            {
                aliases.push(ExternalCardAliasSource {
                    alias: alias.clone(),
                    canonical: front_name.clone(),
                });
            }
        }
        return Some(ExternalCardSourceFile {
            canonical_name: front_name,
            aliases,
            replace_existing: false,
            group: ExternalCardSourceGroup::Linked {
                layout: layout.to_string(),
                combined_name,
                has_fuse,
                faces: linked_faces,
            },
        });
    }

    Some(ExternalCardSourceFile {
        canonical_name: deck_name.clone(),
        aliases: Vec::new(),
        replace_existing: false,
        group: ExternalCardSourceGroup::Single {
            name: deck_name,
            block: manabrew_card_source_block(card),
            score: Some(1.0),
        },
    })
}

fn manabrew_deck_sources(decks: &[Value]) -> Vec<ExternalCardSourceFile> {
    let mut seen = HashSet::new();
    let mut sources = Vec::new();
    for deck in decks {
        for card in manabrew_all_deck_cards(deck) {
            let Some(source) = manabrew_card_source(card) else {
                continue;
            };
            let source_names = WasmGame::external_source_definition_names(&source);
            if source_names
                .iter()
                .any(|name| seen.contains(&name.to_ascii_lowercase()))
            {
                continue;
            }
            seen.extend(
                source_names
                    .into_iter()
                    .map(|name| name.to_ascii_lowercase()),
            );
            sources.push(source);
        }
    }
    sources
}

fn manabrew_match_setup(input: &ManabrewMatchConfigInput) -> MatchSetupInput {
    let format = manabrew_format(input.format.as_deref());
    let starting_life =
        format.effective_starting_life(input.player_names.len(), input.starting_life);
    let opening_hand_size = format.effective_opening_hand_size(input.opening_hand_size);
    let decks: Vec<Vec<String>> = input
        .decks
        .iter()
        .map(|deck| manabrew_deck_section_names(deck, "cards"))
        .collect();
    let sideboards = input
        .decks
        .iter()
        .map(|deck| manabrew_deck_section_names(deck, "sideboard"))
        .collect();
    let commanders: Vec<Vec<String>> = input
        .decks
        .iter()
        .enumerate()
        .map(|(index, deck)| {
            manabrew_deck_commander_names(
                deck,
                input
                    .commander_names
                    .get(index)
                    .and_then(|value| value.as_deref()),
            )
        })
        .collect();
    let seed = input.seed.unwrap_or_else(|| {
        deterministic_match_seed(
            &input.player_names,
            starting_life,
            format,
            Some(&decks),
            Some(&commanders),
            opening_hand_size,
        )
    });
    MatchSetupInput {
        player_names: input.player_names.clone(),
        starting_life,
        seed,
        format,
        decks: Some(decks),
        sideboards: Some(sideboards),
        commanders: Some(commanders),
        planar_decks: None,
        vanguards: None,
        scheme_decks: None,
        conspiracies: None,
        commander_draft: None,
        opening_hand_size: Some(opening_hand_size),
        hidden_deck_manifests: None,
        public_decklists: None,
        free_for_all: None,
        teams: None,
        // Manabrew's protocol has the caller decide seating and expects the
        // first listed player to take the first turn, so this compatibility
        // path keeps its fixed seat instead of drawing one.
        starting_player: Some(0),
    }
}

/// Priority menus for a Manabrew match are bounded for the length of an
/// adapter call.
///
/// The adapter runs the engine synchronously on the host's main thread, and an
/// exact menu proves a payment for every castable card by simulating mana
/// abilities on copies of the game: seconds per priority pass on an ordinary
/// board, and unbounded once sacrifice-for-mana tokens are involved. A bounded
/// menu spends a fixed budget proving its offers and otherwise leaves out only
/// what certainly cannot be paid, as Ironsmith's own deferred browser menu
/// does; the payment prompt that follows stays exact.
/// The policy is restored on drop, so it never leaks into other uses of the
/// engine on the same thread.
struct ManabrewMenuPolicy {
    bounded: bool,
    proof_budget: usize,
    proven_only: u64,
}

/// Payment-search units a menu may spend per card proving what it offers: the
/// cap the browser's deferred menu gives a candidate. A card that needs more
/// stays offered to a person, whose payment prompt may then open unfunded.
const MANABREW_MENU_PROOF_BUDGET: usize = 128;

impl ManabrewMenuPolicy {
    /// `humans[i]` says whether seat `i` is played by a person. A bot answers
    /// a prompt it was refused the same way again, so it is offered only what
    /// was proven payable.
    fn enter(humans: &[bool]) -> Self {
        let previous = Self {
            bounded: ironsmith::game_loop::priority_analysis_bounded(),
            proof_budget: ironsmith::game_loop::priority_analysis_bounded_proof_budget(),
            proven_only: ironsmith::game_loop::priority_analysis_proven_offers_only(),
        };
        let bots = humans
            .iter()
            .enumerate()
            .take(64)
            .filter(|(_, human)| !**human)
            .fold(0u64, |mask, (seat, _)| mask | (1 << seat));
        ironsmith::game_loop::set_priority_analysis_bounded(true);
        ironsmith::game_loop::set_priority_analysis_bounded_proof_budget(
            MANABREW_MENU_PROOF_BUDGET,
        );
        ironsmith::game_loop::set_priority_analysis_proven_offers_only(bots);
        previous
    }
}

impl Drop for ManabrewMenuPolicy {
    fn drop(&mut self) {
        ironsmith::game_loop::set_priority_analysis_bounded(self.bounded);
        ironsmith::game_loop::set_priority_analysis_bounded_proof_budget(self.proof_budget);
        ironsmith::game_loop::set_priority_analysis_proven_offers_only(self.proven_only);
    }
}

fn manabrew_to_js(value: &impl Serialize, label: &str) -> Result<JsValue, JsValue> {
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    value
        .serialize(&serializer)
        .map_err(|error| JsValue::from_str(&format!("failed to encode {label}: {error}")))
}

fn player_id(player: PlayerId) -> String {
    format!("player-{}", player.index())
}

fn object_id(game: &GameState, id: ObjectId) -> String {
    game.object(id)
        .map(|object| format!("card-{}", object.stable_id.0.0))
        .unwrap_or_else(|| format!("card-object-{}", id.0))
}

fn protocol_error(
    code: ProtocolErrorCode,
    message: impl Into<String>,
    prompt_id: Option<u32>,
) -> ProtocolError {
    ProtocolError {
        code,
        message: message.into(),
        prompt_id,
    }
}

fn presentation(
    title: impl Into<String>,
    description: Option<String>,
    _source_card_id: Option<String>,
) -> PromptPresentation {
    PromptPresentation {
        title: title.into(),
        description,
        text: None,
        targets: Vec::new(),
    }
}

fn selection_options(labels: impl IntoIterator<Item = String>) -> Vec<SelectionOption> {
    labels
        .into_iter()
        .map(|label| SelectionOption {
            label,
            weight: 1,
            can_repeat: false,
        })
        .collect()
}

fn color_code(color: Color) -> &'static str {
    match color {
        Color::White => "W",
        Color::Blue => "U",
        Color::Black => "B",
        Color::Red => "R",
        Color::Green => "G",
    }
}

fn manabrew_mana_pool_entries(pool: &ironsmith::player::ManaPool) -> Vec<Mana> {
    [
        (ManaColor::White, pool.white),
        (ManaColor::Blue, pool.blue),
        (ManaColor::Black, pool.black),
        (ManaColor::Red, pool.red),
        (ManaColor::Green, pool.green),
        (ManaColor::Colorless, pool.colorless),
    ]
    .into_iter()
    .filter(|(_, amount)| *amount > 0)
    .map(|(color, amount)| Mana {
        color,
        amount: amount.min(i32::MAX as u64) as i32,
    })
    .collect()
}

fn manabrew_replan_command(
    mut preferences: ironsmith::mana_payment::ManaPaymentPreferences,
) -> ManaPaymentCommand {
    preferences.normalize();
    ManaPaymentCommand::Replan {
        required_source_ids: preferences
            .required_sources
            .into_iter()
            .map(|source| source.0.to_string())
            .collect(),
        required_activations: preferences
            .required_activations
            .into_iter()
            .map(|activation| ManaPaymentActivationCommand {
                source_id: activation.source.0.to_string(),
                ability_index: activation.ability_index,
                color_restriction: activation.color_restriction.as_deref().map(mana_selection_names),
            })
            .collect(),
        required_alternatives: preferences
            .required_alternatives
            .into_iter()
            .filter_map(|alternative| {
                let payment_kind = match alternative.kind {
                    ironsmith::mana_payment::ManaPaymentSourceKind::Convoke => "convoke",
                    ironsmith::mana_payment::ManaPaymentSourceKind::Improvise => "improvise",
                    ironsmith::mana_payment::ManaPaymentSourceKind::Waterbend => "waterbend",
                    ironsmith::mana_payment::ManaPaymentSourceKind::PutMinusOneCounter => "put_minus_one_counter",
                    ironsmith::mana_payment::ManaPaymentSourceKind::Delve => "delve",
                    ironsmith::mana_payment::ManaPaymentSourceKind::ManaAbility => return None,
                };
                Some(ManaPaymentAlternativeCommand {
                    source_id: alternative.source.0.to_string(),
                    payment_kind: payment_kind.to_string(),
                })
            })
            .collect(),
        excluded_source_ids: preferences
            .excluded_sources
            .into_iter()
            .map(|source| source.0.to_string())
            .collect(),
        preserved_source_ids: preferences
            .preserve_sources
            .into_iter()
            .map(|source| source.0.to_string())
            .collect(),
        prefer_life: preferences.prefer_life,
        x_allocation: preferences.x_allocation.map(|allocation| allocation.0),
        actual_mana_allocation: preferences.actual_mana_allocation.map(|allocation| allocation.0),
        required_life_pips: preferences
            .required_life_pips
            .into_iter()
            .map(|pip| pip.0)
            .collect(),
    }
}

fn selected_activation_matches(
    selected: &ironsmith::mana_payment::RequiredManaActivation,
    option: &ironsmith::mana_payment::ManaPaymentActivationOption,
) -> bool {
    selected.source == option.source
        && selected.ability_index == option.ability_index
        && selected.color_restriction == option.color_restriction
}

fn selected_alternative_matches(
    selected: &ironsmith::mana_payment::RequiredAlternativePayment,
    source: ObjectId,
    kind: ironsmith::mana_payment::ManaPaymentSourceKind,
) -> bool {
    selected.source == source && selected.kind == kind
}

fn manabrew_plan_is_fully_selected(
    context: &ironsmith::decisions::context::ManaPaymentContext,
) -> bool {
    if !context.plan.payable {
        return false;
    }
    let preferences = &context.request.preferences;
    let mut matched_activations = vec![false; preferences.required_activations.len()];
    let activations_selected = context.plan.mana_ability_steps.iter().all(|step| {
        let Some((index, _)) =
            preferences
                .required_activations
                .iter()
                .enumerate()
                .find(|(index, selected)| {
                    !matched_activations[*index]
                        && selected.source == step.source
                        && selected.ability_index == step.ability_index
                        && selected.color_restriction == step.color_restriction
                })
        else {
            return false;
        };
        matched_activations[index] = true;
        true
    });
    activations_selected
        && context
            .plan
            .allocations
            .iter()
            .all(|allocation| match allocation.payment {
                ironsmith::mana_payment::PlannedPipPayment::Mana(_) => true,
                ironsmith::mana_payment::PlannedPipPayment::Life(_) => {
                    preferences.prefer_life
                        || preferences.required_life_pips.contains(&allocation.pip)
                }
                ironsmith::mana_payment::PlannedPipPayment::Convoke(source) => {
                    preferences.required_alternatives.iter().any(|selected| {
                        selected_alternative_matches(
                            selected,
                            source,
                            ironsmith::mana_payment::ManaPaymentSourceKind::Convoke,
                        )
                    })
                }
                ironsmith::mana_payment::PlannedPipPayment::Improvise(source) => {
                    preferences.required_alternatives.iter().any(|selected| {
                        selected_alternative_matches(
                            selected,
                            source,
                            ironsmith::mana_payment::ManaPaymentSourceKind::Improvise,
                        )
                    })
                }
                ironsmith::mana_payment::PlannedPipPayment::Waterbend(source) => {
                    preferences.required_alternatives.iter().any(|selected| {
                        selected_alternative_matches(
                            selected,
                            source,
                            ironsmith::mana_payment::ManaPaymentSourceKind::Waterbend,
                        )
                    })
                }
                ironsmith::mana_payment::PlannedPipPayment::Delve(source) => {
                    preferences.required_alternatives.iter().any(|selected| {
                        selected_alternative_matches(
                            selected,
                            source,
                            ironsmith::mana_payment::ManaPaymentSourceKind::Delve,
                        )
                    })
                }
                ironsmith::mana_payment::PlannedPipPayment::PutMinusOneCounter(source) => {
                    preferences.required_alternatives.iter().any(|selected| {
                        selected_alternative_matches(
                            selected,
                            source,
                            ironsmith::mana_payment::ManaPaymentSourceKind::PutMinusOneCounter,
                        )
                    })
                }
                ironsmith::mana_payment::PlannedPipPayment::Assist { .. } => false,
            })
}

fn manabrew_remaining_mana_cost(
    context: &ironsmith::decisions::context::ManaPaymentContext,
) -> String {
    if !context.plan.payable {
        return context
            .request
            .cost
            .pips()
            .iter()
            .map(|pip| {
                if pip.as_slice() == [ManaSymbol::X] {
                    format!("{{{}}}", context.request.x_value)
                } else {
                    ManaCost::from_pips(vec![pip.clone()]).to_oracle()
                }
            })
            .collect();
    }
    let preferences = &context.request.preferences;
    context
        .plan
        .allocations
        .iter()
        .filter(|allocation| match allocation.payment {
            ironsmith::mana_payment::PlannedPipPayment::Mana(_) => true,
            ironsmith::mana_payment::PlannedPipPayment::Life(_) => {
                !preferences.prefer_life
                    && !preferences.required_life_pips.contains(&allocation.pip)
            }
            ironsmith::mana_payment::PlannedPipPayment::Convoke(source) => {
                !preferences.required_alternatives.iter().any(|selected| {
                    selected_alternative_matches(
                        selected,
                        source,
                        ironsmith::mana_payment::ManaPaymentSourceKind::Convoke,
                    )
                })
            }
            ironsmith::mana_payment::PlannedPipPayment::Improvise(source) => {
                !preferences.required_alternatives.iter().any(|selected| {
                    selected_alternative_matches(
                        selected,
                        source,
                        ironsmith::mana_payment::ManaPaymentSourceKind::Improvise,
                    )
                })
            }
            ironsmith::mana_payment::PlannedPipPayment::Waterbend(source) => {
                !preferences.required_alternatives.iter().any(|selected| {
                    selected_alternative_matches(
                        selected,
                        source,
                        ironsmith::mana_payment::ManaPaymentSourceKind::Waterbend,
                    )
                })
            }
            ironsmith::mana_payment::PlannedPipPayment::Delve(source) => {
                !preferences.required_alternatives.iter().any(|selected| {
                    selected_alternative_matches(
                        selected,
                        source,
                        ironsmith::mana_payment::ManaPaymentSourceKind::Delve,
                    )
                })
            }
            ironsmith::mana_payment::PlannedPipPayment::PutMinusOneCounter(source) => {
                !preferences.required_alternatives.iter().any(|selected| {
                    selected_alternative_matches(
                        selected,
                        source,
                        ironsmith::mana_payment::ManaPaymentSourceKind::PutMinusOneCounter,
                    )
                })
            }
            ironsmith::mana_payment::PlannedPipPayment::Assist { .. } => true,
        })
        .map(|allocation| {
            format!(
                "{{{}}}",
                allocation
                    .alternatives
                    .iter()
                    .map(mana_symbol_display_code)
                    .collect::<Vec<_>>()
                    .join("/")
            )
        })
        .collect()
}

fn protocol_step(game: &GameState) -> StepKind {
    use ironsmith::game_state::{Phase, Step};
    match (game.turn.phase, game.turn.step) {
        (_, Some(Step::Untap)) => StepKind::Untap,
        (_, Some(Step::Upkeep)) => StepKind::Upkeep,
        (_, Some(Step::Draw)) => StepKind::Draw,
        (Phase::FirstMain, _) => StepKind::Main1,
        (_, Some(Step::BeginCombat)) => StepKind::CombatBegin,
        (_, Some(Step::DeclareAttackers)) => StepKind::CombatDeclareAttackers,
        (_, Some(Step::DeclareBlockers)) => StepKind::CombatDeclareBlockers,
        (_, Some(Step::CombatDamage)) => StepKind::CombatDamage,
        (_, Some(Step::EndCombat)) => StepKind::CombatEnd,
        (Phase::NextMain, _) => StepKind::Main2,
        (_, Some(Step::End)) => StepKind::EndOfTurn,
        (_, Some(Step::Cleanup)) => StepKind::Cleanup,
        (Phase::Combat, None) => StepKind::CombatBegin,
        (Phase::Ending, None) => StepKind::EndOfTurn,
        (Phase::Beginning, None) => StepKind::Untap,
    }
}

fn protocol_target(game: &GameState, target: Target) -> TargetRef {
    match target {
        Target::Player(player) => TargetRef {
            kind: TargetKind::Player,
            id: player_id(player),
            intent: None,
            oracle: None,
        },
        // An ability on the stack is named by its own stack id, which is not
        // an object; point at its stack object, not at its source's card.
        Target::Object(object)
            if game
                .stack_entry_index_for_target(object)
                .is_some_and(|index| game.stack[index].is_ability) =>
        {
            let index = game
                .stack_entry_index_for_target(object)
                .expect("checked above");
            let entry = &game.stack[index];
            TargetRef {
                kind: TargetKind::Spell,
                id: game
                    .object(entry.object_id)
                    .map(|source| format!("stack-{}-{index}", source.stable_id.0.0))
                    .unwrap_or_else(|| format!("stack-ability-{}", object.0)),
                intent: None,
                oracle: None,
            }
        }
        Target::Object(object) => TargetRef {
            kind: if game
                .object(object)
                .is_some_and(|object| object.zone == Zone::Stack)
            {
                TargetKind::Spell
            } else {
                TargetKind::Card
            },
            id: object_id(game, object),
            intent: None,
            oracle: None,
        },
    }
}

fn counter_name(counter: ironsmith::object::CounterType) -> String {
    match counter {
        ironsmith::object::CounterType::PlusOnePlusOne => "P1P1".to_string(),
        ironsmith::object::CounterType::MinusOneMinusOne => "M1M1".to_string(),
        ironsmith::object::CounterType::Named(value) => value.to_ascii_uppercase(),
        other => format!("{other:?}"),
    }
}

fn protocol_card(game: &GameState, id: ObjectId) -> Option<CardDto> {
    use ironsmith::object::{AttachmentTarget, ObjectKind};

    let object = game.object(id)?;
    let chars = game.current_characteristics(id)?;
    let colors = Color::ALL
        .iter()
        .copied()
        .filter(|color| chars.colors.contains(*color))
        .map(color_code)
        .collect::<String>();
    let counters = object
        .counters
        .iter()
        .map(|(counter, amount)| (counter_name(*counter), *amount))
        .collect();
    let (is_attacking, attacking_player_id, attack_target_id) = game
        .combat
        .as_ref()
        .and_then(|combat| combat.attackers.iter().find(|info| info.creature == id))
        .map(|info| {
            // CR 506.4c: a creature attacking nothing has no attack target.
            let target = match info.target {
                AttackTarget::Player(player) => Some(player_id(player)),
                AttackTarget::Planeswalker(object) => Some(object_id(game, object)),
                AttackTarget::Battle(object) => Some(object_id(game, object)),
                AttackTarget::Nothing { .. } => None,
            };
            let defending_player =
                ironsmith::combat_state::defending_player_for_attack_target(game, &info.target)
                    .map(player_id);
            (true, defending_player, target)
        })
        .unwrap_or((false, None, None));
    let attached_to = object.attached_to.map(|target| match target {
        AttachmentTarget::Object(object) => object_id(game, object),
        AttachmentTarget::Player(player) => player_id(player),
    });

    Some(CardDto {
        id: object_id(game, id),
        identity: CardIdentity {
            name: chars.name.to_owned_string(),
            set_code: String::new(),
            card_number: String::new(),
            is_token: object.kind == ObjectKind::Token,
            token_script: None,
        },
        color: colors,
        mana_cost: object
            .mana_cost
            .as_ref()
            .map(|cost| cost.to_oracle())
            .unwrap_or_default(),
        cmc: object
            .mana_cost
            .as_ref()
            .map(|cost| cost.mana_value() as i32)
            .unwrap_or(0),
        types: chars
            .card_types
            .iter()
            .map(|value| format!("{value:?}"))
            .collect(),
        subtypes: chars
            .subtypes
            .iter()
            .map(|value| format!("{value:?}"))
            .collect(),
        supertypes: chars
            .supertypes
            .iter()
            .map(|value| format!("{value:?}"))
            .collect(),
        power: chars.power.map(|value| value.to_string()),
        toughness: chars.toughness.map(|value| value.to_string()),
        base_power: object.base_power.map(|value| value.base_value()),
        base_toughness: object.base_toughness.map(|value| value.base_value()),
        text: chars.compiled_card_text.to_string(),
        controller_id: player_id(chars.controller),
        owner_id: player_id(object.owner),
        tapped: game.is_tapped(id),
        is_attacking,
        attacking_player_id,
        attack_target_id,
        keywords: chars
            .static_abilities
            .iter()
            .map(|ability| format!("{:?}", ability.id()))
            .collect(),
        counters,
        damage: game.damage_on(id) as i32,
        summoning_sick: game.is_summoning_sick(id),
        is_copy: object.kind == ObjectKind::SpellCopy,
        is_double_faced: object.other_face.is_some() || object.other_face_name.is_some(),
        is_transformed: game.transform_count(id) % 2 == 1,
        is_face_down: game.is_face_down(id),
        is_bestowed: object.is_bestow_overlay_active(),
        phased_out: game.is_phased_out(id),
        exerted: game.object_exerted_this_turn(id),
        is_ring_bearer: game
            .player(chars.controller)
            .is_some_and(|player| player.ring_bearer == Some(id)),
        attached_to,
        attachment_ids: object
            .attachments
            .iter()
            .map(|attachment| object_id(game, *attachment))
            .collect(),
        is_madness_exiled: game.is_madness_exiled(id),
        is_plotted: game.plotted_by(id).is_some(),
        // CR 903.8: {2} for each previous cast from the command zone.
        commander_tax: (object.zone == Zone::Command && game.is_commander(id)).then(|| {
            game.commander_cast_count(id).saturating_mul(2).min(i32::MAX as u32) as i32
        }),
        zone: match object.zone {
            Zone::Battlefield => Some(ZoneKind::Battlefield),
            Zone::Hand => Some(ZoneKind::Hand),
            Zone::Library => Some(ZoneKind::Library),
            Zone::Graveyard => Some(ZoneKind::Graveyard),
            Zone::Exile => Some(ZoneKind::Exile),
            Zone::Command => Some(ZoneKind::Command),
            _ => None,
        },
        ..CardDto::default()
    })
}

/// The protocol's name for an alternative cost, when it has one. Costs it does
/// not name are sent as `PlayCardMode::StaticAlternative`.
fn protocol_alternative_cost_kind(name: &str) -> Option<AlternativeCostKind> {
    let normalized = name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    Some(match normalized.as_str() {
        "flashback" => AlternativeCostKind::Flashback,
        "spectacle" => AlternativeCostKind::Spectacle,
        "evoke" => AlternativeCostKind::Evoke,
        "dash" => AlternativeCostKind::Dash,
        "blitz" => AlternativeCostKind::Blitz,
        "escape" => AlternativeCostKind::Escape,
        "overload" => AlternativeCostKind::Overload,
        "madness" => AlternativeCostKind::Madness,
        "foretell" => AlternativeCostKind::Foretell,
        "emerge" => AlternativeCostKind::Emerge,
        "suspend" => AlternativeCostKind::Suspend,
        "morph" => AlternativeCostKind::Morph,
        "megamorph" => AlternativeCostKind::Megamorph,
        "bestow" => AlternativeCostKind::Bestow,
        "warp" => AlternativeCostKind::Warp,
        "plot" | "plotted" => AlternativeCostKind::Plotted,
        "awaken" => AlternativeCostKind::Awaken,
        "disturb" => AlternativeCostKind::Disturb,
        "harmonize" => AlternativeCostKind::Harmonize,
        "freerunning" => AlternativeCostKind::Freerunning,
        "impending" => AlternativeCostKind::Impending,
        "mayhem" => AlternativeCostKind::Mayhem,
        "morethanmeetstheeye" => AlternativeCostKind::MTMtE,
        "mutate" => AlternativeCostKind::Mutate,
        "prowl" => AlternativeCostKind::Prowl,
        "sneak" => AlternativeCostKind::Sneak,
        "surge" => AlternativeCostKind::Surge,
        "webslinging" => AlternativeCostKind::WebSlinging,
        "prototype" => AlternativeCostKind::Prototype,
        _ => return None,
    })
}

fn protocol_play_card_mode(
    game: &GameState,
    spell: ObjectId,
    method: &ironsmith::alternative_cast::CastingMethod,
) -> PlayCardMode {
    use ironsmith::alternative_cast::CastingMethod;
    let named = |index: usize| {
        game.object(spell)
            .and_then(|object| object.alternative_casts.get(index))
            .and_then(|alternative| protocol_alternative_cost_kind(alternative.name()))
            .map(|cost| PlayCardMode::Alternative { cost })
            .unwrap_or(PlayCardMode::StaticAlternative)
    };
    match method {
        CastingMethod::Normal => PlayCardMode::Normal,
        CastingMethod::SplitOtherHalf => PlayCardMode::RoomRightSplit,
        CastingMethod::GrantedFlashback => PlayCardMode::Alternative {
            cost: AlternativeCostKind::Flashback,
        },
        CastingMethod::GrantedEscape { .. } => PlayCardMode::Alternative {
            cost: AlternativeCostKind::Escape,
        },
        CastingMethod::Alternative(index) => named(*index),
        CastingMethod::AlternativePrice {
            prototype: Some(_), ..
        } => PlayCardMode::Alternative {
            cost: AlternativeCostKind::Prototype,
        },
        CastingMethod::ExactPermission { origin, .. } => {
            protocol_play_card_mode(game, spell, origin)
        }
        _ => PlayCardMode::StaticAlternative,
    }
}

fn visible_card(game: &GameState, id: ObjectId) -> Option<CardView> {
    protocol_card(game, id).map(CardView::Visible)
}

fn hidden_card(game: &GameState, id: ObjectId) -> CardView {
    CardView::Hidden {
        id: object_id(game, id),
    }
}

impl WasmGame {
    /// The state a Manabrew view or prompt describes. While a spell or ability
    /// is partway through resolving (or being cast), the engine keeps that
    /// in-progress state beside the committed one; the pending decision refers
    /// to it, exactly as the native snapshot does.
    fn manabrew_game(&self) -> &GameState {
        self.pending_decision_game.as_deref().unwrap_or(&self.game)
    }

    fn register_manabrew_deck_sources_input(
        &mut self,
        decks: &[Value],
    ) -> ExternalCardRegistrationSummary {
        self.register_external_card_sources_input(ExternalCardSourcesInput::Many(
            manabrew_deck_sources(decks),
        ))
    }

    /// One issue per deck card that would enter without its printed loyalty
    /// or defense, in the shape deck validation already reports.
    fn manabrew_printed_stat_issues(input: &ManabrewMatchConfigInput) -> Vec<MatchValidationIssue> {
        const SECTIONS: &[&str] = &["cards", "commanders", "sideboard", "companion"];
        let mut issues = Vec::new();
        for (player_index, deck) in input.decks.iter().enumerate() {
            let deck_object = object_value(deck);
            let mut seen = HashSet::new();
            for section in SECTIONS {
                let cards: Vec<&Value> = match deck_object.get(*section) {
                    Some(Value::Array(cards)) => cards.iter().collect(),
                    Some(card @ Value::Object(_)) => vec![card],
                    _ => Vec::new(),
                };
                for card in cards {
                    let Some(card_name) = manabrew_card_name(card) else {
                        continue;
                    };
                    if !seen.insert(card_name.to_lowercase()) {
                        continue;
                    }
                    for error in manabrew_missing_printed_stats(card) {
                        issues.push(MatchValidationIssue {
                            player_index,
                            player_name: input
                                .player_names
                                .get(player_index)
                                .cloned()
                                .unwrap_or_default(),
                            section: section.to_string(),
                            card_name: card_name.clone(),
                            error,
                        });
                    }
                }
            }
        }
        issues
    }

    fn manabrew_player(&self, index: u8) -> Result<PlayerId, ProtocolError> {
        let player = PlayerId::from_index(index);
        self.manabrew_game().player(player).map(|_| player).ok_or_else(|| {
            protocol_error(
                ProtocolErrorCode::WrongPlayer,
                format!("player-{index} is not a player in this match"),
                self.manabrew_open_prompt
                    .as_ref()
                    .map(|prompt| prompt.prompt_id),
            )
        })
    }

    fn manabrew_card_is_visible(&self, id: ObjectId, viewer: Option<PlayerId>) -> bool {
        let Some(object) = self.manabrew_game().object(id) else {
            return false;
        };
        // A Verified match holds a hidden card as a placeholder until its
        // identity is opened; there is nothing to show before that.
        if self.manabrew_game().is_hidden_card_placeholder(id) {
            return false;
        }
        let has_view_permission = self
            .active_viewed_cards
            .iter()
            .chain(self.active_audit_viewed_cards.iter())
            .any(|view| {
                view.zone == object.zone
                    && view.subject == object.owner
                    && (view.cards.contains(&id)
                        || view.card_stable_ids.contains(&object.stable_id))
                    && (view.public
                        || viewer == Some(view.viewer)
                        || (view.zone != Zone::OutsideGame
                            && viewer == Some(self.manabrew_game().controlling_player_for(view.viewer))))
            });
        if has_view_permission {
            return true;
        }
        match object.zone {
            Zone::OutsideGame => viewer == Some(object.owner),
            Zone::Hand => {
                viewer == Some(object.owner)
                    || viewer == Some(self.manabrew_game().controlling_player_for(object.owner))
            }
            Zone::Library => self.manabrew_library_top_is_visible(id, object.owner, viewer),
            // CR 708.9: at the end of the game all face-down permanents are
            // revealed to all players. A concession ends the match in the host
            // result without marking an engine player as the winner.
            Zone::Battlefield if self.manabrew_game().is_face_down(id) => self.game_over.is_some()
                || viewer.is_some_and(|viewer| {
                    self.manabrew_game().can_player_look_at_face_down_permanent(id, viewer)
                }),
            Zone::Exile if self.manabrew_game().is_face_down(id) => viewer.is_some_and(|viewer| {
                self.manabrew_game()
                    .can_player_look_at_face_down_exiled_card(id, viewer)
            }),
            _ => true,
        }
    }

    /// The standing library-top views the native snapshot adds on top of the
    /// one-shot view effects: "You may look at the top card of your library
    /// any time" (Bolas's Citadel), a top revealed by a static, or a granted
    /// look at another player's top. A top that changed while an action is
    /// being announced stays hidden until the announcement ends.
    fn manabrew_library_top_is_visible(
        &self,
        id: ObjectId,
        owner: PlayerId,
        viewer: Option<PlayerId>,
    ) -> bool {
        let game = self.manabrew_game();
        if !self.static_library_top_visibility_window().allows(game, owner) {
            return false;
        }
        match viewer {
            Some(viewer) => library_top_visible_to_perspective(game, viewer, owner, id),
            None => {
                game.player(owner).and_then(|player| player.library.last()) == Some(&id)
                    && !game.has_library_top_announcement()
                    && crate::library_top_revealed_by_static_ability(game, owner)
            }
        }
    }

    fn manabrew_zone_cards(
        &self,
        ids: impl IntoIterator<Item = ObjectId>,
        viewer: Option<PlayerId>,
        omit_hidden: bool,
    ) -> Vec<CardView> {
        ids.into_iter()
            .filter_map(|id| {
                if self.manabrew_card_is_visible(id, viewer) {
                    let mut card = visible_card(self.manabrew_game(), id)?;
                    if let CardView::Visible(dto) = &mut card {
                        dto.tapped |= self.manabrew_selected_tap(id);
                    }
                    Some(card)
                } else if omit_hidden {
                    None
                } else {
                    Some(hidden_card(self.manabrew_game(), id))
                }
            })
            .collect()
    }

    fn manabrew_selected_tap(&self, id: ObjectId) -> bool {
        let Some(DecisionContext::ManaPayment(context)) = self.pending_decision.as_ref() else {
            return false;
        };
        context
            .request
            .preferences
            .required_activations
            .iter()
            .any(|selected| {
                selected.source == id
                    && self
                        .manabrew_game()
                        .current_ability(id, selected.ability_index)
                        .is_some_and(|ability| {
                            matches!(&ability.kind,
                    ironsmith::ability::AbilityKind::Activated(a) if a.has_tap_cost())
                        })
            })
            || context
                .request
                .preferences
                .required_alternatives
                .iter()
                .any(|selected| {
                    selected.source == id
                        && matches!(
                            selected.kind,
                            ironsmith::mana_payment::ManaPaymentSourceKind::Convoke
                                | ironsmith::mana_payment::ManaPaymentSourceKind::Improvise
                        | ironsmith::mana_payment::ManaPaymentSourceKind::Waterbend
                        )
                })
    }

    fn manabrew_zones(&self, viewer: Option<PlayerId>) -> Vec<ZoneDto> {
        let mut zones = Vec::new();
        for player in &self.manabrew_game().players {
            let owner_id = player_id(player.id);
            let battlefield: Vec<_> = self
                .manabrew_game()
                .battlefield
                .iter()
                .copied()
                .filter(|id| {
                    self.manabrew_game()
                        .object(*id)
                        .is_some_and(|object| self.manabrew_game().controller_of(object) == player.id)
                })
                .collect();
            zones.push(ZoneDto {
                zone: ZoneKind::Battlefield,
                owner_id: owner_id.clone(),
                count: battlefield.len(),
                cards: self.manabrew_zone_cards(battlefield, viewer, false),
            });
            zones.push(ZoneDto {
                zone: ZoneKind::Hand,
                owner_id: owner_id.clone(),
                count: player.hand.len(),
                cards: self.manabrew_zone_cards(player.hand.iter().copied(), viewer, true),
            });
            zones.push(ZoneDto {
                zone: ZoneKind::Library,
                owner_id: owner_id.clone(),
                count: player.library.len(),
                cards: self.manabrew_zone_cards(player.library.iter().copied(), viewer, true),
            });
            zones.push(ZoneDto {
                zone: ZoneKind::Graveyard,
                owner_id: owner_id.clone(),
                count: player.graveyard.len(),
                cards: self.manabrew_zone_cards(player.graveyard.iter().copied(), viewer, false),
            });
            let exile: Vec<_> = self
                .manabrew_game()
                .exile
                .iter()
                .copied()
                .filter(|id| {
                    self.manabrew_game()
                        .object(*id)
                        .is_some_and(|object| object.owner == player.id)
                })
                .collect();
            zones.push(ZoneDto {
                zone: ZoneKind::Exile,
                owner_id: owner_id.clone(),
                count: exile.len(),
                cards: self.manabrew_zone_cards(exile, viewer, false),
            });
            let command: Vec<_> = self
                .manabrew_game()
                .command_zone
                .iter()
                .copied()
                .filter(|id| {
                    self.manabrew_game()
                        .object(*id)
                        .is_some_and(|object| object.owner == player.id)
                })
                .collect();
            zones.push(ZoneDto {
                zone: ZoneKind::Command,
                owner_id,
                count: command.len(),
                cards: self.manabrew_zone_cards(command, viewer, false),
            });
        }
        zones
    }

    fn manabrew_projected_payment_state(
        &self,
        player: PlayerId,
    ) -> Option<(ironsmith::player::ManaPool, i32)> {
        let Some(DecisionContext::ManaPayment(context)) = self.pending_decision.as_ref() else {
            return None;
        };
        if context.request.payer != player {
            return None;
        }
        let mut pool = context.plan.pool_before.clone();
        let mut selected = context.request.preferences.required_activations.clone();
        let mut selected_prefix = true;
        for step in &context.plan.mana_ability_steps {
            let Some(index) = selected.iter().position(|activation| {
                activation.source == step.source
                    && activation.ability_index == step.ability_index
                    && activation.color_restriction == step.color_restriction
            }) else {
                selected_prefix = false;
                continue;
            };
            selected.remove(index);
            if selected_prefix {
                pool = step.expected_pool_after.clone();
            } else {
                pool.white = pool.white.saturating_add(step.expected_mana.white);
                pool.blue = pool.blue.saturating_add(step.expected_mana.blue);
                pool.black = pool.black.saturating_add(step.expected_mana.black);
                pool.red = pool.red.saturating_add(step.expected_mana.red);
                pool.green = pool.green.saturating_add(step.expected_mana.green);
                pool.colorless = pool.colorless.saturating_add(step.expected_mana.colorless);
            }
        }
        let life_paid = context
            .plan
            .allocations
            .iter()
            .filter_map(|allocation| match allocation.payment {
                ironsmith::mana_payment::PlannedPipPayment::Life(amount)
                    if context.request.preferences.prefer_life
                        || context
                            .request
                            .preferences
                            .required_life_pips
                            .contains(&allocation.pip) =>
                {
                    Some(amount)
                }
                _ => None,
            })
            .fold(0u32, u32::saturating_add)
            .min(i32::MAX as u32) as i32;
        let life = self
            .manabrew_game()
            .player(player)
            .map(|player| player.life.saturating_sub(life_paid))?;
        Some((pool, life))
    }

    fn manabrew_players(&self) -> Vec<PlayerDto> {
        self.manabrew_game()
            .players
            .iter()
            .map(|player| {
                let projected = self.manabrew_projected_payment_state(player.id);
                let mana_pool_view = projected
                    .as_ref()
                    .map(|(pool, _)| pool)
                    .unwrap_or(&player.mana_pool);
                let mut counters = BTreeMap::new();
                if player.poison_counters > 0 {
                    counters.insert(PlayerCounterKind::Poison, player.poison_counters);
                }
                if player.energy_counters > 0 {
                    counters.insert(PlayerCounterKind::Energy, player.energy_counters);
                }
                if player.experience_counters > 0 {
                    counters.insert(PlayerCounterKind::Experience, player.experience_counters);
                }
                for (counter, amount) in &player.other_counters {
                    match counter {
                        ironsmith::object::CounterType::Rad => {
                            counters.insert(PlayerCounterKind::Radiation, *amount);
                        }
                        ironsmith::object::CounterType::Named(name)
                            if name.eq_ignore_ascii_case("ticket") =>
                        {
                            counters.insert(PlayerCounterKind::Ticket, *amount);
                        }
                        _ => {}
                    }
                }
                let mana_pool = [
                    (ManaColor::White, mana_pool_view.white),
                    (ManaColor::Blue, mana_pool_view.blue),
                    (ManaColor::Black, mana_pool_view.black),
                    (ManaColor::Red, mana_pool_view.red),
                    (ManaColor::Green, mana_pool_view.green),
                    (ManaColor::Colorless, mana_pool_view.colorless),
                ]
                .into_iter()
                // The protocol carries u32 counts; larger pools saturate.
                .map(|(color, amount)| (color, u32::try_from(amount).unwrap_or(u32::MAX)))
                .collect();
                let commander_damage = player
                    .commander_damage
                    .iter()
                    .map(|(commander, damage)| (object_id(self.manabrew_game(), *commander), *damage as i32))
                    .collect();
                PlayerDto {
                    id: player_id(player.id),
                    name: player.name.clone(),
                    // A player who lost has also left the game (CR 104.5), so
                    // check the loss first. The engine does not record whether
                    // a loss was a concession; the host knows which seat sent
                    // the concede directive and labels that itself.
                    status: if player.has_lost {
                        PlayerStatus::Lost
                    } else if player.has_left_game {
                        PlayerStatus::Conceded
                    } else {
                        PlayerStatus::Playing
                    },
                    is_human: self
                        .manabrew_human_players
                        .get(player.id.index())
                        .copied()
                        .unwrap_or(true),
                    life: projected.map(|(_, life)| life).unwrap_or(player.life),
                    max_hand_size: player.max_hand_size,
                    unlimited_hand_size: player.max_hand_size == i32::MAX,
                    lands_played_this_turn: player
                        .lands_played_this_turn
                        .saturating_add(player.nonland_land_play_usage_this_turn)
                        .min(i32::MAX as u32) as i32,
                    max_land_plays_per_turn: player.land_plays_per_turn.min(i32::MAX as u32)
                        as i32,
                    unlimited_land_plays: false,
                    cards_drawn_this_turn: 0,
                    damage_prevention: 0,
                    is_extra_turn: false,
                    extra_turn_count: 0,
                    controlled_by: None,
                    player_keywords: Vec::new(),
                    commander_casts: player
                        .commanders
                        .iter()
                        .map(|commander| {
                            (
                                object_id(self.manabrew_game(), *commander),
                                self.manabrew_game().commander_cast_count(*commander),
                            )
                        })
                        .collect(),
                    dungeon_state: None,
                    active_scheme_names: None,
                    team_number: None,
                    has_enduring_story: false,
                    counters,
                    mana_pool,
                    commander_damage,
                    has_city_blessing: self.manabrew_game().has_citys_blessing(player.id),
                    ring_level: player.ring_temptations as i32,
                    speed: player.speed.unwrap_or(0) as i32,
                }
            })
            .collect()
    }

    fn manabrew_stack(&self) -> Vec<StackObjectDto> {
        self.manabrew_game()
            .stack
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                let live = self
                    .manabrew_game()
                    .object(entry.object_id)
                    .zip(self.manabrew_game().current_characteristics(entry.object_id));
                let Some((object, chars)) = live else {
                    // CR 113.7a: an ability on the stack exists independently
                    // of its source. Once the source has left its zone, show
                    // the ability from what the stack entry remembers of it.
                    if !entry.is_ability {
                        return None;
                    }
                    let name = entry
                        .source_name
                        .clone()
                        .or_else(|| {
                            entry
                                .source_snapshot
                                .as_ref()
                                .map(|snapshot| snapshot.name.to_string())
                        })
                        .unwrap_or_else(|| "Ability".to_string());
                    let source_id = entry
                        .source_stable_id
                        .map(|stable| format!("card-{}", stable.0.0))
                        .unwrap_or_else(|| format!("card-object-{}", entry.object_id.0));
                    return Some(StackObjectDto {
                        id: format!(
                            "stack-ability-{}",
                            entry.ability_id.unwrap_or(entry.object_id).0
                        ),
                        source_id,
                        controller_id: player_id(entry.controller),
                        owner_id: player_id(entry.controller),
                        identity: Some(CardIdentity {
                            name,
                            ..CardIdentity::default()
                        }),
                        targets: entry
                            .targets
                            .iter()
                            .copied()
                            .map(|target| protocol_target(self.manabrew_game(), target))
                            .collect(),
                        ..StackObjectDto::default()
                    });
                };
                Some(StackObjectDto {
                    id: format!("stack-{}-{index}", object.stable_id.0.0),
                    source_id: object_id(self.manabrew_game(), entry.object_id),
                    controller_id: player_id(entry.controller),
                    owner_id: player_id(object.owner),
                    identity: Some(CardIdentity {
                        name: chars.name.to_owned_string(),
                        set_code: String::new(),
                        card_number: String::new(),
                        is_token: object.kind == ironsmith::object::ObjectKind::Token,
                        token_script: None,
                    }),
                    text: chars.compiled_card_text.to_string(),
                    source_ability_text: None,
                    // An ability's stack object is not a card; give the client
                    // the permanent or card it came from to render.
                    source_card: entry
                        .is_ability
                        .then(|| protocol_card(self.manabrew_game(), entry.object_id))
                        .flatten(),
                    is_permanent_spell: !entry.is_ability
                        && !chars.card_types.contains(&CardType::Instant)
                        && !chars.card_types.contains(&CardType::Sorcery),
                    is_casting: self
                        .priority_state
                        .pending_cast
                        .as_ref()
                        .is_some_and(|pending| pending.stack_id == entry.object_id),
                    targets: entry
                        .targets
                        .iter()
                        .copied()
                        .map(|target| protocol_target(self.manabrew_game(), target))
                        .collect(),
                })
            })
            .collect()
    }

    fn manabrew_state(&self, viewer: Option<PlayerId>) -> StateUpdate {
        let combat_assignments = self
            .manabrew_game()
            .combat
            .as_ref()
            .into_iter()
            .flat_map(|combat| {
                combat.blockers.iter().flat_map(|(attacker, blockers)| {
                    blockers.iter().map(|blocker| CombatAssignmentDto {
                        blocker_id: object_id(self.manabrew_game(), *blocker),
                        attacker_id: object_id(self.manabrew_game(), *attacker),
                    })
                })
            })
            .collect();
        let winner_id = match self.game_over.as_ref() {
            Some(GameResult::Winner(player)) => Some(player_id(*player)),
            _ => None,
        };
        StateUpdate {
            game_view: GameViewDto {
                game_id: self.manabrew_game_id.clone(),
                turn: self.manabrew_game().turn.turn_number,
                step: protocol_step(self.manabrew_game()),
                combat_assignments,
                active_player_id: player_id(self.manabrew_game().turn.active_player),
                priority_player_id: player_id(
                    self.manabrew_game()
                        .turn
                        .priority_player
                        .unwrap_or(self.manabrew_game().turn.active_player),
                ),
                players: self.manabrew_players(),
                zones: self.manabrew_zones(viewer),
                stack: self.manabrew_stack(),
                game_over: self.game_over.is_some(),
                winner_id,
                monarch_id: self.manabrew_game().monarch.map(player_id),
                initiative_holder_id: self.manabrew_game().initiative.map(player_id),
                day_time: if !self.manabrew_game().has_day_night {
                    DayTime::Neither
                } else if self.manabrew_game().is_night {
                    DayTime::Night
                } else {
                    DayTime::Day
                },
                active_plane_names: None,
                // Ironsmith keeps no Manabrew checkpoints or decision journal;
                // the restore, recording and repeat directives are rejected.
                checkpoints: Vec::new(),
                restore_vote: None,
                snapshot_recording: false,
                decision_history: None,
            },
        }
    }

    fn manabrew_source_card_id(&self, context: &DecisionContext) -> Option<String> {
        context.source().map(|source| object_id(self.manabrew_game(), source))
    }

    fn manabrew_action_card(&self, action: &LegalAction) -> Option<ObjectId> {
        use ironsmith::special_actions::SpecialAction;
        match action {
            LegalAction::UsePregameAction { card_id, .. }
            | LegalAction::OpenExiledCardForPlay { card_id, .. }
            | LegalAction::CastExiledCardFaceDown { card_id, .. } => Some(*card_id),
            LegalAction::CastSpell { spell_id, .. } => Some(*spell_id),
            LegalAction::ActivateAbility { source, .. }
            | LegalAction::ActivateManaAbility { source, .. } => Some(*source),
            LegalAction::PlayLand { land_id } | LegalAction::PlayLandBackFace { land_id } => {
                Some(*land_id)
            }
            LegalAction::TurnFaceUp { creature_id, .. } => Some(*creature_id),
            LegalAction::SpecialAction(action) => match action {
                SpecialAction::PlayLand { card_id }
                | SpecialAction::PlayLandBackFace { card_id }
                | SpecialAction::Suspend { card_id }
                | SpecialAction::PrepareExile { card_id, .. }
                | SpecialAction::PrepareGrantedExile { card_id, .. }
                | SpecialAction::Foretell { card_id }
                | SpecialAction::Plot { card_id }
                | SpecialAction::PlotGranted { card_id }
                | SpecialAction::Companion { card_id }
                | SpecialAction::DiscardFromHand { card_id } => Some(*card_id),
                SpecialAction::TurnFaceUp { permanent_id, .. }
                | SpecialAction::ActivateManaAbility { permanent_id, .. } => Some(*permanent_id),
                SpecialAction::UnlockRoomDoor { room_id, .. } => Some(*room_id),
                SpecialAction::TurnConspiracyFaceUp { conspiracy_id } => Some(*conspiracy_id),
                SpecialAction::IgnoreAttachedRestriction { source_id, .. }
                | SpecialAction::IgnoreSourceEffect { source_id, .. }
                | SpecialAction::PerformIntrinsicAction { source_id, .. } => Some(*source_id),
                SpecialAction::RollPlanarDie
                | SpecialAction::PayDelayedTrigger { .. }
                | SpecialAction::PerformRepeatableManaPaymentAction { .. } => None,
            },
            LegalAction::PassPriority
            | LegalAction::KeepOpeningHand
            | LegalAction::TakeMulligan
            | LegalAction::ContinuePregame
            | LegalAction::BeginGame => None,
        }
    }

    fn manabrew_available_action(
        &self,
        index: usize,
        action: &LegalAction,
    ) -> Option<AvailableAction> {
        let id = format!("action-{index}");
        // The engine's own wording ("Cast X with flashback", "Activate X: ...").
        let described = Some(describe_action_with_face_up_cost(self.manabrew_game(), action, None))
            // An unnamed printed alternative cost carries an internal name.
            .map(|label| label.replace("(parsed alternative cost)", "(alternative cost)"))
            .filter(|label| !label.trim().is_empty());
        match action {
            LegalAction::PassPriority
            | LegalAction::KeepOpeningHand
            | LegalAction::TakeMulligan
            | LegalAction::ContinuePregame
            | LegalAction::BeginGame => None,
            LegalAction::CastExiledCardFaceDown { card_id, .. } => Some(AvailableAction {
                id,
                kind: AvailableActionKind::Cast {
                    card_id: object_id(self.manabrew_game(), *card_id),
                    mode: PlayCardMode::StaticAlternative,
                    label: "Cast exiled card face down".into(),
                    face_index: None,
                },
            }),
            LegalAction::OpenExiledCardForPlay { card_id, .. } => Some(AvailableAction {
                id,
                kind: AvailableActionKind::Cast {
                    card_id: object_id(self.manabrew_game(), *card_id),
                    mode: PlayCardMode::StaticAlternative,
                    label: "Play exiled card".into(),
                    face_index: None,
                },
            }),
            LegalAction::CastSpell {
                spell_id,
                casting_method,
                ..
            } => Some(AvailableAction {
                id,
                kind: AvailableActionKind::Cast {
                    card_id: object_id(self.manabrew_game(), *spell_id),
                    mode: protocol_play_card_mode(self.manabrew_game(), *spell_id, casting_method),
                    label: described.unwrap_or_else(|| {
                        format!(
                            "Cast {}",
                            self.manabrew_game().current_name(*spell_id).unwrap_or_default()
                        )
                    }),
                    face_index: None,
                },
            }),
            LegalAction::PlayLand { land_id } => Some(AvailableAction {
                id,
                kind: AvailableActionKind::Cast {
                    card_id: object_id(self.manabrew_game(), *land_id),
                    mode: PlayCardMode::Normal,
                    label: format!(
                        "Play {}",
                        self.manabrew_game().current_name(*land_id).unwrap_or_default()
                    ),
                    face_index: None,
                },
            }),
            // CR 712.12: the back face of a land//land modal DFC.
            LegalAction::PlayLandBackFace { land_id } => Some(AvailableAction {
                id,
                kind: AvailableActionKind::Cast {
                    card_id: object_id(self.manabrew_game(), *land_id),
                    mode: PlayCardMode::Normal,
                    label: format!(
                        "Play {}",
                        self.manabrew_game()
                            .object(*land_id)
                            .and_then(|object| {
                                ironsmith::decision::linked_back_face_land_definition(
                                    self.manabrew_game(), object,
                                )
                            })
                            .map(|def| def.card.name)
                            .unwrap_or_default()
                    ),
                    face_index: None,
                },
            }),
            LegalAction::ActivateAbility {
                source,
                ability_index,
            }
            | LegalAction::ActivateManaAbility {
                source,
                ability_index,
            } => Some(AvailableAction {
                id,
                kind: AvailableActionKind::ActivateAbility(ActivatableAbilityInfo {
                    card_id: object_id(self.manabrew_game(), *source),
                    ability_index: *ability_index,
                    // The client shows this on the source card, so the engine's
                    // "Activate <name>: " lead-in is redundant there.
                    description: described
                        .map(|label| {
                            let lead_in = format!(
                                "Activate {}: ",
                                self.manabrew_game().current_name(*source).unwrap_or_default()
                            );
                            label
                                .strip_prefix(&lead_in)
                                .map(str::to_string)
                                .unwrap_or(label)
                        })
                        .unwrap_or_else(|| format!("Activate ability {}", ability_index + 1)),
                    is_mana_ability: matches!(action, LegalAction::ActivateManaAbility { .. }),
                    level_up: None,
                    cost: None,
                    produced_mana: None,
                }),
            }),
            LegalAction::SpecialAction(
                ironsmith::special_actions::SpecialAction::UnlockRoomDoor { room_id, door },
            ) => Some(AvailableAction {
                id,
                kind: AvailableActionKind::Cast {
                    card_id: object_id(self.manabrew_game(), *room_id),
                    // CR 709.5: the printed left half is the front face. The
                    // engine names doors relative to the permanent's current
                    // half, which is the right one once it has been swapped.
                    mode: PlayCardMode::UnlockDoor {
                        door: match door {
                            ironsmith::special_actions::RoomDoor::Linked => RoomDoor::Right,
                            ironsmith::special_actions::RoomDoor::Current => RoomDoor::Left,
                        },
                    },
                    label: described.unwrap_or_else(|| "Unlock door".to_string()),
                    face_index: None,
                },
            }),
            _ => self
                .manabrew_action_card(action)
                .map(|card| AvailableAction {
                    id,
                    kind: AvailableActionKind::Cast {
                        card_id: object_id(self.manabrew_game(), card),
                        mode: PlayCardMode::StaticAlternative,
                        label: described.unwrap_or_else(|| "Special action".to_string()),
                        face_index: None,
                    },
                }),
        }
    }

    fn manabrew_known_card_names(&self) -> Vec<String> {
        let mut names = BTreeMap::new();
        for (name, lower) in Self::autocomplete_name_corpus() {
            names.entry(lower.clone()).or_insert_with(|| name.clone());
        }
        for (source_name, _) in self.external_parse_sources.values() {
            let name = source_name.trim();
            if !name.is_empty() {
                names
                    .entry(name.to_lowercase())
                    .or_insert_with(|| name.to_string());
            }
        }
        names.into_values().collect()
    }

    fn manabrew_text_name_prompt(
        description: String,
        source_card_id: Option<String>,
        names: Vec<String>,
    ) -> (PromptInput, ManabrewPromptBinding) {
        if names.len() <= MANABREW_TEXT_OPTIONS_PER_PROMPT {
            return (
                PromptInput::ChooseFromSelection(ChooseFromSelectionInput {
                    presentation: presentation(
                        "Choose a card name",
                        Some(description),
                        source_card_id,
                    ),
                    options: selection_options(names.clone()),
                    min_total: 1,
                    max_total: 1,
                }),
                ManabrewPromptBinding::TextNames { names },
            );
        }

        let chunk_size = names.len().div_ceil(MANABREW_TEXT_OPTIONS_PER_PROMPT);
        let groups = names
            .chunks(chunk_size)
            .map(|group| group.to_vec())
            .collect::<Vec<_>>();
        let options: Vec<String> = groups
            .iter()
            .map(|group| match (group.first(), group.last()) {
                (Some(first), Some(last)) if first != last => format!("{first} — {last}"),
                (Some(first), _) => first.clone(),
                _ => String::new(),
            })
            .collect();
        (
            PromptInput::ChooseFromSelection(ChooseFromSelectionInput {
                presentation: presentation(
                    "Choose a card-name range",
                    Some(description.clone()),
                    source_card_id,
                ),
                options: selection_options(options),
                min_total: 1,
                max_total: 1,
            }),
            ManabrewPromptBinding::TextNameGroups {
                description,
                groups,
            },
        )
    }

    fn manabrew_distribution_prompt(
        state: ManabrewDistributionState,
        source_card_id: Option<String>,
    ) -> Result<(PromptInput, ManabrewPromptBinding), ProtocolError> {
        let Some(target_name) = state.target_names.get(state.target_index) else {
            return Err(protocol_error(
                ProtocolErrorCode::InvalidShape,
                "Ironsmith distribution has no target to prompt for",
                None,
            ));
        };
        let targets_left = state.target_names.len() - state.target_index - 1;
        let legal_amounts = if targets_left == 0 {
            if state.remaining == 0 || state.remaining >= state.min_per_target {
                vec![state.remaining]
            } else {
                Vec::new()
            }
        } else if state.min_per_target <= 1 {
            Vec::new()
        } else {
            (0..=state.remaining)
                .filter(|amount| {
                    let remainder = state.remaining - amount;
                    (*amount == 0 || *amount >= state.min_per_target)
                        && (remainder == 0 || remainder >= state.min_per_target)
                })
                .collect()
        };
        if (targets_left == 0 || state.min_per_target > 1) && legal_amounts.is_empty() {
            return Err(protocol_error(
                ProtocolErrorCode::InvalidShape,
                format!(
                    "remaining distribution amount {} cannot satisfy the per-target minimum {}",
                    state.remaining, state.min_per_target
                ),
                None,
            ));
        }
        let description = Some(format!(
            "{} Remaining: {}. Target {} of {}.",
            state.description,
            state.remaining,
            state.target_index + 1,
            state.target_names.len()
        ));
        if targets_left > 0 && state.min_per_target > 1 {
            return Ok((
                PromptInput::ChooseFromSelection(ChooseFromSelectionInput {
                    presentation: presentation(
                        format!("Amount for {target_name}"),
                        description,
                        source_card_id,
                    ),
                    options: selection_options(legal_amounts.iter().map(u32::to_string)),
                    min_total: 1,
                    max_total: 1,
                }),
                ManabrewPromptBinding::DistributionOptions {
                    state,
                    amounts: legal_amounts,
                },
            ));
        }
        let (min, max) = if targets_left == 0 {
            (state.remaining, state.remaining)
        } else {
            (0, state.remaining)
        };
        Ok((
            PromptInput::ChooseNumber(ChooseNumberInput {
                presentation: presentation(
                    format!("Amount for {target_name}"),
                    description,
                    source_card_id,
                ),
                min: min.min(i32::MAX as u32) as i32,
                max: max.min(i32::MAX as u32) as i32,
            }),
            ManabrewPromptBinding::DistributionNumber { state },
        ))
    }

    fn manabrew_counter_chunk_bounds(state: &ManabrewCounterState) -> Result<(u32,u32),ProtocolError> {
        let invalid = |message: &str| protocol_error(ProtocolErrorCode::InvalidShape,message,None);
        let available = state.available.get(state.counter_index).copied()
            .ok_or_else(|| invalid("counter type index is out of range"))?;
        let allocated = state.allocations.get(state.counter_index).copied()
            .ok_or_else(|| invalid("counter allocation index is out of range"))?;
        let capacity = available.checked_sub(allocated)
            .ok_or_else(|| invalid("counter allocation exceeds available amount"))?;
        let max = u64::from(capacity).min(state.remaining);
        let future = state.available.iter().skip(state.counter_index + 1)
            .fold(0u64, |sum, value| sum.saturating_add(u64::from(*value)));
        let required = state.minimum_remaining.saturating_sub(future);
        if required > max { return Err(invalid("required counter allocation exceeds remaining capacity")); }
        let chunk = max.min(i32::MAX as u64) as u32;
        Ok((required.min(u64::from(chunk)) as u32,chunk))
    }

    fn manabrew_counter_prompt(
        state: ManabrewCounterState,
        source_card_id: Option<String>,
    ) -> Result<(PromptInput, ManabrewPromptBinding), ProtocolError> {
        let Some(counter_name) = state.counter_names.get(state.counter_index) else {
            return Err(protocol_error(
                ProtocolErrorCode::InvalidShape,
                "Ironsmith counter allocation has no counter type to prompt for",
                None,
            ));
        };
        let (min, max) = Self::manabrew_counter_chunk_bounds(&state)?;
        Ok((
            PromptInput::ChooseNumber(ChooseNumberInput {
                presentation: presentation(
                    format!("Remove {counter_name} counters"),
                    Some(format!(
                        "Choose how many {counter_name} counters to remove from {}. Up to {} total counter(s) remain.",
                        state.target_name, state.remaining
                    )),
                    source_card_id,
                ),
                min: min as i32,
                max: max as i32,
            }),
            ManabrewPromptBinding::CounterNumber { state },
        ))
    }

    fn manabrew_payment_actions(
        &self,
        context: &ironsmith::decisions::context::ManaPaymentContext,
    ) -> Result<(
        Vec<PaymentAction>, HashMap<String, ManaPaymentCommand>, bool,
    ), ironsmith::effects::ExecutionError> {
        let mut payment_actions = Vec::new();
        let mut commands = HashMap::new();
        let mut next_action_id = 0usize;
        let mut add_action =
            |kind: PaymentActionKind,
             preferences: ironsmith::mana_payment::ManaPaymentPreferences| {
                let id = format!("payment-action-{next_action_id}");
                next_action_id += 1;
                commands.insert(id.clone(), manabrew_replan_command(preferences));
                payment_actions.push(PaymentAction { id, kind });
            };

        // Each probe is one planner run; a very wide board keeps its tail unprobed.
        const MAX_ACTIVATION_PROBES: usize = 24;
        const ACTIVATION_PROBE_BUDGET: usize = 64;
        let mut activation_probes = 0usize;
        let activation_inventory = ironsmith::mana_payment::mana_payment_activation_inventory_checked(
            self.manabrew_game(),
            &context.request,
        )?;
        for option in &activation_inventory {
            if !self.manabrew_can_defer_mana_activation(option.source, option.ability_index) {
                continue;
            }
            let selected_index = context
                .request
                .preferences
                .required_activations
                .iter()
                .position(|selected| selected_activation_matches(selected, option));
            let source_has_blocking_activation = context
                .request
                .preferences
                .required_activations
                .iter()
                .filter(|selected| selected.source == option.source)
                .any(|selected| {
                    activation_inventory
                        .iter()
                        .find(|candidate| selected_activation_matches(selected, candidate))
                        .is_none_or(|candidate| !candidate.repeatable)
                });
            let source_has_selected_alternative = context
                .request
                .preferences
                .required_alternatives
                .iter()
                .any(|selected| {
                    selected.source == option.source
                        && selected.kind != ironsmith::mana_payment::ManaPaymentSourceKind::PutMinusOneCounter
                });
            let has_specific_color_action = activation_inventory.iter().any(|candidate| {
                candidate.source == option.source
                    && candidate.ability_index == option.ability_index
                    && candidate.color_restriction.is_some()
            });
            if source_has_selected_alternative
                || (source_has_blocking_activation && selected_index.is_none())
                || (option.color_restriction.is_none() && has_specific_color_action)
            {
                continue;
            }

            let card_id = object_id(self.manabrew_game(), option.source);
            let source_name = self
                .manabrew_game()
                .object(option.source)
                .map(|object| object.name.to_string())
                .unwrap_or_else(|| format!("Mana source #{}", option.source.0));
            if let Some(selected_index) = selected_index {
                let mut preferences = context.request.preferences.clone();
                preferences.required_activations.remove(selected_index);
                add_action(
                    PaymentActionKind::UndoMana {
                        card_id: card_id.clone(),
                    },
                    preferences,
                );
            }
            if selected_index.is_none() || option.repeatable {
                let mut preferences = context.request.preferences.clone();
                preferences
                    .excluded_sources
                    .retain(|source| *source != option.source);
                preferences.required_activations.push(
                    ironsmith::mana_payment::RequiredManaActivation {
                        source: option.source,
                        ability_index: option.ability_index,
                        color_restriction: option.color_restriction.clone(),
                    },
                );
                // Selecting an activation requires the plan to use it. One that
                // leaves the cost unpayable (the blue ability of a dual land for
                // a red cost) is a dead end, and a client that taps a land by
                // clicking it takes the first action it is offered for that card.
                if context.plan.payable && activation_probes < MAX_ACTIVATION_PROBES {
                    activation_probes += 1;
                    let mut probe = context.request.clone();
                    probe.preferences = preferences.clone();
                    probe.preferences.normalize();
                    // One legal payment answers the question, and the search
                    // for it is budgeted: proving that none exists is the
                    // planner's exponential case, and this runs once per
                    // source on the host's main thread. A probe that runs out
                    // is treated like a dead end.
                    match ironsmith::mana_payment::ManaPaymentAnalysis::new(
                        self.manabrew_game(),
                        probe,
                    )
                    .step(ACTIVATION_PROBE_BUDGET)
                    {
                        Some(Ok(plan)) if plan.payable => {}
                        Some(Err(
                            ironsmith::mana_payment::ManaPaymentFailure::EffectExecutionFailed(
                                error,
                            ),
                        )) => return Err(error),
                        _ => continue,
                    }
                }
                let produced_mana = manabrew_mana_pool_entries(&option.expected_mana);
                add_action(
                    PaymentActionKind::ActivateManaAbility(ActivatableAbilityInfo {
                        card_id,
                        ability_index: option.ability_index,
                        description: source_name,
                        is_mana_ability: true,
                        level_up: None,
                        cost: None,
                        produced_mana: (!produced_mana.is_empty()).then_some(produced_mana),
                    }),
                    preferences,
                );
            }
        }

        for source_option in
            ironsmith::mana_payment::mana_payment_source_inventory(self.manabrew_game(), &context.request)
        {
            let source_has_blocking_activation = context
                .request
                .preferences
                .required_activations
                .iter()
                .filter(|selected| selected.source == source_option.source)
                .any(|selected| {
                    activation_inventory
                        .iter()
                        .find(|candidate| selected_activation_matches(selected, candidate))
                        .is_none_or(|candidate| !candidate.repeatable)
                });

            for kind in source_option.kinds.into_iter().filter(|kind| {
                matches!(
                    kind,
                    ironsmith::mana_payment::ManaPaymentSourceKind::Convoke
                        | ironsmith::mana_payment::ManaPaymentSourceKind::Improvise
                        | ironsmith::mana_payment::ManaPaymentSourceKind::Waterbend
                        | ironsmith::mana_payment::ManaPaymentSourceKind::Delve
                        | ironsmith::mana_payment::ManaPaymentSourceKind::PutMinusOneCounter
                )
            }) {
                if source_has_blocking_activation && kind != ironsmith::mana_payment::ManaPaymentSourceKind::PutMinusOneCounter { continue; }
                let selected = context
                    .request
                    .preferences
                    .required_alternatives
                    .iter()
                    .find(|selected| {
                        selected_alternative_matches(selected, source_option.source, kind)
                    });
                let source_has_selected_alternative = context
                    .request
                    .preferences
                    .required_alternatives
                    .iter()
                    .any(|selected| selected.source == source_option.source
                        && selected.kind != ironsmith::mana_payment::ManaPaymentSourceKind::PutMinusOneCounter
                        && kind != ironsmith::mana_payment::ManaPaymentSourceKind::PutMinusOneCounter);
                if source_has_selected_alternative && selected.is_none() {
                    continue;
                }

                let card_id = object_id(self.manabrew_game(), source_option.source);
                let mut preferences = context.request.preferences.clone();
                if let Some(selected) = selected {
                    preferences
                        .required_alternatives
                        .retain(|candidate| candidate != selected);
                    let resource = match kind {
                        ironsmith::mana_payment::ManaPaymentSourceKind::Convoke => {
                            PaymentResourceKind::Convoke
                        }
                        ironsmith::mana_payment::ManaPaymentSourceKind::Improvise => {
                            PaymentResourceKind::Improvise
                        }
                        ironsmith::mana_payment::ManaPaymentSourceKind::Waterbend => {
                            PaymentResourceKind::Waterbend
                        }
                        // Upstream protocol 6 has no resource for this custom-mechanics
                        // payment; it stays available through automatic payment.
                        ironsmith::mana_payment::ManaPaymentSourceKind::PutMinusOneCounter => continue,
                        ironsmith::mana_payment::ManaPaymentSourceKind::Delve => {
                            PaymentResourceKind::Delve
                        }
                        ironsmith::mana_payment::ManaPaymentSourceKind::ManaAbility => continue,
                    };
                    add_action(
                        PaymentActionKind::ReleaseResource { card_id, resource },
                        preferences,
                    );
                } else {
                    preferences
                        .excluded_sources
                        .retain(|source| *source != source_option.source);
                    preferences.required_alternatives.push(
                        ironsmith::mana_payment::RequiredAlternativePayment {
                            source: source_option.source,
                            kind,
                        },
                    );
                    let resource = match kind {
                        ironsmith::mana_payment::ManaPaymentSourceKind::Convoke => {
                            PaymentResourceKind::Convoke
                        }
                        ironsmith::mana_payment::ManaPaymentSourceKind::Improvise => {
                            PaymentResourceKind::Improvise
                        }
                        ironsmith::mana_payment::ManaPaymentSourceKind::Waterbend => {
                            PaymentResourceKind::Waterbend
                        }
                        // Upstream protocol 6 has no resource for this custom-mechanics
                        // payment; it stays available through automatic payment.
                        ironsmith::mana_payment::ManaPaymentSourceKind::PutMinusOneCounter => continue,
                        ironsmith::mana_payment::ManaPaymentSourceKind::Delve => {
                            PaymentResourceKind::Delve
                        }
                        ironsmith::mana_payment::ManaPaymentSourceKind::ManaAbility => continue,
                    };
                    add_action(
                        PaymentActionKind::UseResource { card_id, resource },
                        preferences,
                    );
                }
            }
        }

        for (pip, amount) in
            ironsmith::mana_payment::mana_payment_life_options(self.manabrew_game(), &context.request)
        {
            let mut life_request = context.request.clone();
            life_request.preferences.required_life_pips.push(pip);
            let life_plans = match ironsmith::mana_payment::plan_mana_payment(self.manabrew_game(), &life_request) {
                Ok(plans) => plans,
                Err(ironsmith::mana_payment::ManaPaymentFailure::EffectExecutionFailed(error)) => return Err(error),
                Err(_) => continue,
            };
            if let Some(life_plan) = life_plans.first()
                && life_plan.life_to_pay > 0
            {
                add_action(
                    PaymentActionKind::PayLife { amount },
                    life_request.preferences,
                );
            }
        }

        // Complex activations need the live continuation machinery so costs and
        // choices are paid interactively, rather than chosen by a simulation.
        drop(add_action);
        for (source, ability_index) in
            ironsmith::mana_payment::manual_mana_abilities_checked(self.manabrew_game(), &context.request)?
        {
            if self.manabrew_can_defer_mana_activation(source, ability_index) {
                continue;
            }
            let id = format!("payment-action-{next_action_id}");
            next_action_id += 1;
            commands.insert(
                id.clone(),
                ManaPaymentCommand::Activate {
                    source_id: source.0.to_string(),
                    ability_index,
                },
            );
            payment_actions.push(PaymentAction {
                id,
                kind: PaymentActionKind::ActivateManaAbility(ActivatableAbilityInfo {
                    card_id: object_id(self.manabrew_game(), source),
                    ability_index,
                    description: current_ability_action_text(self.manabrew_game(), source, ability_index)
                        .unwrap_or_else(|| "Activate mana ability".into()),
                    is_mana_ability: true,
                    level_up: None,
                    cost: None,
                    produced_mana: None,
                }),
            });
        }

        Ok((payment_actions, commands, manabrew_plan_is_fully_selected(context)))
    }

    fn manabrew_can_defer_mana_activation(&self, source: ObjectId, index: usize) -> bool {
        if ironsmith::game_loop::mana_ability_is_undo_safe(self.manabrew_game(), source, index) {
            return true;
        }
        self.manabrew_game()
            .current_ability(source, index)
            .is_some_and(|ability| {
                matches!(&ability.kind, ironsmith::ability::AbilityKind::Activated(a)
                if a.mana_cost.costs().is_empty() && a.effects.is_empty())
            })
    }

    fn manabrew_number_prompt(minimum_remaining:u32, maximum_remaining:u32, accumulated:u32, description:String, source:Option<String>) -> Result<(PromptInput, ManabrewPromptBinding), ProtocolError> {
        if minimum_remaining > maximum_remaining {
            return Err(protocol_error(ProtocolErrorCode::InvalidShape, "number minimum exceeds its maximum", None));
        }
        let title = if accumulated == 0 { "Choose a number" } else { "Choose an additional amount" };
        let detail = if accumulated == 0 { description.clone() } else { format!("{description}. Selected so far: {accumulated}. Choose an additional amount; choose zero to finish when allowed.") };
        Ok((PromptInput::ChooseNumber(ChooseNumberInput {
            presentation: presentation(title, Some(detail), source),
            min: minimum_remaining.min(i32::MAX as u32) as i32,
            max: maximum_remaining.min(i32::MAX as u32) as i32,
        }), ManabrewPromptBinding::Number { minimum_remaining, maximum_remaining, accumulated, description }))
    }

    fn manabrew_combat_damage_prompt(
        &self,
        combat: &ironsmith::game_loop::CombatDamageAssignmentPrompt,
    ) -> Result<(PromptInput, ManabrewPromptBinding), ProtocolError> {
        let game = self.manabrew_game();
        let targets = combat.division.targets();
        let total_damage = i32::try_from(combat.total).map_err(|_| {
            protocol_error(
                ProtocolErrorCode::InvalidShape,
                "combat damage exceeds the Manabrew assignment range",
                None,
            )
        })?;
        let defender = targets
            .iter()
            .find(|target| matches!(target, Target::Player(_)))
            .copied()
            .or(combat.division.excess_target);
        let blocker_ids = targets
            .iter()
            .filter_map(|target| match target {
                Target::Object(id) if Some(*target) != defender => Some(object_id(game, *id)),
                _ => None,
            })
            .collect();
        Ok((
            PromptInput::ChooseCombatDamageAssignment(ChooseCombatDamageAssignmentInput {
                attacker_id: object_id(game, combat.source),
                blocker_ids,
                defender_id: defender.map(|target| protocol_target(game, target).id),
                total_damage,
                attacker_has_deathtouch: combat.division.deathtouch,
            }),
            ManabrewPromptBinding::CombatDamage {
                targets: targets
                    .into_iter()
                    .map(|target| protocol_target(game, target).id)
                    .collect(),
            },
        ))
    }

    fn manabrew_target_prefix_valid(state: &ManabrewTargetState, chosen: &[Target]) -> bool {
        let index = state.selections.len();
        let mut requirements = state.requirements[..=index].to_vec();
        for (requirement, selection) in requirements.iter_mut().zip(
            state
                .selections
                .iter()
                .map(Vec::as_slice)
                .chain(std::iter::once(chosen)),
        ) {
            requirement.min_targets = selection.len();
            requirement.max_targets = Some(selection.len());
            requirement.additional_target_budget = 0;
        }
        let selected = state
            .selections
            .iter()
            .flatten()
            .copied()
            .chain(chosen.iter().copied())
            .collect::<Vec<_>>();
        validate_flat_target_assignment(&requirements, &selected)
    }

    fn manabrew_targets_prompt(
        &self,
        state: ManabrewTargetState,
        source: Option<String>,
    ) -> Result<(PromptInput, ManabrewPromptBinding), ProtocolError> {
        let requirement = state.requirements.get(state.selections.len());
        let mut targets = HashMap::new();
        let mut candidates = Vec::new();
        if let Some(requirement) = requirement {
            for target in &requirement.legal_targets {
                if requirement.max_targets == Some(1)
                    && !Self::manabrew_target_prefix_valid(&state, &[*target])
                {
                    continue;
                }
                let reference = protocol_target(self.manabrew_game(), *target);
                let key = format!("{:?}:{}", reference.kind, reference.id);
                if targets.insert(key, *target).is_none() {
                    candidates.push(reference);
                }
            }
        }
        let min = requirement.map_or(0, |requirement| requirement.min_targets);
        let max = requirement.map_or(0, |requirement| {
            requirement
                .max_targets
                .map(|max| {
                    max.saturating_sub(requirement.additional_target_budget)
                        .saturating_add(
                            state
                                .additional_remaining
                                .min(requirement.additional_target_budget),
                        )
                })
                .unwrap_or(candidates.len())
                .min(candidates.len())
        });
        let description = requirement.map_or_else(
            || state.context.clone(),
            |requirement| format!("{}: {}", state.context, requirement.description),
        );
        Ok((
            PromptInput::ChooseBoardTargets(ChooseBoardTargetsInput {
                presentation: presentation("Choose targets", Some(description), source),
                candidates,
                hostile: false,
                intent: manabrew_protocol::game::TargetingIntent::Friendly,
                min_targets: i32::try_from(min).map_err(|_| {
                    protocol_error(
                        ProtocolErrorCode::InvalidShape,
                        "target minimum exceeds protocol range",
                        None,
                    )
                })?,
                max_targets: i32::try_from(max).map_err(|_| {
                    protocol_error(
                        ProtocolErrorCode::InvalidShape,
                        "target maximum exceeds protocol range",
                        None,
                    )
                })?,
                chosen_targets: 0,
                cancellable: state.cancellable,
            }),
            ManabrewPromptBinding::Targets { targets, state },
        ))
    }

    fn build_manabrew_prompt(
        &self,
        context: &DecisionContext,
    ) -> Result<(PromptInput, ManabrewPromptBinding), ProtocolError> {
        if matches!(context, DecisionContext::Distribute(_))
            && self.runner_pending_decision
            && let Some(runner) = self.runner.as_ref()
            && !runner.has_pending_replay_choice()
            && let Some(combat) = runner.current_combat_damage_assignment_prompt(self.manabrew_game())
        {
            return self.manabrew_combat_damage_prompt(&combat);
        }
        let source = self.manabrew_source_card_id(context);
        let unsupported = |name: &str| {
            protocol_error(
                ProtocolErrorCode::InvalidShape,
                format!("Ironsmith decision {name} has no lossless Manabrew v1 prompt mapping"),
                None,
            )
        };
        match context {
            DecisionContext::ManaPayment(ctx) => {
                let source_name = self
                    .manabrew_game()
                    .object(ctx.source)
                    .map(|object| object.name.to_string())
                    .unwrap_or_else(|| ctx.subject.clone());
                let source_id = object_id(self.manabrew_game(), ctx.source);
                let (actions, action_commands, can_confirm_manually) =
                    self.manabrew_payment_actions(ctx).map_err(|error| protocol_error(
                        ProtocolErrorCode::InvalidShape, format!("Payment analysis incomplete: {error}"), None))?;
                let mana_cost = manabrew_remaining_mana_cost(ctx);
                Ok((
                    PromptInput::PayManaCost(PayManaCostInput {
                        presentation: presentation(
                            "Pay mana",
                            Some(if mana_cost.is_empty() {
                                format!("Confirm payment for {source_name}.")
                            } else {
                                format!("Pay {mana_cost} for {source_name}.")
                            }),
                            Some(source_id.clone()),
                        ),
                        card_id: source_id,
                        card_name: source_name,
                        mana_cost,
                        can_confirm_from_pool: can_confirm_manually,
                        actions,
                    }),
                    ManabrewPromptBinding::AuthoritativePayment {
                        plan_id: ctx.plan.id.to_string(),
                        request_hash: ctx.plan.request_hash.to_string(),
                        actions: action_commands,
                        can_confirm_manually,
                        payable: ctx.plan.payable,
                    },
                ))
            }
            DecisionContext::Priority(ctx) => {
                let keep_index = ctx
                    .actions
                    .iter()
                    .position(|action| matches!(action, LegalAction::KeepOpeningHand));
                let mulligan_index = ctx
                    .actions
                    .iter()
                    .position(|action| matches!(action, LegalAction::TakeMulligan));
                if let (Some(keep_index), Some(mulligan_index)) = (keep_index, mulligan_index) {
                    let hand_card_ids = self
                        .manabrew_game()
                        .player(ctx.player)
                        .map(|player| {
                            player
                                .hand
                                .iter()
                                .map(|id| object_id(self.manabrew_game(), *id))
                                .collect()
                        })
                        .unwrap_or_default();
                    return Ok((
                        PromptInput::Mulligan(MulliganInput {
                            hand_card_ids,
                            mulligan_count: self
                                .pregame
                                .as_ref()
                                .and_then(|pregame| pregame.mulligans_taken.get(&ctx.player))
                                .copied()
                                .unwrap_or(0) as _,
                        }),
                        ManabrewPromptBinding::Mulligan {
                            keep_index,
                            mulligan_index,
                        },
                    ));
                }
                let pass_index = ctx
                    .actions
                    .iter()
                    .position(|action| {
                        matches!(
                            action,
                            LegalAction::PassPriority
                                | LegalAction::ContinuePregame
                                | LegalAction::BeginGame
                        )
                    })
                    .ok_or_else(|| unsupported("priority without a pass action"))?;
                let mut actions = HashMap::new();
                let mut opaque_exile = HashMap::new();
                let available = ctx
                    .actions
                    .iter()
                    .enumerate()
                    .filter_map(|(index, action)| {
                        let available = self.manabrew_available_action(index, action)?;
                        actions.insert(available.id.clone(), index);
                        if let LegalAction::OpenExiledCardForPlay { card_id, .. }
                            | LegalAction::CastExiledCardFaceDown { card_id, .. } = action {
                            opaque_exile.insert(index, ManabrewOpaqueExileBinding {
                                action_ref: priority_action_ref(action),
                                hidden_identity: self.manabrew_opaque_exile_identity(*card_id),
                            });
                        }
                        Some(available)
                    })
                    .collect();
                Ok((
                    PromptInput::ChooseAction(ChooseActionInput { actions: available }),
                    ManabrewPromptBinding::Priority {
                        actions,
                        opaque_exile,
                        pass_index,
                    },
                ))
            }
            DecisionContext::Boolean(ctx) => Ok((
                PromptInput::ChooseBoolean(ChooseBooleanInput {
                    presentation: presentation(
                        ctx.source_name
                            .clone()
                            .unwrap_or_else(|| "Choose".to_string()),
                        Some(ctx.description.clone()),
                        source,
                    ),
                    confirm_label: "Yes".to_string(),
                    deny_label: "No".to_string(),
                }),
                ManabrewPromptBinding::Boolean,
            )),
            DecisionContext::Number(ctx) => Self::manabrew_number_prompt(ctx.min, ctx.max, 0, ctx.description.clone(), source),
            DecisionContext::SelectOptions(ctx) => {
                let legal: Vec<_> = ctx.options.iter().filter(|option| option.legal).collect();
                Ok((
                    PromptInput::ChooseFromSelection(ChooseFromSelectionInput {
                        presentation: presentation("Choose", Some(ctx.description.clone()), source),
                        options: legal
                            .iter()
                            .map(|option| SelectionOption {
                                label: option.description.clone(),
                                weight: option.point_cost.max(1) as usize,
                                can_repeat: option.repeatable,
                            })
                            .collect(),
                        min_total: ctx.min,
                        max_total: ctx.max,
                    }),
                    ManabrewPromptBinding::Options {
                        indices: legal.iter().map(|option| option.index).collect(),
                    },
                ))
            }
            DecisionContext::Modes(ctx) => {
                let legal: Vec<_> = ctx.spec.modes.iter().filter(|mode| mode.legal).collect();
                Ok((
                    PromptInput::ChooseFromSelection(ChooseFromSelectionInput {
                        presentation: presentation(
                            "Choose modes",
                            Some(ctx.spell_name.clone()),
                            source,
                        ),
                        options: selection_options(
                            legal.iter().map(|mode| mode.description.clone()),
                        ),
                        min_total: ctx.spec.min_modes,
                        max_total: ctx.spec.max_modes,
                    }),
                    ManabrewPromptBinding::Options {
                        indices: legal.iter().map(|mode| mode.index).collect(),
                    },
                ))
            }
            DecisionContext::HybridChoice(ctx) => Ok((
                PromptInput::ChooseFromSelection(ChooseFromSelectionInput {
                    presentation: presentation(
                        format!("Choose payment for pip {}", ctx.pip_number),
                        Some(ctx.spell_name.clone()),
                        source,
                    ),
                    options: selection_options(
                        ctx.options.iter().map(|option| option.label.clone()),
                    ),
                    min_total: 1,
                    max_total: 1,
                }),
                ManabrewPromptBinding::Options {
                    indices: ctx.options.iter().map(|option| option.index).collect(),
                },
            )),
            DecisionContext::SelectObjects(ctx) => {
                let legal: Vec<_> = ctx
                    .candidates
                    .iter()
                    .filter(|candidate| candidate.legal)
                    .collect();
                let objects = legal
                    .iter()
                    .map(|candidate| (object_id(self.manabrew_game(), candidate.id), candidate.id))
                    .collect();
                // CR 103.5: the cards a mulligan puts on the bottom have their
                // own protocol prompt.
                if matches!(
                    self.pregame.as_ref().map(|pregame| &pregame.stage),
                    Some(PregameStage::BottomCards {
                        pending_order: None,
                        ..
                    })
                ) {
                    return Ok((
                        PromptInput::MulliganPutBack(MulliganPutBackInput {
                            hand_card_ids: legal
                                .iter()
                                .map(|candidate| object_id(self.manabrew_game(), candidate.id))
                                .collect(),
                            cards: legal
                                .iter()
                                .filter_map(|candidate| {
                                    protocol_card(self.manabrew_game(), candidate.id)
                                })
                                .collect(),
                            count: ctx.min,
                        }),
                        ManabrewPromptBinding::Objects { objects },
                    ));
                }
                Ok((
                    PromptInput::ChooseCards(ChooseCardsInput {
                        presentation: presentation(
                            "Choose cards",
                            Some(ctx.description.clone()),
                            source,
                        ),
                        cards: legal
                            .iter()
                            .filter_map(|candidate| protocol_card(self.manabrew_game(), candidate.id))
                            .collect(),
                        min: if ctx.allow_partial_completion {
                            0
                        } else {
                            ctx.min
                        },
                        max: ctx.max.unwrap_or(legal.len()),
                    }),
                    ManabrewPromptBinding::Objects { objects },
                ))
            }
            DecisionContext::Order(ctx) => {
                // Trigger ordering hands out placeholder ids for the pending
                // abilities and names their sources separately, so an item is
                // not always a game object. Every item still needs a wire id
                // and a card: an empty or shortened list can never be answered.
                let mut indices = HashMap::new();
                let mut items = Vec::with_capacity(ctx.items.len());
                for (index, (id, label)) in ctx.items.iter().enumerate() {
                    let item_source = ctx.item_sources.get(index).copied().flatten();
                    let real = item_source.is_none() && self.manabrew_game().object(*id).is_some();
                    let mut item_id = if real {
                        object_id(self.manabrew_game(), *id)
                    } else {
                        format!("order-{index}")
                    };
                    if indices.contains_key(&item_id) {
                        item_id = format!("order-{index}");
                    }
                    let card = item_source
                        .or(real.then_some(*id))
                        .and_then(|object| protocol_card(self.manabrew_game(), object))
                        .unwrap_or_else(|| CardDto {
                            id: item_id.clone(),
                            identity: CardIdentity {
                                name: label.clone(),
                                ..CardIdentity::default()
                            },
                            text: label.clone(),
                            ..CardDto::default()
                        });
                    indices.insert(item_id.clone(), index);
                    items.push(ReorderItem {
                        id: item_id,
                        card,
                        oracle: (!real && !label.trim().is_empty()).then(|| label.clone()),
                    });
                }
                Ok((
                    PromptInput::Reorder(ReorderInput {
                        presentation: presentation(
                            "Choose order",
                            Some(ctx.description.clone()),
                            source,
                        ),
                        items,
                    }),
                    ManabrewPromptBinding::Reorder { indices },
                ))
            }
            DecisionContext::Colors(ctx) => {
                let colors = ctx
                    .available_colors
                    .clone()
                    .unwrap_or_else(|| Color::ALL.to_vec());
                Ok((
                    PromptInput::ChooseColor(ChooseColorInput {
                        presentation: presentation(
                            "Choose colors",
                            Some(ctx.description.clone()),
                            source,
                        ),
                        valid_colors: colors
                            .iter()
                            .map(|color| color_code(*color).to_string())
                            .collect(),
                        amount: ctx.count,
                        repeat_allowed: !ctx.distinct_colors,
                    }),
                    ManabrewPromptBinding::Colors {
                        indices: colors
                            .iter()
                            .enumerate()
                            .map(|(index, color)| (color_code(*color).to_string(), index))
                            .collect(),
                    },
                ))
            }
            DecisionContext::Partition(ctx) => {
                let secondary_zone = if ctx.secondary_label.to_ascii_lowercase().contains("grave") {
                    ScryDestination::Graveyard
                } else if ctx.secondary_label.to_ascii_lowercase().contains("exile") {
                    ScryDestination::Exile
                } else if ctx.secondary_label.to_ascii_lowercase().contains("hand") {
                    ScryDestination::Hand
                } else {
                    ScryDestination::LibraryBottom
                };
                Ok((
                    PromptInput::Scry(ScryInput {
                        presentation: presentation(
                            "Arrange cards",
                            Some(ctx.description.clone()),
                            source,
                        ),
                        cards: ctx
                            .cards
                            .iter()
                            .filter_map(|(id, _)| protocol_card(self.manabrew_game(), *id))
                            .collect(),
                        zones: vec![ScryDestination::LibraryTop, secondary_zone],
                    }),
                    ManabrewPromptBinding::Partition {
                        objects: ctx
                            .cards
                            .iter()
                            .map(|(id, _)| (object_id(self.manabrew_game(), *id), *id))
                            .collect(),
                        secondary_zone_index: 1,
                    },
                ))
            }
            DecisionContext::Attackers(ctx) => {
                let mut attackers = HashMap::new();
                let mut targets = HashMap::new();
                let mut target_dtos = HashMap::new();
                let options = ctx
                    .attacker_options
                    .iter()
                    .map(|option| {
                        let attacker_id = object_id(self.manabrew_game(), option.creature);
                        attackers.insert(attacker_id.clone(), option.creature);
                        let valid_target_ids = option
                            .valid_targets
                            .iter()
                            .filter_map(|target| {
                                let (id, label, kind) = match target {
                                    AttackTarget::Player(player) => (
                                        player_id(*player),
                                        self.manabrew_game()
                                            .player(*player)
                                            .map(|player| player.name.clone())
                                            .unwrap_or_else(|| player_id(*player)),
                                        AttackTargetKind::Player,
                                    ),
                                    AttackTarget::Planeswalker(object) => (
                                        object_id(self.manabrew_game(), *object),
                                        self.manabrew_game().current_name(*object).unwrap_or_default(),
                                        AttackTargetKind::Planeswalker,
                                    ),
                                    AttackTarget::Battle(object) => (
                                        object_id(self.manabrew_game(), *object),
                                        self.manabrew_game().current_name(*object).unwrap_or_default(),
                                        AttackTargetKind::Battle,
                                    ),
                                    // Never a declarable attack target.
                                    AttackTarget::Nothing { .. } => return None,
                                };
                                targets.insert(id.clone(), target.clone());
                                target_dtos.entry(id.clone()).or_insert(AttackTargetDto {
                                    id: id.clone(),
                                    label,
                                    kind,
                                });
                                Some(id)
                            })
                            .collect();
                        AttackerOptionDto {
                            attacker_id,
                            valid_target_ids,
                            must_attack: option.must_attack,
                        }
                    })
                    .collect();
                Ok((
                    PromptInput::ChooseAttackers(ChooseAttackersInput {
                        attackers: options,
                        attack_targets: target_dtos.into_values().collect(),
                    }),
                    ManabrewPromptBinding::Attackers { attackers, targets },
                ))
            }
            DecisionContext::Blockers(ctx) => {
                let mut objects = HashMap::new();
                let mut available = Vec::new();
                let attackers = ctx
                    .blocker_options
                    .iter()
                    .map(|option| {
                        let attacker_id = object_id(self.manabrew_game(), option.attacker);
                        objects.insert(attacker_id.clone(), option.attacker);
                        let valid_blocker_ids = option
                            .valid_blockers
                            .iter()
                            .map(|(blocker, _)| {
                                let id = object_id(self.manabrew_game(), *blocker);
                                objects.insert(id.clone(), *blocker);
                                if !available.contains(&id) {
                                    available.push(id.clone());
                                }
                                id
                            })
                            .collect();
                        BlockableAttackerDto {
                            attacker_id,
                            valid_blocker_ids,
                            min_blockers: option.min_blockers.min(u32::MAX as usize) as u32,
                            max_blockers: None,
                            // `min_blockers` is how many creatures a block of this
                            // attacker needs (menace), not a requirement to block it;
                            // the engine reports no "must be blocked" requirement here.
                            must_be_blocked: false,
                        }
                    })
                    .collect();
                Ok((
                    PromptInput::ChooseBlockers(ChooseBlockersInput {
                        attackers,
                        available_blocker_ids: available,
                        error: None,
                    }),
                    ManabrewPromptBinding::Blockers { objects },
                ))
            }
            DecisionContext::Targets(ctx) => self.manabrew_targets_prompt(
                ManabrewTargetState {
                    requirements: ctx.requirements.clone(),
                    selections: Vec::new(),
                    additional_remaining: ctx
                        .requirements
                        .iter()
                        .map(|requirement| requirement.additional_target_budget)
                        .max()
                        .unwrap_or(0),
                    context: ctx.context.clone(),
                    cancellable: self.is_cancelable(),
                },
                source,
            ),
            DecisionContext::TextInput(ctx) => {
                if !ctx.require_known_value {
                    return Err(unsupported("free-form text input"));
                }
                let names = self.manabrew_known_card_names();
                if names.is_empty() {
                    return Err(unsupported("known card-name input with an empty registry"));
                }
                Ok(Self::manabrew_text_name_prompt(
                    ctx.description.clone(),
                    source,
                    names,
                ))
            }
            DecisionContext::Distribute(ctx) if ctx.targets.is_empty() => Ok((
                PromptInput::ChooseFromSelection(ChooseFromSelectionInput {
                    presentation: presentation(
                        "Complete distribution",
                        Some(ctx.description.clone()),
                        source,
                    ),
                    options: Vec::new(),
                    min_total: 0,
                    max_total: 0,
                }),
                ManabrewPromptBinding::Options {
                    indices: Vec::new(),
                },
            )),
            DecisionContext::Distribute(ctx) => Self::manabrew_distribution_prompt(
                ManabrewDistributionState {
                    description: ctx.description.clone(),
                    target_names: ctx
                        .targets
                        .iter()
                        .map(|target| target.name.clone())
                        .collect(),
                    target_index: 0,
                    remaining: ctx.total,
                    min_per_target: ctx.min_per_target,
                    allocations: Vec::with_capacity(ctx.targets.len()),
                },
                source,
            ),
            DecisionContext::Counters(ctx) if ctx.available_counters.is_empty() => Ok((
                PromptInput::ChooseFromSelection(ChooseFromSelectionInput {
                    presentation: presentation(
                        "Complete counter removal",
                        Some(format!(
                            "There are no counters to remove from {}.",
                            ctx.target_name
                        )),
                        source,
                    ),
                    options: Vec::new(),
                    min_total: 0,
                    max_total: 0,
                }),
                ManabrewPromptBinding::Options {
                    indices: Vec::new(),
                },
            )),
            DecisionContext::Counters(ctx) => Self::manabrew_counter_prompt(
                ManabrewCounterState {
                    target_name: ctx.target_name.clone(),
                    counter_names: ctx
                        .available_counters
                        .iter()
                        .map(|(counter, _)| counter_name(*counter))
                        .collect(),
                    available: ctx
                        .available_counters
                        .iter()
                        .map(|(_, available)| *available)
                        .collect(),
                    counter_index: 0,
                    remaining: ctx.max_total,
                    minimum_remaining: ctx.min_total,
                    allocations: vec![0; ctx.available_counters.len()],
                },
                source,
            ),
            DecisionContext::Proliferate(ctx) => {
                let options = ctx
                    .eligible_permanents
                    .iter()
                    .map(|(_, name)| format!("Permanent: {name}"))
                    .chain(
                        ctx.eligible_players
                            .iter()
                            .map(|(_, name)| format!("Player: {name}")),
                    )
                    .collect::<Vec<_>>();
                let indices = (0..options.len()).collect();
                Ok((
                    PromptInput::ChooseFromSelection(ChooseFromSelectionInput {
                        presentation: presentation(
                            "Choose permanents and players to proliferate",
                            Some(
                                "Choose any number. Each chosen permanent or player gets one more counter of each kind already there."
                                    .to_string(),
                            ),
                            source,
                        ),
                        min_total: 0,
                        max_total: options.len(),
                        options: selection_options(options),
                    }),
                    ManabrewPromptBinding::Options { indices },
                ))
            }
        }
    }

    fn ensure_manabrew_prompt(&mut self) -> Result<Option<AgentPrompt>, ProtocolError> {
        self.recompute_stale_priority_decision().map_err(|error| {
            protocol_error(
                ProtocolErrorCode::InvalidShape,
                error
                    .as_string()
                    .unwrap_or_else(|| "failed to refresh priority".to_string()),
                None,
            )
        })?;
        // A decision with exactly one possible answer is not put to the player:
        // an optional choice with no legal option (a kicker the player cannot
        // pay; the protocol cannot show disabled options), and a combat
        // declaration with nothing to declare.
        self.manabrew_forced_command = None;
        for _ in 0..16 {
            let forced = match self.pending_decision.as_ref() {
                Some(DecisionContext::SelectOptions(ctx))
                    if ctx.min == 0 && ctx.options.iter().all(|option| !option.legal) =>
                {
                    Some(UiCommand::SelectOptions {
                        option_indices: Vec::new(),
                    })
                }
                Some(DecisionContext::Attackers(ctx)) if ctx.attacker_options.is_empty() => {
                    Some(UiCommand::DeclareAttackers {
                        declarations: Vec::new(),
                        bands: Vec::new(),
                    })
                }
                Some(DecisionContext::Blockers(ctx))
                    if ctx
                        .blocker_options
                        .iter()
                        .all(|option| option.valid_blockers.is_empty()) =>
                {
                    Some(UiCommand::DeclareBlockers {
                        declarations: Vec::new(),
                    })
                }
                _ => None,
            };
            let Some(command) = forced else {
                break;
            };
            if self.manabrew_verified {
                // Applying it here would skip signing and replay on other seats.
                self.manabrew_forced_command = Some(command);
                self.manabrew_open_prompt = None;
                return Ok(None);
            }
            if self.dispatch_typed_command(command, 0.0).is_err() {
                break;
            }
        }
        let Some(context) = self.pending_decision.clone() else {
            self.manabrew_open_prompt = None;
            return Ok(None);
        };
        let decision_hash = hash_debug_value(&context);
        if let Some(open) = self.manabrew_open_prompt.as_ref()
            && open.decision_hash == decision_hash
            && open.deciding_player == context.player()
        {
            return Ok(Some(AgentPrompt {
                prompt_id: open.prompt_id,
                deciding_player_id: player_id(open.deciding_player),
                source_card: open.source_card.clone(),
                source_ability_text: None,
                input: open.input.clone(),
            }));
        }
        let (input, binding) = self.build_manabrew_prompt(&context)?;
        let prompt_id = self.manabrew_next_prompt_id;
        self.manabrew_next_prompt_id = prompt_id.checked_add(1).ok_or_else(|| {
            protocol_error(
                ProtocolErrorCode::InvalidShape,
                "Manabrew prompt ID space is exhausted",
                None,
            )
        })?;
        let open = ManabrewOpenPrompt {
            prompt_id,
            deciding_player: context.player(),
            decision_hash,
            source_card_id: self.manabrew_source_card_id(&context),
            source_card: context
                .source()
                .and_then(|source| protocol_card(self.manabrew_game(), source)),
            input,
            binding,
        };
        let prompt = AgentPrompt {
            prompt_id: open.prompt_id,
            deciding_player_id: player_id(open.deciding_player),
            source_card: open.source_card.clone(),
            source_ability_text: None,
            input: open.input.clone(),
        };
        self.manabrew_open_prompt = Some(open);
        Ok(Some(prompt))
    }

    fn manabrew_result(
        &mut self,
        viewer: Option<PlayerId>,
        error: Option<ProtocolError>,
    ) -> ManabrewViewResult {
        let prompt = match self.ensure_manabrew_prompt() {
            Ok(prompt) => prompt.filter(|prompt| {
                viewer.is_some_and(|viewer| prompt.deciding_player_id == player_id(viewer))
            }),
            Err(prompt_error) => {
                return ManabrewViewResult {
                    protocol_version: manabrew_protocol_major(),
                    state: self.manabrew_state(viewer),
                    prompt: None,
                    error: error.or(Some(prompt_error)),
                    forced_command: None,
                };
            }
        };
        let forced_command = viewer
            .filter(|viewer| {
                self.pending_decision
                    .as_ref()
                    .is_some_and(|context| context.player() == *viewer)
            })
            .and(self.manabrew_forced_command.as_ref())
            .and_then(|command| serde_json::to_value(command).ok());
        ManabrewViewResult {
            protocol_version: manabrew_protocol_major(),
            state: self.manabrew_state(viewer),
            prompt,
            error,
            forced_command,
        }
    }

    fn validate_manabrew_prompt_owner(
        &self,
        player: PlayerId,
        prompt_id: u32,
    ) -> Result<&ManabrewOpenPrompt, ProtocolError> {
        let Some(open) = self.manabrew_open_prompt.as_ref() else {
            return Err(protocol_error(
                ProtocolErrorCode::StalePrompt,
                "there is no open prompt",
                Some(prompt_id),
            ));
        };
        if open.prompt_id != prompt_id {
            return Err(protocol_error(
                ProtocolErrorCode::StalePrompt,
                format!(
                    "prompt {prompt_id} is stale; current prompt is {}",
                    open.prompt_id
                ),
                Some(prompt_id),
            ));
        }
        if open.deciding_player != player {
            return Err(protocol_error(
                ProtocolErrorCode::WrongPlayer,
                format!("{} does not own prompt {prompt_id}", player_id(player)),
                Some(prompt_id),
            ));
        }
        Ok(open)
    }

    fn validate_manabrew_response(
        &self,
        player: PlayerId,
        prompt_id: u32,
        output: &PromptOutput,
    ) -> Result<&ManabrewOpenPrompt, ProtocolError> {
        let open = self.validate_manabrew_prompt_owner(player, prompt_id)?;
        open.input
            .validate_response(output)
            .map_err(|violation| match violation {
                ResponseViolation::WrongPromptType => protocol_error(
                    ProtocolErrorCode::WrongPromptType,
                    "prompt output family does not match the open prompt",
                    Some(prompt_id),
                ),
                ResponseViolation::UnknownActionId(action) => protocol_error(
                    ProtocolErrorCode::UnknownActionId,
                    format!("unknown action id {action}"),
                    Some(prompt_id),
                ),
                ResponseViolation::CancelNotAllowed => protocol_error(
                    ProtocolErrorCode::CancelNotAllowed,
                    "this prompt cannot be cancelled",
                    Some(prompt_id),
                ),
            })?;
        Ok(open)
    }

    fn manabrew_distribution_response(
        open: &ManabrewOpenPrompt,
        state: &ManabrewDistributionState,
        amount: u32,
    ) -> Result<ManabrewResponseAction, ProtocolError> {
        let invalid = |message: String| {
            protocol_error(
                ProtocolErrorCode::InvalidShape,
                message,
                Some(open.prompt_id),
            )
        };
        let remaining = state.remaining.checked_sub(amount).ok_or_else(|| {
            invalid(format!(
                "distribution amount {amount} exceeds the remaining {}",
                state.remaining
            ))
        })?;
        if amount != 0 && amount < state.min_per_target {
            return Err(invalid(format!(
                "distribution amount {amount} is below the per-target minimum {}",
                state.min_per_target
            )));
        }
        let targets_left = state
            .target_names
            .len()
            .checked_sub(state.target_index + 1)
            .ok_or_else(|| invalid("distribution target index is out of range".to_string()))?;
        if remaining != 0 && (targets_left == 0 || remaining < state.min_per_target) {
            return Err(invalid(format!(
                "distribution amount {amount} leaves an unassignable remainder of {remaining}"
            )));
        }

        let mut next = state.clone();
        next.allocations.push(amount);
        next.remaining = remaining;
        next.target_index += 1;
        if remaining == 0 {
            next.allocations.resize(next.target_names.len(), 0);
        }
        if next.target_index >= next.target_names.len() || remaining == 0 {
            if remaining != 0 {
                return Err(invalid(format!(
                    "distribution is incomplete; {remaining} remains"
                )));
            }
            let mut option_indices = Vec::new();
            for (index, count) in next.allocations.iter().copied().enumerate() {
                option_indices.extend(std::iter::repeat_n(index, count as usize));
            }
            return Ok(ManabrewResponseAction::Dispatch(UiCommand::SelectOptions {
                option_indices,
            }));
        }

        let (input, binding) =
            Self::manabrew_distribution_prompt(next, open.source_card_id.clone())?;
        Ok(ManabrewResponseAction::Continue { input, binding })
    }

    fn manabrew_counter_response(
        open: &ManabrewOpenPrompt,
        state: &ManabrewCounterState,
        amount: u32,
    ) -> Result<ManabrewResponseAction, ProtocolError> {
        let invalid = |message: String| {
            protocol_error(
                ProtocolErrorCode::InvalidShape,
                message,
                Some(open.prompt_id),
            )
        };
        let (min, max) = Self::manabrew_counter_chunk_bounds(state)?;
        if amount < min || amount > max {
            return Err(invalid(format!("counter chunk {amount} must be between {min} and {max}")));
        }
        let mut next = state.clone();
        let allocation = next.allocations.get_mut(next.counter_index)
            .ok_or_else(|| invalid("counter allocation index is out of range".into()))?;
        *allocation = allocation.checked_add(amount)
            .ok_or_else(|| invalid("counter allocation exceeds per-kind range".into()))?;
        next.remaining -= u64::from(amount);
        next.minimum_remaining = next.minimum_remaining.saturating_sub(u64::from(amount));
        let continue_kind = amount == max && amount > 0 && next.remaining > 0
            && *allocation < next.available[next.counter_index];
        if !continue_kind { next.counter_index += 1; }
        if next.counter_index >= next.counter_names.len() || next.remaining == 0 {
            if next.minimum_remaining > 0 {
                return Err(invalid("counter allocation does not satisfy the required total".into()));
            }
            let allocations = next.allocations.iter().copied().enumerate()
                .filter(|(_,count)| *count > 0)
                .map(|(index,count)| CounterAllocation { index,count }).collect();
            return Ok(ManabrewResponseAction::Dispatch(UiCommand::SelectCounters { allocations }));
        }

        let (input, binding) = Self::manabrew_counter_prompt(next, open.source_card_id.clone())?;
        Ok(ManabrewResponseAction::Continue { input, binding })
    }

    fn manabrew_opaque_exile_identity(&self, card: ObjectId) -> Option<(u8, u16, String)> {
        self.manabrew_game().hidden_card_info(card).map(|info| {
            let (slot, commitment) = match (info.public_slot, info.public_commitment.as_ref()) {
                (Some(slot), Some(commitment)) => (slot, commitment.clone()),
                _ => (info.slot, info.commitment.clone()),
            };
            (info.owner.0, slot, commitment)
        })
    }

    /// A protocol pass. `exhaust_stack` and `until` grant the engine's own
    /// priority yield, which keeps passing for this seat until the stack
    /// empties or the named step arrives, and always stops for a required
    /// choice, a new stack object, or the end of the turn.
    fn manabrew_pass_command(
        &self,
        until: Option<&manabrew_protocol::prompts::choose_action::PassUntil>,
        exhaust_stack: bool,
    ) -> UiCommand {
        // A plain pass is sent by reference so that it continues a yield
        // another seat has already granted.
        let plain = UiCommand::PriorityAction {
            action_index: None,
            action_ref: Some(PriorityActionRef::PassPriority),
        };
        if !self.manabrew_game().stack.is_empty() {
            return if exhaust_stack {
                UiCommand::PriorityYield {
                    mode: PassMode::Stack,
                    stops: Vec::new(),
                }
            } else {
                plain
            };
        }
        let Some(until) = until else {
            return plain;
        };
        let stops = if until.player_id == player_id(self.manabrew_game().turn.active_player) {
            let stop = match until.phase {
                StepKind::Untap => "step:Untap",
                StepKind::Upkeep => "step:Upkeep",
                StepKind::Draw => "step:Draw",
                StepKind::Main1 => "phase:Main",
                StepKind::CombatBegin => "step:BeginCombat",
                StepKind::CombatDeclareAttackers => "step:DeclareAttackers",
                StepKind::CombatDeclareBlockers => "step:DeclareBlockers",
                StepKind::CombatFirstStrikeDamage => "step:FirstStrikeDamage",
                StepKind::CombatDamage => "step:CombatDamage",
                StepKind::CombatEnd => "step:EndCombat",
                StepKind::Main2 => "phase:Main2",
                StepKind::EndOfTurn => "step:End",
                StepKind::Cleanup => "step:Cleanup",
            };
            vec![stop.to_string()]
        } else if until.through_combat {
            // The destination is in a later turn: yield for the rest of this
            // one. With no stops the engine also declares no attackers.
            Vec::new()
        } else {
            // As above, but still ask this seat about its own attack. No
            // player receives priority in the cleanup step, so this stop only
            // keeps the attack declaration a required choice.
            vec!["step:Cleanup".to_string()]
        };
        UiCommand::PriorityYield {
            mode: PassMode::Phases,
            stops,
        }
    }

    fn manabrew_response_action(
        &self,
        open: &ManabrewOpenPrompt,
        output: PromptOutput,
    ) -> Result<ManabrewResponseAction, ProtocolError> {
        let invalid = |message: String| {
            protocol_error(
                ProtocolErrorCode::InvalidShape,
                message,
                Some(open.prompt_id),
            )
        };
        match (&open.binding, output) {
            (
                ManabrewPromptBinding::Priority {
                    actions,
                    opaque_exile,
                    pass_index,
                },
                PromptOutput::ChooseAction(output),
            ) => {
                let index = match output {
                    ChooseActionOutput::Pass {
                        until,
                        exhaust_stack,
                    } => {
                        let passes_priority = matches!(
                            self.pending_decision.as_ref(),
                            Some(DecisionContext::Priority(priority))
                                if matches!(
                                    priority.actions.get(*pass_index),
                                    Some(LegalAction::PassPriority)
                                )
                        );
                        if passes_priority {
                            return Ok(ManabrewResponseAction::Dispatch(
                                self.manabrew_pass_command(until.as_ref(), exhaust_stack),
                            ));
                        }
                        *pass_index
                    }
                    ChooseActionOutput::Act { action_id } => *actions
                        .get(&action_id)
                        .ok_or_else(|| invalid(format!("unknown action id {action_id}")))?,
                };
                if let Some(captured) = opaque_exile.get(&index) {
                    let Some(context @ DecisionContext::Priority(priority)) = self.pending_decision.as_ref() else {
                        return Err(invalid("opaque exile prompt no longer owns priority".into()));
                    };
                    if hash_debug_value(context) != open.decision_hash || priority.player != open.deciding_player {
                        return Err(invalid("opaque exile prompt belongs to a different decision epoch".into()));
                    }
                    let card = match &captured.action_ref {
                        PriorityActionRef::OpenExiledCardForPlay { card_id, .. }
                        | PriorityActionRef::CastExiledCardFaceDown { card_id, .. } => ObjectId::from_raw(*card_id),
                        _ => return Err(invalid("opaque exile prompt lost its captured reference".into())),
                    };
                    if self.manabrew_opaque_exile_identity(card) != captured.hidden_identity
                        || resolve_priority_action(self.manabrew_game(), priority, None, Some(&captured.action_ref))
                            .map_err(|error| invalid(error.to_string()))?.is_none() {
                        return Err(invalid("opaque exile prompt lost its exact paired origin".into()));
                    }
                    return Ok(ManabrewResponseAction::Dispatch(UiCommand::PriorityAction {
                        action_index: None, action_ref: Some(captured.action_ref.clone()),
                    }));
                }
                Ok(ManabrewResponseAction::Dispatch(
                    UiCommand::PriorityAction {
                        action_index: Some(index),
                        action_ref: None,
                    },
                ))
            }
            (
                ManabrewPromptBinding::Mulligan {
                    keep_index,
                    mulligan_index,
                },
                PromptOutput::Mulligan(MulliganOutput::MulliganDecision { keep }),
            ) => Ok(ManabrewResponseAction::Dispatch(
                UiCommand::PriorityAction {
                    action_index: Some(if keep { *keep_index } else { *mulligan_index }),
                    action_ref: None,
                },
            )),
            (
                ManabrewPromptBinding::Boolean,
                PromptOutput::ChooseBoolean(ChooseBooleanOutput::Decision { value }),
            ) => Ok(ManabrewResponseAction::Dispatch(UiCommand::SelectOptions {
                option_indices: vec![usize::from(value)],
            })),
            (
                ManabrewPromptBinding::Number { minimum_remaining, maximum_remaining, accumulated, description },
                PromptOutput::ChooseNumber(ChooseNumberOutput::NumberDecision { chosen_number }),
            ) => {
                let amount = chosen_number.filter(|value| *value >= 0)
                    .ok_or_else(|| invalid("a non-negative number is required".into()))? as u32;
                let chunk_min = (*minimum_remaining).min(i32::MAX as u32);
                let chunk_max = (*maximum_remaining).min(i32::MAX as u32);
                if minimum_remaining > maximum_remaining || amount < chunk_min || amount > chunk_max {
                    return Err(invalid(format!("number chunk {amount} must be between {chunk_min} and {chunk_max}")));
                }
                let total = accumulated.checked_add(amount).ok_or_else(|| invalid("number choice exceeds the unsigned range".into()))?;
                let minimum = minimum_remaining.saturating_sub(amount);
                let maximum = maximum_remaining - amount;
                if amount == chunk_max && amount > 0 && maximum > 0 {
                    let (input, binding) = Self::manabrew_number_prompt(minimum, maximum, total, description.clone(), open.source_card_id.clone())?;
                    return Ok(ManabrewResponseAction::Continue { input, binding });
                }
                if minimum > 0 { return Err(invalid("number choice does not satisfy the required minimum".into())); }
                Ok(ManabrewResponseAction::Dispatch(UiCommand::NumberChoice { value: total }))
            }
            (
                ManabrewPromptBinding::TextNameGroups {
                    description,
                    groups,
                },
                PromptOutput::ChooseFromSelection(ChooseFromSelectionOutput::SelectionDecision {
                    chosen_indices,
                }),
            ) => {
                let [index] = chosen_indices.as_slice() else {
                    return Err(invalid(
                        "exactly one card-name range is required".to_string(),
                    ));
                };
                let names = groups
                    .get(*index)
                    .cloned()
                    .ok_or_else(|| invalid(format!("card-name range {index} is out of range")))?;
                let (input, binding) = Self::manabrew_text_name_prompt(
                    description.clone(),
                    open.source_card_id.clone(),
                    names,
                );
                Ok(ManabrewResponseAction::Continue { input, binding })
            }
            (
                ManabrewPromptBinding::TextNames { names },
                PromptOutput::ChooseFromSelection(ChooseFromSelectionOutput::SelectionDecision {
                    chosen_indices,
                }),
            ) => {
                let [index] = chosen_indices.as_slice() else {
                    return Err(invalid("exactly one card name is required".to_string()));
                };
                let value = names
                    .get(*index)
                    .cloned()
                    .ok_or_else(|| invalid(format!("card-name index {index} is out of range")))?;
                Ok(ManabrewResponseAction::Dispatch(UiCommand::TextChoice {
                    value,
                }))
            }
            (
                ManabrewPromptBinding::CombatDamage { targets },
                PromptOutput::ChooseCombatDamageAssignment(
                    ChooseCombatDamageAssignmentOutput::CombatDamageAssignmentDecision {
                        assignments,
                    },
                ),
            ) => {
                let mut option_indices = Vec::new();
                for assignment in assignments {
                    let index = targets
                        .iter()
                        .position(|target| target == &assignment.assignee_id)
                        .ok_or_else(|| {
                            invalid(format!(
                                "unknown combat damage recipient {}",
                                assignment.assignee_id
                            ))
                        })?;
                    let amount = usize::try_from(assignment.damage)
                        .map_err(|_| invalid("combat damage must be non-negative".to_string()))?;
                    option_indices.extend(std::iter::repeat_n(index, amount));
                }
                Ok(ManabrewResponseAction::Dispatch(UiCommand::SelectOptions {
                    option_indices,
                }))
            }
            (
                ManabrewPromptBinding::DistributionNumber { state },
                PromptOutput::ChooseNumber(ChooseNumberOutput::NumberDecision { chosen_number }),
            ) => {
                let amount = chosen_number
                    .filter(|value| *value >= 0)
                    .ok_or_else(|| invalid("a non-negative amount is required".to_string()))?
                    as u32;
                Self::manabrew_distribution_response(open, state, amount)
            }
            (
                ManabrewPromptBinding::DistributionOptions { state, amounts },
                PromptOutput::ChooseFromSelection(ChooseFromSelectionOutput::SelectionDecision {
                    chosen_indices,
                }),
            ) => {
                let [index] = chosen_indices.as_slice() else {
                    return Err(invalid(
                        "exactly one distribution amount is required".to_string(),
                    ));
                };
                let amount = amounts.get(*index).copied().ok_or_else(|| {
                    invalid(format!("distribution amount index {index} is out of range"))
                })?;
                Self::manabrew_distribution_response(open, state, amount)
            }
            (
                ManabrewPromptBinding::CounterNumber { state },
                PromptOutput::ChooseNumber(ChooseNumberOutput::NumberDecision { chosen_number }),
            ) => {
                let amount = chosen_number
                    .filter(|value| *value >= 0)
                    .ok_or_else(|| invalid("a non-negative amount is required".to_string()))?
                    as u32;
                Self::manabrew_counter_response(open, state, amount)
            }
            (
                ManabrewPromptBinding::Options { indices },
                PromptOutput::ChooseFromSelection(ChooseFromSelectionOutput::SelectionDecision {
                    chosen_indices,
                }),
            ) => {
                let option_indices = chosen_indices
                    .into_iter()
                    .map(|index| {
                        indices
                            .get(index)
                            .copied()
                            .ok_or_else(|| invalid(format!("option index {index} is out of range")))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ManabrewResponseAction::Dispatch(UiCommand::SelectOptions {
                    option_indices,
                }))
            }
            (
                ManabrewPromptBinding::Objects { objects },
                PromptOutput::ChooseCards(ChooseCardsOutput::ChooseCardsDecision {
                    chosen_card_ids,
                })
                | PromptOutput::MulliganPutBack(MulliganPutBackOutput::MulliganPutBackDecision {
                    card_ids: chosen_card_ids,
                }),
            ) => {
                let object_ids = chosen_card_ids
                    .into_iter()
                    .map(|id| {
                        objects
                            .get(&id)
                            .map(|object| object.0)
                            .ok_or_else(|| invalid(format!("unknown card id {id}")))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ManabrewResponseAction::Dispatch(UiCommand::SelectObjects {
                    object_ids,
                    object_stable_ids: Vec::new(),
                    object_hidden_refs: Vec::new(),
                }))
            }
            (
                ManabrewPromptBinding::Targets { .. },
                PromptOutput::ChooseBoardTargets(ChooseBoardTargetsOutput::Cancel),
            ) => {
                // `validate_response` admits Cancel only for a prompt issued as
                // cancellable; the engine re-checks at dispatch.
                Ok(ManabrewResponseAction::Cancel)
            }
            (
                ManabrewPromptBinding::Targets { targets, state },
                PromptOutput::ChooseBoardTargets(ChooseBoardTargetsOutput::BoardTargets { chosen }),
            ) => {
                let selected = chosen
                    .into_iter()
                    .map(|target| {
                        let key = format!("{:?}:{}", target.kind, target.id);
                        targets
                            .get(&key)
                            .copied()
                            .ok_or_else(|| invalid(format!("unknown target {}", target.id)))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let PromptInput::ChooseBoardTargets(input) = &open.input else {
                    return Err(invalid("target binding requires a target prompt".into()));
                };
                if selected.len() < input.min_targets as usize
                    || selected.len() > input.max_targets as usize
                    || (!state.requirements.is_empty() && !Self::manabrew_target_prefix_valid(state, &selected))
                {
                    return Err(invalid(
                        "targets do not satisfy this targeting requirement".into(),
                    ));
                }
                let mut next = state.clone();
                if let Some(requirement) = state.requirements.get(state.selections.len()) {
                    if let Some(max) = requirement.max_targets {
                        let base_max = max.saturating_sub(requirement.additional_target_budget);
                        next.additional_remaining -= selected.len().saturating_sub(base_max);
                    }
                    next.selections.push(selected);
                }
                if next.selections.len() < next.requirements.len() {
                    let (input, binding) = self.manabrew_targets_prompt(next, open.source_card_id.clone())?;
                    return Ok(ManabrewResponseAction::Continue { input, binding });
                }
                let targets = next
                    .selections
                    .into_iter()
                    .flatten()
                    .map(|target| match target {
                        Target::Player(player) => TargetInput::Player { player: player.0 },
                        Target::Object(object) => TargetInput::Object { object: object.0 },
                    })
                    .collect();
                Ok(ManabrewResponseAction::Dispatch(UiCommand::SelectTargets {
                    targets,
                }))
            }
            (
                ManabrewPromptBinding::Attackers { attackers, targets },
                PromptOutput::ChooseAttackers(ChooseAttackersOutput::DeclareAttackers {
                    assignments,
                }),
            ) => {
                let declarations = assignments
                    .into_iter()
                    .map(
                        |AttackAssignment {
                             attacker_id,
                             target_id,
                         }| {
                            let creature =
                                attackers.get(&attacker_id).copied().ok_or_else(|| {
                                    invalid(format!("unknown attacker {attacker_id}"))
                                })?;
                            let target =
                                match targets.get(&target_id).cloned().ok_or_else(|| {
                                    invalid(format!("unknown attack target {target_id}"))
                                })? {
                                    AttackTarget::Player(player) => {
                                        AttackTargetInput::Player { player: player.0 }
                                    }
                                    AttackTarget::Planeswalker(object) => {
                                        AttackTargetInput::Planeswalker { object: object.0 }
                                    }
                                    AttackTarget::Battle(object) => {
                                        AttackTargetInput::Battle { object: object.0 }
                                    }
                                    AttackTarget::Nothing { .. } => {
                                        return Err(invalid(format!(
                                            "unknown attack target {target_id}"
                                        )));
                                    }
                                };
                            Ok(AttackerDeclarationInput {
                                creature: creature.0,
                                target,
                            })
                        },
                    )
                    .collect::<Result<Vec<_>, ProtocolError>>()?;
                Ok(ManabrewResponseAction::Dispatch(
                    UiCommand::DeclareAttackers {
                        declarations,
                        bands: Vec::new(),
                    },
                ))
            }
            (
                ManabrewPromptBinding::Blockers { objects },
                PromptOutput::ChooseBlockers(ChooseBlockersOutput::DeclareBlockers { assignments }),
            ) => {
                let declarations = assignments
                    .into_iter()
                    .map(
                        |BlockAssignment {
                             blocker_id,
                             attacker_id,
                         }| {
                            let blocker = objects
                                .get(&blocker_id)
                                .copied()
                                .ok_or_else(|| invalid(format!("unknown blocker {blocker_id}")))?;
                            let blocking = objects.get(&attacker_id).copied().ok_or_else(|| {
                                invalid(format!("unknown attacker {attacker_id}"))
                            })?;
                            Ok(BlockerDeclarationInput {
                                blocker: blocker.0,
                                blocking: blocking.0,
                            })
                        },
                    )
                    .collect::<Result<Vec<_>, ProtocolError>>()?;
                Ok(ManabrewResponseAction::Dispatch(
                    UiCommand::DeclareBlockers { declarations },
                ))
            }
            (
                ManabrewPromptBinding::Reorder { indices },
                PromptOutput::Reorder(ReorderOutput::ReorderDecision { ordered_ids }),
            ) => {
                let option_indices = ordered_ids
                    .into_iter()
                    .map(|id| {
                        indices
                            .get(&id)
                            .copied()
                            .ok_or_else(|| invalid(format!("unknown reorder id {id}")))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ManabrewResponseAction::Dispatch(UiCommand::SelectOptions {
                    option_indices,
                }))
            }
            (
                ManabrewPromptBinding::Colors { indices },
                PromptOutput::ChooseColor(ChooseColorOutput::ColorDecision { chosen_colors }),
            ) => {
                let mut option_indices = Vec::new();
                for (color, count) in chosen_colors {
                    let index = indices
                        .get(&color)
                        .copied()
                        .ok_or_else(|| invalid(format!("unknown color {color}")))?;
                    option_indices.extend(std::iter::repeat_n(index, count as usize));
                }
                Ok(ManabrewResponseAction::Dispatch(UiCommand::SelectOptions {
                    option_indices,
                }))
            }
            (
                ManabrewPromptBinding::Partition {
                    objects,
                    secondary_zone_index,
                },
                PromptOutput::Scry(ScryOutput::ScryDecision { zone_card_ids }),
            ) => {
                let chosen = zone_card_ids
                    .get(*secondary_zone_index)
                    .cloned()
                    .unwrap_or_default();
                let object_ids = chosen
                    .into_iter()
                    .map(|id| {
                        objects
                            .get(&id)
                            .map(|object| object.0)
                            .ok_or_else(|| invalid(format!("unknown partition card {id}")))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ManabrewResponseAction::Dispatch(UiCommand::SelectObjects {
                    object_ids,
                    object_stable_ids: Vec::new(),
                    object_hidden_refs: Vec::new(),
                }))
            }
            (
                ManabrewPromptBinding::AuthoritativePayment {
                    plan_id,
                    request_hash,
                    actions,
                    can_confirm_manually,
                    payable,
                },
                PromptOutput::PayManaCost(output),
            ) => match output {
                PayManaCostOutput::Pay { .. } if !*payable => Err(invalid(
                    "this cost cannot be paid with the current payment choices; cancel, or undo and choose differently"
                        .to_string(),
                )),
                PayManaCostOutput::Pay { auto } => {
                    if auto || *can_confirm_manually {
                        Ok(ManabrewResponseAction::Dispatch(UiCommand::ManaPayment {
                            response: ManaPaymentCommand::Confirm {
                                plan_id: plan_id.clone(),
                                request_hash: request_hash.clone(),
                            },
                        }))
                    } else {
                        Err(invalid(
                            "manual payment is incomplete; select the remaining actions or use auto pay"
                                .to_string(),
                        ))
                    }
                }
                PayManaCostOutput::Cancel => {
                    Ok(ManabrewResponseAction::Dispatch(UiCommand::ManaPayment {
                        response: ManaPaymentCommand::Cancel,
                    }))
                }
                PayManaCostOutput::Act { action_id } => {
                    Ok(ManabrewResponseAction::Dispatch(UiCommand::ManaPayment {
                        response: actions.get(&action_id).cloned().ok_or_else(|| {
                            invalid(format!("unknown payment action {action_id}"))
                        })?,
                    }))
                }
            },
            _ => Err(protocol_error(
                ProtocolErrorCode::WrongPromptType,
                "prompt output family does not match the stored binding",
                Some(open.prompt_id),
            )),
        }
    }
}

/// Printed loyalty for a planeswalker face the adapter knows, by name.
#[wasm_bindgen(js_name = manabrewPrintedLoyalty)]
pub fn manabrew_printed_loyalty(name: &str) -> Option<String> {
    manabrew_printed_stat(name, ManabrewPrintedStat::Loyalty).map(str::to_string)
}

/// Printed defense for a battle face the adapter knows, by name.
#[wasm_bindgen(js_name = manabrewPrintedDefense)]
pub fn manabrew_printed_defense(name: &str) -> Option<String> {
    manabrew_printed_stat(name, ManabrewPrintedStat::Defense).map(str::to_string)
}

/// The card sources a list of Manabrew decks describes, in the shape
/// `registerExternalCardSources` takes. A host that compiles card source
/// outside the engine reads them here, so there is one reading of a deck card.
#[wasm_bindgen(js_name = manabrewDeckSources)]
pub fn manabrew_deck_sources_js(decks: JsValue) -> Result<JsValue, JsValue> {
    let decks: Vec<Value> = if decks.is_undefined() || decks.is_null() {
        Vec::new()
    } else {
        serde_wasm_bindgen::from_value(decks)
            .map_err(|error| JsValue::from_str(&format!("invalid Manabrew decks: {error}")))?
    };
    manabrew_to_js(&manabrew_deck_sources(&decks), "Manabrew deck sources")
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(js_name = manabrewProtocolVersion)]
    pub fn manabrew_protocol_version(&self) -> u32 { manabrew_protocol_major() }

    /// The exact `manabrew-protocol` crate version (also the `@manabrew/protocol`
    /// npm version) whose DTO shapes this adapter implements.
    #[wasm_bindgen(js_name = manabrewProtocolCrateVersion)]
    pub fn manabrew_protocol_crate_version(&self) -> String {
        manabrew_protocol::VERSION.to_string()
    }

    #[wasm_bindgen(js_name = registerManabrewDeckSources)]
    pub fn register_manabrew_deck_sources(&mut self, decks: JsValue) -> Result<JsValue, JsValue> {
        let decks: Vec<Value> = serde_wasm_bindgen::from_value(decks)
            .map_err(|error| JsValue::from_str(&format!("invalid Manabrew decks: {error}")))?;
        let summary = self.register_manabrew_deck_sources_input(&decks);
        manabrew_to_js(&summary, "Manabrew deck source registration summary")
    }

    #[wasm_bindgen(js_name = validateManabrewMatchConfig)]
    pub fn validate_manabrew_match_config(&mut self, config: JsValue) -> Result<JsValue, JsValue> {
        let input: ManabrewMatchConfigInput =
            serde_wasm_bindgen::from_value(config).map_err(|error| {
                JsValue::from_str(&format!("invalid Manabrew match config: {error}"))
            })?;
        validate_manabrew_protocol_version(input.protocol_version)
            .map_err(|error| JsValue::from_str(&error))?;
        self.register_manabrew_deck_sources_input(&input.decks);
        let mut validation = self.validate_match_setup_input(&manabrew_match_setup(&input))?;
        let missing = Self::manabrew_printed_stat_issues(&input);
        if !missing.is_empty() {
            validation.valid = false;
            validation.issues.extend(missing);
        }
        manabrew_to_js(&validation, "Manabrew match validation")
    }

    #[wasm_bindgen(js_name = startManabrewMatch)]
    pub fn start_manabrew_match(&mut self, config: JsValue) -> Result<JsValue, JsValue> {
        let input: ManabrewMatchConfigInput =
            serde_wasm_bindgen::from_value(config).map_err(|error| {
                JsValue::from_str(&format!("invalid Manabrew match config: {error}"))
            })?;
        validate_manabrew_protocol_version(input.protocol_version)
            .map_err(|error| JsValue::from_str(&error))?;
        // Such a permanent would be put into the graveyard as it enters.
        if let Some(issue) = Self::manabrew_printed_stat_issues(&input).first() {
            return Err(JsValue::from_str(&issue.error));
        }
        // The seats are known before the first menu is built.
        self.manabrew_human_players = if input.human_players.len() == input.player_names.len() {
            input.human_players.clone()
        } else {
            (0..input.player_names.len())
                .map(|index| !input.bot_players.contains(&(index as u8)))
                .collect()
        };
        let _menu_policy = ManabrewMenuPolicy::enter(&self.manabrew_human_players);
        self.register_manabrew_deck_sources_input(&input.decks);
        let setup = manabrew_match_setup(&input);
        let seed = setup.seed;
        let setup_js = manabrew_to_js(&setup, "Ironsmith match setup")?;
        let result = self.start_match(setup_js)?;
        self.manabrew_game_id = input
            .game_id
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| format!("ironsmith-{seed:016x}"));
        self.manabrew_next_prompt_id = 1;
        self.manabrew_open_prompt = None;
        Ok(result)
    }

    /// Build a typed player-specific view. Passing no viewer produces a spectator-safe public view.
    #[wasm_bindgen(js_name = manabrewView)]
    pub fn manabrew_view(&mut self, viewer: Option<u8>) -> Result<JsValue, JsValue> {
        let _menu_policy = ManabrewMenuPolicy::enter(&self.manabrew_human_players);
        let viewer = match viewer {
            Some(viewer) => match self.manabrew_player(viewer) {
                Ok(player) => Some(player),
                Err(error) => {
                    return manabrew_to_js(
                        &ManabrewViewResult {
                            protocol_version: manabrew_protocol_major(),
                            state: self.manabrew_state(None),
                            prompt: None,
                            error: Some(error),
                            forced_command: None,
                        },
                        "Manabrew view",
                    );
                }
            },
            None => None,
        };
        let result = self.manabrew_result(viewer, None);
        manabrew_to_js(&result, "Manabrew view")
    }

    /// Deprecated alias for a spectator-safe typed view.
    #[wasm_bindgen(js_name = manabrewPublicState)]
    pub fn manabrew_public_state(&self) -> Result<JsValue, JsValue> {
        manabrew_to_js(&self.manabrew_state(None), "Manabrew public state")
    }

    #[wasm_bindgen(js_name = manabrewRespond)]
    pub fn manabrew_respond(
        &mut self,
        player: u8,
        prompt_id: u32,
        output: JsValue,
    ) -> Result<JsValue, JsValue> {
        let _menu_policy = ManabrewMenuPolicy::enter(&self.manabrew_human_players);
        let player = match self.manabrew_player(player) {
            Ok(player) => player,
            Err(error) => {
                let result = self.manabrew_result(None, Some(error));
                return manabrew_to_js(&result, "Manabrew response");
            }
        };
        if let Err(error) = self.validate_manabrew_prompt_owner(player, prompt_id) {
            let result = self.manabrew_result(Some(player), Some(error));
            return manabrew_to_js(&result, "Manabrew response");
        }
        let output: PromptOutput = match serde_wasm_bindgen::from_value(output) {
            Ok(output) => output,
            Err(error) => {
                let result = self.manabrew_result(
                    Some(player),
                    Some(protocol_error(
                        ProtocolErrorCode::InvalidShape,
                        format!("invalid Manabrew prompt output: {error}"),
                        Some(prompt_id),
                    )),
                );
                return manabrew_to_js(&result, "Manabrew response");
            }
        };
        let open = match self.validate_manabrew_response(player, prompt_id, &output) {
            Ok(open) => open.clone(),
            Err(error) => {
                let result = self.manabrew_result(Some(player), Some(error));
                return manabrew_to_js(&result, "Manabrew response");
            }
        };
        let action = match self.manabrew_response_action(&open, output) {
            Ok(action) => action,
            Err(error) => {
                let result = self.manabrew_result(Some(player), Some(error));
                return manabrew_to_js(&result, "Manabrew response");
            }
        };
        let engine_result = match action {
            ManabrewResponseAction::Dispatch(command) => {
                let command = manabrew_to_js(&command, "Ironsmith command")?;
                self.dispatch(command)
            }
            ManabrewResponseAction::Cancel => self.cancel_decision(),
            ManabrewResponseAction::Continue { input, binding } => {
                let next_prompt_id = self.manabrew_next_prompt_id;
                let Some(after_next_prompt_id) = next_prompt_id.checked_add(1) else {
                    let result = self.manabrew_result(
                        Some(player),
                        Some(protocol_error(
                            ProtocolErrorCode::InvalidShape,
                            "Manabrew prompt ID space is exhausted",
                            Some(prompt_id),
                        )),
                    );
                    return manabrew_to_js(&result, "Manabrew response");
                };
                self.manabrew_next_prompt_id = after_next_prompt_id;
                self.manabrew_open_prompt = Some(ManabrewOpenPrompt {
                    prompt_id: next_prompt_id,
                    deciding_player: open.deciding_player,
                    decision_hash: open.decision_hash,
                    source_card_id: open.source_card_id,
                    source_card: open.source_card,
                    input,
                    binding,
                });
                let result = self.manabrew_result(Some(player), None);
                return manabrew_to_js(&result, "Manabrew response");
            }
        };
        if let Err(error) = engine_result {
            let result = self.manabrew_result(
                Some(player),
                Some(protocol_error(
                    ProtocolErrorCode::InvalidShape,
                    error
                        .as_string()
                        .unwrap_or_else(|| "Ironsmith rejected the response".to_string()),
                    Some(prompt_id),
                )),
            );
            return manabrew_to_js(&result, "Manabrew response");
        }
        self.manabrew_open_prompt = None;
        let result = self.manabrew_result(Some(player), None);
        manabrew_to_js(&result, "Manabrew response")
    }

    /// Attach the Manabrew adapter to a match that was started elsewhere (a
    /// Verified match starts through `startMatch` with committed decks, not
    /// through `startManabrewMatch`).
    #[wasm_bindgen(js_name = manabrewAttach)]
    pub fn manabrew_attach(&mut self, game_id: String, human_players: Vec<u8>, verified: bool) {
        let count = self.manabrew_game().players.len();
        self.manabrew_human_players = (0..count)
            .map(|index| human_players.is_empty() || human_players.contains(&(index as u8)))
            .collect();
        self.manabrew_game_id = if game_id.trim().is_empty() {
            "ironsmith-verified".to_string()
        } else {
            game_id
        };
        self.manabrew_next_prompt_id = 1;
        self.manabrew_open_prompt = None;
        self.manabrew_forced_command = None;
        self.manabrew_verified = verified;
    }

    /// Verified matches replicate every engine change through a signed
    /// transcript: the adapter stops answering forced decisions itself and the
    /// host turns responses into commands with `manabrewTranslate`.
    #[wasm_bindgen(js_name = manabrewSetVerified)]
    pub fn manabrew_set_verified(&mut self, enabled: bool) {
        self.manabrew_verified = enabled;
        if !enabled {
            self.manabrew_forced_command = None;
        }
    }

    /// Translate a Manabrew prompt response into the engine command it means,
    /// without applying it. The host submits `dispatch` commands through its
    /// own pipeline (signing, quorum, replay on every seat) and then asks for
    /// the next view. `continue` answers need no command: the prompt advanced
    /// locally.
    #[wasm_bindgen(js_name = manabrewTranslate)]
    pub fn manabrew_translate(
        &mut self,
        player: u8,
        prompt_id: u32,
        output: JsValue,
    ) -> Result<JsValue, JsValue> {
        let _menu_policy = ManabrewMenuPolicy::enter(&self.manabrew_human_players);
        let player = match self.manabrew_player(player) {
            Ok(player) => player,
            Err(error) => {
                let view = self.manabrew_result(None, Some(error));
                return manabrew_to_js(&ManabrewTranslation::Rejected { view }, "Manabrew translation");
            }
        };
        if let Err(error) = self.validate_manabrew_prompt_owner(player, prompt_id) {
            let view = self.manabrew_result(Some(player), Some(error));
            return manabrew_to_js(&ManabrewTranslation::Rejected { view }, "Manabrew translation");
        }
        let output: PromptOutput = match serde_wasm_bindgen::from_value(output) {
            Ok(output) => output,
            Err(error) => {
                let view = self.manabrew_result(
                    Some(player),
                    Some(protocol_error(
                        ProtocolErrorCode::InvalidShape,
                        format!("invalid Manabrew prompt output: {error}"),
                        Some(prompt_id),
                    )),
                );
                return manabrew_to_js(&ManabrewTranslation::Rejected { view }, "Manabrew translation");
            }
        };
        let open = match self.validate_manabrew_response(player, prompt_id, &output) {
            Ok(open) => open.clone(),
            Err(error) => {
                let view = self.manabrew_result(Some(player), Some(error));
                return manabrew_to_js(&ManabrewTranslation::Rejected { view }, "Manabrew translation");
            }
        };
        let action = match self.manabrew_response_action(&open, output) {
            Ok(action) => action,
            Err(error) => {
                let view = self.manabrew_result(Some(player), Some(error));
                return manabrew_to_js(&ManabrewTranslation::Rejected { view }, "Manabrew translation");
            }
        };
        let translation = match action {
            ManabrewResponseAction::Dispatch(command) => match serde_json::to_value(&command) {
                Ok(command) => ManabrewTranslation::Dispatch { command },
                Err(error) => {
                    let view = self.manabrew_result(
                        Some(player),
                        Some(protocol_error(
                            ProtocolErrorCode::InvalidShape,
                            format!("could not encode the Ironsmith command: {error}"),
                            Some(prompt_id),
                        )),
                    );
                    ManabrewTranslation::Rejected { view }
                }
            },
            ManabrewResponseAction::Cancel => ManabrewTranslation::Cancel,
            ManabrewResponseAction::Continue { input, binding } => {
                let next_prompt_id = self.manabrew_next_prompt_id;
                let Some(after_next_prompt_id) = next_prompt_id.checked_add(1) else {
                    let view = self.manabrew_result(
                        Some(player),
                        Some(protocol_error(
                            ProtocolErrorCode::InvalidShape,
                            "Manabrew prompt ID space is exhausted",
                            Some(prompt_id),
                        )),
                    );
                    return manabrew_to_js(
                        &ManabrewTranslation::Rejected { view },
                        "Manabrew translation",
                    );
                };
                self.manabrew_next_prompt_id = after_next_prompt_id;
                self.manabrew_open_prompt = Some(ManabrewOpenPrompt {
                    prompt_id: next_prompt_id,
                    deciding_player: open.deciding_player,
                    decision_hash: open.decision_hash,
                    source_card_id: open.source_card_id,
                    source_card: open.source_card,
                    input,
                    binding,
                });
                let view = self.manabrew_result(Some(player), None);
                ManabrewTranslation::Continue { view }
            }
        };
        manabrew_to_js(&translation, "Manabrew translation")
    }

    #[wasm_bindgen(js_name = manabrewApplyDirective)]
    pub fn manabrew_apply_directive(
        &mut self,
        player: u8,
        directive: JsValue,
    ) -> Result<JsValue, JsValue> {
        let _menu_policy = ManabrewMenuPolicy::enter(&self.manabrew_human_players);
        let player_id = match self.manabrew_player(player) {
            Ok(player) => player,
            Err(error) => {
                let result = self.manabrew_result(None, Some(error));
                return manabrew_to_js(&result, "Manabrew directive result");
            }
        };
        let directive: DirectiveInput = match serde_wasm_bindgen::from_value(directive) {
            Ok(directive) => directive,
            Err(error) => {
                let result = self.manabrew_result(
                    Some(player_id),
                    Some(protocol_error(
                        ProtocolErrorCode::InvalidShape,
                        format!("invalid Manabrew directive: {error}"),
                        None,
                    )),
                );
                return manabrew_to_js(&result, "Manabrew directive result");
            }
        };
        match directive {
            DirectiveInput::Concede => {
                if let Err(error) = self.forfeit_player(player) {
                    let result = self.manabrew_result(
                        Some(player_id),
                        Some(protocol_error(
                            ProtocolErrorCode::InvalidShape,
                            error
                                .as_string()
                                .unwrap_or_else(|| "concession was rejected".to_string()),
                            None,
                        )),
                    );
                    return manabrew_to_js(&result, "Manabrew directive result");
                }
            }
            // Checkpoint restore, snapshot recording and action repeat belong
            // to engines that journal decisions. Ironsmith publishes no
            // checkpoints (`GameViewDto::checkpoints` stays empty), so these
            // are refused without disturbing the open prompt.
            DirectiveInput::RequestRestore { .. }
            | DirectiveInput::RestoreVote { .. }
            | DirectiveInput::SetSnapshotRecording { .. }
            | DirectiveInput::StartRecording
            | DirectiveInput::StopRecording
            | DirectiveInput::RequestRepeat { .. }
            | DirectiveInput::RepeatVote { .. } => {
                let result = self.manabrew_result(
                    Some(player_id),
                    Some(protocol_error(
                        ProtocolErrorCode::InvalidShape,
                        "the Ironsmith runtime does not support this directive",
                        None,
                    )),
                );
                return manabrew_to_js(&result, "Manabrew directive result");
            }
        }
        self.manabrew_open_prompt = None;
        let result = self.manabrew_result(Some(player_id), None);
        manabrew_to_js(&result, "Manabrew directive result")
    }
}

#[cfg(test)]
mod manabrew_tests {
    use super::*;

    fn game() -> WasmGame {
        let mut game = WasmGame::new();
        game.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 42);
        game
    }

    fn valid_brawl_decks() -> (Vec<Vec<String>>, Vec<Vec<String>>) {
        (
            vec![vec!["Plains".to_string(); 59]; 2],
            vec![vec!["Geist of Saint Traft".to_string()]; 2],
        )
    }

    #[test]
    fn manabrew_brawl_import_uses_distinct_format_and_rules_life() {
        let two_player = ManabrewMatchConfigInput {
            protocol_version: Some(manabrew_protocol_major()),
            player_names: vec!["Alice".to_string(), "Bob".to_string()],
            starting_life: 40,
            seed: Some(7),
            game_id: None,
            human_players: Vec::new(),
            bot_players: Vec::new(),
            format: Some("brawl".to_string()),
            decks: Vec::new(),
            commander_names: Vec::new(),
            opening_hand_size: Some(7),
        };
        let setup = manabrew_match_setup(&two_player);
        assert_eq!(setup.format, MatchFormatInput::Brawl);
        assert_eq!(setup.starting_life, 25);

        let mut multiplayer = two_player;
        multiplayer.player_names.push("Cara".to_string());
        let setup = manabrew_match_setup(&multiplayer);
        assert_eq!(setup.starting_life, 30);
    }

    #[test]
    fn brawl_setup_enforces_construction_and_runtime_profile() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = game();
        let (decks, commanders) = valid_brawl_decks();
        game.validate_brawl_setup(&decks, &commanders)
            .expect("59 Plains plus Geist should be a legal Brawl construction");

        let config = MatchSetupInput {
            player_names: vec!["Alice".to_string(), "Bob".to_string()],
            starting_life: 40,
            seed: 7,
            format: MatchFormatInput::Brawl,
            decks: Some(decks.clone()),
            sideboards: None,
            commanders: Some(commanders.clone()),
            planar_decks: None,
            vanguards: None,
            scheme_decks: None,
            conspiracies: None,
            commander_draft: None,
            opening_hand_size: Some(0),
            hidden_deck_manifests: None,
            public_decklists: None,
            free_for_all: None,
            teams: None,
            starting_player: None,
        };
        game.apply_match_setup(config)
            .expect("legal Brawl match should start");

        assert_eq!(game.match_format, MatchFormatInput::Brawl);
        assert!(game.game.players.iter().all(|player| player.life == 25));
        assert!(!game.game.commander_damage_loss_enabled());
        assert_eq!(
            game.pregame
                .as_ref()
                .expect("pregame should be active")
                .free_mulligan_count(),
            1
        );
        assert!(
            game.game
                .players
                .iter()
                .all(|player| { player.library.len() == 59 && player.commanders.len() == 1 })
        );

        let mut oversized = decks.clone();
        oversized[0].push("Plains".to_string());
        assert!(game.validate_brawl_setup(&oversized, &commanders).is_err());

        let mut duplicate_nonbasic = decks.clone();
        duplicate_nonbasic[0] = vec!["Plains".to_string(); 57];
        duplicate_nonbasic[0].extend(["Sol Ring".to_string(), "Sol Ring".to_string()]);
        assert!(
            game.validate_brawl_setup(&duplicate_nonbasic, &commanders)
                .is_err()
        );

        let mut off_identity = decks;
        off_identity[0] = vec!["Plains".to_string(); 58];
        off_identity[0].push("Lightning Bolt".to_string());
        assert!(
            game.validate_brawl_setup(&off_identity, &commanders)
                .is_err()
        );

        let invalid_commanders = vec![vec!["Sol Ring".to_string()]; 2];
        assert!(
            game.validate_brawl_setup(&valid_brawl_decks().0, &invalid_commanders)
                .is_err()
        );
    }

    fn open_prompt(input: PromptInput, binding: ManabrewPromptBinding) -> ManabrewOpenPrompt {
        ManabrewOpenPrompt {
            prompt_id: 1,
            deciding_player: PlayerId::from_index(0),
            decision_hash: 1,
            source_card_id: None,
            source_card: None,
            input,
            binding,
        }
    }

    fn target_output(targets: &[Target], game: &WasmGame) -> PromptOutput {
        PromptOutput::ChooseBoardTargets(ChooseBoardTargetsOutput::BoardTargets {
            chosen: targets
                .iter()
                .map(|target| protocol_target(&game.game, *target))
                .collect(),
        })
    }

    #[test]
    fn manabrew_target_roles_preserve_order_and_reject_wrong_role() {
        use ironsmith::decisions::context::{TargetRequirementContext, TargetsContext};
        let _guard = crate::test_id_counter_guard();
        let mut game = game();
        let card = ironsmith::card::CardBuilder::new(
            ironsmith::ids::CardId::from_raw(990_011),
            "Target role fixture",
        )
        .build();
        let player = PlayerId::from_index(0);
        let spell = Target::Object(
            game.game
                .create_object_from_card(&card, player, Zone::Stack),
        );
        let permanent = Target::Object(game.game.create_object_from_card(
            &card,
            player,
            Zone::Battlefield,
        ));
        let source = ObjectId::from_raw(990_012);
        let context = DecisionContext::Targets(TargetsContext::new(
            player,
            source,
            "Cryptic Command",
            vec![
                TargetRequirementContext::single("target spell", vec![spell]),
                TargetRequirementContext::single("target permanent", vec![permanent]),
            ],
        ));
        let (input, binding) = game.build_manabrew_prompt(&context).unwrap();
        let open = open_prompt(input, binding);
        let PromptInput::ChooseBoardTargets(input) = &open.input else {
            panic!("target prompt")
        };
        assert_eq!(serde_json::to_value(&input.candidates).unwrap(), serde_json::to_value(vec![protocol_target(&game.game, spell)]).unwrap());
        assert_eq!((input.min_targets, input.max_targets), (1, 1));
        assert!(
            game.manabrew_response_action(&open, target_output(&[permanent], &game))
                .is_err()
        );
        assert!(
            game.manabrew_response_action(&open, target_output(&[], &game))
                .is_err()
        );
        let response = game
            .manabrew_response_action(&open, target_output(&[spell], &game))
            .unwrap();
        let ManabrewResponseAction::Continue { input, binding } = response else {
            panic!("second role required")
        };
        let open = open_prompt(input, binding);
        let PromptInput::ChooseBoardTargets(input) = &open.input else {
            panic!("target prompt")
        };
        assert_eq!(
            serde_json::to_value(&input.candidates).unwrap(),
            serde_json::to_value(vec![protocol_target(&game.game, permanent)]).unwrap()
        );
        let response = game
            .manabrew_response_action(&open, target_output(&[permanent], &game))
            .unwrap();
        let ManabrewResponseAction::Dispatch(command) = response else {
            panic!("complete target command")
        };
        let answer = game.command_to_replay_answer(&context, command).unwrap();
        let ReplayDecisionAnswer::Targets(targets) = answer else {
            panic!("target replay answer")
        };
        assert_eq!(targets, vec![spell, permanent]);
    }

    #[test]
    fn manabrew_target_roles_allow_reuse_except_when_distinct() {
        use ironsmith::decisions::context::TargetRequirementContext;
        let _guard = crate::test_id_counter_guard();
        let game = game();
        let alice = Target::Player(PlayerId::from_index(0));
        let bob = Target::Player(PlayerId::from_index(1));
        for distinct in [false, true] {
            let mut requirement =
                TargetRequirementContext::single("target player", vec![alice, bob]);
            requirement.distinct_player_group = distinct.then_some(0);
            let state = ManabrewTargetState {
                requirements: vec![requirement.clone(), requirement],
                selections: vec![vec![alice]],
                additional_remaining: 0,
                context: "Two targets".into(),
                cancellable: true,
            };
            let (input, binding) = game.manabrew_targets_prompt(state, None).unwrap();
            let open = open_prompt(input, binding);
            let PromptInput::ChooseBoardTargets(input) = &open.input else {
                panic!("target prompt")
            };
            assert_eq!(input.candidates.len(), if distinct { 1 } else { 2 });
            assert!(matches!(
                game.manabrew_response_action(
                    &open,
                    PromptOutput::ChooseBoardTargets(ChooseBoardTargetsOutput::Cancel)
                )
                .unwrap(),
                ManabrewResponseAction::Cancel
            ));
            let selected = if distinct { bob } else { alice };
            assert!(matches!(
                game.manabrew_response_action(&open, target_output(&[selected], &game))
                    .unwrap(),
                ManabrewResponseAction::Dispatch(UiCommand::SelectTargets { .. })
            ));
        }
    }

    #[test]
    fn manabrew_target_roles_share_extra_target_budget() {
        use ironsmith::decisions::context::TargetRequirementContext;
        let _guard = crate::test_id_counter_guard();
        let game = game();
        let alice = Target::Player(PlayerId::from_index(0));
        let bob = Target::Player(PlayerId::from_index(1));
        let mut requirement = TargetRequirementContext::single("target player", vec![alice, bob]);
        requirement.max_targets = Some(2);
        requirement.additional_target_budget = 1;
        let state = ManabrewTargetState {
            requirements: vec![requirement.clone(), requirement],
            selections: Vec::new(),
            additional_remaining: 1,
            context: "Extra target".into(),
            cancellable: false,
        };
        let (input, binding) = game.manabrew_targets_prompt(state, None).unwrap();
        let open = open_prompt(input, binding);
        let response = game
            .manabrew_response_action(&open, target_output(&[alice, bob], &game))
            .unwrap();
        let ManabrewResponseAction::Continue { input, binding } = response else {
            panic!("second role required")
        };
        let open = open_prompt(input, binding);
        let PromptInput::ChooseBoardTargets(input) = &open.input else {
            panic!("target prompt")
        };
        assert_eq!((input.min_targets, input.max_targets), (1, 1));
        assert!(
            game.manabrew_response_action(&open, target_output(&[alice, bob], &game))
                .is_err()
        );
        assert!(matches!(
            game.manabrew_response_action(&open, target_output(&[alice], &game))
                .unwrap(),
            ManabrewResponseAction::Dispatch(UiCommand::SelectTargets { .. })
        ));
    }

    fn combat_assignment_game(deathtouch: bool) -> (WasmGame, DecisionContext, ObjectId, ObjectId) {
        let mut game = game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut make_creature = |name: &str, power, toughness, owner| {
            let id = game.game.new_object_id();
            let card = ironsmith::card::CardBuilder::new(
                ironsmith::ids::CardId::from_raw(id.0 as u32),
                name,
            )
            .card_types(vec![ironsmith::types::CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(power, toughness))
            .build();
            game.game.add_object(ironsmith::object::Object::from_card(
                id,
                &card,
                owner,
                ironsmith::zone::Zone::Battlefield,
            ));
            id
        };
        let attacker = make_creature("Trampler", 4, 6, alice);
        let blocker = make_creature("Blocker", 2, 2, bob);
        let object = game.game.object_mut(attacker).unwrap();
        object
            .abilities_mut()
            .push(ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::trample(),
            ));
        if deathtouch {
            object
                .abilities_mut()
                .push(ironsmith::ability::Ability::static_ability(
                    ironsmith::static_abilities::StaticAbility::deathtouch(),
                ));
        }
        game.game.refresh_continuous_state().unwrap();
        let mut runner = ironsmith::turn_runner::TurnRunner::from_state_for_sync(
            ironsmith::turn_runner::TurnState::CombatDamageRegularAssign,
        );
        runner
            .combat_mut()
            .attackers
            .push(ironsmith::combat_state::AttackerInfo {
                creature: attacker,
                target: AttackTarget::Player(bob),
            });
        runner.combat_mut().blockers.insert(attacker, vec![blocker]);
        let combat = runner
            .current_combat_damage_assignment_prompt(&game.game)
            .unwrap();
        let context = DecisionContext::Distribute(combat.decision_context(&game.game));
        game.runner = Some(runner);
        game.runner_pending_decision = true;
        game.pending_decision = Some(context.clone());
        (game, context, attacker, blocker)
    }

    #[test]
    fn manabrew_combat_damage_uses_one_complete_assignment() {
        let _guard = crate::test_id_counter_guard();
        let (game, context, attacker, blocker) = combat_assignment_game(false);
        let (input, _) = game.build_manabrew_prompt(&context).unwrap();
        let PromptInput::ChooseCombatDamageAssignment(input) = input else {
            panic!("combat damage must not be split into independent number choices");
        };
        assert_eq!(input.attacker_id, object_id(&game.game, attacker));
        assert_eq!(input.blocker_ids, vec![object_id(&game.game, blocker)]);
        assert_eq!(input.defender_id.as_deref(), Some("player-1"));
        assert_eq!(input.total_damage, 4);
        assert!(!input.attacker_has_deathtouch);
    }

    #[test]
    fn manabrew_combat_damage_can_correct_an_illegal_trample_split() {
        let _guard = crate::test_id_counter_guard();
        let (mut game, context, _, blocker) = combat_assignment_game(false);
        let (input, binding) = game.build_manabrew_prompt(&context).unwrap();
        let open = open_prompt(input, binding);
        for (to_blocker, legal) in [(1, false), (2, true), (4, true)] {
            let output = PromptOutput::ChooseCombatDamageAssignment(
                ChooseCombatDamageAssignmentOutput::CombatDamageAssignmentDecision {
                    assignments: vec![
                        manabrew_protocol::prompts::common::CombatDamageAssignmentEntry {
                            assignee_id: object_id(&game.game, blocker),
                            damage: to_blocker,
                        },
                        manabrew_protocol::prompts::common::CombatDamageAssignmentEntry {
                            assignee_id: "player-1".into(),
                            damage: 4 - to_blocker,
                        },
                    ],
                },
            );
            let action = game.manabrew_response_action(&open, output).unwrap();
            let ManabrewResponseAction::Dispatch(command) = action else {
                panic!("the entire assignment must dispatch at once");
            };
            let ReplayDecisionAnswer::Distribute(distribution) =
                game.command_to_replay_answer(&context, command).unwrap()
            else {
                panic!("combat response must remain a distribution");
            };
            assert_eq!(
                game.runner
                    .as_ref()
                    .unwrap()
                    .validate_combat_damage_distribution(&game.game, &distribution)
                    .is_ok(),
                legal
            );
        }
    }

    #[test]
    fn manabrew_combat_damage_preserves_deathtouch() {
        let _guard = crate::test_id_counter_guard();
        let (game, context, _, blocker) = combat_assignment_game(true);
        let (input, _) = game.build_manabrew_prompt(&context).unwrap();
        let PromptInput::ChooseCombatDamageAssignment(input) = input else {
            panic!("combat assignment required");
        };
        assert!(input.attacker_has_deathtouch);
        assert!(
            game.runner
                .as_ref()
                .unwrap()
                .validate_combat_damage_distribution(
                    &game.game,
                    &[
                        (Target::Object(blocker), 1),
                        (Target::Player(PlayerId::from_index(1)), 3)
                    ],
                )
                .is_ok()
        );
    }

    #[test]
    fn deck_sources_keep_loyalty_defense_and_linked_faces() {
        let decks = vec![serde_json::json!({
            "cards": [{
                "identity": { "name": "Daybound Adept" },
                "layout": "transform",
                "cardFaces": [{
                    "name": "Daybound Adept",
                    "manaCost": "{2}{U}",
                    "typeLine": "Legendary Planeswalker — Adept",
                    "loyalty": "4",
                    "oracleText": "+1: Draw a card."
                }, {
                    "name": "Nightbound Adept",
                    "type_line": "Battle — Siege",
                    "defense": 5,
                    "oracle_text": "When this enters, draw a card."
                }]
            }]
        })];

        let sources = manabrew_deck_sources(&decks);
        assert_eq!(sources.len(), 1);
        assert!(!sources[0].replace_existing);
        assert_eq!(sources[0].canonical_name, "Daybound Adept");
        assert!(sources[0].aliases.iter().any(|alias| {
            alias.alias == "Daybound Adept // Nightbound Adept"
                && alias.canonical == "Daybound Adept"
        }));
        let ExternalCardSourceGroup::Linked {
            layout,
            combined_name,
            faces,
            ..
        } = &sources[0].group
        else {
            panic!("two supplied faces should produce a linked source");
        };
        assert_eq!(layout, "transform");
        assert_eq!(combined_name, "Daybound Adept // Nightbound Adept");
        assert!(faces[0].block.contains("Loyalty: 4"));
        assert!(faces[1].block.contains("Defense: 5"));
        assert!(faces[0].block.contains("+1: Draw a card."));
    }

    /// Scryfall-derived deck cards for single-image two-part cards: the host
    /// keeps the front half's name and the card-level joined mana cost.
    #[test]
    fn deck_sources_take_the_front_half_of_a_joined_mana_cost() {
        let decks = vec![serde_json::json!({
            "cards": [{
                "identity": { "name": "Fire" },
                "layout": "split",
                "manaCost": "{1}{R} // {1}{U}",
                "types": ["Instant"],
                "text": "Fire deals 2 damage divided as you choose among one or two targets."
            }]
        })];

        let sources = manabrew_deck_sources(&decks);
        let ExternalCardSourceGroup::Single { name, block, .. } = &sources[0].group else {
            panic!("a front-only half stays a single source");
        };
        assert_eq!(name, "Fire");
        assert!(block.starts_with("Mana cost: {1}{R}\n"), "{block}");
    }

    #[test]
    fn deck_sources_link_single_image_two_part_cards_by_layout() {
        let two_part = |name: &str, layout: &str, back: serde_json::Value| {
            serde_json::json!({
                "identity": { "name": name },
                "layout": layout,
                "manaCost": "{1}{R} // {W}",
                "cardFaces": [{
                    "name": name,
                    "mana_cost": "{1}{R}",
                    "type_line": "Instant",
                    "oracle_text": "Destroy target artifact.\nFuse (You may cast one or both halves of this card from your hand.)"
                }, back]
            })
        };
        let decks = vec![serde_json::json!({
            "cards": [
                two_part("Wear", "split", serde_json::json!({
                    "name": "Tear", "mana_cost": "{W}", "type_line": "Instant",
                    "oracle_text": "Destroy target enchantment.\nFuse (You may cast one or both halves of this card from your hand.)"
                })),
                two_part("Shieldbreaker", "adventure", serde_json::json!({
                    "name": "Battle Display", "mana_cost": "{R}",
                    "type_line": "Sorcery — Adventure", "oracle_text": "Destroy target artifact."
                })),
                two_part("Emeritus", "prepare", serde_json::json!({
                    "name": "Regrowth", "mana_cost": "{1}{G}", "type_line": "Sorcery",
                    "oracle_text": "Return target card from your graveyard to your hand."
                })),
            ]
        })];

        let sources = manabrew_deck_sources(&decks);
        let linked = sources
            .iter()
            .map(|source| {
                let ExternalCardSourceGroup::Linked {
                    layout,
                    combined_name,
                    has_fuse,
                    faces,
                } = &source.group
                else {
                    panic!("two supplied faces should produce a linked source");
                };
                assert!(faces[0].block.starts_with("Mana cost: {1}{R}\n"));
                (layout.as_str(), combined_name.as_str(), *has_fuse)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            linked,
            vec![
                ("split", "Wear // Tear", true),
                ("transform_like", "Shieldbreaker // Battle Display", true),
                ("prepare", "Emeritus // Regrowth", true),
            ]
        );
    }

    #[test]
    fn deck_sources_accept_front_only_manabrew_cards() {
        let decks = vec![serde_json::json!({
            "cards": [{
                "identity": { "name": "Front Only Walker" },
                "manaCost": "{1}{W}",
                "types": ["Planeswalker"],
                "subtypes": ["Test"],
                "loyalty": 3,
                "text": "+1: You gain 1 life."
            }]
        })];

        let sources = manabrew_deck_sources(&decks);
        let ExternalCardSourceGroup::Single { block, .. } = &sources[0].group else {
            panic!("a current Manabrew rules summary should remain a single source");
        };
        assert!(block.contains("Type: Planeswalker — Test"));
        assert!(block.contains("Loyalty: 3"));
    }

    #[test]
    fn state_update_round_trips_and_spectator_hides_private_zones() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = game();
        let card = ironsmith::card::CardBuilder::new(
            ironsmith::ids::CardId::from_raw(990_001),
            "Protocol Test Card",
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
        .build();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.game.create_object_from_card(&card, alice, Zone::Hand);
        game.game.create_object_from_card(&card, bob, Zone::Hand);
        let library_card = game.game.create_object_from_card(&card, bob, Zone::Library);
        let face_down = game.game.create_object_from_card(&card, bob, Zone::Exile);
        game.game.set_face_down(face_down);

        let state = game.manabrew_state(None);
        let encoded = serde_json::to_string(&state).expect("state serializes");
        let decoded: StateUpdate = serde_json::from_str(&encoded).expect("state round trips");
        assert_eq!(decoded.game_view.game_id, "ironsmith-000000000000002a");
        for zone in &decoded.game_view.zones {
            if matches!(zone.zone, ZoneKind::Hand | ZoneKind::Library) {
                assert!(zone.cards.is_empty());
            }
        }
        let hidden_exile = decoded
            .game_view
            .zones
            .iter()
            .find(|zone| zone.zone == ZoneKind::Exile && zone.owner_id == "player-1")
            .expect("Bob exile exists");
        assert!(matches!(
            hidden_exile.cards.as_slice(),
            [CardView::Hidden { .. }]
        ));

        game.active_viewed_cards = Some(ActiveViewedCards {
            acknowledged_by: Vec::new(),
            viewer: bob,
            subject: bob,
            zone: Zone::Library,
            cards: vec![library_card],
            card_stable_ids: stable_ids_for_viewed_cards(&game.game, &[library_card]),
            public: false,
            source: None,
            description: "Look at a library card".to_string(),
        });
        game.active_audit_viewed_cards.push(ActiveViewedCards {
            acknowledged_by: Vec::new(),
            viewer: bob,
            subject: bob,
            zone: Zone::Exile,
            cards: vec![face_down],
            card_stable_ids: stable_ids_for_viewed_cards(&game.game, &[face_down]),
            public: false,
            source: None,
            description: "Look at a face-down exiled card".to_string(),
        });
        let alice_view = game.manabrew_state(Some(alice));
        let alice_hand = alice_view
            .game_view
            .zones
            .iter()
            .find(|zone| zone.zone == ZoneKind::Hand && zone.owner_id == "player-0")
            .expect("Alice hand exists");
        let bob_hand = alice_view
            .game_view
            .zones
            .iter()
            .find(|zone| zone.zone == ZoneKind::Hand && zone.owner_id == "player-1")
            .expect("Bob hand exists");
        assert_eq!(alice_hand.cards.len(), 1);
        assert!(bob_hand.cards.is_empty());
        assert_eq!(bob_hand.count, 1);
        let bob_library = alice_view
            .game_view
            .zones
            .iter()
            .find(|zone| zone.zone == ZoneKind::Library && zone.owner_id == "player-1")
            .expect("Bob library exists");
        assert!(bob_library.cards.is_empty());

        let bob_view = game.manabrew_state(Some(bob));
        let bob_library = bob_view
            .game_view
            .zones
            .iter()
            .find(|zone| zone.zone == ZoneKind::Library && zone.owner_id == "player-1")
            .expect("Bob library exists");
        assert!(matches!(
            bob_library.cards.as_slice(),
            [CardView::Visible(_)]
        ));
        let bob_exile = bob_view
            .game_view
            .zones
            .iter()
            .find(|zone| zone.zone == ZoneKind::Exile && zone.owner_id == "player-1")
            .expect("Bob exile exists");
        assert!(matches!(bob_exile.cards.as_slice(), [CardView::Visible(_)]));
    }

    #[test]
    fn controlled_player_manabrew_visibility_respects_outside_game_exception() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card = ironsmith::card::CardBuilder::new(
            ironsmith::ids::CardId::from_raw(990_002),
            "Controlled Information Probe",
        )
        .build();
        let hand = game.game.create_object_from_card(&card, alice, Zone::Hand);
        let outside = game
            .game
            .create_object_from_card(&card, alice, Zone::OutsideGame);
        let exiled = game.game.create_object_from_card(&card, alice, Zone::Exile);
        game.game.set_face_down(exiled);
        game.game.grant_face_down_exile_view(exiled, alice);

        let control_token = game.game.add_scoped_player_control(bob, alice, None);
        assert!(game.manabrew_card_is_visible(hand, Some(alice)));
        assert!(game.manabrew_card_is_visible(hand, Some(bob)));
        assert!(game.manabrew_card_is_visible(exiled, Some(alice)));
        assert!(game.manabrew_card_is_visible(exiled, Some(bob)));
        assert!(game.manabrew_card_is_visible(outside, Some(alice)));
        assert!(!game.manabrew_card_is_visible(outside, Some(bob)));

        game.game.remove_scoped_player_control(control_token);
        assert!(!game.manabrew_card_is_visible(hand, Some(bob)));
        assert!(!game.manabrew_card_is_visible(exiled, Some(bob)));
    }

    #[test]
    fn prompt_round_trips_and_reuses_id_for_same_decision() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = game();
        game.pending_decision = Some(DecisionContext::Priority(
            ironsmith::decisions::context::PriorityContext::new(&game.game,
                PlayerId::from_index(0),
                vec![LegalAction::PassPriority],
            ).expect("fixture has complete replacement state"),
        ));
        let first = game
            .ensure_manabrew_prompt()
            .expect("prompt maps")
            .expect("prompt exists");
        let second = game
            .ensure_manabrew_prompt()
            .expect("prompt maps")
            .expect("prompt exists");
        assert_eq!(first.prompt_id, second.prompt_id);
        let encoded = serde_json::to_string(&first).expect("prompt serializes");
        let decoded: AgentPrompt = serde_json::from_str(&encoded).expect("prompt round trips");
        assert_eq!(decoded.prompt_id, first.prompt_id);

        game.pending_decision = Some(DecisionContext::Number(
            ironsmith::decisions::context::NumberContext::new(
                PlayerId::from_index(0),
                None,
                0,
                3,
                "Choose a value",
            ),
        ));
        let next = game
            .ensure_manabrew_prompt()
            .expect("number prompt maps")
            .expect("number prompt exists");
        assert!(next.prompt_id > first.prompt_id);
    }

    #[test]
    fn response_validation_rejects_stale_wrong_player_type_and_action() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = game();
        game.pending_decision = Some(DecisionContext::Priority(
            ironsmith::decisions::context::PriorityContext::new(&game.game,
                PlayerId::from_index(0),
                vec![
                    LegalAction::PassPriority,
                    LegalAction::PlayLand {
                        land_id: ObjectId::from_raw(999),
                    },
                ],
            ).expect("fixture has complete replacement state"),
        ));
        let prompt = game
            .ensure_manabrew_prompt()
            .expect("prompt maps")
            .expect("prompt exists");
        let pass = PromptOutput::ChooseAction(ChooseActionOutput::Pass {
            until: None,
            exhaust_stack: false,
        });
        assert_eq!(
            game.validate_manabrew_response(PlayerId::from_index(0), prompt.prompt_id + 1, &pass)
                .expect_err("stale prompt")
                .code,
            ProtocolErrorCode::StalePrompt
        );
        assert_eq!(
            game.validate_manabrew_response(PlayerId::from_index(1), prompt.prompt_id, &pass)
                .expect_err("wrong player")
                .code,
            ProtocolErrorCode::WrongPlayer
        );
        let wrong_type = PromptOutput::ChooseBoolean(ChooseBooleanOutput::Decision { value: true });
        assert_eq!(
            game.validate_manabrew_response(PlayerId::from_index(0), prompt.prompt_id, &wrong_type)
                .expect_err("wrong type")
                .code,
            ProtocolErrorCode::WrongPromptType
        );
        let unknown = PromptOutput::ChooseAction(ChooseActionOutput::Act {
            action_id: "not-advertised".to_string(),
        });
        assert_eq!(
            game.validate_manabrew_response(PlayerId::from_index(0), prompt.prompt_id, &unknown)
                .expect_err("unknown action")
                .code,
            ProtocolErrorCode::UnknownActionId
        );
    }

    #[test]
    fn manabrew_payment_act_replans_and_manual_pay_requires_selected_steps() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = game();
        let alice = PlayerId::from_index(0);
        let land =
            ironsmith::card::CardBuilder::new(ironsmith::ids::CardId::new(), "Protocol Island")
                .card_types(vec![ironsmith::types::CardType::Land])
                .build();
        let land = game
            .game
            .create_object_from_card(&land, alice, Zone::Battlefield);
        game.game
            .object_mut(land)
            .expect("land should exist")
            .abilities_mut()
            .push(ironsmith::ability::Ability::mana(
                ironsmith::cost::TotalCost::from_cost(ironsmith::costs::Cost::tap()),
                vec![ironsmith::mana::ManaSymbol::Blue],
            ));
        let source = game.game.new_object_id();
        let request = ironsmith::mana_payment::ManaPaymentRequest::new(
            alice,
            source,
            ironsmith::costs::PaymentReason::Effect,
            ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Blue]),
        )
        .with_spend_policy(game.game.mana_spend_policy(alice, Some(source)));
        let plan = ironsmith::mana_payment::plan_mana_payment(&game.game, &request)
            .expect("payment should be plannable")
            .remove(0);
        let context =
            DecisionContext::ManaPayment(ironsmith::decisions::context::ManaPaymentContext::new(
                alice,
                source,
                "Protocol payment",
                request.clone(),
                plan,
            ));
        let (input, binding) = game
            .build_manabrew_prompt(&context)
            .expect("payment should map");
        let PromptInput::PayManaCost(payment) = &input else {
            panic!("payment should use payManaCost");
        };
        assert!(!payment.can_confirm_from_pool);
        let activate_id = payment
            .actions
            .iter()
            .find(|action| matches!(action.kind, PaymentActionKind::ActivateManaAbility(_)))
            .expect("the island activation should be offered")
            .id
            .clone();
        let open = open_prompt(input, binding);
        assert!(
            game.manabrew_response_action(
                &open,
                PromptOutput::PayManaCost(PayManaCostOutput::Pay { auto: false }),
            )
            .is_err(),
            "manual confirmation must not auto-select an advertised activation"
        );
        assert!(matches!(
            game.manabrew_response_action(
                &open,
                PromptOutput::PayManaCost(PayManaCostOutput::Pay { auto: true }),
            )
            .expect("auto pay should confirm the authoritative remainder"),
            ManabrewResponseAction::Dispatch(UiCommand::ManaPayment {
                response: ManaPaymentCommand::Confirm { .. }
            })
        ));

        let ManabrewResponseAction::Dispatch(UiCommand::ManaPayment { response }) = game
            .manabrew_response_action(
                &open,
                PromptOutput::PayManaCost(PayManaCostOutput::Act {
                    action_id: activate_id,
                }),
            )
            .expect("the advertised activation should map to a replan")
        else {
            panic!("Act should dispatch a typed mana-payment command");
        };
        let ironsmith::mana_payment::ManaPaymentResponse::Replan { preferences } = response
            .into_runtime()
            .expect("generated replan constraints should parse")
        else {
            panic!("Act should replan");
        };
        assert_eq!(preferences.required_activations.len(), 1);
        assert_eq!(preferences.required_activations[0].source, land);
        assert_eq!(preferences.required_activations[0].ability_index, 0);

        let mut selected_request = request;
        selected_request.preferences = preferences;
        let selected_plan =
            ironsmith::mana_payment::plan_mana_payment(&game.game, &selected_request)
                .expect("selected payment should remain plannable")
                .remove(0);
        let selected_context =
            DecisionContext::ManaPayment(ironsmith::decisions::context::ManaPaymentContext::new(
                alice,
                source,
                "Protocol payment",
                selected_request,
                selected_plan,
            ));
        game.pending_decision = Some(selected_context.clone());
        let projected_state = game.manabrew_state(Some(alice));
        let projected_alice = projected_state
            .game_view
            .players
            .iter()
            .find(|player| player.id == player_id(alice))
            .expect("Alice should be present in the projected state");
        assert_eq!(
            projected_alice.mana_pool.get(&ManaColor::Blue),
            Some(&1),
            "selected activations should be reflected in the protocol mana pool"
        );
        let (selected_input, selected_binding) = game
            .build_manabrew_prompt(&selected_context)
            .expect("selected payment should map");
        let PromptInput::PayManaCost(selected_payment) = &selected_input else {
            panic!("selected payment should use payManaCost");
        };
        assert!(selected_payment.can_confirm_from_pool);
        let undo_id = selected_payment
            .actions
            .iter()
            .find(|action| matches!(action.kind, PaymentActionKind::UndoMana { .. }))
            .expect("the selected activation should be reversible")
            .id
            .clone();
        let selected_open = open_prompt(selected_input, selected_binding);
        assert!(matches!(
            game.manabrew_response_action(
                &selected_open,
                PromptOutput::PayManaCost(PayManaCostOutput::Pay { auto: false }),
            )
            .expect("manual confirmation should now be legal"),
            ManabrewResponseAction::Dispatch(UiCommand::ManaPayment {
                response: ManaPaymentCommand::Confirm { .. }
            })
        ));
        let ManabrewResponseAction::Dispatch(UiCommand::ManaPayment { response }) = game
            .manabrew_response_action(
                &selected_open,
                PromptOutput::PayManaCost(PayManaCostOutput::Act { action_id: undo_id }),
            )
            .expect("undo should map to a replan")
        else {
            panic!("undo should dispatch a typed mana-payment command");
        };
        let ironsmith::mana_payment::ManaPaymentResponse::Replan { preferences } = response
            .into_runtime()
            .expect("generated undo constraints should parse")
        else {
            panic!("undo should replan");
        };
        assert!(preferences.required_activations.is_empty());
    }

    #[test]
    fn known_card_name_input_uses_bounded_selection_prompts_and_dispatches_text() {
        let _id_guard = crate::test_id_counter_guard();
        let mut game = game();
        game.external_parse_sources.insert(
            "protocol test card".to_string(),
            ("Protocol Test Card".to_string(), String::new()),
        );
        let context = DecisionContext::TextInput(
            ironsmith::decisions::context::TextInputContext::new(
                PlayerId::from_index(0),
                None,
                "Choose a card name",
            )
            .require_known_value(true),
        );
        assert!(game.build_manabrew_prompt(&context).is_ok());

        let target = "Card 219".to_string();
        let names = (0..250).map(|index| format!("Card {index:03}")).collect();
        let (input, binding) =
            WasmGame::manabrew_text_name_prompt("Choose a card name".to_string(), None, names);
        let mut open = open_prompt(input, binding);
        for _ in 0..4 {
            let PromptInput::ChooseFromSelection(selection) = &open.input else {
                panic!("card-name navigation should use selection prompts");
            };
            assert!(
                selection.options.len() <= MANABREW_TEXT_OPTIONS_PER_PROMPT,
                "card-name prompt exceeded its option bound"
            );
            assert!(
                selection
                    .options
                    .iter()
                    .all(|option| option.weight == 1 && !option.can_repeat),
                "card-name choices should use unit-weight, non-repeatable protocol options"
            );
            let chosen_index = match &open.binding {
                ManabrewPromptBinding::TextNameGroups { groups, .. } => groups
                    .iter()
                    .position(|group| group.contains(&target))
                    .expect("target range exists"),
                ManabrewPromptBinding::TextNames { names } => names
                    .iter()
                    .position(|name| name == &target)
                    .expect("target name exists"),
                _ => panic!("unexpected text-name binding"),
            };
            let output =
                PromptOutput::ChooseFromSelection(ChooseFromSelectionOutput::SelectionDecision {
                    chosen_indices: vec![chosen_index],
                });
            match game
                .manabrew_response_action(&open, output)
                .expect("text response maps")
            {
                ManabrewResponseAction::Continue { input, binding } => {
                    open.prompt_id += 1;
                    open.input = input;
                    open.binding = binding;
                }
                ManabrewResponseAction::Dispatch(UiCommand::TextChoice { value }) => {
                    assert_eq!(value, target);
                    return;
                }
                _ => panic!("unexpected text-name response action"),
            }
        }
        panic!("card-name selection did not terminate");
    }

    #[test]
    fn distribution_uses_number_sequence_and_dispatches_repeated_indices() {
        let _id_guard = crate::test_id_counter_guard();
        let game = game();
        let context =
            DecisionContext::Distribute(ironsmith::decisions::context::DistributeContext::new(
                PlayerId::from_index(0),
                None,
                "Distribute 3 damage",
                3,
                vec![
                    ironsmith::decisions::context::DistributeTarget {
                        target: Target::Player(PlayerId::from_index(0)),
                        name: "Alice".to_string(),
                    },
                    ironsmith::decisions::context::DistributeTarget {
                        target: Target::Player(PlayerId::from_index(1)),
                        name: "Bob".to_string(),
                    },
                ],
                1,
            ));
        let (input, binding) = game.build_manabrew_prompt(&context).expect("prompt maps");
        let open = open_prompt(input, binding);
        let first = game
            .manabrew_response_action(
                &open,
                PromptOutput::ChooseNumber(ChooseNumberOutput::NumberDecision {
                    chosen_number: Some(2),
                }),
            )
            .expect("first allocation maps");
        let ManabrewResponseAction::Continue { input, binding } = first else {
            panic!("first allocation should continue");
        };
        let mut open = open_prompt(input, binding);
        open.prompt_id = 2;
        match game
            .manabrew_response_action(
                &open,
                PromptOutput::ChooseNumber(ChooseNumberOutput::NumberDecision {
                    chosen_number: Some(1),
                }),
            )
            .expect("final allocation maps")
        {
            ManabrewResponseAction::Dispatch(UiCommand::SelectOptions { option_indices }) => {
                assert_eq!(option_indices, vec![0, 0, 1]);
            }
            _ => panic!("final allocation should dispatch"),
        }
    }

    #[test]
    fn counter_removal_uses_number_sequence_and_dispatches_counted_allocations() {
        let _id_guard = crate::test_id_counter_guard();
        let game = game();
        let context =
            DecisionContext::Counters(ironsmith::decisions::context::CountersContext::new(
                PlayerId::from_index(0),
                None,
                ironsmith::Target::Object(ObjectId::from_raw(900_001)),
                "Counter Test Permanent",
                0,
                3,
                vec![
                    (ironsmith::object::CounterType::PlusOnePlusOne, 2),
                    (ironsmith::object::CounterType::Named("charge".into()), 3),
                ],
            ));
        let (input, binding) = game.build_manabrew_prompt(&context).expect("prompt maps");
        let open = open_prompt(input, binding);
        let first = game
            .manabrew_response_action(
                &open,
                PromptOutput::ChooseNumber(ChooseNumberOutput::NumberDecision {
                    chosen_number: Some(1),
                }),
            )
            .expect("first counter allocation maps");
        let ManabrewResponseAction::Continue { input, binding } = first else {
            panic!("first counter allocation should continue");
        };
        let mut open = open_prompt(input, binding);
        open.prompt_id = 2;
        match game
            .manabrew_response_action(
                &open,
                PromptOutput::ChooseNumber(ChooseNumberOutput::NumberDecision {
                    chosen_number: Some(2),
                }),
            )
            .expect("final counter allocation maps")
        {
            ManabrewResponseAction::Dispatch(UiCommand::SelectCounters { allocations }) => {
                assert_eq!(allocations, vec![CounterAllocation { index: 0, count: 1 }, CounterAllocation { index: 1, count: 2 }]);
            }
            _ => panic!("final counter allocation should dispatch"),
        }
    }

    #[test]
    fn proliferate_maps_to_optional_multi_selection() {
        let _id_guard = crate::test_id_counter_guard();
        let game = game();
        let context =
            DecisionContext::Proliferate(ironsmith::decisions::context::ProliferateContext::new(
                PlayerId::from_index(0),
                None,
                vec![(ObjectId::from_raw(900_002), "Permanent".to_string())],
                vec![(PlayerId::from_index(1), "Bob".to_string())],
            ));
        let (input, binding) = game.build_manabrew_prompt(&context).expect("prompt maps");
        let PromptInput::ChooseFromSelection(selection) = &input else {
            panic!("proliferate should use a selection prompt");
        };
        assert_eq!(selection.min_total, 0);
        assert_eq!(selection.max_total, 2);
        assert_eq!(
            selection
                .options
                .iter()
                .map(|option| option.label.as_str())
                .collect::<Vec<_>>(),
            vec!["Permanent: Permanent", "Player: Bob"]
        );
        let open = open_prompt(input, binding);
        match game
            .manabrew_response_action(
                &open,
                PromptOutput::ChooseFromSelection(ChooseFromSelectionOutput::SelectionDecision {
                    chosen_indices: vec![0, 1],
                }),
            )
            .expect("proliferate response maps")
        {
            ManabrewResponseAction::Dispatch(UiCommand::SelectOptions { option_indices }) => {
                assert_eq!(option_indices, vec![0, 1]);
            }
            _ => panic!("proliferate should dispatch directly"),
        }
    }

    /// Bolas's Citadel: its controller sees the top of their library at all
    /// times and is offered that card from the library. The opponent sees
    /// neither.
    #[test]
    fn bolass_citadel_shows_and_offers_the_library_top() {
        use ironsmith_registry_test::compile_to_runtime_definition;
        let _id_guard = crate::test_id_counter_guard();
        let compile = |name: &str, text: &str| {
            compile_to_runtime_definition(name, text, false)
                .unwrap_or_else(|error| panic!("{name}: {error:?}"))
        };
        let citadel = compile(
            "Bolas's Citadel",
            "Mana cost: {3}{B}{B}{B}\nType: Legendary Artifact\nYou may look at the top card of your library any time.\nYou may play lands and cast spells from the top of your library. If you cast a spell this way, pay life equal to its mana value rather than pay its mana cost.\n{T}, Sacrifice ten nonland permanents: Each opponent loses 10 life.",
        );
        let bears = compile(
            "Grizzly Bears",
            "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2",
        );
        let mut game = game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.manabrew_human_players = vec![true, true];
        game.game.turn.active_player = alice;
        game.game.turn.priority_player = Some(alice);
        game.game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.game.turn.step = None;
        game.game.create_object_from_definition(&citadel, alice, Zone::Battlefield);
        game.game.create_object_from_definition(&bears, alice, Zone::Library);
        let top = game.game.create_object_from_definition(&bears, alice, Zone::Library);
        game.game.refresh_continuous_state().unwrap();
        let top_id = object_id(&game.game, top);

        let library = |game: &WasmGame, viewer: PlayerId| {
            game.manabrew_state(Some(viewer))
                .game_view
                .zones
                .into_iter()
                .find(|zone| zone.zone == ZoneKind::Library && zone.owner_id == "player-0")
                .expect("Alice library exists")
        };
        let alice_library = library(&game, alice);
        assert_eq!(alice_library.count, 2);
        assert!(
            matches!(alice_library.cards.as_slice(), [CardView::Visible(card)] if card.id == top_id),
            "Alice should see exactly her library top: {:?}",
            alice_library.cards
        );
        assert!(library(&game, bob).cards.is_empty(), "Bob must not see Alice's top");

        let _menu_policy = ManabrewMenuPolicy::enter(&game.manabrew_human_players);
        let context = ironsmith::game_loop::priority_context(&game.game, alice)
            .expect("priority menu computes");
        game.pending_decision = Some(DecisionContext::Priority(context));
        let prompt = game
            .ensure_manabrew_prompt()
            .expect("prompt maps")
            .expect("prompt exists");
        let PromptInput::ChooseAction(choose) = &prompt.input else {
            panic!("priority should map to chooseAction");
        };
        let cast_id = choose
            .actions
            .iter()
            .find(|action| matches!(
                &action.kind,
                AvailableActionKind::Cast { card_id, .. } if *card_id == top_id
            ))
            .unwrap_or_else(|| panic!("the library top should be castable: {:?}", choose.actions))
            .id
            .clone();

        // Taking the offer casts the card for 2 life (its mana value).
        let mut output = PromptOutput::ChooseAction(ChooseActionOutput::Act { action_id: cast_id });
        for _ in 0..6 {
            let open = game.manabrew_open_prompt.clone().expect("a prompt is open");
            let ManabrewResponseAction::Dispatch(command) = game
                .manabrew_response_action(&open, output)
                .expect("response maps")
            else {
                panic!("response should dispatch an engine command");
            };
            game.dispatch_typed_command(command, 0.0).expect("engine accepts the response");
            game.manabrew_open_prompt = None;
            if game.manabrew_game().stack.len() == 1 {
                break;
            }
            let prompt = game
                .ensure_manabrew_prompt()
                .expect("prompt maps")
                .expect("the cast is still being announced");
            output = match &prompt.input {
                PromptInput::PayManaCost(_) => {
                    PromptOutput::PayManaCost(PayManaCostOutput::Pay { auto: true })
                }
                other => panic!("unexpected prompt while casting: {other:?}"),
            };
        }
        let game_state = game.manabrew_game();
        assert_eq!(game_state.stack.len(), 1, "Grizzly Bears should be on the stack");
        assert_eq!(
            game_state.object(game_state.stack[0].object_id).map(|object| object.name.to_string()),
            Some("Grizzly Bears".to_string())
        );
        assert_eq!(game_state.player(alice).unwrap().life, 18, "Citadel pays life, not mana");
        let next_top = object_id(game_state, *game_state.player(alice).unwrap().library.last().unwrap());
        let alice_library = library(&game, alice);
        assert!(
            matches!(alice_library.cards.as_slice(), [CardView::Visible(card)] if card.id == next_top),
            "the new top is visible once the cast is done: {:?}",
            alice_library.cards
        );
    }
}

#[cfg(test)]
mod counter_transport_boundary_contract_tests {
    use super::*;
    fn context(min:u64,max:u64,amount:u32)->DecisionContext {
        DecisionContext::Counters(ironsmith::decisions::context::CountersContext::new_wide(
            PlayerId::from_index(0),None,ironsmith::Target::Object(ObjectId::from_raw(900001)),"Counter transport owner",min,max,
            vec![(ironsmith::object::CounterType::Charge,amount),(ironsmith::object::CounterType::PlusOnePlusOne,amount)]))
    }
    fn game()->WasmGame {let mut game=WasmGame::new();game.initialize_empty_match(vec!["Alice".into(),"Bob".into()],20,42);game}
    fn prompt(input:PromptInput,binding:ManabrewPromptBinding)->ManabrewOpenPrompt {
        ManabrewOpenPrompt {prompt_id:1,deciding_player:PlayerId::from_index(0),decision_hash:1,source_card:None,input,binding,source_card_id:None}
    }
    #[test]
    fn snapshot_preserves_actual_counter_minimum() {
        let _guard=crate::test_id_counter_guard();let game=game();let ctx=context(3,5,4);
        let view=DecisionView::from_context(&game.game,&ctx,PlayerId::from_index(0),None,None);
        let json=serde_json::to_value(view).unwrap();assert_eq!(json["min_total"],serde_json::json!("3"));assert_eq!(json["max_total"],serde_json::json!("5"));
    }
    #[test]
    fn snapshot_preserves_wide_counter_budget_without_pointer_width() {
        let _guard=crate::test_id_counter_guard();let game=game();let maximum=2*u64::from(u32::MAX);let ctx=context(maximum,maximum,u32::MAX);
        let view=DecisionView::from_context(&game.game,&ctx,PlayerId::from_index(0),None,None);
        let json=serde_json::to_value(view).unwrap();assert_eq!(json["max_total"],serde_json::json!(maximum.to_string()));assert_eq!(json["min_total"],serde_json::json!(maximum.to_string()));
    }
    #[test]
    fn manabrew_counter_prompt_preserves_required_total() {
        let _guard=crate::test_id_counter_guard();let game=game();let ctx=context(7,7,4);let (input,_)=game.build_manabrew_prompt(&ctx).unwrap();
        let PromptInput::ChooseNumber(number)=input else{panic!("counter quantity prompt required")};assert_eq!(number.min,3);assert_eq!(number.max,4);
    }
    #[test]
    fn manabrew_unsigned_quantity_requires_residual_chunk() {
        let _guard=crate::test_id_counter_guard();let game=game();let ctx=DecisionContext::Counters(ironsmith::decisions::context::CountersContext::new_wide(PlayerId::from_index(0),None,ironsmith::Target::Object(ObjectId::from_raw(900001)),"Unsigned counter owner",u64::from(u32::MAX),u64::from(u32::MAX),vec![(ironsmith::object::CounterType::Charge,u32::MAX)]));
        let (input,_)=game.build_manabrew_prompt(&ctx).unwrap();let PromptInput::ChooseNumber(number)=input else{panic!("counter quantity prompt required")};assert_eq!(number.min,i32::MAX);assert_eq!(number.max,i32::MAX);
    }
    #[test]
    fn manabrew_final_response_is_counted_without_index_expansion() {
        let _guard=crate::test_id_counter_guard();let game=game();let ctx=context(0,3,3);let (input,binding)=game.build_manabrew_prompt(&ctx).unwrap();let open=prompt(input,binding);
        let first=game.manabrew_response_action(&open,PromptOutput::ChooseNumber(ChooseNumberOutput::NumberDecision{chosen_number:Some(1)})).unwrap();let ManabrewResponseAction::Continue{input,binding}=first else{panic!("second counter kind required")};let open=prompt(input,binding);
        let last=game.manabrew_response_action(&open,PromptOutput::ChooseNumber(ChooseNumberOutput::NumberDecision{chosen_number:Some(2)})).unwrap();let ManabrewResponseAction::Dispatch(command)=last else{panic!("completed allocation must dispatch")};let json=serde_json::to_value(command).unwrap();assert_eq!(json["type"],"select_counters");assert_eq!(json["allocations"],serde_json::json!([{"index":0,"count":1},{"index":1,"count":2}]));
    }
}

#[cfg(test)]
mod sparse_counter_transport_execution_tests {
    use super::*;
    fn context(min:u64,max:u64)->ironsmith::decisions::context::CountersContext {
        ironsmith::decisions::context::CountersContext::new_wide(PlayerId::from_index(0),None,ironsmith::Target::Object(ObjectId::from_raw(900001)),"Sparse counter execution",min,max,vec![(ironsmith::object::CounterType::Charge,u32::MAX),(ironsmith::object::CounterType::PlusOnePlusOne,u32::MAX)])
    }
    #[test]
    fn sparse_backend_retains_wide_total_and_authored_order() {
        let _guard=crate::test_id_counter_guard();let mut game=WasmGame::new();let maximum=2*u64::from(u32::MAX);let ctx=DecisionContext::Counters(context(maximum,maximum));
        let command:UiCommand=serde_json::from_value(serde_json::json!({"type":"select_counters","allocations":[{"index":1,"count":u32::MAX},{"index":0,"count":u32::MAX}]})).unwrap();
        let retained_command:UiCommand=serde_json::from_str(&serde_json::to_string(&command).unwrap()).unwrap();let answer=game.command_to_replay_answer(&ctx,command).unwrap();let ReplayDecisionAnswer::Counters(values)=answer else{panic!("counter answer required")};assert_eq!(values,vec![(ironsmith::object::CounterType::PlusOnePlusOne,u32::MAX),(ironsmith::object::CounterType::Charge,u32::MAX)]);
        let replay=game.command_to_replay_answer(&ctx,retained_command).unwrap();let ReplayDecisionAnswer::Counters(restored)=replay else{panic!("restored counter answer required")};assert_eq!(restored,values);
    }
    #[test]
    fn sparse_backend_rejects_bad_bounds_duplicates_indices_and_capacity() {
        let ctx=context(3,5);assert!(validate_counter_allocations(&ctx,&[CounterAllocation{index:0,count:2}]).is_err());assert!(validate_counter_allocations(&ctx,&[CounterAllocation{index:0,count:6}]).is_err());assert!(validate_counter_allocations(&ctx,&[CounterAllocation{index:2,count:3}]).is_err());assert!(validate_counter_allocations(&ctx,&[CounterAllocation{index:0,count:3},CounterAllocation{index:0,count:0}]).is_err());
        let mut limited=context(0,5);limited.available_counters[0].1=2;assert!(validate_counter_allocations(&limited,&[CounterAllocation{index:0,count:3}]).is_err());assert_eq!(validate_counter_allocations(&ctx,&[CounterAllocation{index:0,count:3}]).unwrap(),vec![(ironsmith::object::CounterType::Charge,3)]);
    }
    #[test]
    fn bounded_legacy_counter_response_preserves_first_selection_order() {
        let _guard=crate::test_id_counter_guard();let mut game=WasmGame::new();let ctx=DecisionContext::Counters(context(3,3));let answer=game.command_to_replay_answer(&ctx,UiCommand::SelectOptions{option_indices:vec![1,0,1]}).unwrap();let ReplayDecisionAnswer::Counters(values)=answer else{panic!("counter answer required")};assert_eq!(values,vec![(ironsmith::object::CounterType::PlusOnePlusOne,2),(ironsmith::object::CounterType::Charge,1)]);
    }
    #[test]
    fn manabrew_reaches_every_unsigned_boundary_without_expansion() {
        let _guard=crate::test_id_counter_guard();let game=WasmGame::new();
        for required in [false,true] {for quantity in [0,i32::MAX as u32-1,i32::MAX as u32,i32::MAX as u32+1,2*(i32::MAX as u32),u32::MAX] {
            let ctx=DecisionContext::Counters(ironsmith::decisions::context::CountersContext::new_wide(PlayerId::from_index(0),None,ironsmith::Target::Object(ObjectId::from_raw(900001)),"Unsigned chunk owner",if required{u64::from(quantity)}else{0},u64::from(u32::MAX),vec![(ironsmith::object::CounterType::Charge,u32::MAX)]));let (mut input,mut binding)=game.build_manabrew_prompt(&ctx).unwrap();let mut remaining=quantity;let mut prompts=0;
            loop {prompts+=1;assert!(prompts<=3);let PromptInput::ChooseNumber(number)=&input else{panic!("number chunk required")};let amount=remaining.min(i32::MAX as u32);assert!(i64::from(amount)>=i64::from(number.min));assert!(i64::from(amount)<=i64::from(number.max));remaining-=amount;
                let open=ManabrewOpenPrompt{prompt_id:prompts,deciding_player:PlayerId::from_index(0),decision_hash:1,source_card_id:None,source_card:None,input,binding};let response=game.manabrew_response_action(&open,PromptOutput::ChooseNumber(ChooseNumberOutput::NumberDecision{chosen_number:Some(amount as i32)})).unwrap();
                match response {ManabrewResponseAction::Continue{input:next,binding:next_binding}=>{input=next;binding=next_binding;},ManabrewResponseAction::Dispatch(UiCommand::SelectCounters{allocations})=>{assert_eq!(remaining,0);assert_eq!(allocations,if quantity==0{vec![]}else{vec![CounterAllocation{index:0,count:quantity}]});break;},_=>panic!("counter chunk completion must remain sparse")}
            }
        }}
    }
    #[test]
    fn manabrew_future_capacity_enforces_remaining_minimum() {
        let _guard=crate::test_id_counter_guard();let game=WasmGame::new();let ctx=DecisionContext::Counters(ironsmith::decisions::context::CountersContext::new_wide(PlayerId::from_index(0),None,ironsmith::Target::Object(ObjectId::from_raw(900001)),"Required residual owner",7,7,vec![(ironsmith::object::CounterType::Charge,4),(ironsmith::object::CounterType::PlusOnePlusOne,4)]));
        let (input,binding)=game.build_manabrew_prompt(&ctx).unwrap();let open=ManabrewOpenPrompt{prompt_id:1,deciding_player:PlayerId::from_index(0),decision_hash:1,source_card_id:None,source_card:None,input,binding};assert!(game.manabrew_response_action(&open,PromptOutput::ChooseNumber(ChooseNumberOutput::NumberDecision{chosen_number:Some(2)})).is_err());
        let response=game.manabrew_response_action(&open,PromptOutput::ChooseNumber(ChooseNumberOutput::NumberDecision{chosen_number:Some(3)})).unwrap();let ManabrewResponseAction::Continue{input,binding}=response else{panic!("residual prompt required")};let PromptInput::ChooseNumber(number)=&input else{panic!("residual number required")};assert_eq!(number.min,4);assert_eq!(number.max,4);let open=ManabrewOpenPrompt{prompt_id:2,deciding_player:PlayerId::from_index(0),decision_hash:1,source_card_id:None,source_card:None,input,binding};let response=game.manabrew_response_action(&open,PromptOutput::ChooseNumber(ChooseNumberOutput::NumberDecision{chosen_number:Some(4)})).unwrap();let ManabrewResponseAction::Dispatch(UiCommand::SelectCounters{allocations})=response else{panic!("sparse completion required")};assert_eq!(allocations,vec![CounterAllocation{index:0,count:3},CounterAllocation{index:1,count:4}]);
    }
}

#[cfg(test)]
mod unsigned_number_transport_contract_tests {
    use super::*;
    fn context(min:u32,max:u32)->DecisionContext {DecisionContext::Number(ironsmith::decisions::context::NumberContext::new(PlayerId::from_index(0),None,min,max,"Unsigned legal number"))}
    fn exercise(min:u32,max:u32,chosen:u32) {
        let game=WasmGame::new();let (mut input,mut binding)=game.build_manabrew_prompt(&context(min,max)).unwrap();let mut remaining=chosen;let mut prompts=0;
        loop {prompts+=1;assert!(prompts<=3,"bounded unsigned choice must terminate");let PromptInput::ChooseNumber(number)=&input else{panic!("number prompt required")};let chunk=remaining.min(i32::MAX as u32);assert!(chunk>=number.min as u32);assert!(chunk<=number.max as u32);remaining-=chunk;
            let open=ManabrewOpenPrompt{prompt_id:prompts,deciding_player:PlayerId::from_index(0),decision_hash:1,source_card_id:None,source_card:None,input,binding};let action=game.manabrew_response_action(&open,PromptOutput::ChooseNumber(ChooseNumberOutput::NumberDecision{chosen_number:Some(chunk as i32)})).unwrap();
            match action {ManabrewResponseAction::Continue{input:next,binding:next_binding}=>{input=next;binding=next_binding;},ManabrewResponseAction::Dispatch(command)=>{assert_eq!(remaining,0,"unsigned residual cannot be discarded");let wire=serde_json::to_value(command).unwrap();assert_eq!(wire,serde_json::json!({"type":"number_choice","value":chosen}));let decoded:UiCommand=serde_json::from_value(wire).unwrap();let mut backend=WasmGame::new();let replay=backend.command_to_replay_answer(&context(min,max),decoded).unwrap();assert!(matches!(replay,ReplayDecisionAnswer::Number(value) if value==chosen));break;},ManabrewResponseAction::Cancel=>panic!("a number prompt is never cancelled")}
        }
    }
    #[test] fn bounded_number_control(){let _g=crate::test_id_counter_guard();exercise(2,9,7);}
    #[test] fn optional_unsigned_numbers_reach_storage_maximum(){let _g=crate::test_id_counter_guard();for chosen in [i32::MAX as u32+1,2*(i32::MAX as u32),u32::MAX]{exercise(0,u32::MAX,chosen);}}
    #[test] fn required_unsigned_numbers_retain_minimum(){let _g=crate::test_id_counter_guard();for chosen in [i32::MAX as u32+1,u32::MAX]{exercise(chosen,chosen,chosen);}}
}
