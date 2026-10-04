#!/usr/bin/env python3
"""Generate the typed runtime-effect materializer modules.

The registry is captured once from the engine materializer and then becomes
the stable input for regeneration. Payloads are assigned by runtime ownership
family, and each family is one module of ironsmith-artifact-effect-decoder.
They share a crate so serde instantiations common to several families (filters,
values, conditions, ...) are compiled once rather than once per family.
"""

from __future__ import annotations

import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REGISTRY = ROOT / "crates/ironsmith-artifact-effect-decoder/effect-registry.tsv"
MATERIALIZER = ROOT / "crates/ironsmith-engine/src/artifact_materializer.rs"
SHARD_COUNT = 8
FAMILY_NAMES = (
    "zone-library",
    "player",
    "resources",
    "permanent",
    "combat",
    "stack-event",
    "composition-a-l",
    "composition-m-z",
)


def runtime_family_for(kind: str) -> str:
    """Locate the engine module that implements one serialized effect kind."""

    effects_root = ROOT / "crates/ironsmith-engine/src/effects"
    patterns = (
        f"impl EffectExecutor for {kind}",
        f"impl CostExecutableEffect for {kind}",
        f"pub struct {kind}",
        f"pub enum {kind}",
        f"pub type {kind}",
        f"pub use ironsmith_core::{kind};",
    )
    sources = sorted(effects_root.rglob("*.rs"))
    for path in sources:
        source = path.read_text(encoding="utf-8")
        if any(pattern in source for pattern in patterns):
            return path.relative_to(effects_root).parts[0]
    # A number of shared-schema executors use multiline ``pub use`` lists.
    for path in sources:
        source = path.read_text(encoding="utf-8")
        if (
            kind in source
            and "pub use ironsmith_core" in source
            and ("impl EffectExecutor" in source or path.name == "mod.rs")
        ):
            return path.relative_to(effects_root).parts[0]
    raise SystemExit(f"could not locate runtime effect family for {kind}")


def shard_for(kind: str) -> int:
    # Core-model effects interpreted directly may have no native executor file.
    # The versioned registry owns their routing alongside the payload schema.
    if REGISTRY.exists():
        for line in REGISTRY.read_text(encoding="utf-8").splitlines():
            if line.strip() and not line.startswith("#"):
                columns = line.split("\t")
                if columns[0] == kind and len(columns) == 3:
                    return FAMILY_NAMES.index(columns[2])
    family = runtime_family_for(kind)
    if family in {"zones", "cards"}:
        return 0
    if family == "player":
        return 1
    if family in {"mana", "life", "counters"}:
        return 2
    if family in {"permanents", "tokens", "control", "continuous"}:
        return 3
    if family in {"combat", "damage"}:
        return 4
    if family in {"stack", "delayed", "replacement", "restrictions.rs"}:
        return 5
    if family == "composition":
        return 6 if kind[0].lower() < "m" else 7
    raise SystemExit(f"unassigned runtime effect family {family!r} for {kind}")


def load_registry() -> list[tuple[str, str]]:
    if REGISTRY.exists():
        return [
            tuple(line.split("\t", 2)[:2])
            for line in REGISTRY.read_text(encoding="utf-8").splitlines()
            if line.strip() and not line.startswith("#")
        ]

    pattern = re.compile(
        r'^\s*"([^"]+)"\s*=>\s*decode_as::<T,\s*(.+)>\(effect\),\s*$'
    )
    entries = []
    for line in MATERIALIZER.read_text(encoding="utf-8").splitlines():
        match = pattern.match(line)
        if match:
            entries.append((match.group(1), match.group(2)))
    if len(entries) < 200:
        raise SystemExit(f"expected at least 200 effect decoders, found {len(entries)}")
    REGISTRY.parent.mkdir(parents=True, exist_ok=True)
    REGISTRY.write_text(
        "# effect kind\tserde payload type\n"
        + "".join(f"{kind}\t{payload}\n" for kind, payload in entries),
        encoding="utf-8",
    )
    return entries


def manifest(name: str, dependencies: str) -> str:
    return f"""[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

[dependencies]
{dependencies}

[lib]
path = "src/lib.rs"

[features]
default = []
"""


def module_name(index: int) -> str:
    return FAMILY_NAMES[index].replace("-", "_")


