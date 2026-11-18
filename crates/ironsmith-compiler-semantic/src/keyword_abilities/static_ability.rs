use std::fmt::Debug;

use ironsmith_core::{ConditionConjunction, CostComponent, StaticAbility, ThisSpellCostCondition};

use crate::ability::ProtectionFrom;
use crate::effect::Value;
use crate::filter::{ObjectFilter, PlayerFilter};
use crate::mana::{ManaCost, ManaSymbol};
use crate::model::CompilerStaticAbilityCore;
use crate::object::CounterType;
use crate::payload::KeywordAction;

pub fn static_ability_for_keyword_action(
    action: KeywordAction,
) -> Option<CompilerStaticAbilityCore> {
    keyword_static_ability(action)
}

/// A static keyword has the same payload in the recognized and lowered models.
/// Keywords with triggered or activated programs expand through their own builders.
pub fn keyword_static_ability<T, E, C, ICond>(
    action: KeywordAction,
) -> Option<StaticAbility<T, E, C, ThisSpellCostCondition, ICond>>
where
    T: Clone + PartialEq + Debug + 'static,
    E: Clone + PartialEq + Debug + 'static,
    C: CostComponent + 'static,
    ICond: Clone + PartialEq + Debug + ConditionConjunction + 'static,
{
    Some(match action {
        KeywordAction::Flying => StaticAbility::flying(),
        KeywordAction::Menace => StaticAbility::menace(),
        KeywordAction::Banding => StaticAbility::banding(),
        KeywordAction::Hexproof => StaticAbility::hexproof(),
        KeywordAction::HexproofFrom(filter) => StaticAbility::hexproof_from(filter),
        KeywordAction::Haste => StaticAbility::haste(),
        KeywordAction::Improvise => StaticAbility::improvise(),
        KeywordAction::Convoke => StaticAbility::convoke(),
        KeywordAction::AffinityForArtifacts => StaticAbility::affinity_for_artifacts(),
        KeywordAction::CantBeCountered => StaticAbility::cant_be_countered_ability(),
        KeywordAction::Delve => StaticAbility::delve(),
        KeywordAction::FirstStrike => StaticAbility::first_strike(),
        KeywordAction::DoubleStrike => StaticAbility::double_strike(),
        KeywordAction::Deathtouch => StaticAbility::deathtouch(),
        KeywordAction::Lifelink => StaticAbility::lifelink(),
        KeywordAction::Vigilance => StaticAbility::vigilance(),
        KeywordAction::Trample => StaticAbility::trample(),
        KeywordAction::TrampleOverPlaneswalkers => StaticAbility::trample_over_planeswalkers(),
        KeywordAction::Reach => StaticAbility::reach(),
        KeywordAction::Defender => StaticAbility::defender(),
        KeywordAction::Decayed => StaticAbility::cant_block(),
        KeywordAction::Flash => StaticAbility::flash(),
        KeywordAction::Phasing => StaticAbility::phasing(),
        KeywordAction::Indestructible => StaticAbility::indestructible(),
        KeywordAction::Shroud => StaticAbility::shroud(),
        KeywordAction::Daybound => StaticAbility::daybound(),
        KeywordAction::Nightbound => StaticAbility::nightbound(),
        KeywordAction::Ward(amount) => {
            let generic = u8::try_from(amount).ok()?;
            StaticAbility::ward(ManaCost::from_symbols(vec![ManaSymbol::Generic(generic)]))
        }
        KeywordAction::Wither => StaticAbility::wither(),
        KeywordAction::Infect => StaticAbility::infect(),
        KeywordAction::Cascade => StaticAbility::cascade(),
        KeywordAction::Toxic(amount) => StaticAbility::toxic(amount),
        KeywordAction::Skulk => StaticAbility::skulk(),
        KeywordAction::Unleash => StaticAbility::unleash(),
        KeywordAction::Partner => StaticAbility::partner(),
        KeywordAction::StartYourEngines => StaticAbility::start_your_engines(),
        KeywordAction::Assist => StaticAbility::assist(),
        KeywordAction::SplitSecond => StaticAbility::split_second(),
        KeywordAction::Rebound => StaticAbility::rebound(),
        KeywordAction::ReadAhead => StaticAbility::read_ahead(),
        KeywordAction::Fear => StaticAbility::fear(),
        KeywordAction::Intimidate => StaticAbility::intimidate(),
        KeywordAction::Shadow => StaticAbility::shadow(),
        KeywordAction::Horsemanship => StaticAbility::horsemanship(),
        KeywordAction::Flanking => StaticAbility::flanking(),
        KeywordAction::UmbraArmor => StaticAbility::umbra_armor(),
        KeywordAction::Landwalk(kind) => StaticAbility::landwalk_for_kind(kind),
        KeywordAction::Bloodthirst(amount) => StaticAbility::bloodthirst(amount),
        KeywordAction::BloodthirstX => StaticAbility::enters_with_counters_value(
            CounterType::PlusOnePlusOne,
            Value::DamageDealtToPlayersThisTurn(PlayerFilter::Opponent),
        ),
        KeywordAction::Tribute(amount) => StaticAbility::tribute(amount),
        KeywordAction::Changeling => StaticAbility::changeling(),
        KeywordAction::ProtectionFrom(colors) => {
            StaticAbility::protection(ProtectionFrom::Color(colors))
        }
        KeywordAction::ProtectionFromOwnColors => {
            StaticAbility::protection(ProtectionFrom::OwnColors)
        }
        KeywordAction::ProtectionFromColorsAmong(filter) => {
            StaticAbility::protection(ProtectionFrom::ColorsAmong {
                filter,
                reference_source: None,
            })
        }
        KeywordAction::ProtectionFromAllColors => {
            StaticAbility::protection(ProtectionFrom::AllColors)
        }
        KeywordAction::ProtectionFromColorless => {
            StaticAbility::protection(ProtectionFrom::Colorless)
        }
        KeywordAction::ProtectionFromEverything => {
            StaticAbility::protection(ProtectionFrom::Everything)
        }
        KeywordAction::ProtectionFromChosenPlayer => {
            StaticAbility::protection(ProtectionFrom::ChosenPlayer)
        }
        KeywordAction::ProtectionFromChosenColor => {
            StaticAbility::protection(ProtectionFrom::ChosenColor)
        }
        KeywordAction::ProtectionFromColorsOutsideCommanderIdentity => {
            StaticAbility::protection(ProtectionFrom::ColorsOutsideCommanderIdentity)
        }
        KeywordAction::ProtectionFromManaValuesOtherThanChosenNumber => {
            StaticAbility::protection(ProtectionFrom::ManaValuesOtherThanChosenNumber)
        }
        KeywordAction::ProtectionFromFilter(filter) => {
            StaticAbility::protection(ProtectionFrom::Permanents(filter))
        }
        KeywordAction::ProtectionFromEachManaValueAmong(filter) => {
            StaticAbility::protection(ProtectionFrom::EachManaValueAmong(filter))
        }
        KeywordAction::ProtectionFromCardType(card_type) => {
            StaticAbility::protection(ProtectionFrom::CardType(card_type))
        }
        KeywordAction::ProtectionFromSubtype(subtype) => StaticAbility::protection(
            ProtectionFrom::Permanents(ObjectFilter::default().with_subtype(subtype)),
        ),
        KeywordAction::Unblockable => StaticAbility::unblockable(),
        KeywordAction::CantBeBlockedByMoreThan(count) => {
            StaticAbility::cant_be_blocked_by_more_than(count as usize)
        }
        KeywordAction::Devoid => StaticAbility::make_colorless(ObjectFilter::source()),
        KeywordAction::Dredge(amount) => StaticAbility::dredge(amount),
        KeywordAction::StaticMarker(name) => StaticAbility::keyword_marker(name),
        KeywordAction::StaticMarkerText(text) => StaticAbility::keyword_marker(text),
        KeywordAction::Marker(name) => StaticAbility::keyword_fallback_text(name),
        KeywordAction::MarkerText(text) => StaticAbility::keyword_fallback_text(text),
        _ => return None,
    })
}
