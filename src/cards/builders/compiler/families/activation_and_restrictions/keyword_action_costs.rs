pub(crate) fn target_ast_to_object_filter(target: TargetAst) -> Option<ObjectFilter> {
    match target {
        TargetAst::Source(_) => Some(ObjectFilter::source()),
        TargetAst::Object(filter, _, _) => Some(filter),
        TargetAst::Spell(_) => Some(ObjectFilter::spell()),
        TargetAst::Tagged(tag, _) => Some(ObjectFilter::tagged(tag)),
        TargetAst::WithCount(inner, _) => target_ast_to_object_filter(*inner),
        _ => None,
    }
}

pub(crate) fn is_supported_untap_restriction_tail(words: &[&str]) -> bool {
    if words.is_empty() {
        return false;
    }
    if !(words[0] == "untap" || words[0] == "untaps") {
        return false;
    }
    if words.len() == 1 {
        return true;
    }

    let allowed = [
        "untap",
        "untaps",
        "during",
        "its",
        "their",
        "your",
        "controllers",
        "controller",
        "untap",
        "step",
        "steps",
        "next",
        "the",
    ];
    if words.iter().any(|word| !slice_contains(&allowed, word)) {
        return false;
    }

    slice_contains(&words, &"during")
        && (slice_contains(&words, &"step") || slice_contains(&words, &"steps"))
}

pub(crate) fn normalize_cant_words(tokens: &[OwnedLexToken]) -> Vec<String> {
    ActivationRestrictionCompatWords::new(tokens)
        .to_word_refs()
        .into_iter()
        .map(|word| {
            if word == "cannot" {
                "cant".to_string()
            } else {
                word.to_string()
            }
        })
        .collect()
}

pub(crate) fn keyword_title(keyword: &str) -> String {
    let mut words = keyword.split_whitespace();
    let Some(first) = words.next() else {
        return String::new();
    };
    let mut out = String::new();
    let mut first_chars = first.chars();
    if let Some(ch) = first_chars.next() {
        out.push(ch.to_ascii_uppercase());
        out.push_str(first_chars.as_str());
    }
    for word in words {
        out.push(' ');
        out.push_str(word);
    }
    out
}

pub(crate) fn leading_mana_symbols_to_oracle(words: &[&str]) -> Option<(String, usize)> {
    if words.is_empty() {
        return None;
    }
    let mut pips = Vec::new();
    let mut consumed = 0usize;
    for word in words {
        let Ok(symbol) = parse_mana_symbol(word) else {
            break;
        };
        pips.push(vec![symbol]);
        consumed += 1;
    }
    if consumed == 0 {
        return None;
    }
    Some((ManaCost::from_pips(pips).to_oracle(), consumed))
}

pub(crate) fn marker_keyword_id(keyword: &str) -> Option<&'static str> {
    match keyword {
        "banding" => Some("banding"),
        "fabricate" => Some("fabricate"),
        "foretell" => Some("foretell"),
        "bestow" => Some("bestow"),
        "dash" => Some("dash"),
        "overload" => Some("overload"),
        "soulshift" => Some("soulshift"),
        "adapt" => Some("adapt"),
        "bolster" => Some("bolster"),
        "disturb" => Some("disturb"),
        "echo" => Some("echo"),
        "modular" => Some("modular"),
        "ninjutsu" => Some("ninjutsu"),
        "outlast" => Some("outlast"),
        "scavenge" => Some("scavenge"),
        "suspend" => Some("suspend"),
        "vanishing" => Some("vanishing"),
        "offering" => Some("offering"),
        "soulbond" => Some("soulbond"),
        "unearth" => Some("unearth"),
        "specialize" => Some("specialize"),
        "squad" => Some("squad"),
        "spectacle" => Some("spectacle"),
        "graft" => Some("graft"),
        "backup" => Some("backup"),
        "saddle" => Some("saddle"),
        "fading" => Some("fading"),
        "fuse" => Some("fuse"),
        "plot" => Some("plot"),
        "disguise" => Some("disguise"),
        "tribute" => Some("tribute"),
        "buyback" => Some("buyback"),
        "flashback" => Some("flashback"),
        "rebound" => Some("rebound"),
        _ => None,
    }
}

