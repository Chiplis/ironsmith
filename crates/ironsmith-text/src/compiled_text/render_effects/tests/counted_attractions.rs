use super::*;

#[test]
fn counted_attractions_preserve_the_number_of_openings() {
    for (count, word) in [(2, "two"), (3, "three")] {
        let effect = Effect::new(crate::effects::RepeatEffectsEffect {
            count: Value::Fixed(count),
            effects: vec![Effect::new(crate::effects::OpenAttractionEffect { reminder: false })],
        });
        assert_eq!(describe_effect(&effect), format!("Open {word} Attractions"));
    }
}
