use super::*;

/// A suspected and↔or flip between an oracle clause and its best-matching
/// compiled clause: the same content-word neighbors joined by a different
/// conjunction.  "and"/"or" are comparison stopwords, so the similarity score
/// cannot see these — this targeted invariant exists to surface them.
#[derive(Debug, Clone, PartialEq)]
pub struct ConjunctionFlip {
    pub left: String,
    pub right: String,
    pub oracle_conjunction: String,
    pub compiled_conjunction: String,
    pub oracle_clause: String,
    pub compiled_clause: String,
}

/// Conjunction triples (left-content-word, and|or, right-content-word) in a
/// clause.  "and/or" and mass-quantified clauses are excluded by the caller.
fn conjunction_triples(clause: &str) -> Vec<(String, String, String)> {
    let tokens = tokenize_text(clause);
    let mut triples = Vec::new();
    for (idx, token) in tokens.iter().enumerate() {
        if token != "and" && token != "or" {
            continue;
        }
        let left = tokens[..idx]
            .iter()
            .rev()
            .filter_map(|t| normalize_word(t))
            .find(|t| !is_stopword(t));
        let right = tokens[idx + 1..]
            .iter()
            .filter_map(|t| normalize_word(t))
            .find(|t| !is_stopword(t));
        if let (Some(left), Some(right)) = (left, right) {
            triples.push((left, token.clone(), right));
        }
    }
    triples
}

/// True when a clause quantifies over every matching object; in mass contexts
/// oracle idiom uses "and" for what a filter expresses with "or" ("destroy
/// all artifacts and enchantments"), so flips there are legitimate variance.
fn clause_has_mass_quantifier(clause: &str) -> bool {
    tokenize_text(clause)
        .iter()
        .any(|t| matches!(t.as_str(), "all" | "each" | "every"))
}

/// Detect and↔or flips between oracle text and compiled text.  Clauses are
/// paired by comparison-token overlap; a flip is reported when the same
/// (left, right) content-word pair is joined by "and" on one side and "or"
/// on the other.  Conservative by design: mass-quantified clauses and
/// "and/or" surfaces are skipped.
pub fn conjunction_flips_between(oracle: &str, compiled: &str) -> Vec<ConjunctionFlip> {
    let oracle_clauses = semantic_clauses_for_compare(oracle);
    let compiled_clauses = semantic_clauses_for_compare(compiled);
    let oracle_tokens: Vec<std::collections::HashSet<String>> = oracle_clauses
        .iter()
        .map(|c| comparison_tokens(c).into_iter().collect())
        .collect();

    let mut flips = Vec::new();
    for compiled_clause in &compiled_clauses {
        if compiled_clause.contains("and/or") || compiled_clause.contains("and or ") {
            continue;
        }
        let compiled_set: std::collections::HashSet<String> =
            comparison_tokens(compiled_clause).into_iter().collect();
        if compiled_set.is_empty() {
            continue;
        }
        let best = oracle_clauses
            .iter()
            .enumerate()
            .map(|(idx, clause)| {
                let inter = oracle_tokens[idx].intersection(&compiled_set).count() as f32;
                let union = oracle_tokens[idx].union(&compiled_set).count() as f32;
                (if union > 0.0 { inter / union } else { 0.0 }, clause)
            })
            .max_by(|a, b| a.0.total_cmp(&b.0));
        let Some((overlap, oracle_clause)) = best else {
            continue;
        };
        if overlap < 0.4 || oracle_clause.contains("and/or") || oracle_clause.contains("and or ") {
            continue;
        }
        if clause_has_mass_quantifier(oracle_clause) || clause_has_mass_quantifier(compiled_clause)
        {
            continue;
        }
        let oracle_triples = conjunction_triples(oracle_clause);
        for (left, conj, right) in conjunction_triples(compiled_clause) {
            if let Some((_, oracle_conj, _)) = oracle_triples
                .iter()
                .find(|(l, c, r)| *l == left && *r == right && *c != conj)
            {
                flips.push(ConjunctionFlip {
                    left: left.clone(),
                    right: right.clone(),
                    oracle_conjunction: oracle_conj.clone(),
                    compiled_conjunction: conj.clone(),
                    oracle_clause: oracle_clause.clone(),
                    compiled_clause: compiled_clause.clone(),
                });
            }
        }
    }
    flips
}

pub(super) fn collapse_repeated_tokens(tokens: Vec<String>) -> Vec<String> {
    let mut collapsed = Vec::with_capacity(tokens.len());
    for token in tokens {
        if collapsed.last() != Some(&token) {
            collapsed.push(token);
        }
    }
    collapsed
}

pub(super) fn collapse_named_reference_tokens(tokens: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::with_capacity(tokens.len());
    let mut idx = 0usize;

    while idx < tokens.len() {
        if tokens[idx] != "nam" {
            normalized.push(tokens[idx].clone());
            idx += 1;
            continue;
        }

        normalized.push("nam".to_string());
        idx += 1;
        while idx < tokens.len() && !is_named_reference_boundary(&tokens[idx]) {
            idx += 1;
        }
    }

    normalized
}

fn is_named_reference_boundary(token: &str) -> bool {
    matches!(
        token,
        "from"
            | "to"
            | "into"
            | "in"
            | "on"
            | "at"
            | "under"
            | "over"
            | "with"
            | "for"
            | "if"
            | "unless"
            | "while"
            | "until"
            | "except"
            | "despite"
            | "of"
            | "that"
            | "this"
            | "it"
            | "its"
            | "they"
            | "their"
            | "them"
            | "you"
            | "your"
            | "controller"
            | "owner"
            | "each"
            | "all"
            | "any"
            | "graveyard"
            | "graveyards"
            | "battlefield"
            | "library"
            | "hand"
            | "permanent"
            | "permanents"
            | "card"
            | "cards"
            | "artifact"
            | "creature"
            | "enchantment"
            | "planeswalker"
            | "land"
            | "token"
            | "spell"
            | "player"
            | "opponent"
            | "target"
            | "and"
            | "or"
    )
}

pub(super) fn normalize_turn_frequency_scaffolding(tokens: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::with_capacity(tokens.len());
    let mut idx = 0;

    while idx < tokens.len() {
        let token = &tokens[idx];
        if token == "only"
            && idx + 3 < tokens.len()
            && (tokens[idx + 1] == "once" || tokens[idx + 1] == "twice")
            && tokens[idx + 2] == "each"
            && tokens[idx + 3] == "turn"
        {
            idx += 4;
            continue;
        }

        normalized.push(token.to_string());
        idx += 1;
    }

    normalized
}

fn is_effect_token(token: &str) -> bool {
    token == "<num>"
        || token.chars().all(|ch| ch.is_ascii_digit())
        || (token.starts_with('#')
            && token.len() > 1
            && token.chars().nth(1).is_some_and(|ch| ch.is_ascii_digit()))
}

