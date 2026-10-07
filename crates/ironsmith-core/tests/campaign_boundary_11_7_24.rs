//! Source-authored compatibility cases, UNRUN.
//! Observe Serde's actual outer variant calls and roundtrip native named JSON.
//! Recording an index does not promise support for any binary codec.
#![cfg(feature = "serde")]

use ironsmith_core::trigger_model::PlayerAttackGrouping;
use ironsmith_core::{
    ActivatedAbilityCostCondition as CostCondition, ActivatedAbilityKeyword as Keyword,
    CombatParticipantCondition, Condition, DelayedTriggerSpec, ObjectFilter, PlayerFilter,
    TriggerKind,
};
use serde::de::DeserializeOwned;
use serde::ser::{Error as _, Impossible, SerializeStructVariant, SerializeTupleVariant};
use serde::{Serialize, Serializer};
use serde_json::{Value, json};
use std::fmt::Debug;

type Error = serde_json::Error;
type Observation = (&'static str, u32, &'static str);

struct OuterVariantSerializer;
struct OuterVariant(Observation);

impl SerializeTupleVariant for OuterVariant {
    type Ok = Observation;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, _value: &T) -> Result<(), Error> {
        Ok(())
    }

    fn end(self) -> Result<Observation, Error> {
        Ok(self.0)
    }
}

impl SerializeStructVariant for OuterVariant {
    type Ok = Observation;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _key: &'static str,
        _value: &T,
    ) -> Result<(), Error> {
        Ok(())
    }

    fn end(self) -> Result<Observation, Error> {
        Ok(self.0)
    }
}

macro_rules! reject_scalars {
    ($($method:ident($ty:ty)),* $(,)?) => {
        $(fn $method(self, _value: $ty) -> Result<Observation, Error> {
            Err(Error::custom("expected an outer enum variant"))
        })*
    };
}

impl Serializer for OuterVariantSerializer {
    type Ok = Observation;
    type Error = Error;
    type SerializeSeq = Impossible<Observation, Error>;
    type SerializeTuple = Impossible<Observation, Error>;
    type SerializeTupleStruct = Impossible<Observation, Error>;
    type SerializeTupleVariant = OuterVariant;
    type SerializeMap = Impossible<Observation, Error>;
    type SerializeStruct = Impossible<Observation, Error>;
    type SerializeStructVariant = OuterVariant;

    reject_scalars! {
        serialize_bool(bool), serialize_i8(i8), serialize_i16(i16),
        serialize_i32(i32), serialize_i64(i64), serialize_i128(i128),
        serialize_u8(u8), serialize_u16(u16), serialize_u32(u32),
        serialize_u64(u64), serialize_u128(u128), serialize_f32(f32),
        serialize_f64(f64), serialize_char(char), serialize_str(&str),
        serialize_bytes(&[u8]),
    }

    fn serialize_none(self) -> Result<Observation, Error> {
        Err(Error::custom("expected an outer enum variant"))
    }

    fn serialize_some<T: ?Sized + Serialize>(self, _value: &T) -> Result<Observation, Error> {
        Err(Error::custom("expected an outer enum variant"))
    }

    fn serialize_unit(self) -> Result<Observation, Error> {
        Err(Error::custom("expected an outer enum variant"))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Observation, Error> {
        Err(Error::custom("expected an outer enum variant"))
    }

    fn serialize_unit_variant(
        self, name: &'static str, index: u32, variant: &'static str,
    ) -> Result<Observation, Error> {
        Ok((name, index, variant))
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self, _name: &'static str, _value: &T,
    ) -> Result<Observation, Error> {
        Err(Error::custom("expected an outer enum variant"))
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self, name: &'static str, index: u32, variant: &'static str, _value: &T,
    ) -> Result<Observation, Error> {
        Ok((name, index, variant))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        Err(Error::custom("expected an outer enum variant"))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Error> {
        Err(Error::custom("expected an outer enum variant"))
    }

    fn serialize_tuple_struct(
        self, _name: &'static str, _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        Err(Error::custom("expected an outer enum variant"))
    }

    fn serialize_tuple_variant(
        self, name: &'static str, index: u32, variant: &'static str, _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        Ok(OuterVariant((name, index, variant)))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Error> {
        Err(Error::custom("expected an outer enum variant"))
    }

    fn serialize_struct(
        self, _name: &'static str, _len: usize,
    ) -> Result<Self::SerializeStruct, Error> {
        Err(Error::custom("expected an outer enum variant"))
    }

    fn serialize_struct_variant(
        self, name: &'static str, index: u32, variant: &'static str, _len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        Ok(OuterVariant((name, index, variant)))
    }
}

fn assert_case<T>(value: T, expected: Observation, named_json: Value)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    assert_eq!(value.serialize(OuterVariantSerializer).unwrap(), expected);
    let bytes = serde_json::to_vec(&value).unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), named_json);
    let restored: T = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(restored, value);
    assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
}

