//! Pure re-phrasings of finished semantic-audit lines (round-5 residual
//! slice D): the same typed reading in the words Oracle prints for it. Each
//! rewrite keys on the structure of the audit reading itself (never on
//! retained prose), keeps its meaning, and was checked against the Oracle
//! corpus phrasing for the cards that reach it.

pub(super) fn audit_residual_d_line(line: String) -> String {
    let mut line = line
        // CR 119.5: "your life total becomes equal to your starting life
        // total" (Oketra's Last Mercy, Long Rest, Resolute Archangel).
        .replace(
            "life total becomes your starting life total",
            "life total becomes equal to your starting life total",
        )
        // A bargained spell's own reduction (Ice Out, Hamlet Glutton).
        .replace(" less to cast if this spell was bargained", " less to cast if it's bargained")
        // Mental Modulation: a reduction gated on your turn.
        .replace(" less to cast if it's your turn.", " less to cast during your turn.")
        // A spell giving its controller hexproof (Blossoming Calm).
        .replace("You have hexproof until your next turn", "You gain hexproof until your next turn")
        .replace("You have shroud until your next turn", "You gain shroud until your next turn")
        // Vedalken Squirrel-Whacker.
        .replace("roll a 6-sided die", "roll a six-sided die")
        .replace("Roll a 6-sided die", "Roll a six-sided die")
        // The Immortal Sun.
        .replace(
            "Loyalty abilities of planeswalker can't be activated.",
            "Players can't activate planeswalkers' loyalty abilities.",
        )
        // Ekundu Cyclops: the attack requirement on this creature.
        .replace(" attacks, this creature attack if able.", " attacks, this creature also attacks if able.")
        // Mana Short: the target is named once by the tap instruction.
        .replace(
            "Choose target player, tap all lands target player controls, and ",
            "Tap all lands target player controls and ",
        )
        // Drana and Linvala: every creature an opponent controls.
        .replace(
            "Activated abilities of a creature an opponent controls can't be activated.",
            "Activated abilities of creatures your opponents control can't be activated.",
        )
        .replace(" of all creatures an opponent controls.", " of all creatures your opponents control.")
        // Choice of Damnations: the chosen number of permanents kept.
        .replace(" sacrifices all but the chosen number permanents ", " sacrifices all but that many permanents ")
        // Void Winnower: every spell with an even mana value.
        .replace("can't cast spells with even mana value.", "can't cast spells with even mana values.")
        .replace("can't cast spells with odd mana value.", "can't cast spells with odd mana values.")
        // Grand Master of Flowers: a creature lacking each listed keyword.
        .replace(
            "Target creature that doesn't have first strike, double strike, or vigilance ",
            "Target creature without first strike, double strike, or vigilance ",
        );
    // "Look at the top seven cards of your library. Put two of them into
    // your hand and the rest ..." (Drawn from Dreams, Dig Through Time).
    for count in ["one", "two", "three"] {
        line = line.replace(
            &format!("Put {count} of those cards into your hand"),
            &format!("Put {count} of them into your hand"),
        );
    }
    // Counters put on the entering permanent as it enters (CR 614.1c):
    // "If you do, this creature enters with a +1/+1 counter on it"
    // (Rescuer Sphinx, Devouring Hellion, Ulamog's Despoiler).
    if let Some(noun) = ["creature", "artifact", "enchantment", "land"]
        .into_iter()
        .find(|noun| line.starts_with(&format!("As this {noun} enters, ")))
        && let Some((head, counters)) = line.split_once(". If you do, put ")
        && let Some(counters) = counters.strip_suffix(&format!(" on this {noun}."))
        && !counters.contains(". ")
        && (counters.ends_with(" counter") || counters.ends_with(" counters"))
    {
        line = format!("{head}. If you do, this {noun} enters with {counters} on it.");
    }
    // CR 700.4: a noncreature permanent "dies" as it's put into a graveyard
    // from the battlefield, which Oracle spells out (Ugin's Nexus).
    for noun in ["artifact", "enchantment"] {
        line = line.replace(
            &format!("If this {noun} would die, "),
            &format!("If this {noun} would be put into a graveyard from the battlefield, "),
        );
    }
    // Unbreathing Horde, Oathsworn Knight: a single counter reads "a".
    line = line
        .replace("remove one +1/+1 counter from", "remove a +1/+1 counter from")
        .replace("remove one -1/-1 counter from", "remove a -1/-1 counter from");
    // Draugr Necromancer, Rayami: "exile it with an ice counter on it
    // instead".
    if let Some((head, rest)) = line.split_once(" exile it instead with one ")
        && let Some(kind) = rest.strip_suffix(" counter on it.")
        && !kind.contains(' ')
    {
        let article = if kind.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" };
        line = format!("{head} exile it with {article} {kind} counter on it instead.");
    }
    // Deadly Wanderings, Homicidal Seclusion: under "exactly one creature"
    // the creatures you control are that creature.
    if line.starts_with("As long as you control exactly one creature, that creature gets ")
        && !line.contains(". ")
    {
        line = line.replacen(" and creatures you control have ", " and has ", 1);
    }
    // Keen-Eyed Curator: "there are four or more card types among cards
    // exiled with this creature".
    if let Some((head, rest)) = line.split_once("the number of card types among ")
        && let Some((cards, rest)) = rest.split_once(" is greater than or equal to ")
        && let Some((count, tail)) = rest.split_once([',', '.'])
        && !count.contains(' ')
        && !cards.contains(" is ")
    {
        let separator = &rest[count.len()..count.len() + 1];
        line = format!("{head}there are {count} or more card types among {cards}{separator}{tail}");
    }
    line = vowel_counter_article(&line);
    let rewrites: [fn(&str) -> Option<String>; 13] = [
        storage_matrix_untap_choice,
        copied_abilities_while_on_battlefield,
        once_each_turn_graveyard_permanent,
        plural_keyword_grant_list,
        conditional_entry_counter_lead,
        keyword_ability_cost_reduction,
        dream_halls_alternative_price,
        at_least_other_permanents,
        third_target_counters,
        land_card_in_graveyard_cost,
        attached_power_unblockable,
        unless_this_is_a_creature,
        behold_land_entry,
    ];
    for rewrite in rewrites {
        if let Some(rewritten) = rewrite(&line) {
            line = rewritten;
        }
    }
    line
}