fn normalize_internal_compiler_scaffolding(tokens: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::with_capacity(tokens.len());
    let mut idx = 0;

    while idx < tokens.len() {
        let Some(token) = tokens.get(idx).map(String::as_str) else {
            break;
        };

        if token == "if" {
            let remaining = &tokens[idx..];
            if remaining.len() >= 3
                && (remaining[1] == "doesnt" || remaining[1] == "doesn't")
                && remaining[2] == "happen"
            {
                idx += 3;
                continue;
            }
            if remaining.len() >= 5
                && remaining[1] == "effect"
                && is_effect_token(&remaining[2])
                && (remaining[3] == "doesnt" || remaining[3] == "doesn't")
                && remaining[4] == "happen"
            {
                idx += 5;
                continue;
            }
            if remaining.len() >= 6
                && remaining[1] == "effect"
                && is_effect_token(&remaining[2])
                && remaining[3] == "that"
                && (remaining[4] == "doesnt" || remaining[4] == "doesn't")
                && remaining[5] == "happen"
            {
                idx += 6;
                continue;
            }
            if remaining.len() >= 4
                && remaining[1] == "effect"
                && is_effect_token(&remaining[2])
                && remaining[3] == "happen"
            {
                idx += 4;
                continue;
            }
        }

        if token == "count"
            && tokens.len() >= idx + 5
            && tokens[idx + 1] == "result"
            && tokens[idx + 2] == "of"
            && tokens[idx + 3] == "effect"
            && is_effect_token(&tokens[idx + 4])
        {
            idx += 5;
            continue;
        }

        normalized.push(token.to_string());
        idx += 1;
    }

    normalized
}

pub(super) fn normalize_that_references(tokens: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::with_capacity(tokens.len());
    let mut idx = 0;
    while idx < tokens.len() {
        let token = &tokens[idx];
        let should_skip = token == "that"
            && idx + 1 < tokens.len()
            && matches!(
                tokens[idx + 1].as_str(),
                "card"
                    | "creature"
                    | "artifact"
                    | "enchantment"
                    | "permanent"
                    | "land"
                    | "planeswalker"
                    | "player"
                    | "spell"
                    | "object"
                    | "aura"
                    | "token"
                    | "battlefield"
                    | "controller"
                    | "owner"
                    | "mana"
            );
        if should_skip {
            idx += 1;
            continue;
        }
        normalized.push(token.to_string());
        idx += 1;
    }
    normalized
}

pub(super) fn compiled_comparison_tokens(clause: &str) -> Vec<String> {
    let comparable_clause = normalize_explicit_damage_source_for_compare(clause);
    let tokens = tokenize_text(&comparable_clause)
        .into_iter()
        .filter_map(|token| normalize_word(&token))
        .collect();
    let tokens = collapse_named_reference_tokens(tokens);
    let tokens = collapse_repeated_tokens(tokens);
    let tokens = normalize_turn_frequency_scaffolding(tokens);
    let tokens = normalize_internal_compiler_scaffolding(tokens);
    normalize_that_references(tokens)
        .into_iter()
        .filter(|token| !is_stopword(token))
        .collect()
}

fn embedding_tokens(clause: &str) -> Vec<String> {
    let comparable_clause = normalize_explicit_damage_source_for_compare(clause);
    tokenize_text(&comparable_clause)
        .into_iter()
        .filter_map(|token| normalize_word(&token))
        .collect()
}

fn hash_index(feature: &str, dims: usize) -> usize {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    feature.hash(&mut hasher);
    (hasher.finish() as usize) % dims.max(1)
}

fn hash_sign(feature: &str) -> f32 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    ("sign", feature).hash(&mut hasher);
    if hasher.finish() & 1 == 0 { 1.0 } else { -1.0 }
}

fn add_feature(vec: &mut [f32], feature: &str, weight: f32) {
    let idx = hash_index(feature, vec.len());
    vec[idx] += hash_sign(feature) * weight;
}

fn l2_normalize(vec: &mut [f32]) {
    let norm = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in vec {
            *v /= norm;
        }
    }
}

fn embed_clause(clause: &str, dims: usize) -> Vec<f32> {
    let mut vec = vec![0.0f32; dims.max(1)];
    let tokens = embedding_tokens(clause);

    for token in &tokens {
        add_feature(&mut vec, &format!("u:{token}"), 1.0);
    }
    for window in tokens.windows(2) {
        add_feature(&mut vec, &format!("b:{}|{}", window[0], window[1]), 0.85);
    }
    for window in tokens.windows(3) {
        add_feature(
            &mut vec,
            &format!("t:{}|{}|{}", window[0], window[1], window[2]),
            1.0,
        );
    }

    // Structural anchors for common semantic clauses.
    let lower = clause.to_ascii_lowercase();
    for marker in ["where", "plus", "minus", "for each", "as long as", "unless"] {
        if lower.contains(marker) {
            add_feature(&mut vec, &format!("m:{marker}"), 1.8);
        }
    }

    // Lightweight character n-grams help when token sets are similar but syntax differs.
    let compact = lower
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == ' ')
        .collect::<String>();
    let chars: Vec<char> = compact.chars().collect();
    for ngram in chars.windows(4).take(200) {
        let key = ngram.iter().collect::<String>();
        add_feature(&mut vec, &format!("c:{key}"), 0.2);
    }

    l2_normalize(&mut vec);
    vec
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    if len == 0 {
        return 0.0;
    }
    let mut dot = 0.0f32;
    for i in 0..len {
        dot += a[i] * b[i];
    }
    dot.clamp(-1.0, 1.0)
}

fn directional_embedding_coverage(from: &[Vec<f32>], to: &[Vec<f32>]) -> f32 {
    if from.is_empty() {
        return if to.is_empty() { 1.0 } else { 0.0 };
    }

    let mut total = 0.0f32;
    for source in from {
        let mut best = -1.0f32;
        for target in to {
            let score = cosine_similarity(source, target);
            if score > best {
                best = score;
            }
        }
        total += best.max(0.0);
    }
    total / from.len() as f32
}

fn jaccard_similarity(a: &[String], b: &[String]) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let a_set: std::collections::HashSet<&str> = a.iter().map(String::as_str).collect();
    let b_set: std::collections::HashSet<&str> = b.iter().map(String::as_str).collect();
    let inter = a_set.intersection(&b_set).count() as f32;
    let union = a_set.union(&b_set).count() as f32;
    if union == 0.0 { 0.0 } else { inter / union }
}

fn tokens_match_subsetish(tokens: &[String], reference: &[String]) -> bool {
    tokens_match_subsetish_with_threshold(tokens, reference, 0.80)
}

fn tokens_match_subsetish_with_threshold(
    tokens: &[String],
    reference: &[String],
    threshold: f32,
) -> bool {
    if tokens.is_empty() || reference.is_empty() {
        return false;
    }
    let reference_set: std::collections::HashSet<&str> =
        reference.iter().map(String::as_str).collect();
    let overlapping_tokens = tokens
        .iter()
        .filter(|token| reference_set.contains(token.as_str()))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if overlapping_tokens.is_empty() {
        return false;
    }
    let has_non_placeholder_overlap = overlapping_tokens.iter().any(|token| {
        !matches!(token, &"<mana>" | &"<num>" | &"<pt>")
            && !is_number_token(token)
            && !is_pt_token(token)
            && !(token.starts_with('{') && token.ends_with('}'))
    });
    if !has_non_placeholder_overlap {
        return false;
    }
    (overlapping_tokens.len() as f32 / tokens.len() as f32) >= threshold
}

