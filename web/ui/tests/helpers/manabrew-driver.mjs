// Answers Manabrew prompts (as the shim produces them) with simple legal
// choices: the flow a Manabrew player's clicks take, minus the pixels.
export function answerFor(prompt) {
  const input = prompt?.input;
  if (!input) return null;
  switch (input.type) {
    case 'mulligan': return { type: 'mulligan', output: { type: 'mulliganDecision', keep: true } };
    case 'mulliganPutBack': return { type: 'mulliganPutBack', output: { type: 'putBackDecision', cardIds: (input.handCardIds || []).slice(0, input.count ?? 0) } };
    case 'chooseAction': {
      const actions = input.actions || [];
      const land = actions.find(a => a.type === 'cast' && /land/i.test(`${a.mode} ${a.label}`));
      if (land) return { type: 'chooseAction', output: { type: 'act', actionId: land.id } };
      return { type: 'chooseAction', output: { type: 'pass', exhaustStack: false } };
    }
    case 'chooseAttackers': return { type: 'chooseAttackers', output: { type: 'declareAttackers', assignments: [] } };
    case 'chooseBlockers': return { type: 'chooseBlockers', output: { type: 'declareBlockers', assignments: [] } };
    case 'chooseFromSelection': return { type: 'chooseFromSelection', output: { type: 'selectionDecision', chosenIndices: [] } };
    case 'chooseCards': {
      // A cleanup discard must discard; a permission-reveal prompt may reveal nothing.
      const need = Number(input.minSelections ?? input.min ?? 0);
      const pool = (input.cards || []).map(card => card.id);
      return { type: 'chooseCards', output: { type: 'chooseCardsDecision', chosenCardIds: pool.slice(0, need) } };
    }
    default: return null;
  }
}