#[test]
fn activated_keywords_preserve_class_position_before_appended_cost_keywords() {
    for (value, index, name, named_json) in [
        (Keyword::Equip, 0, "Equip", json!("Equip")),
        (Keyword::PowerUp, 1, "PowerUp", json!("PowerUp")),
        (Keyword::ClassLevel(3), 2, "ClassLevel", json!({"ClassLevel": 3})),
        (Keyword::Cycling, 3, "Cycling", json!("Cycling")),
        (Keyword::Ninjutsu, 4, "Ninjutsu", json!("Ninjutsu")),
        (Keyword::Boast, 5, "Boast", json!("Boast")),
        (Keyword::Exhaust, 6, "Exhaust", json!("Exhaust")),
    ] {
        assert_case(value, ("ActivatedAbilityKeyword", index, name), named_json);
    }
    assert_case(
        Keyword::ClassLevel(u32::MAX),
        ("ActivatedAbilityKeyword", 2, "ClassLevel"),
        json!({"ClassLevel": u32::MAX}),
    );
}

#[test]
fn activated_cost_selectors_preserve_published_positions_and_named_payloads() {
    let filter = ObjectFilter::creature().you_control();
    for (value, index, name, named_json) in [
        (CostCondition::TargetsExactly { count: 2, filter: filter.clone() }, 0,
            "TargetsExactly", json!({"TargetsExactly": {"count": 2, "filter": filter}})),
        (CostCondition::EquipAbility { targeting: None }, 1,
            "EquipAbility", json!({"EquipAbility": {"targeting": null}})),
        (CostCondition::ThisAbility { ability_index: Some(3) }, 2,
            "ThisAbility", json!({"ThisAbility": {"ability_index": 3}})),
        (CostCondition::All(vec![CostCondition::ThisAbility { ability_index: None }]), 3,
            "All", json!({"All": [{"ThisAbility": {"ability_index": null}}]})),
        (CostCondition::Keyword(Keyword::Cycling), 4,
            "Keyword", json!({"Keyword": "Cycling"})),
        (CostCondition::NonManaAbility, 5, "NonManaAbility", json!("NonManaAbility")),
        (CostCondition::LoyaltyAbility, 6, "LoyaltyAbility", json!("LoyaltyAbility")),
        (CostCondition::Activator(PlayerFilter::Opponent), 7,
            "Activator", json!({"Activator": "Opponent"})),
    ] {
        assert_case(value, ("ActivatedAbilityCostCondition", index, name), named_json);
    }
}

#[test]
fn combat_conditions_append_after_the_published_activation_history_condition() {
    assert_case(
        Condition::YouControl(ObjectFilter::creature()),
        ("Condition", 0, "YouControl"),
        json!({"YouControl": ObjectFilter::creature()}),
    );
    assert_case(
        Condition::ThisAbilityActivatedThisTurnAtLeast(2),
        ("Condition", 201, "ThisAbilityActivatedThisTurnAtLeast"),
        json!({"ThisAbilityActivatedThisTurnAtLeast": 2}),
    );
    for (condition, name) in [
        (CombatParticipantCondition::YouAreDefendingPlayer, "YouAreDefendingPlayer"),
        (CombatParticipantCondition::AttackingPlayerAttackedYouOrYourPlaneswalker,
            "AttackingPlayerAttackedYouOrYourPlaneswalker"),
        (CombatParticipantCondition::AttackingPlayerIsNotAttackingYou,
            "AttackingPlayerIsNotAttackingYou"),
        (CombatParticipantCondition::AnyAttackedPlayerIsPoisoned, "AnyAttackedPlayerIsPoisoned"),
        (CombatParticipantCondition::TriggeringCreatureAttacksMostLifePlayer,
            "TriggeringCreatureAttacksMostLifePlayer"),
    ] {
        assert_case(
            Condition::CombatParticipant(condition),
            ("Condition", 202, "CombatParticipant"),
            json!({"CombatParticipant": name}),
        );
    }
}

