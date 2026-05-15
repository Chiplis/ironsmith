pub(crate) use crate::ability::ActivationTiming;
use crate::card::{CardBuilder, LinkedFaceLayout, PowerToughness};
pub use crate::cards::CardDefinition;
use crate::color::ColorSet;
pub use crate::cost::OptionalCost;
use crate::cost::TotalCost;
pub use crate::diagnostics::{CardTextError, ParseAnnotations, TextSpan};
pub(crate) use crate::effect::EffectPredicate;
pub use crate::effect::{ChoiceCount, EventValueSpec, Value};
use crate::mana::ManaCost;
pub use crate::model::reference::RefState;
pub use crate::model::{
    AdditionalCostChoiceOptionAst, ClashOpponentAst, ControlDurationAst, DamageBySpec,
    ExchangeValueAst, ExchangeValueKindAst, ExtraTurnAnchorAst, LibraryBottomOrderAst,
    LibraryConsultModeAst, LibraryConsultStopRuleAst, ObjectRefAst, PlayerAst,
    PreventNextTimeDamageSourceAst, PreventNextTimeDamageTargetAst, RetargetModeAst,
    ReturnControllerAst, SearchLibrarySlotAst, SharedTypeConstraintAst, TargetAst,
    ZoneReplacementDurationAst,
};
use crate::object::AuraAttachmentFilter;
pub use crate::payload::{IfResultPredicate, KeywordAction};
use crate::resolution::ResolutionProgram;
pub(crate) use crate::runtime_backend::semantic::{
    GiftTimingAst, LineAst, ParsedAbility, ParsedCardItem, ParsedLevelAbilityAst,
    ParsedLevelAbilityItemAst, ParsedLineAst, ParsedModalActivatedHeader, ParsedModalAst,
    ParsedModalGate, ParsedModalHeader, ParsedModalModeAst, ParsedRestrictions,
};
pub(crate) use crate::runtime_backend::util::SubjectAst;
pub(crate) use crate::runtime_backend::{
    CarryContext, EffectLoweringContext, IdGenContext, LineInfo, LoweringFrame, MetadataLine,
    NormalizedLine, TokenCopyFollowup, Verb, parse_object_filter_lexed,
};
pub(crate) use crate::runtime_backend::{
    PermissionClauseSpec, PermissionLifetime, ReferenceEnv, ReferenceImports,
};
#[cfg(test)]
pub(crate) use crate::runtime_backend::{
    find_verb, parse_effect_sentence_lexed, parse_shared_color_target_fanout_sentence,
};
use crate::static_abilities::StaticAbility;
pub(crate) use crate::tag::TagKey;
pub use crate::target::{ObjectFilter, PlayerFilter};
pub use crate::types::CardType;
use crate::types::{Subtype, Supertype};
pub use ironsmith_core::CardId;

