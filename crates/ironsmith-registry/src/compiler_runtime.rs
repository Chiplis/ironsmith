#[cfg(ironsmith_runtime_inline_compiler_runtime)]
use crate as ironsmith;
use ironsmith_compiler as compiler;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompilerIntegrationError {
    Parse(compiler::CardTextError),
    UnsupportedEffect { detail: String },
    UnsupportedStaticAbility { detail: String },
    UnsupportedTrigger { detail: String },
}

impl std::fmt::Display for CompilerIntegrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(err) => err.fmt(f),
            Self::UnsupportedEffect { detail } => {
                write!(
                    f,
                    "runtime compiler integration does not support effect conversion: {detail}"
                )
            }
            Self::UnsupportedStaticAbility { detail } => {
                write!(
                    f,
                    "runtime compiler integration does not support static ability conversion: {detail}"
                )
            }
            Self::UnsupportedTrigger { detail } => {
                write!(
                    f,
                    "runtime compiler integration does not support trigger conversion: {detail}"
                )
            }
        }
    }
}

impl std::error::Error for CompilerIntegrationError {}

impl From<compiler::CardTextError> for CompilerIntegrationError {
    fn from(value: compiler::CardTextError) -> Self {
        Self::Parse(value)
    }
}

struct CompilerEffectModel;

impl ironsmith::effect_model_interpreter::EffectModel for CompilerEffectModel {
    type Effect = compiler::effect::Effect;
    type StaticAbility = compiler::static_abilities::StaticAbility;
    type CardDefinition = compiler::cards::CardDefinition;
    type Ability = compiler::ability::Ability;
    type EmblemDescription = compiler::effect::EmblemDescription;
    type ContinuousTarget = compiler::continuous::EffectTarget;
    type ContinuousModification = compiler::continuous::Modification;
    type RuntimeModification = compiler::effects::continuous::RuntimeModification;
    type Grantable = compiler::grant::Grantable;
    type GrantDuration = compiler::grant::GrantDuration;
    type GrantSpec = compiler::grant::GrantSpec;

    fn downcast_ref<T: 'static>(effect: &Self::Effect) -> Option<&T> {
        effect.downcast_ref::<T>()
    }

    fn payload_type_name(effect: &Self::Effect) -> &'static str {
        effect.payload_type_name()
    }
}

struct CompilerEffectModelHooks;

