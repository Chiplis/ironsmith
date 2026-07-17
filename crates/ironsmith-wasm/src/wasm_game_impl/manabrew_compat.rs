use std::collections::BTreeMap;

use manabrew_protocol::game::{
    CardDto, CardIdentity, CardView, CombatAssignmentDto, DayTime, GameViewDto, ManaColor,
    PlayerCounterKind, PlayerDto, PlayerStatus, StackObjectDto, StepKind, ZoneDto, ZoneKind,
};
use manabrew_protocol::prompts::choose_attackers::AttackerOptionDto;
use manabrew_protocol::prompts::choose_blockers::BlockableAttackerDto;
use manabrew_protocol::prompts::common::{
    ActivatableAbilityInfo, AttackAssignment, AttackTargetDto, AttackTargetKind, AvailableAction,
    AvailableActionKind, BlockAssignment, PaymentAction, PaymentActionKind, PaymentResourceKind,
    PlayCardMode, PromptPresentation, TargetKind, TargetRef,
};
use manabrew_protocol::prompts::scry::ScryDestination;
use manabrew_protocol::prompts::{
    ChooseActionInput, ChooseActionOutput, ChooseAttackersInput, ChooseAttackersOutput,
    ChooseBlockersInput, ChooseBlockersOutput, ChooseBoardTargetsInput, ChooseBoardTargetsOutput,
    ChooseBooleanInput, ChooseBooleanOutput, ChooseCardsInput, ChooseCardsOutput, ChooseColorInput,
    ChooseColorOutput, ChooseFromSelectionInput, ChooseFromSelectionOutput, ChooseNumberInput,
    ChooseNumberOutput, MulliganInput, MulliganOutput, PayManaCostInput, PayManaCostOutput,
    PromptInput, PromptOutput, ReorderInput, ReorderItem, ReorderOutput, ResponseViolation,
    ScryInput, ScryOutput,
};
use manabrew_protocol::transport::{
    AgentPrompt, DirectiveInput, ProtocolError, ProtocolErrorCode, StateUpdate,
};
use serde_json::Value;

type JsonMap = serde_json::Map<String, Value>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManabrewMatchConfigInput {
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
    state: StateUpdate,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<AgentPrompt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ProtocolError>,
}