pub(crate) fn marker_keyword_display(words: &[&str]) -> Option<String> {
    let keyword = words.first().copied()?;
    let title = keyword_title(keyword);

    match keyword {
        "soulshift" | "adapt" | "bolster" | "modular" | "vanishing" | "backup" | "saddle"
        | "fading" | "graft" | "tribute" => {
            let amount = words.get(1)?.parse::<u32>().ok()?;
            Some(format!("{title} {amount}"))
        }
        "bestow" | "dash" | "disturb" | "ninjutsu" | "outlast" | "scavenge" | "unearth"
        | "specialize" | "spectacle" | "plot" | "disguise" | "flashback" | "foretell"
        | "overload" => {
            let (cost, _) = leading_mana_symbols_to_oracle(&words[1..])?;
            Some(format!("{title} {cost}"))
        }
        "echo" => {
            if let Some((cost, _)) = leading_mana_symbols_to_oracle(&words[1..]) {
                return Some(format!("Echo {cost}"));
            }
            if words.len() > 1 {
                let payload = words[1..].join(" ");
                let mut chars = payload.chars();
                let Some(first) = chars.next() else {
                    return Some("Echo".to_string());
                };
                let mut normalized = String::new();
                normalized.push(first.to_ascii_uppercase());
                normalized.push_str(chars.as_str());
                return Some(format!("Echo—{normalized}"));
            }
            Some("Echo".to_string())
        }
        "buyback" => {
            if let Some((cost, _)) = leading_mana_symbols_to_oracle(&words[1..]) {
                Some(format!("Buyback {cost}"))
            } else if words.len() > 1 {
                Some(format!("Buyback—{}", words[1..].join(" ")))
            } else {
                Some("Buyback".to_string())
            }
        }
        "suspend" => {
            let time = words.get(1)?.parse::<u32>().ok()?;
            let (cost, _) = leading_mana_symbols_to_oracle(&words[2..])?;
            Some(format!("Suspend {time}—{cost}"))
        }
        "rebound" => Some("Rebound".to_string()),
        "squad" => {
            let (cost, _) = leading_mana_symbols_to_oracle(&words[1..])?;
            Some(format!("Squad {cost}"))
        }
        _ => None,
    }
}

fn marker_text_from_words(words: &[&str]) -> Option<String> {
    let first = words.first().copied()?;
    let mut text = keyword_title(first);
    if words.len() > 1 {
        text.push(' ');
        text.push_str(&words[1..].join(" "));
    }
    Some(text)
}

fn parse_numeric_keyword_action<F>(
    words: &[&str],
    keyword: &'static str,
    build: F,
) -> Option<KeywordAction>
where
    F: FnOnce(u32) -> KeywordAction,
{
    if words.first().copied() != Some(keyword) {
        return None;
    }
    if let Some(amount) = words.get(1).and_then(|word| word.parse::<u32>().ok()) {
        return Some(build(amount));
    }
    Some(KeywordAction::Marker(keyword))
}

enum KeywordCostFallback {
    MarkerOnly,
    MarkerOrText,
}

fn parse_cost_keyword_action<F>(
    words: &[&str],
    keyword: &'static str,
    fallback: KeywordCostFallback,
    build: F,
) -> Option<KeywordAction>
where
    F: FnOnce(ManaCost) -> KeywordAction,
{
    if words.first().copied() != Some(keyword) {
        return None;
    }
    if matches!(words.get(1).copied(), Some("cost" | "costs")) {
        return None;
    }
    if let Some((cost_text, _consumed)) = leading_mana_symbols_to_oracle(&words[1..])
        && let Ok(cost) = parse_scryfall_mana_cost(&cost_text)
    {
        return Some(build(cost));
    }
    if matches!(fallback, KeywordCostFallback::MarkerOrText) && words.len() > 1 {
        if let Some(display) = marker_keyword_display(words) {
            return Some(KeywordAction::MarkerText(display));
        }
    }
    Some(KeywordAction::Marker(keyword))
}