fn is_activation_restriction_reminder_clause(clause: &str) -> bool {
    let lower = clause.to_ascii_lowercase();
    let words = lower
        .split(|ch: char| {
            !ch.is_ascii_alphanumeric() && ch != '/' && ch != '+' && ch != '-' && ch != '\''
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    if words.is_empty() {
        return false;
    }
    if words[0] == "as" && words.len() > 1 {
        return false;
    }

    let has_activate = words.iter().any(|word| *word == "activate");
    if !has_activate {
        return false;
    }

    let has_only = words.iter().any(|word| *word == "only");
    let has_trigger_limit = words.iter().any(|word| *word == "turn") && has_only;
    let has_condition = words.iter().any(|word| *word == "if")
        || words.iter().any(|word| *word == "when")
        || words.iter().any(|word| *word == "as");
    has_only && (has_trigger_limit || has_condition)
}

fn remove_redundant_compiled_clauses(
    mut clauses: Vec<(String, Vec<String>)>,
) -> Vec<(String, Vec<String>)> {
    let mut filtered: Vec<(String, Vec<String>)> = Vec::new();
    'outer: for (clause, tokens) in clauses.drain(..) {
        let clause_key = normalize_clause_prefix_key(&clause);
        let mut idx = 0usize;
        while idx < filtered.len() {
            let existing_key = normalize_clause_prefix_key(&filtered[idx].0);
            if existing_key == clause_key {
                continue 'outer;
            }

            if clause_key.len() > existing_key.len()
                && clause_key.starts_with(&existing_key)
                && clause_key[existing_key.len()..]
                    .trim_start()
                    .starts_with("and ")
            {
                filtered.remove(idx);
                continue;
            }
            if existing_key.len() > clause_key.len()
                && existing_key.starts_with(&clause_key)
                && existing_key[clause_key.len()..]
                    .trim_start()
                    .starts_with("and ")
            {
                continue 'outer;
            }
            idx += 1;
        }
        filtered.push((clause, tokens));
    }
    filtered
}

fn normalize_clause_prefix_key(clause: &str) -> String {
    clause
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|ch: char| ch == '.' || ch == ',' || ch == ';' || ch == ':')
                .to_ascii_lowercase()
        })
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnlessPayPayerRole {
    You,
    NonYou,
}

fn unless_pay_payer_role(clause: &str) -> Option<UnlessPayPayerRole> {
    let lower = clause.to_ascii_lowercase();
    let (_, tail) = lower.split_once("unless ")?;
    let tokens = tokenize_text(tail);
    let pay_idx = tokens
        .iter()
        .position(|token| matches!(token.as_str(), "pay" | "pays" | "paying" | "paid"))?;
    if pay_idx == 0 {
        return None;
    }

    let payer_tokens = &tokens[..pay_idx];
    if payer_tokens
        .iter()
        .any(|token| matches!(token.as_str(), "you" | "your"))
    {
        return Some(UnlessPayPayerRole::You);
    }
    if payer_tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "opponent"
                | "player"
                | "that"
                | "they"
                | "controller"
                | "their"
                | "them"
                | "its"
                | "it"
        )
    }) {
        return Some(UnlessPayPayerRole::NonYou);
    }
    None
}

fn count_unless_pay_role_mismatches(
    oracle_clauses: &[String],
    oracle_tokens: &[Vec<String>],
    compiled_clauses: &[String],
    compiled_tokens: &[Vec<String>],
) -> usize {
    let mut mismatches = 0usize;

    for (idx, oracle_clause) in oracle_clauses.iter().enumerate() {
        let Some(oracle_role) = unless_pay_payer_role(oracle_clause) else {
            continue;
        };
        let Some(oracle_token_set) = oracle_tokens.get(idx) else {
            continue;
        };

        let mut best_match: Option<(usize, f32)> = None;
        for (compiled_idx, compiled_token_set) in compiled_tokens.iter().enumerate() {
            let score = jaccard_similarity(oracle_token_set, compiled_token_set);
            if best_match.is_none_or(|(_, best)| score > best) {
                best_match = Some((compiled_idx, score));
            }
        }

        let Some((compiled_idx, overlap)) = best_match else {
            continue;
        };

        // Require moderate lexical overlap so we only compare semantically related clauses.
        if overlap < 0.55 {
            continue;
        }

        let Some(compiled_clause) = compiled_clauses.get(compiled_idx) else {
            continue;
        };
        if let Some(compiled_role) = unless_pay_payer_role(compiled_clause)
            && compiled_role != oracle_role
        {
            mismatches += 1;
        }
    }

    mismatches
}

fn has_type_among_count_semantics(clause: &str) -> bool {
    let lower = clause.to_ascii_lowercase();
    lower.contains("for each ")
        && (lower.contains(" type among ") || lower.contains(" types among "))
}

fn count_type_among_count_mismatches(
    oracle_clauses: &[String],
    oracle_tokens: &[Vec<String>],
    compiled_clauses: &[String],
    compiled_tokens: &[Vec<String>],
) -> usize {
    let mut mismatches = 0usize;

    for (idx, oracle_clause) in oracle_clauses.iter().enumerate() {
        if !has_type_among_count_semantics(oracle_clause) {
            continue;
        }
        let Some(oracle_token_set) = oracle_tokens.get(idx) else {
            continue;
        };

        let Some((compiled_idx, overlap)) = best_clause_match(oracle_token_set, compiled_tokens)
        else {
            continue;
        };
        if overlap < 0.55 {
            continue;
        }

        let Some(compiled_clause) = compiled_clauses.get(compiled_idx) else {
            continue;
        };
        if !has_type_among_count_semantics(compiled_clause) {
            mismatches += 1;
        }
    }

    mismatches
}

fn has_blocked_or_blocking_creature_qualifier(clause: &str) -> bool {
    let lower = clause.to_ascii_lowercase();
    lower.contains("blocked creature")
        || lower.contains("blocked creatures")
        || lower.contains("blocking creature")
        || lower.contains("blocking creatures")
}

fn count_blocked_or_blocking_qualifier_mismatches(
    oracle_clauses: &[String],
    oracle_tokens: &[Vec<String>],
    compiled_tokens: &[Vec<String>],
) -> usize {
    let mut mismatches = 0usize;

    for (idx, oracle_clause) in oracle_clauses.iter().enumerate() {
        if !has_blocked_or_blocking_creature_qualifier(oracle_clause) {
            continue;
        }
        let Some(oracle_token_set) = oracle_tokens.get(idx) else {
            continue;
        };

        let Some((compiled_idx, overlap)) = best_clause_match(oracle_token_set, compiled_tokens)
        else {
            continue;
        };
        if overlap < 0.55 {
            continue;
        }

        let Some(compiled_token_set) = compiled_tokens.get(compiled_idx) else {
            continue;
        };
        if !compiled_token_set.iter().any(|token| token == "block") {
            mismatches += 1;
        }
    }

    mismatches
}

fn best_clause_match(
    oracle_token_set: &[String],
    compiled_tokens: &[Vec<String>],
) -> Option<(usize, f32)> {
    let mut best_match: Option<(usize, f32)> = None;
    for (compiled_idx, compiled_token_set) in compiled_tokens.iter().enumerate() {
        let score = jaccard_similarity(oracle_token_set, compiled_token_set);
        if best_match.is_none_or(|(_, best)| score > best) {
            best_match = Some((compiled_idx, score));
        }
    }
    best_match
}

fn has_reflexive_when_you_do(clause: &str) -> bool {
    clause.to_ascii_lowercase().contains("when you do")
}

