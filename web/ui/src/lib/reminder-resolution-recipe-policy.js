// Explicit multiplicity is part of an authored terminal-resolution recipe.
// No keyword spelling or program-text equality supplies idempotence.
const header=/^(If this spell was cast from your (?:hand|graveyard), instead of putting it into your graveyard as it resolves, exile it)\s+and\s+/i;
export function normalizeResolutionProgramMultiplicity(value,reminder){
 if(value==null)return null;
 if(typeof value!=='object'||Array.isArray(value)||Object.keys(value).some(key=>!['mode','key'].includes(key)))throw Error('Resolution recipe multiplicity must declare mode and an optional authored key.');
 if(!header.test(String(reminder)))throw Error('Resolution recipe multiplicity requires a complete ordinary terminal destination program.');
 if(value.mode==='per-instance'){
  if(Object.hasOwn(value,'key'))throw Error('Per-instance resolution recipes do not declare a shared key.');
  return null;
 }
 if(value.mode!=='once-per-declared-key'||typeof value.key!=='string')throw Error('Resolution recipe multiplicity must be per-instance or once-per-declared-key.');
 const key=value.key.normalize('NFKC').replace(/\s+/g,' ').trim().toLowerCase();
 if(!/^[a-z][a-z0-9 -]{0,95}$/.test(key))throw Error('A shared resolution recipe requires a plain authored key.');
 return {mode:'once-per-declared-key',key};
}
export function bindResolutionProgramMultiplicity(expansion,policy){
 if(!policy||policy.mode==='per-instance')return expansion;
 if(!header.test(String(expansion)))throw Error('Resolution recipe expansion lost its destination header.');
 return String(expansion).replace(header,(_,lead)=>`${lead} once for the resolution recipe named "${policy.key}" and `);
}
