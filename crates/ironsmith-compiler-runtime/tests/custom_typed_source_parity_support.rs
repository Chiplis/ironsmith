// native-test-suite: helper-only
//! Full typed source parity; executor allocations are not serialized semantics.
use ironsmith::CardDefinition;
use serde_json::Value;
fn typed(definition:&CardDefinition)->Result<Value,String>{
    let wire=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(definition.clone())
        .map_err(|error|format!("typed source encoding: {error:?}"))?;
    serde_json::to_value(wire).map_err(|error|error.to_string())
}
fn differences(path:&str,left:&Value,right:&Value,out:&mut Vec<String>){
    if left==right || out.len()>=8{return;}
    match (left,right){
        (Value::Object(left),Value::Object(right))=>{
            let keys:std::collections::BTreeSet<_>=left.keys().chain(right.keys()).collect();
            for key in keys{
                match(left.get(key),right.get(key)){
                    (Some(a),Some(b))=>differences(&format!("{path}/{key}"),a,b,out),
                    _=>out.push(format!("{path}/{key}: field presence differs")),
                }
                if out.len()>=8{break;}
            }
        },
        (Value::Array(left),Value::Array(right))=>{
            if left.len()!=right.len(){out.push(format!("{path}: array length {} != {}",left.len(),right.len()));}
            for (index,(a,b)) in left.iter().zip(right).enumerate(){
                differences(&format!("{path}/{index}"),a,b,out);if out.len()>=8{break;}
            }
        },
        _=>out.push(format!("{path}: native={} artifact={}",left.to_string().chars().take(160).collect::<String>(),right.to_string().chars().take(160).collect::<String>())),
    }
}
/// Preserve every typed program/cost/target/card identity/label/metadata field.
/// Only nonsemantic executor allocation identity is absent from the encoder.
pub fn compare(native:&CardDefinition,artifact:&CardDefinition)->Result<(),String>{
    let native=typed(native)?;let artifact=typed(artifact)?;
    if native==artifact{return Ok(());}
    let mut deltas=Vec::new();differences("$",&native,&artifact,&mut deltas);
    Err(format!("complete typed native/artifact source differs:\n{}",deltas.join("\n")))
}