fn has_conditional_if_you_do(clause: &str) -> bool {
    clause.to_ascii_lowercase().contains("if you do")
}

fn count_reflexive_when_you_do_mismatches(
    oracle_clauses: &[String],
    oracle_tokens: &[Vec<String>],
    compiled_clauses: &[String],
    compiled_tokens: &[Vec<String>],
) -> usize {
    let mut mismatches = 0usize;

    for (idx, oracle_clause) in oracle_clauses.iter().enumerate() {
        let oracle_when = has_reflexive_when_you_do(oracle_clause);
        let oracle_if = has_conditional_if_you_do(oracle_clause);
        if !oracle_when && !oracle_if {
            continue;
        }

        let Some(oracle_token_set) = oracle_tokens.get(idx) else {
            continue;
        };
        let Some((compiled_idx, overlap)) = best_clause_match(oracle_token_set, compiled_tokens)
        else {
            continue;
        };
        if overlap < 0.55 {
            continue;
        }

        let Some(compiled_clause) = compiled_clauses.get(compiled_idx) else {
            continue;
        };
        let compiled_when = has_reflexive_when_you_do(compiled_clause);
        let compiled_if = has_conditional_if_you_do(compiled_clause);

        if (oracle_when && compiled_if) || (oracle_if && compiled_when) {
            mismatches += 1;
        }
    }

    mismatches
}

fn has_first_noncreature_each_turn(clause: &str) -> bool {
    clause
        .to_ascii_lowercase()
        .contains("first noncreature spell each turn")
}

fn has_noncreature_as_first_spell_this_turn(clause: &str) -> bool {
    let lower = clause.to_ascii_lowercase();
    lower.contains("noncreature spell as that player's first spell this turn")
        || lower.contains("noncreature spell as their first spell this turn")
        || lower.contains("noncreature spell as its first spell this turn")
}

fn count_first_noncreature_scope_mismatches(
    oracle_clauses: &[String],
    oracle_tokens: &[Vec<String>],
    compiled_clauses: &[String],
    compiled_tokens: &[Vec<String>],
) -> usize {
    let mut mismatches = 0usize;

    for (idx, oracle_clause) in oracle_clauses.iter().enumerate() {
        let oracle_each_turn = has_first_noncreature_each_turn(oracle_clause);
        let oracle_first_spell = has_noncreature_as_first_spell_this_turn(oracle_clause);
        if !oracle_each_turn && !oracle_first_spell {
            continue;
        }

        let Some(oracle_token_set) = oracle_tokens.get(idx) else {
            continue;
        };
        let Some((compiled_idx, overlap)) = best_clause_match(oracle_token_set, compiled_tokens)
        else {
            continue;
        };
        if overlap < 0.55 {
            continue;
        }

        let Some(compiled_clause) = compiled_clauses.get(compiled_idx) else {
            continue;
        };
        let compiled_each_turn = has_first_noncreature_each_turn(compiled_clause);
        let compiled_first_spell = has_noncreature_as_first_spell_this_turn(compiled_clause);

        if (oracle_each_turn && compiled_first_spell) || (oracle_first_spell && compiled_each_turn)
        {
            mismatches += 1;
        }
    }

    mismatches
}

fn has_target_instant_and_sorcery(clause: &str) -> bool {
    clause
        .to_ascii_lowercase()
        .contains("target instant and sorcery spell")
}

fn has_target_instant_or_sorcery(clause: &str) -> bool {
    clause
        .to_ascii_lowercase()
        .contains("target instant or sorcery spell")
}

fn count_instant_and_or_target_mismatches(
    oracle_clauses: &[String],
    oracle_tokens: &[Vec<String>],
    compiled_clauses: &[String],
    compiled_tokens: &[Vec<String>],
) -> usize {
    let mut mismatches = 0usize;

    for (idx, oracle_clause) in oracle_clauses.iter().enumerate() {
        let oracle_and = has_target_instant_and_sorcery(oracle_clause);
        let oracle_or = has_target_instant_or_sorcery(oracle_clause);
        if !oracle_and && !oracle_or {
            continue;
        }

        let Some(oracle_token_set) = oracle_tokens.get(idx) else {
            continue;
        };
        let Some((compiled_idx, overlap)) = best_clause_match(oracle_token_set, compiled_tokens)
        else {
            continue;
        };
        if overlap < 0.55 {
            continue;
        }

        let Some(compiled_clause) = compiled_clauses.get(compiled_idx) else {
            continue;
        };
        let compiled_and = has_target_instant_and_sorcery(compiled_clause);
        let compiled_or = has_target_instant_or_sorcery(compiled_clause);

        if (oracle_and && compiled_or) || (oracle_or && compiled_and) {
            mismatches += 1;
        }
    }

    mismatches
}

fn has_opponent_controls_qualifier(clause: &str) -> bool {
    clause.to_ascii_lowercase().contains("an opponent controls")
}

fn has_you_dont_control_qualifier(clause: &str) -> bool {
    let lower = clause.to_ascii_lowercase();
    lower.contains("you don't control") || lower.contains("you dont control")
}

fn count_opponent_control_scope_mismatches(
    oracle_clauses: &[String],
    oracle_tokens: &[Vec<String>],
    compiled_clauses: &[String],
    compiled_tokens: &[Vec<String>],
) -> usize {
    let mut mismatches = 0usize;

    for (idx, oracle_clause) in oracle_clauses.iter().enumerate() {
        let oracle_opponent_controls = has_opponent_controls_qualifier(oracle_clause);
        let oracle_you_dont_control = has_you_dont_control_qualifier(oracle_clause);
        if !oracle_opponent_controls && !oracle_you_dont_control {
            continue;
        }

        let Some(oracle_token_set) = oracle_tokens.get(idx) else {
            continue;
        };
        let Some((compiled_idx, overlap)) = best_clause_match(oracle_token_set, compiled_tokens)
        else {
            continue;
        };
        if overlap < 0.55 {
            continue;
        }

        let Some(compiled_clause) = compiled_clauses.get(compiled_idx) else {
            continue;
        };
        let compiled_opponent_controls = has_opponent_controls_qualifier(compiled_clause);
        let compiled_you_dont_control = has_you_dont_control_qualifier(compiled_clause);

        if (oracle_opponent_controls && compiled_you_dont_control)
            || (oracle_you_dont_control && compiled_opponent_controls)
        {
            mismatches += 1;
        }
    }

    mismatches
}

fn has_you_control_object_scope(clause: &str) -> bool {
    let lower = clause.to_ascii_lowercase();
    [
        "artifact",
        "artifacts",
        "creature",
        "creatures",
        "enchantment",
        "enchantments",
        "land",
        "lands",
        "permanent",
        "permanents",
        "planeswalker",
        "planeswalkers",
        "spell",
        "spells",
    ]
    .into_iter()
    .any(|noun| {
        lower.contains(&format!("{noun} you control")) || lower.contains(&format!("your {noun}"))
    })
}