/// Vampire Socialite: "As long as an opponent lost life this turn, each
/// other Vampire you control enters with an additional +1/+1 counter on it."
fn conditional_entry_counter_lead(line: &str) -> Option<String> {
    let body = line.strip_suffix('.')?;
    if body.starts_with("As long as ") || body.starts_with("If ") || body.contains(['"', ':']) || body.contains(". ") {
        return None;
    }
    let (clause, condition) = body.split_once(" as long as ")?;
    if !clause.contains(" enter with an additional ") && !clause.contains(" enters with an additional ") {
        return None;
    }
    if condition.contains(" as long as ") || clause.contains(", ") {
        return None;
    }
    Some(format!("As long as {condition}, {}.", super::lowercase_first(clause)))
}

/// Hulk, Gamma Goliath; Bladehold War-Whip: "Power-up abilities of other
/// creatures you control cost {3} less to activate."
fn keyword_ability_cost_reduction(line: &str) -> Option<String> {
    let rest = line.strip_prefix("Activated abilities of another ")?;
    let (kind, rest) = rest.split_once(" you control cost ")?;
    let (amount, rest) = rest.split_once(" less to activate if it's ")?;
    let keyword = rest
        .strip_prefix("an ")
        .or_else(|| rest.strip_prefix("a "))?
        .strip_suffix(" ability.")?;
    if kind.contains(' ') || keyword.is_empty() || keyword.contains(' ') || amount.contains(' ') {
        return None;
    }
    let kinds = if kind == "Equipment" { kind.to_string() } else { super::pluralize_word(kind) };
    Some(format!(
        "{} abilities of other {kinds} you control cost {amount} less to activate.",
        super::capitalize_first(keyword)
    ))
}

