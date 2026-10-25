use super::*;

use crate::static_abilities::StaticAbilityId;

/// Visibility of a card in a drafted pile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftVisibility {
    FaceDown,
    FaceUp,
}

/// Stable draft-local identity and state for one card.
#[derive(Debug, Clone)]
pub struct DraftCard {
    pub id: u64,
    pub definition: crate::cards::CardDefinition,
    pub visibility: DraftVisibility,
    /// Public information noted while this card was revealed as drafted.
    pub public_note: Option<String>,
    /// Colors chosen as this card was drafted ("the player to your right
    /// chooses a color, you choose another color, ..."), parsed from the
    /// public note of a reveal-as-drafted card whose draft text chooses colors.
    pub chosen_colors: Option<crate::color::ColorSet>,
    /// The player who passed this card to its drafter, for a reveal-as-drafted
    /// card that notes them ("note the player who passed it to you"). `None`
    /// for a card picked from a freshly opened pack.
    pub noted_player: Option<PlayerId>,
}

/// One player's simultaneous choice for a single draft step.
#[derive(Debug, Clone, Default)]
pub struct DraftSelection {
    pub player: PlayerId,
    /// The format's ordinary pick count, plus any explicitly granted extra pick.
    pub card_ids: Vec<u64>,
    /// A previously drafted face-up card returned to the current pack.
    pub exchange_face_up: Option<u64>,
    /// Public notes required by reveal-as-drafted cards, keyed by draft-card ID.
    pub public_notes: HashMap<u64, String>,
}

/// Viewer-safe description of one drafted card slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftCardView {
    pub id: u64,
    pub name: Option<String>,
    pub face_up: bool,
    pub public_note: Option<String>,
}

/// Three-round Conspiracy draft state. Packs are passed by seat after each
/// simultaneous batch of picks; no active player or priority is involved.
#[derive(Debug, Clone)]
pub struct ConspiracyDraftState {
    players: Vec<PlayerId>,
    unopened: HashMap<PlayerId, Vec<Vec<crate::cards::CardDefinition>>>,
    current_packs: HashMap<PlayerId, Vec<DraftCard>>,
    drafted: HashMap<PlayerId, Vec<DraftCard>>,
    round: u8,
    next_card_id: u64,
    ordinary_pick_count: usize,
    complete: bool,
    /// Who passed each player their current pack this round; absent for a
    /// pack its holder just opened.
    pack_passers: HashMap<PlayerId, PlayerId>,
}

/// "Reveal this card as you draft it and note the player who passed it to
/// you." (Cogwork Tracker): the noted information is the passing seat itself.
fn notes_passing_player(definition: &crate::cards::CardDefinition) -> bool {
    draft_rules(definition).any(|rule| rule.notes_passing_player)
}

fn draft_rules(
    definition: &crate::cards::CardDefinition,
) -> impl Iterator<Item = ironsmith_core::DraftRule> + '_ {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => ability.draft_rule(),
            _ => None,
        })
}

fn is_drafted_face_up(definition: &crate::cards::CardDefinition) -> bool {
    draft_rules(definition).any(|rule| rule.face_up)
}

fn is_revealed_and_noted(definition: &crate::cards::CardDefinition) -> bool {
    draft_rules(definition).any(|rule| rule.revealed)
}

/// How many colors a reveal-as-drafted card has players choose while it is
/// drafted (Paliano, the High City: three, each different). Zero when the
/// card's draft text chooses no colors.
fn draft_color_choice_count(definition: &crate::cards::CardDefinition) -> usize {
    draft_rules(definition)
        .map(|rule| rule.chosen_colors as usize)
        .sum()
}

/// Parse the public note of a color-choosing draft card: exactly `count`
/// different color words in choice order ("red, green, white").
fn parse_draft_chosen_colors(note: &str, count: usize) -> Option<crate::color::ColorSet> {
    let mut colors = Vec::new();
    for word in note
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|word| !word.is_empty())
    {
        let word = word.to_ascii_lowercase();
        if matches!(word.as_str(), "and" | "then") {
            continue;
        }
        let color = crate::color::Color::from_name(&word)?;
        if colors.contains(&color) {
            return None;
        }
        colors.push(color);
    }
    (colors.len() == count).then(|| {
        colors
            .into_iter()
            .fold(crate::color::ColorSet::COLORLESS, |set, color| {
                set.union(crate::color::ColorSet::from_color(color))
            })
    })
}

