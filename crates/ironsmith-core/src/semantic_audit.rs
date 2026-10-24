//! Semantic audit rendering.
//!
//! Ordinary rendering may print authored prose that the compiler retained
//! next to an executable model ("as long as there are four or more card types
//! among ..."). That prose survives even when the executable fields are wrong,
//! so it cannot be used as evidence that the model means what the card says.
//!
//! Inside [`with_semantic_audit`], every renderer site that would substitute
//! retained prose for a description of executable fields must instead either
//! describe the fields structurally or emit an explicit marker from
//! [`retained`]. The marker carries a digest of the executable fields, so a
//! change to the model is always visible in the audit output, and an audit
//! line can never be a plausible-looking quotation of the printed card.

use std::cell::Cell;
use std::fmt::Debug;

type ConditionDescriber = fn(&crate::value_model::Condition) -> String;
/// Structural reader for runtime values this crate cannot name (an engine
/// `Ability`, for example); returns `None` for values it does not handle.
type ValueDescriber = fn(&dyn std::any::Any) -> Option<String>;

std::thread_local! {
    static AUDIT_DEPTH: Cell<u32> = const { Cell::new(0) };
    static CONDITION_DESCRIBER: Cell<Option<ConditionDescriber>> = const { Cell::new(None) };
    static VALUE_DESCRIBER: Cell<Option<ValueDescriber>> = const { Cell::new(None) };
}

/// Whether the current thread is rendering for a semantic audit.
pub fn semantic_audit_active() -> bool {
    AUDIT_DEPTH.with(|depth| depth.get() > 0)
}

/// Run `render` with semantic audit rendering enabled on this thread.
pub fn with_semantic_audit<R>(render: impl FnOnce() -> R) -> R {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            AUDIT_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
        }
    }
    AUDIT_DEPTH.with(|depth| depth.set(depth.get() + 1));
    let _reset = Reset;
    render()
}

/// Run `render` as a semantic audit with `describer` as the structural reader
/// for conditions. Families that own a condition but render it from retained
/// prose (entry gates, spell-cost conditions) use it through
/// [`audited_condition`]; the condition renderer lives above this crate.
pub fn with_semantic_audit_describing<R>(
    describer: ConditionDescriber,
    values: ValueDescriber,
    render: impl FnOnce() -> R,
) -> R {
    struct Restore(Option<ConditionDescriber>, Option<ValueDescriber>);
    impl Drop for Restore {
        fn drop(&mut self) {
            CONDITION_DESCRIBER.with(|slot| slot.set(self.0));
            VALUE_DESCRIBER.with(|slot| slot.set(self.1));
        }
    }
    let _restore = Restore(
        CONDITION_DESCRIBER.with(|slot| slot.replace(Some(describer))),
        VALUE_DESCRIBER.with(|slot| slot.replace(Some(values))),
    );
    with_semantic_audit(render)
}

/// The installed structural reading of `value`, if a reader handles it.
pub fn described_value(value: &dyn std::any::Any) -> Option<String> {
    VALUE_DESCRIBER.with(Cell::get).and_then(|describe| describe(value))
}

/// The installed structural reading of `value`, or an explicit marker for
/// `model` when no reader handles it.
pub fn audited_value(family: &str, value: &dyn std::any::Any, model: &dyn Debug) -> String {
    VALUE_DESCRIBER
        .with(Cell::get)
        .and_then(|describe| describe(value))
        .unwrap_or_else(|| unverified_marker(family, model, &[]))
}

/// The structural reading of `condition` during an audit, or an explicit
/// marker when no condition reader is installed.
pub fn audited_condition(family: &str, condition: &crate::value_model::Condition) -> String {
    match CONDITION_DESCRIBER.with(Cell::get) {
        Some(describe) => describe(condition),
        None => unverified_marker(family, condition, &[]),
    }
}

/// A structural reading of a typed enum value for audit output:
/// `DuringCombatBeforeBlockersAreDeclared` reads "during combat before
/// blockers are declared"; field payloads stay in their Debug form.
pub fn humanize_variant(value: &dyn Debug) -> String {
    let debug = format!("{value:?}");
    let (head, rest) = debug
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .map_or((debug.as_str(), ""), |split| debug.split_at(split));
    let mut words = String::new();
    for (i, c) in head.char_indices() {
        if c.is_ascii_uppercase() && i > 0 {
            words.push(' ');
        }
        words.push(c.to_ascii_lowercase());
    }
    format!("{words}{rest}")
}

/// Retained authored prose a renderer may print in place of describing the
/// executable fields. During an audit there is none: the caller must take its
/// structural path.
pub fn authored(display: Option<&str>) -> Option<&str> {
    if semantic_audit_active() { None } else { display }
}