pub(crate) fn parse_single_word_keyword_action(word: &str) -> Option<KeywordAction> {
    match word {
        "flying" => Some(KeywordAction::Flying),
        "menace" => Some(KeywordAction::Menace),
        "hexproof" => Some(KeywordAction::Hexproof),
        "haste" => Some(KeywordAction::Haste),
        "improvise" => Some(KeywordAction::Improvise),
        "convoke" => Some(KeywordAction::Convoke),
        "delve" => Some(KeywordAction::Delve),
        "deathtouch" => Some(KeywordAction::Deathtouch),
        "lifelink" => Some(KeywordAction::Lifelink),
        "vigilance" => Some(KeywordAction::Vigilance),
        "trample" => Some(KeywordAction::Trample),
        "reach" => Some(KeywordAction::Reach),
        "defender" => Some(KeywordAction::Defender),
        "flash" => Some(KeywordAction::Flash),
        "phasing" => Some(KeywordAction::Phasing),
        "indestructible" => Some(KeywordAction::Indestructible),
        "shroud" => Some(KeywordAction::Shroud),
        "assist" => Some(KeywordAction::Assist),
        "backup" => Some(KeywordAction::Marker("backup")),
        "cipher" => Some(KeywordAction::Cipher),
        "devoid" => Some(KeywordAction::Devoid),
        "dethrone" => Some(KeywordAction::Dethrone),
        "enlist" => Some(KeywordAction::Enlist),
        "evolve" => Some(KeywordAction::Evolve),
        "extort" => Some(KeywordAction::Extort),
        "haunt" => Some(KeywordAction::Haunt),
        "ingest" => Some(KeywordAction::Ingest),
        "mentor" => Some(KeywordAction::Mentor),
        "melee" => Some(KeywordAction::Melee),
        "training" => Some(KeywordAction::Training),
        "myriad" => Some(KeywordAction::Myriad),
        "partner" => Some(KeywordAction::Partner),
        "provoke" => Some(KeywordAction::Provoke),
        "ravenous" => Some(KeywordAction::Ravenous),
        "riot" => Some(KeywordAction::Riot),
        "skulk" => Some(KeywordAction::Skulk),
        "sunburst" => Some(KeywordAction::Sunburst),
        "undaunted" => Some(KeywordAction::Undaunted),
        "unleash" => Some(KeywordAction::Unleash),
        "wither" => Some(KeywordAction::Wither),
        "infect" => Some(KeywordAction::Infect),
        "undying" => Some(KeywordAction::Undying),
        "persist" => Some(KeywordAction::Persist),
        "prowess" => Some(KeywordAction::Prowess),
        "exalted" => Some(KeywordAction::Exalted),
        "cascade" => Some(KeywordAction::Cascade),
        "storm" => Some(KeywordAction::Storm),
        "rebound" => Some(KeywordAction::Rebound),
        "ascend" => Some(KeywordAction::Ascend),
        "compleated" => Some(KeywordAction::Marker("compleated")),
        "daybound" => Some(KeywordAction::Daybound),
        "nightbound" => Some(KeywordAction::Nightbound),
        "islandwalk" => Some(KeywordAction::Landwalk(
            crate::static_abilities::LandwalkKind::Subtype {
                subtype: Subtype::Island,
                snow: false,
            },
        )),
        "swampwalk" => Some(KeywordAction::Landwalk(
            crate::static_abilities::LandwalkKind::Subtype {
                subtype: Subtype::Swamp,
                snow: false,
            },
        )),
        "mountainwalk" => Some(KeywordAction::Landwalk(
            crate::static_abilities::LandwalkKind::Subtype {
                subtype: Subtype::Mountain,
                snow: false,
            },
        )),
        "forestwalk" => Some(KeywordAction::Landwalk(
            crate::static_abilities::LandwalkKind::Subtype {
                subtype: Subtype::Forest,
                snow: false,
            },
        )),
        "plainswalk" => Some(KeywordAction::Landwalk(
            crate::static_abilities::LandwalkKind::Subtype {
                subtype: Subtype::Plains,
                snow: false,
            },
        )),
        "fear" => Some(KeywordAction::Fear),
        "intimidate" => Some(KeywordAction::Intimidate),
        "shadow" => Some(KeywordAction::Shadow),
        "horsemanship" => Some(KeywordAction::Horsemanship),
        "flanking" => Some(KeywordAction::Flanking),
        "changeling" => Some(KeywordAction::Changeling),
        _ => None,
    }
}

