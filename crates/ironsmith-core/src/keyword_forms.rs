//! The one lexicon of keyword surface forms.
//!
//! A keyword's name appears in rules text in several context-dependent forms:
//! the bare name ("Foretell"), a past participle or designation ("foretold",
//! "kicked", "beamed up"), the head of its cost noun ("its foretell cost",
//! "its upkeep cost"), and self-references ("Equip only as a sorcery"). The
//! parser, the reminder-cost binder, custom-keyword substitution, and the
//! keyword audit all read these forms from this table rather than spelling
//! them out locally.
//!
//! Forms are vocabulary, never identity: recognizing "warped" says which
//! keyword a phrase talks about, not that an object has that keyword.
//!
//! Custom keywords are not in the table. Their forms come from
//! [`derive_participle`] (the same rule the web UI uses, checked by
//! [`DERIVATION_FIXTURES`]) and the name itself as cost noun and
//! self-reference, unless the custom-mechanic definition overrides them.

/// Surface forms of one official keyword. All strings are lowercase; multiword
/// forms separate words with single spaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeywordForms {
    /// The keyword's printed name ("foretell", "beam me up"). The name is
    /// also its self-reference ("Equip only as a sorcery").
    pub name: &'static str,
    /// Past participle in "was X" / "becomes X" ("foretold", "beamed up").
    pub participle: Option<&'static str>,
    /// Adjective an object or spell carries as a designation ("a kicked
    /// spell", "a foretold card", "monstrous"). Often equal to the participle.
    pub designation: Option<&'static str>,
    /// Head of the keyword's named cost ("upkeep" in "its upkeep cost").
    pub cost_noun: Option<&'static str>,
    /// Other phrasal or reflexive forms that refer to the keyword.
    pub variants: &'static [&'static str],
}

impl KeywordForms {
    const fn named(name: &'static str) -> Self {
        Self { name, participle: None, designation: None, cost_noun: None, variants: &[] }
    }
    const fn participle(mut self, participle: &'static str) -> Self {
        self.participle = Some(participle);
        self
    }
    /// The participle doubles as the designation adjective.
    const fn designates(mut self, participle: &'static str) -> Self {
        self.participle = Some(participle);
        self.designation = Some(participle);
        self
    }
    const fn designation(mut self, designation: &'static str) -> Self {
        self.designation = Some(designation);
        self
    }
    /// The cost noun is the name itself ("its foretell cost").
    const fn costed(mut self) -> Self {
        self.cost_noun = Some(self.name);
        self
    }
    const fn cost_noun(mut self, noun: &'static str) -> Self {
        self.cost_noun = Some(noun);
        self
    }
    const fn variants(mut self, variants: &'static [&'static str]) -> Self {
        self.variants = variants;
        self
    }

    /// Every surface form, name first, without duplicates.
    pub fn surface_forms(&self) -> Vec<&'static str> {
        let mut forms = vec![self.name];
        for form in [self.participle, self.designation].into_iter().flatten() {
            if !forms.contains(&form) {
                forms.push(form);
            }
        }
        for form in self.variants {
            if !forms.contains(form) {
                forms.push(form);
            }
        }
        forms
    }
}

// Forms the grammar matches directly. Each is also the corresponding table
// entry's value, so a parser site that names one of these names the table.
pub const BARGAINED: &str = "bargained";
pub const BEAMED_UP: &str = "beamed up";
pub const BEHELD: &str = "beheld";
pub const FORETOLD: &str = "foretold";
pub const GOADED: &str = "goaded";
pub const KICKED: &str = "kicked";
pub const PLOTTED: &str = "plotted";
pub const SADDLED: &str = "saddled";
pub const SUSPECTED: &str = "suspected";
pub const SUSPENDED: &str = "suspended";
pub const WARPED: &str = "warped";

const fn k(name: &'static str) -> KeywordForms {
    KeywordForms::named(name)
}

