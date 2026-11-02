export function normalizeDraftForApi(draft) {
  return {
    enforceFilterClaims: draft.enforceFilterClaims !== false,
    enforceLineClaims: draft.enforceLineClaims !== false,
    customSubtypes: structuredClone(draft.customSubtypes || []),
    customMechanics: structuredClone(draft.customMechanics || []),
    ...(draft.suppliedCardDefinitions?.length?{suppliedCardDefinitions:structuredClone(draft.suppliedCardDefinitions)}:{}),
    ...(draft.customTokens?.length ? { customTokens: structuredClone(draft.customTokens) } : {}),
    ...(draft.token === true ? { token: true } : {}),
    layout: draft.layout || "single",
    hasFuse: draft.hasFuse === true,
    faces: draft.faces.map((face) => ({
      originalPrintedRulesText: String(face.originalPrintedRulesText ?? face.oracleText ?? ""),
      ...(face.displayOracleText != null ? { displayOracleText: face.displayOracleText } : {}),
      ...(face.customKeywordIdentities?.length ? { customKeywordIdentities: [...face.customKeywordIdentities] } : {}),
      ...(face.customKeywordPayments?.length ? { customKeywordPayments: [...face.customKeywordPayments] } : {}),
      ...(face.customKeywordCounterDefinitions?.length ? {customKeywordCounterDefinitions:structuredClone(face.customKeywordCounterDefinitions)} : {}),
      ...(face.suppliedCardDefinitions?.length?{suppliedCardDefinitions:structuredClone(face.suppliedCardDefinitions)}:{}),
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