fn grants_exchange_pick(definition: &crate::cards::CardDefinition) -> bool {
    draft_rules(definition).any(|rule| rule.exchange_pick)
}

impl ConspiracyDraftState {
    /// Create a canonical three-round draft. Every player supplies exactly one
    /// pack for each round in round order.
    pub fn new(
        players: Vec<PlayerId>,
        packs: Vec<(PlayerId, Vec<Vec<crate::cards::CardDefinition>>)>,
    ) -> Result<Self, String> {
        Self::new_with_pick_count(players, packs, 1)
    }

    pub(super) fn new_with_pick_count(
        players: Vec<PlayerId>,
        packs: Vec<(PlayerId, Vec<Vec<crate::cards::CardDefinition>>)>,
        ordinary_pick_count: usize,
    ) -> Result<Self, String> {
        if players.len() < 2 {
            return Err("a draft requires at least two players".to_string());
        }
        if ordinary_pick_count == 0 {
            return Err("a draft must select at least one card per pick".to_string());
        }
        let player_set = players.iter().copied().collect::<HashSet<_>>();
        if player_set.len() != players.len() {
            return Err("draft seats must contain distinct players".to_string());
        }
        if packs.len() != players.len() {
            return Err("each draft seat must provide three booster packs".to_string());
        }
        let mut unopened = HashMap::new();
        for (player, player_packs) in packs {
            if !player_set.contains(&player) || unopened.contains_key(&player) {
                return Err("booster packs must belong to distinct draft seats".to_string());
            }
            if player_packs.len() != 3 || player_packs.iter().any(Vec::is_empty) {
                return Err("each draft seat must provide exactly three nonempty packs".to_string());
            }
            unopened.insert(player, player_packs);
        }
        let mut state = Self {
            drafted: players
                .iter()
                .copied()
                .map(|player| (player, Vec::new()))
                .collect(),
            players,
            unopened,
            current_packs: HashMap::new(),
            round: 0,
            next_card_id: 1,
            ordinary_pick_count,
            complete: false,
            pack_passers: HashMap::new(),
        };
        state.open_round()?;
        Ok(state)
    }

    pub fn round(&self) -> u8 {
        self.round
    }

    pub fn is_complete(&self) -> bool {
        self.complete
    }

    fn wrap_pack(&mut self, definitions: Vec<crate::cards::CardDefinition>) -> Vec<DraftCard> {
        definitions
            .into_iter()
            .map(|definition| {
                let id = self.next_card_id;
                self.next_card_id = self.next_card_id.saturating_add(1);
                DraftCard {
                    id,
                    definition,
                    visibility: DraftVisibility::FaceDown,
                    public_note: None,
                    chosen_colors: None,
                    noted_player: None,
                }
            })
            .collect()
    }

    fn open_round(&mut self) -> Result<(), String> {
        if self.round >= 3 {
            self.complete = true;
            self.current_packs.clear();
            return Ok(());
        }
        self.current_packs.clear();
        self.pack_passers.clear();
        for player in self.players.clone() {
            let definitions = self
                .unopened
                .get_mut(&player)
                .and_then(|packs| (!packs.is_empty()).then(|| packs.remove(0)))
                .ok_or_else(|| format!("missing round {} pack", self.round + 1))?;
            let pack = self.wrap_pack(definitions);
            self.current_packs.insert(player, pack);
        }
        self.round += 1;
        Ok(())
    }

    /// Resolve one simultaneous set of picks, then pass every nonempty pack in
    /// the canonical direction. The operation validates against a clone first,
    /// so an illegal batch cannot partially mutate private draft state.
    pub fn draft_step(&mut self, selections: Vec<DraftSelection>) -> Result<(), String> {
        if self.complete {
            return Err("the draft is complete".to_string());
        }
        let mut next = self.clone();
        next.apply_draft_step(selections)?;
        *self = next;
        Ok(())
    }