/// Official keywords whose forms differ from the bare name. Keywords absent
/// here only ever appear by name. Hand-maintained; ordered by CR number.
pub const OFFICIAL_KEYWORD_FORMS: &[KeywordForms] = &[
    // 701 keyword actions.
    k("behold").participle(BEHELD),
    k("counter").participle("countered"),
    k("destroy").participle("destroyed"),
    k("discard").participle("discarded"),
    k("exile").participle("exiled"),
    k("goad").designates(GOADED),
    k("investigate").participle("investigated"),
    k("mill").participle("milled"),
    k("regenerate").participle("regenerated"),
    k("reveal").participle("revealed"),
    k("sacrifice").participle("sacrificed"),
    k("shuffle").participle("shuffled"),
    k("surveil").participle("surveilled"),
    k("tap").designates("tapped"),
    k("untap").designates("untapped"),
    k("transform").designates("transformed"),
    k("convert").participle("converted"),
    k("monstrosity").designation("monstrous"),
    k("vote").participle("voted"),
    k("manifest").participle("manifested"),
    k("exert").participle("exerted"),
    k("explore").participle("explored"),
    k("amass").participle("amassed"),
    k("connive").participle("connived"),
    k("discover").participle("discovered"),
    k("cloak").participle("cloaked"),
    k("collect evidence").participle("collected evidence"),
    k("suspect").designates(SUSPECTED),
    k("harness").designates("harnessed"),
    k("endure").participle("endured"),
    k("blight").participle("blighted"),
    // 702 keyword abilities.
    k("equip").costed(),
    k("cumulative upkeep").cost_noun("upkeep"),
    k("phasing").variants(&["phased in", "phased out"]),
    k("buyback").costed(),
    k("cycling").costed(),
    k("echo").costed(),
    k("kicker").designates(KICKED).costed(),
    k("multikicker").costed(),
    k("flashback").costed(),
    k("madness").costed(),
    k("morph").costed(),
    k("megamorph").costed(),
    k("entwine").costed(),
    k("splice").participle("spliced").costed(),
    k("ninjutsu").costed(),
    k("convoke").participle("convoked"),
    k("replicate").participle("replicated").costed(),
    k("fortify").participle("fortified"),
    k("suspend").designation(SUSPENDED),
    k("champion").participle("championed"),
    k("evoke").participle("evoked").costed(),
    k("prowl").costed(),
    k("devour").participle("devoured"),
    k("unearth").costed(),
    k("miracle").costed(),
    k("soulbond").variants(&["paired"]),
    k("overload").costed(),
    k("scavenge").costed(),
    k("cipher").variants(&["encoded"]),
    k("fuse").designates("fused"),
    k("bestow").participle("bestowed").costed(),
    k("dash").participle("dashed").costed(),
    k("exploit").participle("exploited"),
    k("renown").designation("renowned"),
    k("awaken").participle("awakened").costed(),
    k("surge").costed(),
    k("emerge").costed(),
    k("crew").participle("crewed"),
    k("improvise").participle("improvised"),
    k("embalm").participle("embalmed").costed(),
    k("eternalize").costed(),
    k("spectacle").costed(),
    k("escape").participle("escaped").costed(),
    k("mutate").participle("mutated").costed(),
    k("encore").costed(),
    k("foretell").designates(FORETOLD).costed(),
    k("disturb").costed(),
    k("cleave").costed(),
    k("blitz").participle("blitzed").costed(),
    k("enlist").participle("enlisted"),
    k("squad").costed(),
    k("disguise").costed(),
    k("bargain").participle(BARGAINED),
    k("plot").designates(PLOTTED).costed(),
    k("saddle").designates(SADDLED),
    k("freerunning").costed(),
    k("gift").participle("gifted"),
    k("offspring").costed(),
    k("impending").costed(),
    k("harmonize").costed(),
    k("warp").designates(WARPED).costed(),
    k("mayhem").costed(),
    k("web-slinging").costed(),
    k("sneak").costed(),
    k("beam me up").participle(BEAMED_UP).costed(),
];