enum ManabrewResponseAction {
    Dispatch(UiCommand),
    Continue {
        input: PromptInput,
        binding: ManabrewPromptBinding,
    },
    Cancel,
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

fn manabrew_card_source_block(card: &Value) -> String {
    let card = object_value(card);
    let mut lines = Vec::new();
    if let Some(mana_cost) = scalar_string(first_value(card, &["manaCost", "mana_cost"])) {
        lines.push(format!("Mana cost: {mana_cost}"));
    }
    lines.push(format!("Type: {}", manabrew_type_line(card)));
    let power = scalar_string(card.get("power"));
    let toughness = scalar_string(card.get("toughness"));
    if let (Some(power), Some(toughness)) = (power, toughness) {
        lines.push(format!("Power/Toughness: {power}/{toughness}"));
    }
    if let Some(loyalty) = scalar_string(card.get("loyalty")) {
        lines.push(format!("Loyalty: {loyalty}"));
    }
    if let Some(defense) = scalar_string(card.get("defense")) {
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
    let linked_faces = faces
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

    if linked_faces.len() == 2 {
        let front_name = linked_faces[0].name.clone();
        let combined_name =
            scalar_string(first_value(card_object, &["combinedName", "combined_name"]))
                .or_else(|| deck_name.contains(" // ").then(|| deck_name.clone()))
                .unwrap_or_else(|| format!("{} // {}", linked_faces[0].name, linked_faces[1].name));
        let layout = scalar_string(card_object.get("layout")).unwrap_or_default();
        let layout = if layout.eq_ignore_ascii_case("split") {
            "split"
        } else {
            "transform_like"
        };
        let has_fuse = first_value(card_object, &["hasFuse", "has_fuse"])
            .and_then(Value::as_bool)
            .unwrap_or(false);
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
        free_for_all: None,
        teams: None,
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
    source_card_id: Option<String>,
) -> PromptPresentation {
    PromptPresentation {
        title: title.into(),
        description,
        text: None,
        source_card_id,
        targets: Vec::new(),
    }
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
            let target = match info.target {
                AttackTarget::Player(player) => player_id(player),
                AttackTarget::Planeswalker(object) => object_id(game, object),
                AttackTarget::Battle(object) => object_id(game, object),
            };
            let defending_player = match info.target {
                AttackTarget::Player(player) => Some(player_id(player)),
                AttackTarget::Planeswalker(object) => game
                    .object(object)
                    .map(|object| player_id(game.controller_of(object))),
                AttackTarget::Battle(object) => game.battle_protector(object).map(player_id),
            };
            (true, defending_player, Some(target))
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
        ..CardDto::default()
    })
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
    fn register_manabrew_deck_sources_input(
        &mut self,
        decks: &[Value],
    ) -> ExternalCardRegistrationSummary {
        self.register_external_card_sources_input(ExternalCardSourcesInput::Many(
            manabrew_deck_sources(decks),
        ))
    }

    fn manabrew_player(&self, index: u8) -> Result<PlayerId, ProtocolError> {
        let player = PlayerId::from_index(index);
        self.game.player(player).map(|_| player).ok_or_else(|| {
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
        let Some(object) = self.game.object(id) else {
            return false;
        };
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
                            && viewer == Some(self.game.controlling_player_for(view.viewer))))
            });
        if has_view_permission {
            return true;
        }
        match object.zone {
            Zone::OutsideGame => viewer == Some(object.owner),
            Zone::Hand => {
                viewer == Some(object.owner)
                    || viewer == Some(self.game.controlling_player_for(object.owner))
            }
            Zone::Library => false,
            Zone::Battlefield if self.game.is_face_down(id) => {
                let permanent_controller = self.game.controller_of(object);
                viewer == Some(permanent_controller)
                    || viewer == Some(self.game.controlling_player_for(permanent_controller))
            }
            Zone::Exile if self.game.is_face_down(id) => viewer.is_some_and(|viewer| {
                self.game
                    .can_player_look_at_face_down_exiled_card(id, viewer)
            }),
            _ => true,
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
                    visible_card(&self.game, id)
                } else if omit_hidden {
                    None
                } else {
                    Some(hidden_card(&self.game, id))
                }
            })
            .collect()
    }

    fn manabrew_zones(&self, viewer: Option<PlayerId>) -> Vec<ZoneDto> {
        let mut zones = Vec::new();
        for player in &self.game.players {
            let owner_id = player_id(player.id);
            let battlefield: Vec<_> = self
                .game
                .battlefield
                .iter()
                .copied()
                .filter(|id| {
                    self.game
                        .object(*id)
                        .is_some_and(|object| self.game.controller_of(object) == player.id)
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
                .game
                .exile
                .iter()
                .copied()
                .filter(|id| {
                    self.game
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
                .game
                .command_zone
                .iter()
                .copied()
                .filter(|id| {
                    self.game
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

    fn manabrew_players(&self) -> Vec<PlayerDto> {
        self.game
            .players
            .iter()
            .map(|player| {
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
                    (ManaColor::White, player.mana_pool.white),
                    (ManaColor::Blue, player.mana_pool.blue),
                    (ManaColor::Black, player.mana_pool.black),
                    (ManaColor::Red, player.mana_pool.red),
                    (ManaColor::Green, player.mana_pool.green),
                    (ManaColor::Colorless, player.mana_pool.colorless),
                ]
                .into_iter()
                .collect();
                let commander_damage = player
                    .commander_damage
                    .iter()
                    .map(|(commander, damage)| (object_id(&self.game, *commander), *damage as i32))
                    .collect();
                PlayerDto {
                    id: player_id(player.id),
                    name: player.name.clone(),
                    status: if player.has_left_game {
                        PlayerStatus::Conceded
                    } else if player.has_lost {
                        PlayerStatus::Lost
                    } else {
                        PlayerStatus::Playing
                    },
                    is_human: self
                        .manabrew_human_players
                        .get(player.id.index())
                        .copied()
                        .unwrap_or(true),
                    life: player.life,
                    counters,
                    mana_pool,
                    commander_damage,
                    has_city_blessing: self.game.has_citys_blessing(player.id),
                    ring_level: player.ring_temptations as i32,
                    speed: player.speed.unwrap_or(0) as i32,
                }
            })
            .collect()
    }

    fn manabrew_stack(&self) -> Vec<StackObjectDto> {
        self.game
            .stack
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                let object = self.game.object(entry.object_id)?;
                let chars = self.game.current_characteristics(entry.object_id)?;
                Some(StackObjectDto {
                    id: format!("stack-{}-{index}", object.stable_id.0.0),
                    source_id: object_id(&self.game, entry.object_id),
                    controller_id: player_id(entry.controller),
                    identity: CardIdentity {
                        name: chars.name.to_owned_string(),
                        set_code: String::new(),
                        card_number: String::new(),
                        is_token: object.kind == ironsmith::object::ObjectKind::Token,
                    },
                    text: chars.compiled_card_text.to_string(),
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
                        .map(|target| protocol_target(&self.game, target))
                        .collect(),
                })
            })
            .collect()
    }

    fn manabrew_state(&self, viewer: Option<PlayerId>) -> StateUpdate {
        let combat_assignments = self
            .game
            .combat
            .as_ref()
            .into_iter()
            .flat_map(|combat| {
                combat.blockers.iter().flat_map(|(attacker, blockers)| {
                    blockers.iter().map(|blocker| CombatAssignmentDto {
                        blocker_id: object_id(&self.game, *blocker),
                        attacker_id: object_id(&self.game, *attacker),
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
                turn: self.game.turn.turn_number,
                step: protocol_step(&self.game),
                combat_assignments,
                active_player_id: player_id(self.game.turn.active_player),
                priority_player_id: player_id(
                    self.game
                        .turn
                        .priority_player
                        .unwrap_or(self.game.turn.active_player),
                ),
                players: self.manabrew_players(),
                zones: self.manabrew_zones(viewer),
                stack: self.manabrew_stack(),
                game_over: self.game_over.is_some(),
                winner_id,
                monarch_id: self.game.monarch.map(player_id),
                initiative_holder_id: self.game.initiative.map(player_id),
                day_time: if !self.game.has_day_night {
                    DayTime::Neither
                } else if self.game.is_night {
                    DayTime::Night
                } else {
                    DayTime::Day
                },
            },
        }
    }

    fn manabrew_source_card_id(&self, context: &DecisionContext) -> Option<String> {
        context.source().map(|source| object_id(&self.game, source))
    }

    fn manabrew_action_card(&self, action: &LegalAction) -> Option<ObjectId> {
        use ironsmith::special_actions::SpecialAction;
        match action {
            LegalAction::UsePregameAction { card_id, .. } => Some(*card_id),
            LegalAction::CastSpell { spell_id, .. } => Some(*spell_id),
            LegalAction::ActivateAbility { source, .. }
            | LegalAction::ActivateManaAbility { source, .. } => Some(*source),
            LegalAction::PlayLand { land_id } => Some(*land_id),
            LegalAction::TurnFaceUp { creature_id, .. } => Some(*creature_id),
            LegalAction::SpecialAction(action) => match action {
                SpecialAction::PlayLand { card_id }
                | SpecialAction::Suspend { card_id }
                | SpecialAction::Foretell { card_id }
                | SpecialAction::Plot { card_id }
                | SpecialAction::Companion { card_id } => Some(*card_id),
                SpecialAction::TurnFaceUp { permanent_id, .. }
                | SpecialAction::ActivateManaAbility { permanent_id, .. } => Some(*permanent_id),
                SpecialAction::UnlockRoomDoor { room_id } => Some(*room_id),
                SpecialAction::TurnConspiracyFaceUp { conspiracy_id } => Some(*conspiracy_id),
                SpecialAction::RollPlanarDie => None,
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
        use ironsmith::alternative_cast::CastingMethod;

        let id = format!("action-{index}");
        match action {
            LegalAction::PassPriority
            | LegalAction::KeepOpeningHand
            | LegalAction::TakeMulligan
            | LegalAction::ContinuePregame
            | LegalAction::BeginGame => None,
            LegalAction::CastSpell {
                spell_id,
                casting_method,
                ..
            } => Some(AvailableAction {
                id,
                kind: AvailableActionKind::Cast {
                    card_id: object_id(&self.game, *spell_id),
                    mode: match casting_method {
                        CastingMethod::Normal => PlayCardMode::Normal,
                        CastingMethod::FaceDown => PlayCardMode::StaticAlternative,
                        CastingMethod::SplitOtherHalf => PlayCardMode::RoomRightSplit,
                        _ => PlayCardMode::StaticAlternative,
                    },
                    label: format!(
                        "Cast {}",
                        self.game.current_name(*spell_id).unwrap_or_default()
                    ),
                },
            }),
            LegalAction::PlayLand { land_id } => Some(AvailableAction {
                id,
                kind: AvailableActionKind::Cast {
                    card_id: object_id(&self.game, *land_id),
                    mode: PlayCardMode::Normal,
                    label: format!(
                        "Play {}",
                        self.game.current_name(*land_id).unwrap_or_default()
                    ),
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
                    card_id: object_id(&self.game, *source),
                    ability_index: *ability_index,
                    description: format!("Activate ability {}", ability_index + 1),
                    is_mana_ability: matches!(action, LegalAction::ActivateManaAbility { .. }),
                    cost: None,
                    produced_mana: None,
                }),
            }),
            _ => self
                .manabrew_action_card(action)
                .map(|card| AvailableAction {
                    id,
                    kind: AvailableActionKind::Cast {
                        card_id: object_id(&self.game, card),
                        mode: PlayCardMode::StaticAlternative,
                        label: format!("{action:?}"),
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
                    options: names.clone(),
                    min_choices: 1,
                    max_choices: 1,
                }),
                ManabrewPromptBinding::TextNames { names },
            );
        }

        let chunk_size = names.len().div_ceil(MANABREW_TEXT_OPTIONS_PER_PROMPT);
        let groups = names
            .chunks(chunk_size)
            .map(|group| group.to_vec())
            .collect::<Vec<_>>();
        let options = groups
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
                options,
                min_choices: 1,
                max_choices: 1,
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
                    options: legal_amounts.iter().map(u32::to_string).collect(),
                    min_choices: 1,
                    max_choices: 1,
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
        let max = state
            .available
            .get(state.counter_index)
            .copied()
            .unwrap_or(0)
            .min(state.remaining);
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
                min: 0,
                max: max.min(i32::MAX as u32) as i32,
            }),
            ManabrewPromptBinding::CounterNumber { state },
        ))
    }

    fn build_manabrew_prompt(
        &self,
        context: &DecisionContext,
    ) -> Result<(PromptInput, ManabrewPromptBinding), ProtocolError> {
        let source = self.manabrew_source_card_id(context);
        let unsupported = |name: &str| {
            protocol_error(
                ProtocolErrorCode::InvalidShape,
                format!("Ironsmith decision {name} has no lossless Manabrew v1 prompt mapping"),
                None,
            )
        };
        match context {
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
                        .game
                        .player(ctx.player)
                        .map(|player| {
                            player
                                .hand
                                .iter()
                                .map(|id| object_id(&self.game, *id))
                                .collect()
                        })
                        .unwrap_or_default();
                    return Ok((
                        PromptInput::Mulligan(MulliganInput {
                            hand_card_ids,
                            mulligan_count: 0,
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
                let available = ctx
                    .actions
                    .iter()
                    .enumerate()
                    .filter_map(|(index, action)| {
                        let available = self.manabrew_available_action(index, action)?;
                        actions.insert(available.id.clone(), index);
                        Some(available)
                    })
                    .collect();
                Ok((
                    PromptInput::ChooseAction(ChooseActionInput { actions: available }),
                    ManabrewPromptBinding::Priority {
                        actions,
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
            DecisionContext::Number(ctx) => Ok((
                PromptInput::ChooseNumber(ChooseNumberInput {
                    presentation: presentation(
                        "Choose a number",
                        Some(ctx.description.clone()),
                        source,
                    ),
                    min: ctx.min.min(i32::MAX as u32) as i32,
                    max: ctx.max.min(i32::MAX as u32) as i32,
                }),
                ManabrewPromptBinding::Number,
            )),
            DecisionContext::SelectOptions(ctx) => {
                let legal: Vec<_> = ctx.options.iter().filter(|option| option.legal).collect();
                if ctx.description.to_ascii_lowercase().contains("mana pip") {
                    let mut actions = HashMap::new();
                    let mut payment_actions = Vec::new();
                    let mut pay_index = None;
                    for option in &legal {
                        let Some(pending) = self.priority_state.pending_cast.as_ref() else {
                            break;
                        };
                        let Some(payment) = pending
                            .current_pip_payment_options
                            .iter()
                            .find(|payment| payment.index == option.index)
                        else {
                            continue;
                        };
                        use ironsmith::decision::{AlternativePaymentEffect, ManaPipPaymentAction};
                        match &payment.action {
                            ManaPipPaymentAction::UseFromPool(_) => pay_index = Some(option.index),
                            ManaPipPaymentAction::ActivateManaAbility {
                                source_id,
                                ability_index,
                            } => {
                                let id = format!("payment-{}", option.index);
                                actions.insert(id.clone(), option.index);
                                payment_actions.push(PaymentAction {
                                    id,
                                    kind: PaymentActionKind::ActivateManaAbility(
                                        ActivatableAbilityInfo {
                                            card_id: object_id(&self.game, *source_id),
                                            ability_index: *ability_index,
                                            description: option.description.clone(),
                                            is_mana_ability: true,
                                            cost: None,
                                            produced_mana: None,
                                        },
                                    ),
                                });
                            }
                            ManaPipPaymentAction::PayLife(amount) => {
                                let id = format!("payment-{}", option.index);
                                actions.insert(id.clone(), option.index);
                                payment_actions.push(PaymentAction {
                                    id,
                                    kind: PaymentActionKind::PayLife { amount: *amount },
                                });
                            }
                            ManaPipPaymentAction::PayViaAlternative {
                                permanent_id,
                                effect,
                            } => {
                                let id = format!("payment-{}", option.index);
                                actions.insert(id.clone(), option.index);
                                payment_actions.push(PaymentAction {
                                    id,
                                    kind: PaymentActionKind::UseResource {
                                        card_id: object_id(&self.game, *permanent_id),
                                        resource: match effect {
                                            AlternativePaymentEffect::Convoke => {
                                                PaymentResourceKind::Convoke
                                            }
                                            AlternativePaymentEffect::Improvise => {
                                                PaymentResourceKind::Improvise
                                            }
                                        },
                                    },
                                });
                            }
                        }
                    }
                    if pay_index.is_some() || !payment_actions.is_empty() {
                        let source_id = ctx.source.unwrap_or(ObjectId::from_raw(0));
                        let view = self.current_mana_payment_view();
                        return Ok((
                            PromptInput::PayManaCost(PayManaCostInput {
                                card_id: object_id(&self.game, source_id),
                                card_name: view
                                    .as_ref()
                                    .map(|view| view.source_name.clone())
                                    .unwrap_or_else(|| ctx.description.clone()),
                                mana_cost: view
                                    .as_ref()
                                    .map(|view| {
                                        view.pips
                                            .iter()
                                            .map(|pip| format!("{{{}}}", pip.join("/")))
                                            .collect()
                                    })
                                    .unwrap_or_default(),
                                can_confirm_from_pool: pay_index.is_some(),
                                actions: payment_actions,
                                description: Some(ctx.description.clone()),
                            }),
                            ManabrewPromptBinding::Payment { actions, pay_index },
                        ));
                    }
                    return Err(unsupported(
                        "mana payment without typed Ironsmith payment actions",
                    ));
                }
                Ok((
                    PromptInput::ChooseFromSelection(ChooseFromSelectionInput {
                        presentation: presentation("Choose", Some(ctx.description.clone()), source),
                        options: legal
                            .iter()
                            .map(|option| option.description.clone())
                            .collect(),
                        min_choices: ctx.min,
                        max_choices: ctx.max,
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
                        options: legal.iter().map(|mode| mode.description.clone()).collect(),
                        min_choices: ctx.spec.min_modes,
                        max_choices: ctx.spec.max_modes,
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
                    options: ctx
                        .options
                        .iter()
                        .map(|option| option.label.clone())
                        .collect(),
                    min_choices: 1,
                    max_choices: 1,
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
                    .map(|candidate| (object_id(&self.game, candidate.id), candidate.id))
                    .collect();
                Ok((
                    PromptInput::ChooseCards(ChooseCardsInput {
                        presentation: presentation(
                            "Choose cards",
                            Some(ctx.description.clone()),
                            source,
                        ),
                        cards: legal
                            .iter()
                            .filter_map(|candidate| protocol_card(&self.game, candidate.id))
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
                let indices = ctx
                    .items
                    .iter()
                    .enumerate()
                    .map(|(index, (id, _))| (object_id(&self.game, *id), index))
                    .collect();
                Ok((
                    PromptInput::Reorder(ReorderInput {
                        presentation: presentation(
                            "Choose order",
                            Some(ctx.description.clone()),
                            source,
                        ),
                        items: ctx
                            .items
                            .iter()
                            .filter_map(|(id, _)| {
                                protocol_card(&self.game, *id).map(|card| ReorderItem {
                                    id: object_id(&self.game, *id),
                                    card,
                                    oracle: None,
                                })
                            })
                            .collect(),
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
                            .filter_map(|(id, _)| protocol_card(&self.game, *id))
                            .collect(),
                        zones: vec![ScryDestination::LibraryTop, secondary_zone],
                    }),
                    ManabrewPromptBinding::Partition {
                        objects: ctx
                            .cards
                            .iter()
                            .map(|(id, _)| (object_id(&self.game, *id), *id))
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
                        let attacker_id = object_id(&self.game, option.creature);
                        attackers.insert(attacker_id.clone(), option.creature);
                        let valid_target_ids = option
                            .valid_targets
                            .iter()
                            .map(|target| {
                                let (id, label, kind) = match target {
                                    AttackTarget::Player(player) => (
                                        player_id(*player),
                                        self.game
                                            .player(*player)
                                            .map(|player| player.name.clone())
                                            .unwrap_or_else(|| player_id(*player)),
                                        AttackTargetKind::Player,
                                    ),
                                    AttackTarget::Planeswalker(object) => (
                                        object_id(&self.game, *object),
                                        self.game.current_name(*object).unwrap_or_default(),
                                        AttackTargetKind::Planeswalker,
                                    ),
                                    AttackTarget::Battle(object) => (
                                        object_id(&self.game, *object),
                                        self.game.current_name(*object).unwrap_or_default(),
                                        AttackTargetKind::Battle,
                                    ),
                                };
                                targets.insert(id.clone(), target.clone());
                                target_dtos.entry(id.clone()).or_insert(AttackTargetDto {
                                    id: id.clone(),
                                    label,
                                    kind,
                                });
                                id
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
                        let attacker_id = object_id(&self.game, option.attacker);
                        objects.insert(attacker_id.clone(), option.attacker);
                        let valid_blocker_ids = option
                            .valid_blockers
                            .iter()
                            .map(|(blocker, _)| {
                                let id = object_id(&self.game, *blocker);
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
                            must_be_blocked: option.min_blockers > 0,
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
            DecisionContext::Targets(ctx) => {
                let min_targets = ctx
                    .requirements
                    .iter()
                    .map(|req| req.min_targets)
                    .sum::<usize>();
                let max_targets = ctx
                    .requirements
                    .iter()
                    .map(|req| req.max_targets.unwrap_or(req.legal_targets.len()))
                    .sum::<usize>();
                let mut targets = HashMap::new();
                let mut candidates = Vec::new();
                for requirement in &ctx.requirements {
                    for target in &requirement.legal_targets {
                        let reference = protocol_target(&self.game, *target);
                        let key = format!("{:?}:{}", reference.kind, reference.id);
                        if targets.insert(key, *target).is_none() {
                            candidates.push(reference);
                        }
                    }
                }
                Ok((
                    PromptInput::ChooseBoardTargets(ChooseBoardTargetsInput {
                        candidates,
                        hostile: false,
                        intent: manabrew_protocol::game::TargetingIntent::Friendly,
                        min_targets: min_targets.min(i32::MAX as usize) as i32,
                        max_targets: max_targets.min(i32::MAX as usize) as i32,
                        chosen_targets: 0,
                        label: ctx.context.clone(),
                    }),
                    ManabrewPromptBinding::Targets { targets },
                ))
            }
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
                    min_choices: 0,
                    max_choices: 0,
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
                    min_choices: 0,
                    max_choices: 0,
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
                    allocations: Vec::with_capacity(ctx.available_counters.len()),
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
                        min_choices: 0,
                        max_choices: options.len(),
                        options,
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
                source_card_id: open.source_card_id.clone(),
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
            input,
            binding,
        };
        let prompt = AgentPrompt {
            prompt_id: open.prompt_id,
            deciding_player_id: player_id(open.deciding_player),
            source_card_id: open.source_card_id.clone(),
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
                    state: self.manabrew_state(viewer),
                    prompt: None,
                    error: error.or(Some(prompt_error)),
                };
            }
        };
        ManabrewViewResult {
            state: self.manabrew_state(viewer),
            prompt,
            error,
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
        let available = state
            .available
            .get(state.counter_index)
            .copied()
            .ok_or_else(|| invalid("counter type index is out of range".to_string()))?;
        if amount > available || amount > state.remaining {
            return Err(invalid(format!(
                "cannot remove {amount} counter(s); {available} of this type and {} total are available",
                state.remaining
            )));
        }

        let mut next = state.clone();
        next.allocations.push(amount);
        next.remaining -= amount;
        next.counter_index += 1;
        if next.remaining == 0 {
            next.allocations.resize(next.counter_names.len(), 0);
        }
        if next.counter_index >= next.counter_names.len() || next.remaining == 0 {
            let mut option_indices = Vec::new();
            for (index, count) in next.allocations.iter().copied().enumerate() {
                option_indices.extend(std::iter::repeat_n(index, count as usize));
            }
            return Ok(ManabrewResponseAction::Dispatch(UiCommand::SelectOptions {
                option_indices,
            }));
        }

        let (input, binding) = Self::manabrew_counter_prompt(next, open.source_card_id.clone())?;
        Ok(ManabrewResponseAction::Continue { input, binding })
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
                    pass_index,
                },
                PromptOutput::ChooseAction(output),
            ) => {
                let index = match output {
                    ChooseActionOutput::Pass { .. } => *pass_index,
                    ChooseActionOutput::Act { action_id } => *actions
                        .get(&action_id)
                        .ok_or_else(|| invalid(format!("unknown action id {action_id}")))?,
                    ChooseActionOutput::RestoreSnapshot { .. } => {
                        return Err(invalid("snapshot restoration is not supported".to_string()));
                    }
                };
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
                ManabrewPromptBinding::Number,
                PromptOutput::ChooseNumber(ChooseNumberOutput::NumberDecision { chosen_number }),
            ) => {
                let value = chosen_number
                    .filter(|value| *value >= 0)
                    .ok_or_else(|| invalid("a non-negative number is required".to_string()))?;
                Ok(ManabrewResponseAction::Dispatch(UiCommand::NumberChoice {
                    value: value as u32,
                }))
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
                ManabrewPromptBinding::Targets { targets },
                PromptOutput::ChooseBoardTargets(ChooseBoardTargetsOutput::BoardTargets { chosen }),
            ) => {
                let targets = chosen
                    .into_iter()
                    .map(|target| {
                        let key = format!("{:?}:{}", target.kind, target.id);
                        match targets
                            .get(&key)
                            .copied()
                            .ok_or_else(|| invalid(format!("unknown target {}", target.id)))?
                        {
                            Target::Player(player) => Ok(TargetInput::Player { player: player.0 }),
                            Target::Object(object) => Ok(TargetInput::Object { object: object.0 }),
                        }
                    })
                    .collect::<Result<Vec<_>, ProtocolError>>()?;
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
                ManabrewPromptBinding::Payment { actions, pay_index },
                PromptOutput::PayManaCost(output),
            ) => match output {
                PayManaCostOutput::Act { action_id } => {
                    Ok(ManabrewResponseAction::Dispatch(UiCommand::SelectOptions {
                        option_indices: vec![*actions.get(&action_id).ok_or_else(|| {
                            invalid(format!("unknown payment action {action_id}"))
                        })?],
                    }))
                }
                PayManaCostOutput::Pay { .. } => {
                    Ok(ManabrewResponseAction::Dispatch(UiCommand::SelectOptions {
                        option_indices: vec![pay_index.ok_or_else(|| {
                            invalid("mana pool cannot pay the current pip".to_string())
                        })?],
                    }))
                }
                PayManaCostOutput::Cancel => Ok(ManabrewResponseAction::Cancel),
            },
            _ => Err(protocol_error(
                ProtocolErrorCode::WrongPromptType,
                "prompt output family does not match the stored binding",
                Some(open.prompt_id),
            )),
        }
    }
}

#[wasm_bindgen]
impl WasmGame {
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
        self.register_manabrew_deck_sources_input(&input.decks);
        let validation = self.validate_match_setup_input(&manabrew_match_setup(&input))?;
        manabrew_to_js(&validation, "Manabrew match validation")
    }

    #[wasm_bindgen(js_name = startManabrewMatch)]
    pub fn start_manabrew_match(&mut self, config: JsValue) -> Result<JsValue, JsValue> {
        let input: ManabrewMatchConfigInput =
            serde_wasm_bindgen::from_value(config).map_err(|error| {
                JsValue::from_str(&format!("invalid Manabrew match config: {error}"))
            })?;
        self.register_manabrew_deck_sources_input(&input.decks);
        let setup = manabrew_match_setup(&input);
        let seed = setup.seed;
        let setup_js = manabrew_to_js(&setup, "Ironsmith match setup")?;
        let result = self.start_match(setup_js)?;
        self.manabrew_game_id = input
            .game_id
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| format!("ironsmith-{seed:016x}"));
        self.manabrew_human_players = if input.human_players.len() == input.player_names.len() {
            input.human_players
        } else {
            (0..input.player_names.len())
                .map(|index| !input.bot_players.contains(&(index as u8)))
                .collect()
        };
        self.manabrew_next_prompt_id = 1;
        self.manabrew_open_prompt = None;
        Ok(result)
    }

    /// Build a typed player-specific view. Passing no viewer produces a spectator-safe public view.
    #[wasm_bindgen(js_name = manabrewView)]
    pub fn manabrew_view(&mut self, viewer: Option<u8>) -> Result<JsValue, JsValue> {
        let viewer = match viewer {
            Some(viewer) => match self.manabrew_player(viewer) {
                Ok(player) => Some(player),
                Err(error) => {
                    return manabrew_to_js(
                        &ManabrewViewResult {
                            state: self.manabrew_state(None),
                            prompt: None,
                            error: Some(error),
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
                    input,
                    binding,
                });
                let result = self.manabrew_result(Some(player), None);
                return manabrew_to_js(&result, "Manabrew response");
            }
            ManabrewResponseAction::Cancel => self.cancel_decision(),
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

    #[wasm_bindgen(js_name = manabrewApplyDirective)]
    pub fn manabrew_apply_directive(
        &mut self,
        player: u8,
        directive: JsValue,
    ) -> Result<JsValue, JsValue> {
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
            free_for_all: None,
            teams: None,
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
            input,
            binding,
        }
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
        assert_eq!(layout, "transform_like");
        assert_eq!(combined_name, "Daybound Adept // Nightbound Adept");
        assert!(faces[0].block.contains("Loyalty: 4"));
        assert!(faces[1].block.contains("Defense: 5"));
        assert!(faces[0].block.contains("+1: Draw a card."));
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
            ironsmith::decisions::context::PriorityContext::new(
                PlayerId::from_index(0),
                vec![LegalAction::PassPriority],
            ),
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
            ironsmith::decisions::context::PriorityContext::new(
                PlayerId::from_index(0),
                vec![
                    LegalAction::PassPriority,
                    LegalAction::PlayLand {
                        land_id: ObjectId::from_raw(999),
                    },
                ],
            ),
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
    fn counter_removal_uses_number_sequence_and_dispatches_repeated_indices() {
        let _id_guard = crate::test_id_counter_guard();
        let game = game();
        let context =
            DecisionContext::Counters(ironsmith::decisions::context::CountersContext::new(
                PlayerId::from_index(0),
                None,
                ObjectId::from_raw(900_001),
                "Counter Test Permanent",
                3,
                vec![
                    (ironsmith::object::CounterType::PlusOnePlusOne, 2),
                    (ironsmith::object::CounterType::Named("charge"), 3),
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
            ManabrewResponseAction::Dispatch(UiCommand::SelectOptions { option_indices }) => {
                assert_eq!(option_indices, vec![0, 1, 1]);
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
        assert_eq!(selection.min_choices, 0);
        assert_eq!(selection.max_choices, 2);
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
}