impl ironsmith::effect_model_interpreter::EffectModelInterpreterHooks<CompilerEffectModel>
    for CompilerEffectModelHooks
{
    type Error = CompilerIntegrationError;

    fn unsupported_effect(&mut self, detail: String) -> Self::Error {
        CompilerIntegrationError::UnsupportedEffect { detail }
    }

    fn runtime_static_ability_hook(
        &mut self,
        ability: compiler::static_abilities::StaticAbility,
    ) -> Result<ironsmith::static_abilities::StaticAbility, Self::Error> {
        runtime_static_ability(ability)
    }

    fn runtime_card_definition_hook(
        &mut self,
        definition: compiler::cards::CardDefinition,
    ) -> Result<ironsmith::cards::CardDefinition, Self::Error> {
        runtime_definition_from_core_model(definition)
    }

    fn runtime_ability_hook(
        &mut self,
        ability: compiler::ability::Ability,
    ) -> Result<ironsmith::ability::Ability, Self::Error> {
        runtime_ability_from_core_model(ability)
    }

    fn runtime_emblem_hook(
        &mut self,
        emblem: compiler::effect::EmblemDescription,
    ) -> Result<ironsmith::effect::EmblemDescription, Self::Error> {
        let mut converted = ironsmith::effect::EmblemDescription::new(&emblem.name, &emblem.text);
        for ability in emblem.abilities {
            converted = converted.with_ability(runtime_ability_from_core_model(ability)?);
        }
        Ok(converted)
    }

    fn runtime_continuous_modification_hook(
        &mut self,
        modification: compiler::continuous::Modification,
    ) -> Result<ironsmith::continuous::Modification, Self::Error> {
        ironsmith::continuous::Modification::try_from_model(
            modification,
            runtime_static_ability,
            runtime_ability_from_core_model,
            convert_removed_ability,
        )
    }

    fn runtime_continuous_runtime_modification_hook(
        &mut self,
        modification: compiler::effects::continuous::RuntimeModification,
    ) -> Result<ironsmith::effects::continuous::RuntimeModification, Self::Error> {
        Ok(match modification {
            compiler::effects::continuous::RuntimeModification::ModifyPowerToughness {
                power,
                toughness,
            } => ironsmith::effects::continuous::RuntimeModification::ModifyPowerToughness {
                power,
                toughness,
            },
            compiler::effects::continuous::RuntimeModification::ChangeControllerToEffectController => {
                ironsmith::effects::continuous::RuntimeModification::ChangeControllerToEffectController
            }
            compiler::effects::continuous::RuntimeModification::ChangeControllerToPlayer(player) => {
                ironsmith::effects::continuous::RuntimeModification::ChangeControllerToPlayer(player)
            }
            compiler::effects::continuous::RuntimeModification::CopyOf {
                source,
                preserve_source_abilities,
            } => ironsmith::effects::continuous::RuntimeModification::CopyOf {
                source,
                preserve_source_abilities,
            },
        })
    }

    fn runtime_grantable_hook(
        &mut self,
        grantable: compiler::grant::Grantable,
    ) -> Result<ironsmith::grant::Grantable, Self::Error> {
        Ok(match grantable {
            compiler::grant::Grantable::Ability(ability) => {
                ironsmith::grant::Grantable::Ability(runtime_static_ability(ability)?)
            }
            compiler::grant::Grantable::AlternativeCast(method) => {
                ironsmith::grant::Grantable::AlternativeCast(convert_alternative_cast(method)?)
            }
            compiler::grant::Grantable::PlayFrom => ironsmith::grant::Grantable::PlayFrom,
            compiler::grant::Grantable::DerivedAlternativeCast(spec) => {
                ironsmith::grant::Grantable::DerivedAlternativeCast(
                    convert_derived_alternative_cast(spec)?,
                )
            }
        })
    }

    fn runtime_grant_duration_hook(
        &mut self,
        duration: compiler::grant::GrantDuration,
    ) -> Result<ironsmith::grant::GrantDuration, Self::Error> {
        match duration {
            compiler::grant::GrantDuration::Forever => Ok(ironsmith::grant::GrantDuration::Forever),
            compiler::grant::GrantDuration::UntilEndOfTurn => {
                Ok(ironsmith::grant::GrantDuration::UntilEndOfTurn)
            }
            compiler::grant::GrantDuration::UntilYourNextTurnEnd => {
                Err(CompilerIntegrationError::UnsupportedEffect {
                    detail:
                        "grant duration UntilYourNextTurnEnd has no runtime one-shot grant model"
                            .to_string(),
                })
            }
        }
    }

    fn runtime_grant_spec_hook(
        &mut self,
        spec: compiler::grant::GrantSpec,
    ) -> Result<ironsmith::grant::GrantSpec, Self::Error> {
        Ok(ironsmith::grant::GrantSpec {
            grantable: self.runtime_grantable_hook(spec.grantable)?,
            filter: spec.filter,
            zone: spec.zone,
            beneficiary: spec.beneficiary,
        })
    }

    fn runtime_external_model_effect_hook(
        &mut self,
        effect: &compiler::effect::Effect,
    ) -> Result<Option<ironsmith::effect::Effect>, Self::Error> {
        if let Some(payload) =
            effect.downcast_ref::<compiler::effects::cards::ImprintFromHandEffect>()
        {
            return Ok(Some(ironsmith::effect::Effect::new(
                ironsmith::effects::cards::ImprintFromHandEffect::new(payload.filter.clone()),
            )));
        }
        Ok(None)
    }
}

fn runtime_effect_from_core_model(
    effect: compiler::effect::Effect,
) -> Result<ironsmith::effect::Effect, CompilerIntegrationError> {
    ironsmith::effect_model_interpreter::interpret_effect_model::<CompilerEffectModel, _>(
        effect,
        &mut CompilerEffectModelHooks,
    )
}

fn remove_redundant_target_only_effects_in_program(
    program: &mut ironsmith::resolution::ResolutionProgram,
) {
    ironsmith::effect_model_interpreter::prune_redundant_target_only_effects_in_program(program);
}

fn convert_removed_ability(
    ability: compiler::ability::Ability,
) -> Result<ironsmith::static_abilities::StaticAbility, CompilerIntegrationError> {
    match ability.kind {
        compiler::ability::AbilityKind::Static(static_ability) => {
            runtime_static_ability(static_ability)
        }
        other => Err(CompilerIntegrationError::UnsupportedEffect {
            detail: format!("continuous RemoveAbility for non-static ability `{other:?}`"),
        }),
    }
}