pub(crate) fn parse_ability_phrase(tokens: &[OwnedLexToken]) -> Option<KeywordAction> {
    let mut phrase_tokens = tokens;
    if phrase_tokens
        .first()
        .is_some_and(|token| token.is_word("and"))
    {
        phrase_tokens = &phrase_tokens[1..];
    }

    let word_view = ActivationRestrictionCompatWords::new(phrase_tokens);
    let words = word_view.to_word_refs();
    if words.is_empty() {
        return None;
    }

    let (head, second) = lexed_head_words(phrase_tokens).unwrap_or(("", None));

    match words.as_slice() {
        ["landwalk"] => {
            return Some(KeywordAction::Landwalk(
                crate::static_abilities::LandwalkKind::AnyLand,
            ));
        }
        ["nonbasic", "landwalk"] => {
            return Some(KeywordAction::Landwalk(
                crate::static_abilities::LandwalkKind::NonbasicLand,
            ));
        }
        ["artifact", "landwalk"] => {
            return Some(KeywordAction::Landwalk(
                crate::static_abilities::LandwalkKind::ArtifactLand,
            ));
        }
        ["snow", subtype_walk] => {
            if let Some(action) = parse_single_word_keyword_action(subtype_walk)
                && let KeywordAction::Landwalk(crate::static_abilities::LandwalkKind::Subtype {
                    subtype,
                    ..
                }) = action
            {
                return Some(KeywordAction::Landwalk(
                    crate::static_abilities::LandwalkKind::Subtype {
                        subtype,
                        snow: true,
                    },
                ));
            }
        }
        _ => {}
    }

    if strip_prefix_phrase(phrase_tokens, &["cumulative", "upkeep"]).is_some() {
        let reminder_start =
            find_index(phrase_tokens, |token| token.is_period()).unwrap_or(phrase_tokens.len());
        let cost_tokens = trim_commas(&phrase_tokens[2..reminder_start]).to_vec();
        let cost_word_view = ActivationRestrictionCompatWords::new(&cost_tokens);
        let cost_words = cost_word_view.to_word_refs();

        if cost_words.len() == 3
            && cost_words[0] == "pay"
            && cost_words[2] == "life"
            && let Ok(life_per_counter) = cost_words[1].parse::<u32>()
            && life_per_counter > 0
        {
            return Some(KeywordAction::CumulativeUpkeep {
                mana_symbols_per_counter: Vec::new(),
                life_per_counter,
                text: format!("Cumulative upkeep—Pay {life_per_counter} life"),
            });
        }

        let mut pips = Vec::new();
        let mut parsed_all = !cost_tokens.is_empty();
        for token in &cost_tokens {
            let Some(group) = mana_pips_from_token(token) else {
                parsed_all = false;
                break;
            };
            pips.push(group);
        }
        if parsed_all && !pips.is_empty() {
            let cost = crate::mana::ManaCost::from_pips(pips.clone()).to_oracle();
            let mut mana_symbols_per_counter = Vec::new();
            let mut flattenable = true;
            for pip in pips {
                let [symbol] = pip.as_slice() else {
                    flattenable = false;
                    break;
                };
                mana_symbols_per_counter.push(*symbol);
            }
            if flattenable && !mana_symbols_per_counter.is_empty() {
                return Some(KeywordAction::CumulativeUpkeep {
                    mana_symbols_per_counter,
                    life_per_counter: 0,
                    text: format!("Cumulative upkeep {cost}"),
                });
            }
        }

        let mut text = "Cumulative upkeep".to_string();
        let tail = &words[2..];
        if !tail.is_empty() {
            if tail.first().copied() == Some("add")
                && let Some((cost, consumed)) = leading_mana_symbols_to_oracle(&tail[1..])
                && consumed + 1 == tail.len()
            {
                text = format!("Cumulative upkeep—Add {cost}");
            } else if let Some((cost, consumed)) = leading_mana_symbols_to_oracle(tail)
                && consumed == tail.len()
            {
                text = format!("Cumulative upkeep {cost}");
            } else if tail.len() == 3
                && tail[1] == "or"
                && let (Some((left, 1)), Some((right, 1))) = (
                    leading_mana_symbols_to_oracle(&tail[..1]),
                    leading_mana_symbols_to_oracle(&tail[2..3]),
                )
            {
                text = format!("Cumulative upkeep {left} or {right}");
            } else {
                let mut tail_text = tail.join(" ");
                if let Some(first) = tail_text.chars().next() {
                    let upper = first.to_ascii_uppercase().to_string();
                    let rest = &tail_text[first.len_utf8()..];
                    tail_text = format!("{upper}{rest}");
                }
                text = format!("Cumulative upkeep—{tail_text}");
            }
        }
        return Some(KeywordAction::MarkerText(text));
    }

    if let Some(action) = parse_numeric_keyword_action(&words, "bushido", KeywordAction::Bushido) {
        return Some(action);
    }
    if let Some(action) =
        parse_numeric_keyword_action(&words, "bloodthirst", KeywordAction::Bloodthirst)
    {
        return Some(action);
    }
    if let Some(action) = parse_numeric_keyword_action(&words, "backup", KeywordAction::Backup) {
        return Some(action);
    }
    if let Some(action) = parse_numeric_keyword_action(&words, "rampage", KeywordAction::Rampage) {
        return Some(action);
    }
    if let Some(action) =
        parse_numeric_keyword_action(&words, "annihilator", KeywordAction::Annihilator)
    {
        return Some(action);
    }

    // Crew appears as "Crew N" and is often followed by inline restrictions/reminder text.
    if head == "crew" {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            let has_sorcery_speed =
                contains_word_sequence(&words, &["activate", "only", "as", "a", "sorcery"]);

            let has_once_per_turn = contains_any_word_sequence(
                &words,
                &[
                    &["activate", "only", "once", "each", "turn"],
                    &["activate", "only", "once", "per", "turn"],
                ],
            );

            let mut additional_restrictions = Vec::new();
            let timing = if has_sorcery_speed {
                if has_once_per_turn {
                    additional_restrictions.push("Activate only once each turn.".to_string());
                }
                ActivationTiming::SorcerySpeed
            } else if has_once_per_turn {
                ActivationTiming::OncePerTurn
            } else {
                ActivationTiming::AnyTime
            };

            return Some(KeywordAction::Crew {
                amount,
                timing,
                additional_restrictions,
            });
        }
        // Fallback: preserve unsupported crew variants as marker text.
        if let Some(display) = marker_keyword_display(&words) {
            return Some(KeywordAction::MarkerText(display));
        }
        return Some(KeywordAction::Marker("crew"));
    }

    // Saddle appears as "Saddle N" and is often followed by reminder text.
    // Per CR 702.171a, Saddle can be activated only as a sorcery.
    if head == "saddle" {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            let has_once_per_turn = contains_any_word_sequence(
                &words,
                &[
                    &["activate", "only", "once", "each", "turn"],
                    &["activate", "only", "once", "per", "turn"],
                ],
            );

            let mut additional_restrictions = Vec::new();
            let timing = ActivationTiming::SorcerySpeed;
            if has_once_per_turn {
                additional_restrictions.push("Activate only once each turn.".to_string());
            }

            return Some(KeywordAction::Saddle {
                amount,
                timing,
                additional_restrictions,
            });
        }
        // Fallback: preserve unsupported saddle variants as marker text.
        if let Some(display) = marker_keyword_display(&words) {
            return Some(KeywordAction::MarkerText(display));
        }
        return Some(KeywordAction::Marker("saddle"));
    }

    if let Some(action) =
        parse_numeric_keyword_action(&words, "afterlife", KeywordAction::Afterlife)
    {
        return Some(action);
    }
    if let Some(action) =
        parse_numeric_keyword_action(&words, "fabricate", KeywordAction::Fabricate)
    {
        return Some(action);
    }

    if head == "evolve" {
        return Some(KeywordAction::Evolve);
    }

    if head == "mentor" {
        return Some(KeywordAction::Mentor);
    }

    if head == "training" {
        return Some(KeywordAction::Training);
    }

    if head == "soulbond" {
        return Some(KeywordAction::Soulbond);
    }

    if let Some(action) = parse_numeric_keyword_action(&words, "renown", KeywordAction::Renown) {
        return Some(action);
    }
    if let Some(action) =
        parse_numeric_keyword_action(&words, "soulshift", KeywordAction::Soulshift)
    {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "outlast",
        KeywordCostFallback::MarkerOnly,
        KeywordAction::Outlast,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "scavenge",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Scavenge,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "unearth",
        KeywordCostFallback::MarkerOnly,
        KeywordAction::Unearth,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "ninjutsu",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Ninjutsu,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "dash",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Dash,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "warp",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Warp,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "plot",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Plot,
    ) {
        return Some(action);
    }

    if head == "suspend" {
        if let Some(time_word) = words.get(1)
            && let Ok(time) = time_word.parse::<u32>()
            && let Some((cost_text, _consumed)) = leading_mana_symbols_to_oracle(&words[2..])
            && let Ok(cost) = parse_scryfall_mana_cost(&cost_text)
        {
            return Some(KeywordAction::Suspend { time, cost });
        }
        if words.len() == 1 {
            return Some(KeywordAction::Marker("suspend"));
        }
        if let Some(display) = marker_keyword_display(&words) {
            return Some(KeywordAction::MarkerText(display));
        }
        return Some(KeywordAction::Marker("suspend"));
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "disturb",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Disturb,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "foretell",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Foretell,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "spectacle",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Spectacle,
    ) {
        return Some(action);
    }

    if head == "hideaway" {
        if words.len() == 1 {
            return Some(KeywordAction::MarkerText("Hideaway".to_string()));
        }
        return marker_text_from_words(&words).map(KeywordAction::MarkerText);
    }

    if head == "mobilize" {
        if let Some(amount_word) = words.get(1)
            && let Ok(amount) = amount_word.parse::<u32>()
        {
            return Some(KeywordAction::Mobilize(amount));
        }
        if words.len() == 1 {
            return Some(KeywordAction::Marker("mobilize"));
        }
        return marker_text_from_words(&words).map(KeywordAction::MarkerText);
    }

    if head == "impending" {
        if words.len() == 1 {
            return Some(KeywordAction::MarkerText("Impending".to_string()));
        }
        return marker_text_from_words(&words).map(KeywordAction::MarkerText);
    }

    if let Some((matched_phrase, _)) = strip_prefix_phrases(
        phrase_tokens,
        &[&["emerge", "from"], &["job", "select"], &["umbra", "armor"]],
    ) {
        return match matched_phrase {
            ["emerge", "from"] => marker_text_from_words(&words).map(KeywordAction::MarkerText),
            ["job", "select"] => Some(KeywordAction::MarkerText("Job select".to_string())),
            ["umbra", "armor"] => Some(KeywordAction::UmbraArmor),
            _ => None,
        };
    }

    if head == "exert" {
        return marker_text_from_words(&words).map(KeywordAction::MarkerText);
    }

    if head == "airbend" {
        return marker_text_from_words(&words).map(KeywordAction::MarkerText);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "overload",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Overload,
    ) {
        return Some(action);
    }

    if head == "echo" {
        if let Some((cost_text, consumed)) = leading_mana_symbols_to_oracle(&words[1..])
            && consumed > 0
            && let Ok(cost) = parse_scryfall_mana_cost(&cost_text)
        {
            return Some(KeywordAction::Echo {
                total_cost: crate::cost::TotalCost::mana(cost),
                text: format!("Echo {cost_text}"),
            });
        }

        let reminder_start = find_index(phrase_tokens, |token| token.is_period())
            .or_else(|| {
                phrase_tokens
                    .iter()
                    .enumerate()
                    .skip(1)
                    .find_map(|(idx, token)| token.is_word("at").then_some(idx))
            })
            .unwrap_or(phrase_tokens.len());
        let cost_tokens = trim_commas(&phrase_tokens[1..reminder_start]).to_vec();

        if !cost_tokens.is_empty()
            && let Ok(total_cost) = parse_activation_cost(&cost_tokens)
        {
            let text = if let Some(cost) = total_cost.mana_cost()
                && !total_cost.has_non_mana_costs()
            {
                format!("Echo {}", cost.to_oracle())
            } else {
                let payload = cost_tokens
                    .iter()
                    .filter_map(OwnedLexToken::as_word)
                    .collect::<Vec<_>>()
                    .join(" ");
                if payload.is_empty() {
                    "Echo".to_string()
                } else {
                    let mut chars = payload.chars();
                    let first = chars.next().expect("payload is not empty");
                    let mut normalized = String::new();
                    normalized.push(first.to_ascii_uppercase());
                    normalized.push_str(chars.as_str());
                    format!("Echo—{normalized}")
                }
            };
            return Some(KeywordAction::Echo { total_cost, text });
        }

        if words.len() == 1 {
            return Some(KeywordAction::Marker("echo"));
        }
        if let Some(display) = marker_keyword_display(&words) {
            return Some(KeywordAction::MarkerText(display));
        }
        return Some(KeywordAction::Marker("echo"));
    }

    if head == "modular" {
        if words.get(1).copied() == Some("sunburst") {
            return Some(KeywordAction::ModularSunburst);
        }
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Modular(amount));
        }
        return Some(KeywordAction::Marker("modular"));
    }

    if head == "graft" {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Graft(amount));
        }
        return Some(KeywordAction::Marker("graft"));
    }

    if head == "fading" {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Fading(amount));
        }
        return Some(KeywordAction::Marker("fading"));
    }

    if head == "vanishing" {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Vanishing(amount));
        }
        if words.len() == 1 {
            return Some(KeywordAction::Vanishing(0));
        }
        return Some(KeywordAction::Marker("vanishing"));
    }

    if head == "harness" {
        if words.len() > 1 {
            return Some(KeywordAction::MarkerText(format!(
                "Harness {}",
                words[1..].join(" ")
            )));
        }
        return Some(KeywordAction::MarkerText("Harness".to_string()));
    }

    if head == "sunburst" {
        return Some(KeywordAction::Sunburst);
    }
    if let Some((matched_phrase, _)) = strip_prefix_phrases(
        phrase_tokens,
        &[
            &["for", "mirrodin"],
            &["living", "weapon"],
            &["battle", "cry"],
            &["split", "second"],
            &["doctor", "companion"],
        ],
    ) {
        return Some(match matched_phrase {
            ["for", "mirrodin"] => KeywordAction::ForMirrodin,
            ["living", "weapon"] => KeywordAction::LivingWeapon,
            ["battle", "cry"] => KeywordAction::BattleCry,
            ["split", "second"] => KeywordAction::SplitSecond,
            ["doctor", "companion"] => KeywordAction::Marker("doctor companion"),
            _ => unreachable!("matched phrase must be one of the declared keyword heads"),
        });
    }
    if head == "cascade" {
        return Some(KeywordAction::Cascade);
    }

    // Casualty N - "as you cast this spell, you may sacrifice a creature with power N or greater"
    if head == "casualty" {
        if words.len() == 2 {
            if let Ok(power) = words[1].parse::<u32>() {
                return Some(KeywordAction::Casualty(power));
            }
        }
        if words.len() == 1 {
            return Some(KeywordAction::Casualty(1));
        }
        return None;
    }

    // Conspire - "as you cast this spell, you may tap two untapped creatures..."
    if head == "conspire" && words.len() == 1 {
        return Some(KeywordAction::Conspire);
    }

    // Devour N - "as this enters, you may sacrifice any number of creatures..."
    if head == "devour" {
        if words.len() == 2 {
            if let Ok(multiplier) = words[1].parse::<u32>() {
                return Some(KeywordAction::Devour(multiplier));
            }
        }
        if words.len() == 1 {
            return Some(KeywordAction::Devour(1));
        }
        return None;
    }

    if let Some(first) = (!head.is_empty()).then_some(head)
        && matches!(
            first,
            "banding"
                | "fabricate"
                | "foretell"
                | "bestow"
                | "dash"
                | "overload"
                | "soulshift"
                | "adapt"
                | "bolster"
                | "disturb"
                | "echo"
                | "modular"
                | "ninjutsu"
                | "outlast"
                | "suspend"
                | "vanishing"
                | "offering"
                | "specialize"
                | "spectacle"
                | "graft"
                | "backup"
                | "fading"
                | "fuse"
                | "plot"
                | "disguise"
                | "tribute"
                | "buyback"
                | "flashback"
        )
    {
        if let Some(display) = marker_keyword_display(&words) {
            return Some(KeywordAction::MarkerText(display));
        }
        if words.len() > 1 {
            return None;
        }
        return Some(KeywordAction::Marker(
            marker_keyword_id(first).expect("marker keyword id must exist for matched keyword"),
        ));
    }

    if words.len() == 1
        && let Some(action) = parse_single_word_keyword_action(words[0])
    {
        return Some(action);
    }

    let action = match words.as_slice() {
        ["affinity", "for", "artifacts"] => KeywordAction::AffinityForArtifacts,
        ["first", "strike"] => KeywordAction::FirstStrike,
        ["double", "strike"] => KeywordAction::DoubleStrike,
        ["for", "mirrodin"] => KeywordAction::ForMirrodin,
        ["living", "weapon"] => KeywordAction::LivingWeapon,
        ["fading", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Fading(value)
        }
        ["vanishing", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Vanishing(value)
        }
        ["modular", "sunburst"] => KeywordAction::ModularSunburst,
        ["modular", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Modular(value)
        }
        ["graft", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Graft(value)
        }
        ["soulshift", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Soulshift(value)
        }
        ["outlast", cost] => {
            let parsed_cost = parse_scryfall_mana_cost(cost).ok()?;
            KeywordAction::Outlast(parsed_cost)
        }
        ["ward", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Ward(value)
        }
        ["afterlife", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Afterlife(value)
        }
        ["backup", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Backup(value)
        }
        ["fabricate", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Fabricate(value)
        }
        ["renown", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Renown(value)
        }
        ["protection", "from", "all", "colors"] => KeywordAction::ProtectionFromAllColors,
        ["protection", "from", "all", "color"] => KeywordAction::ProtectionFromAllColors,
        ["protection", "from", "colorless"] => KeywordAction::ProtectionFromColorless,
        ["protection", "from", "everything"] => KeywordAction::ProtectionFromEverything,
        ["protection", "from", value] => {
            if let Some(color) = parse_color(value) {
                KeywordAction::ProtectionFrom(color)
            } else if let Some(card_type) = parse_card_type(value) {
                KeywordAction::ProtectionFromCardType(card_type)
            } else if let Some(subtype) = parse_subtype_flexible(value) {
                KeywordAction::ProtectionFromSubtype(subtype)
            } else {
                return None;
            }
        }
        _ => {
            // "toxic N" needs exactly 2 words
            if words.len() == 2 && words[0] == "toxic" {
                let amount = words[1].parse::<u32>().ok().unwrap_or(1);
                return Some(KeywordAction::Toxic(amount));
            }
            if words.len() >= 2 {
                if matches!((head, second), ("first", Some("strike"))) {
                    if words.len() > 2 && slice_contains(&words, &"and") {
                        return None;
                    }
                    return Some(KeywordAction::FirstStrike);
                }
                if matches!((head, second), ("double", Some("strike"))) {
                    if words.len() > 2 && slice_contains(&words, &"and") {
                        return None;
                    }
                    return Some(KeywordAction::DoubleStrike);
                }
                if matches!((head, second), ("protection", Some("from"))) && words.len() >= 3 {
                    let value = words[2];
                    return if let Some(color) = parse_color(value) {
                        Some(KeywordAction::ProtectionFrom(color))
                    } else if value == "everything" {
                        Some(KeywordAction::ProtectionFromEverything)
                    } else {
                        parse_card_type(value)
                            .map(KeywordAction::ProtectionFromCardType)
                            .or_else(|| {
                                parse_subtype_flexible(value)
                                    .map(KeywordAction::ProtectionFromSubtype)
                            })
                    };
                }
            }
            if words.len() >= 3 {
                let suffix = &words[words.len() - 3..];
                if suffix == ["cant", "be", "blocked"] || suffix == ["cannot", "be", "blocked"] {
                    return Some(KeywordAction::Unblockable);
                }
            }
            return None;
        }
    };

    Some(action)
}