/// A bare adjacent "creatures get +X/+X" pumps every creature; oracle's
/// back-referenced "the/that creature gets" pumps one. Token sets cannot see
/// the difference (plural stems to singular, articles are stopwords), so an
/// asymmetric bare mass pump is a semantic scope regression.
fn has_bare_mass_creature_pump(clause: &str) -> bool {
    let lower = clause.to_ascii_lowercase();
    ["creatures get +", "creatures get -"].iter().any(|needle| {
        let mut rest = lower.as_str();
        while let Some(idx) = rest.find(needle) {
            let before = &rest[..idx];
            // A qualified plural ("attacking creatures get", "those
            // creatures get") scopes the pump; only the bare form is mass.
            let qualified = before.trim_end().rsplit(' ').next().is_some_and(|word| {
                word.chars().all(|c| c.is_ascii_alphabetic()) && !word.is_empty()
            });
            if !qualified {
                return true;
            }
            rest = &rest[idx + needle.len()..];
        }
        false
    })
}

fn count_bare_mass_pump_scope_mismatches(
    oracle_clauses: &[String],
    oracle_tokens: &[Vec<String>],
    compiled_clauses: &[String],
    compiled_tokens: &[Vec<String>],
) -> usize {
    let mut mismatches = 0usize;

    for (idx, oracle_clause) in oracle_clauses.iter().enumerate() {
        let Some(oracle_token_set) = oracle_tokens.get(idx) else {
            continue;
        };
        let Some((compiled_idx, overlap)) = best_clause_match(oracle_token_set, compiled_tokens)
        else {
            continue;
        };
        if overlap < 0.55 {
            continue;
        }

        let Some(compiled_clause) = compiled_clauses.get(compiled_idx) else {
            continue;
        };
        if has_bare_mass_creature_pump(oracle_clause)
            != has_bare_mass_creature_pump(compiled_clause)
        {
            mismatches += 1;
        }
    }

    mismatches
}

fn count_you_control_scope_mismatches(
    oracle_clauses: &[String],
    oracle_tokens: &[Vec<String>],
    compiled_clauses: &[String],
    compiled_tokens: &[Vec<String>],
) -> usize {
    let mut mismatches = 0usize;

    for (idx, oracle_clause) in oracle_clauses.iter().enumerate() {
        let Some(oracle_token_set) = oracle_tokens.get(idx) else {
            continue;
        };
        let Some((compiled_idx, overlap)) = best_clause_match(oracle_token_set, compiled_tokens)
        else {
            continue;
        };
        if overlap < 0.55 {
            continue;
        }

        let Some(compiled_clause) = compiled_clauses.get(compiled_idx) else {
            continue;
        };
        if has_you_control_object_scope(oracle_clause)
            != has_you_control_object_scope(compiled_clause)
        {
            mismatches += 1;
        }
    }

    mismatches
}

fn directional_coverage(from: &[Vec<String>], to: &[Vec<String>]) -> f32 {
    if from.is_empty() {
        return if to.is_empty() { 1.0 } else { 0.0 };
    }

    let mut total = 0.0f32;
    for source in from {
        let mut best = 0.0f32;
        for target in to {
            let score = jaccard_similarity(source, target);
            if score > best {
                best = score;
            }
        }
        total += best;
    }
    total / from.len() as f32
}

fn is_compiled_heading_prefix(prefix: &str) -> bool {
    let prefix = prefix.trim().to_ascii_lowercase();
    prefix == "spell effects"
        || prefix.starts_with("activated ability ")
        || prefix.starts_with("triggered ability ")
        || prefix.starts_with("static ability ")
        || prefix.starts_with("keyword ability ")
        || prefix.starts_with("mana ability ")
        || prefix.starts_with("ability ")
        || prefix.starts_with("alternative cast ")
}

fn strip_compiled_prefix(line: &str) -> &str {
    let Some((prefix, rest)) = line.split_once(':') else {
        return line;
    };
    if is_compiled_heading_prefix(prefix) {
        rest.trim()
    } else {
        line
    }
}

fn split_lose_all_abilities_subject(line: &str) -> Option<&str> {
    let trimmed = line.trim().trim_end_matches('.');
    trimmed
        .strip_suffix(" loses all abilities")
        .or_else(|| trimmed.strip_suffix(" lose all abilities"))
        .map(str::trim)
}

fn extract_base_pt_tail_for_subject(line: &str, subject: &str) -> Option<String> {
    if let Some(pt) = line.strip_prefix("Affected permanents have base power and toughness ") {
        return Some(pt.trim().to_string());
    }
    for verb in ["has", "have"] {
        let prefix = format!("{subject} {verb} base power and toughness ");
        if let Some(pt) = line.strip_prefix(&prefix) {
            return Some(pt.trim().to_string());
        }
    }
    None
}

fn split_mana_add_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim().trim_end_matches('.');
    let (cost, effect) = trimmed.split_once(':')?;
    let add_tail = effect.trim().strip_prefix("Add ")?;
    let add_tail = add_tail.trim();
    if add_tail.is_empty() || add_tail.contains('.') || add_tail.contains(';') {
        return None;
    }
    Some((cost.trim().to_string(), add_tail.to_string()))
}

fn merge_simple_mana_add_compiled_lines(
    lines: &[String],
    oracle_clauses: &[String],
) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if let Some((base_cost, first_add)) = split_mana_add_line(&lines[idx]) {
            let mut adds = vec![first_add];
            let mut consumed = 1usize;
            while idx + consumed < lines.len() {
                let Some((next_cost, next_add)) = split_mana_add_line(&lines[idx + consumed])
                else {
                    break;
                };
                if !next_cost.eq_ignore_ascii_case(&base_cost) {
                    break;
                }
                if !adds
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(&next_add))
                {
                    adds.push(next_add);
                }
                consumed += 1;
            }
            if adds.len() >= 2 {
                let combined = format!("{base_cost}: Add {}", adds.join(" or "));
                let combined_tokens = compiled_comparison_tokens(&combined);
                let oracle_has_combined_choice = oracle_clauses.iter().any(|clause| {
                    split_mana_add_line(clause).is_some()
                        && comparison_tokens(clause) == combined_tokens
                });
                if oracle_has_combined_choice {
                    merged.push(combined);
                    idx += consumed;
                    continue;
                }
            }
        }
        merged.push(lines[idx].clone());
        idx += 1;
    }
    merged
}

fn is_simple_mana_add_clause(line: &str) -> bool {
    let lower = line.trim().to_ascii_lowercase();
    lower.starts_with("add ")
        || lower.starts_with("{t}: add ")
        || lower.starts_with("{t}, tap: add ")
        || (lower.starts_with("mana ability ")
            && lower
                .split_once(':')
                .is_some_and(|(_, rest)| rest.trim_start().starts_with("{t}: add ")))
}

fn merge_blockability_compiled_lines(lines: &[String]) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 1 < lines.len() {
            let left = lines[idx].trim().trim_end_matches('.');
            let right = lines[idx + 1].trim().trim_end_matches('.');
            let is_pair = (left.eq_ignore_ascii_case("This creature can't block")
                && right.eq_ignore_ascii_case("This creature can't be blocked"))
                || (left.eq_ignore_ascii_case("Can't block")
                    && right.eq_ignore_ascii_case("Can't be blocked"));
            if is_pair {
                merged.push("This creature can't block and can't be blocked".to_string());
                idx += 2;
                continue;
            }
        }
        merged.push(lines[idx].clone());
        idx += 1;
    }
    merged
}