/// Prefix of every audit marker; callers and tests search for it.
pub const UNVERIFIED_MARKER_PREFIX: &str = "⟦unverified ";

/// Retained `display` prose for a `family` whose executable meaning lives in
/// `model`. Outside an audit this is the display itself. During an audit it
/// is an explicit marker naming the family and a digest of `model` with the
/// display text removed, so the line reports unverifiable prose instead of
/// presenting it as a reading of the model.
pub fn retained(family: &str, display: &str, model: &dyn Debug) -> String {
    if !semantic_audit_active() {
        return display.to_string();
    }
    unverified_marker(family, model, &[display])
}

/// The audit marker for `model`: its family plus a structural reading of the
/// executable fields (see [`structural_reading`]), ignoring the given
/// presentation strings.
pub fn unverified_marker(family: &str, model: &dyn Debug, presentation: &[&str]) -> String {
    format!(
        "{UNVERIFIED_MARKER_PREFIX}{family}: {}⟧",
        structural_reading(model, presentation)
    )
}

/// Longest structural reading printed in full; longer ones end in a digest of
/// the whole model so a mutation past the cut still changes the line.
const MAX_READING_CHARS: usize = 600;

/// The executable fields of `model` in compact Debug form: presentation
/// literals blanked, and `false` / `None` / empty / default fields dropped.
/// It is derived from the model alone, so it can never quote the card.
pub fn structural_reading(model: &dyn Debug, presentation: &[&str]) -> String {
    let mut text = strip_presentation_literals(&format!("{model:?}"));
    for surface in presentation {
        if !surface.is_empty() {
            text = text.replace(&format!("{surface:?}"), "\"\"");
        }
    }
    // Compiled-text normalization strips parenthesized groups as reminder
    // text, which would erase payloads such as `SetTo(3)`.
    let reading = drop_default_fields(&text).replace('(', "‹").replace(')', "›");
    if reading.chars().count() <= MAX_READING_CHARS {
        return reading;
    }
    let cut: String = reading.chars().take(MAX_READING_CHARS).collect();
    format!("{cut}… #{:08x}", semantic_digest(model, presentation))
}

/// Remove `name: <default>` fields (`false`, `None`, `[]`, `{}`, `""`,
/// `Default`) from a single-line Debug rendering, repeating until stable so
/// structs left empty collapse too.
fn drop_default_fields(debug: &str) -> String {
    const DEFAULTS: [&str; 6] = ["false", "None", "[]", "{}", "\"\"", "Default"];
    let mut text = debug.to_string();
    loop {
        let mut out = String::with_capacity(text.len());
        let mut rest = text.as_str();
        while let Some(colon) = rest.find(": ") {
            let name_start = rest[..colon]
                .char_indices()
                .rev()
                .find(|(_, c)| !(c.is_ascii_alphanumeric() || *c == '_'))
                .map_or(0, |(p, c)| p + c.len_utf8());
            let value = &rest[colon + 2..];
            let default = DEFAULTS.iter().find(|d| {
                value.starts_with(**d)
                    && value[d.len()..]
                        .chars()
                        .next()
                        .is_none_or(|c| c == ',' || c == ' ' || c == '}' || c == ')')
            });
            match default {
                Some(d) if name_start < colon => {
                    out.push_str(&rest[..name_start]);
                    rest = &value[d.len()..];
                    rest = rest.strip_prefix(", ").unwrap_or(rest);
                }
                _ => {
                    out.push_str(&rest[..colon + 2]);
                    rest = value;
                }
            }
        }
        out.push_str(rest);
        let out = out
            .replace(",  }", " }")
            .replace(", }", " }")
            .replace(",  )", ")")
            .replace(", )", ")")
            .replace("{ , ", "{ ")
            .replace(" {  }", "")
            .replace(" { }", "");
        if out == text {
            return out;
        }
        text = out;
    }
}

