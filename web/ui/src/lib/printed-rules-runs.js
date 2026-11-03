import {MTG_KEYWORD_RULES} from './mtg-keyword-rules.generated.js';

// Keyword abilities can introduce a cost with the same dash as an italic
// ability or flavor word (e.g. "Equip legendary creature — Pay 3 life.").
const keywordAbilities = new Set(MTG_KEYWORD_RULES
  .filter(rule => rule.kind === 'keywordAbility')
  .map(rule => rule.title.toLowerCase()));

function isPrintedLabel(label) {
  if (/^[IVXLCDM]+$/.test(label) || /^(?:to solve|solved|choose\b.*)$/i.test(label)) return false;
  const words = label.toLowerCase().split(/\s+/);
  while (words.length) {
    if (keywordAbilities.has(words.join(' '))) return false;
    words.pop();
  }
  return true;
}

// Work on whole paragraphs so a mode bullet stays upright and a dash at the
// end of a "Choose one" line cannot consume the next line as its body.
export function printedRulesRuns(text, {labels = true} = {}) {
  const runs = [];
  for (const [index, line] of text.split('\n').entries()) {
    if (index) runs.push({text:'\n'});
    const heading = labels && line.match(/^([ \t]*(?:•[ \t]+)?)([\p{L}][\p{L}\p{M}\d ’'!?….-]*?)[ \t]+[—–][ \t]+(?=\S)/u);
    let body = line;
    if (heading && isPrintedLabel(heading[2])) {
      if (heading[1]) runs.push({text:heading[1]});
      runs.push({text:heading[2], italic:true});
      body = line.slice(heading[1].length + heading[2].length);
    }
    for (const part of body.split(/(\([^()]*\))/).filter(Boolean)) {
      runs.push(part.startsWith('(') && part.endsWith(')') ? {text:part, italic:true} : {text:part});
    }
  }
  return runs;
}
