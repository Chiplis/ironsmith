//! Set only base toughness (CR 613.4b), leaving base power unchanged:
//! "Creatures your opponents control have base toughness 1." (Maha, Its
//! Feathers Night).

use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct SetBaseToughnessForFilter {
    pub filter: ObjectFilter,
    pub toughness: i32,
    pub condition: Option<crate::ConditionExpr>,
}

impl SetBaseToughnessForFilter {
    pub fn new(filter: ObjectFilter, toughness: i32) -> Self {
        Self {
            filter,
            toughness,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl StaticAbilityKind for SetBaseToughnessForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SetBasePowerToughnessForFilter
    }

    fn display(&self) -> String {
        let subject = pluralized_subject_text(&self.filter);
        let singular = subject.starts_with("enchanted ")
            || subject.starts_with("equipped ")
            || subject.starts_with("this ")
            || subject.starts_with("that ");
        let verb = if singular { "has" } else { "have" };
        let mut text = format!("{subject} {verb} base toughness {}", self.toughness);
        if let Some(condition) = &self.condition {
            text.push(' ');
            text.push_str(&describe_static_condition(condition));
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![effect_with_optional_static_condition(
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::SetToughness {
                    value: Value::Fixed(self.toughness),
                    sublayer: PtSublayer::Setting,
                },
            )
            .with_source_type(EffectSourceType::StaticAbility),
            &self.condition,
        )]
    }
}