/// Every CR 702 keyword ability name, lowercase, in rule order, as the parser
/// sees its words (punctuation such as "!" dropped). "Daybound and
/// Nightbound" lists its two keywords separately; "∞" has no name to write.
pub const OFFICIAL_KEYWORD_ABILITIES: &[&str] = &[
    "deathtouch", "defender", "double strike", "enchant", "equip", "first strike", "flash",
    "flying", "haste", "hexproof", "indestructible", "intimidate", "landwalk", "lifelink",
    "protection", "reach", "shroud", "trample", "vigilance", "ward", "banding", "rampage",
    "cumulative upkeep", "flanking", "phasing", "buyback", "shadow", "cycling", "echo",
    "horsemanship", "fading", "kicker", "flashback", "madness", "fear", "morph", "amplify",
    "provoke", "storm", "affinity", "entwine", "modular", "sunburst", "bushido", "soulshift",
    "splice", "offering", "ninjutsu", "epic", "convoke", "dredge", "transmute", "bloodthirst",
    "haunt", "replicate", "forecast", "graft", "recover", "ripple", "split second", "suspend",
    "vanishing", "absorb", "aura swap", "delve", "fortify", "frenzy", "gravestorm", "poisonous",
    "transfigure", "champion", "changeling", "evoke", "hideaway", "prowl", "reinforce",
    "conspire", "persist", "wither", "retrace", "devour", "exalted", "unearth", "cascade",
    "annihilator", "level up", "rebound", "umbra armor", "infect", "battle cry",
    "living weapon", "undying", "miracle", "soulbond", "overload", "scavenge", "unleash",
    "cipher", "evolve", "extort", "fuse", "bestow", "tribute", "dethrone", "hidden agenda",
    "outlast", "prowess", "dash", "exploit", "menace", "renown", "awaken", "devoid", "ingest",
    "myriad", "surge", "skulk", "emerge", "escalate", "melee", "crew", "fabricate", "partner",
    "undaunted", "improvise", "aftermath", "embalm", "eternalize", "afflict", "ascend",
    "assist", "jump-start", "mentor", "afterlife", "riot", "spectacle", "escape", "companion",
    "mutate", "encore", "boast", "foretell", "demonstrate", "daybound", "nightbound",
    "disturb", "decayed", "cleave", "training", "compleated", "reconfigure", "blitz",
    "casualty", "enlist", "read ahead", "ravenous", "squad", "space sculptor", "visit",
    "prototype", "living metal", "more than meets the eye", "for mirrodin", "toxic", "backup",
    "bargain", "craft", "disguise", "solved", "plot", "saddle", "spree", "freerunning", "gift",
    "offspring", "impending", "exhaust", "max speed", "start your engines", "harmonize",
    "mobilize", "job select", "tiered", "station", "warp", "mayhem", "web-slinging",
    "firebending", "sneak", "increment", "paradigm", "power-up", "teamwork", "storied",
];

/// The official keyword ability whose name is exactly `words`. Hyphens and
/// spaces are interchangeable, since the lexer splits "jump-start".
pub fn official_keyword_ability(words: &[&str]) -> Option<&'static str> {
    let spelled = words.join(" ").replace('-', " ");
    OFFICIAL_KEYWORD_ABILITIES
        .iter()
        .copied()
        .find(|name| name.replace('-', " ").eq_ignore_ascii_case(&spelled))
}

/// The official keyword ability a printed keyword line starts with ("Echo
/// {2}{R}" → echo, "Level up {1}" → level up), longest name first.
pub fn official_keyword_ability_prefix(text: &str) -> Option<&'static str> {
    let words = text
        .split(|ch: char| ch.is_whitespace() || ch == '—')
        .map(|word| word.trim_matches(|ch: char| !(ch.is_alphanumeric() || ch == '-')))
        .take_while(|word| !word.is_empty())
        .collect::<Vec<_>>();
    (1..=words.len().min(5)).rev().find_map(|len| official_keyword_ability(&words[..len]))
}

/// Official participles the custom-keyword rule cannot produce. Every other
/// table participle must equal [`derive_participle`] of its name.
pub const IRREGULAR_PARTICIPLES: &[&str] =
    &[BEHELD, "surveilled", "untapped", FORETOLD, KICKED, BEAMED_UP];