/// "Demons you control have menace and has lifelink and has haste" is one
/// keyword list on a plural subject: "have menace, lifelink, and haste"
/// (Ardyn, the Usurper; Shelob, Child of Ungoliant).
fn plural_keyword_grant_list(line: &str) -> Option<String> {
    let body = line.strip_suffix('.')?;
    let (subject, rest) = body.split_once(" have ")?;
    if !rest.contains(" and has ")
        || subject.contains(['"', ',', ':'])
        || rest.contains(['"', ':'])
        || rest.contains(". ")
        || rest.contains(" during ")
        || rest.contains(" as long as ")
    {
        return None;
    }
    let parts: Vec<&str> = rest
        .split(" and has ")
        .flat_map(|part| part.split(" and "))
        .collect();
    if parts.iter().any(|part| part.is_empty() || !super::merge_passes::is_keyword_phrase(part)) {
        return None;
    }
    let list = match parts.as_slice() {
        [first, second] => format!("{first} and {second}"),
        [init @ .., last] => format!("{}, and {last}", init.join(", ")),
        [] => return None,
    };
    Some(format!("{subject} have {list}."))
}

/// The Eighth Doctor, Serra Paragon: one land played or one permanent spell
/// cast from the graveyard each turn: "you may play a historic land or cast
/// a historic permanent spell from your graveyard".
fn once_each_turn_graveyard_permanent(line: &str) -> Option<String> {
    let prefix = "Once during each of your turns, you may play ";
    let rest = line.strip_prefix(prefix)?;
    let (quality, rest) = rest.split_once("lands or nonland ")?;
    let (permanents, tail) = rest.split_once(" from your graveyard")?;
    let qualifier = permanents.strip_prefix(quality)?;
    let qualifier = qualifier.strip_prefix("permanents")?;
    if quality.contains(',') || qualifier.contains(',') || !(quality.is_empty() || quality.ends_with(' ')) {
        return None;
    }
    let article = if quality.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" };
    Some(format!(
        "{prefix}{article} {quality}land or cast a {quality}permanent spell{qualifier} from your graveyard{tail}"
    ))
}

/// Necrotic Ooze, Mirran Safehouse, Trazyn: "As long as this creature is on
/// the battlefield, it has all activated abilities of all creature cards in
/// all graveyards."
fn copied_abilities_while_on_battlefield(line: &str) -> Option<String> {
    let rest = line.strip_prefix("This ")?;
    let (noun, rest) = rest.split_once(" has all activated abilities of all ")?;
    let objects = rest.strip_suffix(" as long as this object is on the battlefield.")?;
    if noun.contains(' ') || objects.contains(" as long as ") || objects.contains(". ") {
        return None;
    }
    let objects = match objects.strip_suffix(" in a graveyard") {
        Some(cards) => format!("{cards} in all graveyards"),
        None => objects.to_string(),
    };
    Some(format!(
        "As long as this {noun} is on the battlefield, it has all activated abilities of all {objects}."
    ))
}

/// "a ice counter" is "an ice counter" (Woolly Razorback, the Myojin).
fn vowel_counter_article(line: &str) -> String {
    if !line.contains(" a ") {
        return line.to_string();
    }
    let words: Vec<&str> = line.split(' ').collect();
    let mut out = Vec::with_capacity(words.len());
    for (index, word) in words.iter().enumerate() {
        let next = words.get(index + 1).copied().unwrap_or("");
        let after = words.get(index + 2).copied().unwrap_or("");
        let counter_noun = after.trim_end_matches([',', '.', ';']) == "counter"
            || after.trim_end_matches([',', '.', ';']) == "counters";
        if (*word == "a" || *word == "A")
            && counter_noun
            && next.starts_with(['a', 'e', 'i', 'o'])
            && next.chars().all(|ch| ch.is_ascii_lowercase() || ch == '-')
        {
            out.push(if *word == "a" { "an" } else { "An" });
        } else {
            out.push(*word);
        }
    }
    out.join(" ")
}

