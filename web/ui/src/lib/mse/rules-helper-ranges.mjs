import {splitTextWithMtgKeywordRules} from '../mtg-keywords.js';

// Source ranges remain relative to displayed rules, then resolve through MSE's
// measured characters just like ability buttons. Mana choices keep activation.
export function mseRulesHelperView(rulesView,locale='en',keywordHelpers=true,customMechanics=[]) {
  const ranges=new Map();
  rulesView.lines.forEach((line,index)=>{
    const found=[];let offset=0;
    if(keywordHelpers && !rulesView.manaGroups?.has(index))for(const segment of splitTextWithMtgKeywordRules(line,locale,customMechanics)){
      if(segment.type==='keyword')found.push({start:offset,end:offset+segment.text.length,helper:{type:'keyword',text:segment.text,context:segment.context,rule:segment.rule}});
      offset+=segment.text.length;
    }
    if(!rulesView.manaGroups?.has(index)){
      const costEnd=line.indexOf(':');
      for(const match of line.matchAll(/\{([^{}]+)\}/g)){
       if((rulesView.actions.get(index)||[]).length && costEnd>=0 && match.index>costEnd)continue;
       found.push({start:match.index,end:match.index+match[0].length,helper:{type:'symbol',symbol:match[1]}});
      }
    }
    ranges.set(index,found);
  });
  return {lines:rulesView.lines,actions:new Map(),ranges};
}
