use super::*;
use crate::ability::AbilityKind;
use crate::cards::CardDefinitionRuntimeExt;
use crate::color::Color;
use crate::compiled_text::{
    canonical_compiled_lines, compiled_text_lines, debug_compiled_lines, unprocessed_compiled_lines,
};
use crate::effects::{
    AddManaEffect, ChooseModeEffect, ChooseObjectsEffect, ConditionalEffect,
    ConsultTopOfLibraryEffect, CreateTokenCopyEffect, CreateTokenEffect, DestroyEffect,
    DoubleCountersEffect, DrawCardsEffect, EffectExecutor, GainLifeEffect, IfEffect,
    MoveToZoneEffect, ReturnFromGraveyardToHandEffect, TaggedEffect, TargetOnlyEffect, UntapEffect,
    WithIdEffect,
};
use crate::filter::ObjectFilterExt;
use crate::ids::StableId;
use crate::object::AuraAttachmentFilter;
use crate::static_abilities::StaticAbilityId;
use crate::target::{ChooseSpec, ObjectRef, PlayerFilter, SourceReferenceSurface};
use crate::zone::Zone;
use crate::{ObjectId, PlayerId};
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

mod additional_cost_object_provenance;
mod behold_exile_additional_cost;
mod same_name_search_pipeline;
mod shard_00;
mod shard_01;
mod shard_02;
mod shard_03;
mod shard_04;
mod shard_05;
mod shard_06;
mod shard_07;
mod shard_08;
mod shard_09;
mod shard_10;
mod shard_11;
mod shard_12;
mod shard_13;
mod shard_14;
mod shard_15;
mod shard_16;
mod shard_17;
mod shard_18;
mod shard_19;
mod shard_20;
mod shard_21;
mod shard_22;
mod shard_23;
mod shard_24;
mod shard_25;
mod shard_26;
mod shard_27;
mod shard_28;
mod shard_29;
mod shard_30;
mod shard_31;
mod shard_32;
mod shard_33;
mod shard_34;
mod shard_35;
mod shard_36;
mod shard_37;
mod shard_38;
mod shard_39;
mod shard_40;
mod shard_41;
mod shard_42;
mod shard_43;
mod shard_44;
mod shard_45;
mod consult_result_surfaces;
mod consult_revealed_collection_counts;
mod deferred_consult_pair;
mod identical_cost_activated_abilities;
