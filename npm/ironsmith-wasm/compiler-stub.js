// The npm package does not ship the standalone card compiler: its engine
// module compiles card source itself. This module keeps the facade's import
// shape; the facade sees `available === false` and routes source through the
// engine.
export const available = false;

export default async function init() {}

function unavailable(name) {
  throw new Error(
    `${name} is not available: the ironsmith-wasm npm package compiles card source in the engine module. Use registerExternalCardSources or compileAndRegisterCardSources.`,
  );
}

export function compileCardArtifact() {
  unavailable("compileCardArtifact");
}

export function validateCompiledCardArtifact() {
  unavailable("validateCompiledCardArtifact");
}
