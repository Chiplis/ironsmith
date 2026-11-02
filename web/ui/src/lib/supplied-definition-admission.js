// Standalone source descriptors cannot silently discard executable dependencies.
export function validateStandaloneSuppliedDefinitionMetadata(definition) {
 const present=value=>value!==undefined&&value!==null;
 for(const field of ['token','isToken','is_token']) {
  if(present(definition[field])&&definition[field]!==false)throw new Error(`Supplied real-card definition cannot carry ${field}.`);
 }
 if(String(definition.text||'').split('\n').some(line=>/^\s*Type:\s*Token(?:\s|$)/i.test(line)))throw new Error('Supplied real-card definition cannot have a Token type header.');
 for(const field of ['customTokens','custom_tokens','customTokenFaces','custom_token_faces',
  'customKeywordCounterDefinitions','custom_keyword_counter_definitions','abilityCounterDefinitions','ability_counter_definitions',
  'faces','sourceFaces','source_faces','executableFaces','executable_faces']) {
  const value=definition[field];
  if(present(value)&&(!Array.isArray(value)||value.length))throw new Error(`Standalone supplied definition cannot drop unsupported ${field}; provide a complete typed dependency graph.`);
 }
 for(const field of ['otherFaceId','other_face_id','otherFaceName','other_face_name','meldPartnerName','meld_partner_name','linkedFaceDefinition','linked_face_definition']) {
  if(present(definition[field])&&definition[field]!=='')throw new Error(`Standalone supplied definition cannot drop linked-face metadata ${field}.`);
 }
 for(const field of ['linkedFaceLayout','linked_face_layout','sourceLayout','source_layout','sourceShape','source_shape','layout']) {
  const value=definition[field];
  if(present(value)&&(typeof value!=='string'||!['','none','normal','single'].includes(value.trim().toLowerCase())))throw new Error(`Standalone supplied definition cannot represent ${field}.`);
 }
}