#[test]
fn attack_groupings_keep_direct_player_positions_before_any_target() {
    for (value, index, name) in [
        (PlayerAttackGrouping::Attacker, 0, "Attacker"),
        (PlayerAttackGrouping::Defender, 1, "Defender"),
        (PlayerAttackGrouping::Pair, 2, "Pair"),
        (PlayerAttackGrouping::AttackerAnyTarget, 3, "AttackerAnyTarget"),
    ] {
        assert_case(value, ("PlayerAttackGrouping", index, name), json!(name));
    }
}

fn assert_defaulted_controller_group<T>(ungrouped: T, grouped: T, variant: &str)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let ungrouped_json = serde_json::to_value(&ungrouped).unwrap();
    let mut legacy_json = ungrouped_json.clone();
    assert_eq!(
        legacy_json[variant].as_object_mut().unwrap().remove("per_source_controller"),
        Some(json!(false)),
    );
    assert_eq!(serde_json::from_value::<T>(legacy_json).unwrap(), ungrouped);
    for invalid in [Value::Null, json!("false"), json!(0)] {
        let mut invalid_json = ungrouped_json.clone();
        invalid_json[variant]["per_source_controller"] = invalid;
        assert!(serde_json::from_value::<T>(invalid_json).is_err(),
            "invalid retained controller cardinality is not absent evidence");
    }
    let bytes = serde_json::to_vec(&grouped).unwrap();
    let grouped_json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(grouped_json[variant]["per_source_controller"], true);
    assert_ne!(ungrouped_json, grouped_json);
    let restored: T = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(restored, grouped);
    assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
}

#[test]
fn combat_damage_controller_group_defaults_false_in_direct_and_delayed_named_models() {
    let source = ObjectFilter::creature().controlled_by(PlayerFilter::Opponent);
    for each_damaged_player in [false, true] {
        let direct = |per_source_controller| TriggerKind::DealsCombatDamageToPlayer {
            source: source.clone(),
            player: PlayerFilter::You,
            one_or_more: true,
            each_damaged_player,
            per_source_controller,
        };
        assert_defaulted_controller_group(
            direct(false), direct(true), "DealsCombatDamageToPlayer",
        );
        let delayed = |per_source_controller| DelayedTriggerSpec::DealsCombatDamageToPlayerOneOrMore {
            source: source.clone(),
            player: PlayerFilter::You,
            each_damaged_player,
            per_source_controller,
        };
        assert_defaulted_controller_group(
            delayed(false), delayed(true), "DealsCombatDamageToPlayerOneOrMore",
        );
    }
}

#[test]
fn class_grant_scope_omits_none_defaults_absent_and_retains_explicit_levels() {
    type Grant = ironsmith_core::GrantSpec<
        (), (), ironsmith_core::Cost<()>, ironsmith_core::ThisSpellCostCondition,
    >;
    let baseline = Grant::new(
        ironsmith_core::Grantable::PlayFrom,
        ObjectFilter::default(),
        ironsmith_core::Zone::Exile,
    );
    let baseline_json = serde_json::to_value(&baseline).unwrap();
    assert!(!baseline_json.as_object().unwrap().contains_key("linked_exile_class_level"));
    let restored: Grant = serde_json::from_value(baseline_json.clone()).unwrap();
    assert_eq!(restored.linked_exile_class_level, None);
    assert_eq!(restored, baseline);
    assert_eq!(serde_json::to_value(restored).unwrap(), baseline_json);

    for level in [2, 3] {
        let mut scoped = baseline.clone();
        scoped.linked_exile_class_level = Some(level);
        let bytes = serde_json::to_vec(&scoped).unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["linked_exile_class_level"], level);
        assert_ne!(json, baseline_json);
        let restored: Grant = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(restored, scoped);
        assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
    }
}

