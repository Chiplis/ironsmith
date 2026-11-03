// Shape the existing single-face draft grammar for the standalone compiler.
// Token attachments still use the full custom-card compiler until its artifact
// interface can transport them without losing their definitions.
export function showcaseArtifactInput(draft) {
 if(draft.layout!=='single'||draft.token||draft.faces.length!==1||draft.customTokenFaces?.length)return null;
 const face=draft.faces[0],lines=[];
 if(face.manaCost)lines.push(`Mana cost: ${face.manaCost}`);
 lines.push(`Type: ${[...(face.supertypes||[]),...(face.cardTypes||[]),...(face.subtypes?.length?['—',...face.subtypes]:[])].join(' ')}`);
 if(face.power!=null&&face.toughness!=null)lines.push(`Power/Toughness: ${face.power}/${face.toughness}`);
 if(face.loyalty!=null)lines.push(`Loyalty: ${face.loyalty}`);
 if(face.defense!=null)lines.push(`Defense: ${face.defense}`);
 if(face.colorIndicator?.length)lines.push(`Color indicator: ${face.colorIndicator.join(', ')}`);
 return {name:face.name,text:[...lines,face.oracleText].join('\n'),localId:1,
  enforceFilterClaims:draft.enforceFilterClaims!==false,enforceLineClaims:draft.enforceLineClaims!==false,
  customKeywordIdentities:face.customKeywordIdentities||[],customKeywordPayments:face.customKeywordPayments||[],
  customKeywordCounterDefinitions:face.customKeywordCounterDefinitions||[],customSubtypes:draft.customSubtypes||[],suppliedCardDefinitions:draft.suppliedCardDefinitions||[]};
}
export function showcaseArtifactSource(draft,input) {
 return {canonicalName:input.name,replaceExisting:true,group:{kind:'single',name:input.name,block:input.text}};
}