#[cfg(test)]
pub(crate) mod document_parser {
    pub(crate) use crate::runtime_backend::cst::KeywordLineKindCst;
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GrantedAbilityAst {
    KeywordAction(KeywordAction),
    StaticAbility(StaticAbility),
    ThisAbility,
    MustAttack,
    MustBlock,
    CanAttackAsThoughNoDefender,
    CanBlockAdditionalCreatureEachCombat {
        additional: usize,
    },
    ParsedObjectAbility {
        ability: ParsedAbility,
        display: String,
    },
}

impl From<KeywordAction> for GrantedAbilityAst {
    fn from(action: KeywordAction) -> Self {
        Self::KeywordAction(action)
    }
}

pub(crate) fn replace_whole_word_case_insensitive(
    input: &str,
    needle: &str,
    replacement: &str,
) -> String {
    input.replace(needle, replacement)
}

pub(crate) use crate::runtime_backend::lexer::OwnedLexToken;

pub(crate) use crate::runtime_backend::ast::{
    EffectAst, PredicateAst, ReturnAsAuraAst, StaticAbilityAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, SubjectVerbRoleAst, SubjectVerbSubjectAst, TriggerSpec,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InsteadSemantics {
    SelfReplacement,
    FutureReplacement,
    NonReplacement,
}

#[derive(Debug, Clone)]
pub struct CardDefinitionBuilder {
    pub(crate) card_builder: CardBuilder,
    pub(crate) abilities: Vec<crate::ability::Ability>,
    pub(crate) spell_effect: Option<ResolutionProgram>,
    pub(crate) alternative_casts: Vec<crate::alternative_cast::AlternativeCastingMethod>,
    pub(crate) optional_costs: Vec<OptionalCost>,
    pub(crate) additional_cost: TotalCost,
    pub(crate) aura_attach_filter: Option<AuraAttachmentFilter>,
    pub(crate) has_fuse: bool,
}

impl CardDefinitionBuilder {
    pub fn new(id: CardId, name: impl Into<String>) -> Self {
        Self {
            card_builder: CardBuilder::new(id, name),
            abilities: Vec::new(),
            spell_effect: None,
            alternative_casts: Vec::new(),
            optional_costs: Vec::new(),
            additional_cost: TotalCost::free(),
            aura_attach_filter: None,
            has_fuse: false,
        }
    }

    pub fn mana_cost(mut self, cost: ManaCost) -> Self {
        self.card_builder = self.card_builder.mana_cost(cost);
        self
    }

    pub fn color_indicator(mut self, colors: ColorSet) -> Self {
        self.card_builder = self.card_builder.color_indicator(colors);
        self
    }

    pub fn supertypes(mut self, supertypes: Vec<Supertype>) -> Self {
        self.card_builder = self.card_builder.supertypes(supertypes);
        self
    }

    pub fn card_types(mut self, types: Vec<CardType>) -> Self {
        self.card_builder = self.card_builder.card_types(types);
        self
    }

    pub fn subtypes(mut self, subtypes: Vec<Subtype>) -> Self {
        self.card_builder = self.card_builder.subtypes(subtypes);
        self
    }

    pub fn oracle_text(mut self, text: impl Into<String>) -> Self {
        self.card_builder = self.card_builder.oracle_text(text);
        self
    }

    pub fn other_face(mut self, face: CardId) -> Self {
        self.card_builder = self.card_builder.other_face(face);
        self
    }

    pub fn other_face_name(mut self, name: impl Into<String>) -> Self {
        self.card_builder = self.card_builder.other_face_name(name);
        self
    }

    pub fn linked_face_layout(mut self, layout: LinkedFaceLayout) -> Self {
        self.card_builder = self.card_builder.linked_face_layout(layout);
        self
    }

    pub fn has_fuse(mut self) -> Self {
        self.has_fuse = true;
        self
    }

    pub fn power_toughness(mut self, pt: PowerToughness) -> Self {
        self.card_builder = self.card_builder.power_toughness(pt);
        self
    }

    pub fn loyalty(mut self, loyalty: u32) -> Self {
        self.card_builder = self.card_builder.loyalty(loyalty);
        self
    }

    pub fn defense(mut self, defense: u32) -> Self {
        self.card_builder = self.card_builder.defense(defense);
        self
    }

    pub fn token(mut self) -> Self {
        self.card_builder = self.card_builder.token();
        self
    }

    pub fn with_abilities(mut self, abilities: Vec<crate::ability::Ability>) -> Self {
        self.abilities.extend(abilities);
        self
    }

    pub fn with_ability(mut self, ability: crate::ability::Ability) -> Self {
        if let crate::ability::AbilityKind::Static(static_ability) = &ability.kind
            && matches!(
                static_ability.id(),
                crate::static_abilities::StaticAbilityId::KeywordMarker
            )
        {
            let text = static_ability.display();
            if text.eq_ignore_ascii_case("fuse") {
                self.has_fuse = true;
                return self;
            }
            if let Some(cost) = parse_prototype_marker_cost(&text) {
                self.alternative_casts.push(
                    crate::alternative_cast::AlternativeCastingMethod::alternative_cost(
                        "Prototype",
                        Some(cost),
                        vec![],
                    ),
                );
            }
            if let Some(amount) = parse_standalone_bolster_marker(&text)
                && self
                    .card_builder
                    .card_types_ref()
                    .iter()
                    .any(|card_type| matches!(card_type, CardType::Instant | CardType::Sorcery))
            {
                let effect = crate::effect::Effect::bolster(amount);
                if let Some(existing) = &mut self.spell_effect {
                    existing.push(effect);
                } else {
                    self.spell_effect =
                        Some(crate::resolution::ResolutionProgram::from_effects(vec![
                            effect,
                        ]));
                }
                return self;
            }
        }
        self.abilities.push(ability);
        self
    }

    pub fn with_level_abilities(mut self, abilities: Vec<crate::ability::LevelAbility>) -> Self {
        self.abilities.extend(abilities.into_iter().map(|ability| {
            crate::ability::Ability::static_ability(crate::static_abilities::StaticAbility::level(
                ability,
            ))
        }));
        self
    }

    pub fn optional_cost(mut self, cost: OptionalCost) -> Self {
        self.optional_costs.push(cost);
        self
    }

    pub fn alternative_cast(
        mut self,
        method: crate::alternative_cast::AlternativeCastingMethod,
    ) -> Self {
        self.alternative_casts.push(method);
        self
    }

    pub fn enchants(mut self, filter: AuraAttachmentFilter) -> Self {
        self.aura_attach_filter = Some(filter);
        self
    }

    pub fn apply_keyword_action(self, action: KeywordAction) -> Self {
        match action {
            KeywordAction::Flying => self.flying(),
            KeywordAction::Banding => self.with_ability(crate::ability::Ability::static_ability(
                StaticAbility::banding(),
            )),
            KeywordAction::Defender => self.defender(),
            KeywordAction::Decayed => self.decayed(),
            KeywordAction::Vigilance => self.vigilance(),
            KeywordAction::Prowess => self.prowess(),
            KeywordAction::Trample => self.trample(),
            KeywordAction::Lifelink => self.lifelink(),
            KeywordAction::Deathtouch => self.deathtouch(),
            KeywordAction::Haste => self.haste(),
            KeywordAction::Menace => self.menace(),
            KeywordAction::Reach => self.reach(),
            KeywordAction::Hexproof => self.hexproof(),
            KeywordAction::Indestructible => self.indestructible(),
            KeywordAction::Toxic(amount) => self.toxic(amount),
            KeywordAction::Afterlife(amount) => self.afterlife(amount),
            KeywordAction::Fabricate(amount) => self.fabricate(amount),
            KeywordAction::FirstStrike => self.first_strike(),
            KeywordAction::DoubleStrike => self.double_strike(),
            KeywordAction::Exalted => self.exalted(),
            KeywordAction::Storm => self.storm(),
            KeywordAction::BattleCry => self.battle_cry(),
            KeywordAction::Dethrone => self.dethrone(),
            KeywordAction::Evolve => self.evolve(),
            KeywordAction::Ingest => self.ingest(),
            KeywordAction::Mentor => self.mentor(),
            KeywordAction::Training => self.training(),
            KeywordAction::Riot => self.riot(),
            KeywordAction::Soulbond => self.soulbond(),
            KeywordAction::Soulshift(amount) => self.soulshift(amount),
            KeywordAction::Outlast(cost) => self.outlast(cost),
            KeywordAction::Scavenge(cost) => self.scavenge(cost),
            KeywordAction::Unearth(cost) => self.unearth(cost),
            KeywordAction::Embalm(cost) => self.embalm(cost),
            KeywordAction::Eternalize(cost) => self.eternalize(cost),
            KeywordAction::Emerge(cost) => self.emerge(cost),
            KeywordAction::Vanishing(amount) => self.vanishing(amount),
            KeywordAction::Bloodthirst(amount) => self.bloodthirst(amount),
            KeywordAction::Ninjutsu(cost) => self.ninjutsu(cost),
            KeywordAction::Backup(amount) => self.backup(amount),
            KeywordAction::Dash(cost) => self.dash(cost),
            KeywordAction::Blitz(cost) => self.blitz(cost),
            KeywordAction::BlitzFromGraveyard => {
                self.with_ability(crate::ability::Ability::static_ability(
                    crate::static_abilities::StaticAbility::keyword_marker(
                        KeywordAction::BlitzFromGraveyard.display_text(),
                    ),
                ))
            }
            KeywordAction::Warp(cost) => self.warp(cost),
            KeywordAction::Plot(cost) => self.plot(cost),
            KeywordAction::Disturb(cost) => self.disturb(cost),
            KeywordAction::Spectacle(cost) => self.spectacle(cost),
            KeywordAction::Foretell(cost) => self.foretell(cost),
            KeywordAction::Unleash => self.unleash(),
            KeywordAction::Ward(amount) => self.ward_generic(amount),
            KeywordAction::Afflict(amount) => self.afflict(amount),
            KeywordAction::Undying => self.undying(),
            KeywordAction::Persist => self.persist(),
            KeywordAction::Renown(amount) => self.renown(amount),
            KeywordAction::Myriad => self.myriad(),
            KeywordAction::Mobilize(amount) => self.mobilize(amount),
            KeywordAction::Cipher => self.cipher(),
            KeywordAction::Suspend { time, cost } => self.suspend(time, cost),
            KeywordAction::Overload(cost) => self.overload(cost),
            KeywordAction::Awaken { amount, cost } => self.awaken(amount, cost),
            KeywordAction::Echo { total_cost, .. } => self.echo(total_cost),
            KeywordAction::CumulativeUpkeep { total_cost, .. } => {
                self.cumulative_upkeep(total_cost)
            }
            KeywordAction::Casualty(amount) => self.casualty(amount),
            KeywordAction::VariableCasualtyPlaneswalkerCopy => {
                self.variable_casualty_planeswalker_copy()
            }
            KeywordAction::Conspire => self.conspire(),
            KeywordAction::Amplify(amount) => self.amplify(amount),
            KeywordAction::Devour(multiplier) => self.devour(multiplier),
            KeywordAction::AuraSwap(cost) => self.aura_swap(cost),
            KeywordAction::Ravenous => self.ravenous(),
            KeywordAction::Ascend => self.ascend(),
            KeywordAction::Daybound => self.daybound(),
            KeywordAction::Nightbound => self.nightbound(),
            KeywordAction::Haunt => self.haunt(),
            KeywordAction::Provoke => self.provoke(),
            KeywordAction::Enlist => self.enlist(),
            KeywordAction::Crew {
                amount,
                timing,
                additional_restrictions,
            } => self.crew(amount, timing, additional_restrictions),
            KeywordAction::Undaunted => self.undaunted(),
            KeywordAction::Extort => self.extort(),
            KeywordAction::Partner => self.partner(),
            KeywordAction::StartYourEngines => {
                self.with_ability(crate::ability::Ability::static_ability(
                    crate::static_abilities::StaticAbility::start_your_engines(),
                ))
            }
            KeywordAction::Assist => self.assist(),
            KeywordAction::SplitSecond => self.split_second(),
            KeywordAction::Cascade => self.cascade(),
            KeywordAction::Rebound => self.rebound(),
            KeywordAction::Sunburst => self.sunburst(),
            KeywordAction::ReadAhead => self.read_ahead(),
            KeywordAction::Fading(amount) => self.fading(amount),
            KeywordAction::Modular(amount) => self.modular(amount),
            KeywordAction::ModularSunburst => self.modular_sunburst(),
            KeywordAction::Graft(amount) => self.graft(amount),
            KeywordAction::Rampage(amount) => self.rampage(amount),
            KeywordAction::Bushido(amount) => self.bushido(amount),
            KeywordAction::ProtectionFrom(colors) => self.protection_from(colors),
            KeywordAction::ProtectionFromAllColors => {
                self.with_ability(crate::ability::Ability::static_ability(
                    crate::static_abilities::StaticAbility::protection(
                        crate::ability::ProtectionFrom::AllColors,
                    ),
                ))
            }
            KeywordAction::ProtectionFromColorless => {
                self.with_ability(crate::ability::Ability::static_ability(
                    crate::static_abilities::StaticAbility::protection(
                        crate::ability::ProtectionFrom::Colorless,
                    ),
                ))
            }
            KeywordAction::ProtectionFromEverything => {
                self.with_ability(crate::ability::Ability::static_ability(
                    crate::static_abilities::StaticAbility::protection(
                        crate::ability::ProtectionFrom::Everything,
                    ),
                ))
            }
            KeywordAction::ProtectionFromChosenPlayer => {
                self.with_ability(crate::ability::Ability::static_ability(
                    crate::static_abilities::StaticAbility::protection(
                        crate::ability::ProtectionFrom::ChosenPlayer,
                    ),
                ))
            }
            KeywordAction::ProtectionFromChosenColor => {
                self.with_ability(crate::ability::Ability::static_ability(
                    crate::static_abilities::StaticAbility::protection(
                        crate::ability::ProtectionFrom::ChosenColor,
                    ),
                ))
            }
            KeywordAction::ProtectionFromFilter(filter) => self.protection_from_filter(filter),
            KeywordAction::ProtectionFromCardType(card_type) => {
                self.protection_from_card_type(card_type)
            }
            KeywordAction::ProtectionFromSubtype(subtype) => self.protection_from_subtype(subtype),
            KeywordAction::Devoid => self.devoid(),
            KeywordAction::Annihilator(amount) => self.annihilator(amount),
            KeywordAction::ForMirrodin => self.for_mirrodin(),
            KeywordAction::LivingWeapon => self.living_weapon(),
            KeywordAction::Marker(name) if name.eq_ignore_ascii_case("fuse") => self.has_fuse(),
            KeywordAction::Marker(name)
                if parse_standalone_bolster_marker(name).is_some()
                    && self.card_builder.card_types_ref().iter().any(|card_type| {
                        matches!(card_type, CardType::Instant | CardType::Sorcery)
                    }) =>
            {
                self.with_ability(crate::ability::Ability::static_ability(
                    crate::static_abilities::StaticAbility::keyword_marker(format!(
                        "Bolster {}",
                        parse_standalone_bolster_marker(name).unwrap()
                    )),
                ))
            }
            KeywordAction::MarkerText(text) if text.eq_ignore_ascii_case("fuse") => self.has_fuse(),
            KeywordAction::MarkerText(text)
                if parse_standalone_bolster_marker(&text).is_some()
                    && self.card_builder.card_types_ref().iter().any(|card_type| {
                        matches!(card_type, CardType::Instant | CardType::Sorcery)
                    }) =>
            {
                let amount = parse_standalone_bolster_marker(&text).unwrap();
                self.with_ability(crate::ability::Ability::static_ability(
                    crate::static_abilities::StaticAbility::keyword_marker(format!(
                        "Bolster {amount}"
                    )),
                ))
            }
            other if other.lowers_to_static_ability() => {
                let text = other.display_text();
                let static_ability =
                    crate::runtime_backend::static_ability_helpers::static_ability_for_keyword_action(
                        other,
                    )
                    .unwrap_or_else(|| {
                        crate::static_abilities::StaticAbility::keyword_marker(text.clone())
                    });
                self.with_ability(crate::ability::Ability::static_ability(static_ability))
            }
            other => self.with_ability(crate::ability::Ability::triggered(
                crate::triggers::Trigger::custom("compiler-keyword", other.display_text()),
                crate::resolution::ResolutionProgram::default(),
            )),
        }
    }

    pub fn apply_metadata(
        mut self,
        meta: impl Into<crate::front_end::MetadataLine>,
    ) -> Result<Self, CardTextError> {
        let meta = meta.into();
        match meta {
            crate::front_end::MetadataLine::ManaCost(raw) => {
                let cost = crate::runtime_backend::parse_scryfall_mana_cost(&raw)?;
                if !cost.is_empty() {
                    self.card_builder = self.card_builder.mana_cost(cost);
                }
            }
            crate::front_end::MetadataLine::TypeLine(raw) => {
                let (supertypes, card_types, subtypes) =
                    crate::runtime_backend::parse_type_line(&raw)?;
                if !supertypes.is_empty() {
                    self.card_builder = self.card_builder.supertypes(supertypes);
                }
                if !card_types.is_empty() {
                    self.card_builder = self.card_builder.card_types(card_types);
                }
                if !subtypes.is_empty() {
                    self.card_builder = self.card_builder.subtypes(subtypes);
                }
            }
            crate::front_end::MetadataLine::PowerToughness(raw) => {
                if let Some(pt) = crate::runtime_backend::parse_power_toughness(&raw) {
                    self.card_builder = self.card_builder.power_toughness(pt);
                }
            }
            crate::front_end::MetadataLine::Loyalty(raw) => {
                if let Ok(value) = raw.trim().parse::<u32>() {
                    self.card_builder = self.card_builder.loyalty(value);
                }
            }
            crate::front_end::MetadataLine::Defense(raw) => {
                if let Ok(value) = raw.trim().parse::<u32>() {
                    self.card_builder = self.card_builder.defense(value);
                }
            }
        }
        Ok(self)
    }

    pub fn parse_text(self, text: impl Into<String>) -> Result<CardDefinition, CardTextError> {
        crate::runtime_backend::parse_card_text(self, text)
    }

    pub fn parse_text_allow_unsupported(
        self,
        text: impl Into<String>,
    ) -> Result<CardDefinition, CardTextError> {
        crate::runtime_backend::parse_card_text_allow_unsupported(self, text)
    }

    pub fn flying(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::flying(),
        ))
    }

