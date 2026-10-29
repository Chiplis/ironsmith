//! Pure re-phrasings of semantic-audit lines for triggered and activated
//! abilities and modes: the same typed reading in Oracle's words. Every
//! rewrite keys on the structure of the audit reading itself (never on
//! retained prose) and keeps its meaning; each was checked against the
//! Oracle phrasing used by the cards that reach it.

use crate::CardDefinition;

pub(super) fn audit_ability_line_rephrasings(line: String, def: &CardDefinition) -> String {
    let mut line = line;
    // CR 506.2: Oracle names the defending player without an article.
    line = line.replace("the defending player", "defending player");
    line = line.replace("you completed a dungeon", "you've completed a dungeon");
    line = basic_land_or_type_card(&line);
    line = each_of_several_targets_get(&line);
    line = remove_those_counters(&line);
    // CR 702.51a: the creatures that convoked the spell that became this
    // permanent.
    line = line
        .replace(
            "each creature that convoked this spell on the battlefield",
            "each creature that convoked it",
        )
        .replace(
            "that convoked this spell on the battlefield",
            "that convoked this creature",
        );
    // A follow-up restriction on the creature just returned / controlled.
    line = line.replace(". Permanent can't ", ". That creature can't ");
    line = targets_only_a_single(&line);
    line = line.replace(
        "Whenever a spell is countered by a spell or ability you control",
        "Whenever a spell or ability you control counters a spell",
    );
    line = line
        .replace("that object's power", "its power")
        .replace("that object's toughness", "its toughness");
    line = reflexive_when_you_do(&line);
    line = shared_trigger_subject(&line);
    line = sacrifice_cost_without_you_control(&line);
    line = line.replace(
        "You may exile a nonland card from target opponent's hand. Its owner may play that card for as long as it remains exiled.",
        "You may exile a nonland card from it. For as long as that card remains exiled, its owner may play it.",
    );
    line = return_the_exiled_cards(&line);
    line = line.replace(
        "assimilate that creature card in exile",
        "assimilate a creature card exiled with it",
    );
    for (from, to) in [
        ("if you haven't cast a spell this turn", "if you didn't cast a spell this turn"),
        (
            "if you haven't cast a creature spell this turn",
            "if you didn't cast a creature spell this turn",
        ),
        (
            "if that player hasn't cast a spell this turn",
            "if that player didn't cast a spell this turn",
        ),
        (
            "if that player hasn't cast a creature spell this turn",
            "if that player didn't cast a creature spell this turn",
        ),
    ] {
        line = line.replace(from, to);
    }
    line = plural_pump_subject(&line);
    for attached in ["equipped", "enchanted"] {
        line = line.replace(
            &format!("on {attached} creature if {attached} creature is "),
            &format!("on {attached} creature if it's "),
        );
    }
    line = line.replace("library for exactly ", "library for ");
    line = line
        .replace(", for each card searched for this way, put it", ", put it")
        .replace(" and for each card searched for this way, you put it", " and put it");
    line = line.replace(", then shuffle your library.", ", then shuffle.");
    if let Some(rest) = line.strip_prefix("• You draw ") {
        line = format!("• Draw {rest}");
    }
    line = shared_keyword_condition(&line);
    // "At the beginning of each upkeep" when nothing refers back to the
    // upkeep's player.
    if line.starts_with("At the beginning of each player's upkeep, if ")
        && !line.contains("that player")
        && !line.contains("their ")
    {
        line = line.replacen("each player's upkeep", "each upkeep", 1);
    }
    if line.starts_with("Whenever a player attacks") {
        line = line.replacen(", the attacking player ", ", that attacking player ", 1);
    }
    line = event_amount_where_x(&line);
    line = dead_permanent_conditions(&line);
    line = you_may_have(&line);
    line = each_creature_without_battlefield(&line);
    line = commander_name(&line, &def.card.name);
    line = mode_source_damage(&line, def);
    line = bare_self_reference(&line, def);
    // The controller returns the card itself; Oracle leaves "you" implicit.
    line = line
        .replace(": You return this card ", ": Return this card ")
        .replace(", you return this card ", ", return this card ");
    if let Some(rest) = line.strip_prefix("You return this card ") {
        line = format!("Return this card {rest}");
    }
    line = line.replace(
        "if there are no creatures on the battlefield",
        "if no creatures are on the battlefield",
    );
    line = reanimation_aura_surface(&line);
    // The ability's controller acts in the imperative, unless another
    // player acted first in the sentence ("that player reveals their hand,
    // you choose a nonland card from it": Devour Intellect).
    let other_actor_first = line.split(", you choose ").next().is_some_and(|head| {
        let clause = head.rsplit(". ").next().unwrap_or(head);
        clause.contains(" their ") || clause.contains("player ") || clause.contains("opponent ")
    });
    if !other_actor_first {
        line = line.replace(", you choose ", ", choose ");
    }
    line = line
        .replace(". Then you choose ", ". Then choose ")
        .replace(", you sacrifice ", ", sacrifice ");
    // A trigger's subject is any one object of the kind.
    line = line
        .replace("Whenever creature ", "Whenever a creature ")
        .replace("Whenever permanent ", "Whenever a permanent ");
    line = collapse_counter_choice(&line);
    line = trailing_where_x(&line);
    line = dangling_where_x(&line);
    // A state of the referenced permanent is an adjective predicate: "if
    // it's suspected", not a reclassified noun ("a suspected permanent").
    for state in ["suspected", "equipped", "tapped", "untapped", "attacking", "blocking", "enchanted"] {
        for article in ["a", "an"] {
            line = line
                .replace(
                    &format!("if it's {article} {state} permanent "),
                    &format!("if it's {state} "),
                )
                .replace(
                    &format!("if it's {article} {state} permanent,"),
                    &format!("if it's {state},"),
                )
                .replace(
                    &format!("If it's {article} {state} permanent,"),
                    &format!("If it's {state},"),
                )
                .replace(
                    &format!("If it's {article} {state} permanent "),
                    &format!("If it's {state} "),
                );
        }
    }
    line = line.replace("if it's attacking attacking ", "if it's attacking ").replace("If it's attacking attacking ", "If it's attacking ");
    line = audit_ability_line_rephrasings_r5t(line);
    line = audit_resync_line_rephrasings(line);
    line = revealed_hand_choice_from_it_r5f(&line);
    line = damage_dealt_to_this_r5f(&line);
    line = alternative_cost_rather_than_r5f(&line);
    line = each_permanent_doesnt_untap_r5f(&line);
    line = line.replace(" can't attack as long as it's an extra turn.", " can't attack during extra turns.");
    line = trigger_event_gerunds_r5f(&line);
    line = serial_predicate_list_r5f(&line);
    line = self_zone_references_r5f(&line, def);
    line = keyword_non_mana_cost_dash_r5f(&line);
    line = line.replace(", Octopus, ", ", Octopuses, ");
    // The replaced life gain is yours, so you draw (Nefarious Lich).
    line = line.replace("If you would gain life, that player draws ", "If you would gain life, draw ");
    // Basic land types are proper subtypes (CR 205.3i): "are Plains",
    // "control no Islands", "a snow Mountain".
    for basic in ["Plains", "Islands", "Swamps", "Mountains", "Forests", "Island", "Swamp", "Mountain", "Forest"] {
        let lower = basic.to_ascii_lowercase();
        for lead in [" are ", " no ", " snow "] {
            for end in [".", " ", ","] {
                line = line.replace(&format!("{lead}{lower}{end}"), &format!("{lead}{basic}{end}"));
            }
        }
    }
    line = line.replace(
        "Creatures you control can't be blocked by more than one creature",
        "Each creature you control can't be blocked by more than one creature",
    );
    // CR 609.4b mana-type permission, in Oracle's passive form: "mana of
    // any type can be spent to cast that spell".
    line = line
        .replace(
            "Players may spend mana as though it were mana of any type to activate abilities of this creature",
            "Mana of any type can be spent to activate this creature's abilities",
        )
        .replace(
            "You may spend mana as though it were mana of any type to cast ",
            "Mana of any type can be spent to cast ",
        )
        .replace(
            "you may spend mana as though it were mana of any type to cast ",
            "mana of any type can be spent to cast ",
        );
    // A self-replacement that deals more damage to the same target leaves
    // the recipient implicit: "deals 5 damage instead if a creature died
    // this turn" (Brimstone Volley, Galvanic Blast).
    line = line.replace(
        " damage to that permanent or player instead if ",
        " damage instead if ",
    );
    line
}