fn runtime_cost_from_core_model(
    cost: compiler::costs::Cost,
) -> Result<ironsmith::costs::Cost, CompilerIntegrationError> {
    let model = cost.try_map_effect(runtime_effect_from_core_model)?;
    ironsmith::costs::Cost::from_model(model)
        .map_err(|detail| CompilerIntegrationError::UnsupportedEffect { detail })
}

fn runtime_optional_cost_from_core_model(
    cost: compiler::cost::OptionalCost,
) -> Result<ironsmith::cost::OptionalCost, CompilerIntegrationError> {
    cost.try_map(runtime_cost_from_core_model)
}

fn convert_alternative_cast(
    method: compiler::alternative_cast::AlternativeCastingMethod,
) -> Result<ironsmith::alternative_cast::AlternativeCastingMethod, CompilerIntegrationError> {
    let mut method =
        method.try_map(runtime_effect_from_core_model, runtime_cost_from_core_model)?;
    if let ironsmith::alternative_cast::AlternativeCastingMethod::Overload { effects, .. } =
        &mut method
    {
        *effects = effects
            .drain(..)
            .filter_map(detarget_overload_effect)
            .collect();
    }
    Ok(method)
}

fn detarget_overload_effect(
    effect: ironsmith::effect::Effect,
) -> Option<ironsmith::effect::Effect> {
    if effect
        .downcast_ref::<ironsmith::effects::TargetOnlyEffect>()
        .is_some()
    {
        return None;
    }

    if let Some(tagged) = effect.downcast_ref::<ironsmith::effects::TaggedEffect>() {
        let inner = detarget_overload_effect((*tagged.effect).clone())?;
        return Some(ironsmith::effect::Effect::new(
            ironsmith::effects::TaggedEffect::new(tagged.tag.clone(), inner),
        ));
    }

    if let Some(apply) = effect.downcast_ref::<ironsmith::effects::ApplyContinuousEffect>()
        && let Some(ironsmith::target::ChooseSpec::Target(inner)) = &apply.target_spec
        && let ironsmith::target::ChooseSpec::Object(filter) = inner.as_ref()
    {
        let mut detargeted = apply.clone();
        detargeted.target = ironsmith::continuous::EffectTarget::Filter(filter.clone());
        detargeted.target_spec = Some(ironsmith::target::ChooseSpec::Object(filter.clone()));
        detargeted.require_creature_target = false;
        return Some(ironsmith::effect::Effect::new(detargeted));
    }

    Some(effect)
}

fn convert_derived_alternative_cast(
    spec: compiler::grant::DerivedAlternativeCast,
) -> Result<ironsmith::grant::DerivedAlternativeCast, CompilerIntegrationError> {
    Ok(match spec {
        compiler::grant::DerivedAlternativeCast::FlashbackFromCardManaCost { additional_costs } => {
            ironsmith::grant::DerivedAlternativeCast::FlashbackFromCardManaCost {
                additional_costs: additional_costs
                    .into_iter()
                    .map(runtime_cost_from_core_model)
                    .collect::<Result<Vec<_>, _>>()?,
            }
        }
        compiler::grant::DerivedAlternativeCast::EscapeFromCardManaCost { exile_count } => {
            ironsmith::grant::DerivedAlternativeCast::EscapeFromCardManaCost { exile_count }
        }
        compiler::grant::DerivedAlternativeCast::ManaValueAsGenericFromHand => {
            ironsmith::grant::DerivedAlternativeCast::ManaValueAsGenericFromHand
        }
    })
}

fn runtime_static_ability_model(
    ability: compiler::static_abilities::StaticAbility,
) -> Result<ironsmith::static_abilities::CompiledStaticAbility, CompilerIntegrationError> {
    ability.try_map(
        runtime_trigger_from_core_model,
        runtime_effect_from_core_model,
        runtime_cost_from_core_model,
    )
}

fn runtime_static_ability(
    ability: compiler::static_abilities::StaticAbility,
) -> Result<ironsmith::static_abilities::StaticAbility, CompilerIntegrationError> {
    Ok(ironsmith::static_abilities::StaticAbility::from_model(
        runtime_static_ability_model(ability)?,
    ))
}

