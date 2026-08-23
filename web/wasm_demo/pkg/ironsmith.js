import initEngine, { WasmGame } from "./engine.js";
import initCompiler, {
  compileCardArtifact,
  validateCompiledCardArtifact,
} from "./compiler.js";
import initVerifier, {
  ziffleBuildRevealToken,
  ziffleBuildRevealTokens,
  ziffleBuildShuffleStep,
  ziffleKeygen,
  ziffleRevealCard,
  ziffleRevealCards,
  ziffleVerifyShuffle,
} from "./verifier.js";

export * from "./engine.js";
export {
  compileCardArtifact,
  validateCompiledCardArtifact,
  ziffleBuildRevealToken,
  ziffleBuildRevealTokens,
  ziffleBuildShuffleStep,
  ziffleKeygen,
  ziffleRevealCard,
  ziffleRevealCards,
  ziffleVerifyShuffle,
};

/** Compile source in the compiler module and register the typed artifact in an engine session. */
export function compileAndRegisterCard(game, input) {
  const artifact = compileCardArtifact(input);
  game.registerCompiledCardArtifact(artifact);
  return artifact;
}

function sourceArtifacts(source) {
  if (Array.isArray(source?.artifacts) && source.artifacts.length > 0) {
    return source.artifacts;
  }
  const group = source?.group;
  if (!group || typeof group !== "object") {
    throw new TypeError("card source is missing its group");
  }
  if (group.kind === "single") {
    return [compileCardArtifact({
      name: group.name,
      text: group.block,
      semanticScore: group.score,
      localId: 1,
    })];
  }
  if (group.kind !== "linked" || !Array.isArray(group.faces) || group.faces.length < 2) {
    throw new TypeError(`unsupported card source group: ${String(group.kind)}`);
  }
  return group.faces.map((face, index) => {
    const otherIndex = index === 0 ? 1 : 0;
    const other = group.faces[otherIndex];
    return compileCardArtifact({
      name: face.name,
      text: face.block,
      semanticScore: face.score,
      localId: index + 1,
      otherFaceId: otherIndex + 1,
      otherFaceName: other?.name,
      linkedFaceLayout: group.layout === "split" ? "split" : "transform_like",
    });
  });
}

/** Compile legacy frontend card-source groups in the compiler module and load their artifacts. */
export function compileAndRegisterCardSources(game, input) {
  const sources = Array.isArray(input) ? input : [input];
  const summary = { loaded: 0, failed: [] };
  for (const source of sources) {
    const failureName = source?.group?.name
      ?? source?.group?.faces?.[0]?.name
      ?? source?.group?.combinedName
      ?? source?.canonicalName
      ?? "unknown card source";
    try {
      const registered = game.registerCompiledCardSourceArtifacts(
        source,
        sourceArtifacts(source),
      );
      summary.loaded += Number(registered?.loaded ?? 0);
      if (Array.isArray(registered?.failed)) summary.failed.push(...registered.failed);
    } catch (error) {
      summary.failed.push({
        name: String(failureName),
        error: String(error?.message ?? error),
      });
    }
  }
  return summary;
}

function manabrewCardName(card) {
  return String(card?.identity?.name ?? card?.name ?? "").trim();
}

function manabrewTypeLine(card) {
  const explicit = String(card?.typeLine ?? card?.type_line ?? "").trim();
  if (explicit) return explicit;
  const front = [...(card?.supertypes ?? []), ...(card?.types ?? [])]
    .map(String).filter(Boolean).join(" ");
  const subtypes = (card?.subtypes ?? []).map(String).filter(Boolean).join(" ");
  if (front && subtypes) return `${front} — ${subtypes}`;
  return front || subtypes || "Card";
}

function manabrewCardBlock(card) {
  const lines = [];
  const manaCost = String(card?.manaCost ?? card?.mana_cost ?? "").trim();
  if (manaCost) lines.push(`Mana cost: ${manaCost}`);
  lines.push(`Type: ${manabrewTypeLine(card)}`);
  if (card?.power != null && card?.toughness != null) {
    lines.push(`Power/Toughness: ${card.power}/${card.toughness}`);
  }
  if (card?.loyalty != null) lines.push(`Loyalty: ${card.loyalty}`);
  if (card?.defense != null) lines.push(`Defense: ${card.defense}`);
  const text = String(card?.text ?? card?.oracleText ?? card?.oracle_text ?? "").trim();
  if (text) lines.push(text);
  return lines.join("\n");
}

