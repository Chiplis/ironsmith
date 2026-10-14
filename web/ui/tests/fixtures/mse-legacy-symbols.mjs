// Same drawing in old 0..500 coordinates and modern 0..1 coordinates.
export const legacySymbolCases=['square','curve','group'].map(id=>{
  const shape=(scale,curve=false)=>'part:\n\ttype: shape\n\tcombine: merge\n'+
    [[0.2,0.2],[0.2,0.8],[0.8,0.8],[0.8,0.2]].map(([x,y],i)=>
      `\tpoint:\n\t\tposition: (${x*scale},${y*scale})\n`+
      (curve&&i===0?`\t\tline after: curve\n\t\thandle after: (${-0.2*scale},${0.1*scale})\n`:'')+
      (curve&&i===1?`\t\thandle before: (${-0.2*scale},${-0.1*scale})\n`:'')).join('');
  const source=scale=>id==='group'?'part:\n\ttype: group\n'+shape(scale,true).split('\n').filter(Boolean).map(line=>'\t'+line).join('\n')+'\n':shape(scale,id==='curve');
  return {id,legacy:source(500),modern:'mse version: 0.3.5\n'+source(1)};
});
