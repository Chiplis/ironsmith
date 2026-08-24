use super::*;

pub fn token_reminder_followup_facts(tokens: &[OwnedLexToken]) -> TokenReminderFollowupFacts {
    let lifecycle_head = primitives::parse_prefix(tokens, lifecycle_head).is_some();
    let has_pronoun = marker_anywhere(tokens, alt((primitives::kw("it"), primitives::kw("them"))));
    TokenReminderFollowupFacts {
        lifecycle_head,
        delayed_pronoun_lifecycle: lifecycle_head && has_pronoun,
        pronoun_trigger_prefix: primitives::parse_prefix(tokens, pronoun_trigger_prefix).is_some(),
    }
}