/// A keyword cost with a non-mana part follows an em dash (CR 702 keyword
/// costs): "Squad\u{2014}Exile four cards from your graveyard.",
/// "Eternalize\u{2014}{3}{U}{U}, Discard a card."
fn keyword_non_mana_cost_dash_r5f(line: &str) -> String {
    const KEYWORDS: [&str; 20] = [
        "Squad", "Eternalize", "Embalm", "Bestow", "Escape", "Unearth", "Disturb", "Encore",
        "Plot", "Foretell", "Madness", "Buyback", "Kicker", "Flashback", "Echo", "Cycling",
        "Evoke", "Blitz", "Dash", "Ninjutsu",
    ];
    let Some((keyword, cost)) = line.split_once(' ') else {
        return line.to_string();
    };
    if !KEYWORDS.contains(&keyword) || cost.contains(['\u{2014}', '(', '"', ':']) || cost.contains(". ") {
        return line.to_string();
    }
    let cost = cost.trim_end_matches('.');
    let non_mana = cost.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
        || cost
            .split(", ")
            .skip(1)
            .any(|part| part.chars().next().is_some_and(|ch| ch.is_ascii_uppercase()));
    if !non_mana || cost.is_empty() {
        return line.to_string();
    }
    format!("{keyword}\u{2014}{cost}.")
}

/// The card's own location, read about the card itself: "if Edgar Markov
/// is in the command zone or on the battlefield" (eminence, CR 207.2c);
/// "As long as this creature is on the battlefield, it has ...".
fn self_zone_references_r5f(line: &str, def: &CardDefinition) -> String {
    let name = def.card.name.as_str();
    let subject = super::subject_for_card(&def.card);
    let mut line = line
        .replace(
            "this card is in the command zone or this object is on the battlefield",
            &format!("{name} is in the command zone or on the battlefield"),
        )
        .replace(
            "this card is in your graveyard or this object is on the battlefield",
            &format!("{name} is in your graveyard or on the battlefield"),
        )
        .replace(" as long as a this entered this turn.", &format!(" as long as {subject} entered this turn."))
        .replace(" unless a this creature entered ", " unless this creature entered ");
    // "This creature has annihilator X, where X is the number of +1/+1
    // counters on this source": the source is the subject itself.
    if line.starts_with("This ") && line.ends_with(" counters on this source.") {
        line = format!("{} counters on it.", line.trim_end_matches(" counters on this source."));
    }
    if let Some(body) = line.strip_suffix(" as long as this object is on the battlefield.")
        && let Some(rest) = body.strip_prefix(&format!("{} ", super::capitalize_first(subject)))
        && (rest.starts_with("has ") || rest.starts_with("is "))
        && !rest.contains(" as long as ")
    {
        line = format!("As long as {subject} is on the battlefield, it {rest}.");
    }
    line
}

/// Three or more predicates of one subject read as Oracle's serial list:
/// "Enchanted creature gets +1/+1, has deathtouch, and is goaded."
fn serial_predicate_list_r5f(line: &str) -> String {
    const VERBS: [&str; 15] = [
        "gets", "get", "has", "have", "is", "are", "can't", "loses", "lose", "must", "doesn't",
        "don't", "attacks", "blocks", "can",
    ];
    let Some(body) = line.strip_suffix('.') else {
        return line.to_string();
    };
    if body.contains(['"', ':', '\u{27e6}'])
        || body.contains(". ")
        || body.contains(", and ")
        || ["When", "At ", "If ", "{", "\u{2022}"].iter().any(|head| body.starts_with(head))
    {
        return line.to_string();
    }
    let parts = body.split(" and ").collect::<Vec<_>>();
    if parts.len() < 3 {
        return line.to_string();
    }
    let Some(first_verb) = VERBS
        .iter()
        .filter_map(|verb| parts[0].find(&format!(" {verb} ")).map(|at| (at, *verb)))
        .min_by_key(|(at, _)| *at)
        .map(|(_, verb)| verb)
    else {
        return line.to_string();
    };
    let mut verbs = vec![first_verb];
    for part in &parts[1..] {
        let Some(verb) = VERBS.iter().find(|verb| part.split(' ').next() == Some(**verb)) else {
            return line.to_string();
        };
        if verbs.contains(verb) || part.contains(',') {
            return line.to_string();
        }
        verbs.push(*verb);
    }
    let (last, init) = parts.split_last().unwrap();
    format!("{}, and {last}.", init.join(", "))
}

/// Events that cause abilities to trigger read as gerunds (CR 603.2):
/// "Creatures entering or dying don't cause abilities to trigger", "If
/// turning a face-down permanent face up causes ...", "If a legendary
/// permanent or an artifact entering or leaving the battlefield causes ...".
fn trigger_event_gerunds_r5f(line: &str) -> String {
    let mut line = line.replace(
        "A creature entering or a creature dies doesn't cause abilities to trigger.",
        "Creatures entering or dying don't cause abilities to trigger.",
    );
    line = line.replace(
        "If face-down permanent being turned face up causes ",
        "If turning a face-down permanent face up causes ",
    );
    if let Some(rest) = line.strip_prefix("If a ")
        && let Some((subject, tail)) = rest.split_once(" entering or a ")
        && let Some(after) = tail.strip_prefix(subject)
        && let Some(after) = after.strip_prefix(" leaves the battlefield causes ")
    {
        let subject = match subject.split_once(" or ") {
            Some((first, second))
                if !second.starts_with("a ")
                    && !second.starts_with("an ")
                    && second.starts_with(['a', 'e', 'i', 'o', 'u']) =>
            {
                format!("{first} or an {second}")
            }
            _ => subject.to_string(),
        };
        line = format!("If a {subject} entering or leaving the battlefield causes {after}");
    }
    line
}

/// "Permanents with an hourglass counter on it don't untap ...": a subject
/// read one object at a time ("on it") is Oracle's "Each permanent with an
/// hourglass counter on it doesn't untap during its controller's untap step."
fn each_permanent_doesnt_untap_r5f(line: &str) -> String {
    const TAIL: &str = " don't untap during their controllers' untap steps.";
    let Some(subject) = line.strip_suffix(TAIL) else {
        return line.to_string();
    };
    let singular = if let Some(rest) = subject.strip_prefix("Lands with an activated ability that aren't mana ability") {
        rest.is_empty().then(|| "land with an activated ability that isn't a mana ability".to_string())
    } else {
        ["Permanents", "Creatures", "Lands", "Artifacts"]
            .iter()
            .find_map(|plural| {
                let rest = subject.strip_prefix(plural)?.strip_prefix(" with ")?;
                rest.ends_with(" on it").then(|| {
                    format!("{} with {rest}", plural.trim_end_matches('s').to_ascii_lowercase())
                })
            })
    };
    match singular {
        Some(singular) => format!("Each {singular} doesn't untap during its controller's untap step."),
        None => line.to_string(),
    }
}

/// "You may cast creature spells by paying {0} rather than their mana
/// costs." in Oracle's order (CR 118.9): "You may pay {0} rather than pay
/// the mana cost for creature spells you cast."
fn alternative_cost_rather_than_r5f(line: &str) -> String {
    let Some(body) = line
        .strip_prefix("You may cast ")
        .and_then(|rest| rest.strip_suffix(" rather than their mana costs."))
    else {
        return line.to_string();
    };
    let Some((spells, cost)) = body.split_once(" spells by paying ") else {
        return line.to_string();
    };
    if spells.contains(',')
        || spells.contains(" with ")
        || cost.contains("stated effect")
        || cost.contains('\u{27e6}')
    {
        return line.to_string();
    }
    let spells = match spells {
        "card" => "spells".to_string(),
        other => format!("{other} spells"),
    };
    let payment = if cost.starts_with('{') || cost.chars().next().is_some_and(|ch| ch.is_ascii_lowercase()) {
        format!("pay {cost}")
    } else {
        super::lowercase_first(cost)
    };
    format!("You may {payment} rather than pay the mana cost for {spells} you cast.")
}