pub(crate) fn rewrite_attached_controller_trigger_effect_tokens(
    trigger_tokens: &[OwnedLexToken],
    effects_tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    let trigger_words = crate::cards::builders::compiler::token_word_refs(trigger_tokens);
    let references_enchanted_controller = find_window_by(&trigger_words, 3, |window| {
        window[0] == "enchanted"
            && matches!(
                window[1],
                "creature"
                    | "creatures"
                    | "permanent"
                    | "permanents"
                    | "artifact"
                    | "artifacts"
                    | "enchantment"
                    | "enchantments"
                    | "land"
                    | "lands"
            )
            && window[2] == "controller"
    })
    .is_some();
    if !references_enchanted_controller {
        return effects_tokens.to_vec();
    }

    let mut rewritten = Vec::with_capacity(effects_tokens.len());
    let mut idx = 0usize;
    while idx < effects_tokens.len() {
        if idx + 1 < effects_tokens.len()
            && effects_tokens[idx].is_word("that")
            && effects_tokens[idx + 1].is_word("creature")
        {
            let first_span = effects_tokens[idx].span();
            let second_span = effects_tokens[idx + 1].span();
            rewritten.push(OwnedLexToken::word("enchanted".to_string(), first_span));
            rewritten.push(OwnedLexToken::word("creature".to_string(), second_span));
            idx += 2;
            continue;
        }
        if idx + 1 < effects_tokens.len()
            && effects_tokens[idx].is_word("that")
            && effects_tokens[idx + 1].is_word("permanent")
        {
            let first_span = effects_tokens[idx].span();
            let second_span = effects_tokens[idx + 1].span();
            rewritten.push(OwnedLexToken::word("enchanted".to_string(), first_span));
            rewritten.push(OwnedLexToken::word("permanent".to_string(), second_span));
            idx += 2;
            continue;
        }
        rewritten.push(effects_tokens[idx].clone());
        idx += 1;
    }

    rewritten
}