    fn apply_draft_step(&mut self, selections: Vec<DraftSelection>) -> Result<(), String> {
        let expected = self
            .players
            .iter()
            .filter(|player| {
                self.current_packs
                    .get(player)
                    .is_some_and(|pack| !pack.is_empty())
            })
            .copied()
            .collect::<HashSet<_>>();
        let supplied = selections
            .iter()
            .map(|selection| selection.player)
            .collect::<HashSet<_>>();
        if supplied.len() != selections.len() || supplied != expected {
            return Err(
                "each player holding a nonempty pack must make exactly one draft selection"
                    .to_string(),
            );
        }

        for selection in selections {
            let extra = selection.exchange_face_up.is_some();
            let pack_len = self
                .current_packs
                .get(&selection.player)
                .expect("validated current pack")
                .len();
            let required = self.ordinary_pick_count.min(pack_len) + usize::from(extra);
            if selection.card_ids.len() != required {
                return Err(format!(
                    "player {:?} must draft {required} card(s)",
                    selection.player
                ));
            }
            let mut unique = selection.card_ids.clone();
            unique.sort_unstable();
            unique.dedup();
            if unique.len() != selection.card_ids.len() {
                return Err("a draft selection cannot choose the same card twice".to_string());
            }

            let pack = self
                .current_packs
                .get_mut(&selection.player)
                .expect("validated current pack");
            let mut indices = Vec::new();
            for card_id in &selection.card_ids {
                let index = pack
                    .iter()
                    .position(|card| card.id == *card_id)
                    .ok_or_else(|| {
                        format!("draft card {card_id} is not in that player's current pack")
                    })?;
                indices.push(index);
            }
            indices.sort_unstable_by(|left, right| right.cmp(left));
            let mut picked = indices
                .into_iter()
                .map(|index| pack.remove(index))
                .collect::<Vec<_>>();
            picked.sort_by_key(|card| {
                selection
                    .card_ids
                    .iter()
                    .position(|id| *id == card.id)
                    .unwrap_or(usize::MAX)
            });

            if let Some(exchange) = selection.exchange_face_up {
                let drafted = self
                    .drafted
                    .get_mut(&selection.player)
                    .expect("validated draft seat");
                let index = drafted
                    .iter()
                    .position(|card| card.id == exchange)
                    .ok_or_else(|| {
                        "the exchanged card is not in that player's drafted pile".to_string()
                    })?;
                if drafted[index].visibility != DraftVisibility::FaceUp
                    || !grants_exchange_pick(&drafted[index].definition)
                {
                    return Err("an additional pick requires returning a face-up card whose draft ability grants it".to_string());
                }
                let mut returned = drafted.remove(index);
                returned.visibility = DraftVisibility::FaceDown;
                returned.public_note = None;
                returned.chosen_colors = None;
                returned.noted_player = None;
                pack.push(returned);
            }

            for mut card in picked {
                if is_revealed_and_noted(&card.definition)
                    && notes_passing_player(&card.definition)
                {
                    // The noted information is determined by the draft, not
                    // chosen: the seat that passed this pack (if any).
                    card.noted_player = self.pack_passers.get(&selection.player).copied();
                    card.public_note = card
                        .noted_player
                        .map(|passer| format!("passed by player {}", passer.0 + 1));
                    card.visibility = DraftVisibility::FaceDown;
                } else if is_revealed_and_noted(&card.definition) {
                    let note = selection
                        .public_notes
                        .get(&card.id)
                        .map(|note| note.trim())
                        .filter(|note| !note.is_empty())
                        .ok_or_else(|| {
                            format!("draft card {} requires public noted information", card.id)
                        })?;
                    let color_choices = draft_color_choice_count(&card.definition);
                    if color_choices > 0 {
                        let colors = parse_draft_chosen_colors(note, color_choices).ok_or_else(|| {
                            format!(
                                "draft card {} requires {color_choices} different chosen colors",
                                card.id
                            )
                        })?;
                        card.chosen_colors = Some(colors);
                    }
                    card.public_note = Some(note.to_string());
                    card.visibility = DraftVisibility::FaceDown;
                } else if selection.public_notes.contains_key(&card.id) {
                    return Err(format!(
                        "draft card {} does not instruct its drafter to note information",
                        card.id
                    ));
                } else if is_drafted_face_up(&card.definition) {
                    card.visibility = DraftVisibility::FaceUp;
                }
                self.drafted
                    .get_mut(&selection.player)
                    .expect("validated drafted pile")
                    .push(card);
            }
        }

        if self.current_packs.values().all(Vec::is_empty) {
            return self.open_round();
        }

        let pass_left = self.round == 1 || self.round == 3;
        let mut passed = HashMap::new();
        let mut passers = HashMap::new();
        let player_count = self.players.len();
        for (index, holder) in self.players.iter().copied().enumerate() {
            let destination_index = if pass_left {
                (index + 1) % player_count
            } else {
                (index + player_count - 1) % player_count
            };
            let destination = self.players[destination_index];
            let pack = self.current_packs.remove(&holder).unwrap_or_default();
            passed.insert(destination, pack);
            passers.insert(destination, holder);
        }
        self.current_packs = passed;
        self.pack_passers = passers;
        Ok(())
    }