/// A replacement of damage dealt to the permanent itself that never refers
/// back to the source names only the damage: "If damage would be dealt to
/// this creature, put that many +1/+1 counters on it instead."
fn damage_dealt_to_this_r5f(line: &str) -> String {
    for subject in ["this creature", "this permanent", "this enchantment", "this artifact"] {
        let Some(rest) = line.strip_prefix(&format!("If a source would deal damage to {subject}, ")) else {
            continue;
        };
        if rest.contains("source") || rest.contains("it deals") || rest.contains("its controller") {
            return line.to_string();
        }
        let rest = rest
            .replace(&format!(" counters on {subject} instead."), " counters on it instead.");
        return format!("If damage would be dealt to {subject}, {rest}");
    }
    line.to_string()
}

/// "Target opponent reveals their hand. You may choose a nonland card
/// revealed this way." picks from the revealed hand: "... card from it."
fn revealed_hand_choice_from_it_r5f(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(pos) = rest.find("reveals their hand. ") {
        let split = pos + "reveals their hand. ".len();
        out.push_str(&rest[..split]);
        rest = &rest[split..];
        let sentence_end = rest.find(". ").map(|end| end + 1).unwrap_or(rest.len());
        let sentence = &rest[..sentence_end];
        if (sentence.starts_with("You may choose ") || sentence.starts_with("You choose "))
            && let Some(head) = sentence
                .strip_suffix(" revealed this way.")
                .or_else(|| sentence.strip_suffix(" revealed this way"))
        {
            out.push_str(head);
            out.push_str(" from it");
            if sentence.ends_with('.') {
                out.push('.');
            }
            rest = &rest[sentence_end..];
        }
    }
    out.push_str(rest);
    out
}

fn mentions_x(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.iter().enumerate().any(|(index, &byte)| {
        byte == b'X'
            && (index == 0 || !bytes[index - 1].is_ascii_alphabetic())
            && bytes.get(index + 1).is_none_or(|next| !next.is_ascii_lowercase())
    })
}

/// A "where X is ..." definition with no X before it: the comparison already
/// names the value ("with power less than this creature's power"), or the
/// count it defines reads "that many" ("tap up to X target nonland
/// permanents, where X is ...").
fn dangling_where_x(line: &str) -> String {
    const WHERE: &str = ", where X is ";
    let Some(at) = line.find(WHERE) else {
        return line.to_string();
    };
    let head = &line[..at];
    if mentions_x(head) {
        return line.to_string();
    }
    let rest = &line[at + WHERE.len()..];
    let Some(end) = rest.find('.') else {
        return line.to_string();
    };
    if mentions_x(&rest[end..]) {
        return line.to_string();
    }
    if let Some(many) = head.find("that many ") {
        return format!("{}X {}", &head[..many], &line[many + "that many ".len()..]);
    }
    let definition = &rest[..end];
    if definition.is_empty()
        || [" and ", " with ", " then ", " or ", ",", "\""]
            .iter()
            .any(|word| definition.contains(word))
    {
        return line.to_string();
    }
    format!("{head}{}", &rest[end..])
}

/// One X shared by two clauses of a sentence is defined once, after both:
/// "draw X cards, then put up to X land cards ..., where X is the sacrificed
/// creature's power".
fn trailing_where_x(line: &str) -> String {
    const WHERE: &str = ", where X is ";
    const THEN: &str = ", then ";
    let Some(at) = line.find(WHERE) else {
        return line.to_string();
    };
    let rest = &line[at + WHERE.len()..];
    let Some(then_at) = rest.find(THEN) else {
        return line.to_string();
    };
    let definition = &rest[..then_at];
    if definition.contains(',') || definition.contains('.') || definition.is_empty() {
        return line.to_string();
    }
    let after = &rest[then_at + THEN.len()..];
    let Some(end) = after.find('.') else {
        return line.to_string();
    };
    let sentence = &after[..end];
    if !sentence.split(' ').any(|word| word == "X") {
        return line.to_string();
    }
    let clause = format!("{WHERE}{definition}");
    let sentence = if sentence.ends_with(&clause) {
        sentence.to_string()
    } else {
        format!("{sentence}{clause}")
    };
    format!("{}{THEN}{sentence}{}", &line[..at], &after[end..])
}

/// A choice among three or more counter kinds names the noun once: "your
/// choice of a +1/+1, first strike, vigilance, or menace counter".
fn collapse_counter_choice(line: &str) -> String {
    const MARK: &str = "your choice of ";
    const OR: &str = ", or a ";
    let Some(at) = line.find(MARK) else {
        return line.to_string();
    };
    let start = at + MARK.len();
    let rest = &line[start..];
    let Some(or_at) = rest.find(OR) else {
        return line.to_string();
    };
    let head = &rest[..or_at];
    let after_or = &rest[or_at + OR.len()..];
    let Some(last_end) = after_or.find(" counter") else {
        return line.to_string();
    };
    let last = &after_or[..last_end];
    if last.contains(',') || last.is_empty() {
        return line.to_string();
    }
    let mut names = Vec::new();
    for item in head.split(", ") {
        let Some(name) = item
            .strip_prefix("a ")
            .or_else(|| item.strip_prefix("an "))
            .and_then(|item| item.strip_suffix(" counter"))
        else {
            return line.to_string();
        };
        if name.is_empty() || name.contains(" counter") {
            return line.to_string();
        }
        names.push(name);
    }
    if names.len() < 2 {
        return line.to_string();
    }
    let article = if names[0].starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" };
    format!(
        "{}{MARK}{article} {}, or {last} counter{}",
        &line[..at],
        names.join(", "),
        &after_or[last_end + " counter".len()..]
    )
}

/// Reanimation Auras (Animate Dead, Dance of the Dead):
/// the enchanted object is still a card in the graveyard when it is
/// returned, and the granted enchant ability is quoted in lower case.
fn reanimation_aura_surface(line: &str) -> String {
    const LOSES: &str = "it loses \"enchant creature card in a graveyard\" and gains \"Enchant creature put onto the battlefield with this Aura.\"";
    if !line.contains(LOSES) {
        return line.to_string();
    }
    line.replace(
        LOSES,
        "it loses \"enchant creature card in a graveyard\" and gains \"enchant creature put onto the battlefield with this Aura.\"",
    )
    .replace(" Return enchanted creature to the battlefield ", " Return enchanted creature card to the battlefield ")
    .replace(" Put enchanted creature onto the battlefield ", " Put enchanted creature card onto the battlefield ")
    .replace(
        "When this Aura leaves the battlefield, its controller sacrifices it.",
        "When this Aura leaves the battlefield, that creature's controller sacrifices it.",
    )
}

