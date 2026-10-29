import { communityCustomCards, normalizeCommunityFormat } from './community-format.js';
import { prepareCustomCard } from './custom-mechanics.js';
import { normalizeDraftForApi } from './custom-card-draft.js';

// Custom creation registers definitions in the shared registry. A savepoint
// restores all gameplay state afterwards, leaving those definitions available
// to normal deck loading, opening hands, and deterministic replay.
export async function installCommunityCards(game, value) {
  if (!value) return;
  const format = normalizeCommunityFormat(value);
  const saved = game.createRuntimeSavepoint();
  try {
    for (const card of communityCustomCards(format)) {
      const prepared = await prepareCustomCard(normalizeDraftForApi(card.draft), input => game.previewCustomCard(input));
      if (!prepared.preview?.canCreate) throw new Error(`Cannot compile ${card.draft.faces[0].name}`);
      await game.createCustomCard({ draft: prepared.input, playerIndex: 0, zoneName: 'hand', skipTriggers: true });
    }
  } finally {
    game.restoreRuntimeSavepoint(saved);
    game.releaseRuntimeSavepoint(saved);
  }
}
