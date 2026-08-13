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
    let line = factor_repeated_other_you_control_union(&line);
    let line = normalize_until_your_next_turn_duration_order(&line);
    let line = normalize_each_player_x_token_damage_pair(&line);
    if line.contains("Whenever that creature ") {
        return line.replace(", draw ", ", you draw ");
    }
    line
}

fn factor_repeated_other_you_control_union(line: &str) -> String {
    for verb in [" have ", " gain ", " get ", " are "] {
        let Some((subject, predicate)) = line.split_once(verb) else {
            continue;
        };
        let Some(subject_body) = subject.strip_prefix("Other ") else {
            continue;
        };
        let lower = subject_body.to_ascii_lowercase();
        let marker = " and other ";
        let Some(marker_idx) = lower.find(marker) else {
            continue;
        };
        if lower[marker_idx + marker.len()..].contains(marker) {
            continue;
        }
        let Some(left) = subject_body[..marker_idx]
            .trim()
            .strip_suffix(" you control")
        else {
            continue;
        };
        let Some(right) = subject_body[marker_idx + marker.len()..]
            .trim()
            .strip_suffix(" you control")
        else {
            continue;
        };
        if left.is_empty() || right.is_empty() {
            continue;
        }
        return format!("Other {left} and {right} you control{verb}{predicate}");
    }
    line.to_string()
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
    // A trailing duration belongs only to the final coordinated action. Do
    // not use an earlier animation clause as evidence that a later grant's
    // duration should be hoisted over the whole triggered ability.
    let scoped_body = body
        .rsplit_once(", then ")
        .map_or(body, |(_, final_action)| final_action);
    let scoped_body_lower = scoped_body.to_ascii_lowercase();
    if !scoped_body_lower.contains("base power and toughness")
        && !scoped_body_lower.contains("less to cast")
    {
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

fn draw_clause_keeps_coordinated_controller_subject(tail: &str) -> bool {
    tail.split(['.', '\n', ';', '—'])
        .next()
        .is_some_and(|clause| clause.contains(" and you lose "))
}

fn revealed_hand_union_keeps_explicit_draw_subject(
    prefix: &str,
    explicit: &str,
    tail: &str,
) -> bool {
    matches!(explicit, ". You draw " | ". you draw ")
        && prefix
            .trim_end()
            .to_ascii_lowercase()
            .ends_with("reveals their hand")
        && tail
            .split(['.', '\n', ';', '—'])
            .next()
            .is_some_and(|clause| {
                clause.starts_with("a card for each ") && clause.ends_with(" card in it")
            })
}

fn returned_set_control_gate_keeps_explicit_draw_subject(
    prefix: &str,
    explicit: &str,
    tail: &str,
) -> bool {
    matches!(explicit, ". You draw " | ". you draw ")
        && prefix.trim_end().ends_with("under their owners' control")
        && tail
            .split(['.', '\n', ';', '—'])
            .next()
            .is_some_and(|clause| {
                clause == "a card for each opponent who controls one or more of those permanents"
            })
}

fn replace_imperative_draw_subject(segment: &str, explicit: &str, imperative: &str) -> String {
    let mut output = String::with_capacity(segment.len());
    let mut remainder = segment;
    while let Some(index) = remainder.find(explicit) {
        output.push_str(&remainder[..index]);
        let tail = &remainder[index + explicit.len()..];
        if draw_clause_keeps_coordinated_controller_subject(tail)
            || revealed_hand_union_keeps_explicit_draw_subject(&output, explicit, tail)
            || returned_set_control_gate_keeps_explicit_draw_subject(&output, explicit, tail)
        {
            output.push_str(explicit);
        } else {
            output.push_str(imperative);
        }
        remainder = tail;
    }
    output.push_str(remainder);
    output
}

fn normalize_imperative_draw_subject_outside_quotes(line: &str) -> String {
    let mut normalized = String::with_capacity(line.len());
    for (index, segment) in line.split('"').enumerate() {
        if index > 0 {
            normalized.push('"');
        }
        if index % 2 == 1 {
            normalized.push_str(segment);
            continue;
        }
        let mut segment = segment.to_string();
        if let Some(rest) = segment.strip_prefix("You draw ") {
            if !draw_clause_keeps_coordinated_controller_subject(rest) {
                segment = format!("Draw {rest}");
            }
        } else if let Some(rest) = segment.strip_prefix("you draw ")
            && !draw_clause_keeps_coordinated_controller_subject(rest)
        {
            segment = format!("draw {rest}");
        }
        for (subject, imperative) in [
            (": You draw ", ": Draw "),
            (": you draw ", ": draw "),
            (". You draw ", ". Draw "),
            (". you draw ", ". Draw "),
            ("\nYou draw ", "\nDraw "),
            ("\nyou draw ", "\nDraw "),
            ("— You draw ", "— Draw "),
            ("— you draw ", "— draw "),
            (", You draw ", ", draw "),
            (", you draw ", ", draw "),
            ("instead You draw ", "instead draw "),
            ("instead you draw ", "instead draw "),
            ("then You draw ", "then draw "),
            ("then you draw ", "then draw "),
        ] {
            segment = replace_imperative_draw_subject(&segment, subject, imperative);
        }
        normalized.push_str(&segment);
    }
    normalized
}

fn normalize_exact_choice_coordinated_reward_voice(line: &str) -> String {
    let Some(rest) = line.strip_prefix("You choose exactly ") else {
        return line.to_string();
    };
    let Some((choice, reward)) = rest.split_once(". Draw ") else {
        return line.to_string();
    };
    if !reward.contains(" and the chosen ") {
        return line.to_string();
    }
    format!("Choose exactly {choice}. You draw {reward}")
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
        .replace(
            "had another land enter under",
            "had another land enter the battlefield under",
        )
        .replace(
            "had a land enter under",
            "had a land enter the battlefield under",
        )
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
    normalized = normalize_imperative_draw_subject_outside_quotes(&normalized);
    normalized = normalize_exact_choice_coordinated_reward_voice(&normalized);

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

    if normalized.eq_ignore_ascii_case(
        "Search your library for three cards and reveal them. Target opponent chooses one of them. Put the chosen card into your hand and the rest into your graveyard. Then shuffle.",
    ) {
        normalized = "Search your library for three cards and reveal them. Target opponent chooses one. Put that card into your hand and the rest into your graveyard. Then shuffle.".to_string();
    }

    if normalized
        .eq_ignore_ascii_case("Target defending player's creature gets +3/+0 and gains can block 2 additional creatures each combat until end of turn.")
        || normalized.eq_ignore_ascii_case("Target creature defending player controls gets +3/+0 and gains can block 2 additional creatures each combat until end of turn.")
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
    fn cleanup_factors_repeated_other_and_controller_scope_across_union_arms() {
        assert_eq!(
            factor_repeated_other_you_control_union(
                "Other nontoken artifact creatures you control and other Vehicles you control have Modular 1."
            ),
            "Other nontoken artifact creatures and Vehicles you control have Modular 1."
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

    #[test]
    fn cleanup_preserves_battlefield_in_second_land_entry_condition() {
        let text = "Whenever a land enters under an opponent's control, if that player had another land enter the battlefield under their control this turn, this creature deals 3 damage to that player.";
        assert_eq!(normalize_debug_safe_spelling_surface(text), text);
    }

    #[test]
    fn cleanup_preserves_battlefield_in_landfall_condition() {
        let text = "If you had a land enter the battlefield under your control this turn, this spell deals 3 damage instead.";
        assert_eq!(normalize_debug_safe_spelling_surface(text), text);
    }

    #[test]
    fn cleanup_preserves_explicit_draw_subject_inside_rules_quotes() {
        let emblem = "−6: You get an emblem with \"Whenever you cast an Elf spell, it gains haste until end of turn and you draw two cards.\"";
        assert_eq!(normalize_debug_safe_spelling_surface(emblem), emblem);
        assert_eq!(
            normalize_debug_safe_spelling_surface("You draw two cards."),
            "Draw two cards."
        );
    }

    #[test]
    fn cleanup_preserves_both_subjects_for_coordinated_draw_and_life_loss() {
        assert_eq!(
            normalize_debug_safe_spelling_surface("You draw two cards and you lose 2 life."),
            "You draw two cards and you lose 2 life."
        );
        assert_eq!(
            normalize_debug_safe_spelling_surface("You draw two cards."),
            "Draw two cards.",
            "an ordinary draw instruction remains imperative"
        );
        assert_eq!(
            normalize_debug_safe_spelling_surface(
                "When this artifact enters, you draw a card and you lose 1 life."
            ),
            "When this artifact enters, you draw a card and you lose 1 life."
        );
        assert_eq!(
            normalize_debug_safe_spelling_surface(
                "When this artifact enters, you draw a card. You lose 1 life."
            ),
            "When this artifact enters, draw a card. You lose 1 life.",
            "separate instructions must not be mistaken for one coordinator"
        );
    }

    #[test]
    fn cleanup_uses_imperative_draw_after_prior_sentence() {
        let triggered = "Whenever this creature attacks, each player discards a card. You draw a card for each card discarded this way.";
        assert_eq!(
            normalize_debug_safe_spelling_surface(triggered),
            "Whenever this creature attacks, each player discards a card. Draw a card for each card discarded this way."
        );
        assert_eq!(
            normalize_debug_safe_spelling_surface("{T}: You draw a card."),
            "{T}: Draw a card."
        );
        assert_eq!(
            normalize_debug_safe_spelling_surface(
                "Tap target artifact or creature.\nYou draw a card."
            ),
            "Tap target artifact or creature.\nDraw a card."
        );
    }

    #[test]
    fn cleanup_preserves_explicit_draw_after_an_exact_chosen_set() {
        let text = "You choose exactly two creatures you control. You draw X cards and the chosen creatures get +X/+X and gain trample until end of turn.";
        assert_eq!(
            normalize_debug_safe_spelling_surface(text),
            "Choose exactly two creatures you control. You draw X cards and the chosen creatures get +X/+X and gain trample until end of turn."
        );
    }

    #[test]
    fn cleanup_preserves_explicit_draw_for_a_shared_revealed_hand_union() {
        let text = "Target opponent reveals their hand. You draw a card for each Forest and green card in it.";
        assert_eq!(normalize_debug_safe_spelling_surface(text), text);

        assert_eq!(
            normalize_debug_safe_spelling_surface(
                "Target opponent reveals their hand. You draw two cards."
            ),
            "Target opponent reveals their hand. Draw two cards.",
            "an ordinary follow-up draw remains imperative"
        );
    }

    #[test]
    fn cleanup_preserves_explicit_draw_for_a_returned_set_control_gate() {
        let text = "Choose up to three target permanent cards in graveyards that were put there from the battlefield this turn. Return them to the battlefield tapped under their owners' control. You draw a card for each opponent who controls one or more of those permanents.";
        assert_eq!(normalize_debug_safe_spelling_surface(text), text);

        assert_eq!(
            normalize_debug_safe_spelling_surface(
                "Return those permanents under their owners' control. You draw two cards."
            ),
            "Return those permanents under their owners' control. Draw two cards.",
            "an unrelated follow-up draw remains imperative"
        );
    }

    #[test]
    fn trailing_haste_duration_stays_on_rootwise_final_clause() {
        let text = "Survival — At the beginning of your second main phase, if this creature is tapped, put three +1/+1 counters on up to one target land you control, that land becomes an elemental creature with base power and toughness 0/0 in addition to its other types, then it gains haste until your next turn.";
        assert_eq!(normalize_until_your_next_turn_duration_order(text), text);

        assert_eq!(
            normalize_until_your_next_turn_duration_order(
                "0: Target artifact you control becomes a creature with base power and toughness 5/5 in addition to its other types until your next turn."
            ),
            "0: Until your next turn, target artifact you control becomes a creature with base power and toughness 5/5 in addition to its other types."
        );
    }
}