/// A stable digest of `model`'s Debug form with presentation text removed:
/// each given presentation string, and every string literal held by a
/// presentation field (`display`, `label`, `*_surface`) at any depth.
/// Digests only need to differ when executable fields differ; they are not
/// persisted.
pub fn semantic_digest(model: &dyn Debug, presentation: &[&str]) -> u32 {
    let mut text = strip_presentation_literals(&format!("{model:?}"));
    for surface in presentation {
        if surface.is_empty() {
            continue;
        }
        text = text.replace(&format!("{surface:?}"), "\"\"");
    }
    // FNV-1a: deterministic across processes and platforms.
    let mut hash: u32 = 0x811c_9dc5;
    for byte in text.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn is_presentation_field(name: &str) -> bool {
    name == "display" || name == "label" || name.ends_with("_surface")
}

/// Blank the string literal following each `<presentation field>: ` (or
/// `<field>: Some(`) in a Debug rendering.
fn strip_presentation_literals(debug: &str) -> String {
    let bytes = debug.as_bytes();
    let mut out = String::with_capacity(debug.len());
    let mut i = 0;
    while i < bytes.len() {
        let rest = &debug[i..];
        if let Some(colon) = rest.find(": ") {
            let field_start = rest[..colon]
                .char_indices()
                .rev()
                .find(|(_, c)| !(c.is_ascii_alphanumeric() || *c == '_'))
                .map_or(0, |(p, c)| p + c.len_utf8());
            let field = &rest[field_start..colon];
            let mut value = colon + 2;
            if rest[value..].starts_with("Some(") {
                value += 5;
            }
            if is_presentation_field(field) && rest[value..].starts_with('"') {
                out.push_str(&rest[..value]);
                out.push_str("\"\"");
                let mut j = value + 1;
                let body = rest.as_bytes();
                while j < body.len() {
                    match body[j] {
                        b'\\' => j += 2,
                        b'"' => {
                            j += 1;
                            break;
                        }
                        _ => j += 1,
                    }
                }
                i += j;
                continue;
            }
            out.push_str(&rest[..colon + 2]);
            i += colon + 2;
            continue;
        }
        out.push_str(rest);
        break;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Model {
        amount: i32,
        display: String,
    }

    #[test]
    fn retained_prose_is_marked_only_during_an_audit() {
        let model = Model {
            amount: 2,
            display: "costs {2} more".into(),
        };
        assert_eq!(retained("Cost", &model.display, &model), "costs {2} more");
        let audited = with_semantic_audit(|| retained("Cost", &model.display, &model));
        assert_eq!(audited, "⟦unverified Cost: Model { amount: 2 }⟧");
        assert!(!semantic_audit_active());
    }

    #[test]
    fn digest_ignores_display_but_tracks_executable_fields() {
        let a = Model {
            amount: 2,
            display: "two".into(),
        };
        let reworded = Model {
            amount: 2,
            display: "2".into(),
        };
        let mutated = Model {
            amount: 9,
            display: "two".into(),
        };
        let digest = |m: &Model| semantic_digest(m, &[&m.display]);
        assert_eq!(digest(&a), digest(&reworded));
        assert_ne!(digest(&a), digest(&mutated));
    }

    #[derive(Debug)]
    struct Sparse {
        flag: bool,
        maybe: Option<u32>,
        list: Vec<u32>,
        count: u32,
    }

    #[test]
    fn structural_reading_drops_default_fields() {
        let sparse = Sparse { flag: false, maybe: None, list: vec![], count: 3 };
        assert_eq!(structural_reading(&sparse, &[]), "Sparse { count: 3 }");
        let set = Sparse { flag: true, maybe: Some(1), list: vec![2], count: 0 };
        assert_eq!(
            structural_reading(&set, &[]),
            "Sparse { flag: true, maybe: Some‹1›, list: [2], count: 0 }"
        );
    }

    #[derive(Debug)]
    enum Modifier {
        SetTo(u32),
    }

    #[test]
    fn default_dropping_keeps_payload_values() {
        assert_eq!(
            drop_default_fields("E { event: Damage { source_filter: None, player: Some(Any), minimum: Some(4) }, modifier: SetTo(3), optional: false }"),
            "E { event: Damage { player: Some(Any), minimum: Some(4) }, modifier: SetTo(3) }"
        );
    }

    #[test]
    fn structural_reading_keeps_tuple_payloads() {
        let sparse = Sparse { flag: false, maybe: Some(4), list: vec![], count: 3 };
        assert_eq!(structural_reading(&(sparse, Modifier::SetTo(9)), &[]), "‹Sparse { maybe: Some‹4›, count: 3 }, SetTo‹9››");
    }

    #[derive(Debug)]
    enum Timing {
        DuringCombatBeforeBlockers,
        NameWord { count: u32 },
    }

    #[test]
    fn variants_read_as_words() {
        assert_eq!(humanize_variant(&Timing::DuringCombatBeforeBlockers), "during combat before blockers");
        assert_eq!(humanize_variant(&Timing::NameWord { count: 3 }), "name word { count: 3 }");
    }

    #[derive(Debug)]
    struct Nested {
        inner: Option<Model>,
        label: String,
    }

    #[test]
    fn digest_ignores_nested_presentation_fields() {
        let nested = |display: &str, label: &str, amount| Nested {
            inner: Some(Model {
                amount,
                display: display.into(),
            }),
            label: label.into(),
        };
        let digest = |n: &Nested| semantic_digest(n, &[]);
        assert_eq!(digest(&nested("a \"quoted\" — one: two", "x", 1)), digest(&nested("b", "y", 1)));
        assert_ne!(digest(&nested("a", "x", 1)), digest(&nested("a", "x", 2)));
    }
}
