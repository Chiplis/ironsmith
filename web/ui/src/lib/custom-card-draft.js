export function normalizeDraftForApi(draft) {
  return {
    customMechanics: structuredClone(draft.customMechanics || []),
    layout: draft.layout || "single",
    hasFuse: draft.hasFuse === true,
    faces: draft.faces.map((face) => ({
      ...(face.displayOracleText != null ? { displayOracleText: face.displayOracleText } : {}),
      ...(face.customKeywordIdentities?.length ? { customKeywordIdentities: [...face.customKeywordIdentities] } : {}),
      name: String(face.name || "").trim(),
      manaCost: String(face.manaCost || "").trim() || null,
      colorIndicator: face.colorIndicator || [],
      supertypes: face.supertypes || [],
      cardTypes: face.cardTypes || [],
      subtypes: face.subtypes || [],
      oracleText: String(face.oracleText || ""),
      power: String(face.power || "").trim() || null,
      toughness: String(face.toughness || "").trim() || null,
      loyalty: face.loyalty == null || face.loyalty === "" ? null : Number(face.loyalty),
      defense: face.defense == null || face.defense === "" ? null : Number(face.defense),
    })),
  };
}