/// The card's own self reference ("this land", "this Spacecraft", a spell's
/// name) where the reading left a bare "this": "As this land enters",
/// "Then this Spacecraft deals 4 damage", "return this card from your
/// graveyard". Quoted abilities name their own (token) source and are left
/// alone.
fn bare_self_reference(line: &str, def: &CardDefinition) -> String {
    const VERBS: [&str; 5] = ["deals ", "enters", "gets ", "has ", "is on the battlefield"];
    if !line.contains("this ") && !line.contains("This ") {
        return line.to_string();
    }
    let spell = def.card.is_instant() || def.card.is_sorcery();
    if spell && (def.card.name.contains("//") || def.card.name.is_empty()) {
        return line.to_string();
    }
    let subject = if spell {
        def.card.name.clone()
    } else {
        super::subject_for_card(&def.card).to_string()
    };
    let mut out = String::with_capacity(line.len() + 16);
    let mut rest = line;
    while !rest.is_empty() {
        let next = [rest.find("this "), rest.find("This ")].into_iter().flatten().min();
        let Some(at) = next else {
            out.push_str(rest);
            break;
        };
        let (before, from_this) = rest.split_at(at);
        out.push_str(before);
        let word_start = out
            .chars()
            .last()
            .is_none_or(|ch| !ch.is_alphanumeric() && ch != '\'');
        let outside_quotes = out.matches('"').count() % 2 == 0;
        let after = &from_this["this ".len()..];
        let capital = from_this.starts_with('T');
        if word_start && outside_quotes {
            if VERBS.iter().any(|verb| after.starts_with(verb)) {
                let phrase = if capital || spell {
                    super::capitalize_first(&subject)
                } else {
                    subject.clone()
                };
                out.push_str(&phrase);
                out.push(' ');
                rest = after;
                continue;
            }
            if !capital
                && after.starts_with("from ")
                && ["return ", "put ", "exile "].iter().any(|verb| out.ends_with(verb))
            {
                out.push_str("this card ");
                rest = after;
                continue;
            }
        }
        out.push_str(&from_this[..1]);
        rest = &from_this[1..];
    }
    out
}

fn starts_uppercase(word: &str) -> bool {
    word.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
}

/// "a basic or Desert card card" -> "a basic land card or a Desert card"
/// (the union of basic lands and one land subtype).
fn basic_land_or_type_card(line: &str) -> String {
    const MARK: &str = "basic or ";
    const TAIL: &str = " card card";
    let mut out = line.to_string();
    let mut from = 0;
    while let Some(at) = out[from..].find(MARK).map(|offset| from + offset) {
        let start = at + MARK.len();
        let rest = &out[start..];
        let Some(word_end) = rest.find(' ') else {
            break;
        };
        let word = rest[..word_end].to_string();
        if starts_uppercase(&word) && rest[word_end..].starts_with(TAIL) {
            let article = if word.starts_with(['A', 'E', 'I', 'O', 'U']) { "an" } else { "a" };
            let replacement = format!("basic land card or {article} {word} card");
            let end = start + word_end + TAIL.len();
            out.replace_range(at..end, &replacement);
            from = at + replacement.len();
        } else {
            from = start;
        }
    }
    out
}

/// Several targets each receive the pump: "up to two target creatures each
/// get +1/+0".
fn each_of_several_targets_get(line: &str) -> String {
    const COUNTS: [&str; 9] = ["two", "three", "four", "five", "six", "seven", "X", "of", "many"];
    const TAILS: [&str; 4] = [
        "creatures",
        "creatures you control",
        "creatures an opponent controls",
        "creatures you don't control",
    ];
    let mut out = line.to_string();
    let mut from = 0;
    while let Some(at) = out[from..].find("target ").map(|offset| from + offset) {
        from = at + "target ".len();
        let previous = out[..at].trim_end().rsplit(' ').next().unwrap_or("");
        if !COUNTS.contains(&previous) {
            continue;
        }
        let Some(get_at) = out[from..].find(" get ").map(|offset| from + offset) else {
            break;
        };
        let noun = &out[from..get_at];
        if noun.contains(',') || noun.contains('.') || noun.split(' ').count() > 5 {
            continue;
        }
        if !TAILS.iter().any(|tail| noun.ends_with(tail)) {
            continue;
        }
        out.insert_str(get_at, " each");
        from = get_at + " each get ".len();
    }
    out
}

/// "if there are three or more ribbon counters on it, remove all ribbon
/// counters from it" -> "remove those counters".
fn remove_those_counters(line: &str) -> String {
    const MARK: &str = "remove all ";
    const TAIL: &str = " counters from it";
    let mut out = line.to_string();
    let mut from = 0;
    while let Some(at) = out[from..].find(MARK).map(|offset| from + offset) {
        from = at + MARK.len();
        let Some(tail_at) = out[from..].find(TAIL).map(|offset| from + offset) else {
            break;
        };
        let kind = out[from..tail_at].to_string();
        if kind.is_empty() || kind.contains(' ') {
            continue;
        }
        let condition = format!(" or more {kind} counters on ");
        let head = &out[..at];
        let Some(cond_at) = head.rfind(&condition) else {
            continue;
        };
        let between = &head[cond_at + condition.len()..];
        if !head[..cond_at].contains("if there are ") || between.contains('.') || !between.ends_with(", ") {
            continue;
        }
        let end = tail_at + TAIL.len();
        out.replace_range(at..end, "remove those counters");
        from = at + "remove those counters".len();
    }
    out
}

/// "a spell that targets only a single creature" (CR 115.10 single target).
fn targets_only_a_single(line: &str) -> String {
    if !line.contains("targets only a") {
        return line.to_string();
    }
    let mut out = line.to_string();
    for (from, to) in [("targets only an ", "targets only a single "), ("targets only a ", "targets only a single ")] {
        let mut start = 0;
        while let Some(at) = out[start..].find(from).map(|offset| start + offset) {
            let rest = &out[at + from.len()..];
            if rest.starts_with("single ") {
                start = at + from.len();
                continue;
            }
            out.replace_range(at..at + from.len(), to);
            start = at + to.len();
        }
    }
    out
}

/// A reflexive trigger on the sacrifice just made reads "When you do".
fn reflexive_when_you_do(line: &str) -> String {
    const MARK: &str = ". When you sacrifice one or more ";
    let Some(at) = line.find(MARK) else {
        return line.to_string();
    };
    let rest = &line[at + MARK.len()..];
    let Some((noun, tail)) = rest.split_once(" this way, ") else {
        return line.to_string();
    };
    if noun.contains(' ') {
        return line.to_string();
    }
    format!("{}. When you do, {tail}", &line[..at])
}

/// "Whenever X enters or X attacks" -> "Whenever X enters or attacks".
fn shared_trigger_subject(line: &str) -> String {
    let (prefix, rest) = if let Some(rest) = line.strip_prefix("Whenever ") {
        ("Whenever ", rest)
    } else if let Some(rest) = line.strip_prefix("When ") {
        ("When ", rest)
    } else {
        return line.to_string();
    };
    let Some(clause_end) = rest.find(',') else {
        return line.to_string();
    };
    let clause = &rest[..clause_end];
    for verb in [" enters or ", " attacks or ", " dies or ", " blocks or ", " becomes blocked or "] {
        let Some(at) = clause.find(verb) else {
            continue;
        };
        let subject = &clause[..at];
        if subject.is_empty() {
            continue;
        }
        let after = &rest[at + verb.len()..];
        if let Some(second) = after.strip_prefix(subject).and_then(|tail| tail.strip_prefix(' ')) {
            return format!("{prefix}{subject}{verb}{second}");
        }
    }
    line.to_string()
}