    pub fn defender(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::defender(),
        ))
    }

    pub fn decayed(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::cant_block(),
        ))
        .with_ability(crate::runtime_backend::static_ability_helpers::decayed_triggered_ability())
    }

    pub fn vigilance(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::vigilance(),
        ))
    }

    pub fn prowess(self) -> Self {
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::spell_cast(
                    Some(crate::target::ObjectFilter::noncreature_spell()),
                    crate::target::PlayerFilter::You,
                ),
                effects: vec![crate::effect::Effect::pump(
                    1,
                    1,
                    crate::target::ChooseSpec::Source,
                    crate::effect::Until::EndOfTurn,
                )]
                .into(),
                choices: vec![],
                intervening_if: None,
                presentation_label: Some("keyword:prowess".to_string()),
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn trample(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::trample(),
        ))
    }

    pub fn lifelink(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::lifelink(),
        ))
    }

    pub fn deathtouch(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::deathtouch(),
        ))
    }

    pub fn haste(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::haste(),
        ))
    }

    pub fn menace(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::menace(),
        ))
    }

    pub fn reach(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::reach(),
        ))
    }

    pub fn hexproof(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::hexproof(),
        ))
    }

    pub fn indestructible(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::indestructible(),
        ))
    }

    pub fn toxic(self, amount: u32) -> Self {
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_deals_combat_damage_to_player(),
                effects: vec![crate::effect::Effect::poison_counters_player(
                    amount as i32,
                    crate::target::PlayerFilter::DamagedPlayer,
                )]
                .into(),
                choices: vec![],
                intervening_if: None,
                presentation_label: Some(format!("keyword:toxic {amount}")),
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn ward_generic(self, amount: u32) -> Self {
        let mana = ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Generic(amount as u8)]);
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::ward(TotalCost::mana(mana)),
        ))
    }

    pub fn first_strike(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::first_strike(),
        ))
    }

    pub fn double_strike(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::double_strike(),
        ))
    }

    pub fn afflict(self, amount: u32) -> Self {
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_becomes_blocked(),
                effects: vec![crate::effect::Effect::lose_life_player(
                    amount as i32,
                    crate::target::PlayerFilter::Defending,
                )]
                .into(),
                choices: vec![],
                intervening_if: None,
                presentation_label: Some(format!("keyword:afflict {amount}")),
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn amplify(self, amount: u32) -> Self {
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_enters_battlefield(),
            vec![crate::effect::Effect::amplify(amount)],
        ))
    }

    pub fn devour(self, multiplier: u32) -> Self {
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_enters_battlefield(),
                effects: vec![crate::effect::Effect::devour(multiplier)].into(),
                choices: vec![],
                intervening_if: None,
                presentation_label: Some(format!("keyword:devour {multiplier}")),
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn afterlife(self, amount: u32) -> Self {
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_dies(),
            vec![crate::effect::Effect::create_tokens(
                Self::afterlife_spirit_token(),
                amount,
            )],
        ))
    }

    pub fn fabricate(self, amount: u32) -> Self {
        let put_description = if amount == 1 {
            "Put a +1/+1 counter on this creature".to_string()
        } else {
            format!("Put {amount} +1/+1 counters on this creature")
        };
        let create_description = if amount == 1 {
            "Create a 1/1 colorless Servo artifact creature token".to_string()
        } else {
            format!("Create {amount} 1/1 colorless Servo artifact creature tokens")
        };
        let modes = vec![
            crate::effect::EffectMode::new(
                put_description,
                vec![crate::effect::Effect::put_counters(
                    crate::object::CounterType::PlusOnePlusOne,
                    amount as i32,
                    crate::target::ChooseSpec::Source,
                )],
            ),
            crate::effect::EffectMode::new(
                create_description,
                vec![crate::effect::Effect::create_tokens(
                    Self::fabricate_servo_token(),
                    amount,
                )],
            ),
        ];

        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_enters_battlefield(),
            vec![crate::effect::Effect::choose_one(modes)],
        ))
    }

    pub fn exalted(self) -> Self {
        let attacker_tag = crate::tag::TagKey::from("exalted_attacker");
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::attacks_alone(
                crate::target::ObjectFilter::creature().you_control(),
            ),
            vec![
                crate::effect::Effect::tag_triggering_object(attacker_tag.clone()),
                crate::effect::Effect::pump(
                    1,
                    1,
                    crate::target::ChooseSpec::Tagged(attacker_tag),
                    crate::effect::Until::EndOfTurn,
                ),
            ],
        ))
    }

    pub fn storm(self) -> Self {
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::you_cast_this_spell(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::with_id(
                        0,
                        crate::effect::Effect::new(crate::effects::CopySpellEffect::new(
                            crate::target::ChooseSpec::Source,
                            crate::effect::Value::SpellsCastBeforeThisTurn(
                                crate::target::PlayerFilter::You,
                            ),
                        )),
                    ),
                    crate::effect::Effect::new(crate::effects::ChooseNewTargetsEffect::may(
                        crate::effect::EffectId(0),
                    )),
                ]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Stack],
        })
    }

    pub fn battle_cry(self) -> Self {
        let mut filter = crate::target::ObjectFilter::creature()
            .you_control()
            .other();
        filter.attacking = true;
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_attacks(),
            vec![crate::effect::Effect::for_each(
                filter,
                vec![crate::effect::Effect::pump(
                    1,
                    0,
                    crate::target::ChooseSpec::Iterated,
                    crate::effect::Until::EndOfTurn,
                )],
            )],
        ))
    }

    pub fn dethrone(self) -> Self {
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_attacks_player_with_most_life(),
            vec![crate::effect::Effect::put_counters(
                crate::object::CounterType::PlusOnePlusOne,
                1,
                crate::target::ChooseSpec::Source,
            )],
        ))
    }

    pub fn evolve(self) -> Self {
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::enters_battlefield(
                crate::target::ObjectFilter::creature().you_control(),
                None,
            ),
            vec![crate::effect::Effect::new(
                crate::effects::EvolveEffect::new(),
            )],
        ))
    }

    pub fn ingest(self) -> Self {
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_deals_combat_damage_to_player(),
            vec![crate::effect::Effect::exile_top_of_library_player(
                1,
                crate::target::PlayerFilter::DamagedPlayer,
                "ingested",
                None,
            )],
        ))
    }

    pub fn mentor(self) -> Self {
        let mut target_filter =
            crate::target::ObjectFilter::creature().with_power_less_than_source();
        target_filter.attacking = true;
        let target =
            crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(target_filter));

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_attacks(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::put_counters(
                        crate::object::CounterType::PlusOnePlusOne,
                        1,
                        target.clone(),
                    ),
                ]),
                choices: vec![target],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn training(self) -> Self {
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_attacks_with_greater_power(),
            vec![
                crate::effect::Effect::put_counters(
                    crate::object::CounterType::PlusOnePlusOne,
                    1,
                    crate::target::ChooseSpec::Source,
                ),
                crate::effect::Effect::emit_keyword_action(
                    crate::events::KeywordActionKind::Train,
                    1,
                ),
            ],
        ))
    }

    pub fn enlist(self) -> Self {
        let tag = "enlisted_creature";
        let mut filter = crate::target::ObjectFilter::creature()
            .you_control()
            .other();
        filter.untapped = true;
        filter.nonattacking = true;
        filter.enlist_eligible = true;
        let effects = vec![
            crate::effect::Effect::tag_triggering_object("enlist_attacker"),
            crate::effect::Effect::choose_objects(filter, 1, crate::target::PlayerFilter::You, tag),
            crate::effect::Effect::tap(crate::target::ChooseSpec::Tagged(tag.into())),
            crate::effect::Effect::pump_for_each(
                crate::target::ChooseSpec::Tagged("enlist_attacker".into()),
                1,
                0,
                crate::effect::Value::PowerOf(Box::new(crate::target::ChooseSpec::Tagged(
                    tag.into(),
                ))),
                crate::effect::Until::EndOfTurn,
            ),
        ];
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_attacks(),
            vec![crate::effect::Effect::may_player(
                crate::target::PlayerFilter::You,
                effects,
            )],
        ))
    }

    pub fn crew(
        self,
        amount: u32,
        timing: crate::ability::ActivationTiming,
        additional_restrictions: Vec<String>,
    ) -> Self {
        let cost = crate::cost::TotalCost::from_cost(crate::costs::Cost::effect(
            crate::effects::CrewCostEffect::new(amount),
        ));
        let animate = crate::effect::Effect::new(crate::effects::ApplyContinuousEffect::new(
            crate::continuous::EffectTarget::Source,
            crate::continuous::Modification::AddCardTypes(vec![crate::types::CardType::Creature]),
            crate::effect::Until::EndOfTurn,
        ));
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![animate]),
                choices: Vec::new(),
                timing,
                additional_restrictions,
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn riot(self) -> Self {
        let modes = vec![
            crate::effect::EffectMode::new(
                "This creature enters with a +1/+1 counter on it",
                vec![crate::effect::Effect::put_counters(
                    crate::object::CounterType::PlusOnePlusOne,
                    1,
                    crate::target::ChooseSpec::Source,
                )],
            ),
            crate::effect::EffectMode::new(
                "This creature gains haste",
                vec![crate::effect::Effect::grant_object_ability_to_source(
                    crate::ability::Ability::static_ability(
                        crate::static_abilities::StaticAbility::haste(),
                    ),
                )],
            ),
        ];

        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_enters_battlefield(),
            vec![crate::effect::Effect::choose_one(modes)],
        ))
    }

    pub fn unleash(self) -> Self {
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_enters_battlefield(),
            vec![crate::effect::Effect::may(vec![
                crate::effect::Effect::put_counters(
                    crate::object::CounterType::PlusOnePlusOne,
                    1,
                    crate::target::ChooseSpec::Source,
                ),
            ])],
        ))
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::unleash(),
        ))
    }

    pub fn outlast(self, cost: ManaCost) -> Self {
        let total_cost = TotalCost::from_costs(vec![
            crate::costs::Cost::mana(cost),
            crate::costs::Cost::tap(),
        ]);

        self.with_ability(crate::ability::Ability::activated_with_timing(
            total_cost,
            vec![crate::effect::Effect::put_counters(
                crate::object::CounterType::PlusOnePlusOne,
                1,
                crate::target::ChooseSpec::Source,
            )],
            crate::ability::ActivationTiming::SorcerySpeed,
        ))
    }

    pub fn scavenge(self, cost: ManaCost) -> Self {
        let total_cost = TotalCost::from_costs(vec![
            crate::costs::Cost::mana(cost),
            crate::costs::Cost::exile_self(),
        ]);
        let target = crate::target::ChooseSpec::target(crate::target::ChooseSpec::creature());

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: total_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::put_counters(
                        crate::object::CounterType::PlusOnePlusOne,
                        crate::effect::Value::SourcePower,
                        target.clone(),
                    ),
                ]),
                choices: vec![target],
                timing: crate::ability::ActivationTiming::SorcerySpeed,
                additional_restrictions: Vec::new(),
                activation_restrictions: Vec::new(),
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: Vec::new(),
                is_loyalty_ability: false,
            }),
            functional_zones: vec![crate::zone::Zone::Graveyard],
        })
    }

    pub fn unearth(self, cost: ManaCost) -> Self {
        let total_cost = TotalCost::from_cost(crate::costs::Cost::mana(cost));

        self.with_ability(
            crate::ability::Ability::activated_with_timing(
                total_cost,
                vec![crate::effect::Effect::unearth()],
                crate::ability::ActivationTiming::SorcerySpeed,
            )
            .in_zones(vec![crate::zone::Zone::Graveyard]),
        )
    }

    pub fn embalm(self, cost: ManaCost) -> Self {
        let total_cost = TotalCost::from_costs(vec![
            crate::costs::Cost::mana(cost),
            crate::costs::Cost::exile_self(),
        ]);
        let create_embalmed_copy = crate::effect::Effect::new(
            crate::effects::CreateTokenCopyEffect::new(
                crate::target::ChooseSpec::Source,
                1,
                crate::target::PlayerFilter::You,
            )
            .set_colors(crate::color::ColorSet::WHITE)
            .added_subtype(crate::types::Subtype::Zombie)
            .without_mana_cost(),
        );

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: total_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    create_embalmed_copy,
                ]),
                choices: vec![],
                timing: crate::ability::ActivationTiming::SorcerySpeed,
                additional_restrictions: Vec::new(),
                activation_restrictions: Vec::new(),
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: Vec::new(),
                is_loyalty_ability: false,
            }),
            functional_zones: vec![crate::zone::Zone::Graveyard],
        })
    }

    pub fn eternalize(self, cost: ManaCost) -> Self {
        let total_cost = TotalCost::from_costs(vec![
            crate::costs::Cost::mana(cost),
            crate::costs::Cost::exile_self(),
        ]);
        let create_eternalized_copy = crate::effect::Effect::new(
            crate::effects::CreateTokenCopyEffect::new(
                crate::target::ChooseSpec::Source,
                1,
                crate::target::PlayerFilter::You,
            )
            .set_colors(crate::color::ColorSet::BLACK)
            .added_subtype(crate::types::Subtype::Zombie)
            .set_base_power_toughness(4, 4)
            .without_mana_cost(),
        );

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: total_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    create_eternalized_copy,
                ]),
                choices: vec![],
                timing: crate::ability::ActivationTiming::SorcerySpeed,
                additional_restrictions: Vec::new(),
                activation_restrictions: Vec::new(),
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: Vec::new(),
                is_loyalty_ability: false,
            }),
            functional_zones: vec![crate::zone::Zone::Graveyard],
        })
    }

    pub fn emerge(self, cost: ManaCost) -> Self {
        self.alternative_cast(
            crate::alternative_cast::AlternativeCastingMethod::alternative_cost(
                "Emerge",
                Some(cost),
                vec![crate::costs::Cost::sacrifice(
                    crate::target::ObjectFilter::creature().you_control(),
                )],
            ),
        )
    }

    pub fn aura_swap(self, cost: ManaCost) -> Self {
        let total_cost = TotalCost::from_cost(crate::costs::Cost::mana(cost));

        self.with_ability(crate::ability::Ability::activated(
            total_cost,
            vec![crate::effect::Effect::aura_swap()],
        ))
    }

    pub fn ninjutsu(self, cost: ManaCost) -> Self {
        let total_cost = TotalCost::from_costs(vec![
            crate::costs::Cost::mana(cost),
            crate::costs::Cost::effect(crate::effect::Effect::new(
                crate::effects::NinjutsuCostEffect::new(),
            )),
        ]);

        self.with_ability(
            crate::ability::Ability::activated_with_timing(
                total_cost,
                vec![crate::effect::Effect::ninjutsu()],
                crate::ability::ActivationTiming::DuringCombat,
            )
            .in_zones(vec![crate::zone::Zone::Hand]),
        )
    }

    pub fn renown(self, amount: u32) -> Self {
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_deals_combat_damage_to_player(),
            vec![crate::effect::Effect::renown_source(amount)],
        ))
    }

    pub fn soulbond(self) -> Self {
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::enters_battlefield(
                crate::target::ObjectFilter::creature().you_control(),
                None,
            ),
            vec![crate::effect::Effect::new(
                crate::effects::SoulbondPairEffect::new(),
            )],
        ))
    }

    pub fn soulshift(self, amount: u32) -> Self {
        let filter = crate::target::ObjectFilter::default()
            .with_subtype(Subtype::Spirit)
            .owned_by(crate::target::PlayerFilter::You)
            .in_zone(crate::zone::Zone::Graveyard)
            .with_mana_value(crate::filter::Comparison::LessThanOrEqual(amount as i32));
        let target = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(filter))
            .with_count(ChoiceCount::up_to(1));

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_dies(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::return_from_graveyard_to_hand(target.clone()),
                ]),
                choices: vec![target],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn backup(self, amount: u32) -> Self {
        let text = format!("Backup {amount}");
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::keyword_marker(text.clone()),
        ))
    }

    pub fn cipher(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::keyword_marker("Cipher"),
        ))
    }

    pub fn dash(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(crate::alternative_cast::AlternativeCastingMethod::Dash { cost });
        self
    }

    pub fn blitz(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(crate::alternative_cast::AlternativeCastingMethod::Blitz {
                total_cost: TotalCost::mana(cost),
            });
        self
    }

    pub fn warp(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(crate::alternative_cast::AlternativeCastingMethod::Warp { cost });
        self
    }

    pub fn plot(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(crate::alternative_cast::AlternativeCastingMethod::Plot { cost });
        self
    }

    pub fn suspend(self, time: u32, cost: ManaCost) -> Self {
        self.alternative_cast(crate::alternative_cast::AlternativeCastingMethod::Suspend {
            cost,
            time,
        })
        .with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::beginning_of_upkeep(
                    crate::target::PlayerFilter::You,
                ),
                effects: vec![crate::effect::Effect::remove_counters(
                    crate::object::CounterType::Time,
                    1,
                    crate::target::ChooseSpec::Source,
                )]
                .into(),
                choices: vec![],
                intervening_if: Some(crate::ConditionExpr::SourceHasCounterAtLeast {
                    counter_type: crate::object::CounterType::Time,
                    count: 1,
                }),
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Exile],
        })
        .with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::new(
                    crate::triggers::CounterRemovedFromTrigger::new(
                        crate::target::ObjectFilter::source(),
                    ),
                ),
                effects: vec![crate::effect::Effect::may(vec![
                    crate::effect::Effect::new(
                        crate::effects::CastSourceEffect::new()
                            .without_paying_mana_cost()
                            .require_exile(),
                    ),
                ])]
                .into(),
                choices: vec![],
                intervening_if: Some(crate::ConditionExpr::SourceHasNoCounter(
                    crate::object::CounterType::Time,
                )),
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Exile],
        })
    }

    pub fn disturb(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(crate::alternative_cast::AlternativeCastingMethod::Disturb { cost });
        self
    }

    pub fn overload(mut self, cost: ManaCost) -> Self {
        self.alternative_casts.push(
            crate::alternative_cast::AlternativeCastingMethod::Overload {
                cost,
                effects: Vec::new(),
            },
        );
        self
    }

    pub fn awaken(mut self, amount: u32, cost: ManaCost) -> Self {
        let mut effects = self
            .spell_effect
            .as_ref()
            .map(|program| program.all_effects_owned())
            .unwrap_or_default();
        let spec = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
            ObjectFilter::land().you_control(),
        ));
        effects.push(crate::effect::Effect::new(
            crate::effects::EarthbendEffect::new(spec, amount),
        ));
        self.alternative_casts
            .push(crate::alternative_cast::AlternativeCastingMethod::Awaken {
                amount,
                cost,
                effects,
            });
        self
    }

    pub fn foretell(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(crate::alternative_cast::AlternativeCastingMethod::Foretell { cost });
        self
    }

    pub fn spectacle(mut self, cost: ManaCost) -> Self {
        self.alternative_casts.push(
            crate::alternative_cast::AlternativeCastingMethod::alternative_cost_with_condition(
                "Spectacle",
                Some(cost),
                Vec::new(),
                crate::static_abilities::ThisSpellCostCondition::ConditionExpr {
                    condition: crate::ConditionExpr::OpponentLostLifeThisTurn,
                    display: "an opponent lost life this turn".to_string(),
                },
            ),
        );
        self
    }

    pub fn echo(self, total_cost: TotalCost) -> Self {
        let payment_effects = crate::costs::total_cost_to_payment_effects(&total_cost);

        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::enters_with_counters_value(
                crate::object::CounterType::Echo,
                1.into(),
            ),
        ))
        .with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::beginning_of_upkeep(
                    crate::target::PlayerFilter::You,
                ),
                effects: vec![crate::effect::Effect::conditional_only(
                    crate::effect::Condition::SourceIsInZone(crate::zone::Zone::Battlefield),
                    vec![
                        crate::effect::Effect::with_id(
                            0,
                            crate::effect::Effect::remove_counters(
                                crate::object::CounterType::Echo,
                                1,
                                crate::target::ChooseSpec::Source,
                            ),
                        ),
                        crate::effect::Effect::if_then(
                            crate::effect::EffectId(0),
                            crate::effect::EffectPredicate::Happened,
                            vec![crate::effect::Effect::unless_action(
                                vec![crate::effect::Effect::sacrifice_source()],
                                payment_effects,
                                crate::target::PlayerFilter::You,
                            )],
                        ),
                    ],
                )]
                .into(),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn cumulative_upkeep(self, total_cost: TotalCost) -> Self {
        let payment_effects = crate::costs::total_cost_to_payment_effects(&total_cost);

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::beginning_of_upkeep(
                    crate::target::PlayerFilter::You,
                ),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::put_counters_on_source(
                        crate::object::CounterType::Age,
                        1,
                    ),
                    crate::effect::Effect::cumulative_upkeep(
                        payment_effects,
                        crate::target::PlayerFilter::You,
                        vec![crate::effect::Effect::sacrifice_source()],
                    ),
                ]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn casualty(self, power: u32) -> Self {
        let mut creature_filter = crate::target::ObjectFilter::creature().you_control();
        creature_filter.power = Some(crate::filter::Comparison::GreaterThanOrEqual(power as i32));

        self.with_ability(
            crate::ability::Ability::triggered(
                crate::triggers::Trigger::you_cast_this_spell(),
                vec![crate::effect::Effect::may(vec![
                    crate::effect::Effect::sacrifice(creature_filter, 1),
                    crate::effect::Effect::with_id(
                        0,
                        crate::effect::Effect::new(crate::effects::CopySpellEffect::single(
                            crate::target::ChooseSpec::Source,
                        )),
                    ),
                    crate::effect::Effect::may_choose_new_targets_player(
                        crate::effect::EffectId(0),
                        crate::target::PlayerFilter::You,
                    ),
                ])],
            )
            .in_zones(vec![crate::zone::Zone::Stack]),
        )
    }

    pub fn variable_casualty_planeswalker_copy(self) -> Self {
        self.with_ability(
            crate::ability::Ability::triggered(
                crate::triggers::Trigger::you_cast_this_spell(),
                vec![crate::effect::Effect::new(
                    crate::effects::VariableCasualtyPlaneswalkerCopyEffect::new(),
                )],
            )
            .in_zones(vec![crate::zone::Zone::Stack]),
        )
    }

    pub fn conspire(mut self) -> Self {
        let existing_instances = self
            .optional_costs
            .iter()
            .filter(|cost| cost.label == "Conspire" || cost.label.starts_with("Conspire "))
            .count();
        let label = if existing_instances == 0 {
            "Conspire".to_string()
        } else {
            format!("Conspire {}", existing_instances + 1)
        };
        let cost = TotalCost::from_cost(crate::costs::Cost::effect(crate::effect::Effect::new(
            crate::effects::ConspireCostEffect::new(),
        )));
        self.optional_costs
            .push(OptionalCost::custom(label.clone(), cost));
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::you_cast_this_spell(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::with_id(
                        0,
                        crate::effect::Effect::new(crate::effects::CopySpellEffect::single(
                            crate::target::ChooseSpec::Source,
                        )),
                    ),
                    crate::effect::Effect::may_choose_new_targets_player(
                        crate::effect::EffectId(0),
                        crate::target::PlayerFilter::You,
                    ),
                ]),
                choices: vec![],
                intervening_if: Some(crate::ConditionExpr::ThisSpellPaidLabel(label)),
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Stack],
        })
    }

    pub fn ravenous(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::enters_with_counters_value(
                crate::object::CounterType::PlusOnePlusOne,
                crate::effect::Value::X,
            ),
        ))
        .with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_enters_battlefield(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::draw(1),
                ]),
                choices: vec![],
                intervening_if: Some(crate::ConditionExpr::XValueAtLeast(5)),
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn ascend(self) -> Self {
        let controls_ten = crate::ConditionExpr::PlayerControlsAtLeast {
            player: crate::target::PlayerFilter::You,
            filter: crate::target::ObjectFilter::permanent().you_control(),
            count: 10,
        };
        let not_blessed =
            crate::ConditionExpr::Not(Box::new(crate::ConditionExpr::PlayerHasCitysBlessing {
                player: crate::target::PlayerFilter::You,
            }));
        let bless_condition =
            crate::ConditionExpr::And(Box::new(controls_ten), Box::new(not_blessed));
        let get_blessing =
            crate::effect::Effect::create_emblem(crate::effect::EmblemDescription::new(
                "City's Blessing",
                "You have the city's blessing for the rest of the game.",
            ));

        let is_nonpermanent_spell = self
            .card_builder
            .card_types_ref()
            .iter()
            .any(|card_type| matches!(card_type, CardType::Instant | CardType::Sorcery));
        if is_nonpermanent_spell {
            let mut out = self;
            let mut effects = out.spell_effect.take().unwrap_or_default();
            effects.insert(
                0,
                crate::effect::Effect::conditional_only(bless_condition, vec![get_blessing]),
            );
            out.spell_effect = Some(effects);
            return out;
        }

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::enters_battlefield(
                    crate::target::ObjectFilter::permanent().you_control(),
                    None,
                ),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![get_blessing]),
                choices: vec![],
                intervening_if: Some(bless_condition),
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn daybound(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::daybound(),
        ))
    }

    pub fn nightbound(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::nightbound(),
        ))
    }

    pub fn haunt(self) -> Self {
        let trigger = if self
            .card_builder
            .card_types_ref()
            .contains(&CardType::Creature)
        {
            crate::triggers::Trigger::this_dies()
        } else {
            crate::triggers::Trigger::new(
                crate::triggers::ZoneChangeTrigger::new()
                    .from(crate::zone::Zone::Stack)
                    .to(crate::zone::Zone::Graveyard)
                    .this(),
            )
        };
        let functional_zones = if self
            .card_builder
            .card_types_ref()
            .contains(&CardType::Creature)
        {
            vec![crate::zone::Zone::Battlefield]
        } else {
            vec![crate::zone::Zone::Graveyard]
        };

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::exile(crate::target::ChooseSpec::Source),
                ]),
                choices: vec![crate::target::ChooseSpec::target(
                    crate::target::ChooseSpec::creature(),
                )],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones,
        })
    }

    pub fn provoke(self) -> Self {
        let target_spec = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
            crate::target::ObjectFilter::creature()
                .controlled_by(crate::target::PlayerFilter::Defending),
        ));
        let untap = crate::effect::Effect::untap(target_spec.clone());
        let must_block =
            crate::effect::Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                target_spec.clone(),
                crate::continuous::Modification::AddAbility(
                    crate::static_abilities::StaticAbility::must_block(),
                ),
                crate::effect::Until::EndOfCombat,
            ));
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_attacks(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    untap, must_block,
                ]),
                choices: vec![target_spec],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn undaunted(self) -> Self {
        let reduction = crate::static_abilities::CostReduction::new(
            crate::target::ObjectFilter::default(),
            crate::effect::Value::CountPlayers(crate::target::PlayerFilter::Opponent),
        );
        self.with_ability(
            crate::ability::Ability::static_ability(crate::static_abilities::StaticAbility::new(
                reduction,
            ))
            .in_zones(vec![crate::zone::Zone::Stack, crate::zone::Zone::Hand]),
        )
    }

    pub fn extort(self) -> Self {
        let pay_cost = ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::White,
            crate::mana::ManaSymbol::Black,
        ]]);
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::spell_cast(
                    None,
                    crate::target::PlayerFilter::You,
                ),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::with_id(
                        0,
                        crate::effect::Effect::may(vec![crate::effect::Effect::new(
                            crate::effects::PayManaEffect::new(
                                pay_cost,
                                crate::target::ChooseSpec::SourceController,
                            ),
                        )]),
                    ),
                    crate::effect::Effect::if_then(
                        crate::effect::EffectId(0),
                        crate::effect::EffectPredicate::Happened,
                        vec![
                            crate::effect::Effect::with_id(
                                1,
                                crate::effect::Effect::for_each_opponent(vec![
                                    crate::effect::Effect::lose_life_player(
                                        1,
                                        crate::target::PlayerFilter::IteratedPlayer,
                                    ),
                                ]),
                            ),
                            crate::effect::Effect::gain_life(crate::effect::Value::EffectValue(
                                crate::effect::EffectId(1),
                            )),
                        ],
                    ),
                ]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn partner(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::partner(),
        ))
    }

    pub fn assist(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::assist(),
        ))
    }

    pub fn split_second(self) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::split_second(),
            )
            .in_zones(vec![crate::zone::Zone::Stack]),
        )
    }

    pub fn cascade(self) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::cascade(),
            )
            .in_zones(vec![crate::zone::Zone::Stack]),
        )
    }

    pub fn rebound(self) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::rebound(),
            )
            .in_zones(vec![crate::zone::Zone::Stack]),
        )
    }

    pub fn read_ahead(self) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::read_ahead(),
        ))
    }

    pub fn sunburst(self) -> Self {
        let counter_type = if self
            .card_builder
            .card_types_ref()
            .contains(&CardType::Creature)
        {
            crate::object::CounterType::PlusOnePlusOne
        } else {
            crate::object::CounterType::Charge
        };

        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::keyword_marker("Sunburst"),
        ))
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::enters_with_counters_value(
                counter_type,
                crate::effect::Value::ColorsOfManaSpentToCastThisSpell,
            ),
        ))
    }

    pub fn for_mirrodin(self) -> Self {
        let created_tag = crate::tag::TagKey::from("for_mirrodin_created");
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_enters_battlefield(),
            vec![
                crate::effect::Effect::create_tokens(Self::for_mirrodin_rebel_token(), 1)
                    .tag(created_tag.clone()),
                crate::effect::Effect::attach_to(crate::target::ChooseSpec::Tagged(created_tag)),
            ],
        ))
    }

    pub fn living_weapon(self) -> Self {
        let created_tag = crate::tag::TagKey::from("living_weapon_created");
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_enters_battlefield(),
            vec![
                crate::effect::Effect::create_tokens(Self::living_weapon_germ_token(), 1)
                    .tag(created_tag.clone()),
                crate::effect::Effect::attach_to(crate::target::ChooseSpec::Tagged(created_tag)),
            ],
        ))
    }

    pub fn myriad(self) -> Self {
        let opponent_other_than_defending = crate::target::PlayerFilter::excluding(
            crate::target::PlayerFilter::Opponent,
            crate::target::PlayerFilter::Defending,
        );
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_attacks(),
            vec![crate::effect::Effect::for_players(
                opponent_other_than_defending,
                vec![crate::effect::Effect::may(vec![
                    crate::effect::Effect::new(
                        crate::effects::CreateTokenCopyEffect::new(
                            crate::target::ChooseSpec::Source,
                            1,
                            crate::target::PlayerFilter::You,
                        )
                        .enters_tapped(true)
                        .attacking_player_or_planeswalker_controlled_by(
                            crate::target::PlayerFilter::IteratedPlayer,
                        )
                        .exile_at_eoc(true),
                    ),
                ])],
            )],
        ))
    }

    pub fn mobilize(self, amount: u32) -> Self {
        let effect = crate::effects::CreateTokenEffect::new(
            Self::mobilize_warrior_token(),
            amount,
            crate::target::PlayerFilter::You,
        )
        .tapped()
        .attacking()
        .sacrifice_at_next_end_step();

        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_attacks(),
            vec![crate::effect::Effect::new(effect)],
        ))
    }

    pub fn fading(self, amount: u32) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::enters_with_counters_value(
                crate::object::CounterType::Fade,
                (amount as i32).into(),
            ),
        ))
        .with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::beginning_of_upkeep(crate::target::PlayerFilter::You),
            vec![crate::effect::Effect::remove_counters(
                crate::object::CounterType::Fade,
                1,
                crate::target::ChooseSpec::Source,
            )],
        ))
        .with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::new(
                    crate::triggers::CounterRemovedFromTrigger::new(
                        crate::target::ObjectFilter::source(),
                    ),
                ),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::sacrifice_source(),
                ]),
                choices: vec![],
                intervening_if: Some(crate::ConditionExpr::SourceHasNoCounter(
                    crate::object::CounterType::Fade,
                )),
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn vanishing(self, amount: u32) -> Self {
        let mut builder = self;
        if amount > 0 {
            builder = builder.with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::enters_with_counters_value(
                    crate::object::CounterType::Time,
                    amount.into(),
                ),
            ));
        }

        builder
            .with_ability(crate::ability::Ability::triggered(
                crate::triggers::Trigger::beginning_of_upkeep(crate::target::PlayerFilter::You),
                vec![crate::effect::Effect::remove_counters(
                    crate::object::CounterType::Time,
                    1,
                    crate::target::ChooseSpec::Source,
                )],
            ))
            .with_ability(crate::ability::Ability::triggered(
                crate::triggers::Trigger::custom(
                    "vanishing-last-time-counter-removed",
                    "when the last time counter is removed".to_string(),
                ),
                vec![crate::effect::Effect::sacrifice_source()],
            ))
    }

    pub fn modular(self, amount: u32) -> Self {
        let target = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
            crate::target::ObjectFilter::default()
                .with_all_type(CardType::Artifact)
                .with_all_type(CardType::Creature),
        ));
        let trigger_tag = crate::tag::TagKey::from("modular_triggering_object");
        let dead_source_filter = crate::target::ObjectFilter::default()
            .in_zone(crate::zone::Zone::Graveyard)
            .same_stable_id_as_tagged(trigger_tag.clone());
        let transfer_count = crate::effect::Value::CountersOn(
            Box::new(crate::target::ChooseSpec::All(dead_source_filter)),
            Some(crate::object::CounterType::PlusOnePlusOne),
        );

        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::enters_with_counters_value(
                crate::object::CounterType::PlusOnePlusOne,
                (amount as i32).into(),
            ),
        ))
        .with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_dies(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::tag_triggering_object(trigger_tag),
                    crate::effect::Effect::may(vec![crate::effect::Effect::put_counters(
                        crate::object::CounterType::PlusOnePlusOne,
                        transfer_count,
                        target.clone(),
                    )]),
                ]),
                choices: vec![target],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn modular_sunburst(self) -> Self {
        let target = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
            crate::target::ObjectFilter::default()
                .with_all_type(CardType::Artifact)
                .with_all_type(CardType::Creature),
        ));
        let trigger_tag = crate::tag::TagKey::from("modular_triggering_object");
        let dead_source_filter = crate::target::ObjectFilter::default()
            .in_zone(crate::zone::Zone::Graveyard)
            .same_stable_id_as_tagged(trigger_tag.clone());
        let transfer_count = crate::effect::Value::CountersOn(
            Box::new(crate::target::ChooseSpec::All(dead_source_filter)),
            Some(crate::object::CounterType::PlusOnePlusOne),
        );

        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::enters_with_counters_value(
                crate::object::CounterType::PlusOnePlusOne,
                crate::effect::Value::ColorsOfManaSpentToCastThisSpell,
            ),
        ))
        .with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_dies(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::tag_triggering_object(trigger_tag),
                    crate::effect::Effect::may(vec![crate::effect::Effect::put_counters(
                        crate::object::CounterType::PlusOnePlusOne,
                        transfer_count,
                        target.clone(),
                    )]),
                ]),
                choices: vec![target],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn graft(self, amount: u32) -> Self {
        let entered_tag = crate::tag::TagKey::from("graft_entered_creature");

        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::enters_with_counters_value(
                crate::object::CounterType::PlusOnePlusOne,
                (amount as i32).into(),
            ),
        ))
        .with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::enters_battlefield(
                    crate::target::ObjectFilter::creature().other(),
                    None,
                ),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::tag_triggering_object(entered_tag.clone()),
                    crate::effect::Effect::may(vec![crate::effect::Effect::new(
                        crate::effects::MoveCountersEffect::new(
                            crate::object::CounterType::PlusOnePlusOne,
                            1,
                            crate::target::ChooseSpec::Source,
                            crate::target::ChooseSpec::Tagged(entered_tag),
                        ),
                    )]),
                ]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn rampage(self, amount: u32) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::keyword_marker(format!("rampage {amount}")),
        ))
        .with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_becomes_blocked(),
            vec![crate::effect::Effect::pump(
                crate::effect::Value::EventValue(
                    crate::effect::EventValueSpec::BlockersBeyondFirst {
                        multiplier: amount as i32,
                    },
                ),
                crate::effect::Value::EventValue(
                    crate::effect::EventValueSpec::BlockersBeyondFirst {
                        multiplier: amount as i32,
                    },
                ),
                crate::target::ChooseSpec::Source,
                crate::effect::Until::EndOfTurn,
            )],
        ))
    }

    pub fn bushido(self, amount: u32) -> Self {
        self.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::either(
                crate::triggers::Trigger::this_blocks(),
                crate::triggers::Trigger::this_becomes_blocked(),
            ),
            vec![crate::effect::Effect::pump(
                amount as i32,
                amount as i32,
                crate::target::ChooseSpec::Source,
                crate::effect::Until::EndOfTurn,
            )],
        ))
    }

    pub fn protection_from(self, colors: ColorSet) -> Self {
        let protection = crate::static_abilities::StaticAbility::protection(
            crate::ability::ProtectionFrom::Color(colors),
        );
        self.with_ability(crate::ability::Ability::static_ability(protection))
    }

    pub fn protection_from_card_type(self, card_type: CardType) -> Self {
        let protection = crate::static_abilities::StaticAbility::protection(
            crate::ability::ProtectionFrom::CardType(card_type),
        );
        self.with_ability(crate::ability::Ability::static_ability(protection))
    }

    pub fn protection_from_filter(self, filter: ObjectFilter) -> Self {
        let protection = crate::static_abilities::StaticAbility::protection(
            crate::ability::ProtectionFrom::Permanents(filter),
        );
        self.with_ability(crate::ability::Ability::static_ability(protection))
    }

    pub fn protection_from_subtype(self, subtype: Subtype) -> Self {
        let protection = crate::static_abilities::StaticAbility::protection(
            crate::ability::ProtectionFrom::Permanents(
                crate::target::ObjectFilter::default().with_subtype(subtype),
            ),
        );
        self.with_ability(crate::ability::Ability::static_ability(protection))
    }

    pub fn devoid(self) -> Self {
        self.with_ability(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::make_colorless(
                    crate::target::ObjectFilter::source(),
                ),
            )
            .in_zones(vec![
                crate::zone::Zone::Battlefield,
                crate::zone::Zone::Stack,
                crate::zone::Zone::Hand,
                crate::zone::Zone::Library,
                crate::zone::Zone::Graveyard,
                crate::zone::Zone::Exile,
                crate::zone::Zone::Command,
            ]),
        )
    }

    pub fn annihilator(self, amount: u32) -> Self {
        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_attacks(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::effect::Effect::sacrifice_player(
                        crate::target::ObjectFilter::permanent(),
                        amount as i32,
                        crate::target::PlayerFilter::Defending,
                    ),
                ]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
        })
    }

    pub fn bloodthirst(self, amount: u32) -> Self {
        self.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::bloodthirst(amount),
        ))
    }

    pub fn undying(self) -> Self {
        let trigger_tag = "undying_trigger";
        let return_tag = "undying_return";
        let returned_tag = "undying_returned";

        let filter = crate::target::ObjectFilter::default()
            .in_zone(crate::zone::Zone::Graveyard)
            .same_stable_id_as_tagged(trigger_tag);

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_dies(),
                effects: vec![
                    crate::effect::Effect::tag_triggering_object(trigger_tag),
                    crate::effect::Effect::new(crate::effects::TagMatchingObjectsEffect::new(
                        filter, return_tag,
                    )),
                    crate::effect::Effect::new(
                        crate::effects::MoveToZoneEffect::new(
                            crate::target::ChooseSpec::Tagged(return_tag.into()),
                            crate::zone::Zone::Battlefield,
                            true,
                        )
                        .under_owner_control(),
                    )
                    .tag(returned_tag),
                    crate::effect::Effect::for_each_tagged(
                        returned_tag,
                        vec![crate::effect::Effect::put_counters(
                            crate::object::CounterType::PlusOnePlusOne,
                            1,
                            crate::target::ChooseSpec::Iterated,
                        )],
                    ),
                ]
                .into(),
                choices: vec![],
                intervening_if: Some(crate::ConditionExpr::Not(Box::new(
                    crate::ConditionExpr::TriggeringObjectHadCounters {
                        counter_type: crate::object::CounterType::PlusOnePlusOne,
                        min_count: 1,
                    },
                ))),
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield, crate::zone::Zone::Graveyard],
        })
    }

    pub fn persist(self) -> Self {
        let trigger_tag = "persist_trigger";
        let return_tag = "persist_return";
        let returned_tag = "persist_returned";

        let filter = crate::target::ObjectFilter::default()
            .in_zone(crate::zone::Zone::Graveyard)
            .same_stable_id_as_tagged(trigger_tag);

        self.with_ability(crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_dies(),
                effects: vec![
                    crate::effect::Effect::tag_triggering_object(trigger_tag),
                    crate::effect::Effect::new(crate::effects::TagMatchingObjectsEffect::new(
                        filter, return_tag,
                    )),
                    crate::effect::Effect::new(
                        crate::effects::MoveToZoneEffect::new(
                            crate::target::ChooseSpec::Tagged(return_tag.into()),
                            crate::zone::Zone::Battlefield,
                            true,
                        )
                        .under_owner_control(),
                    )
                    .tag(returned_tag),
                    crate::effect::Effect::for_each_tagged(
                        returned_tag,
                        vec![crate::effect::Effect::put_counters(
                            crate::object::CounterType::MinusOneMinusOne,
                            1,
                            crate::target::ChooseSpec::Iterated,
                        )],
                    ),
                ]
                .into(),
                choices: vec![],
                intervening_if: Some(crate::ConditionExpr::Not(Box::new(
                    crate::ConditionExpr::TriggeringObjectHadCounters {
                        counter_type: crate::object::CounterType::MinusOneMinusOne,
                        min_count: 1,
                    },
                ))),
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield, crate::zone::Zone::Graveyard],
        })
    }

    fn fabricate_servo_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Servo")
            .token()
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .subtypes(vec![Subtype::Servo])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    fn afterlife_spirit_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Spirit")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Spirit])
            .color_indicator(ColorSet::WHITE.union(ColorSet::BLACK))
            .power_toughness(PowerToughness::fixed(1, 1))
            .flying()
            .build()
    }

    fn for_mirrodin_rebel_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Rebel")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Rebel])
            .color_indicator(ColorSet::RED)
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn living_weapon_germ_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Phyrexian Germ")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Phyrexian, Subtype::Germ])
            .color_indicator(ColorSet::BLACK)
            .power_toughness(PowerToughness::fixed(0, 0))
            .build()
    }

    fn mobilize_warrior_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Warrior")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Warrior])
            .color_indicator(ColorSet::RED)
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    pub fn build(self) -> CardDefinition {
        CardDefinition {
            card: self.card_builder.build(),
            abilities: self.abilities,
            spell_effect: self.spell_effect,
            aura_attach_filter: self.aura_attach_filter,
            alternative_casts: self.alternative_casts,
            has_fuse: self.has_fuse,
            optional_costs: self.optional_costs,
            additional_cost: self.additional_cost,
        }
    }
}

pub const IT_TAG: &str = crate::host::IT_TAG;
pub const COPIED_STACK_OBJECT_TAG: &str = crate::host::COPIED_STACK_OBJECT_TAG;

fn parse_standalone_bolster_marker(text: &str) -> Option<u32> {
    let mut parts = text.split_whitespace();
    matches!(parts.next(), Some(keyword) if keyword.eq_ignore_ascii_case("bolster"))
        .then(|| parts.next().and_then(|amount| amount.parse::<u32>().ok()))
        .flatten()
        .filter(|_| parts.next().is_none())
}

fn parse_prototype_marker_cost(text: &str) -> Option<ManaCost> {
    let trimmed = text.trim();
    let rest = trimmed
        .strip_prefix("Prototype")
        .or_else(|| trimmed.strip_prefix("prototype"))?
        .trim_start();
    let cost_text = rest
        .split(|ch| matches!(ch, '-' | '—' | '–'))
        .next()?
        .trim();
    if cost_text.is_empty() {
        return None;
    }
    crate::runtime_backend::parse_scryfall_mana_cost(&cost_text.to_ascii_uppercase()).ok()
}