    /// The current pack is visible only to the player presently holding it.
    pub fn current_pack_view(&self, viewer: PlayerId, holder: PlayerId) -> Vec<DraftCardView> {
        self.current_packs
            .get(&holder)
            .into_iter()
            .flatten()
            .map(|card| DraftCardView {
                id: card.id,
                name: (viewer == holder).then(|| card.definition.name().to_string()),
                face_up: false,
                public_note: None,
            })
            .collect()
    }

    /// Owners may inspect their whole drafted pile; opponents see names only
    /// for face-up picks. Public notes remain visible even after a card turns down.
    pub fn drafted_view(&self, viewer: PlayerId, owner: PlayerId) -> Vec<DraftCardView> {
        self.drafted
            .get(&owner)
            .into_iter()
            .flatten()
            .map(|card| DraftCardView {
                id: card.id,
                name: (viewer == owner || card.visibility == DraftVisibility::FaceUp)
                    .then(|| card.definition.name().to_string()),
                face_up: card.visibility == DraftVisibility::FaceUp,
                public_note: card.public_note.clone(),
            })
            .collect()
    }

    /// Colors `player` chose as they drafted cards named `card_name`, over
    /// every such card in their drafted pile.
    pub fn draft_chosen_colors(
        &self,
        player: PlayerId,
        card_name: &str,
    ) -> crate::color::ColorSet {
        let wanted = super::normalize_draft_note_card_name(card_name);
        self.drafted
            .get(&player)
            .into_iter()
            .flatten()
            .filter(|card| super::normalize_draft_note_card_name(card.definition.name()) == wanted)
            .filter_map(|card| card.chosen_colors)
            .fold(crate::color::ColorSet::COLORLESS, crate::color::ColorSet::union)
    }

    /// Return a player's limited card pool after all three rounds finish.
    pub fn card_pool(&self, player: PlayerId) -> Result<Vec<crate::cards::CardDefinition>, String> {
        if !self.complete {
            return Err(
                "the draft card pool is not final until all three rounds finish".to_string(),
            );
        }
        Ok(self
            .drafted
            .get(&player)
            .into_iter()
            .flatten()
            .map(|card| card.definition.clone())
            .collect())
    }

    /// Validate a Conspiracy limited main deck against the player's completed
    /// draft pool. Basic lands are available without limit; every other card
    /// must have been drafted, and conspiracies may never be in the deck.
    pub fn validate_deck(
        &self,
        player: PlayerId,
        deck: &[crate::cards::CardDefinition],
    ) -> Result<(), String> {
        if deck.len() < 40 {
            return Err("a Conspiracy Draft deck must contain at least 40 cards".to_string());
        }
        let mut available = HashMap::<String, usize>::new();
        for definition in self.card_pool(player)? {
            *available
                .entry(definition.name().trim().to_ascii_lowercase())
                .or_default() += 1;
        }
        for definition in deck {
            if definition.card.has_card_type(CardType::Conspiracy) {
                return Err(format!(
                    "{} cannot be included in a deck",
                    definition.name()
                ));
            }
            if definition
                .card
                .has_supertype(crate::types::Supertype::Basic)
                && definition.card.has_card_type(CardType::Land)
            {
                continue;
            }
            let name = definition.name().trim().to_ascii_lowercase();
            let remaining = available.get_mut(&name).ok_or_else(|| {
                format!(
                    "{} is not in that player's drafted card pool",
                    definition.name()
                )
            })?;
            if *remaining == 0 {
                return Err(format!(
                    "the deck contains more copies of {} than that player drafted",
                    definition.name()
                ));
            }
            *remaining -= 1;
        }
        Ok(())
    }
}