/// A sacrifice cost names only the kind of permanent: you can sacrifice
/// only permanents you control.
fn sacrifice_cost_without_you_control(line: &str) -> String {
    let Some(colon) = line.find(": ") else {
        return line.to_string();
    };
    let cost = &line[..colon];
    if cost.contains('"') || cost.starts_with("When") || cost.starts_with("At ") || cost.starts_with("• ") {
        return line.to_string();
    }
    let parts = cost
        .split(", ")
        .map(|part| match part.strip_prefix("Sacrifice ") {
            Some(object) => match object.strip_suffix(" you control") {
                Some(object) if !object.is_empty() => format!("Sacrifice {object}"),
                _ => part.to_string(),
            },
            None => part.to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("{parts}{}", &line[colon..])
}

/// "return all cards exiled with it to the battlefield" -> "return the
/// exiled cards to the battlefield".
fn return_the_exiled_cards(line: &str) -> String {
    const MARK: &str = "return all cards exiled with ";
    const TAIL: &str = " to the battlefield";
    let Some(at) = line.find(MARK) else {
        return line.to_string();
    };
    // Per-player returns keep "all cards exiled with it" (Parallax Wave).
    if line[..at].ends_with("for each player, ") {
        return line.to_string();
    }
    let rest = &line[at + MARK.len()..];
    let consumed = if rest.starts_with("it to the battlefield") {
        "it".len()
    } else if let Some(this) = rest.strip_prefix("this ") {
        match this.find(' ') {
            Some(end) if this[end..].starts_with(TAIL) => "this ".len() + end,
            _ => return line.to_string(),
        }
    } else {
        return line.to_string();
    };
    format!(
        "{}return the exiled cards{}",
        &line[..at],
        &rest[consumed..]
    )
}

/// "Each other creature you control gets +1/+1 until end of turn" ->
/// "Other creatures you control get +1/+1 until end of turn".
fn plural_pump_subject(line: &str) -> String {
    for each in ["Each ", "each "] {
        let mut from = 0;
        while let Some(at) = line[from..].find(each).map(|offset| from + offset) {
            from = at + each.len();
            if at > 0 && !line[..at].ends_with(' ') {
                continue;
            }
            let mut rest = &line[from..];
            let mut words = String::new();
            if let Some(tail) = rest.strip_prefix("other ") {
                words.push_str("other ");
                rest = tail;
            }
            for adjective in ["attacking ", "tapped ", "untapped ", "nontoken ", "token "] {
                if let Some(tail) = rest.strip_prefix(adjective) {
                    words.push_str(adjective);
                    rest = tail;
                    break;
                }
            }
            let Some((noun, tail)) = rest.split_once(' ') else {
                continue;
            };
            let Some(tail) = tail.strip_prefix("you control gets ") else {
                continue;
            };
            let Some((pump, after)) = tail.split_once(' ') else {
                continue;
            };
            if !(pump.starts_with('+') || pump.starts_with('-')) || !pump.contains('/') {
                continue;
            }
            if !after.starts_with("until end of turn") {
                continue;
            }
            let plural = if noun == "creature" {
                "creatures".to_string()
            } else if starts_uppercase(noun)
                && noun.chars().all(|ch| ch.is_ascii_alphabetic())
                && !noun.ends_with(['f', 'y', 's', 'x', 'h', 'i'])
            {
                format!("{noun}s")
            } else {
                continue;
            };
            let mut subject = format!("{words}{plural} you control get {pump} ");
            if each == "Each " {
                subject = super::capitalize_first(&subject);
            }
            return format!("{}{subject}{after}", &line[..at]);
        }
    }
    line.to_string()
}

/// "creatures you control gain flying until end of turn if a creature you
/// control has flying. The same is true for ..." (Odric, Lunarch Marshal).
fn shared_keyword_condition(line: &str) -> String {
    const MARK: &str = " until end of turn if you control a creature with ";
    if !line.contains("creatures you control gain ") || !line.contains(". The same is true") {
        return line.to_string();
    }
    let Some(at) = line.find(MARK) else {
        return line.to_string();
    };
    let rest = &line[at + MARK.len()..];
    let Some(end) = rest.find(". The same is true") else {
        return line.to_string();
    };
    let keyword = &rest[..end];
    format!(
        "{} until end of turn if a creature you control has {keyword}{}",
        &line[..at],
        &rest[end..]
    )
}

/// The trigger event's amount named by the event: "where X is the amount of
/// damage it dealt to that player", "where X is that excess damage".
fn event_amount_where_x(line: &str) -> String {
    const MARK: &str = ", where X is that much";
    if !line.contains(MARK) {
        return line.to_string();
    }
    let amount = if line.contains(" is dealt excess ") {
        "that excess damage".to_string()
    } else if line.starts_with("Whenever one or more ") && line.contains(" deal combat damage to a player,") {
        "the amount of damage those creatures dealt to that player".to_string()
    } else if let Some(subject) = line
        .strip_prefix("Whenever ")
        .and_then(|rest| rest.split_once(" deals combat damage to a player,"))
        .map(|(subject, _)| subject)
        .filter(|subject| !subject.contains(','))
    {
        let subject = if subject == "equipped creature" || subject == "enchanted creature" {
            "it"
        } else {
            subject
        };
        format!("the amount of damage {subject} dealt to that player")
    } else {
        return line.to_string();
    };
    line.replacen(MARK, &format!(", where X is {amount}"), 1)
}

/// Conditions on a permanent that just died or was destroyed / exiled from
/// the battlefield: it was necessarily a permanent, so only the rest of the
/// condition reads.
fn dead_permanent_conditions(line: &str) -> String {
    if !line.contains("t was a permanent ") && !line.contains("the amount of damage dealt to it this turn is ") {
        return line.to_string();
    }
    let mut out = line
        .replace("if it was a permanent without a counter on it", "if it had no counters on it")
        .replace("if it was a permanent without ", "if it didn't have ")
        .replace("if it was a permanent and it ", "if it ")
        .replace("If it was a permanent and it ", "If it ")
        .replace("if it was a permanent and the amount", "if the amount");
    const MARK: &str = "if the amount of damage dealt to it this turn is ";
    if let Some(at) = out.find(MARK) {
        let rest = &out[at + MARK.len()..];
        if let Some((amount, tail)) = rest.split_once(" or greater")
            && !amount.is_empty()
            && amount.chars().all(|ch| ch.is_ascii_digit())
        {
            out = format!(
                "{}if {amount} or more damage was dealt to it this turn{tail}",
                &out[..at]
            );
        }
    }
    out
}

/// "you may your life total becomes ..." -> "you may have your life total
/// become ...": an optional effect another object or player performs.
fn you_may_have(line: &str) -> String {
    const SUBJECTS: [&str; 7] = [
        "your life total ",
        "it ",
        "Its controller ",
        "its controller ",
        "defending player ",
        "enchanted creature ",
        "equipped creature ",
    ];
    let mut from = 0;
    while let Some(at) = line[from..].find("you may ").map(|offset| from + offset) {
        let start = at + "you may ".len();
        from = start;
        let rest = &line[start..];
        for subject in SUBJECTS {
            let Some(after) = rest.strip_prefix(subject) else {
                continue;
            };
            let Some((verb, tail)) = after.split_once(' ') else {
                continue;
            };
            if verb.len() <= 3 || verb == "has" || !verb.ends_with('s') || verb.ends_with("ss") {
                continue;
            }
            let base = &verb[..verb.len() - 1];
            let subject = if subject.starts_with("Its ") {
                "its controller "
            } else {
                subject
            };
            return format!("{}you may have {subject}{base} {tail}", &line[..at]);
        }
    }
    line.to_string()
}

/// "each other creature on the battlefield" is "each other creature".
fn each_creature_without_battlefield(line: &str) -> String {
    const MARK: &str = "on each ";
    const TAIL: &str = "creature on the battlefield";
    let Some(at) = line.find(MARK) else {
        return line.to_string();
    };
    let rest = &line[at + MARK.len()..];
    let qualifier_len = if rest.starts_with(TAIL) {
        0
    } else if let Some(tail) = rest.strip_prefix("other ") {
        if tail.starts_with(TAIL) { "other ".len() } else { return line.to_string() }
    } else if rest.starts_with("non") {
        match rest.find(' ') {
            Some(end) if rest[end + 1..].starts_with(TAIL) => end + 1,
            _ => return line.to_string(),
        }
    } else {
        return line.to_string();
    };
    let cut = at + MARK.len() + qualifier_len + "creature".len();
    format!("{}{}", &line[..cut], &line[cut + " on the battlefield".len()..])
}

/// "Will Kenrith can be your commander": the rule names the card.
fn commander_name(line: &str, name: &str) -> String {
    if let Some(rest) = line.strip_prefix("This ")
        && let Some(kind) = rest.strip_suffix(" can be your commander.")
        && !kind.contains(' ')
        && !name.is_empty()
    {
        return format!("{name} can be your commander.");
    }
    line.to_string()
}

/// A mode dealing damage names its source, as the card's other abilities
/// do: "• Collective Defiance deals 4 damage", "• This creature deals 4
/// damage".
fn mode_source_damage(line: &str, def: &CardDefinition) -> String {
    let Some(rest) = line.strip_prefix("• Deal ") else {
        return line.to_string();
    };
    let subject = if def.card.is_instant() || def.card.is_sorcery() {
        if def.card.name.contains("//") || def.card.name.is_empty() {
            return line.to_string();
        }
        def.card.name.clone()
    } else {
        super::capitalize_first(super::subject_for_card(&def.card))
    };
    format!("• {subject} deals {rest}")
}

/// Round-5 re-phrasings of triggered and activated ability lines. Each keys
/// on the structure of the audit reading itself and was checked against the
/// Oracle corpus (the new form is the majority for the code path reaching
/// it).
fn audit_ability_line_rephrasings_r5t(line: String) -> String {
    let mut line = coin_flip_loss_branch_r5t(&line);
    // CR 702.62b: a suspended card is a card with suspend in exile with a
    // time counter on it.
    line = line.replace(
        "if this card is exiled and this creature has a time counter on it",
        "if this card is suspended",
    );
    line = line.replace(
        ". Put a card from among them into your hand",
        ". Put one of them into your hand",
    );
    line = cast_spell_trigger_referent_r5t(&line);
    line = base_pt_becomes_equal_r5t(&line);
    line = each_of_up_to_one_targets_r5t(&line);
    line = graveyard_card_has_keyword_r5t(&line);
    line = also_gains_after_target_pump_r5t(&line).unwrap_or(line);
    line = also_gain_after_group_pump_r5t(&line).unwrap_or(line);
    // A follow-up test of the card just exiled or revealed: "If it's a land
    // card, ..." (Kianne, The Creation of Avacyn).
    line = line.replace(". Then if it's ", ". If it's ");
    line = line
        .replace(" card with kicker from ", " card with a kicker ability from ")
        .replace(" card with cycling from ", " card with a cycling ability from ");
    line = discarded_card_mana_value_r5t(&line);
    line = chosen_name_r5t(&line);
    line = line.replace(
        "When you unlock this door, this enchantment ",
        "When you unlock this door, this Room ",
    );
    line = line.replace("if this creature has power ", "if this creature's power is ");
    line = negated_condition_surfaces_r5t(&line);
    line = revealed_this_way_present_r5t(&line);
    line = line.replace("if you control five or more other ", "if you control at least five other ");
    for count in ["two", "three", "X"] {
        line = line.replace(
            &format!("up to {count} target creatures have base "),
            &format!("up to {count} target creatures each have base "),
        );
    }
    // A granted attack/block tax: within the granted ability "you" is the
    // creature's controller (Whipgrass Entangler).
    line = line.replace(
        "\"This creature source can't attack or block unless you pay ",
        "\"This creature can't attack or block unless its controller pays ",
    );
    line = shared_gain_keyword_list_r5t(&line);
    // CR 702.22c: losing "bands with other" for an unrestricted quality is
    // losing every such ability (Shelkin Brownie, Tolaria).
    line = line
        .replace(
            "loses bands with other permanent ",
            "loses all \"bands with other\" abilities ",
        )
        .replace(
            "loses banding and bands with other permanent ",
            "loses banding and all \"bands with other\" abilities ",
        );
    // Subtypes keep their capital: "a snow Mountain", "enchanted Equipment",
    // "that non-Wall creature"; Eldrazi is its own plural.
    for basic in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
        line = line.replace(
            &format!(" snow {}", basic.to_ascii_lowercase()),
            &format!(" snow {basic}"),
        );
    }
    line = line
        .replace("enchanted equipment", "enchanted Equipment")
        .replace("that non-wall creature", "that non-Wall creature")
        .replace("Eldrazis", "Eldrazi");
    line = line
        .replace(
            "if you've drawn two or more cards this turn",
            "if you drew two or more cards this turn",
        )
        .replace(
            "If the attacking player does, the attacking player draws a card.",
            "If the player does, they draw a card.",
        )
        .replace(
            "If the number of +1/+1 counters put on creatures this way is 2 or greater, you draw a card.",
            "If you put two or more +1/+1 counters on creatures this way, draw a card.",
        );
    // The ability's controller discards / draws in the imperative ("Then
    // discard a card unless ...", "If it's a creature card, draw a card");
    // an alternative cost of the unless clause continues the sentence in
    // lowercase ("unless you exile a historic card").
    line = line
        .replace(". Then you discard ", ". Then discard ")
        .replace(" enters, you discard ", " enters, discard ")
        .replace(", you discard your hand", ", discard your hand")
        .replace(" card, you draw a card.", " card, draw a card.")
        .replace("unless you Exile ", "unless you exile ")
        .replace("unless you You ", "unless you ");
    line = absent_counter_condition_r5t(&line);
    line = aura_entry_attached_condition_r5t(&line);
    line = line
        .replace(
            "if the permanent this Equipment is attached to is a permanent and the amount of combat damage dealt by the equipped creature to creature this turn is equal to 0,",
            "if equipped creature didn't deal combat damage to a creature this turn,",
        )
        .replace(
            "if the number of times this creature regenerated this turn is 1 or greater, if this creature regenerated this turn,",
            "if this creature regenerated this turn,",
        )
        .replace(
            "Then if the number of +1/+1 counters on it is equal to 4,",
            "Then if it has exactly four +1/+1 counters on it,",
        );
    line = looked_at_hand_choice_from_it_r5t(&line);
    line = same_player_then_draws_r5t(&line);
    // The player just named reveals and then acts: "that player puts it
    // into their hand" (Goblin Guide).
    if line.contains("defending player reveals ") {
        line = line.replace(
            ", defending player puts it into their hand",
            ", that player puts it into their hand",
        );
    }
    line
}

/// "if ... and this creature doesn't have a flying counter on it, put a
/// flying counter on it." (Inventive Wingsmith).
fn absent_counter_condition_r5t(line: &str) -> String {
    const LEAD: &str = "there are no ";
    let Some(at) = line.find(LEAD) else {
        return line.to_string();
    };
    let rest = &line[at + LEAD.len()..];
    let Some((kind, tail)) = rest.split_once(" counters on it, put a ") else {
        return line.to_string();
    };
    let expected = format!("{kind} counter on this creature");
    let Some(after) = tail.strip_prefix(&expected) else {
        return line.to_string();
    };
    if kind.is_empty() || kind.contains(' ') {
        return line.to_string();
    }
    format!(
        "{}this creature doesn't have a {kind} counter on it, put a {kind} counter on it{after}",
        &line[..at]
    )
}

/// "When this Aura enters, if enchanted creature is red, tap it." (Ray of
/// Frost), "if enchanted creature has toxic" (Maze's Mantle), "if enchanted
/// permanent is tapped" (Volition Reins).
fn aura_entry_attached_condition_r5t(line: &str) -> String {
    const LEAD: &str = "When this Aura enters, if the permanent this Aura is attached to is ";
    let Some(rest) = line.strip_prefix(LEAD) else {
        return line.to_string();
    };
    let Some((quality, tail)) = rest.split_once(", ") else {
        return line.to_string();
    };
    let condition = if let Some(color) = quality
        .strip_prefix("a ")
        .and_then(|rest| rest.strip_suffix(" creature"))
        .filter(|color| ["white", "blue", "black", "red", "green"].contains(color))
    {
        format!("enchanted creature is {color}")
    } else if let Some(keyword) = quality
        .strip_prefix("a creature with ")
        .filter(|keyword| !keyword.is_empty() && keyword.chars().all(|ch| ch.is_ascii_lowercase() || ch == ' '))
    {
        format!("enchanted creature has {keyword}")
    } else if quality == "a tapped permanent" {
        "enchanted permanent is tapped".to_string()
    } else {
        return line.to_string();
    };
    format!("When this Aura enters, if {condition}, {tail}")
}

/// "look at target player's hand. You may choose a nonland card from it."
/// (Vendilion Clique, Salt Vampire): the hand just looked at.
fn looked_at_hand_choice_from_it_r5t(line: &str) -> String {
    const FROM_HAND: &str = " from their hand.";
    let mut line = line.to_string();
    for looked in ["look at target player's hand. ", "look at target opponent's hand. "] {
        let Some(at) = line.find(looked) else {
            continue;
        };
        let start = at + looked.len();
        let Some(end) = line[start..].find(". ").map(|offset| start + offset + 1).or_else(|| {
            line[start..].ends_with('.').then_some(line.len())
        }) else {
            continue;
        };
        let sentence = &line[start..end];
        if (sentence.starts_with("You may choose ") || sentence.starts_with("You choose "))
            && sentence.ends_with(FROM_HAND)
        {
            let cut = end - FROM_HAND.len();
            line = format!("{} from it.{}", &line[..cut], &line[end..]);
        }
    }
    line
}

/// "that player exiles that card, then draws a card" (Salt Vampire): one
/// subject for both actions.
fn same_player_then_draws_r5t(line: &str) -> String {
    const THEN: &str = ", then that player draws a card";
    let Some(at) = line.find(THEN) else {
        return line.to_string();
    };
    let head = &line[..at];
    let sentence = head.rsplit(". ").next().unwrap_or(head);
    let clause = sentence.rsplit(", ").next().unwrap_or(sentence);
    if !clause.starts_with("that player ") {
        return line.to_string();
    }
    format!("{head}, then draws a card{}", &line[at + THEN.len()..])
}

/// "They gain trample, haste, and annihilator 1 until end of turn."
/// (Hideous Taskmaster): one grant of several keywords.
fn shared_gain_keyword_list_r5t(line: &str) -> String {
    const END: &str = " until end of turn.";
    const AND_GAIN: &str = " and gain ";
    let Some(mid) = line.find(AND_GAIN) else {
        return line.to_string();
    };
    let Some(start) = line[..mid].rfind(" gain ") else {
        return line.to_string();
    };
    let first = &line[start + " gain ".len()..mid];
    let after = mid + AND_GAIN.len();
    let Some(end) = line[after..].find(END).map(|offset| after + offset) else {
        return line.to_string();
    };
    let Some((second, third)) = line[after..end].split_once(" and ") else {
        return line.to_string();
    };
    let keyword_like = |text: &str| {
        !text.is_empty()
            && text.split(' ').count() <= 2
            && text
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == ' ')
    };
    if ![first, second, third].into_iter().all(|text| keyword_like(text)) {
        return line.to_string();
    }
    format!("{} gain {first}, {second}, and {third}{}", &line[..start], &line[end..])
}

