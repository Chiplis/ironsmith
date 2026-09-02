//! Keyword actions lowered into card abilities.
//!
//! A keyword line names an ability rather than spelling it out. Expanding the
//! name into the ability — including the costs and static abilities some
//! keywords carry — is lowering work, so it lives outside the builder that
//! merely collects the result.

use crate::KeywordAction;
use crate::cards::builders::CardDefinitionBuilder;
use crate::static_abilities::StaticAbility;
use crate::types::CardType;

/// Fold a keyword action into a card definition.
///
/// A keyword names an ability the card has; turning that name into the ability
/// itself is lowering, which is why this lives here rather than on the builder.
pub fn apply_keyword_action(
    mut builder: CardDefinitionBuilder,
    action: KeywordAction,
) -> CardDefinitionBuilder {
    match action {
        KeywordAction::Flying => builder.flying(),
        KeywordAction::Banding => builder.with_ability(crate::ability::Ability::static_ability(
            StaticAbility::banding(),
        )),
        KeywordAction::Defender => builder.defender(),
        KeywordAction::Decayed => builder.decayed(),
        KeywordAction::Vigilance => builder.vigilance(),
        KeywordAction::Prowess => builder.prowess(),
        KeywordAction::Trample => builder.trample(),
        KeywordAction::Lifelink => builder.lifelink(),
        KeywordAction::Deathtouch => builder.deathtouch(),
        KeywordAction::Haste => builder.haste(),
        KeywordAction::Menace => builder.menace(),
        KeywordAction::Reach => builder.reach(),
        KeywordAction::Hexproof => builder.hexproof(),
        KeywordAction::Indestructible => builder.indestructible(),
        KeywordAction::Toxic(amount) => builder.toxic(amount),
        KeywordAction::Poisonous(amount) => builder.poisonous(amount),
        KeywordAction::Afterlife(amount) => builder.afterlife(amount),
        KeywordAction::Fabricate(amount) => builder.fabricate(amount),
        KeywordAction::FirstStrike => builder.first_strike(),
        KeywordAction::DoubleStrike => builder.double_strike(),
        KeywordAction::Exalted => builder.exalted(),
        KeywordAction::Storm => builder.storm(),
        KeywordAction::Gravestorm => builder.gravestorm(),
        KeywordAction::BattleCry => builder.battle_cry(),
        KeywordAction::Dethrone => builder.dethrone(),
        KeywordAction::Evolve => builder.evolve(),
        KeywordAction::Ingest => builder.ingest(),
        KeywordAction::Mentor => builder.mentor(),
        KeywordAction::Training => builder.training(),
        KeywordAction::Riot => builder.riot(),
        KeywordAction::Soulbond => builder.soulbond(),
        KeywordAction::Soulshift(amount) => builder.soulshift(amount),
        KeywordAction::SoulshiftValue(value) => builder.soulshift_value(value),
        KeywordAction::Recover(cost) => builder.recover(cost),
        KeywordAction::Outlast(cost) => builder.outlast(cost),
        KeywordAction::Scavenge(cost) => builder.scavenge(cost),
        KeywordAction::Unearth(cost) => builder.unearth(cost),
        KeywordAction::Embalm(cost) => builder.embalm(cost),
        KeywordAction::Eternalize(cost) => builder.eternalize(
            crate::lowering::cost_materialization::materialize_compiler_core_total_cost(&cost)
                .expect("compiler-owned eternalize cost must materialize"),
        ),
        KeywordAction::Emerge(cost) => builder.emerge(cost),
        KeywordAction::Vanishing(amount) => builder.vanishing(amount),
        KeywordAction::Bloodthirst(amount) => builder.bloodthirst(amount),
        KeywordAction::Ninjutsu(cost) => builder.ninjutsu(cost),
        KeywordAction::Backup(amount) => builder.backup(amount),
        KeywordAction::Dash(cost) => builder.dash(cost),
        KeywordAction::Blitz(cost) => builder.blitz(cost),
        KeywordAction::BlitzFromGraveyard => {
            builder.with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::keyword_marker(
                    KeywordAction::BlitzFromGraveyard.display_text(),
                ),
            ))
        }
        KeywordAction::Warp(cost) => builder.warp(cost),
        KeywordAction::Plot(cost) => builder.plot(cost),
        KeywordAction::Disturb(cost) => builder.disturb(cost),
        KeywordAction::Spectacle(cost) => builder.spectacle(cost),
        KeywordAction::Foretell(cost) => builder.foretell(cost),
        KeywordAction::Unleash => builder.unleash(),
        KeywordAction::Ward(amount) => builder.ward_generic(amount),
        KeywordAction::Afflict(amount) => builder.afflict(amount),
        KeywordAction::Undying => builder.undying(),
        KeywordAction::Persist => builder.persist(),
        KeywordAction::Renown(amount) => builder.renown(amount),
        KeywordAction::Myriad => builder.myriad(),
        KeywordAction::Mobilize(amount) => builder.mobilize(amount),
        KeywordAction::Cipher => builder.cipher(),
        KeywordAction::Suspend { time, cost } => builder.suspend(time, cost),
        KeywordAction::Overload(cost) => builder.overload(cost),
        KeywordAction::Cleave(cost) => builder.cleave(cost),
        KeywordAction::Awaken { amount, cost } => builder.awaken(amount, cost),
        KeywordAction::Echo { total_cost, .. } => builder.echo(
            crate::lowering::cost_materialization::materialize_compiler_core_total_cost(
                &total_cost,
            )
            .expect("compiler-owned echo cost must materialize"),
        ),
        KeywordAction::CumulativeUpkeep { total_cost, .. } => builder.cumulative_upkeep(
            crate::lowering::cost_materialization::materialize_compiler_core_total_cost(
                &total_cost,
            )
            .expect("compiler-owned cumulative upkeep cost must materialize"),
        ),
        KeywordAction::Casualty(amount) => builder.casualty(amount),
        KeywordAction::VariableCasualtyPlaneswalkerCopy => {
            builder.variable_casualty_planeswalker_copy()
        }
        KeywordAction::Demonstrate => builder.demonstrate(),
        KeywordAction::Conspire => builder.conspire(),
        KeywordAction::Amplify(amount) => builder.amplify(amount),
        KeywordAction::Devour(multiplier) => builder.devour(multiplier),
        KeywordAction::AuraSwap(cost) => builder.aura_swap(cost),
        KeywordAction::Ravenous => builder.ravenous(),
        KeywordAction::Ascend => builder.ascend(),
        KeywordAction::Daybound => builder.daybound(),
        KeywordAction::Nightbound => builder.nightbound(),
        KeywordAction::Haunt => builder.haunt(),
        KeywordAction::Provoke => builder.provoke(),
        KeywordAction::Enlist => builder.enlist(),
        KeywordAction::Crew {
            amount,
            timing,
            once_per_turn,
        } => builder.crew(
            amount,
            timing,
            once_per_turn
                .then(|| "Activate only once each turn.".to_string())
                .into_iter()
                .collect(),
        ),
        KeywordAction::Undaunted => builder.undaunted(),
        KeywordAction::Extort => builder.extort(),
        KeywordAction::Partner => builder.partner(),
        KeywordAction::StartYourEngines => {
            builder.with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::start_your_engines(),
            ))
        }
        KeywordAction::Assist => builder.assist(),
        KeywordAction::SplitSecond => builder.split_second(),
        KeywordAction::Cascade => builder.cascade(),
        KeywordAction::Rebound => builder.rebound(),
        KeywordAction::Sunburst => builder.sunburst(),
        KeywordAction::ReadAhead => builder.read_ahead(),
        KeywordAction::Firebending(amount) => builder.firebending(amount),
        KeywordAction::FirebendingValue { amount, surface } => {
            builder.firebending_with_surface(amount, surface)
        }
        KeywordAction::Fading(amount) => builder.fading(amount),
        KeywordAction::Modular(amount) => builder.modular(amount),
        KeywordAction::ModularSunburst => builder.modular_sunburst(),
        KeywordAction::Graft(amount) => builder.graft(amount),
        KeywordAction::Rampage(amount) => builder.rampage(amount),
        KeywordAction::Bushido(amount) => builder.bushido(amount),
        KeywordAction::Frenzy(amount) => builder.frenzy(amount),
        KeywordAction::ProtectionFrom(colors) => builder.protection_from(colors),
        KeywordAction::ProtectionFromAllColors => {
            builder.with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::protection(
                    crate::ability::ProtectionFrom::AllColors,
                ),
            ))
        }
        KeywordAction::ProtectionFromColorless => {
            builder.with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::protection(
                    crate::ability::ProtectionFrom::Colorless,
                ),
            ))
        }
        KeywordAction::ProtectionFromEverything => {
            builder.with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::protection(
                    crate::ability::ProtectionFrom::Everything,
                ),
            ))
        }
        KeywordAction::ProtectionFromChosenPlayer => {
            builder.with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::protection(
                    crate::ability::ProtectionFrom::ChosenPlayer,
                ),
            ))
        }
        KeywordAction::ProtectionFromChosenColor => {
            builder.with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::protection(
                    crate::ability::ProtectionFrom::ChosenColor,
                ),
            ))
        }
        KeywordAction::ProtectionFromFilter(filter) => builder.protection_from_filter(filter),
        KeywordAction::ProtectionFromEachManaValueAmong(filter) => {
            builder.with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::protection(
                    crate::ability::ProtectionFrom::EachManaValueAmong(filter),
                ),
            ))
        }
        KeywordAction::ProtectionFromCardType(card_type) => {
            builder.protection_from_card_type(card_type)
        }
        KeywordAction::ProtectionFromSubtype(subtype) => builder.protection_from_subtype(subtype),
        KeywordAction::Devoid => builder.devoid(),
        KeywordAction::Annihilator(amount) => builder.annihilator(amount),
        KeywordAction::ForMirrodin => builder.for_mirrodin(),
        KeywordAction::LivingWeapon => builder.living_weapon(),
        KeywordAction::Fuse => builder.has_fuse(),
        KeywordAction::Prototype {
            cost,
            power_toughness,
        } => builder.alternative_cast(
            crate::alternative_cast::AlternativeCastingMethod::prototype(cost, power_toughness),
        ),
        KeywordAction::Bolster(amount)
            if builder
                .card_builder
                .card_types_ref()
                .iter()
                .any(|card_type| matches!(card_type, CardType::Instant | CardType::Sorcery)) =>
        {
            let effect = crate::effect::Effect::bolster(amount);
            if let Some(existing) = &mut builder.spell_effect {
                existing.push(effect);
            } else {
                builder.spell_effect =
                    Some(crate::resolution::ResolutionProgram::from_effects(vec![
                        effect,
                    ]));
            }
            builder
        }
        KeywordAction::Bolster(amount) => {
            builder.with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::keyword_marker(format!("Bolster {amount}")),
            ))
        }
        other if other.lowers_to_static_ability() => {
            let text = other.display_text();
            let static_ability =
                crate::lowering_support::runtime_static_ability_for_keyword_action(other)
                    .unwrap_or_else(|| {
                        crate::static_abilities::StaticAbility::keyword_marker(text.clone())
                    });
            builder.with_ability(crate::ability::Ability::static_ability(static_ability))
        }
        other => builder.with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::custom("compiler-keyword", other.display_text()),
            crate::resolution::ResolutionProgram::default(),
        )),
    }
}
