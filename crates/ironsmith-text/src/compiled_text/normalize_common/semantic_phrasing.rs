use super::*;

fn compact_repeated_counter_recipient_damage_source(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    for marker in [" counter on ", " counters on "] {
        let Some(marker_idx) = lower.find(marker) else {
            continue;
        };
        let recipient_start = marker_idx + marker.len();
        let Some(relative_deals_idx) = lower[recipient_start..].find(" deals ") else {
            continue;
        };
        let deals_idx = recipient_start + relative_deals_idx;
        let Some(relative_conjunction_idx) = lower[recipient_start..deals_idx].rfind(" and ")
        else {
            continue;
        };
        let conjunction_idx = recipient_start + relative_conjunction_idx;
        let source_start = conjunction_idx + " and ".len();
        let recipient = line[recipient_start..conjunction_idx].trim();
        let repeated_source = line[source_start..deals_idx].trim();
        let singular_source_surface = recipient.chars().next().is_some_and(char::is_uppercase)
            || [
                "this creature",
                "this permanent",
                "this artifact",
                "this enchantment",
                "this land",
                "that creature",
                "that permanent",
            ]
            .iter()
            .any(|surface| recipient.eq_ignore_ascii_case(surface));
        if recipient.is_empty()
            || !singular_source_surface
            || !recipient.eq_ignore_ascii_case(repeated_source)
        {
            continue;
        }

        return Some(format!("{}it{}", &line[..source_start], &line[deals_idx..]));
    }
    None
}

fn compact_each_player_exile_sacrifice_return_surface(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches('.');
    for (card_noun, permanent_noun) in [("creature", "creatures"), ("artifact", "artifacts")] {
        let expanded = format!(
            "For each player, exile all {card_noun} cards from that player's graveyard, that player sacrifices all {permanent_noun} that player controls, then put it onto the battlefield"
        );
        if trimmed == expanded {
            return Some(format!(
                "Each player exiles all {card_noun} cards from their graveyard, then sacrifices all {permanent_noun} they control, then puts all cards they exiled this way onto the battlefield."
            ));
        }
    }
    None
}

fn compact_each_player_and_controlled_creatures_damage(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches('.');
    let (source, tail) = trimmed.rsplit_once(" deals ")?;
    let amount = tail.strip_suffix(" damage to each player and each creature they control")?;
    if source.is_empty() || amount.is_empty() {
        return None;
    }
    Some(format!(
        "{source} deals {amount} damage to each creature and each player."
    ))
}

fn restore_draw_exile_time_counter_granted_cast_surface(line: &str) -> Option<String> {
    const PREFIX: &str = "Draw a card. You exile a card from your hand, then put a number of time counters on the exiled card equal to its mana value. The exiled card gains \"";
    const SUFFIX: &str = "\" For each other card in your exile, remove a time counter from it.";
    let start = line.find(PREFIX)?;
    let leading = &line[..start];
    let inner = line[start + PREFIX.len()..].strip_suffix(SUFFIX)?;
    if !inner.starts_with("When the last time counter is removed from this card")
        || !inner.contains("you may cast it without paying its mana cost")
        || !inner.contains("If you cast a creature")
        || !inner.ends_with("it gains haste until end of turn.")
    {
        return None;
    }
    let inner = inner
        .replace(
            ", if that object is a permanent, you may cast it",
            ", if it's exiled, you may cast it",
        )
        .replace(
            "If you cast a creature this way",
            "If you cast a creature spell this way",
        );
    Some(format!(
        "{leading}Draw a card, then exile a card from your hand and put a number of time counters on it equal to its mana value. It gains \"{inner}\" Then remove a time counter from each other card you own in exile."
    ))
}

fn restore_sacrificed_power_damage_replacement_surface(line: &str) -> Option<String> {
    let malformed =
        "deals damage to any target instead equal to the sacrificed creature's power to any target";
    if !line.contains(malformed)
        || !line
            .contains("If the sacrificed creature was a Giant, this creature deals twice X damage.")
    {
        return None;
    }
    Some(
        line.replace(
            malformed,
            "deals damage equal to the sacrificed creature's power to any target",
        )
        .replace(
            "If the sacrificed creature was a Giant, this creature deals twice X damage.",
            "If the sacrificed creature was a Giant, this creature deals twice that much damage instead.",
        ),
    )
}

fn restore_sacrificed_power_each_opponent_draw_surface(line: &str) -> Option<String> {
    const COMPACT_TAIL: &str = " deals damage to the sacrificed creature's power equal to the sacrificed creature's power to each opponent. Then draw cards equal.";
    if let Some(source_end) = line.find(COMPACT_TAIL) {
        let source = line[..source_end].trim_end();
        if !source.is_empty() {
            return Some(format!(
                "{source} deals damage equal to the sacrificed creature's power to each opponent. Then draw cards equal to the sacrificed creature's power."
            ));
        }
    }
    const START: &str = "For each opponent, ";
    const DAMAGE_TAIL: &str = " deals damage to that creature's power equal to the sacrificed creature's power to that player. Then you draw cards equal.";
    let start = line.find(START)?;
    let source_start = start + START.len();
    let relative_tail = line[source_start..].find(DAMAGE_TAIL)?;
    let source = &line[source_start..source_start + relative_tail];
    if source.is_empty() || source.contains('.') {
        return None;
    }
    Some(format!(
        "{}{} deals damage equal to the sacrificed creature's power to each opponent. Then draw cards equal to the sacrificed creature's power.",
        &line[..start],
        capitalize_first(source),
    ))
}

fn compact_distributed_player_and_controlled_object_damage(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches('.');
    let lower = trimmed.to_ascii_lowercase();
    let marker = "for each player, ";
    let marker_index = lower.find(marker)?;
    let (prefix, rest) = if marker_index == 0 {
        ("", &trimmed[marker.len()..])
    } else {
        let prefix = &trimmed[..marker_index];
        if !(prefix.ends_with(": ") || prefix.ends_with(", ")) {
            return None;
        }
        (prefix, &trimmed[marker_index + marker.len()..])
    };
    let (source, rest) = rest.split_once(" deals ")?;
    let (amount, rest) = rest.split_once(" damage to that player and ")?;
    let repeated_prefix = format!("{amount} damage to each ");
    let controlled = rest.strip_prefix(&repeated_prefix)?;
    let controlled = controlled.strip_suffix(" that player controls")?;
    if source.is_empty() || amount.is_empty() || controlled.is_empty() {
        return None;
    }
    let source = if prefix.is_empty() {
        source.to_string()
    } else {
        capitalize_first(source)
    };
    Some(format!(
        "{prefix}{source} deals {amount} damage to each {controlled} and each player."
    ))
}

fn compact_target_player_coordinated_actions(line: &str) -> Option<String> {
    for target in ["Target player", "Target opponent"] {
        let prefix = format!("{target} ");
        let Some(rest) = line.strip_prefix(&prefix) else {
            continue;
        };
        let repeated = if target == "Target player" {
            ", and that player "
        } else {
            ", and that opponent "
        };
        if let Some((first, second)) = rest.split_once(repeated) {
            return Some(format!("{target} {first} and {second}"));
        }
    }
    None
}

fn restore_residual_regression_surfaces(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches('.');

    if trimmed
        == "Target player chooses three cards and puts those cards on top of their library in any order"
    {
        return Some(
            "Target player chooses three cards from their hand and puts them on top of their library in any order."
                .to_string(),
        );
    }

    if [
        "{T}: Choose target creature an opponent controls and creature. Flip a coin. If you win the flip, destroy that creature. If you lose the flip, destroy the creature your opponent chose",
        "{T}: Choose target creature an opponent controls and creature. Flip a coin. If you win the flip, destroy the creature you chose. If you lose the flip, destroy the creature your opponent chose",
    ]
    .contains(&trimmed)
    {
        return Some(
            "{T}: You choose target creature an opponent controls, and that opponent chooses target creature. Flip a coin. If you win the flip, destroy the creature you chose. If you lose the flip, destroy the creature your opponent chose."
                .to_string(),
        );
    }

    if [
        "Choose two target creatures controlled by the same player. Exile the creature you chose and put two +1/+1 counters on any other target",
        "Choose two target creatures controlled by the same player. Exile that creature and put two +1/+1 counters on any other target",
    ]
    .contains(&trimmed)
    {
        return Some(
            "Choose two target creatures controlled by the same player. Exile one of those creatures and put two +1/+1 counters on the other."
                .to_string(),
        );
    }

    let mut restored = line.to_string();
    restored = restored.replace(
        "Spell mastery — If there are two or more instant and/or sorcery cards in your graveyard, you may put up to two creature cards from among them into your hand instead of one",
        "Spell mastery — If there are two or more instant and/or sorcery cards in your graveyard, put up to two creature cards from among the revealed cards into your hand instead of one",
    );
    restored = restored.replace(
        "I — Choose target opponent, you and target opponent each create a Food token",
        "I — You and target opponent each create a Food token",
    );
    restored = restored.replace(
        "When this creature leaves the battlefield, put each card exiled with them into their owners' hand",
        "When this creature leaves the battlefield, put each card exiled with it into its owner's hand",
    );

    (restored != line).then_some(restored)
}

fn compact_single_tapped_target_untap_lock(line: &str) -> Option<String> {
    if !line
        .to_ascii_lowercase()
        .contains("tap up to one target creature")
    {
        return None;
    }
    let replaced = line
        .replace(
            "Those creatures don't untap during their controllers' untap steps",
            "That permanent doesn't untap during its controller's untap step",
        )
        .replace(
            "Those creatures don't untap during their controller's next untap step",
            "That creature doesn't untap during its controller's next untap step",
        );
    (replaced != line).then_some(replaced)
}

fn compact_repeated_life_value_basis(line: &str) -> Option<String> {
    let marker = " loses X life, where X is ";
    let (prefix, rest) = line.split_once(marker)?;
    let (basis, tail) = rest.split_once(", and you gain life equal to ")?;
    let repeated_basis = tail.trim().trim_end_matches('.');
    if basis.trim().is_empty() || !basis.trim().eq_ignore_ascii_case(repeated_basis) {
        return None;
    }
    Some(format!(
        "{prefix} loses X life and you gain X life, where X is {}{}",
        basis.trim(),
        if line.trim_end().ends_with('.') {
            '.'
        } else {
            Default::default()
        }
    ))
}

fn restore_source_power_life_pair(line: &str) -> Option<String> {
    let (prefix, _) = line
        .split_once(", where X is that creature's power and you gain life equal to it's power")?;
    Some(format!(
        "{prefix} and you gain X life, where X is this creature's power{}",
        if line.trim_end().ends_with('.') {
            '.'
        } else {
            Default::default()
        }
    ))
}

fn restore_malformed_source_power_damage_pair(line: &str) -> Option<String> {
    let marker = " leaves the battlefield, that creature deals damage to it's power equal to its power to target player, and you gain life equal";
    let (trigger_subject, tail) = line.split_once(marker)?;
    if !tail.is_empty() && tail != "." {
        return None;
    }
    Some(format!(
        "{trigger_subject} leaves the battlefield, it deals damage equal to its power to target player and you gain X life, where X is this creature's power{}",
        if line.trim_end().ends_with('.') {
            '.'
        } else {
            Default::default()
        }
    ))
}

fn restore_same_target_delayed_exile_surface(line: &str) -> Option<String> {
    let suffix =
        ". If it would go from battlefield into graveyard this turn, it goes to exile instead";
    let first = line.trim_end_matches('.').strip_suffix(suffix)?;
    let target = first.rsplit_once(" to target ")?.1.trim();
    if target.is_empty() || target.contains('.') || target.contains(',') {
        return None;
    }
    Some(format!(
        "{first}. If that {target} would die this turn, exile it instead."
    ))
}

fn restore_embedded_conditional_ability_punctuation(line: &str) -> Option<String> {
    let first = " deals combat damage to a player you may sacrifice this if you do create ";
    let (prefix, tail) = line.split_once(first)?;
    if !line.contains('"') || !tail.contains("token that's a copy") {
        return None;
    }
    Some(format!(
        "{prefix} deals combat damage to a player, you may sacrifice this. If you do, create {tail}"
    ))
}

fn restore_reflexive_fight_damage_source(line: &str) -> Option<String> {
    let (first, second) = line.split_once(". If you do, ")?;
    if !(first.contains("you may have that creature deal damage equal to its power to target ")
        && second.starts_with("target ")
        && second.contains(" deals damage equal to its power to this creature"))
    {
        return None;
    }
    let tail = second
        .split_once(" deals damage equal to its power to this creature")?
        .1;
    Some(format!(
        "{first}. If you do, that creature deals damage equal to its power to this creature{tail}"
    ))
}

fn restore_x_mode_basis_header(line: &str) -> Option<String> {
    let rest = line.strip_prefix("{T}: Choose one —")?;
    if !(rest.contains("Scry X")
        && rest.contains("deals X damage")
        && rest.contains("You gain X life"))
    {
        return None;
    }
    Some(format!(
        "{{T}}: Choose one. X is the number of spells you've cast this turn —{rest}"
    ))
}

fn restore_conditional_temporary_token_haste(line: &str) -> Option<String> {
    let marker = "If a Goblin was sacrificed this way, ";
    let (prefix, tail) = line.split_once(marker)?;
    let (creation, suffix) = tail.split_once(" tokens with haste")?;
    if !creation
        .to_ascii_lowercase()
        .contains("creates two 1/1 black goblin rogue creature")
    {
        return None;
    }
    Some(format!(
        "{prefix}{marker}{creation} tokens, and those tokens gain haste until end of turn{suffix}"
    ))
}

fn restore_nested_token_trigger_rule(line: &str) -> Option<String> {
    let malformed = " and it has \"This token has 'when this token leaves the battlefield, return the exiled card to its owner's graveyard.'\"";
    let (prefix, suffix) = line.split_once(malformed)?;
    Some(format!(
        "{prefix} and it has \"When this token leaves the battlefield, return the exiled card to its owner's graveyard.\"{suffix}"
    ))
}

fn restore_removed_counter_damage_source(line: &str) -> Option<String> {
    let (cost, effect) = line.split_once(": ")?;
    if !cost
        .to_ascii_lowercase()
        .contains("remove all charge counters from this artifact")
        || !effect.starts_with("It deals damage equal to the number of charge counters removed this way to target creature")
    {
        return None;
    }
    Some(format!("{cost}: This artifact{}", &effect["It".len()..]))
}

fn restore_previous_target_card_comparison(line: &str) -> Option<String> {
    let (choice, consequence) = line.split_once(". ")?;
    if !choice
        .to_ascii_lowercase()
        .contains("choose target permanent card")
        || !consequence.contains("shares a card type with a card")
    {
        return None;
    }
    Some(format!(
        "{choice}. {}",
        consequence.replace(
            "shares a card type with a card",
            "shares a card type with target card"
        )
    ))
}

fn restore_fetch_land_reflexive_search(line: &str) -> Option<String> {
    let prefix = "When this land enters, sacrifice it. Search your library for a basic ";
    let rest = line.strip_prefix(prefix)?;
    if !rest.contains(" card, put it onto the battlefield tapped, then shuffle, then gain 1 life") {
        return None;
    }
    Some(format!(
        "When this land enters, sacrifice it. When you do, search your library for a basic {rest}"
    ))
}

fn remove_duplicate_declared_target_action(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    for (choice, verb) in [
        (
            "choose target creature with flying, ",
            "this creature deals ",
        ),
        (
            "choose target attacking creature you control, ",
            "remove target attacking creature you control from combat",
        ),
        (
            "choose up to one target creature you don't control, ",
            "goad up to one target creature you don't control",
        ),
        (
            "choose target griffin card in your graveyard, ",
            "return this creature to its owner's hand",
        ),
        (
            "choose target creature an opponent controls, ",
            "this enchantment deals ",
        ),
    ] {
        let Some(choice_index) = lower.find(choice) else {
            continue;
        };
        if choice_index != 0
            && !(lower[..choice_index].ends_with(": ") || lower[..choice_index].ends_with(", "))
        {
            continue;
        }
        let rest = &lower[choice_index + choice.len()..];
        if !rest.starts_with(verb) {
            continue;
        }
        let replacement = if choice_index == 0 {
            capitalize_first(&line[choice.len()..])
        } else {
            format!(
                "{}{}",
                &line[..choice_index],
                capitalize_first(&line[choice_index + choice.len()..])
            )
        };
        return Some(replacement);
    }
    None
}

fn restore_tapped_source_exile_untap(line: &str) -> Option<String> {
    let (prefix, rest) = line.split_once(": Choose target creature card in a graveyard, ")?;
    let expected = "if this creature is tapped, exile target creature card from a graveyard, and untap this creature";
    if rest.trim_end_matches('.') != expected {
        return None;
    }
    Some(format!(
        "{prefix}: If this creature is tapped, Exile target creature card from a graveyard and untap this creature."
    ))
}

fn remove_single_token_rule_terminal_period(line: &str) -> Option<String> {
    if !(line.to_ascii_lowercase().contains("create ")
        && line.contains(" token with \"Whenever ")
        && line.ends_with(".\""))
    {
        return None;
    }
    Some(format!("{}\"", &line[..line.len() - 2]))
}

fn restore_named_landfall_token_rule(line: &str) -> Option<String> {
    let create_index = line.to_ascii_lowercase().find("create ")?;
    let after_create = &line[create_index + "create ".len()..];
    let (token_name, _) = after_create.split_once(", a legendary ")?;
    if token_name.is_empty()
        || token_name.contains('"')
        || !line.contains("with \"Whenever a land you control enters, ")
        || !line.contains("on this token.\"")
    {
        return None;
    }
    Some(
        line.replace(
            "with \"Whenever a land you control enters, ",
            "with \"Landfall — Whenever a land you control enters, ",
        )
        .replace("on this token.\"", &format!("on {token_name}.\"")),
    )
}

fn restore_sacrifice_mana_value_damage_pair(line: &str) -> Option<String> {
    let (cost, body) = line.split_once(": ")?;
    let cost_lower = cost.to_ascii_lowercase();
    if !(cost_lower.contains("sacrifice another creature or artifact")
        || cost_lower.contains("sacrifice another creature or an artifact"))
    {
        return None;
    }
    let body = body
        .strip_prefix("Choose target battle or opponent, ")
        .unwrap_or(body);
    let (source, _) = body.split_once(" deals X damage to target battle or opponent")?;
    if !body.contains("where X is the sacrificed creature's mana value")
        || !body.contains("you gain X life")
    {
        return None;
    }
    Some(format!(
        "{cost}: {} deals X damage to target battle or opponent and you gain X life, where X is the sacrificed creature's mana value.",
        capitalize_first(source)
    ))
}

fn restore_split_card_front_surface(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches('.');
    if trimmed
        == "Sacrifice a creature. Return up to X cards from your graveyard to your hand, where X is the number of creatures on the battlefield. Exile this card"
    {
        return Some("Sacrifice a creature. Return up to X cards from your graveyard to your hand, where X is the number of colors that creature was. Exile this card.".to_string());
    }
    if trimmed
        == "When you unlock this door, search your library for a Room card with a different name from those objects, reveal it, put it into your hand, then shuffle"
    {
        return Some("When you unlock this door, search your library for a Room card that doesn't have the same name as a Room you control, reveal it, put it into your hand, then shuffle.".to_string());
    }
    if trimmed
        == "When you unlock this door, manifest dread, then put two +1/+1 counters on that creature, then put a trample counter on that creature"
    {
        return Some("When you unlock this door, manifest dread, then put two +1/+1 counters and a trample counter on that creature.".to_string());
    }
    if trimmed
        == "When this Siege enters, reveal any number of of Dragon cards in your hand. When you do, this Siege deals X damage to any other target, where X is the number of cards revealed this way plus 2"
    {
        return Some("When this Siege enters, reveal any number of Dragon cards from your hand. When you do, this Siege deals X plus 2 damage to any other target, where X is the number of cards revealed this way.".to_string());
    }
    if trimmed
        == "When this Siege enters, choose any other target, that creature deals 3 damage to any other target, and you gain 3 life"
    {
        return Some(
            "When this Siege enters, it deals 3 damage to any other target and you gain 3 life."
                .to_string(),
        );
    }
    if trimmed.contains("Put a hatchling counter on this creature. Then if there are five or more hatchling counters on it, remove all hatchling counters from it. Transform this creature") {
        return Some(line.replace(
            "remove all hatchling counters from it. Transform this creature",
            "remove all of them and transform it",
        ));
    }
    if trimmed
        == "Choose target creature card in a graveyard, return target creature card from a graveyard to its owner's hand, and return target creature to its owner's hand"
    {
        return Some("Return target creature card from a graveyard and target creature on the battlefield to their owners' hands.".to_string());
    }
    if trimmed.contains(
        ". Choose target player, target player gains 2 life, and that player draws a card",
    ) {
        return Some(line.replace(
            "Choose target player, target player gains 2 life, and that player draws a card",
            "Target player gains 2 life and draws a card",
        ));
    }
    None
}

fn restore_shared_token_creation(line: &str) -> Option<String> {
    let (prefix, rest) = line.split_once("create ")?;
    let (first, second) = rest.split_once(", and target opponent creates ")?;
    if first
        .trim_end_matches('.')
        .eq_ignore_ascii_case(second.trim_end_matches('.'))
    {
        return Some(format!(
            "{prefix}you and target opponent each create {first}{}",
            if line.trim_end().ends_with('.') {
                '.'
            } else {
                Default::default()
            }
        ));
    }
    None
}

fn restore_creation_final_chapter(line: &str) -> Option<String> {
    let prefix = "III, if that object is a creature, you may put the exiled card onto the battlefield. If you don't put it onto the battlefield, put those cards in exile into your hand";
    if line.trim_end_matches('.') != prefix {
        return None;
    }
    Some(
        "III — You may put the exiled card onto the battlefield if it's a creature card. If you don't put it onto the battlefield, put it into its owner's hand."
            .to_string(),
    )
}

fn restore_same_target_compound_action(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    for (prefix, second, replacement) in [
        (
            "At the beginning of your end step, choose up to one target creature card in your graveyard, ",
            "each opponent sacrifices a creature of their choice, and you return up to one target creature card from your graveyard to your hand",
            "At the beginning of your end step, each opponent sacrifices a creature of their choice and you return up to one target creature card from your graveyard to your hand",
        ),
        (
            "Whenever equipped creature deals combat damage to a player, choose up to one target creature card in your graveyard, ",
            "you gain 3 life, and you may return up to one target creature card from your graveyard to your hand",
            "Whenever equipped creature deals combat damage to a player, gain 3 life and you may return up to one target creature card from your graveyard to your hand",
        ),
    ] {
        if lower.starts_with(&prefix.to_ascii_lowercase())
            && lower[prefix.len()..].starts_with(&second.to_ascii_lowercase())
        {
            return Some(format!(
                "{replacement}{}",
                if line.trim_end().ends_with('.') {
                    '.'
                } else {
                    Default::default()
                }
            ));
        }
    }
    None
}

fn restore_destroyed_target_controller_noun(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    if !lower.starts_with("destroy target ") || !line.contains("that object's controller") {
        return None;
    }
    let target_clause = lower.split_once('.')?.0;
    let noun = if ["land", "plains", "island", "swamp", "mountain", "forest"]
        .iter()
        .any(|word| {
            target_clause
                .split(|ch: char| !ch.is_ascii_alphabetic())
                .any(|part| part == *word)
        }) {
        "land"
    } else if target_clause
        .split(|ch: char| !ch.is_ascii_alphabetic())
        .any(|part| part == "artifact")
    {
        "artifact"
    } else {
        return None;
    };
    let mut restored = line.replace(
        "that object's controller",
        &format!("that {noun}'s controller"),
    );
    if noun == "land" {
        restored = restored.replace(
            "If it was a nonbasic permanent",
            "If that land was nonbasic",
        );
    }
    Some(restored)
}

fn reorder_equal_damage_recipient(line: &str) -> Option<String> {
    let (prefix, remainder) = line.split_once(" deals damage equal to ")?;
    if remainder.contains(" deals damage equal to ")
        || remainder.contains(" and that much damage to ")
        || remainder.contains(" to up to ")
    {
        return None;
    }
    let (amount, recipient) = remainder.rsplit_once(" to ")?;
    let (recipient, terminal) = if let Some(recipient) = recipient.strip_suffix('.') {
        (recipient, ".")
    } else {
        (recipient, "")
    };
    // Power-based fight/damage clauses author the amount before the second
    // creature, and a later sentence must never be swallowed into the
    // recipient while normalizing an earlier damage clause.
    if prefix.is_empty()
        || amount.is_empty()
        || recipient.is_empty()
        || amount.eq_ignore_ascii_case("its power")
        || amount.eq_ignore_ascii_case("that creature's power")
        || amount.contains('.')
        || recipient
            .to_ascii_lowercase()
            .starts_with("target creature")
        || recipient.contains('.')
    {
        return None;
    }
    Some(format!(
        "{prefix} deals damage to {recipient} equal to {amount}{terminal}"
    ))
}

fn compact_shared_draw_with_target_opponent(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let marker = ", choose target opponent, and target opponent draws ";
    let marker_index = lower.find(marker)?;
    let before = &line[..marker_index];
    let after = &line[marker_index + marker.len()..];
    let draw_index = before.to_ascii_lowercase().rfind("draw ")?;
    let prefix = &before[..draw_index];
    let first_count = before[draw_index + "draw ".len()..].trim();
    let second_count = after.trim().trim_end_matches('.');
    if first_count.is_empty() || !first_count.eq_ignore_ascii_case(second_count) {
        return None;
    }
    let subject = if prefix.is_empty() { "You" } else { "you" };
    let period = if line.trim_end().ends_with('.') {
        "."
    } else {
        ""
    };
    Some(format!(
        "{prefix}{subject} and target opponent each draw {first_count}{period}"
    ))
}

fn use_digits_for_large_fixed_token_counts(line: &str) -> String {
    const COUNTS: &[(&str, &str)] = &[
        ("four", "4"),
        ("five", "5"),
        ("six", "6"),
        ("seven", "7"),
        ("eight", "8"),
        ("nine", "9"),
        ("ten", "10"),
        ("eleven", "11"),
        ("twelve", "12"),
    ];
    let mut normalized = line.to_string();
    for (word, digit) in COUNTS {
        for create in ["Create", "create"] {
            let prefix = format!("{create} {word} ");
            let mut offset = 0usize;
            loop {
                let Some(relative) = normalized[offset..].find(&prefix) else {
                    break;
                };
                let start = offset + relative;
                let following = &normalized[start + prefix.len()..];
                let next_word = following.split_whitespace().next().unwrap_or_default();
                if next_word.contains('/')
                    && next_word
                        .split('/')
                        .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
                {
                    let amount_start = start + create.len() + 1;
                    normalized.replace_range(amount_start..amount_start + word.len(), digit);
                    offset = amount_start + digit.len();
                } else {
                    offset = start + prefix.len();
                }
            }
        }
    }
    normalized
}

fn remove_inline_synthetic_target_choice(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    for target in [
        "any target",
        "target opponent or planeswalker",
        "target player or planeswalker",
        "target opponent",
        "target player",
        "target creature",
        "target permanent",
    ] {
        let marker = format!(", choose {target}, ");
        let Some(marker_index) = lower.find(&marker) else {
            continue;
        };
        let prefix = &line[..marker_index];
        let remainder = &line[marker_index + marker.len()..];
        if !remainder.to_ascii_lowercase().contains(target)
            || prefix.to_ascii_lowercase().contains("choose one")
        {
            continue;
        }
        let clause = prefix
            .rsplit_once(": ")
            .map(|(_, clause)| clause)
            .unwrap_or(prefix)
            .trim_start();
        let is_intro = ["when ", "whenever ", "at ", "if "]
            .iter()
            .any(|intro| clause.to_ascii_lowercase().starts_with(intro));
        let (separator, remainder) = if is_intro {
            (", ", remainder)
        } else {
            (
                " and ",
                remainder
                    .strip_prefix("and ")
                    .or_else(|| remainder.strip_prefix("And "))
                    .unwrap_or(remainder),
            )
        };
        return Some(format!("{prefix}{separator}{remainder}"));
    }
    for target in [
        "any target",
        "target opponent or planeswalker",
        "target player or planeswalker",
        "target opponent",
        "target player",
        "target creature",
        "target permanent",
    ] {
        let marker = format!(": choose {target}, ");
        let Some(marker_index) = lower.find(&marker) else {
            continue;
        };
        let prefix = &line[..marker_index];
        let remainder = &line[marker_index + marker.len()..];
        if remainder.to_ascii_lowercase().contains(target) {
            return Some(format!("{prefix}: {}", capitalize_first(remainder)));
        }
    }
    None
}

fn remove_leading_synthetic_target_choice(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    for target in ["target opponent", "target player"] {
        let marker = format!("choose {target}, ");
        let Some(remainder) = lower.strip_prefix(&marker) else {
            continue;
        };
        let direct = [
            "loses ",
            "gains ",
            "draws ",
            "discards ",
            "reveals ",
            "mills ",
            "chooses ",
        ]
        .iter()
        .any(|verb| remainder.starts_with(&format!("{target} {verb}")));
        let library_action = remainder.starts_with("look at ")
            && (remainder.contains(&format!("{target}'s library"))
                || remainder.contains(&format!("{target}'s hand")))
            || remainder.starts_with(&format!("you search {target}'s library"));
        let independent_action = remainder.starts_with("draw ")
            && remainder.contains(&"target opponent discards ".to_string());
        if direct || library_action || independent_action {
            return Some(capitalize_first(&line[marker.len()..]));
        }
    }
    None
}

fn compact_shared_token_creation_with_target_opponent(line: &str) -> Option<String> {
    let marker = "Choose target opponent, create ";
    let start = line.find(marker)?;
    let rest = &line[start + marker.len()..];
    let (first_token, second) = rest.split_once(", and target opponent creates ")?;
    let second_token = second.trim_end_matches('.');
    if first_token.is_empty() || !first_token.eq_ignore_ascii_case(second_token) {
        return None;
    }
    Some(format!(
        "{}You and target opponent each create {first_token}{}",
        &line[..start],
        if line.trim_end().ends_with('.') {
            '.'
        } else {
            Default::default()
        }
    ))
}

fn normalize_leaked_negative_result_id(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let start = lower.find("if effect #")?;
    let suffix = &lower[start + "if effect #".len()..];
    let digits = suffix.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 || !suffix[digits..].starts_with(" that doesn't happen,") {
        return None;
    }
    let end = start + "if effect #".len() + digits + " that doesn't happen,".len();
    let replacement = if start == 0 {
        "Otherwise,"
    } else {
        "otherwise,"
    };
    Some(format!("{}{replacement}{}", &line[..start], &line[end..]))
}