/// Negated conditions in Oracle's words: "if it isn't being declared as an
/// attacker" (Verity Circle), "If you're not the monarch" (Court of
/// Vantress), "if you didn't lose life this turn" (Luminarch Ascension).
fn negated_condition_surfaces_r5t(line: &str) -> String {
    line.replace(
        "if it isn't the case that it's being declared as an attacker",
        "if it isn't being declared as an attacker",
    )
    .replace(
        "If it isn't the case that you're the monarch,",
        "If you're not the monarch,",
    )
    .replace(
        "if it isn't the case that you've lost life this turn",
        "if you didn't lose life this turn",
    )
    .replace(
        "if it isn't the case that they attacked you that turn",
        "if they didn't attack you that turn",
    )
    .replace(
        "if this creature didn't attack this turn and it isn't the case that this creature came under your control this turn,",
        "if this creature didn't attack or come under your control this turn,",
    )
}

/// "If a creature card is revealed this way, ..." (Runo Stromkirk, Deceiver
/// of Form): Oracle tests a card revealed by the same ability in the present
/// tense.
fn revealed_this_way_present_r5t(line: &str) -> String {
    const PAST: &str = " was revealed this way";
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(at) = rest.find(PAST) {
        let head = &rest[..at];
        let clause = head.rsplit(". ").next().unwrap_or(head);
        let clause = clause.rsplit(": ").next().unwrap_or(clause);
        let tested = (clause.starts_with("If a ") || clause.starts_with("If an "))
            && !clause.contains(',')
            && !clause.contains(" named ");
        out.push_str(head);
        out.push_str(if tested { " is revealed this way" } else { PAST });
        rest = &rest[at + PAST.len()..];
    }
    out.push_str(rest);
    out
}

