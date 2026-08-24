use super::*;

#[test]
fn quoted_effect_can_be_followed_by_an_outer_sorcery_speed_restriction() {
    assert!(authored_trailing_sorcery_speed_restriction(
        "{1}{U}: Create a Fish token with \"This token can't be blocked.\" Activate only as a sorcery."
    ));
    assert!(!authored_trailing_sorcery_speed_restriction(
        "Create a token with \"{T}: Draw a card. Activate only as a sorcery.\""
    ));
    assert!(!authored_trailing_sorcery_speed_restriction(
        "{1}{U}: Create a Fish token."
    ));
}
