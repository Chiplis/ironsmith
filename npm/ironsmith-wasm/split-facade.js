// The engine reads a Manabrew deck card into a card source; the facade never
// keeps a second reading of its own.
import initEngine, { WasmGame, manabrewDeckSources } from "./engine.js";
// Retain the engine's current source compiler before compatibility methods
// replace these entry points with artifact-first registration.
const registerSourceInEngine = WasmGame.prototype.registerExternalCardSources;
import initCompiler, * as compilerModule from "./compiler.js";
const { compileCardArtifact, validateCompiledCardArtifact } = compilerModule;
// The npm package replaces the standalone compiler with a stub and compiles
// card source in the engine module instead.
const compilerAvailable = compilerModule.available !== false;
import initVerifier, {
  ziffleBuildRevealToken,
  ziffleBuildRevealTokens,
  ziffleBuildShuffleStep,
  ziffleDealKeyShares,
  ziffleKeygen,
  ziffleOpenKeyShare,
  ziffleRecoverKey,
  ziffleRevealCard,
  ziffleRevealCards,
  ziffleVerifyKeyShare,
  ziffleVerifyShuffle,
} from "./verifier.js";

export * from "./engine.js";
export {
  compileCardArtifact,
  validateCompiledCardArtifact,
  ziffleBuildRevealToken,
  ziffleBuildRevealTokens,
  ziffleBuildShuffleStep,
  ziffleDealKeyShares,
  ziffleKeygen,
  ziffleOpenKeyShare,
  ziffleRecoverKey,
  ziffleRevealCard,
  ziffleRevealCards,
  ziffleVerifyKeyShare,
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
  if (group.kind === "single" || group.kind === "dungeon") {
    // A dungeon (CR 309) compiles like a single card; the engine files it in
    // its dungeon catalog instead of the deck-building registry.
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
      linkedFaceLayout: ["split", "prepare", "flip", "transform"].includes(group.layout)
        ? group.layout
        : "transform_like",
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
      // The browser engine already includes source compilation for custom cards.
      // Reuse it when the standalone compiler has deliberately not been loaded.
      // Baked artifacts retain the faster artifact-only path.
      if (!compilerInitialization && !source?.artifacts?.length && source?.group
          && typeof registerSourceInEngine === "function") {
        const registered = registerSourceInEngine.call(game, source);
        summary.loaded += Number(registered?.loaded ?? 0);
        if (Array.isArray(registered?.failed)) summary.failed.push(...registered.failed);
        continue;
      }
      const registered = game.registerCompiledCardSourceArtifacts(
        source,
        sourceArtifacts(source),
      );
      summary.loaded += Number(registered?.loaded ?? 0);
      if (Array.isArray(registered?.failed)) summary.failed.push(...registered.failed);
    } catch (error) {
      // Recompile rejected sources through the engine's source entry point.
      // Besides recovering stale artifacts, this records parse failures and
      // source metadata for subsequent card loading and diagnostics. A failure
      // in the standalone compiler has not reached the engine at all yet.
      if (source?.group && typeof registerSourceInEngine === "function") {
        try {
          const registered = registerSourceInEngine.call(game, source);
          summary.loaded += Number(registered?.loaded ?? 0);
          if (Array.isArray(registered?.failed)) summary.failed.push(...registered.failed);
          continue;
        } catch (fallbackError) {
          error = fallbackError;
        }
      }
      summary.failed.push({
        name: String(failureName),
        error: String(error?.message ?? error),
      });
    }
  }
  return summary;
}

const verifierMethods = {
  ziffleBuildRevealToken,
  ziffleBuildRevealTokens,
  ziffleBuildShuffleStep,
  ziffleDealKeyShares,
  ziffleKeygen,
  ziffleOpenKeyShare,
  ziffleRecoverKey,
  ziffleRevealCard,
  ziffleRevealCards,
  ziffleVerifyKeyShare,
  ziffleVerifyShuffle,
};

let initialized;
let compilerInitialization;
let verifierInitialization;
export function initCompilerRuntime(input) {
  if (!compilerAvailable) return Promise.resolve(null);
  compilerInitialization ||= initCompiler(wasmInitOptions(input)).catch(error => { compilerInitialization = null; throw error; });
  return compilerInitialization;
}
export function initVerifierRuntime(input) {
  verifierInitialization ||= initVerifier(wasmInitOptions(input)).catch(error => { verifierInitialization = null; throw error; });
  return verifierInitialization;
}

function wasmInitOptions(input) {
  if (input === undefined) return undefined;
  if (input && typeof input === "object" && "module_or_path" in input) return input;
  return { module_or_path: input };
}

function requireManabrewProtocolVersion(engine, config) {
  // A host that declares a protocol version must match the adapter's. The
  // published Manabrew adapter declares none, which the engine accepts.
  if (config?.protocolVersion == null) return;
  const expected = engine.manabrewProtocolVersion();
  if (config.protocolVersion !== expected) {
    throw new Error(`Manabrew protocol version mismatch: expected ${expected}, received ${config?.protocolVersion ?? 0}`);
  }
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
          requireManabrewProtocolVersion(this, config);
          compileAndRegisterCardSources(this, manabrewDeckSources(config?.decks));
          return validateManabrewMatchConfig.call(this, config);
        },
      });
    }
    if (typeof startManabrewMatch === "function") {
      Object.defineProperty(proto, "startManabrewMatch", {
        configurable: true,
        value(config) {
          requireManabrewProtocolVersion(this, config);
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
    splitInput.compiler === false ? null : initCompilerRuntime(splitInput.compiler),
    splitInput.verifier === false ? null : initVerifierRuntime(splitInput.verifier),
  ]).then(([engine]) => {
    installCompatibilityMethods();
    return engine;
  }).catch(error => { initialized = null; throw error; });
  return initialized;
}