/// "Flip a coin. If you win the flip, create ... . If you lose the flip,
/// ...": the negated result of the creation gated only by the won flip is
/// the lost flip (Wirefly Hive, Bottle of Suleiman).
fn coin_flip_loss_branch_r5t(line: &str) -> String {
    const WIN: &str = "If you win the flip, create ";
    const NEGATED: &str = ". If you didn't create a token this way, ";
    let Some(win) = line.find(WIN) else {
        return line.to_string();
    };
    let Some(end) = line[win..].find(". ") else {
        return line.to_string();
    };
    let at = win + end;
    if !line[at..].starts_with(NEGATED) {
        return line.to_string();
    }
    format!("{}. If you lose the flip, {}", &line[..at], &line[at + NEGATED.len()..])
}

/// A cast trigger's object is the spell: "If that spell's mana value is 5 or
/// greater" (Prismari Pianist), "with that spell's mana value" (Celestial
/// Kirin), "that spell's controller" (Tephraderm).
fn cast_spell_trigger_referent_r5t(line: &str) -> String {
    let Some(start) = line.find("Whenever ") else {
        return line.to_string();
    };
    if start != 0 && !line[..start].ends_with("\u{2014} ") {
        return line.to_string();
    }
    let head = line[start..].split(", ").next().unwrap_or_default();
    let cast_trigger = head.contains(" cast") && head.contains(" spell");
    if !cast_trigger && !head.contains("a spell deals damage") {
        return line.to_string();
    }
    let mut out = line.replace("that object's ", "that spell's ");
    if cast_trigger {
        out = out.replace("with the same mana value as it", "with that spell's mana value");
    }
    out
}

/// "this creature's base power and toughness become equal to that
/// creature's power and toughness": a dynamic base characteristic reads
/// "becomes equal to" (Shape Stealer, Wall of Tombstones, PuPu UFO).
fn base_pt_becomes_equal_r5t(line: &str) -> String {
    let mut line = line
        .replace(
            "have this creature have base power and toughness equal to ",
            "have this creature's base power and toughness become equal to ",
        )
        .replace(
            "this creature has base power and toughness equal to ",
            "this creature's base power and toughness become equal to ",
        );
    for stat in ["power", "toughness"] {
        line = replace_dynamic_base_stat_r5t(
            &line,
            &format!("have this creature have base {stat} "),
            &format!("have this creature's base {stat} become equal to "),
        );
        line = replace_dynamic_base_stat_r5t(
            &line,
            &format!("this creature has base {stat} "),
            &format!("this creature's base {stat} becomes equal to "),
        );
    }
    line
}

fn replace_dynamic_base_stat_r5t(line: &str, marker: &str, replacement: &str) -> String {
    let Some(at) = line.find(marker) else {
        return line.to_string();
    };
    let rest = &line[at + marker.len()..];
    let rest = rest.strip_prefix("equal to ").unwrap_or(rest);
    let digits = rest.bytes().take_while(|byte| byte.is_ascii_digit()).count();
    let dynamic = ["the ", "that ", "its "].iter().any(|lead| rest.starts_with(lead))
        || (digits > 0 && rest[digits..].starts_with(" plus "));
    if !dynamic {
        return line.to_string();
    }
    format!("{}{replacement}{rest}", &line[..at])
}

/// "it deals 1 damage to each of up to one target creature, up to one target
/// player, and/or up to one target planeswalker" (Volatile Arsonist): one
/// amount dealt to each of three optional targets.
fn each_of_up_to_one_targets_r5t(line: &str) -> String {
    const TARGET: &str = " damage to up to one target ";
    let Some(first) = line.find(TARGET) else {
        return line.to_string();
    };
    let head = &line[..first];
    let Some(deals) = head.rfind(" deals ") else {
        return line.to_string();
    };
    let amount = &head[deals + " deals ".len()..];
    if amount.is_empty() || amount.contains(' ') || !head[..deals].ends_with(" it") {
        return line.to_string();
    }
    let rest = &line[first + TARGET.len()..];
    let second_marker = format!(", it deals {amount} damage to up to one target ");
    let Some(second) = rest.find(&second_marker) else {
        return line.to_string();
    };
    let first_kind = &rest[..second];
    let rest = &rest[second + second_marker.len()..];
    let third_marker = format!(", and it deals {amount} damage to up to one target ");
    let Some(third) = rest.find(&third_marker) else {
        return line.to_string();
    };
    let second_kind = &rest[..third];
    let tail = &rest[third + third_marker.len()..];
    if [first_kind, second_kind]
        .iter()
        .any(|kind| kind.is_empty() || kind.contains(',') || kind.contains('.'))
    {
        return line.to_string();
    }
    format!(
        "{head} damage to each of up to one target {first_kind}, up to one target {second_kind}, and/or up to one target {tail}"
    )
}