fn agenda_name_count(definition: &crate::cards::CardDefinition) -> usize {
    let mut count = 0;
    for ability in &definition.abilities {
        let AbilityKind::Static(ability) = &ability.kind else {
            continue;
        };
        count = match ability.id() {
            StaticAbilityId::DoubleAgenda => count.max(2),
            StaticAbilityId::HiddenAgenda => count.max(1),
            _ => count,
        };
    }
    count
}

/// How many colors a card has its owner circle as they build their deck
/// ("As you create your deck, circle two of the colors below.", Cryptic
/// Spires). `None` when the card makes no such choice.
pub fn deck_construction_color_choice_count(abilities: &[crate::ability::Ability]) -> Option<usize> {
    abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Static(ability) => ability.deck_color_choice_count().map(|count| count as usize),
        _ => None,
    })
}

/// The colors written after a card's name in a decklist for a card that has
/// its owner circle colors as they build the deck: "Cryptic Spires ({W}{B})".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeckConstructionColorNote {
    pub colors: crate::color::ColorSet,
    /// Mana symbols written, so a repeated color ("({W}{W})") is detectable.
    pub symbols: usize,
}

/// Split a decklist card name into the card's name and its circled-color
/// note, if it ends in one: `"Cryptic Spires ({W}{B})"` →
/// `("Cryptic Spires", Some(W+B))`. Any other name is returned unchanged.
pub fn split_deck_construction_color_note(
    name: &str,
) -> (&str, Option<DeckConstructionColorNote>) {
    let trimmed = name.trim();
    let Some(open) = trimmed.strip_suffix(')').and_then(|rest| rest.rfind('(')) else {
        return (trimmed, None);
    };
    let inner = &trimmed[open + 1..trimmed.len() - 1];
    let mut colors = crate::color::ColorSet::COLORLESS;
    let mut symbols = 0;
    let mut rest = inner.trim();
    while !rest.is_empty() {
        let Some(symbol) = rest
            .strip_prefix('{')
            .and_then(|tail| tail.split_once('}'))
            .filter(|(symbol, _)| symbol.chars().count() == 1)
        else {
            return (trimmed, None);
        };
        let Some(color) =
            crate::color::Color::from_mana_code_or_name(&symbol.0.to_ascii_lowercase())
        else {
            return (trimmed, None);
        };
        colors = colors.with(color);
        symbols += 1;
        rest = symbol.1.trim_start();
    }
    if symbols == 0 {
        return (trimmed, None);
    }
    let base = trimmed[..open].trim_end();
    if base.is_empty() {
        return (trimmed, None);
    }
    (base, Some(DeckConstructionColorNote { colors, symbols }))
}

/// Why a decklist entry's circled-color note does not fit its card, if it
/// does not: a card that circles N colors needs exactly N different colors,
/// and any other card must not carry a note.
pub fn deck_construction_color_note_error(
    card_name: &str,
    abilities: &[crate::ability::Ability],
    note: Option<DeckConstructionColorNote>,
) -> Option<String> {
    let required = deck_construction_color_choice_count(abilities);
    let example = || {
        let symbols = ["{W}", "{U}", "{B}", "{R}", "{G}"];
        let count = required.unwrap_or(2).min(symbols.len());
        format!("{card_name} ({})", symbols[..count].concat())
    };
    match (required, note) {
        (None, None) => None,
        (None, Some(_)) => Some(format!(
            "{card_name} doesn't choose colors as you build your deck; remove the colors after its name"
        )),
        (Some(count), None) => Some(format!(
            "{card_name} needs the {count} colors circled for it, written after its name, e.g. {}",
            example()
        )),
        (Some(count), Some(note))
            if note.symbols != count || note.colors.count() as usize != count =>
        {
            Some(format!(
                "{card_name} needs exactly {count} different colors after its name, e.g. {}",
                example()
            ))
        }
        (Some(_), Some(_)) => None,
    }
}

