import type { WasmGame } from "./engine";

export * from "./engine";
export {
  compileCardArtifact,
  validateCompiledCardArtifact,
} from "./compiler";
export {
  ziffleKeygen,
} from "./verifier";

export function ziffleBuildRevealToken(input: unknown): unknown;
export function ziffleBuildRevealTokens(input: unknown): unknown;
export function ziffleBuildShuffleStep(input: unknown): unknown;
export function ziffleRevealCard(input: unknown): unknown;
export function ziffleRevealCards(input: unknown): unknown;
export function ziffleVerifyShuffle(input: unknown): unknown;
export function compileAndRegisterCard(
  game: WasmGame,
  input: { name: string; text: string; allowUnsupported?: boolean },
): unknown;
export function compileAndRegisterCardSources(
  game: WasmGame,
  input: unknown | unknown[],
): { loaded: number; failed: Array<{ name: string; error: string }> };

export interface SplitWasmInput {
  engine?: RequestInfo | URL | Response | BufferSource | WebAssembly.Module;
  compiler?: false | RequestInfo | URL | Response | BufferSource | WebAssembly.Module;
  verifier?: false | RequestInfo | URL | Response | BufferSource | WebAssembly.Module;
}

export default function init(input?: SplitWasmInput | SplitWasmInput["engine"]): Promise<unknown>;

export function initCompilerRuntime(input?: Exclude<SplitWasmInput["compiler"], false>): Promise<unknown>;
export function initVerifierRuntime(input?: Exclude<SplitWasmInput["verifier"], false>): Promise<unknown>;
