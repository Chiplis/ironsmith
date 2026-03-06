use super::*;

pub(crate) const RULE_SHAPE_HAS_COLON: u32 = 1 << 0;
pub(crate) const RULE_SHAPE_HAS_COMMA: u32 = 1 << 1;
pub(crate) const RULE_SHAPE_HAS_SEMICOLON: u32 = 1 << 2;
pub(crate) const RULE_SHAPE_STARTS_IF: u32 = 1 << 3;
pub(crate) const RULE_SHAPE_STARTS_WHEN: u32 = 1 << 4;
pub(crate) const RULE_SHAPE_STARTS_WHENEVER: u32 = 1 << 5;
pub(crate) const RULE_SHAPE_STARTS_AT: u32 = 1 << 6;
pub(crate) const RULE_SHAPE_STARTS_MAY: u32 = 1 << 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuleKey<'a> {
    pub(crate) head: &'a str,
    pub(crate) shape: u32,
}

impl<'a> RuleKey<'a> {
    pub(crate) fn new(head: &'a str, shape: u32) -> Self {
        Self { head, shape }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ClauseView<'a> {
    pub(crate) raw: Option<&'a str>,
    pub(crate) tokens: &'a [Token],
    pub(crate) words: Vec<&'a str>,
    pub(crate) key: RuleKey<'a>,
    pub(crate) normalized: Option<&'a str>,
    pub(crate) normalized_without_braces: Option<&'a str>,
}

impl<'a> ClauseView<'a> {
    pub(crate) fn from_tokens(tokens: &'a [Token]) -> Self {
        let words = words(tokens);
        let head = words.first().copied().unwrap_or("");
        let shape = clause_shape(tokens, &words);
        Self {
            raw: None,
            tokens,
            words,
            key: RuleKey::new(head, shape),
            normalized: None,
            normalized_without_braces: None,
        }
    }

    pub(crate) fn from_line(
        raw: &'a str,
        normalized: &'a str,
        normalized_without_braces: &'a str,
        tokens: &'a [Token],
    ) -> Self {
        let words = words(tokens);
        let head = words.first().copied().unwrap_or("");
        let shape = clause_shape(tokens, &words);
        Self {
            raw: Some(raw),
            tokens,
            words,
            key: RuleKey::new(head, shape),
            normalized: Some(normalized),
            normalized_without_braces: Some(normalized_without_braces),
        }
    }

    pub(crate) fn display_text(&self) -> String {
        if let Some(raw) = self.raw {
            raw.trim().to_string()
        } else {
            self.words.join(" ")
        }
    }
}

fn clause_shape(tokens: &[Token], words: &[&str]) -> u32 {
    let mut shape = 0u32;
    if tokens.iter().any(|token| matches!(token, Token::Colon(_))) {
        shape |= RULE_SHAPE_HAS_COLON;
    }
    if tokens.iter().any(|token| matches!(token, Token::Comma(_))) {
        shape |= RULE_SHAPE_HAS_COMMA;
    }
    if tokens
        .iter()
        .any(|token| matches!(token, Token::Semicolon(_)))
    {
        shape |= RULE_SHAPE_HAS_SEMICOLON;
    }
    match words.first().copied().unwrap_or("") {
        "if" => shape |= RULE_SHAPE_STARTS_IF,
        "when" => shape |= RULE_SHAPE_STARTS_WHEN,
        "whenever" => shape |= RULE_SHAPE_STARTS_WHENEVER,
        "at" => shape |= RULE_SHAPE_STARTS_AT,
        "may" => shape |= RULE_SHAPE_STARTS_MAY,
        _ => {}
    }
    shape
}

pub(crate) type ClauseRuleFn<T> = for<'a> fn(&ClauseView<'a>) -> Result<Option<T>, CardTextError>;

#[derive(Clone, Copy)]
pub(crate) struct RuleDef<T> {
    pub(crate) id: &'static str,
    pub(crate) priority: u16,
    pub(crate) heads: &'static [&'static str],
    pub(crate) shape_mask: u32,
    pub(crate) run: ClauseRuleFn<T>,
}

#[derive(Clone, Copy)]
pub(crate) struct RuleIndex<T: 'static> {
    rules: &'static [RuleDef<T>],
}