#[test]
fn absent_activation_keyword_keeps_legacy_none_and_class_keyword_roundtrips() {
    type Model = ironsmith_core::Ability<(), (), (), ironsmith_core::Cost<()>>;
    type Activated = ironsmith_core::ActivatedAbility<(), ironsmith_core::Cost<()>>;
    let ironsmith_core::AbilityKind::Activated(mut activated) =
        Model::activated(ironsmith_core::TotalCost::free(), vec![]).kind
    else { unreachable!() };
    let legacy = serde_json::to_value(&activated).unwrap();
    assert!(!legacy.as_object().unwrap().contains_key("keyword"));
    let restored: Activated = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(restored.keyword, None);
    assert_eq!(restored, activated);
    assert_eq!(serde_json::to_value(restored).unwrap(), legacy);

    activated.keyword = Some(Keyword::ClassLevel(3));
    let bytes = serde_json::to_vec(&activated).unwrap();
    let named: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(named["keyword"], json!({"ClassLevel": 3}));
    let restored: Activated = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(restored, activated);
    assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
}

#[test]
fn successor_variants_append_after_original_enum_tails_without_reordering() {
    use ironsmith_core::{AnthemCountExpression, ContinuousDurationObject, CounterType,
        EffectPredicate, ObjectCharacteristic, Until};
    assert_case(EffectPredicate::Succeeded, ("EffectPredicate", 0, "Succeeded"), json!("Succeeded"));
    assert_case(EffectPredicate::WasDeclined, ("EffectPredicate", 13, "WasDeclined"), json!("WasDeclined"));
    assert_case(EffectPredicate::AffectedObjectsShare { required_count: 2, characteristic: ObjectCharacteristic::Name },
        ("EffectPredicate", 14, "AffectedObjectsShare"),
        json!({"AffectedObjectsShare": {"required_count": 2, "characteristic": "Name"}}));
    assert_case(AnthemCountExpression::MatchingFilter(ObjectFilter::creature()),
        ("AnthemCountExpression", 0, "MatchingFilter"), json!({"MatchingFilter": ObjectFilter::creature()}));
    assert_case(AnthemCountExpression::TotalUnspentMana(PlayerFilter::You),
        ("AnthemCountExpression", 20, "TotalUnspentMana"), json!({"TotalUnspentMana": "You"}));
    assert_case(AnthemCountExpression::PlayerCounters(PlayerFilter::Opponent, CounterType::Poison),
        ("AnthemCountExpression", 21, "PlayerCounters"), json!({"PlayerCounters": ["Opponent", "Poison"]}));
    for (value, index, name) in [(Until::Forever, 0, "Forever"),
        (Until::ControllersNextUntapStep, 6, "ControllersNextUntapStep"),
        (Until::YourNextUntapStep, 15, "YourNextUntapStep")] {
        assert_case(value, ("Until", index, name), json!(name));
    }
    assert_case(Until::UntilControllersNextUntapStep { object: ContinuousDurationObject::AffectedObject },
        ("Until", 16, "UntilControllersNextUntapStep"),
        json!({"UntilControllersNextUntapStep": {"object": "AffectedObject"}}));
    assert_case(Until::PlayersNextUntapStep { player: PlayerFilter::Opponent },
        ("Until", 17, "PlayersNextUntapStep"), json!({"PlayersNextUntapStep": {"player": "Opponent"}}));
    assert_case(DelayedTriggerSpec::ConditionQualified {
        trigger: Box::new(DelayedTriggerSpec::ThisDies), condition: Condition::YourTurn, surface: "your turn".into(),
    }, ("DelayedTriggerSpec", 0, "ConditionQualified"), json!({"ConditionQualified": {
        "trigger": "ThisDies", "condition": "YourTurn", "surface": "your turn",
    }}));
    assert_case(DelayedTriggerSpec::Attacks(ObjectFilter::creature()),
        ("DelayedTriggerSpec", 21, "Attacks"), json!({"Attacks": ObjectFilter::creature()}));
    assert_case(DelayedTriggerSpec::PlayerDiscardsCard {
        player: PlayerFilter::You, filter: None, cause_controller: None, effect_like_only: false, one_or_more: false,
    }, ("DelayedTriggerSpec", 47, "PlayerDiscardsCard"), json!({"PlayerDiscardsCard": {
        "player": "You", "filter": null, "cause_controller": null, "effect_like_only": false, "one_or_more": false,
    }}));
    assert_case(DelayedTriggerSpec::PlayerAttackDeclaration {
        attacker: PlayerFilter::You, defender: PlayerFilter::Opponent, grouping: PlayerAttackGrouping::AttackerAnyTarget,
    }, ("DelayedTriggerSpec", 48, "PlayerAttackDeclaration"), json!({"PlayerAttackDeclaration": {
        "attacker": "You", "defender": "Opponent", "grouping": "AttackerAnyTarget",
    }}));
}
