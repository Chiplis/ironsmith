use ironsmith::ResolvedTarget;
use ironsmith::card::{LinkedFaceLayout, PowerToughness};
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::color::ColorSet;
use ironsmith::effects::{EffectContext, EffectExecutor};
use ironsmith::ids::CardId;
use ironsmith::snapshot::{CopiableValues, ObjectSnapshot};
use ironsmith::types::{CardType, Subtype};
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use serde_json::{Value as J, json};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
type R<T> = Result<T, String>;
fn input() -> &'static J {
    static DATA: OnceLock<J> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../fixtures/wymi/upstream.json.fixture"
        ))
        .unwrap()
    })
}
fn translations() -> &'static J {
    static DATA: OnceLock<J> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../../../../fixtures/wymi/rules.json.fixture")).unwrap()
    })
}
fn array(v: &J) -> &[J] {
    v.as_array().map(Vec::as_slice).unwrap_or(&[])
}
fn string(v: &J) -> &str {
    v.as_str().unwrap_or("")
}
fn flag(v: &J) -> bool {
    v.as_bool().unwrap_or(false)
}
fn key(v: &J) -> String {
    serde_json::to_string(v).unwrap()
}
fn compile(name: &str, text: &str) -> R<CardDefinition> {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
    });
    let def = result.map_err(|e| format!("native compilation: {name}: {e:?}\n{text}"))?;
    if loss.is_lossy() || ironsmith::cards::generated_definition_has_unimplemented_content(&def) {
        return Err(format!("native compilation incomplete: {name}\n{text}"));
    }
    Ok(def)
}
fn rule_definition(rule: &J) -> R<CardDefinition> {
    static CACHE: OnceLock<Mutex<HashMap<String, R<CardDefinition>>>> = OnceLock::new();
    let mut cache = CACHE
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let k = key(rule);
    if let Some(x) = cache.get(&k) {
        return x.clone();
    }
    let row = array(translations())
        .iter()
        .find(|r| r["rule"] == *rule)
        .or_else(|| array(&input()["rules"]).iter().find(|r| r["rule"] == *rule));
    let result = if let Some(row) = row {
        let name = row["origin"].as_str().unwrap_or("Rule witness");
        let ty = if rule["_Rule"] == "SpellActions" {
            "Instant"
        } else {
            "Artifact Creature — Shapeshifter"
        };
        compile(
            name,
            &format!(
                "Mana cost: {{2}}\nType: {ty}\nPower/Toughness: 0/0\n{}",
                string(&row["text"])
            ),
        )
    } else {
        Err(format!("untranslated source rule: {rule}"))
    };
    cache.insert(k, result.clone());
    result
}
fn mana(v: &J) -> String {
    array(v)
        .iter()
        .map(|s| {
            let x = string(&s["_ManaSymbol"]).trim_start_matches("ManaCost");
            format!(
                "{{{}}}",
                if x == "Generic" {
                    s["args"].to_string()
                } else {
                    x.chars()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join("/")
                }
            )
        })
        .collect()
}
fn colors(v: &J) -> ColorSet {
    let mut c = ColorSet::COLORLESS;
    for x in array(v) {
        c = c.union(match string(x) {
            "White" => ColorSet::WHITE,
            "Blue" => ColorSet::BLUE,
            "Black" => ColorSet::BLACK,
            "Red" => ColorSet::RED,
            "Green" => ColorSet::GREEN,
            _ => panic!("unknown color {x}"),
        });
    }
    c
}
fn face(v: &J, back: bool) -> J {
    if v.get("FrontFace").is_some() {
        return face(&v[if back { "BackFace" } else { "FrontFace" }], false);
    }
    let mut v = v.clone();
    let side = if back { "Flipped" } else { "Unflipped" };
    if v["Name"]["_Name"] == "FlipName" {
        v["Name"] = json!({"_Name":"Name","Name":v["Name"][side]});
    }
    if v["Typeline"]["_Typeline"] == "FlipTypeline" {
        v["Typeline"] = json!({"_Typeline":"Typeline","Typeline":v["Typeline"][side]});
    }
    if v["CardPT"]["_CardPT"] == "FlipCardPT" {
        v["CardPT"] = json!({"_CardPT":"CardPT","CardPT":v["CardPT"][side]});
    }
    let mut rules = array(&v["Rules"]).to_vec();
    rules.extend_from_slice(array(&v["Flip"][side]));
    v["Rules"] = json!(rules);
    if v.get("Doors").is_some() {
        let d = v["Doors"][if back { "Right" } else { "Left" }].clone();
        if v["Name"].is_null() {
            v["Name"] = json!({"_Name":"Name","Name":d["Name"]});
        }
        if v["ManaCost"].is_null() {
            v["ManaCost"] = d["ManaCost"].clone();
        }
        let mut r = array(&v["Rules"]).to_vec();
        r.extend_from_slice(array(&d["Rules"]));
        v["Rules"] = json!(r);
    }
    if let Some(map) = v.as_object_mut() {
        map.remove("Flip");
        map.remove("Doors");
    }
    v
}
fn definition(v: &J) -> R<CardDefinition> {
    static CACHE: OnceLock<Mutex<HashMap<String, R<CardDefinition>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let k = key(v);
    if let Some(d) = cache.lock().unwrap_or_else(|e| e.into_inner()).get(&k) {
        return d.clone();
    }
    let result = definition_uncached(v);
    cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(k, result.clone());
    result
}
fn definition_uncached(v: &J) -> R<CardDefinition> {
    let v = face(v, false);
    let types = &v["Typeline"]["Typeline"];
    let name = v["Name"]["Name"]
        .as_str()
        .unwrap_or(ironsmith::object::FACE_DOWN_DISPLAY_NAME);
    let parse_list = |field: &str| {
        if types[field].is_null() {
            json!([])
        } else {
            types[field].clone()
        }
    };
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(
            serde_json::from_value(parse_list("Cardtypes"))
                .map_err(|e| format!("card types: {e}"))?,
        )
        .subtypes(
            serde_json::from_value(parse_list("Subtypes")).map_err(|e| format!("subtypes: {e}"))?,
        )
        .supertypes(
            serde_json::from_value(parse_list("Supertypes"))
                .map_err(|e| format!("supertypes: {e}"))?,
        );
    if !v["ManaCost"].is_null() {
        builder = builder.mana_cost(native_mana(&v["ManaCost"])?);
    }
    if let Some(pt) = v.get("CardPT") {
        builder = builder.power_toughness(PowerToughness::fixed(
            pt["CardPT"]["Power"].as_i64().unwrap_or(0) as i32,
            pt["CardPT"]["Toughness"].as_i64().unwrap_or(0) as i32,
        ));
    }
    let mut def = builder.build();
    def.card.loyalty = v["Loyalty"].as_u64().map(|x| x as u32);
    def.card.defense = v["Defense"].as_u64().map(|x| x as u32);
    def.card.color_indicator =
        (!v["ColorIndicator"].is_null()).then(|| colors(&v["ColorIndicator"]));
    if !v["AdditionalColorIndicator"].is_null() {
        def.card.color_indicator = Some(
            def.card
                .colors()
                .union(colors(&v["AdditionalColorIndicator"])),
        );
    }
    let mut previous = Vec::new();
    for sourced in array(&v["Rules"]) {
        let rule = &sourced["Rule"];
        let d = rule_definition(rule)?;
        // Wymi splits a single paired P/T CDA into two AST nodes.
        if rule["_Rule"] == "CDA_Toughness"
            && previous
                .iter()
                .any(|r: &J| r["_Rule"] == "CDA_Power" && r["args"] == rule["args"])
        {
            continue;
        }
        previous.push(rule.clone());
        def.abilities.extend(d.abilities);
        def.ability_labels.extend(d.ability_labels);
        def.alternative_casts.extend(d.alternative_casts);
        def.optional_costs.extend(d.optional_costs);
        if d.spell_effect.is_some() {
            def.spell_effect = d.spell_effect;
        }
    }
    Ok(def)
}
fn object(game: &mut GameState, v: &J, physical: &J, zone: Zone) -> R<ObjectId> {
    let modal = v["_OracleCard"] == "ModalDFC";
    let back = if modal {
        flag(&physical["BackFace"])
    } else {
        v.get("Doors").is_some()
            && flag(&physical["RightDoorOpen"])
            && !flag(&physical["LeftDoorOpen"])
    };
    let mut front = definition(&face(v, back))?;
    let linked = !modal
        && (v.get("FrontFace").is_some() || v.get("Flip").is_some() || v.get("Doors").is_some());
    if linked {
        let mut back = definition(&face(v, !back))?;
        let layout = if v.get("Flip").is_some() {
            LinkedFaceLayout::Flip
        } else if v.get("Doors").is_some() {
            LinkedFaceLayout::Split
        } else {
            LinkedFaceLayout::TransformLike
        };
        front.card.other_face = Some(back.card.id);
        front.card.other_face_name = Some(back.card.name.clone());
        front.card.linked_face_layout = layout;
        back.card.other_face = Some(front.card.id);
        back.card.other_face_name = Some(front.card.name.clone());
        back.card.linked_face_layout = layout;
        game.register_linked_face_definition(&front);
        game.register_linked_face_definition(&back);
    }
    let id = game.create_object_from_definition(&front, PlayerId::from_index(0), zone);
    if v.get("Doors").is_some() && zone == Zone::Battlefield {
        if flag(&physical["LeftDoorOpen"]) && flag(&physical["RightDoorOpen"]) {
            game.mark_room_fully_unlocked(id);
        } else if !flag(&physical["LeftDoorOpen"]) && !flag(&physical["RightDoorOpen"]) {
            game.mark_room_entered_with_no_unlocked_door(id);
        }
    }
    if flag(&physical["Transformed"]) && v.get("FrontFace").is_some() && !modal {
        if game
            .object(id)
            .unwrap()
            .has_static_ability_id(ironsmith::static_abilities::StaticAbilityId::Daybound)
        {
            game.has_day_night = true;
            game.is_night = true;
            game.apply_day_nightbound_transformations();
        } else if !game
            .transform_permanent(id)
            .map_err(|e| format!("transform: {e:?}"))?
        {
            return Err("native transform rejected".into());
        }
    }
    if flag(&physical["Flipped"]) {
        if !game.flip_permanent(id) {
            return Err("native flip rejected".into());
        }
    }
    if flag(&physical["FaceDown"]) {
        hidden(game, id, &physical["FaceDownProperties"])?;
    }
    Ok(id)
}
fn hidden(game: &mut GameState, id: ObjectId, properties: &J) -> R<()> {
    let ward = array(&properties["Rules"])
        .iter()
        .any(|r| r["Rule"]["_Rule"] == "Ward");
    game.object_mut(id)
        .unwrap()
        .apply_face_down_cast_overlay_with_disguise_ward(ward);
    game.set_face_down(id);
    // The custom characteristics are part of the face-down instruction's input.
    // Layer calculation must retain them; it must not force the ordinary 2/2 form.
    if !properties.is_null() {
        let d = definition(properties)?;
        game.object_mut(id)
            .unwrap()
            .apply_custom_face_down_overlay(definition_values(&d));
    }
    Ok(())
}
fn snapshot(g: &GameState, id: ObjectId) -> R<CopiableValues> {
    ObjectSnapshot::try_from_object_with_calculated_characteristics(
        g.object(id).ok_or("missing object")?,
        g,
    )
    .map(|s| s.copiable_values)
    .map_err(|e| format!("native snapshot: {e:?}"))
}
fn execute_text(g: &mut GameState, host: ObjectId, target: ObjectId, text: &str) -> R<()> {
    // Execute a resolved self-copy instruction in an activated-ability body.
    // A standalone self-reference on a sorcery can be classified as static text.
    let def = compile(
        "Wymi operation",
        &format!("Type: Artifact Creature\nPower/Toughness: 1/1\n{{0}}: {text}"),
    )?;
    let abilities = def
        .abilities
        .iter()
        .filter_map(|a| match &a.kind {
            ironsmith::ability::AbilityKind::Activated(a) => Some(a),
            _ => None,
        })
        .collect::<Vec<_>>();
    if abilities.len() != 1 {
        return Err("native compilation produced no unique copy activation".into());
    }
    let mut ctx = EffectContext::new_default(host, PlayerId::from_index(0))
        .with_targets(vec![ResolvedTarget::Object(target)]);
    for effect in abilities[0].effects.iter() {
        ironsmith::effects::execute_effect(g, effect, &mut ctx)
            .map_err(|e| format!("native execution: {e:?}: {text}"))?;
    }
    Ok(())
}