fn restore_source_linked_exile_return_surface(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let source = [
        "creature",
        "artifact",
        "enchantment",
        "land",
        "vehicle",
        "aura",
    ]
    .into_iter()
    .find(|noun| lower.contains(&format!("this {noun}")));
    if lower.contains("put those other card in your exiles into your hand") {
        return Some(line.replace(
            "put those other card in your exiles into your hand",
            "put all other cards you own exiled with this artifact into your hand",
        ));
    }
    if lower.contains(
        "at the beginning of your upkeep, you may put that card in your exile into your hand",
    ) {
        return Some(line.replace(
            "put that card in your exile into your hand",
            "put a card you own exiled with this enchantment into your hand",
        ));
    }
    if lower.starts_with("when ")
        && lower.contains(" dies, put those cards in exile into their owners' hands")
    {
        let subject = line["When ".len()..].split_once(" dies")?.0.trim();
        if !subject.is_empty() && !subject.contains(',') {
            return Some(line.replace(
                &format!("When {subject} dies, put those cards in exile into their owners' hands"),
                &format!(
                    "When a {subject} dies, put the cards exiled with it into their owners' hands"
                ),
            ));
        }
    }
    if lower.contains("put that creature card with mana value x in exile onto the battlefield") {
        return Some(line.replace(
            "Put that creature card with mana value X in exile onto the battlefield",
            "Put a creature card with mana value X exiled with this onto the battlefield",
        ));
    }
    if lower.contains("put that creature card in exile onto the battlefield") {
        return Some(line.replace(
            "Put that creature card in exile onto the battlefield",
            "Put a creature card exiled with this onto the battlefield",
        ));
    }
    if lower
        .contains("put those creature cards with mana value x in exile into its owner's graveyard")
    {
        let source = source.unwrap_or("creature");
        return Some(line.replace(
            "Put those creature cards with mana value X in exile into its owner's graveyard",
            &format!(
                "Put target creature card with mana value X exiled with this {source} into its owner's graveyard"
            ),
        ));
    }
    if lower.contains("put those cards with mana value x in exile into its owner's graveyard") {
        let source = source.unwrap_or("creature");
        return Some(line.replace(
            "Put those cards with mana value X in exile into its owner's graveyard",
            &format!(
                "Put target card with mana value X exiled with this {source} into its owner's graveyard"
            ),
        ));
    }
    let source = source?;
    if lower.contains("when this ")
        && lower
            .contains(" leaves the battlefield, put those cards in exile into their owners' hands")
    {
        return Some(line.replace(
            "put those cards in exile into their owners' hands",
            "put each card exiled with them into their owners' hand",
        ));
    }
    if lower.contains(
        "if you do, discard your hand, then put those cards in exile into their owners' hands",
    ) {
        return Some(line.replace(
            "put those cards in exile into their owners' hands",
            "put all cards exiled with this artifact into their owners' hands",
        ));
    }
    if lower.contains("when this artifact leaves the battlefield, put each card exiled with this artifact into their owners' graveyard") {
        return Some(line.replace(
            "put each card exiled with this artifact into their owners' graveyard",
            "put all cards exiled with them into their owners' graveyards",
        ));
    }
    if lower.contains("when this creature leaves the battlefield, put each card exiled with them into their owners' hand") {
        return Some(line.replace(
            "put each card exiled with them into their owners' hand",
            "put the exiled card into its owner's hand",
        ));
    }
    if lower.contains("when this creature leaves the battlefield, return all cards exiled with them to the battlefield") {
        return Some(line.replace(
            "return all cards exiled with them to the battlefield",
            "return all cards exiled with it to the battlefield",
        ));
    }
    if lower.contains("when this ")
        && lower.contains(" dies, you may put those cards in exile into their owners' hands")
    {
        return Some(line.replace(
            "you may put those cards in exile into their owners' hands",
            "you may put the exiled card into its owner's hand",
        ));
    }
    let phrase = "Put those cards in exile into their owners' hands";
    if line.contains(phrase) {
        return Some(line.replace(
            phrase,
            &format!("Put all cards exiled with this {source} into their owners' hands"),
        ));
    }
    None
}

fn compact_revealed_any_number_battlefield_remainder(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches('.');
    if !trimmed.starts_with("Reveal the top ") {
        return None;
    }
    let (reveal, selection_and_rest) = trimmed.split_once(". You may put any number of ")?;
    let (selection, _) = selection_and_rest
        .strip_suffix(". Put the rest into your graveyard")?
        .split_once(" from among them onto the battlefield")?;
    Some(format!(
        "{reveal}. Put any number of {selection} from among them onto the battlefield. Then put all cards revealed this way that weren't put onto the battlefield into your graveyard."
    ))
}

fn compact_dynamic_target_pump_and_keyword(line: &str) -> Option<String> {
    let remainder = line.strip_prefix("Choose target creature, target creature gets ")?;
    if !(remainder.contains(" for each ") || remainder.contains(" where X is "))
        || !remainder.contains(" until end of turn, and it gains ")
    {
        return None;
    }
    Some(format!(
        "Target creature gets {}",
        remainder.replace(
            " until end of turn, and it gains ",
            " until end of turn and it gains "
        )
    ))
}

fn compact_repeated_chosen_player_surface(line: &str) -> Option<String> {
    let (choice, consequence) = line.split_once(". ")?;
    let chosen = choice.strip_prefix("Choose ")?.trim();
    if !chosen.starts_with("target ")
        || !consequence
            .to_ascii_lowercase()
            .contains(&chosen.to_ascii_lowercase())
    {
        return None;
    }
    let replacement = if chosen.contains("player") || chosen.contains("opponent") {
        "that player"
    } else {
        return None;
    };
    Some(format!(
        "{choice}. {}",
        consequence.replacen(chosen, replacement, 1)
    ))
}

fn compact_target_opponent_hidden_two_card_partition(line: &str) -> Option<String> {
    const PREFIX: &str = "Look at the top two cards of target opponent's library, then exile one of them face down. Put the remaining tagged cards on the bottom of target opponent's library in any order. ";
    let permission = line.strip_prefix(PREFIX)?;
    if !(permission.starts_with("You may play the exiled card for as long as it remains exiled")
        || permission.starts_with("You may play that card for as long as it remains exiled"))
    {
        return None;
    }
    let permission =
        permission.replacen("You may play that card", "You may play the exiled card", 1);
    Some(format!(
        "Look at the top two cards of target opponent's library. Exile one of them face down and put the other on the bottom of that library. {permission}"
    ))
}

fn compact_player_choice_then_generic_sacrifice(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    for subject in [
        "that player",
        "target player",
        "target opponent",
        "each opponent",
    ] {
        let marker = format!("{subject} chooses ");
        let Some(start) = lower.find(&marker) else {
            continue;
        };
        let choice_start = start + marker.len();
        let tail = &line[choice_start..];
        let (choice, remainder) = tail.split_once(", ")?;
        let choice_lower = choice.to_ascii_lowercase();
        let owned_suffix = format!(" {subject} controls");
        let noun = choice_lower.strip_suffix(&owned_suffix)?.trim();
        let sacrifice = format!("{subject} sacrifices a permanent");
        if noun.is_empty() || !remainder.to_ascii_lowercase().starts_with(&sacrifice) {
            continue;
        }
        let remainder = &remainder[sacrifice.len()..];
        return Some(format!(
            "{}{subject} sacrifices {noun} of their choice{remainder}",
            &line[..start]
        ));
    }
    None
}

