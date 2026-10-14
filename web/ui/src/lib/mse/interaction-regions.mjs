// Hit targets come from the same measured characters that the canvas paints.
// Never infer regions from a frame family, artwork scan, or card dimensions.
const normalize = value => String(value).normalize('NFKD').replace(/\p{M}/gu,'').toLowerCase().replace(/[^\p{L}\p{N}]/gu,'');
export function mseInteractionRegions(result, rulesView) {
  if (!result?.context || !rulesView) return [];
  const context=result.context, fields=[];
  for (const [key, value] of context.textMeasurements) {
    const layout=value.result;
    if (!layout?.measurement?.runs || !layout.lines) continue;
    const dot=key.indexOf('.'),group=key.slice(0,dot),name=key.slice(dot+1);
    // A keyword such as Flying may also appear in the card's name or flavor.
    // Rules interactions must never attach to those matching decorative fields.
    if(/^(?:name|casting_cost|super_type|sub_type|flavor_text|artist|copyright|set_number)(?:_|$)/.test(name))continue;
    if (!context.fieldVisible(name,group)) continue;
    const indexed=[];
    for (const run of layout.measurement.runs) {
      if(run.soft)continue;
      [...run.text].forEach((char,index)=>{
        for(const letter of normalize(char))indexed.push({letter,index:run.characterStart+index});
      });
    }
    fields.push({layout, indexed, text:indexed.map(c=>c.letter).join(''),
      left:Number(context.property(name,'left',group)),top:Number(context.property(name,'top',group)),
      height:Number(context.property(name,'height',group)),angle:Number(context.property(name,'angle',group))||0});
  }
  const regions=[];
  rulesView.lines.forEach((text,lineIndex)=>{
    let needle=normalize(text);
    if(!needle)return;
    let field=fields.find(f=>f.text.includes(needle));
    // Planeswalker templates paint the loyalty cost in its own field.
    if(!field){needle=normalize(text.replace(/^\s*[+−-]?(?:\d+|X)\s*:\s*/i,''));field=needle&&fields.find(f=>f.text.includes(needle));}
    if(!field)return;
    const offset=field.text.indexOf(needle),group=rulesView.manaGroups?.get(lineIndex);
    const targets=[];
    if(rulesView.ranges){
      for(const range of rulesView.ranges.get(lineIndex)||[])targets.push({
        start:offset+normalize(text.slice(0,range.start)).length,length:normalize(text.slice(range.start,range.end)).length,
        helper:range.helper,actions:[],
      });
    }else if(group){
      let prefix=group.prefix;
      for(const part of group.parts){
        if(part.type==='element')targets.push({start:offset+normalize(prefix).length,length:normalize(part.value).length,
          option:part.value,actions:group.options.find(option=>option.output===part.value)?.actions||[]});
        prefix+=part.value;
      }
    }else targets.push({start:offset,length:needle.length,actions:rulesView.actions.get(lineIndex)||[]});
    for(const target of targets){
      const start=field.indexed[target.start]?.index,end=(field.indexed[target.start+target.length-1]?.index??-1)+1;
      if(start===undefined||end<=start)continue;
      for(const line of field.layout.lines){
        const a=Math.max(start,line.start),b=Math.min(end,line.end);
        if(a>=b||line.top>=field.height||line.top+line.height<=0)continue;
        const stretch=field.layout.stretch??1,x=line.positions[a-line.start]*stretch;
        const width=(line.positions[b-line.start]-line.positions[a-line.start])*stretch;
        if(!(width>0&&line.height>0))continue;
        const rad=-field.angle*Math.PI/180;
        regions.push({lineIndex,option:target.option,actions:target.actions,helper:target.helper,
          left:field.left+x*Math.cos(rad)-line.top*Math.sin(rad),
          top:field.top+x*Math.sin(rad)+line.top*Math.cos(rad),width,height:line.height,angle:-field.angle});
      }
    }
  });
  return regions;
}

export function mseActionsCovered(regions,rulesView) {
  if(!rulesView)return true;
  return [...rulesView.actions].every(([lineIndex,actions])=>!actions.length||
    (rulesView.manaGroups?.has(lineIndex)
      ?rulesView.manaGroups.get(lineIndex).options.every(option=>!option.actions.length||regions.some(r=>r.lineIndex===lineIndex&&r.option===option.output))
      :regions.some(r=>r.lineIndex===lineIndex)));
}