def write_shard(index: int, entries: list[tuple[str, str]]) -> None:
    source_dir = ROOT / "crates/ironsmith-artifact-effect-decoder/src"
    source_dir.mkdir(parents=True, exist_ok=True)
    arms = "\n".join(
        f'        "{kind}" => decode_as::<{payload}>(payload).map(Some),'
        for kind, payload in entries
    )
    card_arms = "\n".join(
        f'        "{kind}" => super::card_graph::map_payload_as::<{payload}>(payload, context).map(Some),'
        for kind, payload in entries
    )
    (source_dir / f"{module_name(index)}.rs").write_text(
        f"""//! Generated typed materializers for the {FAMILY_NAMES[index]} runtime effect family.

#[allow(unused_imports)]
use ironsmith_compiled_artifact as wire;
use serde_json::Value;

use super::{{ErasedPayload, decode_as}};

pub fn decode(kind: &str, payload: Value) -> Result<Option<ErasedPayload>, String> {{
    match kind {{
{arms}
        _ => Ok(None),
    }}
}}

pub(super) fn map_card_ids(
    kind: &str,
    payload: Value,
    context: &super::card_graph::Context<'_>,
) -> Result<Option<Value>, String> {{
    match kind {{
{card_arms}
        _ => Ok(None),
    }}
}}
""",
        encoding="utf-8",
    )


def write_facade(entries: list[tuple[str, str]]) -> None:
    crate_dir = ROOT / "crates/ironsmith-artifact-effect-decoder"
    source_dir = crate_dir / "src"
    source_dir.mkdir(parents=True, exist_ok=True)
    dependencies = (
        'ironsmith-core = { path = "../ironsmith-core", default-features = false, features = ["serde"] }\n'
        'ironsmith-compiled-artifact = { path = "../ironsmith-compiled-artifact", default-features = false }\n'
        'serde = "1.0.228"\n'
        'serde_json = "1.0.149"'
    )
    (crate_dir / "Cargo.toml").write_text(
        manifest("ironsmith-artifact-effect-decoder", dependencies),
        encoding="utf-8",
    )
    family_variants = (
        "ZoneLibrary",
        "Player",
        "Resources",
        "Permanent",
        "Combat",
        "StackEvent",
        "CompositionAL",
        "CompositionMZ",
    )
    family_arms = "\n".join(
        f'        "{kind}" => Some(EffectFamily::{family_variants[shard_for(kind)]}),' 
        for kind, _ in entries
    )
    (source_dir / "lib.rs").write_text(
        f"""//! Typed compiled-effect decoding, organized by runtime effect family.
//!
//! The families are modules of one crate rather than sibling crates so that
//! each serde instantiation they share (filters, values, conditions, ...) is
//! compiled once instead of once per family.

use std::any::Any;

use serde::de::DeserializeOwned;
use serde_json::Value;

mod combat;
mod composition_a_l;
mod composition_m_z;
mod permanent;
mod player;
mod resources;
mod stack_event;
mod zone_library;

pub type ErasedPayload = Box<dyn Any + Send + Sync>;

fn decode_as<D>(payload: Value) -> Result<ErasedPayload, String>
where
    D: DeserializeOwned + Send + Sync + 'static,
{{
    serde_json::from_value::<D>(payload)
        .map(|value| Box::new(value) as ErasedPayload)
        .map_err(|error| error.to_string())
}}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectFamily {{
    ZoneLibrary,
    Player,
    Resources,
    Permanent,
    Combat,
    StackEvent,
    CompositionAL,
    CompositionMZ,
}}

pub fn family_for_kind(kind: &str) -> Option<EffectFamily> {{
    match kind {{
{family_arms}
        _ => None,
    }}
}}

pub fn decode(kind: &str, payload: Value) -> Result<ErasedPayload, String> {{
    let decoded = match family_for_kind(kind) {{
        Some(family) => match family {{
            EffectFamily::ZoneLibrary => zone_library::decode(kind, payload),
            EffectFamily::Player => player::decode(kind, payload),
            EffectFamily::Resources => resources::decode(kind, payload),
            EffectFamily::Permanent => permanent::decode(kind, payload),
            EffectFamily::Combat => combat::decode(kind, payload),
            EffectFamily::StackEvent => stack_event::decode(kind, payload),
            EffectFamily::CompositionAL => composition_a_l::decode(kind, payload),
            EffectFamily::CompositionMZ => composition_m_z::decode(kind, payload),
        }},
        None => return Err(format!("unknown compiled effect payload kind: {{kind}}")),
    }}?;
    decoded.ok_or_else(|| format!("unknown compiled effect payload kind: {{kind}}"))
}}

#[cfg(test)]
mod tests {{
    use super::{{EffectFamily, family_for_kind}};

    #[test]
    fn routes_representative_effects_to_domain_families() {{
        assert_eq!(family_for_kind("MoveToZoneEffect"), Some(EffectFamily::ZoneLibrary));
        assert_eq!(family_for_kind("ChoosePlayerEffect"), Some(EffectFamily::Player));
        assert_eq!(family_for_kind("AddManaEffect"), Some(EffectFamily::Resources));
        assert_eq!(family_for_kind("CreateTokenEffect"), Some(EffectFamily::Permanent));
        assert_eq!(family_for_kind("DealDamageEffect"), Some(EffectFamily::Combat));
        assert_eq!(family_for_kind("CopySpellEffect"), Some(EffectFamily::StackEvent));
        assert_eq!(family_for_kind("ChooseModeEffect"), Some(EffectFamily::CompositionAL));
        assert_eq!(family_for_kind("WithIdEffect"), Some(EffectFamily::CompositionMZ));
        assert_eq!(family_for_kind("NotAnEffect"), None);
    }}
}}

{CARD_GRAPH_SOURCE}
""",
        encoding="utf-8",
    )


