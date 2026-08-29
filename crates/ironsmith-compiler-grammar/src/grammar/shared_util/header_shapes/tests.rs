use super::*;

#[test]
fn parses_saga_and_level_headers() {
    assert_eq!(
        parse_saga_chapter_header("I II — Create a token"),
        Some(SagaChapterHeader {
            chapters: vec![1, 2],
            presentation_label: None,
            body: "Create a token".to_string(),
        })
    );
    assert_eq!(
        parse_saga_chapter_header("I, II — Draw a card."),
        Some(SagaChapterHeader {
            chapters: vec![1, 2],
            presentation_label: None,
            body: "Draw a card.".to_string(),
        })
    );
    assert_eq!(
        parse_saga_chapter_header("I, II — Stampede! — Creatures get +1/+0."),
        Some(SagaChapterHeader {
            chapters: vec![1, 2],
            presentation_label: Some("Stampede!".to_string()),
            body: "Creatures get +1/+0.".to_string(),
        })
    );
    assert_eq!(
        parse_level_header("LEVEL 2-4"),
        Some(LevelHeader {
            minimum: 2,
            maximum: Some(4),
        })
    );
}

#[test]
fn parses_short_saga_presentation_label_surfaces() {
    for label in [
        "Pain",
        "Oblivion",
        "Mega Flare",
        "Chain",
        "Gestalt Mode",
        "Stampede!",
        "Judgment Bolt",
        "Wark",
        "Kerplunk",
        "Double",
        "Triple",
        "Aerospark",
        "Lightning",
        "Ice",
        "Fire",
        "Aerial Blast",
        "Slipstream",
        "Gungnir",
        "Zantetsuken",
        "Hall of Sorrow",
        "Heavenly Strike",
        "Diamond Dust",
    ] {
        let line = format!("I — {label} — Draw a card.");
        let parsed = parse_saga_chapter_header(&line).expect("labeled Saga chapter");
        assert_eq!(parsed.presentation_label.as_deref(), Some(label));
        assert_eq!(parsed.body, "Draw a card.");
    }
}