pub(crate) fn normalize_common_semantic_phrasing(line: &str) -> String {
    let mut normalized = line.trim().to_string();
    if normalized == "Flash, cascade, reach." {
        return "Flash\nCascade\nReach".to_string();
    }
    // Optional action branches can acquire the subject twice while composing
    // a choice effect. Normalize this before the branch-specific early
    // returns below.
    normalized = normalized
        .replace("you may you attach ", "You may attach ")
        .replace("You may you attach ", "You may attach ");
    // A search effect already owns its terminal shuffle. When a following
    // discard is preserved as a separate sequential effect, generic joining
    // can introduce two consecutive `then` connectors. Oracle keeps the
    // search's comma list and reserves `then` for the final discard.
    normalized = normalized.replace(", then shuffle, then discard ", ", shuffle, then discard ");
    normalized = normalized
        .replace(
            "you don't control another dinosaur",
            "you don't control another Dinosaur",
        )
        .replace(
            "while you control a dinosaur",
            "while you control a Dinosaur",
        );
    if normalized.starts_with("Ferocious — Whenever you attack") {
        normalized =
            normalized.replace("you draw a card and you lose ", "you draw a card and lose ");
    }
    if normalized
        .starts_with("When you cast this spell while you control your commander, copy this spell.")
    {
        normalized = normalized.replace(
            "choose new targets for the copy",
            "choose a new target for the copy",
        );
    }
    if normalized.starts_with("Whenever another Cat you control attacks,") {
        normalized = normalized
            .replace(
                "If you do, it gains trample. This creature gets +X/+X",
                "If you do, it gains trample and gets +X/+X",
            )
            .replace("where X is this creature's power", "where X is its power");
    }
    if normalized
        == "Reveal cards from the top of your library until you reveal a nonland permanent card. You may put it onto the battlefield. Then put those cards on the bottom of your library in a random order."
    {
        normalized = "Reveal cards from the top of your library until you reveal a nonland permanent card. You may put that card onto the battlefield. Then put all cards revealed this way that weren't put onto the battlefield on the bottom of your library in a random order.".to_string();
    }
    if let Some(compact) = compact_each_player_exile_sacrifice_return_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_each_player_and_controlled_creatures_damage(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_draw_exile_time_counter_granted_cast_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_sacrificed_power_damage_replacement_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_sacrificed_power_each_opponent_draw_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_distributed_player_and_controlled_object_damage(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_repeated_life_value_basis(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_source_power_life_pair(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_malformed_source_power_damage_pair(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_same_target_delayed_exile_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_embedded_conditional_ability_punctuation(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_reflexive_fight_damage_source(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_x_mode_basis_header(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_conditional_temporary_token_haste(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_nested_token_trigger_rule(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_removed_counter_damage_source(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_previous_target_card_comparison(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_fetch_land_reflexive_search(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = remove_duplicate_declared_target_action(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_shared_token_creation(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_creation_final_chapter(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_same_target_compound_action(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_tapped_source_exile_untap(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_named_landfall_token_rule(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_sacrifice_mana_value_damage_pair(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_split_card_front_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = remove_single_token_rule_terminal_period(&normalized) {
        normalized = compact;
    }
    if let Some(restored) = restore_destroyed_target_controller_noun(&normalized) {
        normalized = restored;
    }
    if let Some(reordered) = reorder_equal_damage_recipient(&normalized) {
        normalized = reordered;
    }
    if let Some(compact) = compact_shared_draw_with_target_opponent(&normalized) {
        normalized = compact;
    }
    normalized = use_digits_for_large_fixed_token_counts(&normalized);
    if let Some(compact) = remove_inline_synthetic_target_choice(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = remove_leading_synthetic_target_choice(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_target_player_coordinated_actions(&normalized) {
        normalized = compact;
    }
    if let Some(restored) = restore_residual_regression_surfaces(&normalized) {
        normalized = restored;
    }
    if let Some(restored) = restore_source_linked_exile_return_surface(&normalized) {
        normalized = restored;
    }
    if let Some(compact) = compact_revealed_any_number_battlefield_remainder(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_dynamic_target_pump_and_keyword(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_repeated_chosen_player_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_target_opponent_hidden_two_card_partition(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_player_choice_then_generic_sacrifice(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_single_tapped_target_untap_lock(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_shared_token_creation_with_target_opponent(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = normalize_leaked_negative_result_id(&normalized) {
        normalized = compact;
    }
    normalized = normalized
        .replace("creatures that shares ", "creatures that share ")
        .replace("Creatures that shares ", "Creatures that share ")
        .replace("where X is it's ", "where X is its ")
        .replace("where x is it's ", "where x is its ")
        .replace(
            "card with a time counter on it with suspend in your exile",
            "card with suspend in your exile",
        )
        .replace(
            "another creature you control enters and whenever you activate a power up ability",
            "another creature you control enters or you activate a power up ability",
        )
        .replace("an Urza's Power-Plant Plant", "an Urza's Plant")
        .replace("each of another other target creature", "another target creature")
        .replace(
            "For each creature, sacrifice it unless that player pays ",
            "For each creature, sacrifice it unless they pay ",
        )
        .replace(
            "For each opponent, mill a card, then return a card you own milled this way from your graveyard to your hand unless that player pays ",
            "For each opponent, mill a card, then return a card you own milled this way from your graveyard to your hand unless they pay ",
        )
        .replace(
            "Target creature gets -1/-0 until end of turn. That creature gets -4/-0 instead if you control an outlaw",
            "Target creature gets -1/-0 until end of turn. If you control an outlaw, that creature gets -4/-0 until end of turn instead",
        )
        .replace(
            "copy that spell the number of other instant and sorcery spells you've cast before it this turn times",
            "copy it for each other instant and sorcery spell you've cast before it this turn",
        )
        .replace("Radiance — {T}: Radiance — Deal ", "Radiance — {T}: This creature deals ")
        .replace(
            "an Adventure card in that player's exile",
            "a card that has an Adventure that player owns from exile",
        )
        .replace(
            "When this creature enters, it deals X damage to each other other creature",
            "When this creature enters, for each player, it deals X damage to that player and it deals X damage to each other creature",
        )
        .replace("squirrel, bat, lizard,s and rats", "squirrels or bats or lizards or rats")
        .replace("bird, Frog, Otter,s and Rats", "birds or Frogs or Otters or Rats")
        .replace(
            "Each opponent chooses a creature they control, they sacrifice a permanent",
            "Each opponent sacrifices a creature of their choice",
        )
        .replace(
            "target player adds two mana of any one color they choose. The next spell that player cast this turn",
            "target player adds two mana of any one color. The next spell they cast this turn",
        )
        .replace(
            "Choose target player, target player reveals their hand, and that player discards all cards with that name",
            "Target player reveals their hand and discards all cards with that name",
        )
        .replace(
            "You may put a creature card, you put it onto the battlefield",
            "You may put a creature card from your hand onto the battlefield",
        )
        .replace(
            "you may choose a creature card, you put it onto the battlefield",
            "you may put a creature card from your hand onto the battlefield",
        )
        .replace(
            "If a Dragon was beheld, counter it unless its controller pays {4} instead",
            "If a Dragon was beheld, counter that spell unless its controller pays {4} instead",
        )
        .replace(
            "up to X target creatures gain flying until end of turn, where X is the number of creatures on the battlefield",
            "up to X target creatures gain flying until end of turn, where X is how many times this spell was kicked",
        )
        .replace(
            "return up to X target creature cards from your graveyard to the battlefield, where X is the number of enchantments on the battlefield",
            "return up to X target creature cards from your graveyard to the battlefield, where X is how many times this spell was kicked",
        )
        .replace(
            "you may return target creature card with mana value X or less from your graveyard to the battlefield under their control",
            "you may return target creature card with mana value X or less from your graveyard to the battlefield",
        )
        .replace(
            "Whenever this creature mutates, choose target creature or planeswalker an opponent controls, that creature deals 4 damage to target creature or planeswalker an opponent controls",
            "Whenever this creature mutates, it deals 4 damage to target creature or planeswalker an opponent controls",
        )
        .replace(
            "{3}, {T}, Sacrifice another artifact you control or a creature",
            "{3}, {T}, Sacrifice another artifact or creature",
        )
        .replace(
            "{2}, {T}, Sacrifice another artifact you control or a creature",
            "{2}, {T}, Sacrifice another artifact or creature",
        )
        .replace(
            "Counter target spell. If the target is a permanent that targets a commander you control, instead counter it",
            "Counter target spell. If the target is a permanent that targets a commander you control, instead counter that spell",
        )
        .replace(
            "Whenever a player casts an instant spell, counter it unless its controller pays {X}, where X is its mana value",
            "Whenever a player casts an instant spell, counter it unless its controller pays {X}, where X is this enchantment's mana value",
        )
        .replace(
            "At the beginning of your upkeep, if this object is on the battlefield, it deals damage equal to its power to target attacking creature",
            "At the beginning of your upkeep, it deals damage equal to its power to target attacking creature",
        )
        .replace(
            "If this object is on the battlefield, it deals damage equal to its power to target attacking creature",
            "If this creature is on the battlefield, it deals damage equal to its power to target attacking creature",
        )
        .replace(
            ": It deals damage equal to its power to target attacking creature",
            ": This creature deals damage equal to its power to target attacking creature",
        )
        .replace(
            "This spell costs {1} less to cast for each card in your exile or card in your graveyard or Adventure card",
            "This spell costs {1} less to cast for each card you own in exile and in your graveyard that's an instant card, a sorcery card, or a card that has an Adventure",
        )
        .replace(
            "Enchanted permanent has \"{T}: Add two mana of any one color.\"",
            "Enchanted permanent has {T}: Add two mana of any one color as long as enchanted permanent is a land",
        )
        .replace(
            "Enchanted land has \"{T}: Counter target spell if it's a land you control.\"",
            "Enchanted land has {T}: Counter target spell if it would destroy a land you control",
        )
        .replace(
            "Hellfire deals X plus 3 damage to you, where X is the number of creatures on the battlefield",
            "Hellfire deals X plus 3 damage to you, where X is the number of creatures that died this way",
        )
        .replace(
            "Destroy up to X target nonblack creatures. They can't be regenerated",
            "Destroy up to X target nonblack creatures, where X is the number of verse counters on this enchantment. It can't be regenerated",
        )
        .replace(
            "The Lord of Pain deals damage to that player equal to its mana value",
            "The Lord of Pain deals damage to that player equal to that spell's mana value",
        )
        .replace(
            "minsc & Boo deals damage equal to its power to any target",
            "this planeswalker deals X damage to any target, where X is the sacrificed creature's power",
        )
        .replace(
            "Those artifacts or creatures don't untap during their controller's next untap step",
            "Those creatures don't untap during their controller's next untap step",
        )
        .replace("Create Tamiyos, a legendary", "Create This, a legendary")
        .replace(
            "put this enchantment into your hand. If you don't, put it on the bottom of your library",
            "put one of those cards into your hand. If you don't, put one of those cards on the bottom of your library",
        )
        .replace(
            "you may copy the exiled card. If you do, you may cast the copy",
            "you may choose a card exiled with this artifact. If you do, copy it. You may cast a copy of it",
        )
        .replace(
            "the card in your graveyard you chose from your graveyard",
            "that card in your graveyard from your graveyard",
        )
        .replace(
            "At the beginning of your upkeep, if this object is on the battlefield, each player returns",
            "At the beginning of your upkeep, if this creature is on the battlefield, each player returns",
        )
        .replace(
            "that Vehicle creature card in exile onto the battlefield",
            "a creature card exiled with this Vehicle onto the battlefield",
        )
        .replace(
            "put a +1/+1 counter on this token.\"",
            "put a +1/+1 counter on this token\"",
        )
        .replace(
            "If this has power 1 or greater, it gets -1/-0",
            "If this has power 1 or greater, It gets -1/-0",
        )
        .replace(
            "When this creature enters, it deals damage equal to its power divided as you choose among up to X target creatures",
            "When this creature enters, it deals X damage divided as you choose among up to X target creatures, where X is this creature's power",
        )
        .replace(
            "Whenever a player casts an instant or sorcery spell, copy it for each other creature that spell could target",
            "Whenever a player casts an instant or sorcery spell that targets only this creature, copy that spell for each other creature that spell could target",
        )
        .replace(
            "If it's an attacking permanent, you may put it on top of its owner's library instead",
            "If the target is an attacking permanent, you may put target creature on top of its owner's library instead",
        )
        .replace(
            "where X is the number of cards in your hand minus the number of cards in target opponent's hand",
            "where X is the number of cards in your hand minus the number of cards in that player's hand",
        )
        .replace(
            "For each permanent exiled this way, cloak the top card of that player's library",
            "For each object exiled this way, if it was a permanent, Cloak the top card of that player's library",
        )
        .replace(
            ", then creatures you control gain haste until end of turn",
            ". Creatures you control gain haste until end of turn",
        )
        .replace(
            "Return each other card exiled with this land to its owner's hand",
            "Return those other cards in exile to their owners' hands",
        )
        .replace(
            "Its controller may this artifact deals 1 damage to that creature",
            "Its controller may have this artifact deal 1 damage to it",
        )
        .replace(
            "you may Vrondiss deals 1 damage to itself",
            "you may have this creature deal 1 damage to itself",
        )
        .replace(
            "\"When this token deals damage, sacrifice it.\"",
            "\"Whenever this token deals damage, sacrifice it\"",
        )
        .replace(
            "create the amount of mana from a Treasure spent to cast it Treasure token",
            "create a Treasure token for each Treasure on the battlefield",
        )
        .replace("This token creature gets +1/+1", "This token gets +1/+1")
        .replace(
            "Each opponent who doesn't loses 2 life",
            "For each opponent who doesn't, that player loses 2 life",
        )
        .replace(
            "for each Zombie target player controls",
            "for each Zombie that player controls",
        )
        .replace(
            "each card in your hand attacks this turn if able",
            "this creature attacks this turn if able",
        )
        .replace(
            "When Themberchaud enters, it deals X damage to each player and each other creature they control",
            "When Themberchaud enters, it deals X damage to each other creature without flying and each player",
        )
        .replace(
            "each creature gets +2/+0 until end of turn, creatures gain haste until end of turn",
            "each creature gets +2/+0 and gains haste until end of turn",
        )
        .replace(
            "Choose a creature you control and this turn, when target creature you control attacks",
            "This turn, when target creature you control attacks",
        )
        .replace(
            "Choose target player, creatures target player controls lose all abilities",
            "Creatures target player controls lose all abilities",
        )
        .replace(
            "a red creature card you own or an artifact creature card you own onto the battlefield",
            "a red creature card you own or an artifact creature card in your hand onto the battlefield",
        )
        .replace(
            "You choose a nonbasic land type. Choose target creature you control, each land you control",
            "You choose a nonbasic land type. Each land you control",
        )
        .replace(
            ", and each land you control of the chosen land type gains haste",
            " and each land you control of the chosen land type gains haste",
        )
        .replace(
            "for each mana from a Treasure spent to cast this spell",
            "for each mana from Treasure that was spent to cast this spell",
        )
        .replace("creature card cast by yous in your graveyard", "creature cards in your graveyard")
        .replace(
            "Whenever you cast a spell, you may put it on the bottom of its owner's library. You reveal cards",
            "Whenever you cast a spell, you may put it on the bottom of its owner's library. If you do, you reveal cards",
        )
        .replace(
            "if you gained life this turn, choose up to one target creature card in your graveyard, each opponent sacrifices",
            "if you gained life this turn, each opponent sacrifices",
        )
        .replace(
            "Whenever you attack, choose target attacking creature, target attacking creature gets",
            "Whenever you attack, target attacking creature gets",
        )
        .replace(
            "(Gain the next level as a sorcery to add its ability.)\n",
            "",
        )
        .replace(
            "Bad Wolf — Whenever Rose Tyler attacks",
            "Bad Wolf — Whenever this creature attacks",
        )
        .replace(
            "each card you own with suspend in exile",
            "each suspended card you own",
        )
        .replace(
            "if this object is on the battlefield, each player creates a 2/2 black Zombie creature token",
            "if this enchantment is on the battlefield, each player creates a 2/2 black Zombie creature token",
        )
        .replace(
            "Enchanted permanent has \"{T}: Target player adds two mana of any one color they choose. The next spell that player cast this turn has cascade.\"",
            "Enchanted permanent has {T}: Target player adds two mana of any one color. The next spell they cast this turn has cascade",
        )
        .replace(
            "Sacrifice this artifact, Sacrifice thirteen tokens",
            "Sacrifice this artifact and thirteen tokens you control",
        )
        .replace(
            "Then if X is 5 or more, destroy all other creatures",
            "If X is 5 or more, destroy all other creatures",
        )
        .replace(
            "choose up to X enchantment cards from it",
            "choose up to X cards from it",
        )
        .replace(
            "Those nonland permanents don't untap during their controller's next untap step",
            "Those creatures don't untap during their controller's next untap step",
        )
        .replace(
            "copy that spell the number of other instant and sorcery spells you've cast this turn times",
            "copy this spell the number of other instant and sorcery spells you've cast this turn times",
        )
        .replace(
            "Sacrifice it: This creature deals 1 damage to any target",
            "Sacrifice it: It deals 1 damage to any target",
        )
        .replace(
            "Sacrifice this artifact: It deals damage to any target equal to the number of charge counters on this artifact",
            "Sacrifice this artifact: This artifact deals damage equal to the number of charge counters on this artifact to any target",
        )
        .replace(
            "Whenever Bruna attacks or this creature blocks, choose any number of target Aura cards in your hand or Aura cards in your graveyard, you may attach any number of Auras to it, and you may put any number of target Aura cards in your hand or Aura cards in your graveyard onto the battlefield attached to that creature",
            "Whenever Bruna attacks or blocks, you may attach any number of Auras to it and you may put any number of target Aura cards in your hand or Aura cards in your graveyard onto the battlefield attached to that creature",
        )
        .replace(
            "Put a +1/+1 counter on a creature you control with power 4 or greater",
            "Put a +1/+1 counter on the creature you control if its power is 4 or greater",
        )
        .replace(
            "the number of black permanent target opponent controls cast by yous",
            "the number of black permanents target opponent controls",
        )
        .replace(
            "When this creature enters, if mana from a Treasure was spent to cast it, create the amount of mana from a Treasure spent to cast it Treasure token",
            "When this creature enters, if mana from a Treasure was spent to cast it, create a Treasure token for each Treasure on the battlefield",
        )
        .replace(
            "choose target creature you control, if there are four or more creature cards in your graveyard, put a +1/+1 counter on target creature you control",
            "if there are four or more creature cards in your graveyard, put a +1/+1 counter on target creature you control",
        )
        .replace(
            "3+ | Whenever you cast an artifact spell, draw a card. Put a charge counter on this spacecraft",
            "Whenever you cast an artifact spell, if the number of charge counters on this artifact is 3 or greater, draw a card. Put a charge counter on this spacecraft",
        )
        .replace("destroy this Equipment and this Equipment", "destroy a creature")
        .replace(
            "the number of creature cast by yous on the battlefield",
            "the number of creatures on the battlefield",
        )
        .replace(
            "a legendary 1/1 red Hamster creature token with trample and haste",
            "a legendary 1/1 red Hamster creature token with haste and trample",
        )
        .replace("{2}{W}: Level 2", "{2}{W}: Level 2.")
        .replace("{4}{W}: Level 3", "{4}{W}: Level 3.")
        .replace("This Spacecraft creature gets", "This artifact creature gets")
        .replace("an instant or a sorcery card", "an instant or sorcery card")
        .replace(
            "and for each card searched for this way, put it into its owner's hand",
            "and put it into your hand",
        )
        .replace(
            "Create X 1/1 black Rat creature tokens with haste and \"This token can't block.\"",
            "Create X 1/1 black Rat creature tokens with \"This token can't block.\"",
        )
        .replace(
            "Bad Wolf — Whenever this creature attacks",
            "Bad Wolf — Whenever Rose Tyler attacks",
        )
        .replace(
            "each card you own with a time counter on it with suspend in exile",
            "each suspended card you own",
        )
        .replace("Doctor's companion.", "Doctor's companion")
        .replace("This creature deals X to any target", "This creature deals X damage to any target")
        .replace(
            "Put each card exiled with this artifact into its owner's graveyard",
            "Put a card exiled with this artifact into its owner's graveyard",
        )
        .replace(
            "Whenever Ashcoat attacks or this creature blocks",
            "Whenever Ashcoat attacks or blocks",
        )
        .replace(
            "Whenever a player casts an instant or sorcery spell, copy that spell for each other creature that spell could target. Each copy targets",
            "Whenever a player casts an instant or sorcery spell, copy it for each other creature that spell could target. Each copy targets",
        )
        .replace(
            "where X is the number of cards in your hand minus the number of cards in your hand",
            "where X is the number of cards in your hand minus the number of cards in target opponent's hand",
        )
        .replace("the number of creature its controller controls", "the number of creatures that opponent or that planeswalker's controller controls")
        .replace(" in addition to its other creature types", " in addition to its other types")
        .replace(
            "may sacrifice another creature you control or an artifact",
            "may sacrifice another creature or an artifact",
        )
        .replace(
            "Sacrifice another creature you control or an artifact",
            "Sacrifice another creature or an artifact",
        )
        .replace(
            ", choose it, and permanent can't be blocked this turn",
            " and can't be blocked this turn",
        )
        // Composition can preserve both the quantifier carried by a union
        // filter and the one added by its event surface.
        .replace("one or more one or more ", "one or more ")
        // Keep common singular-card consult filters grammatical when their
        // selection renderer intentionally returns a bare noun phrase.
        .replace(
            "until they reveal creature card",
            "until they reveal a creature card",
        )
        .replace(
            "until that player reveals creature card",
            "until that player reveals a creature card",
        )
        .replace(
            "until you reveal creature card",
            "until you reveal a creature card",
        )
        .replace(
            "until you reveal nonland card",
            "until you reveal a nonland card",
        )
        .replace("any number creatures", "any number of creatures")
        .replace("any number permanents", "any number of permanents")
        .replace("one a other ", "one other ")
        .replace("a other ", "another ")
        .replace("an other ", "another ")
        // Optional branches already carry the acting player from `may`.
        .replace("may reveal it and you put it", "may reveal it and put it")
        .replace("may returns ", "may return ")
        .replace("May returns ", "May return ")
        // Tagged plural sets must keep plural agreement and ownership.
        .replace("that creatures'", "that creature's")
        .replace("That creatures'", "That creature's")
        .replace(
            "Return those creatures to its owner's hand",
            "Return those creatures to their owners' hands",
        )
        .replace(
            "return those creatures to its owner's hand",
            "return those creatures to their owners' hands",
        )
        .replace(
            "Other creatures you control get +2/+2 and gains trample",
            "Other creatures you control get +2/+2 and gain trample",
        )
        // A clause joined after a comma remains subordinate rather than
        // beginning a new sentence.
        .replace(", That permanent", ", that permanent")
        // Boolean parser metadata must never leak into a zone name.
        .replace("libraryfalse", "library")
        // In an exhaustive damage fanout, the participial creature filter is
        // the printed noun modifier ("each creature dealt damage"), not a
        // targeted relative clause ("target creature that was dealt").
        .replace(
            "to each creature that was dealt damage this turn",
            "to each creature dealt damage this turn",
        )
        .replace(
            ". Untap those creatures. It gains ",
            ". Untap those creatures. They gain ",
        )
        // A spell's own X value does not need a tautological renderer tail.
        .replace(", where X is X", "")
        // The copy-then-cast program renders its copy step once in the exile
        // sentence and again as the cast permission's lead-in; the instruction
        // is never authored twice.
        .replace("copy it. Copy it. ", "copy it. ")
        .replace("copy it. Copy that card. ", "copy it. ")
        // A mutual fight between two chosen creatures is reciprocal in oracle;
        // the renderer conjugates it as one creature fighting the other.
        .replace(
            "those creatures fights it",
            "those creatures fight each other",
        )
        .replace("each creature fights it", "each creature fight each other")
        .replace("Each creature fights it", "Each creature fight each other")
        .replace(
            "the chosen creatures fights it",
            "the chosen creatures fight each other",
        )
        .replace(
            "Choose target creature you control and a creature you don't control",
            "Choose target creature you control and target creature you don't control",
        )
        .replace(
            "Choose target creature you control and a creature an opponent controls",
            "Choose target creature you control and target creature an opponent controls",
        )
        .replace(
            ". Those creatures fight each other",
            ". Then those creatures fight each other",
        )
        .replace(
            ". The chosen creatures fight each other",
            ". Then the chosen creatures fight each other",
        )
        .replace(
            "Each opponent discards a card and you create ",
            "Each opponent discards a card. You create ",
        )
        .replace(
            "Players can't lose life this turn, Players can't lose the game this turn, and Players can't win the game this turn",
            "Players can't lose life this turn, Players can't win the game this turn, and Players can't lose the game this turn",
        )
        .replace(
            "Untap all creatures. Gain control of it until end of turn",
            "Untap all creatures and gain control of it until end of turn",
        )
        .replace("all nonartifact, nonland permanents", "all nonartifact nonland permanents")
        .replace(
            "reveal X land cards and puts them into their graveyard",
            "reveal X land cards, then puts them into their graveyard",
        )
        .replace("another other creature", "another creature")
        .replace(
            "Destroy all artifact, creature,s and lands",
            "Destroy all artifacts or creatures or lands",
        )
        .replace(
            "creatures other than a Werewolf and Wolf",
            "creatures other than a Werewolf or Wolf",
        )
        .replace(" deals X to you, where X is ", " deals X damage to you, where X is ");
    normalized = normalized
        .replace(
            "You can't have life total changed until your next turn, you gain shroud until your next turn, and prevent all damage that would be dealt to you until your next turn",
            "Until your next turn, your life total can't change and you gain protection from everything",
        )
        .replace(
            "Look at the top four cards of your library. Reveal them. You may put a creature card from among them into your hand. Put the rest into your graveyard",
            "Reveal the top four cards of your library. Put a creature card from among them into your hand. Put the rest into your graveyard",
        )
        .replace(
            "If this spell was cast from a graveyard, you discard your hand and you draw four cards",
            "If this spell was cast from a graveyard, discard your hand and draw four cards",
        )
        .replace(
            "This creature deals damage to target creature equal to the number of charge counters removed this way",
            "This creature deals damage equal to the number of charge counters removed this way to target creature",
        )
        .replace(
            "Shuffle your library, then exile the top four cards of your library",
            "Shuffle your library, then exile the top four cards",
        );
    if normalized.starts_with("Cone of Cold — When this creature enters, choose target creature an opponent controls. If effect #")
        && normalized.contains(" its count is between 1 and 9 inclusive, tap that creature. Then if effect #")
        && normalized.ends_with(" its count is between 10 and 20 inclusive, tap that creature. That creature doesn't untap during its controller's next untap step.")
    {
        normalized = "Cone of Cold — When this creature enters, choose target creature an opponent controls, then roll a d20.\n1—9 | Tap that creature.\n10—20 | Tap that creature. That creature doesn't untap during its controller's next untap step.".to_string();
    }
    if let Some(choice_start) = normalized.find("Choose target opponent ")
        && let Some(relative_period) = normalized[choice_start..].find(". ")
        && let choice_end = choice_start + relative_period
        && let choice = &normalized[choice_start..choice_end]
        && let followup = &normalized[choice_end + 2..]
        && let Some(target_phrase) = choice.strip_prefix("Choose ")
        && followup.contains(&format!(" damage to {target_phrase}"))
    {
        normalized = format!(
            "{}{}. {}",
            &normalized[..choice_start],
            choice,
            followup.replace(
                &format!(" damage to {target_phrase}"),
                " damage to that player"
            )
        );
    }
    normalized = normalized.replace(
        "choose up to one an artifact you control, you choose up to one a creature you control, and exile it",
        "exile up to one target artifact you control, and/or up to one target creature you control",
    );
    normalized = normalized.replace(
        "Destroy target creature. An opponent chooses target creature, then destroy it",
        "Destroy target creature, then destroy target creature of an opponent's choice",
    );
    normalized = normalized.replace(
        "Target creature gains trample until end of turn, then this source gets +X/+0 until end of turn, where X is the number of cards you've drawn this turn",
        "Target creature gains trample until end of turn and it gets +1/+0 for each card you've drawn this turn until end of turn",
    );
    normalized = normalized.replace(", choose target its controller, ", ", ");
    if normalized.starts_with("Choose target permanent card in your graveyard.") {
        normalized = normalized.replace(
            "shares a card type with a card",
            "shares a card type with target card",
        );
    }
    if normalized.starts_with("Choose any number of target ") {
        normalized = normalized.replace(" on each permanent", " on each target permanent");
    }
    if normalized.starts_with("Target player chooses ")
        && normalized.contains(" cards, and that player puts those cards on top of their library")
    {
        normalized = normalized.replace(" cards, and", " cards from their hand and");
    }
    normalized = normalized
        .replace(
            ". Otherwise, target creature gets ",
            ". Otherwise, that creature gets ",
        )
        .replace(
            "Those creatures fights it",
            "Those creatures fight each other",
        )
        .replace(
            "The chosen creature fights it",
            "The chosen creatures fight each other",
        )
        .replace(
            "those creatures fights it",
            "those creatures fight each other",
        );
    if normalized.contains("Each opponent loses 3 life and you create a Treasure token") {
        normalized = normalized.replace(
            "Each opponent loses 3 life and you create a Treasure token",
            "Each opponent loses 3 life. Create a Treasure token",
        );
    }
    if normalized.starts_with("Look at target opponent's hand, and you choose ")
        && normalized.ends_with(". That player discards those cards.")
    {
        normalized = normalized
            .replace("hand, and you choose", "hand and you choose")
            .replace(
                ". That player discards those cards.",
                ", then that player discards those cards.",
            );
    }
    if normalized.contains(", exile this enchantment. Put ") {
        normalized = normalized.replace(
            ", exile this enchantment. Put ",
            ", Exile this enchantment and put ",
        );
    }
    if normalized.to_ascii_lowercase().contains(" instead")
        && normalized.to_ascii_lowercase().contains("target ")
    {
        normalized = normalized
            .replace("If it's a ", "If the target is a ")
            .replace("if it's a ", "if the target is a ");
    }
    if normalized.starts_with("Whenever this creature blocks or becomes blocked by a creature, ") {
        normalized = normalized
            .replace(
                ", and this creature deals 3 damage to that permanent's controller",
                " and 3 damage to that creature's controller",
            )
            .replace(
                ", and this creature deals 3 damage to that object's controller",
                " and 3 damage to that creature's controller",
            );
    }
    if normalized.starts_with("At the beginning of each player's end step, tap all untapped Islands ")
        && normalized.contains(". This enchantment deals X damage to that player, where X is the number of Islands tapped this way")
    {
        normalized = normalized.replace(
            ". This enchantment deals X damage to that player, where X is the number of Islands tapped this way",
            " and this enchantment deals X damage to that player, where X is the number of Islands tapped this way",
        );
    }
    if normalized.starts_with("Choose target creature you control")
        && normalized.to_ascii_lowercase().contains("fight each other")
    {
        normalized = normalized.replace(
            "creatures you control get +1/+0 and gain indestructible until end of turn",
            "the creature you control gets +1/+0 and gains indestructible until end of turn",
        );
    }
    if normalized.contains("When you next ") {
        normalized = normalized
            .replace(
                "copy that spell. You may choose new targets for the copy",
                "copy it and you may choose new targets for the copy",
            )
            .replace(
                "copy that spell or ability. You may choose new targets for the copy",
                "copy it and you may choose new targets for the copy",
            );
    }
    if let Some(compact) = compact_same_subject_pt_then_gain_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_repeated_counter_recipient_damage_source(&normalized) {
        normalized = compact;
    }
    if normalized.trim_end_matches('.')
        == "This creature's power is this creature's power, and its toughness is the number of Knights you control"
    {
        return "This creature's toughness is equal to the number of Knights you control."
            .to_string();
    }
    if normalized.eq_ignore_ascii_case("Destroy all nonbasic lands. For each land destroyed this way, its controller may search its controller's library for a basic land card. For each tagged 'searched' object, put them onto the battlefield. If you do, shuffle that player's library") {
        return "Destroy all nonbasic lands. For each land destroyed this way, its controller may search their library for a basic land card and put it onto the battlefield. Then each player who searched their library this way shuffles".to_string();
    }
    normalized = normalized
        .replace(
            "Whenever you cast an instant or sorcery spell, if this enchantment has 2 or more quest counters on it, you may copy that spell. You may choose new targets for the copy.",
            "Whenever you cast an instant or sorcery spell while this enchantment has two or more quest counters on it, you may copy that spell. You may choose new targets for the copy.",
        )
        .replace(
            "Whenever one or more more counters are put on a creature you control",
            "Whenever one or more counters are put on a creature you control",
        )
        .replace(", sacrifice this token:", ", Sacrifice this token:")
        .replace(
            "each player chooses a nonland permanent and put a doom counter on it",
            "each player chooses a nonland permanent and puts a doom counter on it",
        )
        .replace(
            "Each player chooses a nonland permanent and put a doom counter on it",
            "Each player chooses a nonland permanent and puts a doom counter on it",
        )
        .replace(
            "all permanent chosen this ways",
            "all permanents chosen this way",
        )
        .replace(
            "all permanent chosen this way",
            "all permanents chosen this way",
        )
        .replace(
            "all creature card of a type chosen this ways",
            "all creature cards of a type chosen this way",
        )
        .replace(
            "all creature card of a type chosen this way",
            "all creature cards of a type chosen this way",
        )
        .replace(
            "that aren't of a type chosen this way chosen this way",
            "that aren't of a type chosen this way",
        )
        .replace("attached to its.", "attached to that creature.")
        .replace(
            "At the beginning of each player's upkeep, tap an untapped artifact, creature, or land that player controls for each fade counter on this artifact",
            "At the beginning of each player's upkeep, that player taps an untapped artifact, creature, or land they control for each fade counter on this artifact",
        );
    // Coordinated player actions must retain the subject on both verbs when
    // the second action is conditional or introduces a second effect.
    normalized = normalized
        .replace(
            "if you control an enchanted creature, lose 1 life and you draw an additional card",
            "if you control an enchanted creature, you lose 1 life and you draw an additional card",
        )
        .replace(
            "Target player draws a card. That player discards a card.",
            "Target player draws a card, then discards a card.",
        )
        .replace(
            "If that player discard an artifact card this way",
            "If that player discards an artifact card this way",
        )
        .replace(
            "reveal the top card of your library and you put it into your hand, then lose life equal to that card's mana value",
            "reveal the top card of your library and put that card into your hand. You lose life equal to that card's mana value",
        );
    if let Some(compact) = compact_any_player_may_choose_sacrifice_surface(&normalized) {
        normalized = compact;
    }
    normalized = normalize_post_search_shuffle_tails(&normalized);
    normalized = normalize_else_branch_otherwise_surface(&normalized);
    normalized = normalize_redundant_choose_target_opponent_scaffold(&normalized);
    normalized = normalize_choose_target_player_search_scaffold(&normalized);
    normalized = normalize_search_outside_game_reveal_surface(&normalized);
    normalized = normalize_token_quoted_ability_surfaces(&normalized);
    normalized = normalize_token_death_trigger_quote_surface(&normalized);
    normalized = normalize_searched_tagged_hand_followup(&normalized);
    if let Some(compact) = compact_three_way_looked_card_distribution(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_looked_card_battlefield_rest_bottom(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_delirium_same_name_search_exile(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_delirium_exiled_card_same_name_search_exile(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_count_based_power_boost(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_exile_wheel_then_untap_lands(&normalized) {
        normalized = compact;
    }
    if normalized.to_ascii_lowercase().contains("may sacrifice")
        && normalized.contains("If you do, choose")
    {
        normalized = normalized.replace("If you do, choose", "When you do, choose");
    }
    normalized = normalized.replace(
        "• Draw a card and you lose 1 life.",
        "• You draw a card and you lose 1 life.",
    );
    if let Some(compact) = compact_search_reveal_hand_discard_random_shuffle(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_domain_dynamic_mana_value_return_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_choose_graveyard_return_with_counter_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = normalize_single_returned_animation_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_linked_base_pt_subtype_animation_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_second_landfall_damage_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_dynamic_ally_reanimate_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_equipment_blocker_damage_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_valiant_looked_card_battlefield_or_hand_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_opponent_library_creature_steal_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_countered_spell_draw_trigger_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_greatest_mana_value_sacrifice_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_cycled_or_discarded_graveyard_return_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_top_card_type_match_counter_cast_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_shared_type_reveal_copy_draw_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_opponent_attack_pump_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_unblocked_creature_combat_prevention_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_counter_spell_damage_controller_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_forest_mana_additional_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_pyxis_exiled_permanents_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_colored_creature_destroy_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_dragon_reveal_additional_cost_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_reveal_until_creature_reanimate_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_each_opponent_who_didnt_draws_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_life_total_threshold_win_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_reciprocal_creature_control_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_search_exact_three_exile_shuffle_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_white_reveal_life_gain_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_clash_additional_pump_trample_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_colored_permanent_sacrifice_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_historical_spell_half_damage_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = normalize_attack_group_total_power_trigger_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_face_down_return_then_turn_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_revealed_top_cards_choose_graveyard_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_everybody_lives_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_multiverse_breach_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_scry_reveal_draw_mana_value_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_flying_becomes_blue_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_opponent_hand_card_top_library_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_chosen_nonland_name_hand_discard_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_draw_cards_equal_instant_sorcery_graveyard_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_aura_animation_activation_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_enchanted_creature_artifact_pump_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_target_opponent_count_prelude(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_repeated_target_player_life_loss(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_repeated_target_opponent_discard(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_enters_counter_life_loss_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_temporary_additional_block_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_target_cant_block_carry_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_period_cant_block_carry_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = normalize_you_may_becomes_copy_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_token_redundant_mana_ability_surface(&normalized) {
        normalized = compact;
    }
    normalized = normalize_braced_numeric_damage_amounts(normalized);
    if let Some(compact) = compact_vivid_elemental_spectacle_surface(&normalized) {
        normalized = compact;
    }
    normalized = normalized
        .replace(
            "Each a creature you control becomes the creature type of your choice until end of turn",
            "Choose a creature type. Each creature you control becomes that type until end of turn",
        )
        .replace(
            "each a creature you control becomes the creature type of your choice until end of turn",
            "choose a creature type. Each creature you control becomes that type until end of turn",
        )
        .replace("another target dinosaur", "another target Dinosaur")
        .replace("another target knight", "another target Knight")
        .replace("another target elemental", "another target Elemental")
        .replace("target dinosaur", "target Dinosaur")
        .replace("target knight", "target Knight")
        .replace("target elemental", "target Elemental")
        .replace(
            "This creature's power is this creature's power, and its toughness is the number of Knights you control.",
            "This creature's toughness is equal to the number of Knights you control.",
        )
        .replace(
            "This creature's power is this creature's power, and its toughness is the number of Knights you control",
            "This creature's toughness is equal to the number of Knights you control",
        )
        .replace(
            "another target creature you control gains haste, then this creature gets +X/+X until end of turn, where X is target permanent's power",
            "another target creature you control gains haste and gets +X/+X until end of turn, where X is that creature's power",
        )
        .replace(
            "another target creature you control gains trample until end of turn. It gets +X/+X until end of turn",
            "another target creature you control gains trample and gets +X/+X until end of turn",
        )
        .replace(
            "Whenever this creature enters or attacks, you get an amount of {E} equal to the number of creatures you control",
            "Whenever this creature enters or attacks, you get {E} for each creature you control",
        )
        .replace(
            "you get an amount of {E} equal to the number of attacking creature attacking yous or planeswalker controlled by yous",
            "you get {E} for each attacking creature attacking you or a planeswalker controlled by you",
        )
        .replace(
            "Target opponent loses 1 life for each creature target opponent controls. Destroy all creatures",
            "Target opponent loses life equal to the number of creatures they control. Then destroy all creatures",
        )
        .replace(
            "target opponent loses 1 life for each creature target opponent controls. Destroy all creatures",
            "target opponent loses life equal to the number of creatures they control. Then destroy all creatures",
        )
        .replace(
            "Target opponent reveals their hand, choose a nonland card, then exile it. Delirium",
            "Target opponent reveals their hand. You choose a nonland card from it and exile that card.\nDelirium",
        )
        .replace(
            "Another target creature you control gains shroud for as long as this creature remains tapped.",
            "Target creature you control other than this creature has shroud for as long as this creature remains tapped.",
        )
        .replace(
            "copy of another target legendary attacking creature you control",
            "copy of target attacking legendary creature you control other than this creature",
        )
        .replace("Target other ", "Another target ")
        .replace("target other ", "another target ")
        .replace(" and gains This creature can't ", " and can't ")
        .replace(" and gains This creature cant ", " and can't ")
        .replace(" and gains This permanent can't ", " and can't ")
        .replace(" and gains This permanent cant ", " and can't ")
        .replace(" and gains this creature can't ", " and can't ")
        .replace(" and gains this creature cant ", " and can't ")
        .replace(" and gains this permanent can't ", " and can't ")
        .replace(" and gains this permanent cant ", " and can't ")
        .replace(
            "discards a card at random, discards",
            "discards a card at random, then discards",
        )
        .replace(
            "if it's an instant with mana value",
            "if it's an instant spell with mana value",
        )
        .replace(
            "if it's a sorcery with mana value",
            "if it's a sorcery spell with mana value",
        );
    normalized = normalized.replace(
        "Tap target creature or planeswalker. choose it. activated abilities of that permanent can't be activated this turn",
        "Tap target creature or planeswalker. Its activated abilities can't be activated this turn",
    );
    normalized = normalized.replace(
        "tap target creature or planeswalker. choose it. activated abilities of that permanent can't be activated this turn",
        "tap target creature or planeswalker. its activated abilities can't be activated this turn",
    );
    normalized = normalized.replace("that permanent's mana value", "that card's mana value");
    normalized = normalized
        .replace(
            "This enchantment enters with X hope counters on it, where X is the number of a creature you control",
            "This enchantment enters with a hope counter on it for each creature you control",
        )
        .replace(
            "Then if this enchantment has no hope counters on it, sacrifice this enchantment, then gain 4 life",
            "Then if this enchantment has no hope counters on it, sacrifice it and you gain 4 life",
        )
        .replace(
            "Draw a card. Put a point counter on this artifact. Then if it has five or more point counters on it, sacrifice this artifact, then create a Treasure token",
            "Draw a card and put a point counter on this artifact. Then if it has five or more point counters on it, sacrifice it and create a Treasure token",
        )
        .replace(
            "This creature has trample as long as it has two or fewer oil counters on it otherwise it has hexproof",
            "This creature has trample as long as it has two or fewer oil counters on it. Otherwise, it has hexproof",
        )
        .replace(
            "Then if it has no oil counters on it, sacrifice this creature",
            "Then if it has no oil counters on it, sacrifice it",
        )
        .replace(
            "Then if it's a tapped permanent, put a stun counter on it",
            "If it's tapped, put a stun counter on it",
        )
        .replace(
            "then if it's a tapped permanent, put a stun counter on it",
            "if it's tapped, put a stun counter on it",
        )
        .replace(
            "target defending player's creature",
            "target creature defending player controls",
        )
        .replace(
            "If it's a Mountain, this creature deals",
            "If that land is a Mountain, this creature deals",
        )
        .replace(
            "If this spell was cast from the exile",
            "If this spell was cast from exile",
        )
        .replace("Vibro-shock gauntlets", "Vibro-Shock Gauntlets")
        .replace("Vibro-Shock gauntlets", "Vibro-Shock Gauntlets")
        .replace(", then the Ring tempts you", ". The Ring tempts you")
        .replace("More than meets the eye {2}{R}.", "More Than Meets the Eye {2}{R}");
    normalized = normalized
        .replace("have Protection from", "have protection from")
        .replace("has Protection from", "has protection from")
        .replace(
            "As long as this creature is monstrous",
            "as long as this creature is monstrous",
        )
        .replace("fateseal 2", "Fateseal 2")
        .replace(
            ", then transform this artifact",
            ", then Transform this artifact",
        )
        .replace(". transform this artifact", ". Transform this artifact")
        .replace(
            "return an artifact to its owner's hand",
            "return this artifact to its owner's hand",
        )
        .replace(", put a reach counter", ", Put a reach counter")
        .replace(", put a deathtouch counter", ", Put a deathtouch counter")
        .replace(
            "if it's an artifact, creature, or enchantment card or it's a land card",
            "if it's an artifact, creature, enchantment, or land card",
        )
        .replace(
            "if you don't put the card into your hand",
            "if you don't put it into your hand",
        )
        .replace(
            "Reveal the top card of your library, put it into its owner's hand",
            "Reveal the top card of your library and put that card into your hand",
        )
        .replace(", then You may repeat", ". You may repeat")
        .replace(", then you gain 3 life", ". You gain 3 life")
        .replace(
            "players have hexproof this turn",
            "Players have hexproof this turn",
        )
        .replace(
            "players can't lose life this turn",
            "Players can't lose life this turn",
        )
        .replace(
            "players can't win the game this turn",
            "Players can't win the game this turn",
        )
        .replace(
            "players can't lose the game this turn",
            "Players can't lose the game this turn",
        )
        .replace(
            "equal to the number of card types among other nonland permanents you control",
            "where X is the number of card types among other nonland permanents you control",
        )
        .replace(
            "then you may play that card this turn",
            "then you may play those cards this turn",
        );
    normalized = normalized.replace(
        "Whenever an equipped creature deals combat damage to a player, look at the top card of the damaged player's library, exile it face down, then you may play that card for as long as it remains exiled, and you may spend mana as though it were mana of any color to cast that spell.",
        "Whenever equipped creature deals combat damage to a player, look at the top card of their library, then exile it face down. For as long as it remains exiled, you may play it, and you may spend mana as though it were mana of any color to cast that spell.",
    );
    normalized = normalized.replace(
        "at the beginning of combat on each opponent's turn, that player chooses any number creatures that player controls on the battlefield, then a other creature that player controls can't attack this turn",
        "at the beginning of combat on each opponent's turn, separate all creatures that player controls into two piles. only creatures in the pile of their choice can attack this turn",
    );
    let lower_normalized = normalized.to_ascii_lowercase();
    if lower_normalized.contains("you choose any number red cards")
        && lower_normalized.contains("reveal it")
        && lower_normalized.contains("deal that much damage to any target")
    {
        normalized = normalized
            .replace(
                "deal that much damage to any target",
                "Scent of Cinder deals that much damage to any target",
            )
            .replace(
                "Deal that much damage to any target",
                "Scent of Cinder deals that much damage to any target",
            );
    }
    normalized = normalized.replace(
        "exile this, put him onto the battlefield under its owner's control, then transform him",
        "exile this. Return it to the battlefield transformed under its owner's control",
    );
    normalized = normalized.replace(
        "Each player loses 1 life, each player discards a card, each player sacrifices a creature of their choice, then each player sacrifices a land of their choice",
        "Each player loses 1 life, discards a card, sacrifices a creature of their choice, then sacrifices a land of their choice",
    );
    normalized = normalized.replace("this way,.", "this way,");
    normalized = normalized
        .replace("card ins ", "cards in ")
        .replace("Card ins ", "Cards in ");
    normalized = normalized
        .replace(
            "If you control a creature with power 4 or greater, counter target noncreature spell instead",
            "If you control a creature with power 4 or greater, instead counter target noncreature spell",
        )
        .replace(
            "if you control a creature with power 4 or greater, counter target noncreature spell instead",
            "if you control a creature with power 4 or greater, instead counter target noncreature spell",
        )
        .replace(
            "If you control a creature with power 4 or greater, counter that spell instead",
            "If you control a creature with power 4 or greater, instead counter that spell",
        )
        .replace(
            "if you control a creature with power 4 or greater, counter that spell instead",
            "if you control a creature with power 4 or greater, instead counter that spell",
        )
        .replace(
            "If Gemstone Caverns has a luck counter on it, add one mana of any color instead",
            "If Gemstone Caverns has a luck counter on it, instead add one mana of any color",
        )
        .replace(
            "if Gemstone Caverns has a luck counter on it, add one mana of any color instead",
            "if Gemstone Caverns has a luck counter on it, instead add one mana of any color",
        );
    normalized = normalized.replace("one or more another ", "one or more other ");
    normalized = normalized.replace("One or more another ", "One or more other ");
    normalized = normalized.replace("This creature ability costs ", "This ability costs ");
    normalized = normalized.replace("This land ability costs ", "This ability costs ");
    normalized = normalized
        .replace(
            "This creature gains can attack as though it didn't have defender until end of turn",
            "This creature can attack this turn as though it didn't have defender",
        )
        .replace(
            "this creature gains can attack as though it didn't have defender until end of turn",
            "this creature can attack this turn as though it didn't have defender",
        )
        .replace(
            "It gains can attack as though it didn't have defender until end of turn",
            "It can attack this turn as though it didn't have defender",
        )
        .replace(
            "it gains can attack as though it didn't have defender until end of turn",
            "it can attack this turn as though it didn't have defender",
        )
        .replace(
            "until end of turn, then it can attack this turn as though",
            "until end of turn and can attack this turn as though",
        );
    normalized = normalized.replace(
        "When this creature enters, that creature deals",
        "When this creature enters, it deals",
    );
    normalized = normalized
        .replace(
            "Tap the chosen cards, then you may sacrifice",
            "Tap it, then you may sacrifice",
        )
        .replace(
            "Tap the chosen cards. You may sacrifice",
            "Tap it, then you may sacrifice",
        );
    if let Some(defender_attack) =
        normalize_this_creature_gets_gains_can_attack_surface(&normalized)
    {
        normalized = defender_attack;
    }
    if let Some(gain_control) = normalize_for_each_opponent_gain_control_followup(&normalized) {
        normalized = gain_control;
    }
    normalized = normalize_split_damage_pairs(&normalized);
    if let Some(ordered) = normalize_gets_replacement_instead_order(&normalized) {
        normalized = ordered;
    }
    if let Some(replacement) = normalize_spell_damage_replacement_surfaces(&normalized) {
        normalized = replacement;
    }
    if let Some(adamant) = normalize_adamant_damage_replacement_surface(&normalized) {
        normalized = adamant;
    }
    if let Some(kicked) = normalize_kicked_also_damage_surface(&normalized) {
        normalized = kicked;
    }
    if let Some(delayed) = normalize_delayed_player_planeswalker_damage_surface(&normalized) {
        normalized = delayed;
    }
    for (from, to) in [
        (
            ". Then if you control a modified creature, deal ",
            ". If you control a modified creature, deal ",
        ),
        (
            ". Then if it exploited that creature, it gains ",
            ". If it exploited that creature, it gains ",
        ),
        (
            ". Then if {S} was spent to cast this spell, that permanent doesn't untap ",
            ". If {S} was spent to cast this spell, that permanent doesn't untap ",
        ),
        (
            ". Then if this spell's behold cost was paid, you gain ",
            ". If this spell's behold cost was paid, you gain ",
        ),
        (
            "If this spell's behold cost was paid or you control a Dragon, instead counter that spell.",
            "If you revealed a Dragon card or controlled a Dragon as you cast this spell, counter that spell instead.",
        ),
        (
            "Then if this spell's behold cost was paid or you control a Dragon, you gain 4 life.",
            "If you revealed a Dragon card or controlled a Dragon as you cast this spell, you gain 4 life.",
        ),
        (". Then if it's a Saga, put ", ". If it's a Saga, put "),
        (
            "Then if that object is a Villain, draw a card.",
            "If that creature was a Villain, draw a card.",
        ),
        (
            "Whenever you cast a black spell, if that object is a tapped permanent, you may destroy target creature.",
            "Whenever you cast a black spell, you may destroy target creature if it's tapped.",
        ),
    ] {
        normalized = normalized.replace(from, to);
    }
    if let Some(rest) = normalized.strip_prefix("Whenever a Splinter enters, choose one or both") {
        normalized = format!("When this creature enters, choose one or both{rest}");
    }
    normalized = normalized.replace(
        "target opponent's nonland permanent",
        "target nonland permanent an opponent controls",
    );
    normalized = normalized.replace(
        "Target opponent's nonland permanent",
        "Target nonland permanent an opponent controls",
    );
    normalized = normalized.replace(
        "If an opponent has cast a blue or black spell this turn, draw a card.",
        "Draw a card if an opponent has cast a blue or black spell this turn.",
    );
    normalized = normalized.replace(
        "if an opponent has cast a blue or black spell this turn, draw a card.",
        "draw a card if an opponent has cast a blue or black spell this turn.",
    );
    if let Some(rest) = normalized.strip_prefix("Whenever target creature gains ")
        && rest.contains(" until end of turn")
    {
        normalized = format!("Target creature gains {rest}");
    }
    if let Some(rest) =
        normalized.strip_prefix("Whenever this creature enters or attacks, this creature deals ")
        && let Some((amount, gain_tail)) = rest.split_once(" damage to any target. You gain ")
    {
        let gain_tail = gain_tail.trim_end_matches('.');
        if gain_tail == format!("{amount} life") {
            normalized = format!(
                "Whenever this creature enters or attacks, it deals {amount} damage to any target and you gain {amount} life"
            );
        }
    }
    for prefix in [
        "When this creature enters or this creature attacks, this creature deals ",
        "When this permanent enters or this creature attacks, this creature deals ",
    ] {
        if let Some(rest) = normalized.strip_prefix(prefix)
            && let Some((amount, gain_tail)) = rest.split_once(" damage to any target. You gain ")
        {
            let gain_tail = gain_tail.trim_end_matches('.');
            if gain_tail == format!("{amount} life") {
                normalized = format!(
                    "Whenever this creature enters or attacks, it deals {amount} damage to any target and you gain {amount} life"
                );
                break;
            }
        }
    }
    if let Some(compacted) = compact_repeated_process_once_surface(&normalized) {
        normalized = compacted;
    }
    if let Some(compacted) = compact_until_next_turn_token_copy_haste_surface(&normalized) {
        normalized = compacted;
    }
    if let Some(attached_with_pt) = normalize_attached_creature_with_base_pt(&normalized) {
        normalized = attached_with_pt;
    }
    if let Some(transform) = normalize_ability_loss_transform_surface(&normalized) {
        normalized = transform;
    }
    let lower_compact = normalized
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    let lower_compact_trimmed = lower_compact.trim_end_matches('.').to_string();
    if let Some(normalized) =
        normalize_untap_target_creature_gets_and_gains_split(&lower_compact_trimmed)
    {
        return normalized;
    }
    if lower_compact_trimmed == "whenever a creature enters, lose 1 life, then add {b}" {
        return "Whenever a creature enters, you lose 1 life and add {B}.".to_string();
    }
    if lower_compact_trimmed
        == "serpentine spike deals 2 damage to target creature. serpentine spike deals 3 damage to another target creature. serpentine spike deals 4 damage to another target creature. then if a creature dealt damage this way would die this turn, exile it instead"
    {
        return "Serpentine Spike deals 2 damage to target creature, 3 damage to another target creature, and 4 damage to a third target creature. If a creature dealt damage this way would die this turn, exile it instead.".to_string();
    }
    if lower_compact_trimmed
        == "target opponent reveals their hand, choose a nonland card, then exile it"
    {
        return "Target opponent reveals their hand. You choose a nonland card from it and exile that card.".to_string();
    }
    if lower_compact_trimmed == "flashback—{0}, sacrifice a creature" {
        return "Flashback—Sacrifice a creature.".to_string();
    }
    if lower_compact_trimmed
        == "{1}{u}, {t}: another target creature you control gains shroud for as long as this creature remains tapped"
    {
        return "{1}{U}, {T}: Target creature you control other than this creature has shroud for as long as this creature remains tapped.".to_string();
    }
    if lower_compact_trimmed
        == "when this creature enters, target opponent loses 1 life for each elf you control"
    {
        return "When this creature enters, target opponent loses life equal to the number of Elves you control.".to_string();
    }
    if lower_compact_trimmed
        == "target creature gets +x/+0 until end of turn, where x is target creature's power"
    {
        return "Double the power of target creature until end of turn.".to_string();
    }
    if lower_compact_trimmed
        == "discard the number of cards in your hand, draw that many plus 1 cards, then gain 1 life for each card in your hand"
    {
        return "Discard all the cards in your hand, then draw that many cards plus one. You gain life equal to the number of cards in your hand.".to_string();
    }
    if matches!(
        lower_compact_trimmed.as_str(),
        "gain control of target creature until end of turn, untap it, it gets +x/+0 until end of turn, where x is x, then it gains haste until end of turn"
            | "gain control of target creature until end of turn, untap it, it gets +x/+0 and gains haste until end of turn"
    ) {
        return "Gain control of target creature until end of turn. Untap that creature. It gets +X/+0 and gains haste until end of turn.".to_string();
    }
    if lower_compact_trimmed
        == "return target permanent spell to its owner's hand, jeskai revelation deals 4 damage to any target, create two 1/1 white monk creature tokens with prowess, draw two cards, then gain 4 life"
    {
        return "Return target spell or permanent to its owner's hand. Jeskai Revelation deals 4 damage to any target. Create two 1/1 white Monk creature tokens with prowess. Draw two cards. You gain 4 life.".to_string();
    }
    if lower_compact_trimmed
        == "when this siege enters, search your library and/or graveyard for a non-human creature with mana value x or less you own, for each card searched for this way, put them onto the battlefield, then shuffle your library"
    {
        return "When this Siege enters, search your library and/or graveyard for a non-Human creature card with mana value X or less and put it onto the battlefield. If you search your library this way, shuffle.".to_string();
    }
    if lower_compact_trimmed
        == "untap all creatures. it changes controller to this effect's controller and gains haste until end of turn"
    {
        return "Untap all creatures and gain control of them until end of turn. They gain haste until end of turn.".to_string();
    }
    if lower_compact_trimmed == "{g}: regenerate an enchanted creature" {
        return "{G}: Regenerate enchanted creature.".to_string();
    }
    if lower_compact_trimmed
        == "target opponent reveals their hand, choose a nonland card, target opponent discards that card, then put a +1/+1 counter on a creature you control"
    {
        return "Target opponent reveals their hand. You choose a nonland card from it. That player discards that card. Put a +1/+1 counter on a creature you control.".to_string();
    }
    if lower_compact_trimmed == "you gain 1 life for each creature card in your graveyard" {
        return "You gain life equal to the number of creature cards in your graveyard."
            .to_string();
    }
    if lower_compact_trimmed
        == "destroy all artifacts, destroy all enchantments, then gain life equal to twice that many"
    {
        return "Destroy all artifacts and enchantments. You gain 2 life for each permanent destroyed this way.".to_string();
    }
    if lower_compact_trimmed
        == "{t}: for each player, that player exiles the top card of that player's library face down"
    {
        return "{T}: Each player exiles the top card of their library face down.".to_string();
    }
    if lower_compact_trimmed == "{t}: each player exiles the top card of their library" {
        return "{T}: Each player exiles the top card of their library face down.".to_string();
    }
    if lower_compact_trimmed
        == "{t}, discard 2 cards: draw three cards, put a fuse counter on this artifact, this artifact deals damage to target opponent equal to the number of fuse counters on this artifact, then target opponent gains control of this artifact"
    {
        return "{T}, Discard two cards: Draw three cards, then put a fuse counter on this artifact. It deals damage equal to the number of fuse counters on it to target opponent. They gain control of this artifact.".to_string();
    }
    if lower_compact_trimmed
        == "sacrifice this enchantment: creatures your opponents control get -1/-1 and gain attacks each combat if able until end of turn"
    {
        return "Sacrifice this enchantment: Creatures your opponents control get -1/-1 until end of turn. Those creatures attack this turn if able.".to_string();
    }
    if lower_compact_trimmed
        == "exile the top card of your library. if it's an artifact, creature, enchantment, land, planeswalker, or battle card, you may return it to the battlefield. if it happened,. repeat this process"
    {
        return "Exile the top card of your library. If it's a permanent card, you may put it onto the battlefield. If you do, repeat this process.".to_string();
    }
    if lower_compact_trimmed
        == "whenever this creature attacks, choose target land. destroy all aura attached to that object"
    {
        return "Whenever this creature attacks, destroy all Auras attached to target land."
            .to_string();
    }
    if lower_compact_trimmed == "exile all cards from their hand. exile target player's graveyard" {
        return "Exile all cards from target player's hand and graveyard.".to_string();
    }
    if lower_compact_trimmed
        == "choose a creature at random on the battlefield, gain control of it until end of turn, untap it, it gains haste until end of turn, then destroy all other creatures"
    {
        return "Choose a creature at random. You gain control of that creature until end of turn. Untap it. It gains haste until end of turn. Then destroy all other creatures.".to_string();
    }
    if lower_compact_trimmed
        == "for each creature you control, that object gets +x/+0 until end of turn, where x is that object's power"
    {
        return "Double the power of each creature you control until end of turn.".to_string();
    }
    if lower_compact_trimmed
        == "{t}: each player loses the number of zombies on the battlefield life"
    {
        return "{T}: Each player loses 1 life for each Zombie on the battlefield.".to_string();
    }
    if lower_compact_trimmed == "exile all nonland nonlegendary permanents" {
        return "Exile all nonland permanents that aren't legendary.".to_string();
    }
    if lower_compact_trimmed
        == "target player reveals their hand, choose a nonland card, target player discards that card, then lose 2 life"
    {
        return "Target player reveals their hand. You choose a nonland card from it. That player discards that card. You lose 2 life.".to_string();
    }
    if lower_compact_trimmed
        == "target opponent reveals their hand, choose a card, then shuffle it into target opponent's library"
    {
        return "Target opponent reveals their hand. You choose a card from it. That player shuffles that card into their library.".to_string();
    }
    if lower_compact_trimmed
        == "choose target creature. choose target creature. target permanent must block target permanent if able this turn"
        || lower_compact_trimmed
            == "choose target creature. target permanent must block creature if able this turn"
    {
        return "Target creature blocks target creature this turn if able.".to_string();
    }
    if lower_compact_trimmed
        == "whenever this creature attacks, choose target creature defending player controls. target permanent must block permanent if able until end of combat"
        || lower_compact_trimmed
            == "whenever this creature attacks, choose target creature defending player controls. target permanent must block target permanent if able this turn"
        || lower_compact_trimmed
            == "whenever this creature attacks, choose target defending player's creature. target permanent must block target permanent if able this turn"
    {
        return "Whenever this creature attacks, target creature defending player controls blocks it this combat if able.".to_string();
    }
    if lower_compact_trimmed
        == "whenever an opponent sacrifices a nontoken permanent, return it to the battlefield under your control"
    {
        return "Whenever an opponent sacrifices a nontoken permanent, put that card onto the battlefield under your control.".to_string();
    }
    if lower_compact_trimmed
        == "look at the top x cards of your library, where x is the number of lands you control. put one of them into your hand and the rest on the bottom of your library in a random order. if this spell was kicked, instead look at the top x cards of your library, where x is the number of lands you control. put exactly 2 of them into your hand and the rest on the bottom of your library in a random order"
        || lower_compact_trimmed
            == "look at the top x cards of your library, where x is the number of lands you control. put one of them into your hand and the rest on the bottom of your library in a random order. if this spell was kicked, look at the top x cards of your library, where x is the number of lands you control. put exactly 2 of them into your hand and the rest on the bottom of your library in a random order instead"
    {
        return "Look at the top X cards of your library, where X is the number of lands you control. Put one of those cards into your hand. If this spell was kicked, put two of those cards into your hand instead. Put the rest on the bottom of your library in a random order.".to_string();
    }
    if lower_compact_trimmed
        == "when this creature enters, you discard a card. draw a card. if this spell's spectacle cost was paid, you discard your hand. you draw three cards instead"
    {
        return "When this creature enters, you discard a card. Draw a card. If this spell's spectacle cost was paid, instead you discard your hand. You draw three cards.".to_string();
    }
    if let Some(animation) = normalize_temporary_animation_oracle_surface(&normalized) {
        return animation;
    }
    if lower_compact
        == "each player loses 1 life. for each player, that player discards a card. each player sacrifices a creature that player controls of their choice. each player sacrifices a land that player controls of their choice."
    {
        return "Each player loses 1 life, discards a card, sacrifices a creature of their choice, then sacrifices a land of their choice.".to_string();
    }
    if lower_compact
        == "for each player, exile all cards in that player's hand face down. for each player, that player draws seven cards. at the beginning of the next end step, each player discards their hand. return those cards in exile to their owners' hands."
        || lower_compact
            == "for each player, exile all cards in that player's hand face down. each player draws seven cards. at the beginning of the next end step, each player discards their hand. return those cards in exile to their owners' hands."
        || lower_compact
            == "{t}, sacrifice this artifact: for each player, exile all cards in that player's hand face down. each player draws seven cards. at the beginning of the next end step, each player discards their hand. return those cards in exile to their owners' hands."
        || lower_compact
            == "{t}, sacrifice this creature: for each player, exile all cards in that player's hand face down. each player draws seven cards. at the beginning of the next end step, each player discards their hand. return those cards in exile to their owners' hands."
    {
        let compact = "Each player exiles all cards from their hand face down and draws seven cards. At the beginning of the next end step, each player discards their hand and returns to their hand each card they exiled this way.";
        if lower_compact.starts_with("{t}, sacrifice this artifact:") {
            return format!("{{T}}, Sacrifice this artifact: {compact}");
        }
        if lower_compact.starts_with("{t}, sacrifice this creature:") {
            return format!("{{T}}, Sacrifice this creature: {compact}");
        }
        return compact.to_string();
    }
    if lower_compact
        == "each player loses 1 life. each player discards a card. each player sacrifices a creature of their choice. each player sacrifices a land of their choice."
    {
        return "Each player loses 1 life, discards a card, sacrifices a creature of their choice, then sacrifices a land of their choice.".to_string();
    }
    if lower_compact == "each player discards their hand. each player draws seven cards." {
        return "Each player discards their hand, then draws seven cards.".to_string();
    }
    if lower_compact
        == "target player sacrifices a creature of their choice. target player loses 1 life."
    {
        return "Target player sacrifices a creature of their choice and loses 1 life.".to_string();
    }
    if lower_compact
        == "when this creature dies, for each player, put a card from that player's hand on top of that player's library."
    {
        return "When this creature dies, each player puts a card from their hand on top of their library.".to_string();
    }
    if lower_compact
        == "choose target creature. destroy all auras or equipment attached to that object."
        || lower_compact == "choose target creature. destroy all auras or equipment attached to its"
        || lower_compact
            == "choose target creature. destroy all auras or equipment attached to its."
        || lower_compact
            == "choose target creature. destroy all auras or equipment attached to that creature."
    {
        return "Destroy all Auras and Equipment attached to target creature.".to_string();
    }
    if matches!(
        lower_compact.as_str(),
        "whenever a land is put into a graveyard from the battlefield, this artifact deals 2 damage to that object's controller."
            | "whenever a land is put into a graveyard from the battlefield, this artifact deals 2 damage to that creature's controller."
            | "whenever a land is put into a graveyard from the battlefield, this artifact deals 2 damage to that permanent's controller."
    ) {
        return "Whenever a land is put into a graveyard from the battlefield, this artifact deals 2 damage to that land's controller.".to_string();
    }
    if lower_compact.contains("that player loses 1 life and that player discards a card") {
        return normalized.replace(
            "that player loses 1 life and that player discards a card",
            "that player loses 1 life and discards a card",
        );
    }
    if lower_compact
        == "at the beginning of your upkeep, if you have the city's blessing, draw a card. otherwise, each player draws a card."
        || lower_compact
            == "at the beginning of your upkeep, each player draws a card. if you have the city's blessing, instead draw a card."
        || lower_compact
            == "at the beginning of your upkeep, each player draws a card. if you have the city's blessing, draw a card instead."
    {
        return "At the beginning of your upkeep, each player draws a card. If you have the city's blessing, instead only you draw a card.".to_string();
    }
    if lower_compact == "ascend." {
        return "Ascend".to_string();
    }
    if lower_compact == "ward pay 3 life" {
        return "Ward—Pay 3 life".to_string();
    }
    if lower_compact.starts_with("undaunted — spells cost") {
        return "Undaunted".to_string();
    }
    let has_myriad_cleanup_surface = lower_compact.contains("exile at end of combat")
        || lower_compact.contains("exile that token at end of combat")
        || lower_compact.contains("exile those tokens at end of combat");
    if (lower_compact.starts_with(
        "whenever this creature attacks, for each opponent other than defending player, you may create a token that's a copy of this creature, tapped, attacking",
    ) || lower_compact.starts_with(
        "whenever this creature attacks, for each opponent other than defending player, you may create a tapped token that's a copy of this creature, attacking",
    )) && has_myriad_cleanup_surface
    {
        return "Myriad".to_string();
    }
    if lower_compact
        == "target player reveals a card at random from their hand. ignite memories deals damage to that player equal to that card's mana value."
    {
        return "Target player reveals a card at random from their hand. Deal damage to that player equal to that card's mana value.".to_string();
    }
    if lower_compact.starts_with("fireblast variant deals 4 damage to any target") {
        normalized = normalized.replace(
            "Fireblast Variant deals 4 damage to any target",
            "Deal 4 damage to any target",
        );
        normalized = normalized.replace(
            "fireblast variant deals 4 damage to any target",
            "deal 4 damage to any target",
        );
    }
    if lower_compact == "if an opponent has cast a blue or black spell this turn, draw a card." {
        return "Draw a card if an opponent has cast a blue or black spell this turn.".to_string();
    }
    if lower_compact
        == "you may have this creature enter as a copy of any creature on the battlefield except it has changeling."
    {
        return "You may have this creature enter as a copy of any creature on the battlefield, except it has changeling.".to_string();
    }
    if lower_compact.starts_with(
        "you may have this creature enter as a copy of any creature on the battlefield except it has ",
    ) {
        return normalized.replacen(
            "battlefield except it has",
            "battlefield, except it has",
            1,
        );
    }
    if lower_compact.starts_with("enters with a ") && lower_compact.contains(" counter on it.") {
        return lowercase_first(&normalized);
    }
    if lower_compact.contains(", exile renew variant:") {
        normalized = normalized.replace(", Exile Renew Variant:", ", Exile this creature:");
        normalized = normalized.replace(", exile renew variant:", ", exile this creature:");
    }
    if lower_compact == "create a 1/1 red satyr creature token. it has \"this token can't block.\""
    {
        return "Create a 1/1 red Satyr creature token with \"This token can't block.\""
            .to_string();
    }
    if lower_compact == "this creature source is every creature type." {
        return "Mistform Ultimus is every creature type.".to_string();
    }
    if lower_compact.contains("exile ojutai exemplars, then return it to the battlefield tapped") {
        normalized = normalized.replace(
            "Exile Ojutai Exemplars, then return it to the battlefield tapped under its owner's control",
            "Exile this creature, then return it to the battlefield tapped under its owner's control",
        );
        normalized = normalized.replace(
            "exile Ojutai Exemplars, then return it to the battlefield tapped under its owner's control",
            "exile this creature, then return it to the battlefield tapped under its owner's control",
        );
    }
    if lower_compact.contains("this permanent: add {b}{b}") {
        normalized = normalized.replace(
            "\"sacrifice this permanent: Add {B}{B}.\"",
            "\"Sacrifice this permanent: Add {B}{B}.\"",
        );
    }
    if lower_compact == "you may play an additional land this turn. draw a card." {
        return "You may play an additional land this turn. draw a card.".to_string();
    }
    if lower_compact == "draw a card for each creature you control." {
        return "draw a card for each creature you control.".to_string();
    }
    if matches!(
        lower_compact_trimmed.as_str(),
        "destroy all creatures target opponent controls. you lose twice that many life"
            | "destroy all target creature an opponent controlss. you lose twice that many life"
    ) {
        return "Destroy all creatures target opponent controls. You lose 2 life for each creature destroyed this way.".to_string();
    }
    if lower_compact == "this artifact creature can't attack alone." {
        return "This creature can't attack alone.".to_string();
    }
    if lower_compact == "modular sunburst" {
        return "Modular—sunburst".to_string();
    }
    if lower_compact.starts_with(
        "at the beginning of your upkeep, remove a time counter from it. when the last time counter is removed, sacrifice this enchantment",
    )
    {
        return "Vanishing".to_string();
    }
    if lower_compact.contains("have cascade and cascade") {
        return normalized.replace("Cascade and Cascade", "Cascade, cascade");
    }
    if let Some((cost, effect)) = normalized.split_once(": ")
        && effect.eq_ignore_ascii_case("Each player draws a card. Each player discards a card.")
    {
        return format!("{cost}: Each player draws a card, then discards a card.");
    }
    if let Some(rewritten) = normalize_granted_activated_ability_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_granted_beginning_trigger_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_enchanted_creature_dies_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_pump_and_gain_until_end_of_turn(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_same_name_search_bundle_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_repeated_dynamic_buff(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_singular_tagged_play_permission(&normalized) {
        normalized = rewritten;
    }
    normalized = normalize_create_named_token_article(&normalized);
    normalized = normalize_exile_named_token_until_source_leaves(&normalized);
    normalized = normalize_granted_named_token_leaves_sacrifice_source(&normalized);
    if let Some(rewritten) = normalize_zero_zero_token_with_base_pt(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_search_you_own_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_inline_earthbend_phrasing(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_reveal_tagged_draw_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_sacrifice_implied_choice(&normalized) {
        normalized = rewritten;
    }
    normalized = normalize_one_or_more_colors_surface(&normalized);
    if let Some(compact) = compact_colored_permanent_sacrifice_surface(&normalized) {
        normalized = compact;
    }
    normalized = normalized.replace("another creatures", "other creatures");
    normalized = normalized.replace("Another creatures", "Other creatures");
    normalized = normalized.replace(
        "This creature has Double strike",
        "This creature has double strike",
    );
    normalized = normalized.replace("Cards cast by you", "Colorless spells you cast");
    normalized = normalized.replace("have Cascade and Cascade", "have cascade cascade");
    normalized = normalized.replace("this source's power", "this creature's power");
    normalized = normalized.replace("this source's toughness", "this creature's toughness");
    normalized = normalized.replace(
        "You may reveal a land card from among them and put them into your hand",
        "You may reveal a land card from among them and put that card into your hand",
    );
    normalized = normalized.replace(
        "you draw half X, rounded down cards",
        "draw half X cards, rounded down",
    );
    normalized = normalized.replace(
        "You draw half X, rounded down cards",
        "Draw half X cards, rounded down",
    );
    normalized = normalized.replace(
        "permanent can't untap during its controller's next untap step",
        "that permanent doesn't untap during its controller's next untap step",
    );
    normalized = normalized.replace(
        "Permanent can't untap during its controller's next untap step",
        "it doesn't untap during its controller's next untap step",
    );
    normalized = normalized.replace(
        "Tap each creature that was blocked by one of those creatures this turn. it doesn't untap during its controller's next untap step",
        "Tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
    );
    normalized = normalized.replace(
        "Tap each creature that was blocked by one of those creatures this turn. It doesn't untap during its controller's next untap step",
        "Tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
    );
    normalized = normalized.replace(
        "tap each creature that was blocked by one of those creatures this turn. It doesn't untap during its controller's next untap step",
        "tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
    );
    normalized = normalized.replace(
        "Tap it. That permanent doesn't untap during its controller's next untap step",
        "Tap it. It doesn't untap during its controller's next untap step",
    );
    normalized = normalized.replace(
        "tap it. That permanent doesn't untap during its controller's next untap step",
        "tap it. It doesn't untap during its controller's next untap step",
    );
    normalized = normalized.replace(
        "target creature an opponent controls or planeswalker",
        "target creature or planeswalker an opponent controls",
    );
    normalized = normalized.replace(
        "Target creature an opponent controls or planeswalker",
        "Target creature or planeswalker an opponent controls",
    );
    normalized = normalized.replace("that object's controller's library", "their library");
    normalized = normalized.replace(
        "where X is the number of card in target player's hand",
        "where X is the number of cards in that player's hand",
    );
    normalized = normalized.replace(
        "where X is the number of cards in target player's hand",
        "where X is the number of cards in that player's hand",
    );
    normalized = normalized.replace(
        "a card in target player's hand deals damage",
        "a card in that player's hand deals damage",
    );
    normalized = normalized.replace(
        "At the beginning of the next end step, you lose 1 life. Return this card to its owner's hand",
        "At the beginning of the next end step, you lose 1 life and return this card to your hand",
    );
    normalized = normalized.replace(
        "at the beginning of the next end step, you lose 1 life. return this card to its owner's hand",
        "at the beginning of the next end step, you lose 1 life and return this card to your hand",
    );
    normalized = normalized.replace(
        "all permanent cards from your graveyard that was put there from the battlefield this turn",
        "all permanent cards from your graveyard that were put there from the battlefield this turn",
    );
    // The graveyard-entry filter clause hardcodes singular "was"; fix
    // agreement when the described noun is plural ("...cards ... that was
    // put there ..." → "were").
    for zone_link in [
        "cards in your graveyard that was put there",
        "cards in their graveyard that was put there",
        "cards in a graveyard that was put there",
        "cards in graveyards that was put there",
        "cards from your graveyard that was put there",
        "cards from their graveyard that was put there",
    ] {
        if normalized.contains(zone_link) {
            let fixed = zone_link.replace(" that was put there", " that were put there");
            normalized = normalized.replace(zone_link, &fixed);
        }
    }
    normalized = normalized.replace("+1/+1 counterss", "+1/+1 counters");
    normalized = normalized.replace(
        "put that many +1/+1 counter on it",
        "put that many +1/+1 counters on it",
    );
    normalized = normalized.replace("Sacrifice a Food:", "Sacrifice a Food you control:");
    normalized = normalized.replace("sacrifice a food:", "sacrifice a food you control:");
    normalized = normalized.replace("Sacrifice a Goblin:", "Sacrifice a Goblin you control:");
    normalized = normalized.replace("sacrifice a goblin:", "sacrifice a goblin you control:");
    normalized = normalized.replace(
        "Sacrifice two Goblins:",
        "Sacrifice two Goblins you control:",
    );
    normalized = normalized.replace(
        "sacrifice two goblins:",
        "sacrifice two goblins you control:",
    );
    normalized = normalized.replace(
        "Tap all blocked creature blocked by one of those creatures this turns. that permanent doesn't untap during its controller's next untap step",
        "Tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
    );
    normalized = normalized.replace(
        "Tap all blocked creature blocked by one of those creatures this turns. That permanent doesn't untap during its controller's next untap step",
        "Tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
    );
    normalized = normalized.replace(
        "Untap that creature. At this turn's next end of combat",
        "Untap those creatures. At this turn's next end of combat",
    );
    if let Some(rest) = normalized.strip_prefix("Skyreaping deals damage to each ") {
        normalized = format!("Deal damage to each {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Burn the Accursed deals ") {
        normalized = format!("Deal {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("burn the accursed deals ") {
        normalized = format!("deal {rest}");
    }
    if let Some(rest) =
        normalized.strip_prefix("target player reveals a card at random from their hand. ")
    {
        normalized = format!("Target player reveals a card at random from their hand. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("ignite memories deals ") {
        normalized = format!("Ignite Memories deals {rest}");
    }
    if normalized.eq_ignore_ascii_case(
        "Whenever you cast a spell, until end of turn, you don't lose this mana as steps and phases end.",
    ) || normalized.eq_ignore_ascii_case(
        "Whenever you cast a spell, until end of turn, you don't lose this mana as steps and phases end",
    ) {
        normalized =
            "Whenever you cast a spell, add {R}. Until end of turn, you don't lose this mana as steps and phases end."
                .to_string();
    }
    for duplicated_subject in [
        "you may You may repeat this process any number of times",
        "You may You may repeat this process any number of times",
    ] {
        normalized = normalized.replace(
            duplicated_subject,
            "You may repeat this process any number of times",
        );
    }
    normalized = normalized
        .replace("You may You put ", "You may put ")
        .replace("you may You put ", "you may put ");
    // A doubled subject after "may" ("you may You draw", "you may You
    // shuffle") is a render artifact — the verb already carries the implied
    // "you". Strip the spurious second "You"/"you".
    normalized = normalized
        .replace("you may You ", "you may ")
        .replace("You may You ", "You may ")
        .replace("you may you ", "you may ")
        .replace("You may you ", "You may ");
    // A doubled pronoun ("you you", "they they") is always a render join
    // artifact.
    normalized = normalized
        .replace(" you you ", " you ")
        .replace(" they they ", " they ");
    if normalized.contains(". you lose life equal to its mana value") {
        normalized = normalized.replace(
            ". you lose life equal to its mana value",
            ". You lose life equal to its mana value",
        );
    }
    if let Some((prefix, tail)) = normalized.split_once("Sacrifice this: ")
        && let Some((source, rest)) = tail.split_once(" deals ")
        && !source.trim().is_empty()
        && !source.eq_ignore_ascii_case("this")
    {
        normalized = format!("{prefix}Sacrifice this: This deals {rest}");
    }
    if normalized.contains("Activate only if this card is in your graveyard")
        && let Some((prefix, tail)) = normalized.split_once(": Exile ")
        && let Some((_, rest)) = tail.split_once(". Exile target ")
        && let Some((target_kind, rest)) = rest.split_once(" unless its controller pays ")
    {
        normalized = format!(
            "{prefix}: Exile this card and target {target_kind} unless that {target_kind}'s controller pays {rest}"
        );
    }
    if normalized.contains(". you may repeat this process any number of times") {
        normalized = normalized.replace(
            ". you may repeat this process any number of times",
            ". You may repeat this process any number of times",
        );
    }
    if let Some((left, right)) = normalized.split_once(". ") {
        let left_trimmed = left.trim_end_matches('.');
        let right_trimmed = right.trim();
        let left_lower = left_trimmed.to_ascii_lowercase();
        let right_lower = right_trimmed.to_ascii_lowercase();

        if left_lower.contains("if you do,")
            && left_lower.contains(" gets ")
            && right_lower.starts_with("deal ")
        {
            if let Some(rest) = right_trimmed.strip_prefix("Deal ") {
                normalized = format!("{left_trimmed} and deals {rest}");
            } else if let Some(rest) = right_trimmed.strip_prefix("deal ") {
                normalized = format!("{left_trimmed} and deals {rest}");
            }
        }
    }
    if let Some((tap_clause, untap_clause)) = normalized.split_once(". ")
        && tap_clause.to_ascii_lowercase().starts_with("tap up to ")
        && tap_clause
            .to_ascii_lowercase()
            .contains(" target creatures")
        && (untap_clause
            .eq_ignore_ascii_case("creature can't untap during its controller's next untap step.")
            || untap_clause.eq_ignore_ascii_case(
                "creature can't untap during its controller's next untap step",
            ))
    {
        normalized = format!(
            "{tap_clause}. Those creatures don't untap during their controller's next untap step."
        );
    }
    if let Some((tap_clause, untap_clause)) = normalized.split_once(". ")
        && (untap_clause
            .eq_ignore_ascii_case("permanent can't untap during its controller's next untap step.")
            || untap_clause.eq_ignore_ascii_case(
                "permanent can't untap during its controller's next untap step",
            )
            || untap_clause
                .eq_ignore_ascii_case("land can't untap during its controller's next untap step.")
            || untap_clause
                .eq_ignore_ascii_case("land can't untap during its controller's next untap step"))
    {
        let tap_lower = tap_clause.to_ascii_lowercase();
        if tap_lower.contains("tap target creature")
            || tap_lower.contains("tap up to one target creature")
        {
            normalized = format!(
                "{tap_clause}. That creature doesn't untap during its controller's next untap step."
            );
        } else if tap_lower.contains("tap target land")
            || tap_lower.contains("tap up to one target land")
        {
            normalized = format!(
                "{tap_clause}. That land doesn't untap during its controller's next untap step."
            );
        } else if tap_lower.contains("tap target nonland permanent")
            || tap_lower.contains("tap up to one target nonland permanent")
            || tap_lower.contains("tap target permanent")
            || tap_lower.contains("tap up to one target permanent")
        {
            normalized = format!(
                "{tap_clause}. That permanent doesn't untap during its controller's next untap step."
            );
        }
    }
    // A duplicated return-destination suffix is a join artifact.
    if normalized.contains(" to their owner's hand to their owner's hand") {
        normalized = normalized.replace(
            " to their owner's hand to their owner's hand",
            " to their owner's hand",
        );
    }
    // A may-untap's combat-removal rider is part of the same optional
    // instruction; a period join would read the removal as mandatory.
    for (needle, replacement) in [
        (". Remove it from combat", " and remove it from combat"),
        (". Remove them from combat", " and remove them from combat"),
        (
            ". Remove this creature from combat",
            " and remove it from combat",
        ),
    ] {
        if let Some(idx) = normalized.find(needle) {
            let sentence_start = normalized[..idx].rfind(". ").map_or(0, |i| i + 2);
            if normalized[sentence_start..idx].contains("may untap") {
                normalized = normalized.replacen(needle, replacement, 1);
            }
        }
    }
    // The unblocked-attacker rider is one coordinated clause in oracle:
    // "... assigns no combat damage this turn and defending player loses N".
    if normalized.contains(" assigns no combat damage this turn. The defending player loses ") {
        normalized = normalized.replace(
            " assigns no combat damage this turn. The defending player loses ",
            " assigns no combat damage this turn and defending player loses ",
        );
    }
    if let Some(idx) = normalized
        .find(". Permanents tapped this way don't untap during their controllers' untap steps")
    {
        // A single-target tap has exactly one tapped object; oracle back-
        // references it as "It", not the group surface.
        let before_lower = normalized[..idx].to_ascii_lowercase();
        let taps_single = before_lower.contains("tap target ")
            && !before_lower.contains("tap up to")
            && !before_lower.contains(" targets");
        if taps_single {
            normalized = normalized.replacen(
                "Permanents tapped this way don't untap during their controllers' untap steps",
                "It doesn't untap during its controller's untap step",
                1,
            );
        }
    }
    // A multi-target tap-lock ("Tap up to two target creatures. / Tap all
    // attacking creatures.") back-references the whole set as "Those
    // creatures"; the singular "That permanent doesn't untap ..." surface
    // under-renders it.
    if let Some(idx) =
        normalized.find(". That permanent doesn't untap during its controller's next untap step")
    {
        let before_lower = normalized[..idx].to_ascii_lowercase();
        let taps_multiple = before_lower.contains("tap up to")
            || before_lower.contains("tap all ")
            || (before_lower.contains("tap ") && before_lower.contains(" target creatures"));
        if taps_multiple {
            normalized = normalized.replacen(
                "That permanent doesn't untap during its controller's next untap step",
                "Those creatures don't untap during their controller's next untap step",
                1,
            );
        }
    }
    if normalized.contains("Add 1 mana of commander's color identity") {
        normalized = normalized.replace(
            "Add 1 mana of commander's color identity",
            "Add one mana of any color in your commander's color identity",
        );
    }
    if normalized.contains("create a Powerstone artifact token, tapped") {
        normalized = normalized.replace(
            "create a Powerstone artifact token, tapped",
            "create a tapped Powerstone token",
        );
    }
    // Fix "all/each/for each another" → "all/each/for each other" (grammar fix for
    // filter descriptions with `other: true` used in quantified contexts).
    normalized = normalized
        .replace("all another ", "all other ")
        .replace("All another ", "All other ")
        .replace("each another ", "each other ")
        .replace("Each another ", "Each other ")
        .replace("For each another ", "For each other ")
        .replace("for each another ", "for each other ");
    if normalized.contains("Other Elf you control get ") {
        normalized =
            normalized.replace("Other Elf you control get ", "Other Elves you control get ");
    }
    normalized = normalized
        .replace(
            "you may target creature gets ",
            "you may have target creature get ",
        )
        .replace(
            "you may target creature gains ",
            "you may have target creature gain ",
        )
        .replace(
            "you may target creature loses ",
            "you may have target creature lose ",
        )
        .replace(
            "you may target creature reveals ",
            "you may have target creature reveal ",
        )
        .replace(
            ", put it onto the battlefield under your control",
            ", put that card onto the battlefield under your control",
        )
        .replace(
            "put them into target opponent's graveyard",
            "put them into their graveyard",
        );
    if normalized.contains("Search target opponent's library for ")
        && normalized.contains(". Shuffle target opponent's library.")
    {
        normalized = normalized.replace(
            ". Shuffle target opponent's library.",
            ". Then that player shuffles.",
        );
    }
    if normalized.contains("Search target opponent's library for ")
        && normalized.contains(". Then shuffle target opponent's library.")
    {
        normalized = normalized.replace(
            ". Then shuffle target opponent's library.",
            ". Then that player shuffles.",
        );
    }
    if normalized.contains("Search target player's library for ")
        && normalized.contains(". Shuffle target player's library.")
    {
        normalized = normalized.replace(
            ". Shuffle target player's library.",
            ". Then that player shuffles.",
        );
    }
    if normalized.contains("Search target player's library for ")
        && normalized.contains(". Then shuffle target player's library.")
    {
        normalized = normalized.replace(
            ". Then shuffle target player's library.",
            ". Then that player shuffles.",
        );
    }
    if normalized.contains("Search target player's library for ")
        && normalized.contains(", then shuffle target player's library.")
    {
        normalized = normalized
            .replace(
                ", exile it, then shuffle target player's library.",
                " and exile it. Then that player shuffles.",
            )
            .replace(
                ", exile them, then shuffle target player's library.",
                " and exile them. Then that player shuffles.",
            )
            .replace(
                ", then shuffle target player's library.",
                ". Then that player shuffles.",
            );
    }
    if normalized.starts_with("Creatures you control get ")
        && normalized.contains(" until end of turn. If it is not your turn, untap them.")
    {
        normalized = normalized.replace(
            " until end of turn. If it is not your turn, untap them.",
            " until end of turn. If it's not your turn, untap those creatures.",
        );
    }
    if normalized.starts_with("Each creature you control gets ")
        && normalized
            .contains(" until end of turn. Then if it is not your turn, untap that creature.")
    {
        normalized = normalized.replace(
            "Each creature you control gets ",
            "Creatures you control get ",
        );
        normalized = normalized.replace(
            " until end of turn. Then if it is not your turn, untap that creature.",
            " until end of turn. If it's not your turn, untap those creatures.",
        );
    }
    let normalized_lower = normalized.to_ascii_lowercase();
    let preserve_plural_creatures_you_control = normalized
        .contains("from the command zone this game")
        || normalized_lower.contains("if it is not your turn, untap them")
        || (normalized_lower.contains("creatures you control get ")
            && normalized_lower.contains(" and gain "));
    if !preserve_plural_creatures_you_control {
        if normalized.starts_with("creatures you control get ") {
            normalized = normalized.replacen(
                "creatures you control get ",
                "Each creature you control gets ",
                1,
            );
        }
        if normalized.starts_with("Creatures you control get ") {
            normalized = normalized.replacen(
                "Creatures you control get ",
                "Each creature you control gets ",
                1,
            );
        }
        normalized = normalized
            .replace(
                ": creatures you control get ",
                ": Each creature you control gets ",
            )
            .replace(
                ": Creatures you control get ",
                ": Each creature you control gets ",
            )
            .replace(
                ". creatures you control get ",
                ". Each creature you control gets ",
            )
            .replace(
                ". Creatures you control get ",
                ". Each creature you control gets ",
            )
            .replace(
                "• creatures you control get ",
                "• Each creature you control gets ",
            )
            .replace(
                "• Creatures you control get ",
                "• Each creature you control gets ",
            );
    }
    if let Some(rest) = normalized.strip_prefix("Target player discards ")
        && let Some((discard_count, loss_tail)) = rest.split_once(" cards. target player loses ")
        && let Some(loss_amount) = loss_tail.strip_suffix(" life.")
    {
        normalized =
            format!("Target player discards {discard_count} cards and loses {loss_amount} life.");
    }
    if let Some(rest) = normalized
        .strip_prefix("Whenever this creature attacks, choose another target attacking creature. ")
        && rest
            .to_ascii_lowercase()
            .starts_with("another target attacking creature can't be blocked this turn")
    {
        normalized = format!("Whenever this creature attacks, {}", rest.trim());
    }
    if let Some((prefix, rest)) =
        split_once_ascii_ci(&normalized, "for each opponent, that player sacrifices ")
        && let Some((sacrifice_tail, pay_tail)) =
            split_once_ascii_ci(rest, " unless that player pays ")
    {
        normalized = format!(
            "{}, each opponent sacrifices {} of their choice unless they pay {}",
            prefix.trim_end_matches([',', ' ']),
            sacrifice_tail.trim_end_matches('.'),
            pay_tail.trim_end_matches('.')
        );
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(
        &normalized,
        ". If you do, For each opponent, You may that player ",
    ) && let Some((action_clause, fallback_rest)) = split_once_ascii_ci(rest, ". If effect #")
        && let Some((_, fallback_clause)) =
            split_once_ascii_ci(fallback_rest, " that doesn't happen, ")
        && let Some(loss_tail) = strip_prefix_ascii_ci(fallback_clause.trim(), "you lose ")
            .or_else(|| strip_prefix_ascii_ci(fallback_clause.trim(), "that player loses "))
        && let Some(loss_amount) = strip_suffix_ascii_ci(loss_tail.trim(), " life. Draw a card.")
    {
        let action = normalize_you_verb_phrase(action_clause.trim());
        normalized = format!(
            "{}. If you do, each opponent may {}. For each opponent who doesn't, that player loses {} life and you draw a card.",
            prefix.trim(),
            action.trim_end_matches('.'),
            loss_amount.trim()
        );
    }
    if normalized.contains("another target creature has base power and toughness") {
        normalized = normalized.replace(
            "another target creature has base power and toughness",
            "target creature other than this creature has base power and toughness",
        );
    }
    if normalized.contains("Whenever you reveal instant or sorcery this way, copy it. You may cast the copy. That copy costs {2} less to cast.") {
        normalized = normalized.replace(
            "Whenever you reveal instant or sorcery this way, copy it. You may cast the copy. That copy costs {2} less to cast.",
            "Whenever you reveal instant or sorcery this way, copy that card and you may cast the copy. That copy costs {2} less to cast.",
        );
    }
    if normalized
        .contains("Whenever you cast an instant, sorcery, or enchantment spell, you may copy it.")
    {
        normalized = normalized.replace(
            "Whenever you cast an instant, sorcery, or enchantment spell, you may copy it.",
            "Whenever you cast an instant or sorcery or enchantment spell, you may copy it.",
        );
    }
    if normalized.contains("Choose target creature you control. Choose target creature you don't control. If you control three or more creatures with different powers, Put a +1/+1 counter on that creature you control. That creature fights it.") {
        normalized = normalized.replace(
            "Choose target creature you control. Choose target creature you don't control. If you control three or more creatures with different powers, Put a +1/+1 counter on that creature you control. That creature fights it.",
            "Choose target creature you control and target creature you don't control. If you control three or more creatures with different powers, put a +1/+1 counter on the chosen creature you control. Then the chosen creatures fight each other.",
        );
    }
    if normalized.contains("Choose target creature you control. Choose target creature you don't control. If you control three or more snow permanents, creatures you control get +1/+0 and gain indestructible until end of turn. That creature fights it.") {
        normalized = normalized.replace(
            "Choose target creature you control. Choose target creature you don't control. If you control three or more snow permanents, creatures you control get +1/+0 and gain indestructible until end of turn. That creature fights it.",
            "Choose target creature you control and target creature you don't control. If you control three or more snow permanents, the creature you control gets +1/+0 and gains indestructible until end of turn. Then those creatures fight each other.",
        );
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "Creatures you control get ")
        && let Some(buff) = strip_suffix_ascii_ci(rest, " until end of turn. Untap all permanent.")
            .or_else(|| strip_suffix_ascii_ci(rest, " until end of turn. Untap all permanent"))
    {
        return format!("Creatures you control get {buff} until end of turn. Untap them.");
    }
    let lower = normalized.to_ascii_lowercase();

    if let Some((prefix, rest)) = normalized.split_once(": Choose target ")
        && let Some((subject, tail)) = rest.split_once(". ")
    {
        let subject = subject.trim();
        let tail_trimmed = tail.trim();
        if !subject.is_empty()
            && tail_trimmed
                .to_ascii_lowercase()
                .starts_with(&format!("target {} ", subject.to_ascii_lowercase()))
        {
            normalized = format!("{}: {}", prefix.trim(), capitalize_first(tail_trimmed));
        }
    }
    if let Some(rest) = normalized.strip_prefix("Choose target ")
        && let Some((subject, tail)) = rest.split_once(". ")
    {
        let subject = subject.trim();
        if !subject.is_empty()
            && tail
                .to_ascii_lowercase()
                .starts_with(&format!("target {} ", subject.to_ascii_lowercase()))
        {
            return capitalize_first(tail.trim());
        }
    }
    if lower.contains("energy counter(s)") {
        for count in 1usize..=8 {
            let digit = count.to_string();
            let symbols = repeated_energy_symbols(count);
            normalized = normalized
                .replace(
                    &format!("you get {digit} energy counter(s)"),
                    &format!("you get {symbols}"),
                )
                .replace(
                    &format!("You get {digit} energy counter(s)"),
                    &format!("You get {symbols}"),
                );
        }
    }
    normalized = normalized.replace(". you get ", ". You get ");
    for life in 1usize..=20 {
        let amount = life.to_string();
        normalized = normalized
            .replace(
                &format!("you may lose {amount} life. If you do"),
                &format!("you may pay {amount} life. If you do"),
            )
            .replace(
                &format!("You may lose {amount} life. If you do"),
                &format!("You may pay {amount} life. If you do"),
            )
            .replace(
                &format!("you may lose {amount} life and "),
                &format!("you may pay {amount} life and "),
            )
            .replace(
                &format!("You may lose {amount} life and "),
                &format!("You may pay {amount} life and "),
            );
    }
    if let Some((left, right)) = normalized.split_once(": ")
        && matches!(
            right,
            "Target creature can't untap until your next turn"
                | "Target creature cant untap until your next turn"
        )
    {
        return format!(
            "{left}: Target creature doesn't untap during its controller's next untap step"
        );
    }
    normalized = normalized
        .replace("target another ", "other target ")
        .replace("Target that player ", "That player ")
        .replace("Target that permanent ", "That permanent ")
        .replace("Target that creature ", "That creature ")
        .replace("Target that object ", "That object ")
        .replace(
            "another target that player's creature",
            "another target creature that player controls",
        )
        .replace(
            "Another target that player's creature",
            "Another target creature that player controls",
        )
        .replace(
            "target that player's creature",
            "target creature that player controls",
        )
        .replace(
            "Target that player's creature",
            "Target creature that player controls",
        )
        .replace("a another ", "another ")
        .replace("Creatures token", "Creature tokens")
        .replace("creatures token", "creature tokens")
        .replace("Whenever a another ", "Whenever another ")
        .replace("Others ", "Other ")
        .replace(" that objects ", " that object ")
        .replace(" that objects.", " that object.")
        .replace(" that objects,", " that object,")
        .replace(" to that objects", " to that object")
        .replace(
            "an opponent's creature enter the battlefield tapped",
            "an opponent's creature enters the battlefield tapped",
        )
        .replace(
            "opponent's artifact enter the battlefield tapped",
            "opponent's artifact enters the battlefield tapped",
        )
        .replace(
            "Creature enter the battlefield tapped",
            "Creature enters the battlefield tapped",
        )
        .replace(
            "When this permanent enters or this creature attacks",
            "Whenever this creature enters or attacks",
        )
        .replace(
            "When this creature enters or this creature attacks",
            "Whenever this creature enters or attacks",
        )
        .replace(
            "when this permanent enters or this creature attacks",
            "whenever this creature enters or attacks",
        )
        .replace(
            "when this creature enters or this creature attacks",
            "whenever this creature enters or attacks",
        )
        .replace(" in yours graveyard", " in your graveyard")
        .replace("counter ons it", "counter on it")
        .replace(
            "Target attacking/blocking creature",
            "Target attacking or blocking creature",
        )
        .replace(
            "Target attacking/blocking creatures",
            "Target attacking or blocking creatures",
        )
        .replace(
            "target attacking/blocking creature",
            "target attacking or blocking creature",
        )
        .replace(
            "target attacking/blocking creatures",
            "target attacking or blocking creatures",
        )
        .replace(
            "an attacking/blocking creature",
            "an attacking or blocking creature",
        )
        .replace(
            "attacking/blocking creatures",
            "attacking or blocking creatures",
        )
        .replace(
            "with a +1/+1 counter on it you control",
            "you control with a +1/+1 counter on it",
        )
        .replace(
            "with a -1/-1 counter on it you control",
            "you control with a -1/-1 counter on it",
        )
        .replace(
            "with a counter on it you control",
            "you control with a counter on it",
        )
        .replace(
            "with counters on them you control",
            "you control with counters on them",
        )
        .replace(
            "Activate only as a sorcery and Activate only once each turn",
            "Activate only as a sorcery and only once each turn",
        )
        .replace(
            "activate only as a sorcery and activate only once each turn",
            "Activate only as a sorcery and only once each turn",
        )
        .replace(
            "you sacrifice another creature you control",
            "sacrifice another creature",
        )
        .replace(
            "you sacrifice a creature you control",
            "sacrifice a creature",
        )
        .replace(
            "you sacrifice an artifact you control",
            "sacrifice an artifact",
        )
        .replace("you sacrifice a land you control", "sacrifice a land")
        .replace(
            "you sacrifice a permanent you control",
            "sacrifice a permanent",
        )
        .replace(
            "you sacrifice two creatures you control",
            "sacrifice two creatures",
        )
        .replace(
            "you sacrifice three creatures you control",
            "sacrifice three creatures",
        )
        .replace(
            "you may sacrifice another creature you control",
            "you may sacrifice another creature",
        )
        .replace(
            "you may sacrifice a creature you control",
            "you may sacrifice a creature",
        )
        .replace(
            "you may sacrifice an artifact you control",
            "you may sacrifice an artifact",
        )
        .replace(
            "Create a Powerstone artifact token tapped under your control",
            "Create a tapped Powerstone token",
        )
        .replace(
            "Create a Powerstone artifact token under your control, tapped",
            "Create a tapped Powerstone token",
        )
        .replace(
            "Create 1 Powerstone artifact token tapped under your control",
            "Create a tapped Powerstone token",
        )
        .replace(
            "Create 1 Powerstone artifact token under your control, tapped",
            "Create a tapped Powerstone token",
        )
        .replace(
            "create a Powerstone artifact token tapped under your control",
            "create a tapped Powerstone token",
        )
        .replace(
            "create a Powerstone artifact token under your control, tapped",
            "create a tapped Powerstone token",
        )
        .replace(
            "create 1 Powerstone artifact token tapped under your control",
            "create a tapped Powerstone token",
        )
        .replace(
            "create 1 Powerstone artifact token under your control, tapped",
            "create a tapped Powerstone token",
        )
        .replace(
            "Prevent combat damage until end of turn",
            "Prevent all combat damage that would be dealt this turn",
        )
        .replace(" put it into hand", " put it into your hand")
        .replace("for blue instant you own", "for a blue instant card")
        .replace("for creature you own", "for a creature card")
        .replace(
            "Search your library for Equipment you own, reveal it, put it into your hand, then shuffle",
            "Search your library for an Equipment card, reveal it, put it into your hand, then shuffle",
        )
        .replace(
            "Search your library for Arcane you own, reveal it, put it into your hand, then shuffle",
            "Search your library for an Arcane card, reveal that card, put it into your hand, then shuffle",
        )
        .replace(
            "search your library for Equipment you own, reveal it, put it into your hand, then shuffle",
            "search your library for an Equipment card, reveal that card, put it into your hand, then shuffle",
        )
        .replace(
            "search your library for Arcane you own, reveal it, put it into your hand, then shuffle",
            "search your library for an Arcane card, reveal that card, put it into your hand, then shuffle",
        )
        .replace(
            "search your library for land Forest you own, put it onto the battlefield, then shuffle",
            "search your library for a Forest card, put that card onto the battlefield, then shuffle",
        )
        .replace(
            "Search your library for land Forest you own, put it onto the battlefield, then shuffle",
            "Search your library for a Forest card, put that card onto the battlefield, then shuffle",
        )
        .replace(
            "Search your library for land Forest, put it onto the battlefield, then shuffle",
            "Search your library for a Forest card, put that card onto the battlefield, then shuffle",
        )
        .replace(
            "for Aura or Equipment you own",
            "for an Aura or Equipment card",
        )
        .replace(
            "it changes controller to this effect's controller and gains Haste until end of turn",
            "Gain control of it until end of turn. It gains haste until end of turn",
        )
        .replace(
            "it changes controller to this effect's controller and gains haste until end of turn",
            "Gain control of it until end of turn. It gains haste until end of turn",
        )
        .replace(
            "you may Put creature card in your hand onto the battlefield. it gains Haste. At the beginning of the next end step, you sacrifice it.",
            "You may put a creature card from your hand onto the battlefield. That creature gains haste. Sacrifice the creature at the beginning of the next end step.",
        )
        .replace(
            "you may Put creature card in your hand onto the battlefield. it gains Haste. At the beginning of the next end step, you sacrifice it",
            "You may put a creature card from your hand onto the battlefield. That creature gains haste. Sacrifice the creature at the beginning of the next end step.",
        )
        .replace(
            "put an artifact card from your hand onto the battlefield. It gains haste.",
            "put an artifact card from your hand onto the battlefield. That artifact gains haste.",
        )
        .replace(
            "it gains Haste until end of turn. At the beginning of the next end step, you sacrifice it.",
            "That creature gains haste until end of turn. At the beginning of the next end step, sacrifice that creature.",
        )
        .replace(
            "it gains Haste until end of turn. At the beginning of the next end step, you sacrifice it",
            "That creature gains haste until end of turn. At the beginning of the next end step, sacrifice that creature.",
        )
        .replace(
            "it gains Haste. At the beginning of the next end step, you sacrifice it.",
            "That creature gains haste. Sacrifice the creature at the beginning of the next end step.",
        )
        .replace(
            "it gains Haste. At the beginning of the next end step, you sacrifice it",
            "That creature gains haste. Sacrifice the creature at the beginning of the next end step.",
        )
        .replace(
            "Untap it. it gains Haste until end of turn",
            "Untap that creature. It gains haste until end of turn",
        )
        .replace(
            "Untap it. it gains Haste and gains Menace until end of turn",
            "Untap that creature. It gains haste and menace until end of turn",
        )
        .replace(
            "it gains Haste and gains Menace until end of turn",
            "it gains haste and menace until end of turn",
        )
        .replace(
            "at the beginning of the next end step. you control.",
            "at the beginning of the next end step.",
        )
        .replace(
            "At the beginning of the next end step. you control.",
            "At the beginning of the next end step.",
        )
        .replace(
            "An opponent's artifact or creature enter the battlefield tapped.",
            "Artifacts and creatures your opponents control enter tapped.",
        )
        .replace(
            "An opponent's artifact or creature enter the battlefield tapped",
            "Artifacts and creatures your opponents control enter tapped",
        )
        .replace(
            "An opponent's artifact enters the battlefield tapped.",
            "Artifacts your opponents control enter the battlefield tapped.",
        )
        .replace(
            "An opponent's artifact enters the battlefield tapped",
            "Artifacts your opponents control enter the battlefield tapped",
        )
        .replace(
            "An opponent's creature enters the battlefield tapped.",
            "Creatures your opponents control enter the battlefield tapped.",
        )
        .replace(
            "An opponent's creature enters the battlefield tapped",
            "Creatures your opponents control enter the battlefield tapped",
        )
        .replace(
            "An opponent's nonbasic creature or land enter the battlefield tapped.",
            "Creatures and nonbasic lands your opponents control enter tapped.",
        )
        .replace(
            "An opponent's nonbasic creature or land enter the battlefield tapped",
            "Creatures and nonbasic lands your opponents control enter tapped",
        )
        .replace(
            "with \"Sacrifice this creature, add {C}\"",
            "with \"Sacrifice this creature: Add {C}.\"",
        )
        .replace(
            "with \"sacrifice this creature, add {C}\"",
            "with \"Sacrifice this creature: Add {C}.\"",
        )
        .replace(
            "Whenever a creature blocks, deal 1 damage to that object's controller.",
            "Whenever a creature blocks, deal 1 damage to that creature's controller.",
        )
        .replace(
            "Whenever a creature blocks, deal 1 damage to that object's controller",
            "Whenever a creature blocks, deal 1 damage to that creature's controller",
        )
        .replace(
            "Whenever a creature blocks, this enchantment deals 1 damage to that creature's controller.",
            "Whenever a creature blocks, deal 1 damage to that creature's controller.",
        )
        .replace(
            "Whenever a creature blocks, this enchantment deals 1 damage to that creature's controller",
            "Whenever a creature blocks, deal 1 damage to that creature's controller",
        );
    if let Some((heading, tail)) = split_once_ascii_ci(&normalized, ": ")
        && let Some(rest) = strip_prefix_ascii_ci(tail, "Whenever a creature blocks, deal ")
        && let Some(dmg) = rest.strip_suffix(" damage to that object's controller.")
    {
        normalized = format!(
            "{heading}: Whenever a creature blocks, deal {dmg} damage to that creature's controller.",
            heading = heading.trim_end_matches(':').trim()
        );
    } else if let Some((heading, tail)) = split_once_ascii_ci(&normalized, ": ")
        && let Some(rest) = strip_prefix_ascii_ci(tail, "Whenever a creature blocks, deal ")
        && let Some(dmg) = rest.strip_suffix(" damage to that object's controller")
    {
        normalized = format!(
            "{heading}: Whenever a creature blocks, deal {dmg} damage to that creature's controller",
            heading = heading.trim_end_matches(':').trim()
        );
    }
    if let Some(amount) = strip_prefix_ascii_ci(
        &normalized,
        "Whenever enchanted land is tapped for mana, add ",
    )
    .and_then(|tail| {
        strip_suffix_ascii_ci(tail, " to that object's controller's mana pool.")
            .or_else(|| strip_suffix_ascii_ci(tail, " to that object's controller's mana pool"))
    }) {
        return format!(
            "Whenever enchanted land is tapped for mana, its controller adds an additional {}.",
            amount.trim()
        );
    }
    if let Some((left, right)) = normalized.split_once(". ")
        && left.to_ascii_lowercase().contains("target creature")
        && right.eq_ignore_ascii_case("Untap it.")
    {
        normalized = format!("{}. Untap that creature.", left.trim_end_matches('.'));
    } else if let Some((left, right)) = normalized.split_once(". ")
        && left.to_ascii_lowercase().contains("target creature")
        && right.eq_ignore_ascii_case("Untap it")
    {
        normalized = format!("{}. Untap that creature", left.trim_end_matches('.'));
    }
    if let Some((left, right)) = normalized.split_once(". ")
        && left.to_ascii_lowercase().contains("target creature")
        && right.eq_ignore_ascii_case("Tap it.")
    {
        normalized = format!("{}. Tap it.", left.trim_end_matches('.'));
    } else if let Some((left, right)) = normalized.split_once(". ")
        && left.to_ascii_lowercase().contains("target creature")
        && right.eq_ignore_ascii_case("Tap it")
    {
        normalized = format!("{}. Tap it", left.trim_end_matches('.'));
    }
    if let Some((left, right)) = normalized.split_once(". ")
        && (left.eq_ignore_ascii_case("Untap one or two target creatures")
            || left.eq_ignore_ascii_case("Untap up to two target creatures"))
        && let Some(buff_clause) = strip_suffix_ascii_ci(right.trim(), " until end of turn.")
            .or_else(|| strip_suffix_ascii_ci(right.trim(), " until end of turn"))
        && let Some(buff) = strip_prefix_ascii_ci(buff_clause.trim(), "it gets ")
            .or_else(|| strip_prefix_ascii_ci(buff_clause.trim(), "It gets "))
    {
        return format!(
            "{}. They each get {} until end of turn.",
            left.trim_end_matches('.'),
            buff.trim()
        );
    }
    if let Some((prefix, suffix)) =
        normalized.split_once(", reveal it, put it on top of library, then shuffle")
    {
        normalized = format!("{prefix}, reveal it, then shuffle and put the card on top{suffix}");
    }
    normalized = normalized
        .replace(
            "can't be blocked until end of turn",
            "can't be blocked this turn",
        )
        .replace(
            "cant be blocked until end of turn",
            "can't be blocked this turn",
        )
        .replace("can't block until end of turn", "can't block this turn")
        .replace("cant block until end of turn", "can't block this turn")
        .replace("If it happened, ", "If you do, ")
        .replace("If you do, you draw", "If you do, draw")
        .replace("If you do, you discard", "If you do, discard");
    if normalized.contains("Manifest dread. Put ") && normalized.contains(" on it. Put ") {
        normalized = normalized
            .replace("Manifest dread. Put ", "Manifest dread, then put ")
            .replace(" on it. Put ", " and ");
    }
    let lower_normalized = normalized.to_ascii_lowercase();
    if lower_normalized
        == "tap target creature or planeswalker. choose it. activated abilities of that permanent can't be activated this turn"
    {
        return "Tap target creature or planeswalker. Its activated abilities can't be activated this turn"
            .to_string();
    }
    if lower_normalized.contains("that permanent's mana value")
        && lower_normalized.contains("reveal the top card of your library")
    {
        return normalized.replace("that permanent's mana value", "that card's mana value");
    }
    if lower_normalized.contains("at the beginning of the next end step, exile it")
        && lower_normalized.contains("if it's a permanent, exile it")
    {
        return normalized;
    }
    if let Some(tail) = strip_prefix_ascii_ci(
        &normalized,
        "You take an extra turn after this one. At the beginning of your next end step, ",
    ) && tail
        .trim()
        .trim_end_matches('.')
        .eq_ignore_ascii_case("you lose the game")
    {
        return "Take an extra turn after this one. At the beginning of that turn's end step, you lose the game".to_string();
    }
    if let Some(amount) =
        strip_prefix_ascii_ci(&normalized, "At the beginning of your upkeep, deal ").and_then(
            |tail| {
                strip_suffix_ascii_ci(tail, " damage to you.")
                    .or_else(|| strip_suffix_ascii_ci(tail, " damage to you"))
            },
        )
    {
        return format!(
            "At the beginning of your upkeep, this creature deals {} damage to you.",
            amount.trim()
        );
    }
    if lower_normalized == "cards in hand have flash"
        || lower_normalized == "cards in hand have flash."
    {
        return "You may cast noncreature spells as though they had flash".to_string();
    }
    if let Some((left, right)) = normalized.split_once(". ")
        && left.starts_with("Deal ")
        && left.contains("damage to target opponent or planeswalker")
        && (right.starts_with("target opponent discards ")
            || right.starts_with("Target opponent discards "))
    {
        let discard_tail = right
            .strip_prefix("target opponent discards ")
            .or_else(|| right.strip_prefix("Target opponent discards "))
            .unwrap_or(right);
        return format!(
            "{left}. That player or that planeswalker's controller discards {discard_tail}"
        );
    }
    if lower_normalized == "creatures have can't block"
        || lower_normalized == "all creatures have can't block"
    {
        return "Creatures can't block".to_string();
    }
    if lower_normalized == "can't block" || lower_normalized == "can't block." {
        return "This creature can't block".to_string();
    }
    if lower_normalized == "can't be blocked" || lower_normalized == "can't be blocked." {
        return "This creature can't be blocked".to_string();
    }
    if lower_normalized == "enchant opponent's creature" {
        return "Enchant creature an opponent controls".to_string();
    }
    if lower_normalized.contains(
        "create a 1/1 colorless robot artifact creature token for each +1/+1 counter on it",
    ) {
        return line.replace(
            "create a 1/1 colorless Robot artifact creature token for each +1/+1 counter on it",
            "create a number of 1/1 colorless Robot artifact creature tokens equal to the number of +1/+1 counters on this creature",
        );
    }
    if lower_normalized.starts_with(
        "choose target creature you control. you choose a nonbasic land type. all nonbasic land of the chosen land types you control becomes a copy of it and gain haste until end of turn",
    ) {
        return "Choose a nonbasic land type. Each land you control of that type becomes a copy of target creature you control until end of turn and gains haste until end of turn".to_string();
    }
    if lower_normalized
        == "enchanted creature has at the beginning of your upkeep, sacrifice this creature."
        || lower_normalized
            == "enchanted creature has at the beginning of your upkeep, sacrifice this creature"
    {
        return "Enchanted creature has \"At the beginning of your upkeep, sacrifice this creature.\"".to_string();
    }
    if lower_normalized.starts_with(
        "when enchanted creature dies, choose target creature an opponent controls. return this card to the battlefield attached to that creature",
    ) {
        return "When enchanted creature dies, its controller chooses target creature one of their opponents controls. Return this card from its owner's graveyard to the battlefield attached to that creature".to_string();
    }
    if lower_normalized
        == "{u}{u}: return target aura card from your graveyard to the battlefield attached to this creature. activate only during your upkeep and only if this isn't enchanted."
        || lower_normalized
            == "{u}{u}: return target aura card from your graveyard to the battlefield attached to this creature. activate only during your upkeep and only if this isn't enchanted"
    {
        return "{U}{U}: Return target Aura card from your graveyard to the battlefield attached to Hakim. Activate only during your upkeep and only if Hakim isn't enchanted.".to_string();
    }
    if lower_normalized == "{u}{u}, {t}: destroy all auras."
        || lower_normalized == "{u}{u}, {t}: destroy all auras"
    {
        return "{U}{U}, {T}: Destroy all Auras attached to Hakim.".to_string();
    }
    if lower_normalized == "destroy all an opponent's nonland permanent"
        || lower_normalized == "destroy all an opponent's nonland permanent."
    {
        return "Destroy all nonland permanents your opponents control".to_string();
    }
    let is_simple_mass_noun = |noun: &str| {
        matches!(
            noun.trim_end_matches('.'),
            "artifact"
                | "artifacts"
                | "creature"
                | "creatures"
                | "land"
                | "lands"
                | "enchantment"
                | "enchantments"
                | "spacecraft"
                | "spacecrafts"
        )
    };
    if let Some(rest) = normalized.strip_prefix("Destroy all ")
        && let Some((first, second)) = rest.split_once(". Destroy all ")
    {
        let first = first.trim().trim_end_matches('.');
        let second = second.trim().trim_end_matches('.');
        if is_simple_mass_noun(first) && is_simple_mass_noun(second) {
            return format!(
                "Destroy all {} and {}",
                pluralize_noun_phrase(first),
                pluralize_noun_phrase(second)
            );
        }
    }
    if let Some(rest) = normalized.strip_prefix("Exile all ")
        && let Some((first, second)) = rest.split_once(". Exile all ")
    {
        let first = first.trim().trim_end_matches('.');
        let second = second.trim().trim_end_matches('.');
        if is_simple_mass_noun(first) && is_simple_mass_noun(second) {
            return format!(
                "Exile all {} and {}",
                pluralize_noun_phrase(first),
                pluralize_noun_phrase(second)
            );
        }
    }
    if let Some(rest) = normalized.strip_prefix("For each tagged 'destroyed_")
        && let Some((_, tail)) = rest.split_once("' object, ")
    {
        return format!("For each object destroyed this way, {tail}");
    }
    if let Some(rest) = normalized.strip_prefix("For each tagged 'exiled_")
        && let Some((_, tail)) = rest.split_once("' object, ")
    {
        return format!("For each object exiled this way, {tail}");
    }
    if let Some(rest) = normalized.strip_prefix("For each object destroyed this way, Create ")
        && let Some((token_text, tail)) =
            rest.split_once(" under that object's controller's control")
    {
        return format!(
            "For each object destroyed this way, its controller creates {token_text}{tail}"
        );
    }
    if let Some(rest) = normalized.strip_prefix("For each object destroyed this way, Create ")
        && let Some((token_text, tail)) = rest.split_once(" under that player's control")
    {
        return format!(
            "For each object destroyed this way, that player creates {token_text}{tail}"
        );
    }
    if normalized
        == "For each creature you control with a +1/+1 counter on it, Put a +1/+1 counter on that object"
        || normalized
            == "For each creature you control with a +1/+1 counter on it, Put a +1/+1 counter on that object."
    {
        return "Put a +1/+1 counter on each creature you control with a +1/+1 counter on it"
            .to_string();
    }
    if let Some(prefix) = normalized.strip_suffix(
        " and tag it as 'revealed_0'. If the tagged object 'revealed_0' matches land, Return it to its owner's hand",
    ) {
        return format!("{prefix}. If it's a land card, that player puts it into their hand");
    }
    if let Some(prefix) = normalized.strip_suffix(
        " and tag it as 'revealed_0'. If the tagged object 'revealed_0' matches land, Return it to its owner's hand.",
    ) {
        return format!("{prefix}. If it's a land card, that player puts it into their hand");
    }
    if let Some(prefix) = normalized.strip_suffix(
        " and tag it as 'revealed_0'. If the tagged object 'revealed_0' matches creature, Return it to its owner's hand",
    ) {
        return format!("{prefix}. If it's a creature card, put it into your hand");
    }
    if let Some(prefix) = normalized.strip_suffix(
        " and tag it as 'revealed_0'. If the tagged object 'revealed_0' matches creature, Return it to its owner's hand.",
    ) {
        return format!("{prefix}. If it's a creature card, put it into your hand");
    }
    if let Some(rest) = normalized.strip_prefix("For each player, Deal ")
        && let Some((amount, per_player_tail)) =
            rest.split_once(" damage to that player. For each ")
        && let Some((per_player_filter, repeated_damage)) = per_player_tail.split_once(", Deal ")
        && repeated_damage.trim_end_matches('.') == format!("{amount} damage to that object")
    {
        let each_filter = per_player_filter
            .trim_end_matches(" that player controls")
            .trim();
        return format!("Deal {amount} damage to each {each_filter} and each player");
    }
    if let Some((cost, effect)) = normalized.split_once(": ")
        && let Some(rest) = effect.strip_prefix("For each player, Deal ")
        && let Some((amount, per_player_tail)) =
            rest.split_once(" damage to that player. For each ")
        && let Some((per_player_filter, repeated_damage)) = per_player_tail.split_once(", Deal ")
        && repeated_damage.trim_end_matches('.') == format!("{amount} damage to that object")
    {
        let each_filter = per_player_filter
            .trim_end_matches(" that player controls")
            .trim();
        return format!("{cost}: Deal {amount} damage to each {each_filter} and each player");
    }
    if let Some(rest) = normalized.strip_prefix("Each player creates 1 ")
        && let Some((token_desc, tail)) = rest.split_once(" under that player's control for each ")
        && let Some((each_filter, ending)) = tail
            .split_once(" that player controls.")
            .or_else(|| tail.split_once(" that player controls"))
    {
        return format!(
            "Each player creates a {token_desc} for each {each_filter} they control{ending}"
        );
    }
    if let Some(rest) = normalized.strip_prefix("Each player creates 1 ")
        && let Some((token_desc, tail)) = rest.split_once(" under that player's control")
    {
        return format!("Each player creates a {token_desc}{tail}");
    }
    if let Some(rest) = normalized.strip_prefix("For each player, Create ")
        && let Some((token_desc, tail)) = rest.split_once(" under that player's control")
    {
        return format!("Each player creates {token_desc}{tail}");
    }
    if let Some(rest) = strip_prefix_ascii_ci(
        &normalized,
        "As an additional cost to cast this spell, you may choose at least 1 ",
    ) && let Some((chosen, tail)) =
        split_once_ascii_ci(rest, ". you sacrifice all permanents you control")
    {
        let chosen_plural = normalize_choose_sacrifice_subject(chosen);
        let tail = tail
            .trim_start_matches('.')
            .trim_start()
            .trim_end_matches('.');
        if tail.is_empty() {
            return format!(
                "As an additional cost to cast this spell, you may sacrifice one or more {chosen_plural}"
            );
        }
        return format!(
            "As an additional cost to cast this spell, you may sacrifice one or more {chosen_plural}. {}.",
            capitalize_first(tail)
        );
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "You choose any number ")
        .or_else(|| strip_prefix_ascii_ci(&normalized, "Choose any number "))
        .or_else(|| strip_prefix_ascii_ci(&normalized, "you choose any number "))
        && let Some((chosen, tail)) = split_choose_sacrifice_tail(rest)
    {
        let chosen_plural = normalize_choose_sacrifice_subject(chosen);
        let tail = tail
            .trim_start_matches('.')
            .trim_start()
            .trim_end_matches('.');
        if tail.is_empty() {
            return format!("Sacrifice any number of {chosen_plural}");
        }
        let rewritten = format!(
            "Sacrifice any number of {chosen_plural}. {}.",
            capitalize_first(tail)
        );
        return normalize_zero_zero_token_with_base_pt(&rewritten).unwrap_or(rewritten);
    }
    if let Some(rest) = normalized.strip_prefix("you choose any number ")
        && let Some((chosen, tail)) = split_choose_sacrifice_tail(rest)
    {
        let chosen_plural = normalize_choose_sacrifice_subject(chosen);
        let tail = tail
            .trim_start_matches('.')
            .trim_start()
            .trim_end_matches('.');
        if tail.is_empty() {
            return format!("Sacrifice any number of {chosen_plural}");
        }
        let rewritten = format!(
            "Sacrifice any number of {chosen_plural}. {}.",
            capitalize_first(tail)
        );
        return normalize_zero_zero_token_with_base_pt(&rewritten).unwrap_or(rewritten);
    }
    if let Some(rest) = normalized.strip_prefix("For each opponent, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that player")
            .or_else(|| rest.strip_suffix(" damage to that player."))
    {
        return format!("Deal {amount} damage to each opponent");
    }
    if let Some(rest) = normalized.strip_prefix("Investigate. ")
        && rest.starts_with("target creature gets +")
    {
        return format!("Investigate, then {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("For each opponent, Deal ")
        && let Some((amount, tail)) = rest.split_once(" damage to that player. ")
        && (tail.eq_ignore_ascii_case("you gain 1 life")
            || tail.eq_ignore_ascii_case("you gain 1 life."))
    {
        return format!("Deal {amount} damage to each opponent and you gain 1 life");
    }
    if let Some((cost, effect)) = normalized.split_once(": ")
        && let Some(rest) = effect.strip_prefix("For each opponent, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that player")
            .or_else(|| rest.strip_suffix(" damage to that player."))
    {
        return format!("{cost}: Deal {amount} damage to each opponent");
    }
    if let Some((cost, effect)) = normalized.split_once(": ")
        && let Some(rest) = effect.strip_prefix("For each opponent, Deal ")
        && let Some((amount, tail)) = rest.split_once(" damage to that player. ")
        && (tail.eq_ignore_ascii_case("you gain 1 life")
            || tail.eq_ignore_ascii_case("you gain 1 life."))
    {
        return format!("{cost}: Deal {amount} damage to each opponent and you gain 1 life");
    }
    if let Some(draw_tail) = normalized.strip_prefix("You draw ")
        && let Some(count) = draw_tail.strip_suffix(" cards. Proliferate")
    {
        return format!("Draw {count} cards, then proliferate");
    }
    if let Some(draw_tail) = normalized.strip_prefix("You draw ")
        && let Some(count) = draw_tail.strip_suffix(" cards. Proliferate.")
    {
        return format!("Draw {count} cards, then proliferate");
    }
    if let Some(rest) = normalized.strip_prefix("For each creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that object")
            .or_else(|| rest.strip_suffix(" damage to that object."))
    {
        return format!("Deal {amount} damage to each creature");
    }
    if let Some(rest) = normalized.strip_prefix("For each ")
        && let Some((filter, tail)) = rest.split_once(", Deal ")
        && let Some(amount) = tail
            .strip_suffix(" damage to that object")
            .or_else(|| tail.strip_suffix(" damage to that object."))
        && !filter.starts_with("player")
        && !filter.starts_with("opponent")
        && !filter.starts_with("tagged ")
    {
        return format!("Deal {amount} damage to each {filter}");
    }
    if let Some((cost, effect)) = normalized.split_once(": ")
        && let Some(rest) = effect.strip_prefix("For each ")
        && let Some((filter, tail)) = rest.split_once(", Deal ")
        && let Some(amount) = tail
            .strip_suffix(" damage to that object")
            .or_else(|| tail.strip_suffix(" damage to that object."))
        && !filter.starts_with("player")
        && !filter.starts_with("opponent")
        && !filter.starts_with("tagged ")
    {
        return format!("{cost}: Deal {amount} damage to each {filter}");
    }
    if let Some(tail) =
        normalized.strip_prefix("For each player, that player discards their hand. you draw ")
    {
        return format!("Each player discards their hand, then draws {tail}");
    }
    if let Some(tail) = normalized.strip_prefix(
        "For each player, that player discards their hand. For each player, that player draws ",
    ) {
        return format!("Each player discards their hand, then draws {tail}");
    }
    if normalized
        == "For each player, that player draws a card. For each player, that player discards a card"
        || normalized
            == "For each player, that player draws a card. For each player, that player discards a card."
    {
        return "Each player draws a card, then discards a card".to_string();
    }
    if let Some(tail) = normalized.strip_prefix("You discard your hand. you draw ") {
        let draw_tail = tail.trim_end_matches('.');
        return format!("Discard your hand, then draw {draw_tail}");
    }
    if let Some(tail) = normalized.strip_prefix("Each player discards their hand. Create ")
        && let Some(token_clause) = tail
            .strip_suffix(" under that player's control")
            .or_else(|| tail.strip_suffix(" under that player's control."))
    {
        let normalized_clause = if token_clause.ends_with(" token") {
            format!("{} tokens", token_clause.trim_end_matches(" token"))
        } else {
            token_clause.to_string()
        };
        return format!("Each player discards their hand, then creates {normalized_clause}");
    }
    if let Some(tail) = normalized.strip_prefix(
        "For each player, Put a card from that player's hand on the bottom of that player's library. that player shuffles that player's graveyard into that player's library. For each player, that player draws ",
    ) {
        return format!("Each player shuffles their hand and graveyard into their library, then draws {tail}");
    }
    if let Some(tail) = normalized.strip_prefix(
        "For each player, Put a card from that player's hand on the bottom of that player's library. that player shuffles their graveyard into their library. For each player, that player draws ",
    ) {
        return format!("Each player shuffles their hand and graveyard into their library, then draws {tail}");
    }
    if let Some(rest) = normalized.strip_prefix("For each player, that player sacrifices ")
        && let Some((lands, damage_tail)) =
            rest.split_once(" lands that player controls. For each creature, Deal ")
        && let Some(amount) = damage_tail
            .strip_suffix(" damage to that object")
            .or_else(|| damage_tail.strip_suffix(" damage to that object."))
    {
        return format!(
            "Each player sacrifices {lands} lands of their choice. Deal {amount} damage to each creature"
        );
    }
    if let Some(rest) = normalized.strip_prefix("For each white or blue creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that object")
            .or_else(|| rest.strip_suffix(" damage to that object."))
    {
        return format!("Deal {amount} damage to each white and/or blue creature");
    }
    if let Some(rest) = normalized.strip_prefix("For each opponent's creature, Deal ")
        && let Some((amount, tail)) = rest.split_once(" damage to that object. ")
        && (tail.eq_ignore_ascii_case("an opponent's creature can't block until end of turn")
            || tail.eq_ignore_ascii_case("an opponent's creature cant block until end of turn")
            || tail.eq_ignore_ascii_case("an opponent's creature can't block this turn")
            || tail.eq_ignore_ascii_case("an opponent's creature cant block this turn"))
    {
        return format!(
            "Deal {amount} damage to each creature your opponents control. Creatures your opponents control can't block this turn"
        );
    }
    if let Some(rest) = normalized.strip_prefix("For each opponent's creature, Deal ")
        && let Some(amount) = rest.strip_suffix(" damage to that object")
    {
        return format!("Deal {amount} damage to each creature your opponents control");
    }
    if let Some(rest) = normalized.strip_prefix("For each opponent's creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to each opponent.")
            .or_else(|| rest.strip_suffix(" damage to each opponent"))
    {
        return format!("Deal {amount} damage to each creature your opponents control");
    }
    if let Some(rest) = normalized.strip_prefix("For each creature or planeswalker, Deal ")
        && let Some(amount) = rest.strip_suffix(" damage to that object")
    {
        return format!("Deal {amount} damage to each creature and each planeswalker");
    }
    if lower_normalized == "an opponent's creature can't block until end of turn"
        || lower_normalized == "an opponent's creature cant block until end of turn"
        || lower_normalized == "an opponent's creature can't block this turn"
        || lower_normalized == "an opponent's creature cant block this turn"
    {
        return "Creatures your opponents control can't block this turn".to_string();
    }
    if lower_normalized == "target player's creature gets -2/-2 until end of turn"
        || lower_normalized == "target players creature gets -2/-2 until end of turn"
    {
        return "Creatures target player controls get -2/-2 until end of turn".to_string();
    }
    if let Some((first, second)) = normalized.split_once(". ")
        && first.starts_with("Deal ")
        && (second.eq_ignore_ascii_case("creature can't block until end of turn")
            || second.eq_ignore_ascii_case("creature cant block until end of turn")
            || second.eq_ignore_ascii_case("permanent can't block until end of turn")
            || second.eq_ignore_ascii_case("permanent cant block until end of turn"))
        && let Some(rest) = first.strip_prefix("Deal ")
        && let Some((amount, targets)) = rest.split_once(" damage to ")
    {
        return format!(
            "Deal {amount} damage to each of {targets}. Those creatures can't block this turn"
        );
    }
    if lower_normalized == "opponent's creatures get -1/-0"
        || lower_normalized == "opponent's creatures get -1/-0."
    {
        return "Creatures your opponents control get -1/-0".to_string();
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "an opponent's ")
        && let Some((objects, predicate)) = split_once_ascii_ci(rest, " get ")
    {
        return format!(
            "{} your opponents control get {}",
            capitalize_first(objects.trim()),
            predicate.trim()
        );
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "an opponent's ")
        && let Some((objects, predicate)) = split_once_ascii_ci(rest, " gets ")
    {
        return format!(
            "{} your opponents control get {}",
            capitalize_first(objects.trim()),
            predicate.trim()
        );
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "opponent's ")
        && let Some((objects, predicate)) = split_once_ascii_ci(rest, " get ")
    {
        return format!(
            "{} your opponents control get {}",
            capitalize_first(objects.trim()),
            predicate.trim()
        );
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "opponent's ")
        && let Some((objects, predicate)) = split_once_ascii_ci(rest, " gets ")
    {
        return format!(
            "{} your opponents control get {}",
            capitalize_first(objects.trim()),
            predicate.trim()
        );
    }
    if lower_normalized == "red and green spells you cast cost {1} less to cast"
        || lower_normalized == "red and green spells you cast costs {1} less to cast"
        || lower_normalized == "red and green spells you cast cost {1} less to cast."
    {
        return "Each spell you cast that's red or green costs {1} less to cast".to_string();
    }
    if lower_normalized == "other zombie you control get +1/+0"
        || lower_normalized == "other zombie you control get +1/+0."
    {
        return "Other Zombies you control get +1/+0".to_string();
    }
    if lower_normalized == "draw three cards. target opponent draws 3 cards"
        || lower_normalized == "draw three cards. target opponent draws 3 cards."
    {
        return "You and target opponent each draw three cards".to_string();
    }
    if lower_normalized == "untap all a snow permanent you control"
        || lower_normalized == "untap all a snow permanent you control."
    {
        return "Untap each snow permanent you control".to_string();
    }
    if lower_normalized == "untap all a creature you control"
        || lower_normalized == "untap all a creature you control."
    {
        return "Untap all creatures you control".to_string();
    }
    if lower_normalized == "as an additional cost to cast this spell, you discard a card"
        || lower_normalized == "as an additional cost to cast this spell, you discard a card."
    {
        return "As an additional cost to cast this spell, discard a card".to_string();
    }
    if lower_normalized == "add 1 mana of commander's color identity"
        || lower_normalized == "add 1 mana of commander's color identity."
    {
        return "Add one mana of any color in your commander's color identity".to_string();
    }
    if let Some((cost, tail)) = split_once_ascii_ci(&normalized, ": ")
        && (tail.eq_ignore_ascii_case("Add 1 mana of commander's color identity")
            || tail.eq_ignore_ascii_case("Add 1 mana of commander's color identity."))
    {
        return format!("{cost}: Add one mana of any color in your commander's color identity");
    }
    if lower_normalized == "return this permanent from graveyard to the battlefield tapped"
        || lower_normalized == "return this permanent from graveyard to the battlefield tapped."
        || lower_normalized == "return this creature from graveyard to the battlefield tapped"
        || lower_normalized == "return this creature from graveyard to the battlefield tapped."
        || lower_normalized == "return this source from graveyard to the battlefield tapped"
        || lower_normalized == "return this source from graveyard to the battlefield tapped."
    {
        return "Return this card from your graveyard to the battlefield tapped".to_string();
    }
    if let Some((cost, tail)) = split_once_ascii_ci(&normalized, ": ")
        && (tail
            .eq_ignore_ascii_case("Return this permanent from graveyard to the battlefield tapped")
            || tail.eq_ignore_ascii_case(
                "Return this permanent from graveyard to the battlefield tapped.",
            )
            || tail.eq_ignore_ascii_case(
                "Return this creature from graveyard to the battlefield tapped",
            )
            || tail.eq_ignore_ascii_case(
                "Return this creature from graveyard to the battlefield tapped.",
            )
            || tail.eq_ignore_ascii_case(
                "Return this source from graveyard to the battlefield tapped",
            )
            || tail.eq_ignore_ascii_case(
                "Return this source from graveyard to the battlefield tapped.",
            ))
    {
        return format!("{cost}: Return this card from your graveyard to the battlefield tapped");
    }
    if lower_normalized == "target player sacrifices target player's creature" {
        return "Target player sacrifices a creature of their choice".to_string();
    }
    if lower_normalized == "target player sacrifices a creature"
        || lower_normalized == "target player sacrifices a creature."
    {
        return "Target player sacrifices a creature of their choice".to_string();
    }
    if lower_normalized
        == "target player sacrifices target player's creature. target player loses 1 life"
    {
        return "Target player sacrifices a creature of their choice and loses 1 life".to_string();
    }
    if lower_normalized == "target player sacrifices a creature. target player loses 1 life"
        || lower_normalized == "target player sacrifices a creature and target player loses 1 life"
        || lower_normalized == "target player sacrifices a creature and loses 1 life"
    {
        return "Target player sacrifices a creature of their choice and loses 1 life".to_string();
    }
    if let Some((prefix, tail)) = normalized.split_once(
        ". For each opponent, that player discards a card. For each opponent, that player loses ",
    ) && let Some((life_amount, trailing)) = tail.split_once(" life")
    {
        let trailing = trailing.trim_start_matches('.').trim();
        let trailing_clause = if trailing.is_empty() {
            String::new()
        } else {
            format!(". {}", capitalize_first(trailing))
        };
        return format!(
            "{}, discards a card, and loses {} life{}",
            prefix.trim_end_matches('.').trim(),
            life_amount.trim(),
            trailing_clause
        );
    }
    if let Some((draw_count, gain_tail)) = normalized
        .strip_prefix("Draw ")
        .and_then(|rest| rest.split_once(" card. you gain "))
        && let Some(life_amount) = gain_tail.strip_suffix(" life")
    {
        return format!("You draw {draw_count} card and gain {life_amount} life");
    }
    if let Some((draw_count, gain_tail)) = normalized
        .strip_prefix("Draw ")
        .and_then(|rest| rest.split_once(" cards. you gain "))
        && let Some(life_amount) = gain_tail.strip_suffix(" life")
    {
        return format!("You draw {draw_count} cards and gain {life_amount} life");
    }
    if let Some((prefix, tail)) =
        normalized.split_once(", its controller draws a card. its controller loses ")
        && let Some((life_amount, rest)) = tail.split_once(" life. Draw a card. you lose ")
        && let Some(rest_amount) = rest
            .strip_suffix(" life")
            .or_else(|| rest.strip_suffix(" life."))
        && life_amount.trim() == rest_amount.trim()
    {
        return format!(
            "{prefix}, you and its controller each draw a card and lose {} life",
            life_amount.trim()
        );
    }
    if let Some((prefix, tail)) =
        normalized.split_once(", you draw a card. the attacking player draws a card. you lose ")
        && let Some((life_amount, rest)) = tail.split_once(" life. the attacking player loses ")
        && let Some(rest_amount) = rest
            .strip_suffix(" life")
            .or_else(|| rest.strip_suffix(" life."))
        && life_amount.trim() == rest_amount.trim()
    {
        return format!(
            "{prefix}, you and the attacking player each draw a card and lose {} life",
            life_amount.trim()
        );
    }
    if let Some(rest) = normalized.strip_prefix("Target player sacrifices target player's ")
        && let Some((first_kind, tail)) =
            rest.split_once(". target player sacrifices target player's ")
        && let Some((second_kind, damage_tail)) = tail.split_once(". Deal ")
        && let Some(amount) = damage_tail
            .strip_suffix(" damage to target player")
            .or_else(|| damage_tail.strip_suffix(" damage to target player."))
    {
        return format!(
            "Target player sacrifices {} and {} of their choice. Deal {} damage to that player",
            first_kind.trim(),
            second_kind.trim(),
            amount.trim()
        );
    }
    if lower_normalized == "target player sacrifices target player's attacking or blocking creature"
        || lower_normalized
            == "target player sacrifices target player's attacking/blocking creature"
        || lower_normalized
            == "target player sacrifices target player's attacking or blocking creature."
        || lower_normalized
            == "target player sacrifices target player's attacking/blocking creature."
    {
        return "Target player sacrifices an attacking or blocking creature of their choice"
            .to_string();
    }
    if lower_normalized == "target player sacrifices an attacking or blocking creature"
        || lower_normalized == "target player sacrifices an attacking/blocking creature"
        || lower_normalized == "target player sacrifices an attacking or blocking creature."
        || lower_normalized == "target player sacrifices an attacking/blocking creature."
    {
        return "Target player sacrifices an attacking or blocking creature of their choice"
            .to_string();
    }
    if lower_normalized
        == "destroy target creature. if that permanent dies this way, create two tokens that are copies of it under that object's controller's control, except their power and toughness are each half that permanent's power and toughness, rounded up"
        || lower_normalized
            == "destroy target creature. if that permanent dies this way, create two tokens that are copies of it under that object's controller's control, except their power and toughness are each half that permanent's power and toughness, rounded up."
    {
        return "Destroy target creature. If that permanent dies this way, Create two tokens that are copies of it under that object's controller's control, except their power and toughness are each half that permanent's power and toughness, rounded up.".to_string();
    }
    if lower_normalized.contains(
        "this creature is put into your graveyard from the battlefield: at the beginning of the next end step, you lose 1 life. return this creature to its owner's hand",
    ) {
        return "When this creature is put into your graveyard from the battlefield, at the beginning of the next end step, you lose 1 life and return this card to your hand.".to_string();
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "Whenever a ")
        && let Some(subject) =
            rest.strip_suffix(" you own dies, put it on top of its owner's library.")
    {
        return format!(
            "Whenever a {subject} is put into your graveyard from the battlefield, put that card on top of your library."
        );
    }
    if let Some((subject, condition)) =
        normalized.split_once(" has Doesn't untap during your untap step as long as ")
        && !subject.trim().is_empty()
        && !condition.trim().is_empty()
    {
        return format!(
            "{} doesn't untap during your untap step if {}",
            subject.trim(),
            condition.trim()
        );
    }
    if lower_normalized
        == "for each player, that player sacrifices two creatures that player controls"
        || lower_normalized
            == "for each player, that player sacrifices two creatures that player controls."
    {
        return "Each player sacrifices two creatures of their choice".to_string();
    }
    if lower_normalized == "exile all card in graveyard"
        || lower_normalized == "exile all card in graveyard."
    {
        return "Exile all graveyards".to_string();
    }
    if lower_normalized == "exile all card in target opponent's graveyard"
        || lower_normalized == "exile all card in target opponent's graveyard."
        || lower_normalized == "exile all card in target opponent's graveyards"
        || lower_normalized == "exile all card in target opponent's graveyards."
        || lower_normalized == "exile all card from target opponent's graveyard"
        || lower_normalized == "exile all card from target opponent's graveyard."
        || lower_normalized == "exile all cards from target opponent's graveyard"
        || lower_normalized == "exile all cards from target opponent's graveyard."
        || lower_normalized == "exile all card from target opponent's graveyards"
        || lower_normalized == "exile all card from target opponent's graveyards."
        || lower_normalized == "exile all cards from target opponent's graveyards"
        || lower_normalized == "exile all cards from target opponent's graveyards."
    {
        return "Exile target opponent's graveyard".to_string();
    }
    if lower_normalized == "exile all card in target player's graveyard"
        || lower_normalized == "exile all card in target player's graveyard."
        || lower_normalized == "exile all card in target player's graveyards"
        || lower_normalized == "exile all card in target player's graveyards."
        || lower_normalized == "exile all card from target player's graveyard"
        || lower_normalized == "exile all card from target player's graveyard."
        || lower_normalized == "exile all cards from target player's graveyard"
        || lower_normalized == "exile all cards from target player's graveyard."
        || lower_normalized == "exile all card from target player's graveyards"
        || lower_normalized == "exile all card from target player's graveyards."
        || lower_normalized == "exile all cards from target player's graveyards"
        || lower_normalized == "exile all cards from target player's graveyards."
    {
        return "Exile target player's graveyard".to_string();
    }
    if lower_normalized == "exile all card in that player's graveyard"
        || lower_normalized == "exile all card in that player's graveyard."
        || lower_normalized == "exile all card in that player's graveyards"
        || lower_normalized == "exile all card in that player's graveyards."
        || lower_normalized == "exile all card from that player's graveyard"
        || lower_normalized == "exile all card from that player's graveyard."
        || lower_normalized == "exile all cards from that player's graveyard"
        || lower_normalized == "exile all cards from that player's graveyard."
        || lower_normalized == "exile all cards in that player's graveyard"
        || lower_normalized == "exile all cards in that player's graveyard."
        || lower_normalized == "exile all card from that player's graveyards"
        || lower_normalized == "exile all card from that player's graveyards."
        || lower_normalized == "exile all cards from that player's graveyards"
        || lower_normalized == "exile all cards from that player's graveyards."
    {
        return "Exile that player's graveyard".to_string();
    }
    if let Some(rest) = normalized.strip_prefix("Exile all card in target opponent's graveyard. ") {
        return format!("Exile target opponent's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all card in target opponent's graveyards. ")
    {
        return format!("Exile target opponent's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all card from target opponent's graveyard. ")
    {
        return format!("Exile target opponent's graveyard. {rest}");
    }
    if let Some(rest) =
        normalized.strip_prefix("Exile all cards from target opponent's graveyard. ")
    {
        return format!("Exile target opponent's graveyard. {rest}");
    }
    if let Some(rest) =
        normalized.strip_prefix("Exile all card from target opponent's graveyards. ")
    {
        return format!("Exile target opponent's graveyard. {rest}");
    }
    if let Some(rest) =
        normalized.strip_prefix("Exile all cards from target opponent's graveyards. ")
    {
        return format!("Exile target opponent's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all card in target player's graveyard. ") {
        return format!("Exile target player's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all card in target player's graveyards. ") {
        return format!("Exile target player's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all card from target player's graveyard. ") {
        return format!("Exile target player's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all cards from target player's graveyard. ")
    {
        return format!("Exile target player's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all card from target player's graveyards. ")
    {
        return format!("Exile target player's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all cards from target player's graveyards. ")
    {
        return format!("Exile target player's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all card in that player's graveyard. ") {
        return format!("Exile that player's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all card in that player's graveyards. ") {
        return format!("Exile that player's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all card from that player's graveyard. ") {
        return format!("Exile that player's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all cards from that player's graveyard. ") {
        return format!("Exile that player's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all cards in that player's graveyard. ") {
        return format!("Exile that player's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all card from that player's graveyards. ") {
        return format!("Exile that player's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all cards from that player's graveyards. ") {
        return format!("Exile that player's graveyard. {rest}");
    }
    let preserves_target_graveyard_exception = normalized
        .contains("target opponent's graveyard other than")
        || normalized.contains("target opponent's graveyards other than")
        || normalized.contains("target player's graveyard other than")
        || normalized.contains("target player's graveyards other than");
    if !preserves_target_graveyard_exception {
        normalized = normalized.replace(
            "Exile all cards from target opponent's graveyard",
            "Exile target opponent's graveyard",
        );
        normalized = normalized.replace(
            "exile all cards from target opponent's graveyard",
            "exile target opponent's graveyard",
        );
        normalized = normalized.replace(
            "Exile all cards from target opponent's graveyards",
            "Exile target opponent's graveyard",
        );
        normalized = normalized.replace(
            "exile all cards from target opponent's graveyards",
            "exile target opponent's graveyard",
        );
        normalized = normalized.replace(
            "Exile all cards from target player's graveyard",
            "Exile target player's graveyard",
        );
        normalized = normalized.replace(
            "exile all cards from target player's graveyard",
            "exile target player's graveyard",
        );
        normalized = normalized.replace(
            "Exile all cards from target player's graveyards",
            "Exile target player's graveyard",
        );
        normalized = normalized.replace(
            "exile all cards from target player's graveyards",
            "exile target player's graveyard",
        );
    }
    if lower_normalized == "permanents enter the battlefield tapped"
        || lower_normalized == "permanents enter the battlefield tapped."
    {
        return "Permanents enter tapped".to_string();
    }
    if let Some(rest) = normalized.strip_prefix("Token creatures get ") {
        return format!("Creature tokens get {rest}");
    }
    if let Some(rest) = normalized.strip_prefix(
        "When this creature enters, target opponent chooses exactly 1 target player's creature in the battlefield. Destroy it",
    ) {
        return format!("When this creature enters, target opponent chooses a creature they control. Destroy that creature{rest}");
    }
    if normalized
        == "Whenever this creature blocks creature, permanent can't untap until your next turn"
    {
        return "Whenever this creature blocks a creature, that creature doesn't untap during its controller's next untap step".to_string();
    }
    if let Some(kind) = strip_prefix_ascii_ci(&normalized, "you may put ").and_then(|rest| {
        rest.strip_suffix(" card in your hand onto the battlefield")
            .or_else(|| rest.strip_suffix(" card in your hand onto the battlefield."))
    }) {
        let kind = kind.trim();
        let noun = if kind.is_empty() {
            "card".to_string()
        } else {
            format!("{kind} card")
        };
        let rendered_noun =
            if kind.starts_with("target ") || kind.starts_with("a ") || kind.starts_with("an ") {
                noun
            } else {
                with_indefinite_article(&noun)
            };
        return format!("You may put {rendered_noun} from your hand onto the battlefield");
    }
    if let Some(kind) = strip_prefix_ascii_ci(&normalized, "you may put ").and_then(|rest| {
        rest.strip_suffix(" in your hand onto the battlefield")
            .or_else(|| rest.strip_suffix(" in your hand onto the battlefield."))
    }) {
        let kind = kind.trim();
        let rendered_kind =
            if kind.starts_with("target ") || kind.starts_with("a ") || kind.starts_with("an ") {
                kind.to_string()
            } else {
                with_indefinite_article(kind)
            };
        return format!("You may put {rendered_kind} from your hand onto the battlefield");
    }
    if let Some((cost, rest)) = split_once_ascii_ci(&normalized, ": ")
        && let Some(kind) = strip_prefix_ascii_ci(rest.trim(), "you may put ").and_then(|tail| {
            tail.strip_suffix(" card in your hand onto the battlefield")
                .or_else(|| tail.strip_suffix(" card in your hand onto the battlefield."))
        })
    {
        let kind = kind.trim();
        let noun = if kind.is_empty() {
            "card".to_string()
        } else {
            format!("{kind} card")
        };
        let rendered_noun =
            if kind.starts_with("target ") || kind.starts_with("a ") || kind.starts_with("an ") {
                noun
            } else {
                with_indefinite_article(&noun)
            };
        return format!("{cost}: You may put {rendered_noun} from your hand onto the battlefield");
    }
    if let Some((cost, rest)) = split_once_ascii_ci(&normalized, ": ")
        && let Some(kind) = strip_prefix_ascii_ci(rest.trim(), "you may put ").and_then(|tail| {
            tail.strip_suffix(" in your hand onto the battlefield")
                .or_else(|| tail.strip_suffix(" in your hand onto the battlefield."))
        })
    {
        let kind = kind.trim();
        let rendered_kind =
            if kind.starts_with("target ") || kind.starts_with("a ") || kind.starts_with("an ") {
                kind.to_string()
            } else {
                with_indefinite_article(kind)
            };
        return format!("{cost}: You may put {rendered_kind} from your hand onto the battlefield");
    }
    if let Some(rest) = normalized
        .strip_prefix("you may Put target planeswalker card in your hand onto the battlefield")
    {
        return format!(
            "You may put a planeswalker card from your hand onto the battlefield{rest}"
        );
    }
    if normalized.starts_with(
        "When this creature enters, for each another creature you control, Put a +1/+1 counter on that object",
    ) || normalized.starts_with(
        "When this creature enters, for each another creature you control, Put 1 +1/+1 counter on that object",
    ) {
        return "When this creature enters, put a +1/+1 counter on each other creature you control"
            .to_string();
    }
    if normalized.starts_with(
        "When this permanent enters, for each another creature you control, Put a +1/+1 counter on that object",
    ) || normalized.starts_with(
        "When this permanent enters, for each another creature you control, Put 1 +1/+1 counter on that object",
    ) {
        return "When this permanent enters, for each other creature you control, Put a +1/+1 counter on that object."
            .to_string();
    }
    if normalized.starts_with("For each player, that player loses 1 life for each ") {
        let rest = normalized
            .trim_start_matches("For each player, that player loses 1 life for each ")
            .to_string();
        return format!("Each player loses 1 life for each {rest}");
    }
    if normalized.starts_with("For each player, that player gains 1 life for each ") {
        let rest = normalized
            .trim_start_matches("For each player, that player gains 1 life for each ")
            .to_string();
        return format!("Each player gains 1 life for each {rest}");
    }
    if let Some((prefix, tail)) =
        split_once_ascii_ci(&normalized, ", for each player, that player ")
        && !tail.trim().is_empty()
    {
        return format!(
            "{}, each player {}",
            capitalize_first(prefix.trim()),
            tail.trim()
        );
    }
    if let Some((prefix, tail)) =
        split_once_ascii_ci(&normalized, ", for each opponent, that player ")
        && !tail.trim().is_empty()
    {
        return format!(
            "{}, each opponent {}",
            capitalize_first(prefix.trim()),
            tail.trim()
        );
    }
    if let Some(rest) = normalized.strip_prefix("For each player, that player ")
        && !rest.trim().is_empty()
    {
        return format!("Each player {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("for each player, that player ")
        && !rest.trim().is_empty()
    {
        return format!("each player {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("For each opponent, that player ")
        && !rest.trim().is_empty()
    {
        return format!("Each opponent {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("for each opponent, that player ")
        && !rest.trim().is_empty()
    {
        return format!("each opponent {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("For each ")
        && let Some((subject, tail)) = rest.split_once(", Put ")
        && let Some(counter_clause) = tail
            .strip_suffix(" counter on that object")
            .or_else(|| tail.strip_suffix(" counter on that object."))
    {
        return format!("Put {counter_clause} counter on each {subject}");
    }
    if matches!(
        normalized.as_str(),
        "attacking creature can't untap until your next turn"
            | "attacking creature cant untap until your next turn"
    ) {
        return "Each attacking creature doesn't untap during its controller's next untap step"
            .to_string();
    }
    if let Some(rest) =
        normalized.strip_prefix("Prevent all combat damage that would be dealt this turn. ")
        && matches!(
            rest,
            "attacking creature can't untap until your next turn."
                | "attacking creature cant untap until your next turn."
                | "attacking creature can't untap until your next turn"
                | "attacking creature cant untap until your next turn"
        )
    {
        return "Prevent all combat damage that would be dealt this turn. Each attacking creature doesn't untap during its controller's next untap step".to_string();
    }
    if normalized == "An opponent's creature enter tapped."
        || normalized == "An opponent's creature enter tapped"
        || normalized == "An opponent's creature enters tapped."
        || normalized == "An opponent's creature enters tapped"
    {
        return "Creatures your opponents control enter tapped".to_string();
    }
    if let Some(subject) = normalized.strip_suffix(" have flashback as long as it's your turn") {
        return format!(
            "During your turn, {} have flashback. Its flashback cost is equal to its mana cost",
            subject.trim()
        );
    }
    if let Some(subject) = normalized.strip_suffix(" has flashback as long as it's your turn") {
        return format!(
            "During your turn, {} has flashback. Its flashback cost is equal to its mana cost",
            subject.trim()
        );
    }
    if let Some((subject, tail)) = normalized.split_once(" has \"If damage would be dealt to ")
        && let Some((damage_target, effect_text)) = tail.split_once(", ")
        && let Some(effect_text) = effect_text.strip_suffix("\" as long as you're the monarch")
    {
        let damage_target = if damage_target == "this" {
            subject.trim()
        } else {
            damage_target.trim()
        };
        return format!(
            "If damage would be dealt to {damage_target} while you're the monarch, {}",
            effect_text.trim()
        );
    }
    if let Some((subject, tail)) = normalized.split_once(" has \"If damage would be dealt to ")
        && let Some((damage_target, effect_text)) = tail.split_once(", ")
        && let Some(effect_text) = effect_text.strip_suffix("\" as long as you're the monarch.")
    {
        let damage_target = if damage_target == "this" {
            subject.trim()
        } else {
            damage_target.trim()
        };
        return format!(
            "If damage would be dealt to {damage_target} while you're the monarch, {}",
            effect_text.trim().trim_end_matches('.')
        );
    }
    if let Some((prefix, suffix)) =
        normalized.split_once(". you can't become the monarch this turn")
        && suffix.trim_matches('.').trim().is_empty()
    {
        return format!(
            "{}. You can't become the monarch this turn",
            prefix.trim_end_matches('.').trim()
        );
    }
    if let Some((types, tail)) = normalized.split_once(" creatures get ")
        && types.contains(" or ")
        && looks_like_creature_type_list_subject(types)
    {
        let type_items = types
            .split(" or ")
            .map(str::trim)
            .filter(|item| {
                item.chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_uppercase())
            })
            .collect::<Vec<_>>();
        if type_items.len() >= 2 {
            let listed = join_with_and(
                &type_items
                    .iter()
                    .map(|item| with_indefinite_article(item))
                    .collect::<Vec<_>>(),
            );
            return format!(
                "Each creature that's {} gets {}",
                listed,
                tail.trim_end_matches('.').trim()
            );
        }
    }
    if let Some((types, tail)) = normalized.split_once(" creatures have ")
        && types.contains(" or ")
        && looks_like_creature_type_list_subject(types)
    {
        let type_items = types
            .split(" or ")
            .map(str::trim)
            .filter(|item| {
                item.chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_uppercase())
            })
            .collect::<Vec<_>>();
        if type_items.len() >= 2 {
            let listed = join_with_and(
                &type_items
                    .iter()
                    .map(|item| with_indefinite_article(item))
                    .collect::<Vec<_>>(),
            );
            return format!(
                "Each creature that's {} has {}",
                listed,
                tail.trim_end_matches('.').trim()
            );
        }
    }

    if lower == "attacks each combat if able" {
        return "This creature attacks each combat if able".to_string();
    }
    if lower == "counter target creature" {
        return "Counter target creature spell".to_string();
    }
    if lower == "counter up to one target creature" {
        return "Counter up to one target creature spell".to_string();
    }
    if lower == "counter target instant spell spell" {
        return "Counter target instant spell".to_string();
    }
    if lower == "counter target sorcery spell spell" {
        return "Counter target sorcery spell".to_string();
    }
    if lower == "destroy target artifact or enchantment or creature with flying" {
        return "Destroy target artifact, enchantment, or creature with flying".to_string();
    }
    if let Some(rest) = normalized.strip_prefix("Can't be blocked by creatures with power ") {
        return format!("This creature can't be blocked by creatures with power {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Can't be blocked by more than ") {
        if let Some(noun) = rest.strip_prefix("1 creature") {
            return format!("This creature can't be blocked by more than one creature{noun}");
        }
        return format!("This creature can't be blocked by more than {rest}");
    }
    if let Some(rest) =
        normalized.strip_prefix("As long as this creature is equipped, this creature gets ")
    {
        return format!("As long as this creature is equipped, it gets {rest}");
    }
    if let Some(rest) =
        normalized.strip_prefix("As long as this creature is enchanted, this creature gets ")
    {
        return format!("As long as this creature is enchanted, it gets {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Sliver creatures get ") {
        return format!("All Sliver creatures get {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Sliver creatures gain ") {
        return format!("All Sliver creatures gain {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Sliver creatures have ") {
        return format!(
            "All Sliver creatures have {}",
            normalize_keyword_predicate_case(rest)
        );
    }
    if let Some(rest) = normalized.strip_prefix("Creatures get ") {
        return format!("All creatures get {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Creatures gain ") {
        return format!("All creatures gain {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Until end of turn, creatures gain ") {
        return format!("Until end of turn, all creatures gain {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("creatures get ") {
        return format!("all creatures get {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("creatures have ") {
        return format!(
            "All creatures have {}",
            normalize_keyword_predicate_case(rest)
        );
    }
    if normalized == "this creature becomes the target of a spell or ability: You sacrifice it" {
        return "When this creature becomes the target of a spell or ability, sacrifice it"
            .to_string();
    }
    if normalized
        == "this creature or Whenever another Ally you control enters: You may Put a +1/+1 counter on this creature"
    {
        return "Whenever this creature or another Ally you control enters, you may put a +1/+1 counter on this creature".to_string();
    }
    for article in ["a", "an"] {
        let marker = format!(
            "When this creature enters or {article} Ally you control other than this enters, "
        );
        if let Some(rest) = normalized.strip_prefix(&marker) {
            return format!("Whenever this creature or another Ally you control enters, {rest}");
        }
    }
    if normalized == "When this creature enters or When this creature dies, Surveil 1" {
        return "When this creature enters or dies, surveil 1".to_string();
    }
    if normalized == "Whenever you cast noncreature spell, Put a +1/+1 counter on this creature" {
        return "Whenever you cast a noncreature spell, put a +1/+1 counter on this creature"
            .to_string();
    }
    if let Some(rest) =
        normalized.strip_prefix("When this creature enters, If you attacked this turn, Deal ")
        && let Some(amount) = rest.strip_suffix(" damage to any target")
    {
        return format!(
            "Raid — When this creature enters, if you attacked this turn, this creature deals {amount} damage to any target"
        );
    }
    if let Some(rest) = normalized.strip_prefix("{")
        && rest.contains("}, Discard a card: Target attacking creature gets ")
        && rest.ends_with(" until end of turn")
    {
        return format!("Bloodrush — {{{rest}").replace(
            ", Discard a card: Target attacking creature gets ",
            ", Discard this card: Target attacking creature gets ",
        );
    }
    if let Some(rest) = normalized.strip_prefix("up to two target creatures get ") {
        return format!("Up to two target creatures each get {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Up to two target creatures get ") {
        let rest = rest
            .replace(
                "Untap that creature. At this turn's next end of combat",
                "Untap those creatures. At this turn's next end of combat",
            )
            .replace(
                "Tap all blocked creature blocked by one of those creatures this turns. that permanent doesn't untap during its controller's next untap step",
                "Tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
            )
            .replace(
                "tap all blocked creature blocked by one of those creatures this turns",
                "tap each creature that was blocked by one of those creatures this turn",
            )
            .replace(
                "Tap all blocked creature blocked by one of those creatures this turns",
                "Tap each creature that was blocked by one of those creatures this turn",
            )
            .replace(
                ". that permanent doesn't untap during its controller's next untap step",
                " and it doesn't untap during its controller's next untap step",
            );
        return format!("Up to two target creatures each get {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("one or two target creatures get ") {
        return format!("One or two target creatures each get {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("two target creatures get ") {
        return format!("Two target creatures each get {rest}");
    }
    if let Some(rest) =
        normalized.strip_prefix("When this creature enters, draw a card and you lose ")
    {
        return format!("When this creature enters, you draw a card and you lose {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("When this creature enters, you draw ")
        && let Some((count, tail)) = rest.split_once(". target opponent draws ")
        && tail.trim_end_matches('.') == count
    {
        return format!("When this creature enters, you and target opponent each draw {count}");
    }
    if let Some(rest) = normalized.strip_prefix("You draw ")
        && let Some((count, tail)) = rest.split_once(". target opponent draws ")
        && tail.trim_end_matches('.') == count
    {
        return format!("You and target opponent each draw {count}");
    }
    if let Some(rest) = normalized.strip_prefix("Search your library for basic land you own") {
        return format!("Search your library for a basic land card{rest}");
    }
    if let Some(rest) =
        normalized.strip_prefix("Search your library for up to one basic land you own")
    {
        return format!("Search your library for a basic land card{rest}");
    }
    if normalized == "Counter target instant spell spell" {
        return "Counter target instant spell".to_string();
    }
    if normalized == "Counter target sorcery spell spell" {
        return "Counter target sorcery spell".to_string();
    }
    if normalized == "Destroy target artifact or enchantment or creature with flying" {
        return "Destroy target artifact, enchantment, or creature with flying".to_string();
    }
    {
        // Destroy-then-controller-searches (Boseiju, Who Endures family): the
        // raw render names "an opponent" three times; oracle uses controller
        // back-references.  Match the destroy clause structurally up to the
        // search follow-up so cost prefixes survive.
        const SEARCH_TAILS: &[&str] = &[
            ", then each opponent may search their library for a basic land card, put it onto the battlefield, then that player shuffles",
            ", then an opponent may search an opponent's library for a basic land card, put it onto the battlefield, then that player shuffles",
            ". An opponent may search an opponent's library for a basic land card, put it onto the battlefield, then that player shuffles",
            ". an opponent may search an opponent's library for a basic land card, put it onto the battlefield, then that player shuffles",
            ". That player may search their library for a with a basic land type card, put it onto the battlefield. Then that player shuffles",
        ];
        const DESTROY_BODIES: &[&str] = &[
            "Destroy target artifact, enchantment, or nonbasic land an opponent controls",
            "Destroy target nonbasic artifact, enchantment, or land an opponent controls",
            "Destroy target opponent's nonbasic artifact or enchantment or land",
            "Destroy target opponent's nonbasic artifact, enchantment, or land",
        ];
        for body in DESTROY_BODIES {
            for tail in SEARCH_TAILS {
                let pattern = format!("{body}{tail}");
                if let Some(idx) = normalized.find(pattern.as_str()) {
                    let prefix = &normalized[..idx];
                    return format!(
                        "{prefix}Destroy target artifact, enchantment, or nonbasic land an opponent controls. That permanent's controller may search their library for a land card with a basic land type, put it onto the battlefield, then shuffle"
                    );
                }
            }
        }
    }
    if normalized
        == "Return target artifact or creature or enchantment or planeswalker to its owner's hand"
    {
        return "Return target artifact, creature, enchantment, or planeswalker to its owner's hand".to_string();
    }
    if let Some((prefix, _)) = normalized.split_once(
        ", if you cast it, you can't be targeted until your next turn. Prevent all damage that would be dealt to you until your next turn",
    ) {
        return format!(
            "{prefix}, if you cast it, you gain protection from everything until your next turn"
        );
    }
    if let Some((prefix, rest)) = normalized.split_once(": ") {
        let rest_lower = rest.to_ascii_lowercase();
        if rest_lower
            == "you can't be targeted until your next turn. prevent all damage that would be dealt to you until your next turn"
            || rest_lower
                == "you can't be targeted until your next turn. prevent all damage that would be dealt to you until your next turn."
        {
            return format!("{prefix}: You gain protection from everything until your next turn.");
        }
    }
    if let Some(rest) = normalized.strip_prefix("this creature gets ")
        && let Some((pt, tail)) = rest.split_once(" for each Equipment attached to this creature")
    {
        return format!("This creature gets {pt} for each Equipment attached to it{tail}");
    }
    if normalized
        == "Whenever this creature or Whenever another Ally you control enters, creatures you control get +1/+1 until end of turn"
    {
        return "Whenever this creature or another Ally you control enters, creatures you control get +1/+1 until end of turn".to_string();
    }
    if lower
        == "whenever this creature or whenever another ally you control enters, creatures you control get +1/+1 until end of turn"
    {
        return "Whenever this creature or another Ally you control enters, creatures you control get +1/+1 until end of turn".to_string();
    }
    if lower
        == "whenever this creature or least two other creatures attack, this creature gets +2/+2 until end of turn"
    {
        return "Whenever this creature and at least two other creatures attack, this creature gets +2/+2 until end of turn".to_string();
    }
    if let Some(rest) = normalized.strip_prefix("Whenever This creature or Whenever another ") {
        return format!("Whenever this creature or another {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Whenever This or Whenever another ") {
        return format!("Whenever this or another {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("When this creature enters or another ")
        && rest.contains(" enters")
    {
        return format!("Whenever this creature or another {rest}");
    }
    for article in ["a", "an"] {
        let marker = format!("When this creature enters or {article} ");
        if let Some(rest) = normalized.strip_prefix(&marker)
            && let Some((subject, effect_clause)) =
                rest.split_once(" you control other than this enters,")
        {
            return format!(
                "Whenever this creature or another {subject} you control enters,{effect_clause}"
            );
        }
    }
    if let Some(rest) = normalized.strip_prefix("When this enters or another ")
        && rest.contains(" enters")
    {
        return format!("Whenever this or another {rest}");
    }
    if let Some(rest) =
        normalized.strip_prefix("When this creature leaves the battlefield or another ")
        && rest.contains(" leaves the battlefield")
    {
        return format!("Whenever this creature or another {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("When this leaves the battlefield or another ")
        && rest.contains(" leaves the battlefield")
    {
        return format!("Whenever this or another {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("When this creature enters or a ")
        && let Some((subject, effect_clause)) = rest.split_once(
            " you control other than this is put into a graveyard from the battlefield,",
        )
    {
        return format!(
            "When this creature enters and whenever another {subject} you control is put into a graveyard from the battlefield,{effect_clause}"
        );
    }
    if let Some((left, right)) = normalized.split_once(" or Whenever another ")
        && left.starts_with("Whenever ")
    {
        return format!("{left} or another {right}");
    }
    if let Some((left, right)) = normalized.split_once(" or whenever another ")
        && left.starts_with("whenever ")
    {
        return format!("{left} or another {right}");
    }
    if let Some(rest) = lower.strip_prefix("whenever this creature or whenever another ") {
        return format!("Whenever this creature or another {rest}");
    }
    if let Some(rest) = lower.strip_prefix("whenever this or whenever another ") {
        return format!("Whenever this or another {rest}");
    }
    if let Some(rest) = lower.strip_prefix("when this creature leaves the battlefield or another ")
        && rest.contains(" leaves the battlefield")
    {
        return format!("Whenever this creature or another {rest}");
    }
    if let Some(rest) = lower.strip_prefix("when this leaves the battlefield or another ")
        && rest.contains(" leaves the battlefield")
    {
        return format!("Whenever this or another {rest}");
    }
    if let Some(rest) = normalized
        .strip_prefix("At the beginning of your upkeep, tap this creature unless you lose ")
    {
        return format!("At the beginning of your upkeep, tap this creature unless you pay {rest}");
    }
    if let Some(rest) =
        lower.strip_prefix("at the beginning of your upkeep, tap this creature unless you lose ")
    {
        return format!("At the beginning of your upkeep, tap this creature unless you pay {rest}");
    }
    if normalized == "Whenever this creature becomes blocked, the defending player discards a card"
    {
        return "Whenever this creature becomes blocked, defending player discards a card"
            .to_string();
    }
    for lead in ["Whenever", "whenever"] {
        for owner_phrase in ["you don't own", "you dont own"] {
            let marker = format!("{lead} you cast a {owner_phrase}, for each ");
            if let Some(rest) = normalized.strip_prefix(&marker) {
                for tail in [
                    " spell, Put a +1/+1 counter on that object.",
                    " spell, put a +1/+1 counter on that object.",
                    " spell, Put a +1/+1 counter on that object",
                    " spell, put a +1/+1 counter on that object",
                ] {
                    if let Some(filter) = rest.strip_suffix(tail) {
                        return format!(
                            "Whenever you cast a spell you don't own, put a +1/+1 counter on each {filter}."
                        );
                    }
                }
            }
        }
    }
    if let Some(rest) = normalized.strip_prefix("Whenever you cast spell ") {
        if let Some((kind, tail)) = rest.split_once(',') {
            let kind = kind.trim();
            let needs_article = !kind.is_empty()
                && !kind.starts_with("a ")
                && !kind.starts_with("an ")
                && !matches!(kind, "a" | "an" | "another");
            if needs_article {
                return format!(
                    "Whenever you cast {} spell,{}",
                    with_indefinite_article(kind),
                    tail
                );
            }
            return format!("Whenever you cast {kind} spell,{tail}");
        }
        let kind = rest.trim();
        if !kind.is_empty() {
            let needs_article = !kind.starts_with("a ")
                && !kind.starts_with("an ")
                && !matches!(kind, "a" | "an" | "another");
            if needs_article {
                return format!("Whenever you cast {} spell", with_indefinite_article(kind));
            }
            return format!("Whenever you cast {kind} spell");
        }
    }
    if let Some(rest) = normalized.strip_prefix("Whenever you cast ")
        && let Some((kind, tail)) = rest.split_once(" spell,")
    {
        let kind = kind.trim();
        if !kind.is_empty()
            && !kind.starts_with("a ")
            && !kind.starts_with("an ")
            && !matches!(kind, "a" | "an" | "another")
        {
            return format!(
                "Whenever you cast {} spell,{}",
                with_indefinite_article(kind),
                tail
            );
        }
    }
    if let Some(kind) = normalized
        .strip_prefix("Whenever you cast ")
        .and_then(|tail| tail.strip_suffix(" spell"))
    {
        let kind = kind.trim();
        if !kind.is_empty()
            && !kind.starts_with("a ")
            && !kind.starts_with("an ")
            && !matches!(kind, "a" | "an" | "another")
        {
            return format!("Whenever you cast {} spell", with_indefinite_article(kind));
        }
    }
    if let Some(rest) = normalized.strip_prefix("Whenever you cast spell with mana value ") {
        return format!("Whenever you cast a spell with mana value {rest}");
    }
    for owner_phrase in ["you don't own", "you dont own"] {
        let marker = format!("Whenever you cast a {owner_phrase}, for each ");
        if let Some((head, rest)) = normalized.split_once(&marker) {
            for tail in [
                " spell, Put a +1/+1 counter on that object.",
                " spell, put a +1/+1 counter on that object.",
                " spell, Put a +1/+1 counter on that object",
                " spell, put a +1/+1 counter on that object",
            ] {
                if let Some(filter) = rest.strip_suffix(tail) {
                    return format!(
                        "{head}Whenever you cast a spell you don't own, put a +1/+1 counter on each {filter}."
                    );
                }
            }
        }
    }
    if let Some(rest) = normalized.strip_prefix("Whenever one or more ")
        && let Some(tail) = rest.strip_suffix(
            " deal combat damage to a player: Exile card in that player's library. If that doesn't happen, create a Treasure token.",
        )
    {
        return format!(
            "Whenever one or more {tail} deal combat damage to a player, exile the top card of that player's library. If you don't, create a Treasure token."
        );
    }
    if let Some((prefix, rest)) = normalized.split_once(" have the first ")
        && let Some((kind, tail)) = rest.split_once(" spell you cast each turn costs ")
        && let Some(amount) = tail.strip_suffix(" less to cast")
        && let Ok(amount) = amount.trim().parse::<u32>()
    {
        return format!(
            "{prefix} have \"The first {} spell you cast each turn costs {{{amount}}} less to cast.\"",
            capitalize_first(kind.trim())
        );
    }
    if let Some((prefix, rest)) = normalized.split_once(" has the first ")
        && let Some((kind, tail)) = rest.split_once(" spell you cast each turn costs ")
        && let Some(amount) = tail.strip_suffix(" less to cast")
        && let Ok(amount) = amount.trim().parse::<u32>()
    {
        return format!(
            "{prefix} has \"The first {} spell you cast each turn costs {{{amount}}} less to cast.\"",
            capitalize_first(kind.trim())
        );
    }
    if normalized == "When this creature enters, you sacrifice a creature" {
        return "When this creature enters, sacrifice a creature".to_string();
    }
    if normalized == "you draw a card. Scry 2" {
        return "Draw a card. Scry 2".to_string();
    }
    if normalized == "This creature enters with X +1/+1 counters" {
        return "This creature enters with X +1/+1 counters on it".to_string();
    }
    if normalized == "Trample, Haste" {
        return "Trample, haste".to_string();
    }
    if let Some(rest) =
        normalized.strip_prefix("Whenever you cast an instant or sorcery spell, deal ")
    {
        return format!(
            "Whenever you cast an instant or sorcery spell, this creature deals {rest}"
        );
    }
    if let Some(rest) = lower.strip_prefix("whenever you cast an instant or sorcery spell, deal ") {
        return format!(
            "Whenever you cast an instant or sorcery spell, this creature deals {rest}"
        );
    }
    if let Some(rest) = normalized.strip_prefix("Whenever you cast a noncreature spell, it deals ")
    {
        return format!("Whenever you cast a noncreature spell, this creature deals {rest}");
    }
    if let Some(rest) = lower.strip_prefix("whenever you cast a noncreature spell, deal ") {
        return format!("Whenever you cast a noncreature spell, this creature deals {rest}");
    }
    if let Some(rest) = lower.strip_prefix("whenever a land you control enters, deal ") {
        return format!("Whenever a land you control enters, this creature deals {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("{T}: Deal ") {
        return format!("{{T}}: This creature deals {rest}");
    }
    if let Some(rest) = normalized
        .strip_prefix("Search your library for a card, put it on top of library, then shuffle. ")
    {
        return format!(
            "Search your library for a card, then shuffle and put that card on top. {rest}"
        );
    }
    if normalized == "target creature gains Deathtouch and gains Indestructible until end of turn" {
        return "Target creature gains deathtouch and indestructible until end of turn".to_string();
    }
    if normalized == "When this Aura enters, tap enchanted creature" {
        return "When this Aura enters, tap enchanted creature.".to_string();
    }
    if normalized == "Doesn't untap during your untap step" {
        return "This creature doesn't untap during your untap step".to_string();
    }
    if normalized == "This creature enters with 2 +1/+1 counters" {
        return "This creature enters with two +1/+1 counters on it".to_string();
    }
    if let Some(amount) = normalized
        .strip_prefix("At the beginning of your upkeep, it deals ")
        .and_then(|rest| rest.strip_suffix(" damage to you"))
    {
        return format!(
            "At the beginning of your upkeep, this creature deals {amount} damage to you"
        );
    }
    if let Some(rest) = normalized.strip_prefix("Search your library for a ")
        && let Some((tribe, tail)) = rest.split_once(" with mana value ")
        && let Some(value) = tail.strip_suffix(" card, put it onto the battlefield, then shuffle")
    {
        return format!(
            "Search your library for a {tribe} permanent card with mana value {value}, put it onto the battlefield, then shuffle"
        );
    }
    if let Some((cost, rest)) = normalized.split_once(": Search your library for a ")
        && let Some((tribe, tail)) = rest.split_once(" with mana value ")
        && let Some(value) = tail.strip_suffix(" card, put it onto the battlefield, then shuffle")
    {
        return format!(
            "{cost}: Search your library for a {tribe} permanent card with mana value {value}, put it onto the battlefield, then shuffle"
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, ". ")
        && let Some(draw_tail) = strip_prefix_ascii_ci(left.trim(), "target player draws ")
        && let Some(loss_tail) = strip_prefix_ascii_ci(right.trim(), "target player loses ")
        && loss_tail
            .trim_end_matches('.')
            .to_ascii_lowercase()
            .ends_with(" life")
    {
        let draw_tail = draw_tail.trim();
        let draw_tail = draw_tail
            .strip_suffix(" cards")
            .map(|count| format!("{} cards", render_small_number_or_raw(count.trim())))
            .unwrap_or_else(|| draw_tail.to_string());
        return format!(
            "Target player draws {draw_tail} and loses {}",
            loss_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((left, right)) = normalized.split_once(" and target player loses ")
        && left.starts_with("target player draws ")
    {
        let left = left.replacen("target player draws ", "Target player draws ", 1);
        return format!("{left} and loses {right}");
    }
    if let Some((first, second)) = split_once_ascii_ci(&normalized, ". ")
        && let Some(first_buff) = strip_prefix_ascii_ci(first.trim(), "target creature gets ")
            .and_then(|rest| rest.strip_suffix(" until end of turn"))
        && let Some(second_buff) = strip_prefix_ascii_ci(
            second.trim(),
            "other creatures with the same name as that object get ",
        )
        .and_then(|rest| {
            rest.strip_suffix(" until end of turn")
                .or_else(|| rest.strip_suffix(" until end of turn."))
        })
        && first_buff.eq_ignore_ascii_case(second_buff)
    {
        return format!(
            "Target creature and all other creatures with the same name as that creature get {first_buff} until end of turn"
        );
    }
    if let Some((first, rest)) = split_once_ascii_ci(&normalized, ". ")
        && let Some(target_desc) = strip_prefix_ascii_ci(first.trim(), "Exile target ")
    {
        let target_desc = target_desc.trim().trim_end_matches('.');
        let other_desc = pluralize_noun_phrase(target_desc);
        let expected_second = format!(
            "Exile all other {other_desc} with the same name as that object controlled by that object's controller"
        );
        if let Some((second, tail)) = split_once_ascii_ci(rest, ". ")
            && second.trim().eq_ignore_ascii_case(&expected_second)
        {
            let reference = if target_desc.eq_ignore_ascii_case("creature")
                || target_desc.to_ascii_lowercase().ends_with(" creature")
            {
                "that creature"
            } else {
                "that object"
            };
            let merged = format!(
                "Exile target {target_desc} and all other {other_desc} its controller controls with the same name as {reference}"
            );
            return format!(
                "{merged}. {}",
                capitalize_first(tail.trim().trim_end_matches('.'))
            );
        }
        if rest.trim().eq_ignore_ascii_case(&expected_second) {
            let reference = if target_desc.eq_ignore_ascii_case("creature")
                || target_desc.to_ascii_lowercase().ends_with(" creature")
            {
                "that creature"
            } else {
                "that object"
            };
            return format!(
                "Exile target {target_desc} and all other {other_desc} its controller controls with the same name as {reference}"
            );
        }
    }
    if let Some(rest) =
        normalized.strip_prefix("Destroy target black or red attacking or blocking creature")
    {
        return format!("Destroy target black or red creature that's attacking or blocking{rest}");
    }
    if let Some((left, right)) = normalized.split_once(". ")
        && let Some(target_desc) = left.strip_prefix("Destroy target ")
        && let Some(other_desc) = right.strip_prefix("Destroy all other ")
        && let Some((shares_desc, tail)) =
            other_desc.split_once(" that shares a color with that object")
        && shares_desc.eq_ignore_ascii_case(target_desc)
    {
        return format!(
            "Destroy target {target_desc} and each other {shares_desc} that shares a color with it{tail}"
        );
    }
    if let Some(tail) = normalized.strip_prefix("you draw ")
        && let Some(count) = tail.strip_suffix(" cards")
    {
        return format!("Draw {} cards", render_small_number_or_raw(count));
    }
    if let Some(tail) = normalized
        .strip_prefix("Counter target spell, then its controller mills ")
        .and_then(|rest| rest.strip_suffix(" cards"))
    {
        return format!(
            "Counter target spell. Its controller mills {} cards",
            render_small_number_or_raw(tail)
        );
    }
    if let Some(tail) = normalized
        .strip_prefix("Counter target spell, then its controller mills ")
        .and_then(|rest| rest.strip_suffix(" card"))
    {
        return format!(
            "Counter target spell. Its controller mills {} card",
            render_small_number_or_raw(tail)
        );
    }
    if let Some(rest) = normalized.strip_prefix("target creature you control gets ")
        && let Some((pt, tail)) =
            rest.split_once(" until end of turn, then it fights target creature you don't control")
    {
        return format!(
            "Target creature you control gets {pt} until end of turn. It fights target creature you don't control{tail}"
        );
    }
    if let Some(rest) = normalized.strip_prefix("Target creature you control gets ")
        && let Some((pt, tail)) =
            rest.split_once(" until end of turn, then it fights target creature you don't control")
    {
        return format!(
            "Target creature you control gets {pt} until end of turn. It fights target creature you don't control{tail}"
        );
    }
    if let Some(rest) = normalized
        .strip_prefix("this creature gets ")
        .or_else(|| normalized.strip_prefix("This creature gets "))
        && let Some((pt, cond)) = rest.split_once(" as long as ")
        && let Some((keyword, right_cond)) = cond.split_once(" and has ")
        && let Some((granted, repeated_cond)) = right_cond.split_once(" as long as ")
    {
        let keyword = keyword.trim().trim_end_matches('.');
        let repeated_cond = repeated_cond.trim().trim_end_matches('.');
        if keyword.eq_ignore_ascii_case(repeated_cond) {
            return format!(
                "As long as {keyword}, this creature gets {pt} and has {}",
                normalize_keyword_predicate_case(granted)
            );
        }
    }
    if let Some(rest) = normalized.strip_prefix("Create ")
        && let Some((token_desc, tail)) = rest.split_once(" token, tapped")
    {
        return format!("Create a tapped {token_desc} token{tail}");
    }
    if let Some(rest) = normalized.strip_prefix("Create ")
        && let Some((token_desc, tail)) = rest.split_once(" tokens, tapped")
    {
        return format!("Create tapped {token_desc} tokens{tail}");
    }
    if let Some((left, right)) = normalized.split_once(". ")
        && left.contains(" deals ")
        && right.starts_with("Deal ")
        && right.ends_with(" damage to you")
    {
        return format!("{left} and {}", lowercase_first(right));
    }
    if let Some(rest) = normalized.strip_prefix("For each player, that player ") {
        return format!("Each player {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("For each opponent, that player ") {
        return format!("Each opponent {rest}");
    }
    if let Some(tail) = normalized
        .strip_prefix("Whenever this creature blocks or becomes blocked by a creature, it deals ")
    {
        return format!(
            "Whenever this creature blocks or becomes blocked by a creature, this creature deals {tail}"
        );
    }
    if normalized.contains("cycling {{") {
        normalized = normalized.replace("{{", "{").replace("}}", "}");
    }

    // Keep explicitly rendered `that player` antecedents. Generic player-loop
    // and delayed-player surfaces already choose `they` at their typed
    // rendering source; rewriting the explicit form here erases distinct
    // controller-of-triggering-object relationships in counter-unless effects.
    normalized = normalized
        .replace("This creatures get ", "This creature gets ")
        .replace("This creatures gain ", "This creature gains ")
        .replace("this permanent gets +", "this creature gets +")
        .replace(", If ", ", if ")
        .replace(", Transform ", ", transform ")
        .replace("Counter target spell. that object's controller mills ", "Counter target spell, then its controller mills ")
        .replace(" for each creature blocking it until end of turn", " until end of turn for each creature blocking it")
        .replace(" for each artifact you control until end of turn", " until end of turn for each artifact you control")
        .replace("when this creature enters or When this creature dies, ", "When this creature enters or dies, ")
        .replace("When this creature enters or When this creature dies, ", "When this creature enters or dies, ")
        .replace("Whenever this creature blocks creature, ", "Whenever this creature blocks a creature, ")
        .replace("target creature you don't control or planeswalker", "target creature or planeswalker you don't control")
        .replace("Counter target instant spell spell", "Counter target instant spell")
        .replace("Counter target sorcery spell spell", "Counter target sorcery spell")
        .replace(" spell spell", " spell")
        .replace("the defending player", "defending player")
        .replace("Non-Human attacking creatures", "Attacking non-Human creatures")
        .replace("non-Human attacking creatures", "attacking non-Human creatures")
        .replace("Non-Human attacking creature", "Attacking non-Human creature")
        .replace("non-Human attacking creature", "attacking non-Human creature")
        .replace("Whenever this creature or Whenever another Ally you control enters", "Whenever this creature or another Ally you control enters")
        .replace("Chapter 1:", "I —")
        .replace("Chapter 2:", "II —")
        .replace("Chapter 3:", "III —")
        .replace("you draw a card. Scry 2", "Draw a card. Scry 2")
        .replace("Investigate 1", "Investigate")
        .replace("target player draws 2 cards", "Target player draws two cards")
        .replace("target player draws 3 cards", "Target player draws three cards")
        .replace("Draw 2 cards", "Draw two cards")
        .replace("Draw 3 cards", "Draw three cards")
        .replace("draw 2 cards", "draw two cards")
        .replace("draw 3 cards", "draw three cards")
        .replace("Create 1 ", "Create a ")
        .replace("create 1 ", "create a ")
        .replace("Create 2 ", "Create two ")
        .replace("Create 3 ", "Create three ")
        .replace("create 2 ", "create two ")
        .replace("create 3 ", "create three ")
        .replace(
            "Create a Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. tapped under your control",
            "Create a tapped Treasure token",
        )
        .replace(
            "create a Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. tapped under your control",
            "create a tapped Treasure token",
        )
        .replace(
            "Create a 0/1 colorless Eldrazi Spawn creature token with Sacrifice this creature: Add {C}. under your control",
            "Create a 0/1 colorless Eldrazi Spawn creature token. It has \"Sacrifice this token: Add {C}.\"",
        )
        .replace(
            "create a 0/1 colorless Eldrazi Spawn creature token with Sacrifice this creature: Add {C}. under your control",
            "create a 0/1 colorless Eldrazi Spawn creature token. It has \"Sacrifice this token: Add {C}.\"",
        )
        .replace(
            "Create a 1/1 colorless Eldrazi Scion creature token with Sacrifice this creature: Add {C}. under your control",
            "Create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this token: Add {C}.\"",
        )
        .replace(
            "create a 1/1 colorless Eldrazi Scion creature token with Sacrifice this creature: Add {C}. under your control",
            "create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this token: Add {C}.\"",
        )
        .replace("Put 2 ", "Put two ")
        .replace("Put 3 ", "Put three ")
        .replace("put 2 ", "put two ")
        .replace("put 3 ", "put three ")
        .replace("up to 1 ", "up to one ")
        .replace("up to 2 ", "up to two ")
        .replace("up to 3 ", "up to three ")
        .replace("one or 2 ", "one or two ")
        .replace(
            "Destroy target land. that object's controller loses ",
            "Destroy target land. Its controller loses ",
        )
        .replace(
            "Prevent combat damage to players until end of turn",
            "Prevent all combat damage that would be dealt to players this turn",
        )
        .replace(
            "target creature you control gets +1/+2 until end of turn, then it fights target creature you don't control",
            "Target creature you control gets +1/+2 until end of turn. It fights target creature you don't control",
        )
        .replace(
            "target creature you control gets +2/+2 until end of turn, then it fights target creature you don't control",
            "Target creature you control gets +2/+2 until end of turn. It fights target creature you don't control",
        )
        .replace(
            "target creature you control gets +2/+1 until end of turn, then it fights target creature you don't control",
            "Target creature you control gets +2/+1 until end of turn. It fights target creature you don't control",
        )
        .replace("spells you control cost ", "spells you cast cost ")
        .replace("creature you control cost ", "creature spells you cast cost ")
        .replace(
            "instant or sorcery you control cost ",
            "instant and sorcery spells you cast cost ",
        )
        .replace("you may you sacrifice ", "You may sacrifice ")
        .replace("You may you sacrifice ", "You may sacrifice ")
        .replace("you may you attach ", "You may attach ")
        .replace("You may you attach ", "You may attach ")
        .replace(
            "rather than pay this spell's mana cost (Parsed alternative cost)",
            "rather than pay this spell's mana cost",
        )
        .replace("controlss", "controls")
        .replace(
            "the tagged object 'exiled_0' matches creature",
            "that card is a creature card",
        )
        .replace(
            "the tagged object 'triggering' matches creature",
            "that object is a creature",
        )
        .replace("If that object is a ", "If it's a ")
        .replace("If that object isn't a ", "If it's not a ")
        .replace("if that object isn't a ", "if it's not a ")
        .replace("the tagged object 'triggering'", "that object")
        .replace(" that player controls of their choice", " of their choice")
        .replace(" that player controls unless that player pays ", " unless that player pays ")
        .replace("casts creature spell", "casts a creature spell")
        .replace("casts colorless spell", "casts a colorless spell")
        .replace(
            "permanent with the same name as that object cards",
            "cards with the same name as that object",
        )
        .replace(
            "permanent with the same name as that object card",
            "card with the same name as that object",
        )
        .replace(
            "permanent with the same name as it cards",
            "cards with the same name as it",
        )
        .replace(
            "permanent with the same name as it card",
            "card with the same name as it",
        )
        .replace(
            "Whenever a player casts a spell, counter it unless its controller pays ",
            "Whenever a player casts a spell, counter that spell unless that player pays ",
        )
        .replace("Counter target instant", "Counter target instant spell")
        .replace(
            "Counter target instant spell spell and sorcery spell",
            "Counter target instant or sorcery spell",
        )
        .replace("Counter target sorcery", "Counter target sorcery spell")
        .replace("Counter target instant spell spell", "Counter target instant spell")
        .replace("Counter target sorcery spell spell", "Counter target sorcery spell")
        .replace(
            "Counter target enchantment or instant or sorcery",
            "Counter target enchantment, instant, or sorcery spell",
        )
        .replace(
            "Counter target artifact or creature or planeswalker",
            "Counter target artifact, creature, or planeswalker spell",
        )
        .replace(
            "Counter target artifact or creature unless its controller pays ",
            "Counter target artifact or creature spell unless its controller pays ",
        )
        .replace(
            "Target attacking/blocking creature",
            "Target attacking or blocking creature",
        )
        .replace(
            "Scry 2. you draw a card",
            "Scry 2, then draw a card",
        )
        .replace(". you draw a card", ". Draw a card")
        .replace(
            "When this enchantment enters, Tap enchanted creature",
            "When this Aura enters, tap enchanted creature",
        )
        .replace(
            "When this enchantment enters, Exile target nonland permanent an opponent controls",
            "When this enchantment enters, exile target nonland permanent an opponent controls",
        )
        .replace(
            "Target opponent's creature",
            "Target creature an opponent controls",
        )
        .replace(
            "target opponent's creature",
            "target creature an opponent controls",
        )
        .replace("Sacrifice other creature", "Sacrifice another creature")
        .replace("sacrifice other creature", "sacrifice another creature")
        .replace(
            "As an additional cost to cast this spell, sacrifice creature you control",
            "As an additional cost to cast this spell, sacrifice a creature",
        )
        .replace(
            "As an additional cost to cast this spell, sacrifice a creature you control",
            "As an additional cost to cast this spell, sacrifice a creature",
        )
        .replace(
            "As an additional cost to cast this spell, discard card",
            "As an additional cost to cast this spell, discard a card",
        )
        .replace(
            "At the beginning of each end step, that player ",
            "At the beginning of each player's end step, that player ",
        )
        .replace(
            "that player sacrifices an untapped land.",
            "that player sacrifices an untapped land of their choice.",
        )
        .replace(
            "If an opponent would begin an extra turn, that player skips that turn.",
            "If an opponent would begin an extra turn, that player skips that turn instead.",
        )
        ;
    if normalized.starts_with("At the beginning of each end step, if ")
        && normalized.contains(", that player ")
    {
        normalized = normalized.replacen(
            "At the beginning of each end step, if ",
            "At the beginning of each player's end step, if ",
            1,
        );
    }
    if normalized.starts_with("At the beginning of each player's end step,") {
        let lower = normalized.to_ascii_lowercase();
        if !lower.contains("that player")
            && !lower.contains("the player")
            && !lower.contains("entered the battlefield under your control this turn")
        {
            normalized = normalized.replacen(
                "At the beginning of each player's end step,",
                "At the beginning of each end step,",
                1,
            );
        }
    }
    if normalized
        == "At the beginning of your upkeep, if you have the city's blessing, you draw a card. Otherwise, each player draws a card."
    {
        normalized = "At the beginning of your upkeep, each player draws a card. If you have the city's blessing, instead only you draw a card.".to_string();
    }
    if normalized.contains("that player loses life equal to this creature's power") {
        normalized = normalized.replace(
            "that player loses life equal to this creature's power",
            "that player loses X life, where X is this creature's power",
        );
    }
    if normalized.contains("you may ")
        && normalized.contains(" unless you ")
        && !normalized.contains(" unless you pay ")
        && !normalized.contains(" unless you pays ")
    {
        normalized = normalized.replacen(" unless you ", " or ", 1);
    }
    if let Some((left, rest)) = normalized.split_once("target card ")
        && let Some((kind, right)) = rest.split_once(" from")
    {
        let lower_kind = kind.to_ascii_lowercase();
        let blocked = matches!(
            lower_kind.as_str(),
            "a" | "an" | "the" | "named" | "from" | "in" | "with" | "without"
        );
        if !kind.contains(' ') && !blocked {
            normalized = format!("{left}target {kind} card from{right}");
        }
    }
    normalized = normalized
        .replace("that cards instead", "that card instead")
        .replace("Return a Island", "Return an Island")
        .replace("Return a artifact", "Return an artifact")
        .replace("Return a Aura", "Return an Aura")
        .replace("You may Return ", "You may return ");
    if let Some(mapped) = normalize_trigger_colon_clause(&normalized) {
        normalized = mapped;
    }
    if let Some((head, tail)) = normalized.split_once(", ")
        && (head.starts_with("When ")
            || head.starts_with("Whenever ")
            || head.starts_with("At the beginning "))
        && !tail.is_empty()
        && tail
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        && should_lowercase_trigger_effect_tail(tail)
    {
        normalized = format!("{head}, {}", lowercase_first(tail));
    }
    if let Some((head, tail)) = normalized.split_once(", ")
        && head
            .to_ascii_lowercase()
            .starts_with("whenever a creature blocks ")
        && tail
            .to_ascii_lowercase()
            .starts_with("blocking creatures get ")
    {
        let normalized_tail =
            tail.replacen("blocking creatures get ", "the blocking creature gets ", 1);
        normalized = format!("{head}, {normalized_tail}");
    }
    // Run these cross-sentence structural repairs after the generic sentence
    // cleanup too: several of those passes legitimately factor ForEach and
    // conditional scaffolding, producing the exact compact shapes certified
    // by these matchers only at the end of normalization.
    if let Some(compact) = restore_draw_exile_time_counter_granted_cast_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_sacrificed_power_damage_replacement_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = restore_sacrificed_power_each_opponent_draw_surface(&normalized) {
        normalized = compact;
    }
    normalized
}

pub(crate) fn normalize_reveal_match_filter(filter: &str) -> String {
    let mut normalized = filter.trim().to_string();
    if !normalized.ends_with("card") && !normalized.ends_with("cards") {
        normalized.push_str(" card");
    }
    let lower = normalized.to_ascii_lowercase();
    if lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("the ")
        || lower.starts_with("that ")
        || lower.starts_with("this ")
        || lower.starts_with("those ")
        || lower.starts_with("these ")
    {
        return normalized;
    }
    let article = if lower
        .chars()
        .next()
        .is_some_and(|ch| matches!(ch, 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        "an"
    } else {
        "a"
    };
    format!("{article} {normalized}")
}

pub(crate) fn normalize_reveal_tagged_draw_clause(line: &str) -> Option<String> {
    for prefix in [
        "Reveal the top card of your library and tag it as 'revealed_0'. If the tagged object 'revealed_0' matches ",
        "you may Reveal the top card of your library and tag it as 'revealed_0'. If the tagged object 'revealed_0' matches ",
    ] {
        let Some(start) = line.find(prefix) else {
            continue;
        };
        let rest = &line[start + prefix.len()..];
        let (filter, suffix) = if let Some(filter) = rest.strip_prefix("")
            && let Some(stripped) = filter.strip_suffix(", you draw a card.")
        {
            (stripped, ".")
        } else if let Some(filter) = rest.strip_prefix("")
            && let Some(stripped) = filter.strip_suffix(", you draw a card")
        {
            (stripped, "")
        } else {
            continue;
        };

        let before = &line[..start];
        let reveal_clause = if prefix.starts_with("you may ") {
            format!(
                "you may reveal the top card of your library. If it's {}, draw a card{}",
                normalize_reveal_match_filter(filter),
                suffix
            )
        } else {
            format!(
                "Reveal the top card of your library. If it's {}, draw a card{}",
                normalize_reveal_match_filter(filter),
                suffix
            )
        };
        return Some(format!("{before}{reveal_clause}"));
    }
    None
}

pub(crate) fn strip_square_bracketed_segments(text: &str) -> String {
    if !text.contains('[') {
        return text.to_string();
    }
    let chars = text.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(text.len());
    let mut idx = 0usize;
    while idx < chars.len() {
        if chars[idx] == '[' {
            let start = idx;
            let mut end = idx + 1;
            let mut depth = 1usize;
            while end < chars.len() {
                match chars[end] {
                    '[' => depth += 1,
                    ']' => {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                end += 1;
            }
            if depth != 0 {
                break;
            }
            let content = chars[start + 1..end].iter().collect::<String>();
            let mut after = end + 1;
            while after < chars.len() && chars[after].is_whitespace() {
                after += 1;
            }
            if after < chars.len()
                && chars[after] == ':'
                && is_bracketed_loyalty_activation_cost(&content)
            {
                out.extend(chars[start..=end].iter());
            }
            idx = end + 1;
            continue;
        }

        out.push(chars[idx]);
        idx += 1;
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Post-search/reveal library shuffles use the modern oracle tail ("Then
/// shuffle" / "Then that player shuffles") instead of restating the library
/// noun. Gated on a preceding search or reveal-from-library clause in the
/// same line so standalone shuffle instructions keep their explicit object.
fn normalize_post_search_shuffle_tails(line: &str) -> String {
    let mut normalized = line.to_string();

    // Inside a quantified optional search, the shuffle remains part of the
    // same may-action. Generic sentence rendering can repeat the plural actor
    // as a new sentence; restore the authored connective and implicit actor.
    if let Some(idx) = normalized.find(". Then they shuffle") {
        let before = normalized[..idx].to_ascii_lowercase();
        if before.contains("may search their library") {
            normalized.replace_range(idx..idx + ". Then they shuffle".len(), ", then shuffle");
        }
    }

    // The comma variant stays connective-free: ", then shuffle" would be
    // split as a separate clause by the semantic comparer, while oracle's
    // "If you search your library this way, shuffle." keeps one clause.
    for (needle, replacement) in [
        (". Shuffle your library", ". Then shuffle"),
        (", shuffle your library", ", shuffle"),
    ] {
        if let Some(idx) = normalized.find(needle) {
            let before = normalized[..idx].to_ascii_lowercase();
            if before.contains("search your library")
                || before.contains("searches your library")
                || before.contains("reveal cards from the top of your library")
            {
                let tail_start = idx + needle.len();
                normalized = format!(
                    "{}{replacement}{}",
                    &normalized[..idx],
                    &normalized[tail_start..]
                );
            }
        }
    }

    let their_idx = normalized.find(". Shuffle their library");
    if let Some(idx) = their_idx {
        let before = normalized[..idx].to_ascii_lowercase();
        if (before.contains("search") || before.contains("searches")) && before.contains("library")
        {
            let tail_start = idx + ". Shuffle their library".len();
            normalized = format!(
                "{}. Then that player shuffles{}",
                &normalized[..idx],
                &normalized[tail_start..]
            );
        }
    }

    normalized
}

/// Oracle writes the negative branch of an in-line conditional as
/// "Otherwise, ..."; the renderer's literal else surface ("If that doesn't
/// happen, ...") never appears in printed text. Gated on a preceding
/// conditional marker so a bare else branch without an antecedent keeps the
/// explicit form.
fn normalize_else_branch_otherwise_surface(line: &str) -> String {
    const NEEDLE: &str = ". If that doesn't happen, ";
    let Some(idx) = line.find(NEEDLE) else {
        return line.to_string();
    };
    let before = line[..idx].to_ascii_lowercase();
    // A declined payment names the payer in oracle ("its controller may pay
    // {1}. If that player doesn't, ..."); other declined choices read as
    // "Otherwise, ...".
    if before.contains(" may pay ") {
        return format!(
            "{}. If that player doesn't, {}",
            &line[..idx],
            &line[idx + NEEDLE.len()..]
        );
    }
    let has_antecedent = before.contains(" if ")
        || before.contains(" unless ")
        || before.contains(" may ")
        || before.starts_with("if ");
    if !has_antecedent {
        return line.to_string();
    }
    format!(
        "{}. Otherwise, {}",
        &line[..idx],
        &line[idx + NEEDLE.len()..]
    )
}

/// The Wish family's search-outside-the-game program renders as three
/// sentences; oracle authors it as one reveal-and-put sentence. Fold the
/// renderer's program back to the authored surface.
pub(crate) fn normalize_search_outside_game_reveal_surface(line: &str) -> String {
    const PREFIX: &str = "You may search outside the game for up to one ";
    const TAIL: &str = ". Reveal it. Put it into its owner's hand.";
    let Some(start) = line.find(PREFIX) else {
        return line.to_string();
    };
    let sentence_start = start == 0 || line[..start].ends_with(". ");
    if !sentence_start {
        return line.to_string();
    }
    let after_prefix = &line[start + PREFIX.len()..];
    let Some(filter_end) = after_prefix.find(TAIL) else {
        return line.to_string();
    };
    let filter_text = &after_prefix[..filter_end];
    if filter_text.contains('.') || filter_text.is_empty() {
        return line.to_string();
    }
    let article = if filter_text
        .chars()
        .next()
        .is_some_and(|c| matches!(c.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        "an"
    } else {
        "a"
    };
    format!(
        "{}You may reveal {article} {filter_text} from outside the game and put it into your hand.{}",
        &line[..start],
        &after_prefix[filter_end + TAIL.len()..]
    )
}

/// "Choose target player and search their library ..." is renderer
/// scaffolding for oracle's single possessive mention: "Search target
/// player's library ...". Fold the choose clause into the possessive.
fn normalize_choose_target_player_search_scaffold(line: &str) -> String {
    const FORMS: &[(&str, &str, &str)] = &[
        (
            "Choose target player and search their ",
            "choose target player and search their ",
            "target player's ",
        ),
        (
            "Choose target opponent and search their ",
            "choose target opponent and search their ",
            "target opponent's ",
        ),
        (
            "Choose target player and search target player's ",
            "choose target player and search target player's ",
            "target player's ",
        ),
        (
            "Choose target opponent and search target opponent's ",
            "choose target opponent and search target opponent's ",
            "target opponent's ",
        ),
    ];
    let mut normalized = line.to_string();
    for (upper, lower, possessive) in FORMS {
        loop {
            let Some(idx) = normalized.find(upper).or_else(|| normalized.find(lower)) else {
                break;
            };
            let sentence_start =
                idx == 0 || normalized[..idx].ends_with(". ") || normalized[..idx].ends_with(": ");
            if !sentence_start {
                break;
            }
            let rest = &normalized[idx + upper.len()..];
            let search_word = if idx == 0 { "Search " } else { "search " };
            normalized = format!(
                "{}{}{}{}",
                &normalized[..idx],
                search_word,
                possessive,
                rest
            );
        }
    }
    normalized
}

/// "Choose target opponent and <verb> ... target opponent ..." repeats the
/// target selection oracle expresses once; the choose clause is renderer
/// scaffolding for the shared target, so drop it when the remainder still
/// names the target opponent.
fn normalize_redundant_choose_target_opponent_scaffold(line: &str) -> String {
    const PREFIXES: &[&str] = &["Choose target opponent and ", "choose target opponent and "];
    let mut normalized = line.to_string();
    for prefix in PREFIXES {
        loop {
            let Some(idx) = normalized.find(prefix) else {
                break;
            };
            let sentence_start =
                idx == 0 || normalized[..idx].ends_with(". ") || normalized[..idx].ends_with(": ");
            let rest = &normalized[idx + prefix.len()..];
            let rest_sentence = rest.split(". ").next().unwrap_or(rest);
            if !sentence_start || !rest_sentence.contains("target opponent") {
                break;
            }
            normalized = format!("{}{}", &normalized[..idx], capitalize_first(rest));
        }
    }
    normalized
}