fn runtime_trigger_from_core_model(
    trigger: compiler::triggers::Trigger,
) -> Result<ironsmith::triggers::Trigger, CompilerIntegrationError> {
    ironsmith::triggers::Trigger::from_model(trigger)
        .map_err(|err| CompilerIntegrationError::UnsupportedTrigger { detail: err.detail })
}

fn runtime_ability_from_core_model(
    ability: compiler::ability::Ability,
) -> Result<ironsmith::ability::Ability, CompilerIntegrationError> {
    let mut converted = ability.try_map(
        runtime_static_ability,
        runtime_trigger_from_core_model,
        runtime_effect_from_core_model,
        runtime_cost_from_core_model,
    )?;
    match &mut converted.kind {
        ironsmith::ability::AbilityKind::Triggered(triggered) => {
            remove_redundant_target_only_effects_in_program(&mut triggered.effects);
        }
        ironsmith::ability::AbilityKind::Activated(activated) => {
            remove_redundant_target_only_effects_in_program(&mut activated.effects);
        }
        ironsmith::ability::AbilityKind::Static(_) => {}
    }
    if converted.text.is_none() {
        converted.text = match &converted.kind {
            ironsmith::ability::AbilityKind::Static(static_ability) => {
                Some(static_ability.display())
            }
            _ => None,
        };
    }
    Ok(converted)
}

fn combine_level_ability_statics(
    abilities: Vec<ironsmith::ability::Ability>,
) -> Vec<ironsmith::ability::Ability> {
    let mut out = Vec::with_capacity(abilities.len());
    let mut levels = Vec::new();

    for ability in abilities {
        let ironsmith::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            out.push(ability);
            continue;
        };
        let Some(level_abilities) = static_ability.level_abilities() else {
            out.push(ability);
            continue;
        };
        levels.extend(level_abilities.iter().cloned());
    }

    if !levels.is_empty() {
        out.push(ironsmith::ability::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::with_level_abilities(levels),
        ));
    }

    out
}

fn runtime_definition_from_core_model(
    definition: compiler::CardDefinition,
) -> Result<ironsmith::cards::CardDefinition, CompilerIntegrationError> {
    let mut definition = definition.try_map(
        runtime_ability_from_core_model,
        runtime_effect_from_core_model,
        runtime_cost_from_core_model,
        convert_alternative_cast,
        runtime_optional_cost_from_core_model,
    )?;
    definition.abilities = combine_level_ability_statics(definition.abilities);
    if let Some(spell_effect) = &mut definition.spell_effect {
        remove_redundant_target_only_effects_in_program(spell_effect);
    }
    Ok(definition)
}

pub fn into_runtime_definition(
    definition: compiler::CardDefinition,
) -> Result<ironsmith::cards::CardDefinition, CompilerIntegrationError> {
    Ok(runtime_definition_from_core_model(definition)?)
}

pub fn into_runtime_compiled_card_text(
    compiled: compiler::CompiledCardText<compiler::CardDefinition>,
) -> Result<compiler::CompiledCardText<ironsmith::cards::CardDefinition>, CompilerIntegrationError>
{
    Ok(compiler::CompiledCardText {
        definition: into_runtime_definition(compiled.definition)?,
        annotations: compiled.annotations,
    })
}

pub fn compile_to_runtime_definition(
    name: &str,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<ironsmith::cards::CardDefinition, CompilerIntegrationError> {
    let builder = compiler::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), name);
    compile_builder_to_runtime_definition(builder, text, allow_unsupported)
}

