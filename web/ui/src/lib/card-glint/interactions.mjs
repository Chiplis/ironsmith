const normalize = value => String(value).normalize('NFKD').replace(/\p{M}/gu,'').toLowerCase().replace(/[^\p{L}\p{N}]/gu,'');

export function cardGlintRegions(result, rulesView) {
  if (!result || !rulesView) return [];
  const {api,face,canvas} = result;
  // Only use the painter's own measured layout. Unknown special layouts retain
  // the accessible DOM fallback when their available actions cannot be mapped.
  if (face.faces?.length || face.frameTemplate || api.hasExtraRules(face) || face.layout === 'prepare') return [];
  const context = canvas.getContext('2d');
  context.save(); context.resetTransform();
  try {
    let layouts;
    const kind = api.faceFrame(face);
    if (kind.startsWith('planeswalker')) layouts = api.planeswalkerRuleRows(context,face).map(row => row.layout);
    else if (kind === 'normal') layouts = [api.isRetro(face) ? api.retroTextLayout(context,face) : api.ordinaryTextLayout(context,face)];
    else return [];
    return measuredRegions(layouts.filter(Boolean),context,api.rulesFont,rulesView);
  } finally {context.restore();}
}

export function measuredRegions(layouts, context, rulesFont, rulesView) {
  const chars = [];
  let lineId = 0;
  for (const layout of layouts) for (const line of layout.lines) {
    lineId++;
    if (line.role !== 'rules') continue;
    let x = line.x ?? layout.x;
    for (const run of line.runs) {
      context.font = rulesFont(run.size,run.italic);
      const letters = [...run.text];
      let prefix = '';
      for (const char of letters) {
        const start = run.symbol ? 0 : context.measureText(prefix).width;
        prefix += char;
        const end = run.symbol ? run.width : context.measureText(prefix).width;
        for (const letter of normalize(char)) chars.push({letter,lineId,left:x+start,top:line.baseline-run.ascent,
          width:end-start,height:run.ascent+run.descent});
      }
      x += run.width;
    }
  }
  const text = chars.map(char => char.letter).join(''), regions = [];
  let cursor = 0;
  rulesView.lines.forEach((line,lineIndex) => {
    let needle = normalize(line), offset = needle ? text.indexOf(needle,cursor) : -1;
    if (offset < 0) {needle = normalize(line.replace(/^\s*[+−-]?(?:\d+|X)\s*:\s*/i,'')); offset = needle ? text.indexOf(needle,cursor) : -1;}
    if (offset < 0) return;
    cursor = offset + needle.length;
    const group = rulesView.manaGroups?.get(lineIndex), targets = [];
    if (rulesView.ranges) {
      for (const range of rulesView.ranges.get(lineIndex) || []) targets.push({start:offset+normalize(line.slice(0,range.start)).length,
        length:normalize(line.slice(range.start,range.end)).length,helper:range.helper,actions:[]});
    } else if (group) {
      let prefix = group.prefix;
      for (const part of group.parts) {
        if (part.type === 'element') targets.push({start:offset+normalize(prefix).length,length:normalize(part.value).length,
          option:part.value,actions:group.options.find(option => option.output === part.value)?.actions || []});
        prefix += part.value;
      }
    } else targets.push({start:offset,length:needle.length,actions:rulesView.actions.get(lineIndex) || []});
    for (const target of targets) {
      const byLine = new Map();
      for (const char of chars.slice(target.start,target.start+target.length)) {
        const region = byLine.get(char.lineId);
        if (region) {
          const right = Math.max(region.left+region.width,char.left+char.width), bottom = Math.max(region.top+region.height,char.top+char.height);
          region.left = Math.min(region.left,char.left); region.top = Math.min(region.top,char.top);
          region.width = right-region.left; region.height = bottom-region.top;
        } else byLine.set(char.lineId,{...char});
      }
      for (const region of byLine.values()) regions.push({...region,lineIndex,actions:target.actions,option:target.option,helper:target.helper,angle:0});
    }
  });
  return regions;
}

export function cardGlintActionsCovered(regions,rulesView) {
  if (!rulesView) return true;
  return [...rulesView.actions].every(([lineIndex,actions]) => !actions.length ||
    (rulesView.manaGroups?.has(lineIndex)
      ? rulesView.manaGroups.get(lineIndex).options.every(option => !option.actions.length || regions.some(r => r.lineIndex===lineIndex && r.option===option.output))
      : regions.some(r => r.lineIndex===lineIndex)));
}