pub(crate) fn maybe_strip_leading_damage_subject_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    if matches!(
        words.get(..2),
        Some(["it", "deals"]) | Some(["this", "deals"])
    ) && !tokens.is_empty()
    {
        return Some(&tokens[1..]);
    }
    None
}

pub(crate) fn looks_like_trigger_object_list_tail(tokens: &[OwnedLexToken]) -> bool {
    if tokens.is_empty() {
        return false;
    }

    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    if words.is_empty() {
        return false;
    }

    let starts_with_or = words.first().copied() == Some("or");
    let first_candidate = if starts_with_or {
        words.get(1).copied()
    } else {
        words.first().copied()
    };
    let Some(first_word) = first_candidate else {
        return false;
    };

    let type_like = parse_card_type(first_word).is_some()
        || parse_subtype_word(first_word).is_some()
        || str_strip_suffix(first_word, "s").is_some_and(|stem| {
            parse_card_type(stem).is_some() || parse_subtype_word(stem).is_some()
        });
    if !type_like {
        return false;
    }

    tokens.iter().any(|token| token.is_comma())
}

pub(crate) fn looks_like_trigger_discard_qualifier_tail(
    trigger_prefix_tokens: &[OwnedLexToken],
    tail_tokens: &[OwnedLexToken],
) -> bool {
    if tail_tokens.is_empty() {
        return false;
    }

    let prefix_words = crate::cards::builders::compiler::token_word_refs(trigger_prefix_tokens);
    if !(slice_contains(&prefix_words, &"discard") || slice_contains(&prefix_words, &"discards")) {
        return false;
    }

    let tail_words = crate::cards::builders::compiler::token_word_refs(tail_tokens);
    if tail_words.is_empty() {
        return false;
    }

    let Some(first_word) = tail_words.first().copied() else {
        return false;
    };
    let typeish = parse_card_type(first_word).is_some()
        || parse_non_type(first_word).is_some()
        || matches!(first_word, "and" | "or");
    if !typeish {
        return false;
    }

    find_index(tail_tokens, |token| token.is_comma()).is_some_and(|comma_idx| {
        let before_words =
            crate::cards::builders::compiler::token_word_refs(&tail_tokens[..comma_idx]);
        slice_contains(&before_words, &"card") || slice_contains(&before_words, &"cards")
    })
}

