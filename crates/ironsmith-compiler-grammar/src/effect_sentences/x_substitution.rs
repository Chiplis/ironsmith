use crate::cards::builders::{
    CardTextError, CharacteristicActionAst, ConditionalEffectAst, DamageActionAst,
    DamagePreventionActionAst, EffectAst, GrantActionAst, GrantedAbilityAst, KeywordActionAst,
    LibraryActionAst, LibraryConsultStopRuleAst, ManaActionAst, ObjectChoiceEffectAst,
    PermanentStateActionAst, PermissionEffectAst, RandomActionAst, ReplacementActionAst,
    RevealLookActionAst, StackActionAst, StatChangeActionAst, SubjectVerbActionAst, TargetAst,
    ZoneMoveActionAst,
};
use crate::effect::Value;
use crate::effect_ast_traversal::try_for_each_nested_effects_mut;
use crate::target::{ChooseSpec, ObjectFilter, SourceReferenceSurface};
use crate::util::{replace_unbound_x_with_value, value_contains_unbound_x};
use ironsmith_core::ValueSurfaceHint;

#[derive(Clone, Copy, PartialEq, Eq)]
enum XBindingScope {
    ModalHeader,
    WhereClause,
}

pub fn replace_modal_header_x_in_effects_ast(
    effects: &mut [EffectAst],
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    XSubstitution {
        replacement,
        clause,
    }
    .replace_effects(effects, XBindingScope::ModalHeader)
}

pub fn replace_unbound_x_in_effects_anywhere(
    effects: &mut [EffectAst],
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    XSubstitution {
        replacement,
        clause,
    }
    .replace_effects(effects, XBindingScope::WhereClause)
}

pub fn replace_unbound_x_in_effect_anywhere(
    effect: &mut EffectAst,
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    XSubstitution {
        replacement,
        clause,
    }
    .replace_effect(effect, XBindingScope::WhereClause)
}

struct XSubstitution<'a> {
    replacement: &'a Value,
    clause: &'a str,
}