/// Storage Matrix (CR 502.3): "As long as this artifact is untapped, each
/// player chooses artifact, creature, or land during their untap step. That
/// player can untap only permanents of the chosen type this step."
fn storage_matrix_untap_choice(line: &str) -> Option<String> {
    let rest = line.strip_prefix("During a player's untap step, only permanents of the chosen card type among [")?;
    let (types, rest) = rest.split_once("] untap as long as ")?;
    let condition = rest.strip_suffix('.')?;
    let types: Vec<String> = types.split(", ").map(|kind| kind.to_ascii_lowercase()).collect();
    if types.len() < 2 || types.iter().any(|kind| kind.is_empty() || kind.contains(' ')) {
        return None;
    }
    let list = match types.as_slice() {
        [first, second] => format!("{first} or {second}"),
        [init @ .., last] => format!("{}, or {last}", init.join(", ")),
        [] => return None,
    };
    Some(format!(
        "As long as {condition}, each player chooses {list} during their untap step. That player can untap only permanents of the chosen type this step."
    ))
}

/// Dream Halls: a granted alternative price for every spell, paid by the
/// spell's controller.
fn dream_halls_alternative_price(line: &str) -> Option<String> {
    let cost = line
        .strip_prefix("Any player may cast spells by paying ")?
        .strip_suffix(" rather than their mana costs.")?;
    if cost.contains("unverified") || cost.contains(" rather than ") {
        return None;
    }
    Some(format!(
        "Rather than pay the mana cost for a spell, its controller may {}.",
        super::lowercase_first(cost)
    ))
}

/// "has indestructible as long as you control at least two other Merfolk"
/// (Svyelun, Ice-Fang Coatl, Ephara): Oracle's form for a static's count of
/// other permanents. Entry gates keep "two or more other lands".
fn at_least_other_permanents(line: &str) -> Option<String> {
    if line.contains("enters tapped") {
        return None;
    }
    let (head, rest) = line.split_once(" as long as you control ")?;
    let (count, tail) = rest.split_once(" or more other ")?;
    if !["two", "three", "four", "five", "six", "seven"].contains(&count) {
        return None;
    }
    Some(format!("{head} as long as you control at least {count} other {tail}"))
}

/// Incremental Growth / Blight: "Put a +1/+1 counter on target creature, two
/// +1/+1 counters on another target creature, and three +1/+1 counters on a
/// third target creature."
fn third_target_counters(line: &str) -> Option<String> {
    let rest = line.strip_prefix("Put ")?;
    let (first, rest) = rest.split_once(", put two ")?;
    let (second, rest) = rest.split_once(", and put three ")?;
    let third = rest.strip_suffix('.')?;
    let (first_counter, first_target) = first.split_once(" on ")?;
    let (second_counter, second_target) = second.split_once(" on ")?;
    let (third_counter, third_target) = third.split_once(" on ")?;
    let noun = first_target.strip_prefix("target ")?;
    let other = format!("another target {noun}");
    if second_target != other || third_target != other || noun.contains(' ') {
        return None;
    }
    Some(format!(
        "Put {first_counter} on target {noun}, two {second_counter} on {other}, and three {third_counter} on a third target {noun}."
    ))
}

/// Igneous Elemental: "costs {2} less to cast if there is a land card in
/// your graveyard".
fn land_card_in_graveyard_cost(line: &str) -> Option<String> {
    let (head, rest) = line.split_once(" less to cast if ")?;
    let (kind, tail) = rest.split_once(" card in your graveyard is in your graveyard")?;
    if kind.is_empty() || kind.contains(' ') {
        return None;
    }
    let article = if kind.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" };
    Some(format!("{head} less to cast if there is {article} {kind} card in your graveyard{tail}"))
}

/// Thieves' Tools: "Equipped creature can't be blocked as long as its power
/// is 3 or less."
fn attached_power_unblockable(line: &str) -> Option<String> {
    for subject in ["Equipped creature", "Enchanted creature"] {
        let Some(rest) = line.strip_prefix(&format!("{subject} with power ")) else {
            continue;
        };
        let Some(bound) = rest.strip_suffix(" can't be blocked.") else {
            continue;
        };
        let (number, direction) = bound.split_once(" or ")?;
        if !number.chars().all(|ch| ch.is_ascii_digit()) || !["less", "greater"].contains(&direction) {
            return None;
        }
        return Some(format!("{subject} can't be blocked as long as its power is {number} or {direction}."));
    }
    None
}