/// `(custom name, derived participle)` pairs both implementations of the
/// derivation rule must agree on.
pub const DERIVATION_FIXTURES: &[(&str, &str)] = &[
    ("blink", "blinked"),
    ("shift", "shifted"),
    ("plan", "planned"),
    ("stir", "stirred"),
    ("glow", "glowed"),
    ("fix", "fixed"),
    ("bury", "buried"),
    ("relay", "relayed"),
    ("forge", "forged"),
    ("panic", "panicked"),
    ("visit", "visited"),
    ("moonpulse", "moonpulsed"),
    ("rise up", "risen up"),
    ("lock away", "locked away"),
    ("decayed", "decayed"),
];

/// Irregular heads the derivation rule knows. Kept small; anything else
/// irregular belongs in a custom definition's explicit override.
const IRREGULAR_HEADS: &[(&str, &str)] = &[
    ("rise", "risen"),
    ("tell", "told"),
    ("hold", "held"),
    ("fight", "fought"),
    ("bind", "bound"),
    ("break", "broken"),
    ("hide", "hidden"),
    ("steal", "stolen"),
];

/// The official entry named `name` (case-insensitive).
pub fn official(name: &str) -> Option<&'static KeywordForms> {
    let name = name.trim();
    OFFICIAL_KEYWORD_FORMS.iter().find(|forms| forms.name.eq_ignore_ascii_case(name))
}

/// The official entry whose participle is `word`.
pub fn by_participle(word: &str) -> Option<&'static KeywordForms> {
    OFFICIAL_KEYWORD_FORMS
        .iter()
        .find(|forms| forms.participle.is_some_and(|form| form.eq_ignore_ascii_case(word)))
}

/// Whether `word` is an official keyword's participle.
pub fn is_participle(word: &str) -> bool {
    by_participle(word).is_some()
}

/// Official entries whose named cost has `noun` as its head.
pub fn by_cost_noun(noun: &str) -> impl Iterator<Item = &'static KeywordForms> + '_ {
    OFFICIAL_KEYWORD_FORMS
        .iter()
        .filter(move |forms| forms.cost_noun.is_some_and(|form| form.eq_ignore_ascii_case(noun)))
}

/// Whether `words` spell the multiword form `form` exactly.
pub fn words_spell(words: &[&str], form: &str) -> bool {
    let mut expected = form.split(' ');
    words.iter().all(|word| expected.next() == Some(*word)) && expected.next().is_none()
}

/// The custom-keyword participle rule: inflect the first word as a regular
/// English past participle and keep the rest ("lock away" -> "locked away").
pub fn derive_participle(name: &str) -> String {
    let name = name.trim().to_ascii_lowercase();
    let (head, rest) = name.split_once(' ').unwrap_or((name.as_str(), ""));
    let mut out = inflect_head(head);
    if !rest.is_empty() {
        out.push(' ');
        out.push_str(rest);
    }
    out
}

fn inflect_head(word: &str) -> String {
    if let Some((_, participle)) = IRREGULAR_HEADS.iter().find(|(head, _)| *head == word) {
        return (*participle).to_string();
    }
    let chars: Vec<char> = word.chars().collect();
    let vowel = |c: char| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u');
    let n = chars.len();
    if word.ends_with("ed") {
        return word.to_string();
    }
    if word.ends_with('e') {
        return format!("{word}d");
    }
    if n >= 2 && chars[n - 1] == 'y' && !vowel(chars[n - 2]) {
        return format!("{}ied", &word[..word.len() - 1]);
    }
    if word.ends_with('c') {
        return format!("{word}ked");
    }
    let vowel_groups = chars
        .iter()
        .enumerate()
        .filter(|(i, c)| vowel(**c) && (*i == 0 || !vowel(chars[i - 1])))
        .count();
    if n >= 3
        && vowel_groups == 1
        && !vowel(chars[n - 3])
        && vowel(chars[n - 2])
        && !vowel(chars[n - 1])
        && !matches!(chars[n - 1], 'w' | 'x' | 'y')
    {
        return format!("{word}{}ed", chars[n - 1]);
    }
    format!("{word}ed")
}

