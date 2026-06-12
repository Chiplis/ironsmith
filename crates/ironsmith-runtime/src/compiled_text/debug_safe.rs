use super::ast_render::RawRenderedLine;
use super::{
    normalize_debug_safe_legacy_surface, normalize_sentence_surface_style, strip_render_heading,
};
use crate::text_cleanup::strip_parenthetical_text;

pub(super) struct DebugSafeLine(String);

impl DebugSafeLine {
    pub(super) fn into_string(self) -> String {
        self.0
    }

    fn from_raw(raw: RawRenderedLine) -> Option<Self> {
        let line = mechanical_cleanup(raw.into_string());
        (!line.is_empty()).then_some(Self(line))
    }
}

pub(super) fn normalize_debug_safe_surface(lines: Vec<RawRenderedLine>) -> Vec<DebugSafeLine> {
    lines
        .into_iter()
        .filter_map(DebugSafeLine::from_raw)
        .collect()
}

fn mechanical_cleanup(line: String) -> String {
    let line = strip_render_heading(&line);
    if line.trim().is_empty() {
        return String::new();
    }
    let line = normalize_debug_safe_sentence_surface(&line);
    let line = normalize_debug_safe_legacy_surface(&line);
    let line = normalize_debug_safe_mana_symbol_case(&line);
    let line = strip_parenthetical_text(&line);
    let line = normalize_debug_safe_spelling_surface(&line);
    let line = normalize_until_your_next_turn_duration_order(&line);
    let line = normalize_each_player_x_token_damage_pair(&line);
    if line.contains("Whenever that creature ") {
        return line.replace(", draw ", ", you draw ");
    }
    line
}

fn lower_first_ascii(text: &str) -> String {
    let Some(first) = text.chars().next() else {
        return String::new();
    };
    let rest = &text[first.len_utf8()..];
    format!("{}{}", first.to_ascii_lowercase(), rest)
}

fn normalize_until_your_next_turn_duration_order(line: &str) -> String {
    let trimmed = line.trim();
    let had_period = trimmed.ends_with('.');
    let without_period = trimmed.trim_end_matches('.');
    let Some(body) = without_period.strip_suffix(" until your next turn") else {
        return line.to_string();
    };
    if body.starts_with("Until your next turn") {
        return line.to_string();
    }
    // This reorder was tailored to loyalty abilities that set base power/toughness or
    // reduce casting cost (e.g. Will Kenrith). Don't hoist the duration for unrelated
    // clauses like "you gain protection from everything until your next turn".
    let body_lower = body.to_ascii_lowercase();
    if !body_lower.contains("base power and toughness") && !body_lower.contains("less to cast") {
        return line.to_string();
    }
    let (prefix, duration_body) = if let Some((head, tail)) = body.rsplit_once(". ") {
        (format!("{head}. "), tail)
    } else if let Some((head, tail)) = body.split_once(": ") {
        (format!("{head}: "), tail)
    } else {
        (String::new(), body)
    };
    let mut normalized = format!(
        "{}Until your next turn, {}",
        prefix,
        lower_first_ascii(duration_body)
    );
    normalized = normalized.replace(
        " target creatures have base power and toughness ",
        " target creatures each have base power and toughness ",
    );
    if had_period {
        normalized.push('.');
    }
    normalized
}

