// The build-time random-table pool (scripts/build-random-card-pool.mjs): every
// compiled card's classifyCard result, dictionary-coded to keep it small.
import { PERMANENT_TYPES, SPELL_TYPES } from "./random-game.js";

export const RANDOM_CARD_POOL_FORMAT = "ironsmith-random-card-pool-v1";

/** The classified cards of a pool file, shaped exactly like classifyCard's output. */
export function decodeRandomCardPool(pool) {
  if (pool?.format !== RANDOM_CARD_POOL_FORMAT || !Array.isArray(pool.cards)) return null;
  const { types = [], supertypes = [], colors = [] } = pool;
  return pool.cards.map(([name, typeCodes, supertypeCodes, colorCodes, manaValue, singleFaced, score]) => {
    const cardTypes = typeCodes.map((code) => types[code]);
    const cardSupertypes = supertypeCodes.map((code) => supertypes[code]);
    return {
      name,
      types: cardTypes,
      supertypes: cardSupertypes,
      colors: colorCodes.map((code) => colors[code]),
      manaValue,
      permanent: cardTypes.some((type) => PERMANENT_TYPES.includes(type))
        && !cardTypes.some((type) => SPELL_TYPES.includes(type)),
      legendary: cardSupertypes.includes("Legendary"),
      basic: cardSupertypes.includes("Basic"),
      singleFaced: singleFaced === 1,
      score,
    };
  });
}