/// "if a creature card in your graveyard has flying" (Bleeding Effect,
/// Kathril).
fn graveyard_card_has_keyword_r5t(line: &str) -> String {
    const LEAD: &str = "if there is a creature card with ";
    const TAIL: &str = " in your graveyard";
    let Some(at) = line.find(LEAD) else {
        return line.to_string();
    };
    let rest = &line[at + LEAD.len()..];
    let Some(end) = rest.find(TAIL) else {
        return line.to_string();
    };
    let keyword = &rest[..end];
    if keyword.is_empty() || !keyword.chars().all(|ch| ch.is_ascii_lowercase() || ch == ' ') {
        return line.to_string();
    }
    format!(
        "{}if a creature card in your graveyard has {keyword}{}",
        &line[..at],
        &rest[end + TAIL.len()..]
    )
}

/// "Target creature gets +3/+2 until end of turn. If it's legendary, it also
/// gains trample until end of turn." A conditional keyword grant after a
/// pump of the same creature adds to it. Oracle omits "also" after a bare
/// type test ("If it's a Spirit, it gains trample"), which stays.
fn also_gains_after_target_pump_r5t(line: &str) -> Option<String> {
    const PUMP_END: &str = " until end of turn. If ";
    const GAINS: &str = ", it gains ";
    let at = line.find(PUMP_END)?;
    let (lead, pt) = line[..at].rsplit_once(' ')?;
    if !lead.ends_with(" gets") || !(pt.starts_with('+') || pt.starts_with('-')) {
        return None;
    }
    let rest = &line[at + PUMP_END.len()..];
    let comma = rest.find(GAINS)?;
    let condition = &rest[..comma];
    let bare_type_test = condition
        .strip_prefix("it's a ")
        .or_else(|| condition.strip_prefix("it's an "))
        .is_some_and(|kind| !kind.contains(' ') || kind == "artifact creature");
    if condition.contains('.') || condition.contains(',') || bare_type_test {
        return None;
    }
    let split = at + PUMP_END.len() + comma;
    Some(format!(
        "{}, it also gains {}",
        &line[..split],
        &line[split + GAINS.len()..]
    ))
}

/// "Creatures you control get +2/+1 until end of turn. White creatures you
/// control also gain first strike until end of turn." (Sanctified Charge,
/// Gallant Fowlknight).
fn also_gain_after_group_pump_r5t(line: &str) -> Option<String> {
    const GROUP: &str = "reatures you control get ";
    const PUMP_END: &str = " until end of turn. ";
    const GAIN: &str = " creatures you control gain ";
    let at = line.find(GROUP)?;
    let rest = &line[at + GROUP.len()..];
    let (pt, tail) = rest.split_once(PUMP_END)?;
    if pt.contains(' ') || !(pt.starts_with('+') || pt.starts_with('-')) {
        return None;
    }
    let (kind, after) = tail.split_once(GAIN)?;
    if kind.contains(' ') || !kind.chars().next().is_some_and(|ch| ch.is_ascii_uppercase()) {
        return None;
    }
    let split = line.len() - tail.len();
    Some(format!(
        "{}{kind} creatures you control also gain {after}",
        &line[..split]
    ))
}

/// "then discard a card. This creature gets +X/+0 until end of turn, where X
/// is the discarded card's mana value." (Spellbound Dragon, Wolf of Devil's
/// Breach).
fn discarded_card_mana_value_r5t(line: &str) -> String {
    const VALUE: &str = "that card's mana value";
    let Some(discard) = line.find("discard a card") else {
        return line.to_string();
    };
    let Some(offset) = line[discard..].find(VALUE) else {
        return line.to_string();
    };
    let between = &line[discard..discard + offset];
    if between.contains("that card") || between.contains("xile") || between.contains("eveal") {
        return line.to_string();
    }
    let at = discard + offset;
    format!(
        "{}the discarded card's mana value{}",
        &line[..at],
        &line[at + VALUE.len()..]
    )
}

/// "Whenever enchanted player casts a spell with the chosen name" (Curse of
/// Silence): the name chosen as the permanent entered, named by nothing
/// earlier in the line.
fn chosen_name_r5t(line: &str) -> String {
    const THAT: &str = "with that name";
    let Some(at) = line.find(THAT) else {
        return line.to_string();
    };
    let head = &line[..at];
    // A later Saga chapter refers back to an earlier chapter's choice
    // ("When you next cast a spell with that name": Medomai's Prophecy).
    if head.contains("name") || head.contains(" next cast ") || head.to_ascii_lowercase().contains("choose a ") {
        return line.to_string();
    }
    format!("{head}with the chosen name{}", &line[at + THAT.len()..])
}

/// Re-phrasings found by the full-corpus resync (each the Oracle majority).
fn audit_resync_line_rephrasings(line: String) -> String {
    let mut line = line;
    // "deals 1 damage to each of up to three target creatures. Those
    // creatures can't block this turn" (Wrap in Flames, Sparkmage's Gambit):
    // Oracle prints "each of" before every plural "up to N" damage target.
    for count in ["two", "three", "four", "five", "six", "X"] {
        let bare = format!(" damage to up to {count} target ");
        if line.contains(&bare) {
            line = line.replace(&bare, &format!(" damage to each of up to {count} target "));
            line = line.replace(
                " target creatures. That creature can't ",
                " target creatures. Those creatures can't ",
            );
        }
    }
    // "is the chosen type in addition to its other types" (10 of 11).
    line = line.replace(
        " the chosen creature type in addition to ",
        " the chosen type in addition to ",
    );
    // Mode bullets sit inside one multi-line entry, so these act per line.
    line = line
        .split('\n')
        .map(audit_resync_mode_line)
        .collect::<Vec<_>>()
        .join("\n");
    // Shadow of the Grave: the cycled/discarded history is a relative
    // clause after the zone.
    line = line.replace(
        "all cards you cycleds or discarded this turns in your graveyard",
        "all cards in your graveyard that you cycled or discarded this turn",
    );
    line
}

fn audit_resync_mode_line(segment: &str) -> String {
    let (bullet, body) = match segment.strip_prefix("• ") {
        Some(body) => ("• ", body),
        None => ("", segment),
    };
    // "Target creature an opponent controls phases out" (Oracle never
    // prints the imperative "Phase out target ...").
    if let Some(object) = body.strip_prefix("Phase out ").and_then(|rest| rest.strip_suffix('.'))
        && !object.contains(['.', ','])
        && !object.contains(" and ")
    {
        return format!("{bullet}{} phases out.", super::capitalize_first(object));
    }
    // "Choose target player. That player draws ..." -> "Target player draws ...".
    if let Some(rest) = body.strip_prefix("Choose target player. That player ")
        && !rest.contains("that player")
        && !rest.contains("That player")
    {
        return format!("{bullet}Target player {rest}");
    }
    // "Target player chooses a creature card in their graveyard. Target
    // player returns it from their graveyard to their hand." -> "Target
    // player returns a creature card from their graveyard to their hand."
    if let Some(rest) = body.strip_prefix("Target player chooses ")
        && let Some((card, tail)) = rest.split_once(" in their graveyard. Target player returns it from their graveyard to their hand")
        && !card.contains('.')
    {
        return format!("{bullet}Target player returns {card} from their graveyard to their hand{tail}");
    }
    segment.to_string()
}