CARD_GRAPH_SOURCE = r'''/// Remap typed card references throughout a canonical payload, including opaque
/// compiled effects. The callback owns the graph namespace and failure policy.
pub fn remap_card_ids<T: serde::Serialize>(
    value: &T,
    bind: &mut dyn FnMut(u32) -> Result<u32, String>,
) -> Result<Value, String> {
    let context = card_graph::Context {
        bind: std::cell::RefCell::new(bind),
    };
    value
        .serialize(card_graph::Serializer { context: &context })
        .map_err(|error| error.to_string())
}

fn remap_effect_payload(
    kind: &str,
    payload: Value,
    context: &card_graph::Context<'_>,
) -> Result<Value, String> {
    let mapped = match family_for_kind(kind) {
        Some(EffectFamily::ZoneLibrary) => zone_library::map_card_ids(kind, payload, context),
        Some(EffectFamily::Player) => player::map_card_ids(kind, payload, context),
        Some(EffectFamily::Resources) => resources::map_card_ids(kind, payload, context),
        Some(EffectFamily::Permanent) => permanent::map_card_ids(kind, payload, context),
        Some(EffectFamily::Combat) => combat::map_card_ids(kind, payload, context),
        Some(EffectFamily::StackEvent) => stack_event::map_card_ids(kind, payload, context),
        Some(EffectFamily::CompositionAL) => composition_a_l::map_card_ids(kind, payload, context),
        Some(EffectFamily::CompositionMZ) => composition_m_z::map_card_ids(kind, payload, context),
        None => return Err(format!("unknown compiled effect payload kind: {kind}")),
    }?;
    mapped.ok_or_else(|| format!("unknown compiled effect payload kind: {kind}"))
}

mod card_graph {
    use serde::ser::{self, Serialize, SerializeMap as _, Serializer as _};
    use serde_json::{Value, value};
    use std::cell::RefCell;

    pub(super) struct Context<'b> {
        pub(super) bind: RefCell<&'b mut dyn FnMut(u32) -> Result<u32, String>>,
    }
    #[derive(Clone, Copy)]
    pub(super) struct Serializer<'a, 'b> {
        pub(super) context: &'a Context<'b>,
    }
    pub(super) fn map_payload_as<D: serde::de::DeserializeOwned + Serialize>(
        payload: Value,
        context: &Context<'_>,
    ) -> Result<Value, String> {
        let decoded: D = serde_json::from_value(payload).map_err(|error| error.to_string())?;
        decoded
            .serialize(Serializer { context })
            .map_err(|error| error.to_string())
    }
    pub(super) struct Compound<'a, 'b, S> {
        inner: S,
        context: &'a Context<'b>,
        name: Option<&'static str>,
    }
    impl<'a, 'b, S> Compound<'a, 'b, S> {
        fn new(inner: S, context: &'a Context<'b>) -> Self {
            Self {
                inner,
                context,
                name: None,
            }
        }
        fn mapped<T: Serialize + ?Sized>(&self, value: &T) -> Result<Value, serde_json::Error> {
            value.serialize(Serializer {
                context: self.context,
            })
        }
    }
    macro_rules! primitive {
        ($method:ident, $ty:ty) => {
            fn $method(self, value: $ty) -> Result<Value, Self::Error> {
                value::Serializer.$method(value)
            }
        };
    }
    impl<'a, 'b> ser::Serializer for Serializer<'a, 'b> {
        type Ok = Value;
        type Error = serde_json::Error;
        type SerializeSeq = Compound<'a, 'b, <value::Serializer as ser::Serializer>::SerializeSeq>;
        type SerializeTuple =
            Compound<'a, 'b, <value::Serializer as ser::Serializer>::SerializeTuple>;
        type SerializeTupleStruct =
            Compound<'a, 'b, <value::Serializer as ser::Serializer>::SerializeTupleStruct>;
        type SerializeTupleVariant =
            Compound<'a, 'b, <value::Serializer as ser::Serializer>::SerializeTupleVariant>;
        type SerializeMap = Compound<'a, 'b, <value::Serializer as ser::Serializer>::SerializeMap>;
        type SerializeStruct =
            Compound<'a, 'b, <value::Serializer as ser::Serializer>::SerializeStruct>;
        type SerializeStructVariant =
            Compound<'a, 'b, <value::Serializer as ser::Serializer>::SerializeStructVariant>;
        primitive!(serialize_bool, bool);
        primitive!(serialize_i8, i8);
        primitive!(serialize_i16, i16);
        primitive!(serialize_i32, i32);
        primitive!(serialize_i64, i64);
        primitive!(serialize_i128, i128);
        primitive!(serialize_u8, u8);
        primitive!(serialize_u16, u16);
        primitive!(serialize_u32, u32);
        primitive!(serialize_u64, u64);
        primitive!(serialize_u128, u128);
        primitive!(serialize_f32, f32);
        primitive!(serialize_f64, f64);
        primitive!(serialize_char, char);
        primitive!(serialize_str, &str);
        primitive!(serialize_bytes, &[u8]);
        fn serialize_none(self) -> Result<Value, Self::Error> {
            value::Serializer.serialize_none()
        }
        fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Value, Self::Error> {
            value.serialize(self)
        }
        fn serialize_unit(self) -> Result<Value, Self::Error> {
            value::Serializer.serialize_unit()
        }
        fn serialize_unit_struct(self, name: &'static str) -> Result<Value, Self::Error> {
            value::Serializer.serialize_unit_struct(name)
        }
        fn serialize_unit_variant(
            self,
            name: &'static str,
            index: u32,
            variant: &'static str,
        ) -> Result<Value, Self::Error> {
            value::Serializer.serialize_unit_variant(name, index, variant)
        }
        fn serialize_newtype_struct<T: Serialize + ?Sized>(
            self,
            name: &'static str,
            value: &T,
        ) -> Result<Value, Self::Error> {
            if name == "CardId" {
                let raw = value.serialize(value::Serializer)?;
                let id = raw
                    .as_u64()
                    .and_then(|number| u32::try_from(number).ok())
                    .ok_or_else(|| <Self::Error as ser::Error>::custom("invalid typed CardId"))?;
                let bound = (self.context.bind.borrow_mut())(id)
                    .map_err(<Self::Error as ser::Error>::custom)?;
                return value::Serializer.serialize_u32(bound);
            }
            let mapped = value.serialize(self)?;
            value::Serializer.serialize_newtype_struct(name, &mapped)
        }
        fn serialize_newtype_variant<T: Serialize + ?Sized>(
            self,
            name: &'static str,
            index: u32,
            variant: &'static str,
            value: &T,
        ) -> Result<Value, Self::Error> {
            let mapped = value.serialize(self)?;
            value::Serializer.serialize_newtype_variant(name, index, variant, &mapped)
        }
        fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
            Ok(Compound::new(
                value::Serializer.serialize_seq(len)?,
                self.context,
            ))
        }
        fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
            Ok(Compound::new(
                value::Serializer.serialize_tuple(len)?,
                self.context,
            ))
        }
        fn serialize_tuple_struct(
            self,
            name: &'static str,
            len: usize,
        ) -> Result<Self::SerializeTupleStruct, Self::Error> {
            Ok(Compound::new(
                value::Serializer.serialize_tuple_struct(name, len)?,
                self.context,
            ))
        }
        fn serialize_tuple_variant(
            self,
            name: &'static str,
            index: u32,
            variant: &'static str,
            len: usize,
        ) -> Result<Self::SerializeTupleVariant, Self::Error> {
            Ok(Compound::new(
                value::Serializer.serialize_tuple_variant(name, index, variant, len)?,
                self.context,
            ))
        }
        fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
            Ok(Compound::new(
                value::Serializer.serialize_map(len)?,
                self.context,
            ))
        }
        fn serialize_struct(
            self,
            name: &'static str,
            len: usize,
        ) -> Result<Self::SerializeStruct, Self::Error> {
            let mut compound =
                Compound::new(value::Serializer.serialize_struct(name, len)?, self.context);
            compound.name = Some(name);
            Ok(compound)
        }
        fn serialize_struct_variant(
            self,
            name: &'static str,
            index: u32,
            variant: &'static str,
            len: usize,
        ) -> Result<Self::SerializeStructVariant, Self::Error> {
            Ok(Compound::new(
                value::Serializer.serialize_struct_variant(name, index, variant, len)?,
                self.context,
            ))
        }
        fn collect_str<T: std::fmt::Display + ?Sized>(
            self,
            value: &T,
        ) -> Result<Value, Self::Error> {
            value::Serializer.collect_str(value)
        }
    }
    macro_rules! positional {
        ($trait:ident, $method:ident) => {
            impl<S> ser::$trait for Compound<'_, '_, S>
            where
                S: ser::$trait<Ok = Value, Error = serde_json::Error>,
            {
                type Ok = Value;
                type Error = serde_json::Error;
                fn $method<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
                    let mapped = self.mapped(value)?;
                    self.inner.$method(&mapped)
                }
                fn end(self) -> Result<Value, Self::Error> {
                    self.inner.end()
                }
            }
        };
    }
    positional!(SerializeSeq, serialize_element);
    positional!(SerializeTuple, serialize_element);
    positional!(SerializeTupleStruct, serialize_field);
    positional!(SerializeTupleVariant, serialize_field);
    impl<S> ser::SerializeMap for Compound<'_, '_, S>
    where
        S: ser::SerializeMap<Ok = Value, Error = serde_json::Error>,
    {
        type Ok = Value;
        type Error = serde_json::Error;
        fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Self::Error> {
            let mapped = self.mapped(key)?;
            self.inner.serialize_key(&mapped)
        }
        fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
            let mapped = self.mapped(value)?;
            self.inner.serialize_value(&mapped)
        }
        fn end(self) -> Result<Value, Self::Error> {
            self.inner.end()
        }
    }
    impl<S> ser::SerializeStruct for Compound<'_, '_, S>
    where
        S: ser::SerializeStruct<Ok = Value, Error = serde_json::Error>,
    {
        type Ok = Value;
        type Error = serde_json::Error;
        fn serialize_field<T: Serialize + ?Sized>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Self::Error> {
            let mapped = self.mapped(value)?;
            self.inner.serialize_field(key, &mapped)
        }
        fn end(self) -> Result<Value, Self::Error> {
            let mut result = self.inner.end()?;
            if self.name == Some("CompiledEffect") {
                let kind = result["kind"]
                    .as_str()
                    .ok_or_else(|| {
                        <Self::Error as ser::Error>::custom("missing compiled effect kind")
                    })?
                    .to_owned();
                let payload = result
                    .get_mut("payload")
                    .ok_or_else(|| {
                        <Self::Error as ser::Error>::custom("missing compiled effect payload")
                    })?
                    .take();
                result["payload"] = super::remap_effect_payload(&kind, payload, self.context)
                    .map_err(<Self::Error as ser::Error>::custom)?;
            }
            if self.name == Some("RetainedCardPayload") {
                match result.get("card_references").and_then(Value::as_str) {
                    Some("Native" | "Bound") => {},
                    _ => return Err(<Self::Error as ser::Error>::custom("missing retained payload card reference mode")),
                }
                result["card_references"] = Value::String("Bound".into());
            }
            Ok(result)
        }
    }
    impl<S> ser::SerializeStructVariant for Compound<'_, '_, S>
    where
        S: ser::SerializeStructVariant<Ok = Value, Error = serde_json::Error>,
    {
        type Ok = Value;
        type Error = serde_json::Error;
        fn serialize_field<T: Serialize + ?Sized>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Self::Error> {
            let mapped = self.mapped(value)?;
            self.inner.serialize_field(key, &mapped)
        }
        fn end(self) -> Result<Value, Self::Error> {
            self.inner.end()
        }
    }
}

#[cfg(test)]
mod card_graph_tests {
    use super::*;
    use ironsmith_compiled_artifact as wire;
    use ironsmith_core::{CardBuilder, CardId, ObjectId, StableId};

    fn nested_token() -> wire::WireEffect {
        let token: wire::WireCardDefinition = ironsmith_core::CardDefinition::new(
            CardBuilder::new(CardId::from_raw(9), "Graph token")
                .token()
                .other_face(CardId::from_raw(11))
                .build(),
        );
        let effect = wire::WireEffect::new(
            "CreateTokenEffect",
            serde_json::to_value(ironsmith_core::CreateTokenEffect::one(token)).unwrap(),
        );
        let tagged = ironsmith_core::TaggedEffect {
            tag: "created".into(),
            effect: Box::new(effect),
            outcome_only: false,
        };
        wire::WireEffect::new("TaggedEffect", serde_json::to_value(tagged).unwrap())
    }
    #[test]
    fn card_graph_typed_ids_preserve_other_namespaces_and_arbitrary_json() {
        #[derive(serde::Serialize)]
        struct Carrier {
            card: CardId,
            object: ObjectId,
            stable: StableId,
            arbitrary: Value,
        }
        let value = Carrier {
            card: CardId::from_raw(9),
            object: ObjectId::from_raw(9),
            stable: StableId::from_raw(9),
            arbitrary: serde_json::json!({"id":9,"CardId":9,"kind":"CardId"}),
        };
        let mut seen = Vec::new();
        let mapped = remap_card_ids(&value, &mut |id| {
            seen.push(id);
            Ok(id + 1000)
        })
        .unwrap();
        assert_eq!(seen, [9]);
        assert_eq!(mapped["card"], 1009);
        assert_eq!(mapped["object"], 9);
        assert_eq!(mapped["stable"], 9);
        assert_eq!(mapped["arbitrary"], value.arbitrary);
    }
    #[test]
    fn card_graph_typed_traversal_rebinds_nested_token_and_linked_face_once() {
        let original = nested_token();
        let mut seen = Vec::new();
        let mapped = remap_card_ids(&original, &mut |id| {
            seen.push(id);
            Ok(id + 1000)
        })
        .unwrap();
        assert_eq!(seen, [9, 11]);
        let receiver: wire::WireEffect = serde_json::from_value(mapped).unwrap();
        let tagged: ironsmith_core::TaggedEffect<wire::WireEffect> =
            serde_json::from_value(receiver.payload().clone()).unwrap();
        let token: ironsmith_core::CreateTokenEffect<wire::WireCardDefinition> =
            serde_json::from_value(tagged.effect.payload().clone()).unwrap();
        assert_eq!(token.token.card.id, CardId::from_raw(1009));
        assert_eq!(token.token.card.other_face, Some(CardId::from_raw(1011)));
        let roundtrip = remap_card_ids(&receiver, &mut |id| Ok(id - 1000)).unwrap();
        assert_eq!(roundtrip, serde_json::to_value(original).unwrap());
    }
    #[test]
    fn card_graph_typed_traversal_propagates_unknown_graph_and_nested_model_errors() {
        let mut seen = Vec::new();
        assert!(
            remap_card_ids(&nested_token(), &mut |id| {
                seen.push(id);
                if id == 11 {
                    Err("unbound linked template".into())
                } else {
                    Ok(1)
                }
            })
            .unwrap_err()
            .contains("unbound linked template")
        );
        assert_eq!(seen, [9, 11]);
        let unknown = wire::WireEffect::new("UnknownGraphEffect", serde_json::json!({}));
        assert!(
            remap_card_ids(&unknown, &mut |id| Ok(id))
                .unwrap_err()
                .contains("unknown compiled effect")
        );
    }
}
'''


def main() -> None:
    entries = load_registry()
    shards = [[] for _ in range(SHARD_COUNT)]
    for entry in entries:
        shards[shard_for(entry[0])].append(entry)
    for index, entries_for_shard in enumerate(shards):
        write_shard(index, entries_for_shard)
    write_facade(entries)
    print(
        f"generated {len(entries)} decoders across {SHARD_COUNT} family modules: "
        + ", ".join(str(len(entries_for_shard)) for entries_for_shard in shards)
    )


if __name__ == "__main__":
    main()
