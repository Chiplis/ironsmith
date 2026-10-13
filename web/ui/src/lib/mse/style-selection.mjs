// Automatic defaults are a presentation policy, not a list of supported styles.
// A requested package always wins, including packages outside the bundled pack.
export function selectMseStyle(snapshot,{style,defaultStyle='auto'}={}){
  if(style&&style!=='auto')return style;
  if(defaultStyle&&defaultStyle!=='auto')return defaultStyle;
  const faces=snapshot?.card_faces;
  const front=Array.isArray(faces)&&faces.length?faces[0]:snapshot;
  const type=String(front?.type_line??front?.type_line_display??'');
  if(snapshot?.layout==='adventure')return 'magic-m15-adventure.mse-style';
  if(snapshot?.layout==='split'){
    const aftermath=snapshot?.keywords?.includes('Aftermath')||faces?.some(face=>/\bAftermath\b/.test(face.oracle_text??''));
    return aftermath?'magic-m15-aftermath.mse-style':'magic-m15-split-fusable.mse-style';
  }
  if(/\bBattle\b/.test(type))return 'magic-m15-mainframe-battles.mse-style';
  if(/\bPlaneswalker\b/.test(type))return 'magic-m15-planeswalker.mse-style';
  if(/\bSaga\b/.test(type))return 'magic-m15-saga.mse-style';
  return 'magic-m15-altered.mse-style';
}