/// Romulan Bird-of-Prey: "As long as this Spacecraft isn't a creature, it
/// has hexproof."
fn unless_this_is_a_creature(line: &str) -> Option<String> {
    let rest = line.strip_prefix("This ")?;
    let (noun, rest) = rest.split_once(" has ")?;
    let (keyword, rest) = rest.split_once(" unless this ")?;
    let tail_noun = rest.strip_suffix(" is a creature.")?;
    if noun != tail_noun || noun.contains(' ') || keyword.contains(' ') {
        return None;
    }
    Some(format!(
        "As long as this {noun} isn't a creature, it has {}.",
        keyword.to_ascii_lowercase()
    ))
}

/// Behold (CR 701.62): reveal one from your hand or choose one you control.
/// "This land enters tapped unless you have a Jace in hand or you control
/// Jace" is Theorist's Sanctum's "you may behold a Jace"; a reveal-a-card
/// land keeps its gate with the indefinite article ("or you control a
/// Soldier").
fn behold_land_entry(line: &str) -> Option<String> {
    let rest = line.strip_prefix("This land enters tapped unless you have a ")?;
    let (kind, rest) = rest.split_once(" in hand or you control ")?;
    let controlled = rest.strip_suffix('.')?;
    if let Some(card_kind) = kind.strip_suffix(" card") {
        if card_kind != controlled || card_kind.contains(' ') {
            return None;
        }
        let article = if controlled.starts_with(['A', 'E', 'I', 'O', 'U']) { "an" } else { "a" };
        return Some(format!(
            "This land enters tapped unless you have a {kind} in hand or you control {article} {controlled}."
        ));
    }
    if kind != controlled || kind.contains(' ') {
        return None;
    }
    Some(format!("As this land enters, you may behold a {kind}. If you don't, this land enters tapped."))
}

/// Cross-line re-phrasings of the finished audit lines.
pub(super) fn audit_residual_d_lines(lines: Vec<String>) -> Vec<String> {
    spells_and_abilities_cant_be_countered(same_is_true_for_spells_and_cards(lines))
}

/// Spider-Punk: "Spells and abilities can't be countered."
fn spells_and_abilities_cant_be_countered(lines: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    for line in lines {
        if line == "Abilities can't be countered."
            && out.last().is_some_and(|previous| previous == "Spells can't be countered.")
        {
            *out.last_mut().unwrap() = "Spells and abilities can't be countered.".to_string();
            continue;
        }
        out.push(line);
    }
    out
}

/// CR 205.1 / 611.3a: one characteristic change for permanents, spells and
/// cards outside the battlefield (Conspiracy, Maskwood Nexus, Celestial
/// Dawn): "Creatures you control are X. The same is true for creature spells
/// you control and creature cards you own that aren't on the battlefield."
fn same_is_true_for_spells_and_cards(lines: Vec<String>) -> Vec<String> {
    fn predicate<'a>(line: &'a str, suffix: &str) -> Option<(&'a str, &'a str)> {
        let body = line.strip_suffix('.')?;
        if body.contains(['"', ':']) || body.contains(". ") || body.contains(" as long as ") {
            return None;
        }
        let (subject, predicate) = body.split_once(" are ")?;
        let head = subject.strip_suffix(suffix)?;
        Some((head, predicate))
    }
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut index = 0;
    while index < lines.len() {
        if index + 2 < lines.len()
            && let Some((permanents, first)) = predicate(&lines[index], "")
            && permanents.ends_with(" you control")
            && let Some((spells, second)) = predicate(&lines[index + 1], "pells you control")
            && let Some((cards, third)) =
                predicate(&lines[index + 2], " cards you own that aren't on the battlefield")
            && first == second
            && first == third
            && (spells == "S" || spells.ends_with(" s"))
            && !cards.is_empty()
        {
            let spells = if spells == "S" {
                "spells you control".to_string()
            } else {
                format!("{}spells you control", super::lowercase_first(&spells[..spells.len() - 1]))
            };
            let cards = super::lowercase_first(cards);
            out.push(format!(
                "{} The same is true for {spells} and {cards} cards you own that aren't on the battlefield.",
                lines[index]
            ));
            index += 3;
            continue;
        }
        out.push(lines[index].clone());
        index += 1;
    }
    out
}