/// The deterministic choice used when no deck-construction note was recorded
/// for a card: the first `count` colors in WUBRG order.
fn default_deck_construction_colors(count: usize) -> crate::color::ColorSet {
    crate::color::Color::ALL
        .into_iter()
        .take(count)
        .collect()
}

impl GameState {

    /// Record the colors chosen for one card as its owner built their deck
    /// (a deck-construction note, CR 100.2-style pregame information). The
    /// choice is keyed by the card's stable identity and lasts the game.
    pub fn set_deck_construction_chosen_colors(
        &mut self,
        card: StableId,
        colors: crate::color::ColorSet,
    ) {
        self.auxiliary_tracking_mut()
            .deck_construction_chosen_colors
            .insert(card, colors);
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
    }

    /// Record a deck-construction note for `card` ("red, green"): exactly as
    /// many different color words as its deck-construction rule chooses.
    /// Intended for deck/pregame setup, before the opening shuffle.
    pub fn record_deck_construction_note(
        &mut self,
        card: ObjectId,
        note: &str,
    ) -> Result<(), String> {
        let object = self
            .object(card)
            .ok_or_else(|| format!("deck-construction note for unknown card {card:?}"))?;
        let count = deck_construction_color_choice_count(&object.abilities).ok_or_else(|| {
            format!("card {card:?} makes no deck-construction color choice")
        })?;
        let stable_id = object.stable_id;
        let colors = parse_draft_chosen_colors(note, count)
            .ok_or_else(|| format!("card {card:?} requires {count} different chosen colors"))?;
        self.set_deck_construction_chosen_colors(stable_id, colors);
        // CR 207.5: the circled colors' mana symbols become part of the
        // card's rules text, so they count toward its color identity.
        if let Some(object) = self.object_mut(card) {
            object.rules_text_color_identity = object.rules_text_color_identity.union(colors);
        }
        Ok(())
    }

    /// Record the circled colors written after a decklist card's name for the
    /// object created from that entry. A note that does not fit the card is
    /// rejected; the card then keeps its deterministic default.
    pub fn apply_deck_construction_color_note(
        &mut self,
        card: ObjectId,
        decklist_name: &str,
    ) -> Result<(), String> {
        let (base, note) = split_deck_construction_color_note(decklist_name);
        let object = self
            .object(card)
            .ok_or_else(|| format!("deck-construction note for unknown card {card:?}"))?;
        if let Some(error) =
            deck_construction_color_note_error(base, &object.abilities, note)
        {
            return Err(error);
        }
        if let Some(note) = note {
            let stable_id = object.stable_id;
            self.set_deck_construction_chosen_colors(stable_id, note.colors);
            // CR 207.5: circled colors count toward color identity.
            if let Some(object) = self.object_mut(card) {
                object.rules_text_color_identity =
                    object.rules_text_color_identity.union(note.colors);
            }
        }
        Ok(())
    }

    /// The colors chosen at deck construction for the card `source` is
    /// (looked up by stable identity, so any zone and any incarnation). With
    /// no recorded note, the card's rule falls back to the deterministic
    /// default: the first N colors in WUBRG order. A card with no such rule
    /// (e.g. a copy without it) has no colors.
    pub fn deck_construction_chosen_colors(&self, source: ObjectId) -> crate::color::ColorSet {
        let Some(object) = self.object(source) else {
            return crate::color::ColorSet::COLORLESS;
        };
        if let Some(colors) = self
            .auxiliary_tracking
            .deck_construction_chosen_colors
            .get(&object.stable_id)
        {
            return *colors;
        }
        deck_construction_color_choice_count(&object.abilities)
            .map_or(crate::color::ColorSet::COLORLESS, default_deck_construction_colors)
    }

