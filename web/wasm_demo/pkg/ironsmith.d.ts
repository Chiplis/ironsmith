/* tslint:disable */
/* eslint-disable */

/**
 * Browser-exposed game handle.
 */
export class WasmGame {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Add a specific card by name to a player's hand.
     */
    addCardToHand(player_index: number, card_name: string): bigint;
    /**
     * Add a specific card by name to a player's zone.
     *
     * When `skip_triggers` is true the card is placed directly without
     * processing ETB or other zone-change triggers.
     */
    addCardToZone(player_index: number, card_name: string, zone_name: string, skip_triggers: boolean): bigint;
    /**
     * Add a signed life delta (negative = damage, positive = gain).
     */
    addLifeDelta(player_index: number, delta: number): void;
    /**
     * Advance to next phase (or next turn if ending phase).
     * Resets the TurnRunner so it picks up from the new game state.
     */
    advancePhase(): void;
    /**
     * Return locally-known card name suggestions from the generated registry.
     */
    autocompleteCardNames(query: string, limit?: number | null): any;
    /**
     * Cancel the current pending decision chain.
     *
     * Rollback preference:
     * 1. The active user-action checkpoint (start of this spell/ability chain).
     * 2. The active replay-action checkpoint (for speculative nested prompts).
     * 3. The priority-epoch checkpoint (start of this priority round).
     *
     * This mirrors "take back this action chain" behavior first, while still
     * preserving the broader epoch rollback as a fallback.
     */
    cancelDecision(): any;
    cardLoadDiagnostics(card_name: string, error_message?: string | null): any;
    /**
     * Get the count of scored cards meeting the current threshold.
     */
    cardsMeetingThreshold(): number;
    createCustomCard(payload_js: any): bigint;
    /**
     * Apply a player command for the currently pending decision.
     */
    dispatch(command: any): any;
    /**
     * Draw one card for a player.
     */
    drawCard(player_index: number): number;
    /**
     * Draw opening hands for all players.
     */
    drawOpeningHands(cards_per_player: number): void;
    /**
     * Get the semantic score for a specific card. Returns -1.0 if score is unavailable.
     */
    getCardSemanticScore(card_name: string): number;
    /**
     * Get the current semantic threshold as percentage points.
     */
    getSemanticThreshold(): number;
    /**
     * Return whether the query resolves to a locally known card name.
     */
    isKnownCardName(query: string): boolean;
    lastAdvanceUntilDecisionPerf(): any;
    lastDispatchPerf(): any;
    lastReplayExecutionPerf(): any;
    lastSnapshotPerf(): any;
    /**
     * Load explicit decks by card name. JS format: `string[][]`.
     *
     * Deck list index maps to player index.
     * Returns a JSON object with total and categorized failures:
     * `{ loaded, failed, failedBelowThreshold, failedToParse }`.
     * Unknown cards are skipped rather than aborting the entire load.
     */
    loadDecks(decks_js: any): any;
    /**
     * Replace game state with demo decks and no battlefield/stack state.
     */
    loadDemoDecks(): void;
    /**
     * Construct a demo game with two players.
     */
    constructor();
    /**
     * Return a detailed, human-readable object snapshot for inspector UI.
     */
    objectDetails(object_id: bigint): any;
    /**
     * Parse/register the next batch of generated cards for startup warmup.
     */
    preloadRegistryChunk(_chunk_size: number): any;
    /**
     * Incremental generated-registry preload status.
     */
    preloadRegistryStatus(): any;
    previewCustomCard(draft_js: any): any;
    /**
     * Number of cards currently available in the registry.
     */
    registrySize(): number;
    /**
     * Reset game with custom player names and starting life.
     */
    reset(player_names: any, starting_life: number): void;
    sampleLoadedDeckSeed(player_index: number): any;
    /**
     * Toggle automatic cleanup discard (random cards).
     */
    setAutoCleanupDiscard(enabled: boolean): void;
    /**
     * Set a player's life total.
     */
    setLife(player_index: number, life: number): void;
    /**
     * Set local perspective explicitly.
     */
    setPerspective(player_index: number): void;
    /**
     * Set the semantic similarity threshold for card addition (0..100%, 0 = off).
     */
    setSemanticThreshold(threshold: number): void;
    /**
     * Return a JS object snapshot of public game state.
     */
    snapshot(): any;
    /**
     * Return game snapshot as pretty JSON.
     */
    snapshotJson(): string;
    /**
     * Start a fully specified match from a synchronized lobby payload.
     */
    startMatch(config: any): any;
    /**
     * Switch local perspective to the next player.
     */
    switchPerspective(): number;
    /**
     * Return the current UI state from the selected player perspective.
     */
    uiState(): any;
    validateMatchConfig(config: any): any;
}