fn merge_transform_compiled_lines(lines: &[String]) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;

    while idx < lines.len() {
        let left = lines[idx].trim().trim_end_matches('.');
        let Some(subject) = split_lose_all_abilities_subject(left) else {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        };

        let mut consumed = 1usize;
        let mut colors: Vec<String> = Vec::new();
        let mut card_types: Vec<String> = Vec::new();
        let mut subtypes: Vec<String> = Vec::new();
        let mut named: Option<String> = None;
        let mut base_pt: Option<String> = None;

        while idx + consumed < lines.len() {
            let line = lines[idx + consumed].trim().trim_end_matches('.');
            if let Some(pt) = extract_base_pt_tail_for_subject(line, subject) {
                base_pt = Some(pt);
                consumed += 1;
                continue;
            }

            let subject_prefix = format!("{subject} is ");
            let Some(rest) = line.strip_prefix(&subject_prefix) else {
                break;
            };
            let rest = rest.trim();
            if let Some(name) = rest.strip_prefix("named ") {
                named = Some(name.trim().to_string());
                consumed += 1;
                continue;
            }
            for part in rest
                .split(" and ")
                .map(str::trim)
                .filter(|part| !part.is_empty())
            {
                let lower = part.to_ascii_lowercase();
                if matches!(
                    lower.as_str(),
                    "white" | "blue" | "black" | "red" | "green" | "colorless"
                ) {
                    if !colors.contains(&lower) {
                        colors.push(lower);
                    }
                    continue;
                }
                if matches!(
                    lower.as_str(),
                    "creature" | "artifact" | "enchantment" | "land" | "planeswalker" | "battle"
                ) {
                    if !card_types.contains(&lower) {
                        card_types.push(lower);
                    }
                    continue;
                }
                if !subtypes.contains(&lower) {
                    subtypes.push(lower);
                }
            }
            consumed += 1;
        }

        if consumed == 1 {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        }

        let mut combined = format!("{subject} loses all abilities");
        let mut descriptor = String::new();
        if !colors.is_empty() {
            descriptor.push_str(&colors.join(" and "));
        }
        if !subtypes.is_empty() {
            if !descriptor.is_empty() {
                descriptor.push(' ');
            }
            descriptor.push_str(&subtypes.join(" and "));
        }
        if !card_types.is_empty() {
            if !descriptor.is_empty() {
                descriptor.push(' ');
            }
            descriptor.push_str(&card_types.join(" and "));
        }
        if !descriptor.is_empty() {
            combined.push_str(" and is ");
            combined.push_str(&descriptor);
        }
        if let Some(pt) = base_pt {
            combined.push_str(" with base power and toughness ");
            combined.push_str(&pt);
        }
        if let Some(name) = named {
            combined.push_str(" named ");
            combined.push_str(&name);
        }
        merged.push(combined);
        idx += consumed;
    }

    merged
}

/// Clause-level inputs shared by scoring and residual reporting.  Built by
/// [`prepare_clause_comparison`]; `trivially_equal` covers the early-return
/// cases where the comparison is a perfect match by construction.
struct ClauseComparisonPrep {
    oracle_clauses: Vec<String>,
    /// Clause strings paired 1:1 with `oracle_tokens` (post empty/bare-keyword
    /// filtering); used for residual reporting only.
    oracle_token_clauses: Vec<String>,
    oracle_tokens: Vec<Vec<String>>,
    compiled_clauses: Vec<String>,
    compiled_tokens: Vec<Vec<String>>,
    trivially_equal: bool,
}

impl ClauseComparisonPrep {
    fn trivially_equal() -> Self {
        Self {
            oracle_clauses: Vec::new(),
            oracle_token_clauses: Vec::new(),
            oracle_tokens: Vec::new(),
            compiled_clauses: Vec::new(),
            compiled_tokens: Vec::new(),
            trivially_equal: true,
        }
    }
}

fn normalize_paired_self_copula_surfaces(
    oracle_text: &str,
    compiled_lines: &[String],
) -> (String, Vec<String>) {
    fn has_contracted_surface(text: &str) -> bool {
        let lower = text.to_ascii_lowercase();
        lower.contains("as long as it's ") || lower.contains("as long as it’s ")
    }

    fn has_explicit_surface(text: &str) -> bool {
        text.to_ascii_lowercase().contains("as long as this is ")
    }

    fn canonicalize(text: &str) -> String {
        text.replace("As long as it's ", "As long as this is ")
            .replace("as long as it's ", "as long as this is ")
            .replace("As long as it’s ", "As long as this is ")
            .replace("as long as it’s ", "as long as this is ")
    }

    let compiled_has_contracted = compiled_lines
        .iter()
        .any(|line| has_contracted_surface(line));
    let compiled_has_explicit = compiled_lines.iter().any(|line| has_explicit_surface(line));
    let surfaces_differ = (has_contracted_surface(oracle_text) && compiled_has_explicit)
        || (has_explicit_surface(oracle_text) && compiled_has_contracted);

    if !surfaces_differ {
        return (oracle_text.to_string(), compiled_lines.to_vec());
    }

    (
        canonicalize(oracle_text),
        compiled_lines
            .iter()
            .map(|line| canonicalize(line))
            .collect(),
    )
}