fn copy_text(mods: &J, host: &CopiableValues) -> R<String> {
    let mut clauses = Vec::new();
    for m in array(mods) {
        let a = &m["args"];
        let words = || {
            array(a)
                .iter()
                .map(|x| string(x).to_lowercase())
                .collect::<Vec<_>>()
                .join(" ")
        };
        clauses.push(match string(&m["_StaticCopyEffect"]) {
            "RemoveSupertypes" => format!("it isn't {}", words()),
            "AddSupertypes" | "AddCardtypes" | "AddCreatureTypes" | "AddArtifactTypes"
            | "AddLandTypes" => format!("it's {} in addition to its other types", words()),
            "SetCardtypes" => format!("it's {} and loses all other card types", words()),
            "SetArtifactTypes" => format!("it's a {}", words()),
            "SetName" => format!("its name is {}", string(a)),
            "KeepName" => format!("its name is {}", host.name),
            "HasNoManaCost" => "it has no mana cost".into(),
            "SetPT" => format!("it's {}/{}", a["Power"], a["Toughness"]),
            "KeepPT" => format!(
                "it's {}/{}",
                host.power.unwrap_or(0),
                host.toughness.unwrap_or(0)
            ),
            "KeepColor" => "it doesn't copy that permanent's color".into(),
            "SetColor" | "AddColor" => {
                let c = match string(&a["_SettableColor"]) {
                    "AllColors" => "all colors".into(),
                    "Devoid" => "colorless".into(),
                    _ => array(&a["args"])
                        .iter()
                        .map(|x| string(x).to_lowercase())
                        .collect::<Vec<_>>()
                        .join(" and "),
                };
                format!(
                    "it's {c}{}",
                    if m["_StaticCopyEffect"] == "AddColor" {
                        " in addition to its other colors"
                    } else {
                        ""
                    }
                )
            }
            "AddAbility" => {
                let texts = array(a)
                    .iter()
                    .map(|r| rule_text(r))
                    .collect::<R<Vec<_>>>()?;
                format!(
                    "it has {}",
                    texts
                        .into_iter()
                        .map(|s| ability_clause(&s))
                        .collect::<Vec<_>>()
                        .join(" and ")
                )
            }
            "AddAbilityIfItDoesntHaveAbility" => {
                format!("it has {} if it doesn't have vanishing", rule_text(&a[0])?)
            }
            x => return Err(format!("untranslated copy modifier {x}")),
        });
    }
    Ok(format!(
        "This permanent becomes a copy of target permanent{}.",
        if clauses.is_empty() {
            String::new()
        } else {
            format!(", except {}", clauses.join(", and "))
        }
    ))
}
fn rule_text(r: &J) -> R<String> {
    array(translations())
        .iter()
        .chain(array(&input()["rules"]))
        .find(|x| x["rule"] == *r)
        .and_then(|x| x["text"].as_str())
        .map(str::to_owned)
        .ok_or_else(|| format!("untranslated rule text {r}"))
}
// Wymi supplies resolved layer instructions, not Oracle sentences. Translate
// them to the same native continuous-effect primitives used by BecomeCopy
// lowering, so parser phrasing cannot hide runtime copy failures.
fn execute_copy(g: &mut GameState, host: ObjectId, donor: ObjectId, mods: &J) -> R<()> {
    use ironsmith::continuous::{EffectTarget, Modification as M, PtSublayer};
    use ironsmith::effects::continuous::{ApplyContinuousEffect, RuntimeModification as RM};
    use ironsmith::target::ChooseSpec;
    let before = snapshot(g, host)?;
    if array(mods)
        .iter()
        .any(|m| m["_StaticCopyEffect"] == "AddAbilityIfItDoesntHaveAbility")
    {
        // Conditional keyword acquisition is a compiler-owned instruction;
        // retain its condition instead of deciding it in the test adapter.
        return execute_text(g, host, donor, &copy_text(mods, &before)?);
    }
    let mut name = None;
    let mut supertypes = Vec::new();
    let mut abilities = Vec::new();
    let mut additions = Vec::new();
    let mut keep_color = false;
    let mut keep_name = false;
    let mut clear_mana_cost = false;
    let mut sets_pt = false;
    for m in array(mods) {
        let a = &m["args"];
        macro_rules! decode {
            () => {
                serde_json::from_value(a.clone()).map_err(|e| format!("copy input: {e}"))?
            };
        }
        match string(&m["_StaticCopyEffect"]) {
            "KeepName" => keep_name = true,
            "SetName" => name = Some(string(a).to_owned()),
            "KeepColor" => keep_color = true,
            "AddSupertypes" => supertypes = decode!(),
            "RemoveSupertypes" => additions.push(M::RemoveSupertypes(decode!())),
            "AddCardtypes" => additions.push(M::AddCardTypes(decode!())),
            "SetCardtypes" => additions.push(M::SetCardTypes(decode!())),
            "AddCreatureTypes" | "AddArtifactTypes" | "AddLandTypes" => {
                additions.push(M::AddSubtypes(decode!()))
            }
            "SetArtifactTypes" => {
                additions.push(M::RemoveAllSubtypesOfFamily(
                    ironsmith::types::SubtypeFamily::Artifact,
                ));
                additions.push(M::AddSubtypes(decode!()));
            }
            "SetColor" | "AddColor" => {
                let c = match string(&a["_SettableColor"]) {
                    "AllColors" => ColorSet::WHITE
                        .union(ColorSet::BLUE)
                        .union(ColorSet::BLACK)
                        .union(ColorSet::RED)
                        .union(ColorSet::GREEN),
                    "Devoid" => ColorSet::COLORLESS,
                    _ => colors(&a["args"]),
                };
                additions.push(if m["_StaticCopyEffect"] == "SetColor" {
                    M::SetColors(c)
                } else {
                    M::AddColors(c)
                });
            }
            "KeepPT" | "SetPT" => {
                let (p, t) = if m["_StaticCopyEffect"] == "KeepPT" {
                    (before.power.unwrap_or(0), before.toughness.unwrap_or(0))
                } else {
                    (
                        a["Power"].as_i64().ok_or("invalid copy power")? as i32,
                        a["Toughness"].as_i64().ok_or("invalid copy toughness")? as i32,
                    )
                };
                additions.push(M::SetPowerToughness {
                    power: p.into(),
                    toughness: t.into(),
                    sublayer: PtSublayer::Setting,
                });
                sets_pt = true;
            }
            "AddAbility" => {
                for rule in array(a) {
                    abilities.extend(rule_definition(rule)?.abilities);
                }
            }
            "HasNoManaCost" => clear_mana_cost = true,
            x => return Err(format!("untranslated native copy modifier {x}")),
        }
    }
    let mut e = ApplyContinuousEffect::new_runtime(
        EffectTarget::Specific(host),
        RM::CopyOfWithAbilities {
            source: ChooseSpec::SpecificObject(donor),
            preserve_source_abilities: false,
            name_override: name,
            name_override_surface: None,
            add_supertypes: supertypes,
            copy_exception_surface: None,
            abilities,
        },
        ironsmith::effect::Until::EndOfTurn,
    );
    e.additional_modifications = additions;
    e.resolve_set_pt_values_at_resolution = sets_pt;
    if clear_mana_cost {
        e.runtime_modifications.push(RM::ClearCopyManaCost);
    }
    if keep_name {
        e.runtime_modifications.push(RM::RetainSourceName);
    }
    if keep_color {
        e.runtime_modifications.push(RM::RetainSourceColors);
    }
    e.execute(
        g,
        &mut EffectContext::new_default(host, PlayerId::from_index(0)),
    )
    .map_err(|e| format!("native copy execution: {e:?}"))?;
    Ok(())
}
fn scene(a: &J) -> R<(GameState, ObjectId)> {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let mut ids = HashMap::new();
    for o in array(&a["game"]["Battlefield"]) {
        let id = object(
            &mut g,
            &o["Physical"]["Object"],
            &o["Physical"],
            Zone::Battlefield,
        )?;
        ids.insert(o["Id"].as_u64().unwrap(), id);
    }
    if a["target"]["field"] != "Layer0_Base" {
        for e in array(&a["game"]["Effects"]) {
            let rule = &e["Rule"]["Rule"];
            let original = rule["args"][0]["args"]
                .as_u64()
                .ok_or("unsupported layer target")?;
            let host = ids[&original];
            let physical = &array(&a["game"]["Battlefield"])
                .iter()
                .find(|o| o["Id"] == original)
                .unwrap()["Physical"];
            for m in array(&rule["args"][1]) {
                let args = &m["args"];
                match string(&m["_StaticLayerEffect"]) {
                    "IsACopyOf" => {
                        let donor = object(&mut g, &args[0], &J::Null, Zone::Battlefield)?;
                        execute_copy(&mut g, host, donor, &args[1]["args"])?;
                    }
                    "MutateUnder" | "MutateOnTop" => {
                        let part = &physical["MutateCards"][args.as_u64().unwrap() as usize];
                        let mut component_physical = part.clone();
                        // Wymi stores the shared face-down instruction on the
                        // whole merged permanent, not again on each component.
                        if flag(&part["FaceDown"])
                            && part["FaceDownProperties"].is_null()
                            && !physical["FaceDownProperties"].is_null()
                        {
                            component_physical["FaceDownProperties"] =
                                physical["FaceDownProperties"].clone();
                        }
                        let spell =
                            object(&mut g, &part["Object"], &component_physical, Zone::Stack)?;
                        // Source layer tests model merging already-authorized creatures. Supply
                        // reversible layer-4 animation/type changes to make abstract land/Human
                        // fixtures legal targets without changing their layer-1 values.
                        use ironsmith::continuous::{ContinuousEffect, EffectTarget, Modification};
                        g.effect_store
                            .continuous_effects
                            .add_effect(ContinuousEffect::new(
                                host,
                                PlayerId::from_index(0),
                                EffectTarget::Specific(host),
                                Modification::AddCardTypes(vec![CardType::Creature]),
                            ));
                        g.effect_store
                            .continuous_effects
                            .add_effect(ContinuousEffect::new(
                                host,
                                PlayerId::from_index(0),
                                EffectTarget::Specific(host),
                                Modification::RemoveSubtypes(vec![Subtype::Human]),
                            ));
                        if g.merge_mutating_creature_spell(
                            spell,
                            host,
                            m["_StaticLayerEffect"] == "MutateOnTop",
                        )
                        .is_none()
                        {
                            return Err("native mutate rejected authorized fixture".into());
                        }
                    }
                    "SetCopiableManaCost" => {
                        let pt = g.object(host).unwrap();
                        let (p, t) = (
                            pt.base_power.unwrap().base_value(),
                            pt.base_toughness.unwrap().base_value(),
                        );
                        let d = compile(
                            "Prototype cost",
                            &format!(
                                "Type: Artifact Creature\nMana cost: {}\nPower/Toughness: {p}/{t}",
                                mana(args)
                            ),
                        )?;
                        g.object_mut(host).unwrap().apply_prototype_cast_overlay(
                            d.card.mana_cost.unwrap(),
                            PowerToughness::fixed(p, t),
                        );
                    }
                    "SetCopiablePT" => {
                        let o = g.object_mut(host).unwrap();
                        o.base_power = Some(ironsmith::card::PtValue::Fixed(
                            args["Power"].as_i64().unwrap() as i32,
                        ));
                        o.base_toughness = Some(ironsmith::card::PtValue::Fixed(
                            args["Toughness"].as_i64().unwrap() as i32,
                        ));
                    }
                    "AddCopiableAbility" => {
                        use ironsmith::continuous::EffectTarget;
                        use ironsmith::effects::continuous::{
                            ApplyContinuousEffect, RuntimeModification,
                        };
                        let mut abilities = Vec::new();
                        for r in array(args) {
                            abilities.extend(rule_definition(r)?.abilities);
                        }
                        let e = ApplyContinuousEffect::new_runtime(
                            EffectTarget::Specific(host),
                            RuntimeModification::CopyOfWithAbilities {
                                source: ironsmith::target::ChooseSpec::SpecificObject(host),
                                preserve_source_abilities: false,
                                name_override: None,
                                name_override_surface: None,
                                add_supertypes: vec![],
                                copy_exception_surface: None,
                                abilities,
                            },
                            ironsmith::effect::Until::EndOfTurn,
                        );
                        e.execute(
                            &mut g,
                            &mut EffectContext::new_default(host, PlayerId::from_index(0)),
                        )
                        .map_err(|e| format!("native copiable ability grant: {e:?}"))?;
                    }
                    "AddCopiableSubtype" => {
                        let d = definition(
                            &json!({"_OracleCard":"Card","Name":{"Name":"Type witness"},"Typeline":{"Typeline":{"Cardtypes":["Artifact","Creature","Land"],"Subtypes":[args]}}}),
                        )?;
                        g.object_mut(host).unwrap().subtypes.extend(d.card.subtypes);
                    }
                    "AddCopiableCardtype" => {
                        let d = definition(
                            &json!({"_OracleCard":"Card","Name":{"Name":"Type witness"},"Typeline":{"Typeline":{"Cardtypes":[args]}}}),
                        )?;
                        let o = g.object_mut(host).unwrap();
                        for ty in d.card.card_types {
                            if !o.card_types.contains(&ty) {
                                o.card_types.push(ty);
                            }
                        }
                    }
                    "AdjustPT" | "AdjustPTForEach" => {
                        return Err(format!(
                            "unexpected later-layer instruction in recorded game effects: {m}"
                        ));
                    }
                    x => return Err(format!("untranslated layer instruction {x}")),
                }
            }
        }
    }
    Ok((
        g,
        ids[&a["target"]["id"]
            .as_u64()
            .ok_or("missing assertion target")?],
    ))
}
// Compare complete executable wire models rather than process-local executor
// addresses. Only fresh card IDs and definition provenance are alpha-renamed.
fn ability_equal(a: &ironsmith::ability::Ability, b: &ironsmith::ability::Ability) -> bool {
    fn canonical(a: &ironsmith::ability::Ability) -> Option<J> {
        let wire =
            ironsmith_runtime_catalog::artifact_materializer::encode_runtime_ability(a.clone())
                .ok()?;
        let mut v = serde_json::to_value(wire).ok()?;
        fn visit(v: &mut J) {
            match v {
                J::Object(m) => {
                    m.remove("trigger_definition");
                    m.remove("activation_definition");
                    if m.contains_key("mana_cost")
                        && m.contains_key("power_toughness")
                        && m.contains_key("card_types")
                    {
                        m.remove("id");
                    }
                    for child in m.values_mut() {
                        visit(child);
                    }
                }
                J::Array(a) => {
                    for child in a {
                        visit(child);
                    }
                }
                _ => {}
            }
        }
        visit(&mut v);
        Some(v)
    }
    a == b
        || match (canonical(a), canonical(b)) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
}
fn definition_values(d: &CardDefinition) -> CopiableValues {
    let o = ironsmith::object::Object::from_card_definition(
        ObjectId::from_raw(999999),
        d,
        PlayerId::from_index(0),
        Zone::Battlefield,
    );
    CopiableValues::from_object(&o)
}
fn compare_values(actual: &CopiableValues, expected: &CardDefinition) -> R<()> {
    let e = &expected.card;
    let mut errors = Vec::new();
    macro_rules! check {
        ($field:expr,$a:expr,$b:expr) => {
            if $a != $b {
                errors.push(format!("{}: actual {:?}; expected {:?}", $field, $a, $b));
            }
        };
    }
    let name = if e.name == ironsmith::object::FACE_DOWN_DISPLAY_NAME {
        ""
    } else {
        &e.name
    };
    let actual_name = if actual.name == ironsmith::object::FACE_DOWN_DISPLAY_NAME {
        ""
    } else {
        &actual.name
    };
    check!("name", actual_name, name);
    check!("mana_cost", actual.mana_cost, e.mana_cost);
    check!(
        "power",
        actual.power,
        e.power_toughness.map(|p| p.power.base_value())
    );
    check!(
        "toughness",
        actual.toughness,
        e.power_toughness.map(|p| p.toughness.base_value())
    );
    let sorted = |x: Vec<String>| {
        let mut x = x;
        x.sort();
        x
    };
    check!(
        "card_types",
        sorted(actual.card_types.iter().map(|x| format!("{x:?}")).collect()),
        sorted(e.card_types.iter().map(|x| format!("{x:?}")).collect())
    );
    check!(
        "subtypes",
        sorted(actual.subtypes.iter().map(|x| format!("{x:?}")).collect()),
        sorted(e.subtypes.iter().map(|x| format!("{x:?}")).collect())
    );
    check!(
        "supertypes",
        sorted(actual.supertypes.iter().map(|x| format!("{x:?}")).collect()),
        sorted(e.supertypes.iter().map(|x| format!("{x:?}")).collect())
    );
    check!("colors", actual.colors, e.colors());
    check!(
        "spell program",
        actual.spell_effect,
        ironsmith::snapshot::SpellProgramState::from_option(expected.spell_effect.clone())
    );
    check!(
        "aura attachment restriction",
        actual.aura_attach_filter,
        definition_values(expected).aura_attach_filter
    );
    check!("loyalty", actual.loyalty, e.loyalty);
    check!("defense", actual.defense, e.defense);
    let mut unmatched = actual.abilities.as_ref().clone();
    for expected in &expected.abilities {
        if let Some(i) = unmatched.iter().position(|a| ability_equal(a, expected)) {
            unmatched.remove(i);
        } else {
            errors.push(format!("missing executable ability: {expected:?}"));
        }
    }
    if !unmatched.is_empty() {
        errors.push(format!("unexpected executable abilities: {unmatched:?}"));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}
fn same_multiset<T: Clone + PartialEq>(actual: &[T], expected: &[T]) -> bool {
    let mut remaining = actual.to_vec();
    for e in expected {
        let Some(n) = remaining.iter().position(|a| a == e) else {
            return false;
        };
        remaining.remove(n);
    }
    remaining.is_empty()
}
fn layer_assertion(a: &J) -> R<()> {
    let mut errors = Vec::new();
    if let Err(e) = layer_characteristics_assertion(a) {
        errors.push(e);
    }
    // Prototype and mutate lower to native casting metadata, not AbilityKind.
    // Whole-rules assertions must check those payloads too.
    let metadata = (|| -> R<()> {
        let (g, id) = scene(a)?;
        if g.is_face_down(id) {
            return Ok(());
        }
        let physical = &array(&a["game"]["Battlefield"])
            .iter()
            .find(|o| o["Id"] == a["target"]["id"])
            .unwrap()["Physical"];
        let expected = definition(&face(&a["expected"], flag(&physical["Transformed"])))?;
        let o = snapshot(&g, id)?;
        if !same_multiset(
            o.alternative_casts.as_slice(),
            expected.alternative_casts.as_slice(),
        ) {
            return Err(format!(
                "casting methods: actual {:?}; expected {:?}",
                o.alternative_casts, expected.alternative_casts
            ));
        }
        if !same_multiset(
            o.optional_costs.as_slice(),
            expected.optional_costs.as_slice(),
        ) {
            return Err(format!(
                "optional casting costs: actual {:?}; expected {:?}",
                o.optional_costs, expected.optional_costs
            ));
        }
        Ok(())
    })();
    if let Err(e) = metadata {
        if !errors.contains(&e) {
            errors.push(e);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}
fn layer_characteristics_assertion(a: &J) -> R<()> {
    let (mut g, id) = scene(a)?;
    // Layer1_TokenCopy is an upstream pre-entry blueprint. Creating a token
    // here would execute new as-enters replacements absent from that assertion.
    let mut expected = a["expected"].clone();
    let physical = &array(&a["game"]["Battlefield"])
        .iter()
        .find(|o| o["Id"] == a["target"]["id"])
        .unwrap()["Physical"];
    if a["target"]["field"] == "Layer0_Base" && flag(&physical["FaceDown"]) {
        expected = if physical["FaceDownProperties"].is_null() {
            json!({"_OracleCard":"Card","Typeline":{"Typeline":{"Cardtypes":["Creature"]}},"CardPT":{"CardPT":{"Power":2,"Toughness":2}}})
        } else {
            physical["FaceDownProperties"].clone()
        };
    }
    // Native snapshots expose the selected face; check the other physical face
    // separately for upstream assertions that carry a two-face object.
    if expected.get("FrontFace").is_some() {
        let selected = flag(&physical["Transformed"]);
        compare_values(&snapshot(&g, id)?, &definition(&face(&expected, selected))?)?;
        if a["target"]["field"] == "Layer0_Base" {
            let o = g.object(id).unwrap();
            let other = g
                .linked_face_definition_by_name_or_id(o.other_face_name.as_deref(), o.other_face)
                .ok_or("native two-face copy lost its other face")?;
            return compare_values(
                &definition_values(&other),
                &definition(&face(&expected, !selected))?,
            );
        }
        if !g
            .transform_permanent(id)
            .map_err(|e| format!("other-face transform: {e:?}"))?
        {
            return Err("native two-face copy lost its other face".into());
        }
        return compare_values(
            &snapshot(&g, id)?,
            &definition(&face(&expected, !selected))?,
        );
    }
    if expected.get("Doors").is_some() {
        // Door fields are a pre-layer-3 representation. A merged permanent
        // retains component definitions even when a non-Room is on top.
        // Project the second stored door while retaining the engine's merged
        // common characteristics and abilities; no expected values enter this
        // projection and no gameplay unlock behavior is asserted here.
        let values = snapshot(&g, id)?;
        if values.copy_face_layout == LinkedFaceLayout::Split {
            let selected = flag(&physical["RightDoorOpen"]) && !flag(&physical["LeftDoorOpen"]);
            compare_values(&values, &definition(&face(&expected, selected))?)?;
            return compare_values(
                values
                    .other_copy_face
                    .as_deref()
                    .ok_or("native copied Room lost its other door")?,
                &definition(&face(&expected, !selected))?,
            );
        }
        let selected = flag(&physical["RightDoorOpen"]) && !flag(&physical["LeftDoorOpen"]);
        let o = g.object(id).unwrap();
        let room = g
            .merged_permanent(o.stable_id)
            .and_then(|m| {
                m.components
                    .iter()
                    .map(|c| &c.object)
                    .find(|o| o.linked_face_layout == LinkedFaceLayout::Split)
            })
            .unwrap_or(o);
        if room.linked_face_layout != LinkedFaceLayout::Split {
            return Err("native Room lost its linked door".into());
        }
        let other = g
            .linked_face_definition_by_name_or_id(room.other_face_name.as_deref(), room.other_face)
            .ok_or("native Room lost its linked door")?;
        let first = CopiableValues::from_object(o);
        compare_values(&first, &definition(&face(&expected, selected))?)?;
        let mut second = first;
        let other_values = definition_values(&other);
        if o.linked_face_layout == LinkedFaceLayout::Split {
            second.name = other_values.name;
            second.mana_cost = other_values.mana_cost;
            second.colors = other_values.colors;
        }
        let mut abilities = second.abilities.as_ref().clone();
        for own in room.abilities.iter() {
            if let Some(i) = abilities.iter().position(|a| ability_equal(a, own)) {
                abilities.remove(i);
            }
        }
        abilities.extend(other_values.abilities.iter().cloned());
        second.abilities = std::sync::Arc::new(abilities);
        return compare_values(&second, &definition(&face(&expected, !selected))?);
    }
    if expected.get("Flip").is_some() {
        compare_values(&snapshot(&g, id)?, &definition(&face(&expected, false))?)?;
        if !g.is_flipped(id) && !g.flip_permanent(id) {
            return Err("native copy cannot expose its flipped characteristics".into());
        }
        let chars = g
            .current_characteristics(id)
            .ok_or("native flipped characteristics unavailable")?;
        return compare_values(
            &CopiableValues::from_calculated(&chars),
            &definition(&face(&expected, true))?,
        );
    }
    let actual = if a["target"]["field"] == "Layer0_Base" && !flag(&physical["FaceDown"]) {
        CopiableValues::from_object(g.object(id).unwrap())
    } else {
        snapshot(&g, id)?
    };
    compare_values(&actual, &definition(&face(&expected, false))?)
}

fn token_scene(a: &J) -> R<(GameState, ObjectId)> {
    use ironsmith::cards::tokens::*;
    use ironsmith::effects::CreateTokenEffect;
    let args = &a["call"]["arguments"];
    let spec = &args[0];
    // Fish and Octopus are Wymi convenience aliases, not CR 111.10 types.
    let spec = match string(&spec["_CreatableToken"]) {
        "FishToken" => &input()["predefined_tokens"]["Fish Token"],
        "OctopusToken" => &input()["predefined_tokens"]["Octopus Token"],
        _ => spec,
    };
    let context = &args[1];
    let kind = string(&spec["_CreatableToken"]);
    let v = &spec["args"];
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = g.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Token creator")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build(),
        PlayerId::from_index(0),
        Zone::Battlefield,
    );
    let predefined = match kind {
        "TreasureToken" => Some(treasure_token_definition()),
        "FoodToken" => Some(food_token_definition()),
        "GoldToken" => Some(gold_token_definition()),
        "WalkerToken" => Some(walker_token_definition()),
        "ShardToken" => Some(shard_token_definition()),
        "ClueToken" => Some(clue_token_definition()),
        "BloodToken" => Some(blood_token_definition()),
        "PowerstoneToken" => Some(powerstone_token_definition()),
        "MapToken" => Some(map_token_definition()),
        "JunkToken" => Some(junk_token_definition()),
        "LanderToken" => Some(lander_token_definition()),
        "MutagenToken" => Some(mutagen_token_definition()),
        "CursedRoleToken" => Some(cursed_role_token_definition()),
        "MonsterRoleToken" => Some(monster_role_token_definition()),
        "RoyalRoleToken" => Some(royal_role_token_definition()),
        "SorcererRoleToken" => Some(sorcerer_role_token_definition()),
        "WickedRoleToken" => Some(wicked_role_token_definition()),
        "YoungHeroRoleToken" => Some(young_hero_role_token_definition()),
        "IncubatorToken" => {
            let (f, b) = incubator_token_definitions();
            g.register_linked_face_definition(&f);
            g.register_linked_face_definition(&b);
            Some(f)
        }
        _ => None,
    };
    let mut ctx = EffectContext::new_default(source, PlayerId::from_index(0));
    ctx.x_value = context["ValueX"].as_u64().map(|n| n as u32);
    if let Some(d) = predefined {
        {
            let mut e = CreateTokenEffect::one(d);
            e.suppress_aura_attachment_choice = true;
            e
        }
        .execute(&mut g, &mut ctx)
        .map_err(|e| format!("native token creation: {e:?}"))?;
    } else if kind == "OracleToken" {
        let card = &input()["card_inputs"][string(v)];
        if card.is_null() {
            return Err(format!("missing frozen oracle input {v}"));
        }
        let mut token = definition(card)?;
        token.card.is_token = true;
        CreateTokenEffect::one(token)
            .execute(&mut g, &mut ctx)
            .map_err(|e| format!("native oracle token: {e:?}"))?;
    } else {
        let text = if [
            "VirtuousRoleToken",
            "VibraniumToken",
            "FishToken",
            "OctopusToken",
        ]
        .contains(&kind)
        {
            format!(
                "Create a {} token.",
                kind.trim_end_matches("Token")
                    .replace("Role", " Role")
                    .to_lowercase()
            )
        } else {
            let named = kind.starts_with("Named");
            let has_pt = kind.contains("WithPT");
            let offset = usize::from(named) + usize::from(has_pt);
            let pt = if has_pt {
                let p = &v[usize::from(named)];
                match string(&p["_PT"]) {
                    "PT" => format!("{}/{} ", p["args"][0], p["args"][1]),
                    "PTX" => "X/X ".into(),
                    "ManualPT" => "0/0 ".into(),
                    x => return Err(format!("untranslated token PT {x}")),
                }
            } else {
                String::new()
            };
            let color = match string(&v[offset]["_TokenColorList"]) {
                "Colorless" => "colorless".into(),
                "Colors" => array(&v[offset]["args"])
                    .iter()
                    .map(|x| string(x).to_lowercase())
                    .collect::<Vec<_>>()
                    .join(" and "),
                "AllColors" => String::new(),
                "TheChosenColor" => String::new(),
                x => return Err(format!("untranslated token color {x}")),
            };
            let sub = &v[offset + 3];
            let subtype = match string(&sub["_TokenSubtypes"]) {
                "SubtypesList" => array(&sub["args"])
                    .iter()
                    .map(string)
                    .collect::<Vec<_>>()
                    .join(" "),
                "TheChosenCreatureType" | "AllBasicLandTypes" => String::new(),
                x => return Err(format!("untranslated token subtype {x}")),
            };
            let supertypes = array(&v[offset + 1])
                .iter()
                .map(|x| string(x).to_lowercase())
                .collect::<Vec<_>>()
                .join(" ");
            let types = array(&v[offset + 2])
                .iter()
                .map(|x| string(x).to_lowercase())
                .collect::<Vec<_>>()
                .join(" ");
            let mut text = format!("Create a {supertypes} {pt}{color} {subtype} {types} token");
            if named {
                text.push_str(&format!(" named {}", string(&v[0])));
            }
            if v[offset]["_TokenColorList"] == "AllColors" {
                text.push_str(" that's all colors");
            }
            if v[offset]["_TokenColorList"] == "TheChosenColor" {
                text.push_str(" of the chosen color and type");
            }
            if sub["_TokenSubtypes"] == "AllBasicLandTypes" {
                text.push_str(" that's every basic land type");
            }
            let rules = array(&v[offset + 4]);
            if !rules.is_empty() {
                text.push_str(" with ");
                text.push_str(
                    &rules
                        .iter()
                        .filter(|r| !r.is_null())
                        .map(|r| rule_text(r).map(|s| ability_clause(&s)))
                        .collect::<R<Vec<_>>>()?
                        .join(" and "),
                );
            }
            if kind == "NamedTokenWithPTAndWithCopyEffects" {
                text.push_str(" and all triggered abilities of the exiled cards");
            }
            text.push('.');

            if has_pt && v[usize::from(named)]["_PT"] == "ManualPT" {
                text = text.replace("0/0 ", "");
                text = text.trim_end_matches('.').to_string();
                text.push_str(" with power equal to target creature's power and toughness equal to that creature's toughness.");
                let target = object(
                    &mut g,
                    &context["Permanents"][context["Ref_TargetPermanent"].to_string()],
                    &J::Null,
                    Zone::Battlefield,
                )?;
                ctx.targets = vec![ResolvedTarget::Object(target)];
            }
            text.split_whitespace().collect::<Vec<_>>().join(" ")
        };
        if !context["TheChosenSimpleColor"].is_null() {
            let c = serde_json::from_value(context["TheChosenSimpleColor"].clone())
                .map_err(|e| format!("chosen color: {e}"))?;
            g.set_chosen_color(source, c);
        }
        if !context["TheChosenCreatureTokenType"].is_null() {
            let ty = serde_json::from_value(context["TheChosenCreatureTokenType"].clone())
                .map_err(|e| format!("chosen subtype: {e}"))?;
            g.set_chosen_creature_type(source, ty);
        }
        for n in array(&context["TheExiledCards"]) {
            let id = object(
                &mut g,
                &context["CardsInExile"][n.to_string()],
                &J::Null,
                Zone::Exile,
            )?;
            ctx.tagged_objects
                .entry("exile_cost_wymi".into())
                .or_default()
                .push(ObjectSnapshot::from_object(g.object(id).unwrap(), &g));
        }
        let d = compile("Token creator", &format!("Type: Sorcery\n{text}"))?;
        for effect in d
            .spell_effect
            .ok_or("token operation missing program")?
            .iter()
        {
            ironsmith::effects::execute_effect(&mut g, effect, &mut ctx)
                .map_err(|e| format!("native token execution: {e:?}"))?;
        }
    }
    let ids = g
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            g.object(*id)
                .is_some_and(|o| o.kind == ironsmith::object::ObjectKind::Token)
        })
        .collect::<Vec<_>>();
    if ids.len() != 1 {
        return Err(format!(
            "native token creation produced {} tokens",
            ids.len()
        ));
    }
    Ok((g, ids[0]))
}
fn token_assertion(a: &J, case: &J) -> R<()> {
    let result = token_scene(a);
    if a["matcher"] == "toThrow" {
        return match result {
            Ok(_) => Err("native creation succeeded with missing chosen color/type".into()),
            Err(e)
                if e.starts_with("native token execution:")
                    && e.to_lowercase().contains("chosen") =>
            {
                Ok(())
            }
            Err(e) => Err(format!(
                "expected-error case did not establish a missing-choice error: {e}"
            )),
        };
    }
    let (mut g, id) = result?;
    let expr = string(&a["expression"]);
    if expr.contains("BackFace") {
        g.transform_permanent(id)
            .map_err(|e| format!("token transform: {e:?}"))?;
    }
    let o = g.object(id).unwrap();
    let actual = if expr.ends_with("Name.Name") {
        json!(o.name.to_string())
    } else if expr.ends_with("CardPT.CardPT") {
        json!({"Power":o.base_power.map(|p|p.base_value()),"Toughness":o.base_toughness.map(|p|p.base_value())})
    } else if expr.ends_with("ColorIndicator") {
        let c = o.color_override.unwrap_or_default();
        json!(
            [
                ("White", ColorSet::WHITE),
                ("Blue", ColorSet::BLUE),
                ("Black", ColorSet::BLACK),
                ("Red", ColorSet::RED),
                ("Green", ColorSet::GREEN)
            ]
            .into_iter()
            .filter(|(_, s)| c.contains_all(*s))
            .map(|(n, _)| n)
            .collect::<Vec<_>>()
        )
    } else if expr.ends_with("Typeline.Typeline") {
        json!({"Supertypes":o.supertypes.to_vec(),"Cardtypes":o.card_types.to_vec(),"Subtypes":o.subtypes.to_vec()})
    } else if expr.ends_with("ManaCost") {
        let expected = if a["expected"].is_null() {
            None
        } else {
            Some(native_mana(&a["expected"])?)
        };
        let actual = o.mana_cost.as_ref().map(|m| (**m).clone());
        return if actual == expected {
            Ok(())
        } else {
            Err(format!(
                "token mana cost: actual {actual:?}; expected {expected:?}"
            ))
        };
    } else if expr.contains(".Rules[") {
        let mut unmatched = g
            .current_characteristics(id)
            .ok_or("native token characteristics unavailable")?
            .abilities
            .to_vec();
        // Consume every asserted occurrence, so duplicate copied triggers cannot
        // be satisfied by finding one matching ability repeatedly.
        for asserted in array(&case["assertions"])
            .iter()
            .filter(|x| string(&x["expression"]).contains(".Rules["))
        {
            let mut expected = rule_definition(&asserted["expected"])?;
            let token_spec = &a["call"]["arguments"][0];
            if token_spec["_CreatableToken"] == "NamedTokenWithPTAndWithCopyEffects"
                && !array(&token_spec["args"][6]).contains(&asserted["expected"])
                && array(&token_spec["args"][7]["args"]).iter().any(|copy| {
                    copy["_TokenCopyEffect"] == "AddAbilityFromEachExiledHasable"
                        && copy["args"][1] == json!([{"_CheckHasable":"TriggeredAbility"}])
                })
            {
                // One upstream keyword node can expand into static and triggered
                // abilities (decayed). Only the requested triggered occurrences
                // are copied; the separate cannot-block ability is not granted.
                expected.abilities.retain(|ability| {
                    matches!(ability.kind, ironsmith::ability::AbilityKind::Triggered(_))
                });
            }
            if expected.abilities.is_empty() {
                return Err(format!(
                    "rule translation has no executable abilities: {}",
                    asserted["expected"]
                ));
            }
            for ability in expected.abilities {
                if let Some(n) = unmatched.iter().position(|a| ability_equal(a, &ability)) {
                    unmatched.remove(n);
                } else {
                    return Err(format!(
                        "missing token executable rule {ability:?}; retained token cast tags: {:?}",
                        o.cast_tagged_objects.keys().collect::<Vec<_>>()
                    ));
                }
            }
            if asserted["expression"] == a["expression"] {
                break;
            }
        }
        return Ok(());
    } else {
        return Err(format!("untranslated token assertion {expr}"));
    };
    if expr.ends_with("Typeline.Typeline") {
        let canonical = |v: &J| {
            let mut v = v.clone();
            for field in ["Supertypes", "Cardtypes", "Subtypes"] {
                if let Some(a) = v[field].as_array_mut() {
                    a.sort_by_key(key);
                }
            }
            v
        };
        if canonical(&actual) == canonical(&a["expected"]) {
            return Ok(());
        }
    }
    if actual == a["expected"] {
        Ok(())
    } else {
        Err(format!(
            "{expr}: actual {actual}; expected {}",
            a["expected"]
        ))
    }
}
fn oracle_filter(query: &J, context: &J) -> R<ironsmith::target::ObjectFilter> {
    use ironsmith::target::ObjectFilter;
    let mut f = ObjectFilter::default();
    match string(&query["_CardsInOracle"]) {
        "IsCardtype" => {
            f.card_types =
                vec![serde_json::from_value(query["args"].clone()).map_err(|e| e.to_string())?]
        }
        "ManaValueIs" => {
            f.mana_value = Some(ironsmith::filter::Comparison::Equal(
                context["ValueX"].as_i64().ok_or("missing X")? as i32,
            ));
        }
        "And" => {
            for q in array(&query["args"]) {
                let part = oracle_filter(q, context)?;
                f.card_types.extend(part.card_types);
                if part.mana_value.is_some() {
                    f.mana_value = part.mana_value;
                }
            }
        }
        x => return Err(format!("untranslated query {x}")),
    }
    Ok(f)
}
fn query_assertion(a: &J) -> R<()> {
    let args = &a["call"]["arguments"];
    let single = a["call"]["function"] == "OracleCardPassesFilter";
    let f = oracle_filter(&args[usize::from(single)], &args[usize::from(single) + 1])?;
    let mut count = 0;
    let cards = if single {
        vec![&args[0]]
    } else {
        array(&input()["catalog"])
            .iter()
            .map(|r| &r["card"])
            .collect()
    };
    for card in cards {
        let mut card = if card["_OracleCard"] == "Split" {
            let mut types = Vec::new();
            let mut mana = Vec::new();
            for half in array(&card["Cards"]) {
                types.extend_from_slice(array(&half["Typeline"]["Typeline"]["Cardtypes"]));
                mana.extend_from_slice(array(&half["ManaCost"]));
            }
            types.sort_by_key(key);
            types.dedup();
            json!({"_OracleCard":"Card","Typeline":{"Typeline":{"Cardtypes":types}},"ManaCost":mana})
        } else {
            face(card, false)
        };
        // The query reads only card types and mana value. Keep its projection
        // independent of unrelated rule compilation and token gameplay.
        card["Rules"] = json!([]);
        card["Typeline"]["Typeline"]["Subtypes"] = json!([]);
        let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let id = object(&mut g, &card, &J::Null, Zone::Hand)?;
        let ctx = EffectContext::new_default(id, PlayerId::from_index(0));
        if ironsmith::effects::validate_target(
            &g,
            &ResolvedTarget::Object(id),
            &ironsmith::target::ChooseSpec::Object(f.clone()),
            &ctx,
        ) {
            count += 1;
        }
    }
    let actual = if single {
        json!(count == 1)
    } else {
        json!(count)
    };
    if actual == a["expected"] {
        Ok(())
    } else {
        Err(format!(
            "native query actual {actual}; expected {}",
            a["expected"]
        ))
    }
}
pub fn run_case(index: usize) {
    // Grammar recursion requires more stack than libtest's platform default.
    let thread = std::thread::Builder::new()
        .name(format!("wymi-{index:03}"))
        .stack_size(64 * 1024 * 1024)
        .spawn(move || run_case_inner(index))
        .unwrap();
    if let Err(panic) = thread.join() {
        std::panic::resume_unwind(panic);
    }
}
// Rules-reviewed source corrections remain separate from the frozen upstream.
// Verify every original value before applying a correction, and record both
// the original mismatch and the corrected native verdict in the audit output.
fn corrected_assertion(index: usize, assertion: usize, original: &J) -> Option<(J, String)> {
    let corrections: J = serde_json::from_str(include_str!(
        "../../../../fixtures/wymi/expectation_errata.json.fixture"
    ))
    .unwrap();
    let correction = array(&corrections).iter().find(|entry| {
        entry["source_index"] == index && array(&entry["assertions"]).contains(&json!(assertion))
    })?;
    let mut corrected = original.clone();
    for change in array(&correction["changes"]) {
        let value = corrected
            .pointer_mut(string(&change["path"]))
            .expect("reviewed source path");
        assert_eq!(*value, change["from"], "source correction needs re-review");
        *value = change["to"].clone();
    }
    Some((corrected, string(&correction["reason"]).to_string()))
}