    /// Carry the draft's structured records into the game that follows it,
    /// keyed by drafter and card name: colors chosen as cards were drafted
    /// (read by "any color chosen as you drafted cards named ..."). Players
    /// are matched by seat id; seats absent from this game are ignored.
    pub fn record_conspiracy_draft_choices(&mut self, draft: &ConspiracyDraftState) {
        let mut chosen = Vec::new();
        let mut noted_players = Vec::new();
        for (player, cards) in &draft.drafted {
            if self.player(*player).is_none() {
                continue;
            }
            for card in cards {
                if let Some(colors) = card.chosen_colors {
                    chosen.push((*player, card.definition.name().to_string(), colors));
                }
                if let Some(noted) = card.noted_player {
                    noted_players.push((*player, card.definition.name().to_string(), noted));
                }
            }
        }
        for (player, card_name, noted) in noted_players {
            self.record_draft_noted_player(player, card_name, noted);
        }
        for (player, card_name, colors) in chosen {
            let combined = self.draft_chosen_colors(player, &card_name).union(colors);
            self.set_draft_chosen_colors(player, card_name, combined);
        }
    }

    /// Enable the post-draft Conspiracy game and put the selected sideboard
    /// conspiracies into command before libraries are shuffled.
    pub fn enable_conspiracy(
        &mut self,
        selections: Vec<(PlayerId, Vec<ConspiracySetupCard>)>,
    ) -> Result<(), String> {
        if self.conspiracy.is_some() {
            return Err("Conspiracy Draft is already enabled".to_string());
        }
        let live_players = self
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .map(|player| player.id)
            .collect::<HashSet<_>>();
        let mut seen = HashSet::new();
        for (player, cards) in &selections {
            if !live_players.contains(player) || !seen.insert(*player) {
                return Err(
                    "conspiracy selections must belong to distinct players in the game".to_string(),
                );
            }
            for card in cards {
                if !card
                    .definition
                    .card
                    .card_types
                    .contains(&CardType::Conspiracy)
                {
                    return Err(format!(
                        "{} is not a Conspiracy card",
                        card.definition.name()
                    ));
                }
                if !card.definition.card.subtypes.is_empty() {
                    return Err(format!(
                        "Conspiracy card {} may not have subtypes",
                        card.definition.name()
                    ));
                }
                let expected = agenda_name_count(&card.definition);
                if card.agenda_names.len() != expected {
                    return Err(format!(
                        "{} requires exactly {expected} secret agenda name(s)",
                        card.definition.name()
                    ));
                }
                let normalized = card
                    .agenda_names
                    .iter()
                    .map(|name| name.split_whitespace().collect::<Vec<_>>().join(" "))
                    .collect::<Vec<_>>();
                if normalized.iter().any(String::is_empty)
                    || normalized
                        .iter()
                        .map(|name| name.to_ascii_lowercase())
                        .collect::<HashSet<_>>()
                        .len()
                        != normalized.len()
                {
                    return Err("agenda names must be nonempty and different".to_string());
                }
            }
        }

        let mut state = ConspiracyState::default();
        for (owner, cards) in selections {
            for mut setup in cards {
                let hidden = agenda_name_count(&setup.definition) > 0;
                for ability in &mut setup.definition.abilities {
                    if hidden {
                        ability.functional_zones.clear();
                    } else {
                        ability.functional_zones = vec![Zone::Command];
                    }
                }
                let object =
                    self.create_object_from_definition(&setup.definition, owner, Zone::Command);
                state.cards.entry(owner).or_default().push(object);
                if hidden {
                    self.set_chosen_named_option(object, setup.agenda_names.join("\n"));
                    state.face_down.insert(object);
                    state.agenda_names.insert(
                        object,
                        setup
                            .agenda_names
                            .into_iter()
                            .map(|name| name.split_whitespace().collect::<Vec<_>>().join(" "))
                            .collect(),
                    );
                }
            }
        }
        for player in &mut self.players {
            if player.is_in_game() {
                player.starting_life = 20;
                player.life = 20;
            }
        }
        self.conspiracy = Some(state);
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(())
    }