export function wasm_start(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmgame_free: (a: number, b: number) => void;
    readonly wasmgame_new: () => number;
    readonly wasmgame_reset: (a: number, b: any, c: number) => [number, number];
    readonly wasmgame_startMatch: (a: number, b: any) => [number, number, number];
    readonly wasmgame_validateMatchConfig: (a: number, b: any) => [number, number, number];
    readonly wasmgame_snapshot: (a: number) => [number, number, number];
    readonly wasmgame_lastSnapshotPerf: (a: number) => [number, number, number];
    readonly wasmgame_lastDispatchPerf: (a: number) => [number, number, number];
    readonly wasmgame_lastReplayExecutionPerf: (a: number) => [number, number, number];
    readonly wasmgame_lastAdvanceUntilDecisionPerf: (a: number) => [number, number, number];
    readonly wasmgame_registrySize: (a: number) => number;
    readonly wasmgame_preloadRegistryStatus: (a: number) => [number, number, number];
    readonly wasmgame_preloadRegistryChunk: (a: number, b: number) => [number, number, number];
    readonly wasmgame_objectDetails: (a: number, b: bigint) => [number, number, number];
    readonly wasmgame_snapshotJson: (a: number) => [number, number, number, number];
    readonly wasmgame_autocompleteCardNames: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly wasmgame_isKnownCardName: (a: number, b: number, c: number) => number;
    readonly wasmgame_setLife: (a: number, b: number, c: number) => [number, number];
    readonly wasmgame_addLifeDelta: (a: number, b: number, c: number) => [number, number];
    readonly wasmgame_drawCard: (a: number, b: number) => [number, number, number];
    readonly wasmgame_addCardToHand: (a: number, b: number, c: number, d: number) => [bigint, number, number];
    readonly wasmgame_addCardToZone: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [bigint, number, number];
    readonly wasmgame_drawOpeningHands: (a: number, b: number) => [number, number];
    readonly wasmgame_loadDemoDecks: (a: number) => [number, number];
    readonly wasmgame_loadDecks: (a: number, b: any) => [number, number, number];
    readonly wasmgame_cardLoadDiagnostics: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly wasmgame_sampleLoadedDeckSeed: (a: number, b: number) => [number, number, number];
    readonly wasmgame_previewCustomCard: (a: number, b: any) => [number, number, number];
    readonly wasmgame_createCustomCard: (a: number, b: any) => [bigint, number, number];
    readonly wasmgame_advancePhase: (a: number) => [number, number];
    readonly wasmgame_setAutoCleanupDiscard: (a: number, b: number) => void;
    readonly wasmgame_setSemanticThreshold: (a: number, b: number) => void;
    readonly wasmgame_getSemanticThreshold: (a: number) => number;
    readonly wasmgame_getCardSemanticScore: (a: number, b: number, c: number) => number;
    readonly wasmgame_cardsMeetingThreshold: (a: number) => number;
    readonly wasmgame_switchPerspective: (a: number) => [number, number, number];
    readonly wasmgame_setPerspective: (a: number, b: number) => [number, number];
    readonly wasmgame_cancelDecision: (a: number) => [number, number, number];
    readonly wasmgame_dispatch: (a: number, b: any) => [number, number, number];
    readonly wasmgame_uiState: (a: number) => [number, number, number];
    readonly wasm_start: () => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