impl<T: 'static> RuleIndex<T> {
    pub(crate) const fn new(rules: &'static [RuleDef<T>]) -> Self {
        Self { rules }
    }

    pub(crate) fn run_first<'a>(
        &self,
        view: &ClauseView<'a>,
    ) -> Result<Option<(&'static str, T)>, CardTextError> {
        let mut candidate_indices = self
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| rule_matches_view(rule, view))
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>();

        candidate_indices.sort_by_key(|idx| self.rules[*idx].priority);
        for idx in candidate_indices {
            let rule = &self.rules[idx];
            if let Some(result) = (rule.run)(view)? {
                return Ok(Some((rule.id, result)));
            }
        }

        Ok(None)
    }
}

fn rule_matches_view<T>(rule: &RuleDef<T>, view: &ClauseView<'_>) -> bool {
    let head_matches =
        rule.heads.is_empty() || rule.heads.iter().any(|candidate| *candidate == view.key.head);
    if !head_matches {
        return false;
    }
    if rule.shape_mask == 0 {
        return true;
    }
    (view.key.shape & rule.shape_mask) == rule.shape_mask
}

pub(crate) type UnsupportedPredicate = for<'a> fn(&ClauseView<'a>) -> bool;

#[derive(Clone, Copy)]
pub(crate) struct UnsupportedRuleDef {
    pub(crate) id: &'static str,
    pub(crate) priority: u16,
    pub(crate) heads: &'static [&'static str],
    pub(crate) shape_mask: u32,
    pub(crate) message: &'static str,
    pub(crate) predicate: UnsupportedPredicate,
}

#[derive(Clone, Copy)]
pub(crate) struct UnsupportedDiagnoser {
    rules: &'static [UnsupportedRuleDef],
}

impl UnsupportedDiagnoser {
    pub(crate) const fn new(rules: &'static [UnsupportedRuleDef]) -> Self {
        Self { rules }
    }

    pub(crate) fn diagnose(
        &self,
        view: &ClauseView<'_>,
        subject_label: &'static str,
    ) -> Option<CardTextError> {
        let mut candidate_indices = self
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| unsupported_rule_matches_view(rule, view))
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>();

        candidate_indices.sort_by_key(|idx| self.rules[*idx].priority);
        for idx in candidate_indices {
            let rule = &self.rules[idx];
            if (rule.predicate)(view) {
                let text = view.display_text();
                return Some(CardTextError::ParseError(format!(
                    "{} ({}: '{}') [rule={}]",
                    rule.message, subject_label, text, rule.id
                )));
            }
        }
        None
    }
}

fn unsupported_rule_matches_view(rule: &UnsupportedRuleDef, view: &ClauseView<'_>) -> bool {
    let head_matches =
        rule.heads.is_empty() || rule.heads.iter().any(|candidate| *candidate == view.key.head);
    if !head_matches {
        return false;
    }
    if rule.shape_mask == 0 {
        return true;
    }
    (view.key.shape & rule.shape_mask) == rule.shape_mask
}

#[derive(Clone, Copy)]
pub(crate) struct KeyedRuleDef<K, V> {
    pub(crate) id: &'static str,
    pub(crate) key: K,
    pub(crate) priority: u16,
    pub(crate) value: V,
}

#[derive(Clone, Copy)]
pub(crate) struct KeyedRuleIndex<K: Copy + Eq + 'static, V: Copy + 'static> {
    rules: &'static [KeyedRuleDef<K, V>],
}

impl<K: Copy + Eq + 'static, V: Copy + 'static> KeyedRuleIndex<K, V> {
    pub(crate) const fn new(rules: &'static [KeyedRuleDef<K, V>]) -> Self {
        Self { rules }
    }

    pub(crate) fn first_for_key(&self, key: K) -> Option<(&'static str, V)> {
        let mut best: Option<(&'static str, u16, V)> = None;
        for rule in self.rules {
            if rule.key != key {
                continue;
            }
            match best {
                Some((_, priority, _)) if priority <= rule.priority => {}
                _ => {
                    best = Some((rule.id, rule.priority, rule.value));
                }
            }
        }
        best.map(|(id, _, value)| (id, value))
    }
}