    pub fn conspiracy_cards(&self) -> Vec<ObjectId> {
        let mut cards = self
            .conspiracy
            .as_ref()
            .into_iter()
            .flat_map(|state| state.cards.values().flatten().copied())
            .collect::<Vec<_>>();
        cards.sort_by_key(|card| card.0);
        cards
    }

    /// Owners of conspiracies that make them the starting player (Power
    /// Play, CR 315.5a), in turn order.
    pub(crate) fn conspiracy_starting_player_candidates(&self) -> Vec<PlayerId> {
        let owners = self
            .conspiracy_cards()
            .into_iter()
            .filter(|card| !self.is_face_down_conspiracy(*card))
            .filter_map(|card| self.object(card))
            .filter(|object| {
                object.abilities.iter().any(|ability| {
                    matches!(&ability.kind, AbilityKind::Static(ability)
                        if ability.id() == StaticAbilityId::StartingPlayer)
                })
            })
            .map(|object| object.owner)
            .collect::<std::collections::HashSet<_>>();
        self.turn_store
            .turn_order
            .iter()
            .copied()
            .filter(|player| owners.contains(player))
            .collect()
    }

    pub fn is_conspiracy_card(&self, object: ObjectId) -> bool {
        self.conspiracy_cards().contains(&object)
    }

    pub fn is_face_down_conspiracy(&self, object: ObjectId) -> bool {
        self.conspiracy
            .as_ref()
            .is_some_and(|state| state.face_down.contains(&object))
    }

    pub fn agenda_names_for(&self, viewer: PlayerId, object: ObjectId) -> Option<&[String]> {
        let state = self.conspiracy.as_ref()?;
        let card = self.object(object)?;
        (card.owner == viewer || !state.face_down.contains(&object))
            .then(|| state.agenda_names.get(&object).map(Vec::as_slice))
            .flatten()
    }

    pub fn turn_conspiracy_face_up(
        &mut self,
        player: PlayerId,
        object: ObjectId,
    ) -> Result<(), String> {
        let card = self
            .object(object)
            .ok_or_else(|| "conspiracy is missing".to_string())?;
        if card.owner != player
            || !self
                .conspiracy
                .as_ref()
                .is_some_and(|state| state.face_down.contains(&object))
        {
            return Err(
                "only a face-down conspiracy you control may be turned face up".to_string(),
            );
        }
        self.conspiracy
            .as_mut()
            .expect("validated Conspiracy state")
            .face_down
            .remove(&object);
        if let Some(card) = self.object_mut(object) {
            for ability in card.abilities_mut() {
                ability.functional_zones = vec![Zone::Command];
            }
        }
        // CR 613.7j: a face-down conspiracy receives a new timestamp when it
        // turns face up.
        self.effect_store.continuous_effects.record_face_change(object);
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        Ok(())
    }

    pub fn reveal_conspiracies_for_player(&mut self, player: PlayerId) {
        let objects = self
            .conspiracy
            .as_ref()
            .and_then(|state| state.cards.get(&player))
            .cloned()
            .unwrap_or_default();
        for object in objects {
            if self.is_face_down_conspiracy(object) {
                let _ = self.turn_conspiracy_face_up(player, object);
            }
        }
    }

    pub fn synchronize_conspiracy_ability_zones(&mut self) {
        let hidden = self
            .conspiracy
            .as_ref()
            .map(|state| state.face_down.clone())
            .unwrap_or_default();
        for object in self.conspiracy_cards() {
            if let Some(card) = self.object_mut(object) {
                for ability in card.abilities_mut() {
                    ability.functional_zones = if hidden.contains(&object) {
                        Vec::new()
                    } else {
                        vec![Zone::Command]
                    };
                }
            }
        }
        self.mark_continuous_state_dirty();
    }

    pub(crate) fn handle_conspiracy_player_departure(&mut self, player: PlayerId) {
        self.reveal_conspiracies_for_player(player);
        if let Some(state) = self.conspiracy.as_mut()
            && let Some(cards) = state.cards.remove(&player)
        {
            for object in cards {
                state.face_down.remove(&object);
                state.agenda_names.remove(&object);
            }
        }
    }
}