pub fn compile_builder_to_runtime_definition(
    builder: compiler::CardDefinitionBuilder,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<ironsmith::cards::CardDefinition, CompilerIntegrationError> {
    let text = text.into();
    let compiled =
        compile_builder_to_runtime_compiled_card_text(builder, text.clone(), allow_unsupported)?;
    let mut runtime = compiled.definition;
    if runtime.card.oracle_text.is_empty() {
        runtime.card.oracle_text = text;
    }
    Ok(runtime)
}

#[derive(Debug, Clone)]
pub struct RuntimeBuilderSnapshot {
    pub card: ironsmith::card::Card,
    pub max_saga_chapter: Option<u32>,
    pub has_fuse: bool,
}

impl RuntimeBuilderSnapshot {
    fn into_compiler_builder(self) -> compiler::CardDefinitionBuilder {
        let mut builder = compiler::CardDefinitionBuilder::new(self.card.id, self.card.name);

        if let Some(cost) = self.card.mana_cost {
            builder = builder.mana_cost(cost);
        }
        if let Some(colors) = self.card.color_indicator {
            builder = builder.color_indicator(colors);
        }
        builder = builder
            .supertypes(self.card.supertypes)
            .card_types(self.card.card_types)
            .subtypes(self.card.subtypes)
            .oracle_text(self.card.oracle_text)
            .linked_face_layout(self.card.linked_face_layout);
        if let Some(pt) = self.card.power_toughness {
            builder = builder.power_toughness(pt);
        }
        if let Some(loyalty) = self.card.loyalty {
            builder = builder.loyalty(loyalty);
        }
        if let Some(defense) = self.card.defense {
            builder = builder.defense(defense);
        }
        if let Some(face) = self.card.other_face {
            builder = builder.other_face(face);
        }
        if let Some(face_name) = self.card.other_face_name {
            builder = builder.other_face_name(face_name);
        }
        if self.card.is_token {
            builder = builder.token();
        }
        if let Some(max_chapters) = self.max_saga_chapter {
            builder = builder.saga(max_chapters);
        }
        if self.has_fuse {
            builder = builder.has_fuse();
        }

        builder
    }
}

pub fn compile_runtime_builder_snapshot_to_runtime_definition(
    snapshot: RuntimeBuilderSnapshot,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<ironsmith::cards::CardDefinition, CompilerIntegrationError> {
    compile_builder_to_runtime_definition(snapshot.into_compiler_builder(), text, allow_unsupported)
}

pub fn compile_runtime_builder_snapshot_to_runtime_compiled_card_text(
    snapshot: RuntimeBuilderSnapshot,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<compiler::CompiledCardText<ironsmith::cards::CardDefinition>, CompilerIntegrationError>
{
    compile_builder_to_runtime_compiled_card_text(
        snapshot.into_compiler_builder(),
        text,
        allow_unsupported,
    )
}

pub fn compile_builder_to_runtime_compiled_card_text(
    builder: compiler::CardDefinitionBuilder,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<compiler::CompiledCardText<ironsmith::cards::CardDefinition>, CompilerIntegrationError>
{
    let compiled = compiler::CompilerFacade::new().compile_definition(
        builder,
        text,
        compiler::CompilePolicy { allow_unsupported },
    )?;
    into_runtime_compiled_card_text(compiled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ironsmith::ids::PlayerId;
    use ironsmith::types::CardType;
    use ironsmith::zone::Zone;

    #[test]
    fn compile_to_runtime_definition_handles_representative_spell_text() {
        let definition = compile_to_runtime_definition(
            "Lightning Bolt",
            "Mana cost: {R}\nType: Instant\nLightning Bolt deals 3 damage to any target.",
            false,
        )
        .expect("lightning bolt should compile through runtime compiler integration");

        assert_eq!(definition.name(), "Lightning Bolt");
        assert!(definition.spell_effect.is_some());
        assert_eq!(definition.card.name, "Lightning Bolt");
    }

    #[test]
    fn compile_builder_to_runtime_definition_preserves_manual_metadata() {
        let definition = compile_builder_to_runtime_definition(
            compiler::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), "Command Tower")
                .card_types(vec![CardType::Land]),
            "{T}: Add one mana of any color in your commander's color identity.",
            false,
        )
        .expect("command tower should compile through runtime compiler integration");

        assert!(definition.card.is_land());
        assert_eq!(definition.abilities.len(), 1);
    }

    #[test]
    fn supported_keyword_mechanics_do_not_lower_to_keyword_markers() {
        let cases = [
            (
                "Grapeshot",
                "Mana cost: {1}{R}\nType: Sorcery\nGrapeshot deals 1 damage to any target.\nStorm",
                "CopySpellEffect",
            ),
            (
                "Alive // Well",
                "Mana cost: {3}{G}\nType: Sorcery\nCreate a 3/3 green Centaur creature token.\nFuse",
                "has_fuse: true",
            ),
            (
                "Akrasan Squire",
                "Mana cost: {W}\nType: Creature — Human Soldier\nPower/Toughness: 1/1\nExalted",
                "exalted_attacker",
            ),
            (
                "Abstruse Interference",
                "Mana cost: {2}{U}\nType: Instant\nDevoid\nCounter target spell unless its controller pays {1}.",
                "MakeColorless",
            ),
            (
                "Accorder Paladin",
                "Mana cost: {1}{W}\nType: Creature — Human Knight\nPower/Toughness: 3/1\nBattle cry",
                "ModifyPowerToughnessEffect",
            ),
            (
                "Adaptive Snapjaw",
                "Mana cost: {4}{G}\nType: Creature — Lizard Beast\nPower/Toughness: 6/2\nEvolve",
                "EvolveEffect",
            ),
            (
                "Bilious Skulldweller",
                "Mana cost: {B}\nType: Creature — Phyrexian Insect\nPower/Toughness: 1/1\nDeathtouch\nToxic 1",
                "PoisonCountersEffect",
            ),
            (
                "Doomed Traveler",
                "Mana cost: {W}\nType: Creature — Human Soldier\nPower/Toughness: 1/1\nAfterlife 1",
                "CreateTokenEffect",
            ),
            (
                "Cached Defenses",
                "Mana cost: {2}{G}\nType: Sorcery\nBolster 3.",
                "BolsterEffect",
            ),
            (
                "Aquastrand Spider",
                "Mana cost: {1}{G}\nType: Creature — Spider Mutant\nPower/Toughness: 0/0\nGraft 2\n{G}: Target creature with a +1/+1 counter on it gains reach until end of turn.",
                "MoveCountersEffect",
            ),
            (
                "Arcbound Worker",
                "Mana cost: {1}\nType: Artifact Creature — Construct\nPower/Toughness: 0/0\nModular 1",
                "modular_triggering_object",
            ),
            (
                "Ronin Houndmaster",
                "Mana cost: {2}{R}\nType: Creature — Human Samurai\nPower/Toughness: 2/2\nBushido 1",
                "ModifyPowerToughnessEffect",
            ),
            (
                "Ulamog's Crusher",
                "Mana cost: {8}\nType: Creature — Eldrazi\nPower/Toughness: 8/8\nAnnihilator 2",
                "SacrificePlayerEffect",
            ),
            (
                "Teysa, Envoy of Ghosts",
                "Mana cost: {5}{W}{B}\nType: Legendary Creature — Human Advisor\nPower/Toughness: 4/4\nProtection from creatures",
                "Protection",
            ),
            (
                "Top Library Fixture",
                "Mana cost: {2}{G}\nType: Creature — Bird\nPower/Toughness: 2/3\nYou may look at the top card of your library any time.",
                "LookAtTopCardOfLibrary",
            ),
            (
                "Mystic Remora",
                "Mana cost: {U}\nType: Enchantment\nCumulative upkeep {1}",
                "CumulativeUpkeepEffect",
            ),
            (
                "Cumulative Discard Fixture",
                "Mana cost: {1}{B}\nType: Enchantment\nCumulative upkeep—Discard a card.",
                "DiscardEffect",
            ),
            (
                "Cumulative Choice Fixture",
                "Mana cost: {G}{W}\nType: Enchantment\nCumulative upkeep {G} or {W}",
                "UnlessActionEffect",
            ),
        ];

        for (name, text, expected_debug) in cases {
            let definition = compile_to_runtime_definition(name, text, false)
                .unwrap_or_else(|err| panic!("{name} should compile: {err}"));
            let debug = format!("{definition:#?}");
            assert!(
                !debug.contains("KeywordFallbackText"),
                "{name} should not lower supported mechanics to KeywordFallbackText:\n{debug}"
            );
            assert!(
                !debug.contains("RuleFallbackText"),
                "{name} should not lower supported mechanics to RuleFallbackText:\n{debug}"
            );
            assert!(
                debug.contains(expected_debug),
                "{name} should contain {expected_debug}, got:\n{debug}"
            );
        }
    }

    #[test]
    fn compiler_integrated_definitions_execute_normally_in_runtime() {
        let definition = compile_to_runtime_definition(
            "Llanowar Elves",
            "Mana cost: {G}\nType: Creature — Elf Druid\nPower/Toughness: 1/1\n{T}: Add {G}.",
            false,
        )
        .expect("llanowar elves should compile");

        let mut game =
            ironsmith::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let object_id = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let object = game.object(object_id).expect("object should exist");

        assert_eq!(object.name, "Llanowar Elves");
        assert_eq!(object.abilities.len(), 1);
        assert!(object.abilities[0].is_mana_ability());
    }
}