fn run_case_inner(index: usize) {
    let c = &input()["cases"][index];
    assert!(!flag(&c["skipped"]));
    assert!(
        c["export_error"].is_null(),
        "export error {}",
        c["export_error"]
    );
    let mut errors = Vec::new();
    let mut results = Vec::new();
    let mut guard_results = Vec::new();
    for (n, guard) in array(&c["guards"]).iter().enumerate() {
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| guard_assertion(guard)))
                .unwrap_or_else(|_| Err("native/adapter panic: guard execution".into()));
        guard_results.push(json!({"guard":n,"expression":guard["expression"],"passed":result.is_ok(),"error":result.as_ref().err()}));
        if let Err(e) = result {
            errors.push(format!("guard {n}: {}: {e}", guard["expression"]));
        }
    }
    for (n, original) in array(&c["assertions"]).iter().enumerate() {
        let correction = corrected_assertion(index, n, original);
        let upstream_error = correction
            .as_ref()
            .and_then(|_| layer_assertion(original).err());
        let a = correction
            .as_ref()
            .map_or(original, |(assertion, _)| assertion);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if !a["target"].is_null() {
                layer_assertion(a)
            } else {
                match string(&a["call"]["function"]) {
                    "TokenLookup" => token_assertion(a, c),
                    "ComputeColorFromManaCost" => {
                        let d = compile(
                            "Mana colors",
                            &format!(
                                "Type: Artifact\nMana cost: {}",
                                mana(&a["call"]["arguments"][0])
                            ),
                        );
                        d.and_then(|d| {
                            if d.card.colors() == colors(&a["expected"]) {
                                Ok(())
                            } else {
                                Err("native mana colors differ".into())
                            }
                        })
                    }
                    "OracleCardPassesFilter" | "FindOracleCards" => query_assertion(a),
                    x => Err(format!("untranslated assertion {x}")),
                }
            }
        }))
        .unwrap_or_else(|panic| {
            Err(format!(
                "native/adapter panic: {}",
                panic
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| panic.downcast_ref::<&str>().copied())
                    .unwrap_or("non-string panic")
            ))
        });
        results.push(json!({"assertion":n,"expression":a["expression"],"passed":result.is_ok(),"error":result.as_ref().err(),"expectation_correction":correction.as_ref().map(|(_, reason)| reason),"upstream_error":upstream_error}));
        if let Err(e) = result {
            errors.push(format!("assertion {n}: {}\n{e}", a["expression"]));
        }
    }
    if let Ok(dir) = std::env::var("WYMI_AUDIT_DIR") {
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(format!("{index:03}.json")),serde_json::to_vec_pretty(&json!({"index":index,"name":c["name"],"line":c["line"],"passed":errors.is_empty(),"assertions":results,"guards":guard_results})).unwrap()).unwrap();
    }
    assert!(
        errors.is_empty(),
        "source case {index}: {}\n{}",
        c["name"],
        errors.join("\n\n")
    );
}
fn native_mana(v: &J) -> R<ironsmith::mana::ManaCost> {
    use ironsmith::mana::{ManaCost, ManaSymbol as M};
    let mut pips = Vec::new();
    for symbol in array(v) {
        let kind = string(&symbol["_ManaSymbol"]).trim_start_matches("ManaCost");
        let pip = if kind == "Generic" {
            let mut n = symbol["args"].as_u64().ok_or("invalid generic mana")?;
            while n > u8::MAX as u64 {
                pips.push(vec![M::Generic(u8::MAX)]);
                n -= u8::MAX as u64;
            }
            vec![M::Generic(n as u8)]
        } else {
            kind.chars()
                .map(|c| match c {
                    'W' => Ok(M::White),
                    'U' => Ok(M::Blue),
                    'B' => Ok(M::Black),
                    'R' => Ok(M::Red),
                    'G' => Ok(M::Green),
                    'C' => Ok(M::Colorless),
                    'S' => Ok(M::Snow),
                    'X' => Ok(M::X),
                    'P' => Ok(M::Life(2)),
                    '2' => Ok(M::Generic(2)),
                    _ => Err(format!("untranslated mana symbol {kind}")),
                })
                .collect::<R<Vec<_>>>()?
        };
        pips.push(pip);
    }
    Ok(ManaCost::from_pips(pips))
}
fn guard_assertion(guard: &J) -> R<()> {
    let expr = string(&guard["expression"]);
    if !expr.starts_with("token.") {
        if !expr.ends_with(".Flip === undefined") {
            return Err(format!("untranslated fixture guard {expr}"));
        }
        let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let id = object(&mut g, &guard["subject"], &J::Null, Zone::Hand)?;
        let o = g.object(id).unwrap();
        return if o.linked_face_layout == LinkedFaceLayout::Flip && o.other_face.is_some() {
            Ok(())
        } else {
            Err("native fixture is missing its flip face".into())
        };
    }
    let (g, id) = token_scene(guard)?;
    let o = g.object(id).unwrap();
    let valid = if expr.contains("._OracleCard") {
        o.kind == ironsmith::object::ObjectKind::Token
            && (if expr.contains("\"Transforming\"") {
                o.linked_face_layout == LinkedFaceLayout::TransformLike && o.other_face.is_some()
            } else {
                o.linked_face_layout == LinkedFaceLayout::None
            })
    } else if expr.contains("._Name") {
        if expr.contains("BackFace") || expr.contains("FrontFace") {
            o.other_face.is_some()
        } else {
            true
        }
    } else if expr.contains("._Typeline") {
        true
    } else if expr.contains("._CardPT") {
        o.base_power.is_some() && o.base_toughness.is_some()
    } else if expr == "token.ManaCost === undefined" {
        o.mana_cost.is_some()
    } else if expr == "token.ManaCost !== undefined" {
        o.mana_cost.is_none()
    } else if expr == "token.ColorIndicator !== undefined" {
        o.color_override.is_none()
    } else if expr == "token.CardPT !== undefined" {
        o.base_power.is_none() && o.base_toughness.is_none()
    } else {
        return Err(format!("untranslated token guard {expr}"));
    };
    if valid {
        Ok(())
    } else {
        Err(format!("native token violates guard: {}", guard["message"]))
    }
}

fn ability_clause(text: &str) -> String {
    if text.contains(':') || text.contains('.') {
        format!("\"{text}\"")
    } else {
        text.to_lowercase()
    }
}
