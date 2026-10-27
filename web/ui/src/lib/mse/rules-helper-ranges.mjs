import {splitTextWithMtgKeywordRules} from '../mtg-keywords.js';

// Source ranges remain relative to displayed rules, then resolve through MSE's
// measured characters just like ability buttons. Mana choices keep activation.
export function mseRulesHelperView(rulesView,locale='en',keywordHelpers=true) {
  const ranges=new Map();
  rulesView.lines.forEach((line,index)=>{
    const found=[];let offset=0;
    if(keywordHelpers && !(rulesView.actions.get(index)||[]).length)for(const segment of splitTextWithMtgKeywordRules(line,locale)){
      if(segment.type==='keyword')found.push({start:offset,end:offset+segment.text.length,helper:{type:'keyword',text:segment.text,context:segment.context,rule:segment.rule}});
      offset+=segment.text.length;
    }
    if(!(rulesView.actions.get(index)||[]).length){
      for(const match of line.matchAll(/\{([^{}]+)\}/g))found.push({start:match.index,end:match.index+match[0].length,helper:{type:'symbol',symbol:match[1]}});
    }
    ranges.set(index,found);
  });
  return {lines:rulesView.lines,actions:new Map(),ranges};
}