/// The generated web catalog (`web/ui/src/lib/keyword-forms.generated.js`).
pub fn render_js_catalog() -> String {
    fn quoted(value: Option<&str>) -> String {
        value.map_or_else(|| "null".to_string(), |value| format!("{value:?}"))
    }
    let mut out = String::from(
        "// Generated from crates/ironsmith-core/src/keyword_forms.rs.\n\
         // Regenerate: UPDATE_KEYWORD_FORMS=1 cargo test -p ironsmith-core --features serde --lib keyword_forms\n\n\
         export const OFFICIAL_KEYWORD_FORMS = Object.freeze([\n",
    );
    for forms in OFFICIAL_KEYWORD_FORMS {
        let variants =
            forms.variants.iter().map(|value| format!("{value:?}")).collect::<Vec<_>>().join(", ");
        out.push_str(&format!(
            "  Object.freeze({{ name: {:?}, participle: {}, designation: {}, costNoun: {}, variants: Object.freeze([{variants}]) }}),\n",
            forms.name,
            quoted(forms.participle),
            quoted(forms.designation),
            quoted(forms.cost_noun),
        ));
    }
    out.push_str("]);\n\nexport const KEYWORD_FORM_DERIVATION_FIXTURES = Object.freeze([\n");
    for (name, participle) in DERIVATION_FIXTURES {
        out.push_str(&format!("  [{name:?}, {participle:?}],\n"));
    }
    out.push_str("]);\n\nexport const KEYWORD_IRREGULAR_HEADS = Object.freeze({\n");
    for (head, participle) in IRREGULAR_HEADS {
        out.push_str(&format!("  {head:?}: {participle:?},\n"));
    }
    out.push_str("});\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_forms_derivation_rule_matches_fixtures_and_regular_official_participles() {
        for (name, participle) in DERIVATION_FIXTURES {
            assert_eq!(derive_participle(name), *participle, "{name}");
        }
        for forms in OFFICIAL_KEYWORD_FORMS {
            let Some(participle) = forms.participle else { continue };
            if IRREGULAR_PARTICIPLES.contains(&participle) {
                assert_ne!(derive_participle(forms.name), participle, "{} is regular", forms.name);
            } else {
                assert_eq!(derive_participle(forms.name), participle, "{}", forms.name);
            }
        }
    }

    #[test]
    fn keyword_forms_are_unambiguous_lowercase_vocabulary() {
        let mut seen = std::collections::HashMap::new();
        for forms in OFFICIAL_KEYWORD_FORMS {
            for form in forms.surface_forms().into_iter().chain(forms.cost_noun) {
                assert_eq!(form, form.to_ascii_lowercase());
                assert!(!form.contains("  ") && form.trim() == form);
            }
            for form in [forms.participle, forms.designation].into_iter().flatten() {
                if let Some(owner) = seen.insert(form, forms.name) {
                    assert_eq!(owner, forms.name, "{form} names two keywords");
                }
            }
        }
        assert_eq!(by_participle("kicked").map(|forms| forms.name), Some("kicker"));
        assert_eq!(by_cost_noun("upkeep").next().map(|forms| forms.name), Some("cumulative upkeep"));
        assert!(words_spell(&["beamed", "up"], BEAMED_UP));
        assert!(!words_spell(&["beamed"], BEAMED_UP));
    }

    #[test]
    fn keyword_forms_js_catalog_is_in_sync() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../web/ui/src/lib/keyword-forms.generated.js");
        let rendered = render_js_catalog();
        if std::env::var_os("UPDATE_KEYWORD_FORMS").is_some() {
            std::fs::write(&path, &rendered).unwrap();
        }
        let current = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            current == rendered,
            "{} is stale; rerun with UPDATE_KEYWORD_FORMS=1",
            path.display()
        );
    }
}