pub(crate) fn looks_like_trigger_type_list_tail(tokens: &[OwnedLexToken]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    if words.is_empty() {
        return false;
    }
    let first_is_card_type = parse_card_type(words[0]).is_some()
        || parse_subtype_word(words[0]).is_some()
        || str_strip_suffix(words[0], "s").is_some_and(|word| {
            parse_card_type(word).is_some() || parse_subtype_word(word).is_some()
        });
    first_is_card_type
        && words.iter().any(|word| matches!(*word, "spell" | "spells"))
        && words.iter().any(|word| *word == "or")
        && tokens.iter().any(|token| token.is_comma())
}

pub(crate) fn looks_like_trigger_color_list_tail(tokens: &[OwnedLexToken]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    if words.is_empty() {
        return false;
    }
    is_basic_color_word(words[0])
        && words.iter().any(|word| *word == "or")
        && tokens.iter().any(|token| token.is_comma())
}

pub(crate) fn looks_like_trigger_numeric_list_tail(tokens: &[OwnedLexToken]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    if words.len() < 3 {
        return false;
    }
    if words[0].parse::<i32>().is_err() {
        return false;
    }
    let has_second_number = words.iter().skip(1).any(|word| word.parse::<i32>().is_ok());
    has_second_number && words.iter().any(|word| *word == "or")
}

pub(crate) fn is_trigger_objectish_word(word: &str) -> bool {
    parse_card_type(word).is_some()
        || parse_subtype_word(word).is_some()
        || str_strip_suffix(word, "s").is_some_and(|stem| {
            parse_card_type(stem).is_some() || parse_subtype_word(stem).is_some()
        })
}