fn prepare_clause_comparison(oracle_text: &str, compiled_lines: &[String]) -> ClauseComparisonPrep {
    // Only canonicalize the contracted and explicit self-copula forms when
    // they occur on opposite sides of this comparison. Applying that rewrite
    // independently changed scores for cards whose compiled output had not
    // changed at all.
    let (oracle_text, compiled_lines) =
        normalize_paired_self_copula_surfaces(oracle_text, compiled_lines);
    let oracle_clauses = semantic_clauses(&oracle_text)
        .into_iter()
        .filter(|clause| !is_ignorable_semantic_clause(clause))
        .collect::<Vec<_>>();
    let reminder_clauses = reminder_clauses(&oracle_text);
    let stripped_lines = compiled_lines
        .iter()
        .map(|line| strip_compiled_prefix(line).to_string())
        .collect::<Vec<_>>();
    let merged_mana_lines = merge_simple_mana_add_compiled_lines(&stripped_lines, &oracle_clauses);
    let merged_blockability_lines = merge_blockability_compiled_lines(&merged_mana_lines);
    let compiled_normalized_lines = merge_transform_compiled_lines(&merged_blockability_lines);
    let flattened_compiled_lines =
        split_compiled_lines_for_semantic_compare(&compiled_normalized_lines);
    let raw_compiled_clauses = flattened_compiled_lines
        .iter()
        .flat_map(|line| semantic_clauses(line))
        .flat_map(|clause| split_compiled_activation_restriction_clauses(&clause))
        .collect::<Vec<_>>();

    let oracle_token_pairs: Vec<(String, Vec<String>)> = oracle_clauses
        .iter()
        .map(|clause| (clause.clone(), comparison_tokens(clause)))
        .filter(|(_, tokens)| !tokens.is_empty())
        .collect();
    let oracle_tokens: Vec<Vec<String>> = oracle_token_pairs
        .iter()
        .map(|(_, tokens)| tokens.clone())
        .collect();
    let reminder_tokens: Vec<Vec<String>> = reminder_clauses
        .iter()
        .map(|clause| comparison_tokens(clause))
        .filter(|tokens| !tokens.is_empty())
        .collect();
    let has_reminder = !reminder_tokens.is_empty();
    let bare_keyword_oracle_tokens = if has_reminder {
        oracle_tokens
            .iter()
            .filter(|tokens| is_bare_keyword_clause(tokens))
            .cloned()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let mut compiled_pairs = raw_compiled_clauses
        .iter()
        .filter(|clause| !is_ignorable_semantic_clause(clause))
        .map(|clause| (clause.clone(), compiled_comparison_tokens(clause)))
        .filter(|(_, tokens)| !tokens.is_empty())
        .collect::<Vec<_>>();
    let reminder_activation_like = reminder_clauses
        .iter()
        .any(|reminder| is_activation_restriction_reminder_clause(reminder));

    compiled_pairs.retain(|(clause, _)| !is_internal_compiled_scaffolding_clause(clause));
    compiled_pairs = remove_redundant_compiled_clauses(compiled_pairs);

    compiled_pairs.retain(|(clause, tokens)| {
        let matches_oracle = oracle_tokens
            .iter()
            .any(|oracle| tokens_match_subsetish(tokens, oracle));
        let clause_lower = clause.to_ascii_lowercase();
        let has_activate_token = tokens.iter().any(|token| token == "activate");
        let reminder_match_threshold = if clause_lower.starts_with("activate only ") {
            0.5
        } else if reminder_activation_like && has_activate_token {
            0.20
        } else {
            0.8
        };
        let matches_reminder = (reminder_activation_like
            && clause_lower.starts_with("activate only "))
            || reminder_tokens.iter().any(|reminder| {
                tokens_match_subsetish_with_threshold(tokens, reminder, reminder_match_threshold)
            });
        !(matches_reminder && !matches_oracle)
    });
    compiled_pairs.retain(|(_, tokens)| {
        !(has_reminder
            && is_bare_keyword_clause(tokens)
            && bare_keyword_oracle_tokens
                .iter()
                .any(|oracle| oracle == tokens))
    });
    let oracle_mentions_mana_add = oracle_clauses
        .iter()
        .any(|clause| clause.to_ascii_lowercase().contains("add "));
    if !oracle_mentions_mana_add {
        compiled_pairs.retain(|(clause, _)| !is_simple_mana_add_clause(clause));
    }

    let compiled_clauses = compiled_pairs
        .iter()
        .map(|(clause, _)| clause.clone())
        .collect::<Vec<_>>();
    let compiled_tokens: Vec<Vec<String>> = compiled_pairs
        .into_iter()
        .map(|(_, tokens)| tokens)
        .collect();

    // Parenthetical-only oracle text (typically reminder text) carries no
    // semantic clauses after normalization, so don't flag as mismatch.
    if oracle_tokens.is_empty() {
        return ClauseComparisonPrep::trivially_equal();
    }

    // Keyword expansion handling: when an oracle clause is a bare keyword name
    // (e.g. "Enlist") whose reminder text was compiled into the expansion, the
    // compiled clauses that matched the reminder were already filtered out
    // above.  Exclude these bare-keyword oracle clauses from the coverage
    // calculation since their semantics are fully captured by the expansion.
    let oracle_token_pairs: Vec<(String, Vec<String>)> = if has_reminder {
        oracle_token_pairs
            .into_iter()
            .filter(|(_, tokens)| !is_bare_keyword_clause(tokens))
            .collect()
    } else {
        oracle_token_pairs
    };
    if oracle_token_pairs.is_empty() {
        return ClauseComparisonPrep::trivially_equal();
    }

    if oracle_clauses == compiled_clauses {
        return ClauseComparisonPrep::trivially_equal();
    }

    let (oracle_token_clauses, oracle_tokens): (Vec<String>, Vec<Vec<String>>) =
        oracle_token_pairs.into_iter().unzip();
    ClauseComparisonPrep {
        oracle_clauses,
        oracle_token_clauses,
        oracle_tokens,
        compiled_clauses,
        compiled_tokens,
        trivially_equal: false,
    }
}

pub fn compare_semantics_scored(
    oracle_text: &str,
    compiled_lines: &[String],
    embedding: Option<EmbeddingConfig>,
) -> (f32, f32, f32, isize, bool) {
    let prep = prepare_clause_comparison(oracle_text, compiled_lines);
    if prep.trivially_equal {
        return (1.0, 1.0, 1.0, 0, false);
    }
    let ClauseComparisonPrep {
        oracle_clauses,
        oracle_token_clauses: _,
        oracle_tokens,
        compiled_clauses,
        compiled_tokens,
        trivially_equal: _,
    } = prep;

    let line_delta = compiled_clauses.len() as isize - oracle_clauses.len() as isize;
    let oracle_coverage = directional_coverage(&oracle_tokens, &compiled_tokens);
    let compiled_coverage = directional_coverage(&compiled_tokens, &oracle_tokens);

    let min_coverage = oracle_coverage.min(compiled_coverage);

    let semantic_gap = min_coverage < 0.25;
    let line_gap = line_delta.abs() >= 3 && min_coverage < 0.50;
    let empty_gap = !oracle_tokens.is_empty() && compiled_tokens.is_empty();

    let mut similarity_score = min_coverage;
    let mut mismatch = semantic_gap || line_gap || empty_gap;
    let unless_pay_role_mismatch_count = count_unless_pay_role_mismatches(
        &oracle_clauses,
        &oracle_tokens,
        &compiled_clauses,
        &compiled_tokens,
    );
    let type_among_count_mismatch_count = count_type_among_count_mismatches(
        &oracle_clauses,
        &oracle_tokens,
        &compiled_clauses,
        &compiled_tokens,
    );
    let blocked_or_blocking_mismatch_count = count_blocked_or_blocking_qualifier_mismatches(
        &oracle_clauses,
        &oracle_tokens,
        &compiled_tokens,
    );
    let reflexive_when_you_do_mismatch_count = count_reflexive_when_you_do_mismatches(
        &oracle_clauses,
        &oracle_tokens,
        &compiled_clauses,
        &compiled_tokens,
    );
    let first_noncreature_scope_mismatch_count = count_first_noncreature_scope_mismatches(
        &oracle_clauses,
        &oracle_tokens,
        &compiled_clauses,
        &compiled_tokens,
    );
    let instant_and_or_target_mismatch_count = count_instant_and_or_target_mismatches(
        &oracle_clauses,
        &oracle_tokens,
        &compiled_clauses,
        &compiled_tokens,
    );
    let opponent_control_scope_mismatch_count = count_opponent_control_scope_mismatches(
        &oracle_clauses,
        &oracle_tokens,
        &compiled_clauses,
        &compiled_tokens,
    );
    let bare_mass_pump_mismatch_count = count_bare_mass_pump_scope_mismatches(
        &oracle_clauses,
        &oracle_tokens,
        &compiled_clauses,
        &compiled_tokens,
    );
    let you_control_scope_mismatch_count = count_you_control_scope_mismatches(
        &oracle_clauses,
        &oracle_tokens,
        &compiled_clauses,
        &compiled_tokens,
    );

    if let Some(cfg) = embedding {
        let oracle_emb = oracle_clauses
            .iter()
            .map(|clause| embed_clause(clause, cfg.dims))
            .collect::<Vec<_>>();
        let compiled_emb = compiled_clauses
            .iter()
            .map(|clause| embed_clause(clause, cfg.dims))
            .collect::<Vec<_>>();
        let emb_oracle = directional_embedding_coverage(&oracle_emb, &compiled_emb);
        let emb_compiled = directional_embedding_coverage(&compiled_emb, &oracle_emb);
        let emb_min = emb_oracle.min(emb_compiled);
        // Fuse embedding and lexical confidence so token overlap can rescue
        // occasional embedding outliers.
        let fused_score = 1.0 - (1.0 - emb_min.max(0.0)) * (1.0 - min_coverage.max(0.0));
        similarity_score = fused_score;
        if fused_score < cfg.mismatch_threshold {
            mismatch = true;
        }
    }

    if unless_pay_role_mismatch_count > 0 {
        let penalty = 0.20 * unless_pay_role_mismatch_count as f32;
        similarity_score = (similarity_score - penalty).max(0.0);
        mismatch = true;
    }
    if type_among_count_mismatch_count > 0 {
        let penalty = 0.20 * type_among_count_mismatch_count as f32;
        similarity_score = (similarity_score - penalty).max(0.0);
        mismatch = true;
    }
    if blocked_or_blocking_mismatch_count > 0 {
        let penalty = 0.20 * blocked_or_blocking_mismatch_count as f32;
        similarity_score = (similarity_score - penalty).max(0.0);
        mismatch = true;
    }
    if reflexive_when_you_do_mismatch_count > 0 {
        let penalty = 0.20 * reflexive_when_you_do_mismatch_count as f32;
        similarity_score = (similarity_score - penalty).max(0.0);
        mismatch = true;
    }
    if first_noncreature_scope_mismatch_count > 0 {
        let penalty = 0.20 * first_noncreature_scope_mismatch_count as f32;
        similarity_score = (similarity_score - penalty).max(0.0);
        mismatch = true;
    }
    if instant_and_or_target_mismatch_count > 0 {
        let penalty = 0.20 * instant_and_or_target_mismatch_count as f32;
        similarity_score = (similarity_score - penalty).max(0.0);
        mismatch = true;
    }
    if opponent_control_scope_mismatch_count > 0 {
        let penalty = 0.20 * opponent_control_scope_mismatch_count as f32;
        similarity_score = (similarity_score - penalty).max(0.0);
        mismatch = true;
    }
    if you_control_scope_mismatch_count > 0 {
        let penalty = 0.20 * you_control_scope_mismatch_count as f32;
        similarity_score = (similarity_score - penalty).max(0.0);
        mismatch = true;
    }
    if bare_mass_pump_mismatch_count > 0 {
        // Keep the penalty light: the mismatch flag is the gate, and the
        // clause pair is otherwise near-identical by construction.
        let penalty = 0.02 * bare_mass_pump_mismatch_count as f32;
        similarity_score = (similarity_score - penalty).max(0.0);
        mismatch = true;
    }

    (
        oracle_coverage,
        compiled_coverage,
        similarity_score,
        line_delta,
        mismatch,
    )
}

pub fn compare_semantics(
    oracle_text: &str,
    compiled_lines: &[String],
    embedding: Option<EmbeddingConfig>,
) -> (f32, f32, isize, bool) {
    let (oracle_coverage, compiled_coverage, _similarity_score, line_delta, mismatch) =
        compare_semantics_scored(oracle_text, compiled_lines, embedding);
    (oracle_coverage, compiled_coverage, line_delta, mismatch)
}

pub fn compare_card_semantics_scored(
    card_name: &str,
    oracle_text: &str,
    compiled_lines: &[String],
    embedding: Option<EmbeddingConfig>,
) -> (f32, f32, f32, isize, bool) {
    let normalized_oracle = normalize_card_self_references_for_compare(oracle_text, card_name);
    let normalized_compiled = compiled_lines
        .iter()
        .map(|line| normalize_card_self_references_for_compare(line, card_name))
        .collect::<Vec<_>>();
    compare_semantics_scored(&normalized_oracle, &normalized_compiled, embedding)
}

/// One side of a clause-level comparison residual: a clause, its best
/// counterpart on the other side (by token Jaccard), and the token sets that
/// kept the pair from matching perfectly.
#[derive(Debug, Clone)]
pub struct ClauseResidual {
    pub clause: String,
    pub best_match: Option<String>,
    pub best_jaccard: f32,
    /// Tokens present in this clause but absent from the best match.
    pub missing_tokens: Vec<String>,
    /// Tokens present in the best match but absent from this clause.
    pub extra_tokens: Vec<String>,
}

fn clause_residuals(
    clauses: &[String],
    tokens: &[Vec<String>],
    other_clauses: &[String],
    other_tokens: &[Vec<String>],
) -> Vec<ClauseResidual> {
    clauses
        .iter()
        .zip(tokens.iter())
        .map(|(clause, token_set)| {
            let mut best: Option<(usize, f32)> = None;
            for (idx, other) in other_tokens.iter().enumerate() {
                let score = jaccard_similarity(token_set, other);
                if best.is_none_or(|(_, best_score)| score > best_score) {
                    best = Some((idx, score));
                }
            }
            let (best_match, best_jaccard, missing_tokens, extra_tokens) = match best {
                Some((idx, score)) => {
                    let other = &other_tokens[idx];
                    let missing = token_set
                        .iter()
                        .filter(|token| !other.contains(token))
                        .cloned()
                        .collect();
                    let extra = other
                        .iter()
                        .filter(|token| !token_set.contains(token))
                        .cloned()
                        .collect();
                    (Some(other_clauses[idx].clone()), score, missing, extra)
                }
                None => (None, 0.0, token_set.clone(), Vec::new()),
            };
            ClauseResidual {
                clause: clause.clone(),
                best_match,
                best_jaccard,
                missing_tokens,
                extra_tokens,
            }
        })
        .collect()
}

/// Clause-level residual report for a card: which oracle clauses are not
/// fully covered by the compiled text (and vice versa), using exactly the
/// clause/token pipeline that drives `compare_card_semantics_scored`.
pub fn compare_card_semantics_clause_residuals(
    card_name: &str,
    oracle_text: &str,
    compiled_lines: &[String],
) -> (Vec<ClauseResidual>, Vec<ClauseResidual>) {
    let normalized_oracle = normalize_card_self_references_for_compare(oracle_text, card_name);
    let normalized_compiled = compiled_lines
        .iter()
        .map(|line| normalize_card_self_references_for_compare(line, card_name))
        .collect::<Vec<_>>();
    let prep = prepare_clause_comparison(&normalized_oracle, &normalized_compiled);
    if prep.trivially_equal {
        return (Vec::new(), Vec::new());
    }
    let oracle_residuals = clause_residuals(
        &prep.oracle_token_clauses,
        &prep.oracle_tokens,
        &prep.compiled_clauses,
        &prep.compiled_tokens,
    );
    let compiled_residuals = clause_residuals(
        &prep.compiled_clauses,
        &prep.compiled_tokens,
        &prep.oracle_token_clauses,
        &prep.oracle_tokens,
    );
    (oracle_residuals, compiled_residuals)
}

pub fn compare_card_semantics(
    card_name: &str,
    oracle_text: &str,
    compiled_lines: &[String],
    embedding: Option<EmbeddingConfig>,
) -> (f32, f32, isize, bool) {
    let (oracle_coverage, compiled_coverage, _similarity_score, line_delta, mismatch) =
        compare_card_semantics_scored(card_name, oracle_text, compiled_lines, embedding);
    (oracle_coverage, compiled_coverage, line_delta, mismatch)
}