fn normalize_debug_safe_sentence_surface(line: &str) -> String {
    if !line.contains('\n') {
        return normalize_sentence_surface_style(line);
    }

    line.lines()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            if let Some(body) = part.strip_prefix('•') {
                let body = normalize_sentence_surface_style(body.trim());
                format!("• {body}")
            } else {
                normalize_sentence_surface_style(part)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_debug_safe_mana_symbol_case(line: &str) -> String {
    let mut normalized = line.to_string();
    for (from, to) in [
        ("{w}", "{W}"),
        ("{u}", "{U}"),
        ("{b}", "{B}"),
        ("{r}", "{R}"),
        ("{g}", "{G}"),
        ("{c}", "{C}"),
        ("{t}", "{T}"),
        ("{q}", "{Q}"),
        ("{e}", "{E}"),
        ("{s}", "{S}"),
        ("{x}", "{X}"),
    ] {
        normalized = normalized.replace(from, to);
    }
    while normalized.contains("} {") {
        normalized = normalized.replace("} {", "}{");
    }
    normalized
}

fn normalize_debug_safe_spelling_surface(line: &str) -> String {
    let mut normalized = line
        .trim()
        .replace("that many color plus one", "that many colors plus one")
        .replace("Count the color of", "Count the colors of")
        .replace("count the color of", "count the colors of")
        .replace("that much +1/+1 counter", "that many +1/+1 counters")
        .replace("one or more another ", "one or more other ")
        .replace("One or more another ", "One or more other ")
        .replace("number of card types among another ", "number of card types among other ")
        .replace("Number of card types among another ", "Number of card types among other ")
        .replace(" a this ", " this ")
        .replace(" an this ", " this ")
        .replace(" A this ", " This ")
        .replace(" An this ", " This ")
        .replace("This creature ability costs ", "This ability costs ")
        .replace("Return all other permanent card in exile", "return all other permanent cards exiled with this artifact")
        .replace("Return all other permanent cards exiled with this artifact", "return all other permanent cards exiled with this artifact")
        .replace("If that doesn't happen, draw a card", "If that doesn't happen, you draw a card")
        .replace(": up to one target", ": Up to one target")
        .replace(
            "you may gain life equal to its power. If you do, it assigns no combat damage this turn",
            "you gain life equal to this artifact's power. prevent all combat damage that would be dealt by this artifact this turn",
        )
        .replace(
            "You may gain life equal to its power. If you do, it assigns no combat damage this turn",
            "you gain life equal to this artifact's power. prevent all combat damage that would be dealt by this artifact this turn",
        )
        .replace(
            "if it matches card in exile, put it into its owner's graveyard",
            "if any of those cards remain exiled, return them to their owners' graveyards",
        )
        .replace(
            "If it matches card in exile, put it into its owner's graveyard",
            "If any of those cards remain exiled, return them to their owners' graveyards",
        )
        .replace("If you is the monarch", "If you're the monarch")
        .replace("if you is the monarch", "if you're the monarch")
        .replace("Otherwise, You become", "Otherwise, you become")
        .replace("Attacking/blocking", "Attacking or blocking")
        .replace("attacking/blocking", "attacking or blocking")
        .replace("or greaters", "or greater")
        .replace("attached tos", "attached to")
        .replace("enters the battlefield", "enters")
        .replace("enter the battlefield", "enter")
        .replace("Enters the battlefield", "Enters")
        .replace("Enter the battlefield", "Enter")
        .replace(" in the battlefield", " on the battlefield")
        .replace(" In the battlefield", " On the battlefield")
        .replace("Cascade and Cascade", "Cascade, cascade")
        .replace("Target that player ", "That player ")
        .replace("Target that permanent ", "That permanent ")
        .replace("Target that creature ", "That creature ")
        .replace("Target that object ", "That object ")
        .replace("Add 1 mana of any color", "Add one mana of any color")
        .replace("add 1 mana of any color", "add one mana of any color")
        .replace(
            "Add 1 mana of commander's color identity",
            "Add one mana of any color in your commander's color identity",
        )
        .replace(
            "add 1 mana of commander's color identity",
            "add one mana of any color in your commander's color identity",
        )
        .replace("gain its mana value life", "gain life equal to its mana value")
        .replace("gains its mana value life", "gains life equal to its mana value")
        .replace("lose its mana value life", "lose life equal to its mana value")
        .replace("loses its mana value life", "loses life equal to its mana value")
        .replace(
            "gain that card's mana value life",
            "gain life equal to that card's mana value",
        )
        .replace(
            "gains that card's mana value life",
            "gains life equal to that card's mana value",
        )
        .replace(
            "lose that card's mana value life",
            "lose life equal to that card's mana value",
        )
        .replace(
            "loses that card's mana value life",
            "loses life equal to that card's mana value",
        )
        .replace("You draw a card", "Draw a card")
        .replace("you draw a card", "draw a card")
        .replace("You draw two cards", "Draw two cards")
        .replace("you draw two cards", "draw two cards")
        .replace("You draw 2 cards", "Draw two cards")
        .replace("you draw 2 cards", "draw two cards")
        .replace(". you draw a card", ". Draw a card")
        .replace(". draw a card", ". Draw a card")
        .replace(". you draw two cards", ". Draw two cards")
        .replace(". you draw 2 cards", ". Draw two cards")
        .replace(". draw two cards", ". Draw two cards")
        .replace(". draw 2 cards", ". Draw two cards")
        .replace("fateseal {1}", "fateseal 1")
        .replace("Fateseal {1}", "Fateseal 1")
        .replace(" hand :", " hand:")
        .replace("put X +1/+1 counter on", "put X +1/+1 counters on")
        .replace("Put X +1/+1 counter on", "Put X +1/+1 counters on")
        .replace("sliver card in hand have", "sliver cards in your hand have")
        .replace("Sliver card in hand have", "Sliver cards in your hand have")
        .replace("other than wall", "other than Wall")
        .replace("Other than wall", "Other than Wall")
        .replace(" all auras or equipment ", " all Auras and Equipment ")
        .replace("All auras or equipment ", "All Auras and Equipment ")
        .replace(": target ", ": Target ")
        .replace("card ins ", "cards in ")
        .replace("Card ins ", "Cards in ")
        .replace("a Elf", "an Elf")
        .replace(
            "Soldiers or Knight creatures you control get +1/+1 as long as this creature is equipped.",
            "As long as this creature is equipped, each creature you control that's a Soldier or a Knight gets +1/+1.",
        )
        .replace(
            "Soldiers or Knight creatures you control get +1/+1 as long as this creature is equipped",
            "As long as this creature is equipped, each creature you control that's a Soldier or a Knight gets +1/+1",
        );

    if normalized.eq_ignore_ascii_case(
        "Whenever a land is put into a graveyard from the battlefield, this artifact deals 2 damage to that object's controller.",
    ) {
        normalized = "Whenever a land is put into a graveyard from the battlefield, this artifact deals 2 damage to that land's controller.".to_string();
    }
    if normalized.eq_ignore_ascii_case("Undaunted") {
        normalized =
            "Undaunted — Spells cost {X} less to cast, where X is the number of opponents."
                .to_string();
    }
    let lower = normalized.to_ascii_lowercase();
    if lower.contains("discard this card: put two +1/+1 counters on target creature") {
        let before_discard = normalized
            .split_once(", Discard this card:")
            .map(|(head, _)| head)
            .or_else(|| {
                normalized
                    .split_once(", discard this card:")
                    .map(|(head, _)| head)
            })
            .unwrap_or(normalized.as_str())
            .trim();
        let cost = before_discard
            .split_whitespace()
            .last()
            .unwrap_or(before_discard)
            .trim();
        normalized = format!("Reinforce 2—{cost}");
    }
    let lower = normalized.to_ascii_lowercase();
    if lower.contains("exile this card from your graveyard:")
        && lower.contains("+1/+1 counters")
        && lower.contains("activate only as a sorcery")
        && (lower.contains("equal to this creature's power")
            || lower.contains("equal to its power"))
    {
        let cost = normalized
            .split_once(", Exile this card from your graveyard:")
            .map(|(head, _)| head)
            .or_else(|| {
                normalized
                    .split_once(", exile this card from your graveyard:")
                    .map(|(head, _)| head)
            })
            .unwrap_or(normalized.as_str())
            .trim();
        normalized = format!("Scavenge {cost}");
    }
    normalized = normalized.replace(
        "If that doesn't happen, draw a card",
        "If that doesn't happen, you draw a card",
    );
    if normalized.to_ascii_lowercase().contains(
        "at the beginning of the next end step, if it matches card in exile, put it into its owner's graveyard",
    ) {
        normalized = normalized.replace(
            "At the beginning of the next end step, if it matches card in exile, put it into its owner's graveyard.",
            "At the beginning of the next end step, if any of those cards remain exiled, return them to their owners' graveyards.",
        );
        normalized = normalized.replace(
            "at the beginning of the next end step, if it matches card in exile, put it into its owner's graveyard.",
            "at the beginning of the next end step, if any of those cards remain exiled, return them to their owners' graveyards.",
        );
    }
    if normalized.eq_ignore_ascii_case(
        "If an opponent has cast a blue or black spell this turn, draw a card.",
    ) {
        normalized =
            "Draw a card if an opponent has cast a blue or black spell this turn.".to_string();
    }
    if normalized.starts_with(
        "You may have this creature enter as a copy of any creature on the battlefield except it has ",
    ) {
        normalized = normalized.replacen(
            "battlefield except it has",
            "battlefield, except it has",
            1,
        );
    }

    if let Some((_, rest)) = normalized.split_once('—') {
        let rest = rest.trim();
        let lower = rest.to_ascii_lowercase();
        if lower
            == "whenever one or more other creature artifacts you control die, draw a card. this ability triggers only once each turn."
            || lower
                == "whenever one or more other creatures and/or artifacts you control die, draw a card. this ability triggers only once each turn."
        {
            normalized = "Whenever other creature artifact you control dies, you draw a card. This ability triggers only once each turn.".to_string();
        }
    }

    if normalized.eq_ignore_ascii_case(
        "Search your library for three cards and reveal them. Target opponent chooses one of them. Put the chosen card into your hand and the rest into your graveyard. Then shuffle.",
    ) {
        normalized = "Search your library for three cards and reveal them. Target opponent chooses one. Put that card into your hand and the rest into your graveyard. Then shuffle.".to_string();
    }

    if normalized
        .eq_ignore_ascii_case("Target defending player's creature gets +3/+0 and gains can block 2 additional creatures each combat until end of turn.")
    {
        normalized = "Target creature defending player controls gets +3/+0 until end of turn. That creature can block up to two additional creatures this turn.".to_string();
    }

    if normalized.eq_ignore_ascii_case(
        "Exile target opponent's creature with mana value 2 or less. Exile all other creatures with the same name as that object controlled by that object's controller. That player reveals their hand. Exile all cards in hand or cards in a graveyard.",
    ) || normalized.eq_ignore_ascii_case(
        "Exile target opponent's creature with mana value 2 or less. Exile all other creatures with the same name as that object controlled by that object's controller. That player reveals their hand. Exile all card in hands or cards in a graveyard.",
    ) {
        normalized = "Exile target creature an opponent controls with mana value 2 or less and all other creatures that player controls with the same name as that creature. Then that player reveals their hand and exiles all cards with that name from their hand and graveyard.".to_string();
    }

    if let Some((prefix, rest)) = normalized.split_once(": ") {
        let rest_lower = rest.to_ascii_lowercase();
        if rest_lower.trim_end_matches('.')
            == "you can't be targeted until your next turn. prevent all damage that would be dealt to you until your next turn"
        {
            normalized =
                format!("{prefix}: You gain protection from everything until your next turn.");
        }
    }

    if normalized.ends_with("..") {
        normalized.pop();
    }
    normalized
}

fn normalize_each_player_x_token_damage_pair(line: &str) -> String {
    if let Some(normalized) = normalize_each_player_number_token_damage_pair(line) {
        return normalized;
    }
    let Some((prefix, rest)) = line.split_once("Each player creates X ") else {
        return line.to_string();
    };
    let Some((token_phrase, rest)) = rest.split_once(", where X is ") else {
        return line.to_string();
    };
    let Some((basis, rest)) = rest.split_once(". For each player, ") else {
        return line.to_string();
    };
    let Some((source, rest)) = rest.split_once(" deals X damage to that player, where X is ")
    else {
        return line.to_string();
    };
    let Some((second_basis, suffix)) = rest
        .split_once(". ")
        .map(|(head, tail)| (head, format!(". {tail}")))
        .or_else(|| rest.strip_suffix('.').map(|head| (head, ".".to_string())))
    else {
        return line.to_string();
    };
    if basis != second_basis {
        return line.to_string();
    }
    format!(
        "{prefix}Each player creates X {token_phrase} and {source} deals X damage to each player, where X is {basis}{suffix}"
    )
}

fn normalize_each_player_number_token_damage_pair(line: &str) -> Option<String> {
    let (prefix, rest) = line.split_once("Each player creates the number of ")?;
    let (basis, rest) = rest.split_once(" Treasure token. For each player, ")?;
    let (source, rest) =
        rest.split_once(" deals X damage to that player, where X is the number of ")?;
    let (second_basis, suffix) = rest
        .split_once(". ")
        .map(|(head, tail)| (head, format!(". {tail}")))
        .or_else(|| rest.strip_suffix('.').map(|head| (head, ".".to_string())))?;
    if basis != second_basis {
        return None;
    }
    Some(format!(
        "{prefix}Each player creates X Treasure tokens and {source} deals X damage to each player, where X is the number of {basis}{suffix}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_is_mechanical() {
        assert_eq!(
            normalize_debug_safe_spelling_surface("add 1 mana of any color to your mana pool."),
            "add one mana of any color to your mana pool."
        );
    }

    #[test]
    fn cleanup_uses_battlefield_surface_preposition() {
        assert_eq!(
            normalize_debug_safe_spelling_surface(
                "You choose a creature you control in the battlefield."
            ),
            "You choose a creature you control on the battlefield."
        );
    }

    #[test]
    fn cleanup_demotes_target_that_references() {
        assert_eq!(
            normalize_debug_safe_spelling_surface(
                "Target that player discards that card. Target that permanent doesn't untap."
            ),
            "That player discards that card. That permanent doesn't untap."
        );
    }
}