function manabrewCardSource(card) {
  const deckName = manabrewCardName(card);
  if (!deckName) return null;
  const rawFaces = card?.cardFaces ?? card?.card_faces ?? card?.faces ?? [];
  const faces = Array.isArray(rawFaces)
    ? rawFaces.slice(0, 2).map((face) => ({
        name: manabrewCardName(face),
        block: manabrewCardBlock(face),
        score: 1,
      })).filter((face) => face.name)
    : [];
  if (faces.length === 2) {
    const combinedName = String(
      card?.combinedName ?? card?.combined_name
      ?? (deckName.includes(" // ") ? deckName : `${faces[0].name} // ${faces[1].name}`)
    ).trim();
    const aliases = [deckName, combinedName]
      .filter((alias, index, values) =>
        alias && alias.toLowerCase() !== faces[0].name.toLowerCase()
        && values.findIndex((candidate) => candidate.toLowerCase() === alias.toLowerCase()) === index)
      .map((alias) => ({ alias, canonical: faces[0].name }));
    return {
      canonicalName: faces[0].name,
      aliases,
      group: {
        kind: "linked",
        layout: String(card?.layout ?? "").toLowerCase() === "split" ? "split" : "transform_like",
        combinedName,
        hasFuse: Boolean(card?.hasFuse ?? card?.has_fuse),
        faces,
      },
    };
  }
  return {
    canonicalName: deckName,
    aliases: [],
    group: {
      kind: "single",
      name: deckName,
      block: manabrewCardBlock(card),
      score: 1,
    },
  };
}

function manabrewDeckSources(decks) {
  const sections = [
    "cards", "sideboard", "commanders", "attractions", "contraptions",
    "schemes", "planes", "maybeboard", "tokens",
  ];
  const seen = new Set();
  const sources = [];
  for (const deck of Array.isArray(decks) ? decks : []) {
    const cards = sections.flatMap((section) => Array.isArray(deck?.[section]) ? deck[section] : []);
    if (deck?.companion) cards.push(deck.companion);
    for (const card of cards) {
      const source = manabrewCardSource(card);
      if (!source) continue;
      const names = source.group.kind === "linked"
        ? source.group.faces.map((face) => face.name)
        : [source.group.name];
      if (names.some((name) => seen.has(name.toLowerCase()))) continue;
      names.forEach((name) => seen.add(name.toLowerCase()));
      sources.push(source);
    }
  }
  return sources;
}

const verifierMethods = {
  ziffleBuildRevealToken,
  ziffleBuildRevealTokens,
  ziffleBuildShuffleStep,
  ziffleKeygen,
  ziffleRevealCard,
  ziffleRevealCards,
  ziffleVerifyShuffle,
};

let initialized;

function wasmInitOptions(input) {
  if (input === undefined) return undefined;
  if (input && typeof input === "object" && "module_or_path" in input) return input;
  return { module_or_path: input };
}

function installCompatibilityMethods() {
  for (const [name, operation] of Object.entries(verifierMethods)) {
    if (typeof WasmGame.prototype[name] === "function") continue;
    Object.defineProperty(WasmGame.prototype, name, {
      configurable: true,
      value(input) {
        return operation(input);
      },
    });
  }
  const proto = WasmGame.prototype;
  if (typeof proto.registerCompiledCardSourceArtifacts === "function") {
    Object.defineProperty(proto, "registerExternalCardSources", {
      configurable: true,
      value(input) {
        return compileAndRegisterCardSources(this, input);
      },
    });
    Object.defineProperty(proto, "registerExternalCardSourcesJson", {
      configurable: true,
      value(input) {
        return JSON.stringify(compileAndRegisterCardSources(this, JSON.parse(input)));
      },
    });
    const validateManabrewMatchConfig = proto.validateManabrewMatchConfig;
    const startManabrewMatch = proto.startManabrewMatch;
    Object.defineProperty(proto, "registerManabrewDeckSources", {
      configurable: true,
      value(decks) {
        return compileAndRegisterCardSources(this, manabrewDeckSources(decks));
      },
    });
    if (typeof validateManabrewMatchConfig === "function") {
      Object.defineProperty(proto, "validateManabrewMatchConfig", {
        configurable: true,
        value(config) {
          compileAndRegisterCardSources(this, manabrewDeckSources(config?.decks));
          return validateManabrewMatchConfig.call(this, config);
        },
      });
    }
    if (typeof startManabrewMatch === "function") {
      Object.defineProperty(proto, "startManabrewMatch", {
        configurable: true,
        value(config) {
          compileAndRegisterCardSources(this, manabrewDeckSources(config?.decks));
          return startManabrewMatch.call(this, config);
        },
      });
    }
  }
}

export default function init(input) {
  if (initialized) return initialized;
  const splitInput =
    input &&
    typeof input === "object" &&
    ("engine" in input || "compiler" in input || "verifier" in input)
      ? input
      : { engine: input };
  initialized = Promise.all([
    initEngine(wasmInitOptions(splitInput.engine)),
    initCompiler(wasmInitOptions(splitInput.compiler)),
    initVerifier(wasmInitOptions(splitInput.verifier)),
  ]).then(([engine]) => {
    installCompatibilityMethods();
    return engine;
  });
  return initialized;
}