impl XSubstitution<'_> {
    fn replace_effects(
        &self,
        effects: &mut [EffectAst],
        scope: XBindingScope,
    ) -> Result<(), CardTextError> {
        for effect in effects {
            self.replace_effect(effect, scope)?;
        }
        Ok(())
    }

    fn replace_effect(
        &self,
        effect: &mut EffectAst,
        scope: XBindingScope,
    ) -> Result<(), CardTextError> {
        if let EffectAst::SubjectVerb(subject) = effect {
            subject
                .action
                .try_for_each_shared_value_mut(|value| self.replace_value(value, scope))?;
            match &mut subject.action {
                SubjectVerbActionAst::KeywordActions(KeywordActionAst::Earthbend { counters, .. }) => {
                    self.replace_value(counters, scope)?;
                }
                SubjectVerbActionAst::Grants(GrantActionAst::GrantActivationCostAlternative {
                    target,
                    ..
                }) => self.replace_in_target(target)?,
                action if scope == XBindingScope::WhereClause => {
                    self.replace_where_x_in_action(action)?;
                }
                _ => {}
            }
            return Ok(());
        }
        let visit_children = |effect: &mut EffectAst| {
            try_for_each_nested_effects_mut(effect, true, |children| {
                self.replace_effects(children, scope)
            })
        };
        if scope == XBindingScope::ModalHeader {
            return visit_children(effect);
        }
        match effect {
            // The initializer can reference an enclosing X; the body reads the
            // new local X and must not be substituted by an outer where clause.
            EffectAst::BindX { value, .. } => self.replace_where_value(value)?,
            EffectAst::Conditionals(ConditionalEffectAst::UnlessPays { effects, cost, .. }) => {
                let consequence_references_it =
                    crate::tag_support::effects_reference_it_tag(effects);
                self.replace_values_in_total_cost(cost)?;
                self.replace_effects(effects, scope)?;
                if consequence_references_it {
                    super::rewrite_unless_cost_source_values_to_it_tag(effect);
                }
            }
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter,
                count,
                count_value,
                ..
            })
            | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
                filter,
                count,
                count_value,
                ..
            })
            | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsBottomOfLibrary {
                filter,
                count,
                count_value,
                ..
            })
            | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsTopOfZone {
                filter,
                count,
                count_value,
                ..
            }) => {
                self.replace_in_filter(filter)?;
                if let Some(value) = count_value.as_mut() {
                    self.replace_where_value(value)?;
                } else if count.dynamic_x {
                    *count_value = Some(self.replacement.clone());
                }
            }
            // "look at the top X cards ..., where X is ... . You may cast a spell
            // with mana value less than X from among them": the selection among
            // the looked-at cards shares the sentence-bound X.
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
                filter,
                ..
            })
            | EffectAst::ObjectChoices(
                ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint { filter, .. },
            ) => {
                self.replace_in_filter(filter)?;
            }
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseSourceLinkedCollection {
                filter,
                aggregate_constraint,
                ..
            }) => {
                self.replace_in_filter(filter)?;
                if let Some(constraint) = aggregate_constraint {
                    self.replace_where_value(&mut constraint.maximum)?;
                    if let Some(minimum) = &mut constraint.minimum {
                        self.replace_where_value(minimum)?;
                    }
                }
            }
            EffectAst::Permissions(
                PermissionEffectAst::MayCastMatchingSpellWithoutPayingManaCost { filter, .. },
            ) => {
                self.replace_in_filter(filter)?;
            }
            _ => return visit_children(effect),
        }
        Ok(())
    }

    fn replace_value(&self, value: &mut Value, scope: XBindingScope) -> Result<(), CardTextError> {
        if value_contains_unbound_x(value) {
            *value = replace_unbound_x_with_value(value.clone(), self.replacement, self.clause)?;
        } else if scope == XBindingScope::WhereClause
            && value.unhinted() == self.replacement.unhinted()
            && self
                .replacement
                .has_surface_hint(ValueSurfaceHint::WhereXIs)
            && !value.has_surface_hint(ValueSurfaceHint::WhereXIs)
        {
            // The effect-local parser may have retained a more precise object
            // surface than the sentence-wide where-X scan.  The latter can
            // include later prose (for example, "put those cards") and must
            // not turn "tapped creatures you control" into "tapped creature
            // cards you control".  Equal unhinted values need only inherit
            // the authored value-level hints.
            *value = value
                .clone()
                .with_surface_hints(self.replacement.surface_hints().iter().copied());
        }
        Ok(())
    }
    fn replace_where_value(&self, value: &mut Value) -> Result<(), CardTextError> {
        self.replace_value(value, XBindingScope::WhereClause)
    }

    fn replace_where_x_in_action(
        &self,
        action: &mut SubjectVerbActionAst,
    ) -> Result<(), CardTextError> {
        match action {
            SubjectVerbActionAst::Library(LibraryActionAst::RetrieveOutsideOrRummage { filter }) => { self.replace_in_filter(filter)?; },
            // The where-X value also fixes a dynamic target count ("deals 1
            // damage to each of up to X target creatures, where X is ...").
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamage {
                target, ..
            })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage {
                target,
                ..
            }) => {
                self.replace_in_target(target)?;
            }
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Amass { amount, .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Support { amount })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Endure { amount, .. })
            // "Roll X six-sided dice, where X is the number of Bobbleheads
            // you control" (Luck Bobblehead).
            | SubjectVerbActionAst::Random(RandomActionAst::RollDiceTotal {
                count: amount,
                ..
            }) => {
                self.replace_where_value(amount)?;
            }
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                source, amount, target, ..
            }) => {
                self.replace_where_value(amount)?;
                self.replace_in_target(source)?;
                self.replace_in_target(target)?;
            }
            SubjectVerbActionAst::Stack(StackActionAst::CounterUnlessPays { target, cost }) => {
                self.replace_in_target(target)?;
                self.replace_values_in_total_cost(cost)?;
            }
            SubjectVerbActionAst::Mana(ManaActionAst::PayMana {
                cost,
                x_value,
                x_maximum,
                independent_x_choice,
                ..
            }) => {
                // This payment chooses its own amount. A later where-X clause
                // may consume its result, but cannot define the payment itself.
                if !*independent_x_choice && cost.has_x() && x_value.is_none() && x_maximum.is_none() {
                    *x_value = Some(self.replacement.clone());
                } else {
                    if let Some(x_value) = x_value.as_mut() {
                        self.replace_where_value(x_value)?;
                    }
                    if let Some(x_maximum) = x_maximum.as_mut() {
                        self.replace_where_value(x_maximum)?;
                    }
                }
            }
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump {
                target,
                ..
            }) => {
                self.replace_in_target(target)?;
            }
            SubjectVerbActionAst::Characteristics(
                CharacteristicActionAst::BecomeBasePtCreature {
                    base_power_toughness: Some((power, toughness)),
                    ..
                },
            ) => {
                self.replace_where_value(power)?;
                self.replace_where_value(toughness)?;
            }
            SubjectVerbActionAst::Stack(StackActionAst::ReduceMatchingSpellCostThisTurn {
                reduction,
                ..
            }) => {
                self.replace_where_value(reduction)?;
            }
            SubjectVerbActionAst::Library(LibraryActionAst::ConsultTopOfLibrary {
                filter,
                stop_rule,
                max_exposed,
                ..
            }) => {
                self.replace_in_filter(filter)?;
                if let LibraryConsultStopRuleAst::MatchCount(count)
                | LibraryConsultStopRuleAst::TotalManaValue(count) = stop_rule
                {
                    self.replace_where_value(count)?;
                }
                if let Some(max_exposed) = max_exposed {
                    self.replace_where_value(max_exposed)?;
                }
            }
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield {
                target,
                count_value,
                ..
            }) => {
                self.replace_in_target(target)?;
                if let Some(value) = count_value.as_mut() {
                    self.replace_where_value(value)?;
                }
            }
            SubjectVerbActionAst::Stack(StackActionAst::Counter { target, .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantToTarget { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MayMoveToZone { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile { target, .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Tap { target })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Untap { target }) => {
                self.replace_in_target(target)?;
            }
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::ReplaceNextDamageToTarget {
                    target,
                    replacement_effects,
                    ..
                },
            ) => {
                self.replace_in_target(target)?;
                self.replace_effects(replacement_effects, XBindingScope::WhereClause)?;
            }
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHand {
                filter, ..
            })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHandOfChosenColor {
                filter,
            })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll { filter, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllOfChosenColor {
                filter,
                ..
            })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAll { filter, .. }) => {
                self.replace_in_filter(filter)?;
            }
            SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealCardsFromHand {
                count,
                count_value,
                ..
            }) => {
                if count.dynamic_x {
                    if let Some(value) = count_value {
                        self.replace_where_value(value)?;
                    } else {
                        *count_value = Some(self.replacement.clone());
                    }
                }
            }
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrary {
                filter,
                count,
                count_value,
                library_position_from_top,
                ..
            }) => {
                self.replace_in_filter(filter)?;
                if let Some(count_value) = count_value.as_mut() {
                    self.replace_where_value(count_value)?;
                } else if count.dynamic_x {
                    *count_value = Some(self.replacement.clone());
                }
                if let Some(position) = library_position_from_top.as_mut() {
                    self.replace_where_value(position)?;
                }
            }
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                target,
                attached_to,
                ..
            }) => {
                self.replace_in_target(target)?;
                if let Some(attached_to) = attached_to {
                    self.replace_in_target(attached_to)?;
                }
            }
            // "shuffle target nonland permanent ... with mana value X or less
            // into its owner's library, where X is ..." (Teferi Akosa).
            SubjectVerbActionAst::Library(LibraryActionAst::ShuffleObjectsIntoLibrary {
                target,
                ..
            }) => {
                self.replace_in_target(target)?;
            }
            SubjectVerbActionAst::Characteristics(
                CharacteristicActionAst::BecomeAuraEnchantment {
                    granted_abilities: abilities,
                    ..
                },
            )
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCopy {
                granted_abilities: abilities,
                ..
            })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll {
                abilities, ..
            })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceAll {
                abilities,
                ..
            }) => {
                self.replace_values_in_granted_abilities(abilities, false)?;
            }
            SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
                target,
                abilities,
                ..
            })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceToTarget {
                target,
                abilities,
                ..
            }) => {
                self.replace_in_target(target)?;
                let rebase_it_to_ability_source = matches!(target, TargetAst::Tagged(tag, _) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str());
                self.replace_values_in_granted_abilities(
                    abilities,
                    rebase_it_to_ability_source,
                )?;
            }
            SubjectVerbActionAst::Grants(GrantActionAst::GrantNextSpellAbilityThisTurn {
                ability,
                ..
            }) => {
                self.replace_values_in_granted_abilities(
                    std::slice::from_mut(ability.as_mut()),
                    false,
                )?;
            }
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Bolster { amount }) => {
                self.replace_where_value(amount)?;
            }
            SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterDamageAddition {
                spec,
            }) => {
                self.replace_where_value(&mut spec.delta)?;
            }
            SubjectVerbActionAst::Replacements(
                ReplacementActionAst::RegisterEnterWithCountersReplacement { count, .. },
            )
            | SubjectVerbActionAst::Replacements(
                ReplacementActionAst::RegisterNextBatchEnterWithCounters { count, .. },
            ) => {
                self.replace_where_value(count)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn replace_in_comparison(
        &self,
        comparison: &mut crate::filter::Comparison,
    ) -> Result<(), CardTextError> {
        use crate::filter::Comparison;

        let value = match comparison {
            Comparison::EqualExpr(value)
            | Comparison::NotEqualExpr(value)
            | Comparison::LessThanExpr(value)
            | Comparison::LessThanOrEqualExpr(value)
            | Comparison::GreaterThanExpr(value)
            | Comparison::GreaterThanOrEqualExpr(value) => value,
            _ => return Ok(()),
        };

        if value_contains_unbound_x(value) {
            **value =
                replace_unbound_x_with_value((**value).clone(), self.replacement, self.clause)?;
        }
        Ok(())
    }

    fn replace_in_filter(&self, filter: &mut ObjectFilter) -> Result<(), CardTextError> {
        if let Some(constraint) = filter.target_set_aggregate_constraint.as_mut() {
            self.replace_where_value(&mut constraint.maximum)?;
            if let Some(minimum) = constraint.minimum.as_mut() {
                self.replace_where_value(minimum)?;
            }
        }
        if let Some(power) = filter.power.as_mut() {
            self.replace_in_comparison(power)?;
        }
        if let Some(toughness) = filter.toughness.as_mut() {
            self.replace_in_comparison(toughness)?;
        }
        if let Some(mana_value) = filter.mana_value.as_mut() {
            self.replace_in_comparison(mana_value)?;
        }
        if let Some(color_count) = filter.color_count.as_mut() {
            self.replace_in_comparison(color_count)?;
        }
        if let Some(card_type_count) = filter.card_type_count.as_mut() {
            self.replace_in_comparison(card_type_count)?;
        }
        if let Some(combat_partner) = filter.blocked_or_was_blocked_by_this_turn.as_mut() {
            self.replace_in_filter(combat_partner)?;
        }
        if let Some(targets_object) = filter.targets_object.as_mut() {
            self.replace_in_filter(targets_object)?;
        }
        if let Some(targets_only_object) = filter.targets_only_object.as_mut() {
            self.replace_in_filter(targets_only_object)?;
        }
        if let Some(attached_to) = filter.attached_to_object.as_mut() {
            self.replace_in_filter(attached_to)?;
        }
        for nested in &mut filter.any_of {
            self.replace_in_filter(nested)?;
        }
        Ok(())
    }

    fn replace_in_target(&self, target: &mut TargetAst) -> Result<(), CardTextError> {
        match target {
            TargetAst::Object(filter, _, _) => self.replace_in_filter(filter)?,
            TargetAst::WithCount(inner, count) => {
                self.replace_in_target(inner)?;
                if count.dynamic_x {
                    *target =
                        TargetAst::WithCountValue(inner.clone(), *count, self.replacement.clone());
                }
            }
            TargetAst::WithCountValue(inner, _, value) => {
                self.replace_in_target(inner)?;
                self.replace_where_value(value)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn replace_values_in_cost_component(
        &self,
        component: &mut crate::model::CompilerCost,
    ) -> Result<(), CardTextError> {
        match component {
            crate::model::CompilerCost::Mana(mana) if mana.has_x() => {
                *component = crate::model::CompilerCost::DynamicMana(
                    ironsmith_core::DynamicManaCost::from_x(mana.clone(), self.replacement.clone()),
                );
            }
            crate::model::CompilerCost::DynamicMana(dynamic) => {
                if dynamic.base.has_x() && dynamic.x_value.is_none() {
                    dynamic.x_value = Some(self.replacement.clone());
                } else if let Some(value) = dynamic.x_value.as_mut() {
                    self.replace_where_value(value)?;
                }
                if let Some(value) = dynamic.additional_generic.as_mut() {
                    self.replace_where_value(value)?;
                }
                if let Some(value) = dynamic.multiplier.as_mut() {
                    self.replace_where_value(value)?;
                }
            }
            crate::model::CompilerCost::Life(value) => self.replace_where_value(value)?,
            crate::model::CompilerCost::Effect(effect)
            | crate::model::CompilerCost::ValidatedEffect(effect) => {
                self.replace_effect(effect, XBindingScope::WhereClause)?
            }
            _ => {}
        }
        Ok(())
    }

    fn replace_values_in_total_cost(
        &self,
        cost: &mut ironsmith_core::TotalCost<crate::model::CompilerCost>,
    ) -> Result<(), CardTextError> {
        match cost.kind() {
            ironsmith_core::TotalCostKind::All(_) => {
                let mut components = cost.costs().to_vec();
                for component in &mut components {
                    self.replace_values_in_cost_component(component)?;
                }
                *cost = ironsmith_core::TotalCost::from_costs(components);
            }
            ironsmith_core::TotalCostKind::OneOf(branches) => {
                let mut branches = branches.to_vec();
                for branch in &mut branches {
                    self.replace_values_in_total_cost(branch)?;
                }
                *cost = ironsmith_core::TotalCost::one_of(branches);
            }
        }
        Ok(())
    }

    fn replace_values_in_granted_abilities(
        &self,
        abilities: &mut [GrantedAbilityAst],
        rebase_it_to_ability_source: bool,
    ) -> Result<(), CardTextError> {
        fn rebase_it_reference(value: &mut Value) {
            match value {
                Value::SurfaceHinted { value, .. }
                | Value::Scaled(value, _)
                | Value::DividedRoundedDown(value, _)
                | Value::HalfRoundedDown(value) => rebase_it_reference(value),
                Value::Add(left, right) | Value::Min(left, right) => {
                    rebase_it_reference(left);
                    rebase_it_reference(right);
                }
                Value::PowerOf(spec)
                | Value::ToughnessOf(spec)
                | Value::ManaValueOf(spec)
                | Value::CommanderCastCountOf(spec)
                | Value::CountersOn(spec, _) => {
                    if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
                    {
                        *spec = Box::new(ChooseSpec::Source.with_surface_hint(
                            ironsmith_core::ChooseSpecSurfaceHint::SourceReference(
                                SourceReferenceSurface::ThisPermanentType("it".to_string()),
                            ),
                        ));
                    }
                }
                _ => {}
            }
        }

        for ability in abilities {
            let static_ability = match ability {
                GrantedAbilityAst::StaticAbility(ability) => match ability.as_mut() {
                    crate::cards::builders::StaticAbilityAst::Static(ability) => ability,
                    _ => continue,
                },
                GrantedAbilityAst::ParsedObjectAbility { ability, .. } => {
                    match ability.kind_mut() {
                        crate::model::CompilerAbilityKindCore::Static(ability) => ability,
                        _ => continue,
                    }
                }
                _ => continue,
            };
            let count = match &mut static_ability.payload {
                ironsmith_core::StaticAbilityPayload::EntersWithCountersValue { count, .. }
                | ironsmith_core::StaticAbilityPayload::EntersWithCountersIfCondition {
                    count,
                    ..
                }
                | ironsmith_core::StaticAbilityPayload::EntersWithCountersAndSubtypesForFilter {
                    count,
                    ..
                } => count,
                _ => continue,
            };
            self.replace_where_value(count)?;
            if rebase_it_to_ability_source {
                rebase_it_reference(count);
            }
        }
        Ok(())
    }
}
