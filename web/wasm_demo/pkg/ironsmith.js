/* @ts-self-types="./ironsmith.d.ts" */

/**
 * Browser-exposed game handle.
 */
export class WasmGame {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmGameFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmgame_free(ptr, 0);
    }
    /**
     * Add a specific card by name to a player's hand.
     * @param {number} player_index
     * @param {string} card_name
     * @returns {bigint}
     */
    addCardToHand(player_index, card_name) {
        const ptr0 = passStringToWasm0(card_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmgame_addCardToHand(this.__wbg_ptr, player_index, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return BigInt.asUintN(64, ret[0]);
    }
    /**
     * Add a specific card by name to a player's zone.
     *
     * When `skip_triggers` is true the card is placed directly without
     * processing ETB or other zone-change triggers.
     * @param {number} player_index
     * @param {string} card_name
     * @param {string} zone_name
     * @param {boolean} skip_triggers
     * @returns {bigint}
     */
    addCardToZone(player_index, card_name, zone_name, skip_triggers) {
        const ptr0 = passStringToWasm0(card_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(zone_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmgame_addCardToZone(this.__wbg_ptr, player_index, ptr0, len0, ptr1, len1, skip_triggers);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return BigInt.asUintN(64, ret[0]);
    }
    /**
     * Add a signed life delta (negative = damage, positive = gain).
     * @param {number} player_index
     * @param {number} delta
     */
    addLifeDelta(player_index, delta) {
        const ret = wasm.wasmgame_addLifeDelta(this.__wbg_ptr, player_index, delta);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Advance to next phase (or next turn if ending phase).
     * Resets the TurnRunner so it picks up from the new game state.
     */
    advancePhase() {
        const ret = wasm.wasmgame_advancePhase(this.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {any} input
     * @returns {any}
     */
    applyVerifiedHiddenLibraryShuffle(input) {
        const ret = wasm.wasmgame_applyVerifiedHiddenLibraryShuffle(this.__wbg_ptr, input);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Return locally-known card name suggestions from the generated registry.
     * @param {string} query
     * @param {number | null} [limit]
     * @returns {any}
     */
    autocompleteCardNames(query, limit) {
        const ptr0 = passStringToWasm0(query, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmgame_autocompleteCardNames(this.__wbg_ptr, ptr0, len0, isLikeNone(limit) ? 0x100000001 : (limit) >>> 0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
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
     * @returns {any}
     */
    cancelDecision() {
        const ret = wasm.wasmgame_cancelDecision(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {string} card_name
     * @param {string | null} [error_message]
     * @returns {any}
     */
    cardLoadDiagnostics(card_name, error_message) {
        const ptr0 = passStringToWasm0(card_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(error_message) ? 0 : passStringToWasm0(error_message, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmgame_cardLoadDiagnostics(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Get the count of scored cards meeting the current threshold.
     * @returns {number}
     */
    cardsMeetingThreshold() {
        const ret = wasm.wasmgame_cardsMeetingThreshold(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @param {any} payload_js
     * @returns {bigint}
     */
    createCustomCard(payload_js) {
        const ret = wasm.wasmgame_createCustomCard(this.__wbg_ptr, payload_js);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return BigInt.asUintN(64, ret[0]);
    }
    /**
     * Apply a player command for the currently pending decision.
     * @param {any} command
     * @returns {any}
     */
    dispatch(command) {
        const ret = wasm.wasmgame_dispatch(this.__wbg_ptr, command);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Draw one card for a player.
     * @param {number} player_index
     * @returns {number}
     */
    drawCard(player_index) {
        const ret = wasm.wasmgame_drawCard(this.__wbg_ptr, player_index);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * Draw opening hands for all players.
     * @param {number} cards_per_player
     */
    drawOpeningHands(cards_per_player) {
        const ret = wasm.wasmgame_drawOpeningHands(this.__wbg_ptr, cards_per_player);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Move directly into an inserted combat phase without rebuilding from a sync checkpoint.
     */
    enterAdditionalCombatPhase() {
        const ret = wasm.wasmgame_enterAdditionalCombatPhase(this.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {bigint} object_id
     * @returns {any}
     */
    exportHiddenCardOpening(object_id) {
        const ret = wasm.wasmgame_exportHiddenCardOpening(this.__wbg_ptr, object_id);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Export a redacted checkpoint suitable for peer audit logs.
     * @returns {any}
     */
    exportPublicAuditCheckpoint() {
        const ret = wasm.wasmgame_exportPublicAuditCheckpoint(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Export an importable checkpoint redacted for one peer's legal knowledge.
     * @param {number} perspective_index
     * @returns {any}
     */
    exportRedactedSyncCheckpoint(perspective_index) {
        const ret = wasm.wasmgame_exportRedactedSyncCheckpoint(this.__wbg_ptr, perspective_index);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Export a WASM-owned resync checkpoint that can hydrate another peer's engine.
     * @returns {any}
     */
    exportSyncCheckpoint() {
        const ret = wasm.wasmgame_exportSyncCheckpoint(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Queue a forced die result for deterministic test harness scenarios.
     * @param {number} result
     */
    forceNextDieRoll(result) {
        wasm.wasmgame_forceNextDieRoll(this.__wbg_ptr, result);
    }
    /**
     * Turn a face-down permanent face up without going through priority action
     * enumeration. Ported tests use this when the UI has not exposed the
     * special action because mana was supplied out of band.
     * @param {number} player_index
     * @param {bigint} object_id
     */
    forceTurnFaceUp(player_index, object_id) {
        const ret = wasm.wasmgame_forceTurnFaceUp(this.__wbg_ptr, player_index, object_id);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Mark a player as having forfeited the match.
     * @param {number} player_index
     * @returns {any}
     */
    forfeitPlayer(player_index) {
        const ret = wasm.wasmgame_forfeitPlayer(this.__wbg_ptr, player_index);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Get the semantic score for a specific card. Returns -1.0 if score is unavailable.
     * @param {string} card_name
     * @returns {number}
     */
    getCardSemanticScore(card_name) {
        const ptr0 = passStringToWasm0(card_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmgame_getCardSemanticScore(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Get the current semantic threshold as percentage points.
     * @returns {number}
     */
    getSemanticThreshold() {
        const ret = wasm.wasmgame_getSemanticThreshold(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {boolean}
     */
    hasDayNight() {
        const ret = wasm.wasmgame_hasDayNight(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Replace this WASM engine with a checkpoint from the current authoritative host.
     * @param {any} checkpoint
     * @param {number} perspective_index
     * @returns {any}
     */
    importSyncCheckpoint(checkpoint, perspective_index) {
        const ret = wasm.wasmgame_importSyncCheckpoint(this.__wbg_ptr, checkpoint, perspective_index);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {any} input
     */
    injectTranscriptRandomSeeds(input) {
        const ret = wasm.wasmgame_injectTranscriptRandomSeeds(this.__wbg_ptr, input);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @returns {boolean}
     */
    isDaytime() {
        const ret = wasm.wasmgame_isDaytime(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Return whether the query resolves to a locally known card name.
     * @param {string} query
     * @returns {boolean}
     */
    isKnownCardName(query) {
        const ptr0 = passStringToWasm0(query, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmgame_isKnownCardName(this.__wbg_ptr, ptr0, len0);
        return ret !== 0;
    }
    /**
     * @returns {any}
     */
    lastAdvanceUntilDecisionPerf() {
        const ret = wasm.wasmgame_lastAdvanceUntilDecisionPerf(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @returns {any}
     */
    lastDispatchPerf() {
        const ret = wasm.wasmgame_lastDispatchPerf(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @returns {any}
     */
    lastReplayExecutionPerf() {
        const ret = wasm.wasmgame_lastReplayExecutionPerf(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @returns {any}
     */
    lastSnapshotPerf() {
        const ret = wasm.wasmgame_lastSnapshotPerf(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Load explicit decks by card name. JS format: `string[][]` or
     * `{ decks: string[][], sideboards?: string[][] }`.
     *
     * Deck list index maps to player index.
     * Returns a JSON object with total and categorized failures:
     * `{ loaded, failed, failedBelowThreshold, failedToParse }`.
     * Unknown cards are skipped rather than aborting the entire load.
     * @param {any} decks_js
     * @returns {any}
     */
    loadDecks(decks_js) {
        const ret = wasm.wasmgame_loadDecks(this.__wbg_ptr, decks_js);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Replace game state with demo decks and no battlefield/stack state.
     */
    loadDemoDecks() {
        const ret = wasm.wasmgame_loadDemoDecks(this.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Move a hand card onto the battlefield with the shared morph-style
     * face-down overlay. This is used by ported test harnesses that set up a
     * cast result directly when the UI has no payable cast action exposed.
     * @param {number} player_index
     * @param {bigint} object_id
     * @param {number} ward_generic_cost
     * @returns {bigint}
     */
    moveHandCardToBattlefieldFaceDown(player_index, object_id, ward_generic_cost) {
        const ret = wasm.wasmgame_moveHandCardToBattlefieldFaceDown(this.__wbg_ptr, player_index, object_id, ward_generic_cost);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return BigInt.asUintN(64, ret[0]);
    }
    /**
     * Construct a demo game with two players.
     */
    constructor() {
        const ret = wasm.wasmgame_new();
        this.__wbg_ptr = ret >>> 0;
        WasmGameFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Return a detailed, human-readable object snapshot for inspector UI.
     * @param {bigint} object_id
     * @returns {any}
     */
    objectDetails(object_id) {
        const ret = wasm.wasmgame_objectDetails(this.__wbg_ptr, object_id);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Parse/register the next batch of generated cards for startup warmup.
     * @param {number} _chunk_size
     * @returns {any}
     */
    preloadRegistryChunk(_chunk_size) {
        const ret = wasm.wasmgame_preloadRegistryChunk(this.__wbg_ptr, _chunk_size);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Incremental generated-registry preload status.
     * @returns {any}
     */
    preloadRegistryStatus() {
        const ret = wasm.wasmgame_preloadRegistryStatus(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {any} command
     * @returns {any}
     */
    previewCryptoRequirements(command) {
        const ret = wasm.wasmgame_previewCryptoRequirements(this.__wbg_ptr, command);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {any} draft_js
     * @returns {any}
     */
    previewCustomCard(draft_js) {
        const ret = wasm.wasmgame_previewCustomCard(this.__wbg_ptr, draft_js);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {any} sources
     * @returns {any}
     */
    registerExternalCardSources(sources) {
        const ret = wasm.wasmgame_registerExternalCardSources(this.__wbg_ptr, sources);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Number of cards currently available in the registry.
     * @returns {number}
     */
    registrySize() {
        const ret = wasm.wasmgame_registrySize(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Reset game with custom player names and starting life.
     * @param {any} player_names
     * @param {number} starting_life
     */
    reset(player_names, starting_life) {
        const ret = wasm.wasmgame_reset(this.__wbg_ptr, player_names, starting_life);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {any} input
     * @returns {any}
     */
    revealHiddenObject(input) {
        const ret = wasm.wasmgame_revealHiddenObject(this.__wbg_ptr, input);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {any} input
     * @returns {any}
     */
    revealHiddenPosition(input) {
        const ret = wasm.wasmgame_revealHiddenPosition(this.__wbg_ptr, input);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {any} input
     * @returns {any}
     */
    revealHiddenSlot(input) {
        const ret = wasm.wasmgame_revealHiddenSlot(this.__wbg_ptr, input);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {number} player_index
     * @returns {any}
     */
    sampleLoadedDeckSeed(player_index) {
        const ret = wasm.wasmgame_sampleLoadedDeckSeed(this.__wbg_ptr, player_index);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Record an attacking band for the current combat.
     * @param {Array<any>} member_ids
     */
    setAttackingBand(member_ids) {
        const ret = wasm.wasmgame_setAttackingBand(this.__wbg_ptr, member_ids);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {boolean} enabled
     */
    setAutoChooseSingleObjectDecisions(enabled) {
        wasm.wasmgame_setAutoChooseSingleObjectDecisions(this.__wbg_ptr, enabled);
    }
    /**
     * Toggle automatic cleanup discard (random cards).
     * @param {boolean} enabled
     */
    setAutoCleanupDiscard(enabled) {
        wasm.wasmgame_setAutoCleanupDiscard(this.__wbg_ptr, enabled);
    }
    /**
     * Set an explicit combat damage assignment for the next combat damage step.
     * @param {bigint} attacker_id
     * @param {bigint} recipient_id
     * @param {number} amount
     */
    setCombatDamageAssignment(attacker_id, recipient_id, amount) {
        wasm.wasmgame_setCombatDamageAssignment(this.__wbg_ptr, attacker_id, recipient_id, amount);
    }
    /**
     * @param {boolean} daytime
     * @returns {any}
     */
    setDaytime(daytime) {
        const ret = wasm.wasmgame_setDaytime(this.__wbg_ptr, daytime);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Set a player's life total.
     * @param {number} player_index
     * @param {number} life
     */
    setLife(player_index, life) {
        const ret = wasm.wasmgame_setLife(this.__wbg_ptr, player_index, life);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set local perspective explicitly.
     * @param {number} player_index
     */
    setPerspective(player_index) {
        const ret = wasm.wasmgame_setPerspective(this.__wbg_ptr, player_index);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Set the semantic similarity threshold for card addition (0..100%, 0 = off).
     * @param {number} threshold
     */
    setSemanticThreshold(threshold) {
        wasm.wasmgame_setSemanticThreshold(this.__wbg_ptr, threshold);
    }
    /**
     * Return a JS object snapshot of public game state.
     * @returns {any}
     */
    snapshot() {
        const ret = wasm.wasmgame_snapshot(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Return game snapshot as pretty JSON.
     * @returns {string}
     */
    snapshotJson() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.wasmgame_snapshotJson(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Start a fully specified match from a synchronized lobby payload.
     * @param {any} config
     * @returns {any}
     */
    startMatch(config) {
        const ret = wasm.wasmgame_startMatch(this.__wbg_ptr, config);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Switch local perspective to the next player.
     * @returns {number}
     */
    switchPerspective() {
        const ret = wasm.wasmgame_switchPerspective(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0];
    }
    /**
     * Return the current UI state from the selected player perspective.
     * @returns {any}
     */
    uiState() {
        const ret = wasm.wasmgame_uiState(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {any} config
     * @returns {any}
     */
    validateMatchConfig(config) {
        const ret = wasm.wasmgame_validateMatchConfig(this.__wbg_ptr, config);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {any} input
     * @returns {any}
     */
    ziffleBuildRevealToken(input) {
        const ret = wasm.wasmgame_ziffleBuildRevealToken(this.__wbg_ptr, input);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {any} input
     * @returns {any}
     */
    ziffleBuildRevealTokens(input) {
        const ret = wasm.wasmgame_ziffleBuildRevealTokens(this.__wbg_ptr, input);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {any} input
     * @returns {any}
     */
    ziffleBuildShuffleStep(input) {
        const ret = wasm.wasmgame_ziffleBuildShuffleStep(this.__wbg_ptr, input);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {any} input
     * @returns {any}
     */
    ziffleKeygen(input) {
        const ret = wasm.wasmgame_ziffleKeygen(this.__wbg_ptr, input);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {any} input
     * @returns {any}
     */
    ziffleRevealCard(input) {
        const ret = wasm.wasmgame_ziffleRevealCard(this.__wbg_ptr, input);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {any} input
     * @returns {any}
     */
    ziffleRevealCards(input) {
        const ret = wasm.wasmgame_ziffleRevealCards(this.__wbg_ptr, input);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {any} input
     * @returns {any}
     */
    ziffleVerifyShuffle(input) {
        const ret = wasm.wasmgame_ziffleVerifyShuffle(this.__wbg_ptr, input);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
}
if (Symbol.dispose) WasmGame.prototype[Symbol.dispose] = WasmGame.prototype.free;

export function wasm_start() {
    wasm.wasm_start();
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg_Error_960c155d3d49e4c2: function(arg0, arg1) {
            const ret = Error(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_Number_32bf70a599af1d4b: function(arg0) {
            const ret = Number(arg0);
            return ret;
        },
        __wbg_String_8564e559799eccda: function(arg0, arg1) {
            const ret = String(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_bigint_get_as_i64_3d3aba5d616c6a51: function(arg0, arg1) {
            const v = arg1;
            const ret = typeof(v) === 'bigint' ? v : undefined;
            getDataViewMemory0().setBigInt64(arg0 + 8 * 1, isLikeNone(ret) ? BigInt(0) : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_boolean_get_6ea149f0a8dcc5ff: function(arg0) {
            const v = arg0;
            const ret = typeof(v) === 'boolean' ? v : undefined;
            return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
        },
        __wbg___wbindgen_debug_string_ab4b34d23d6778bd: function(arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_in_a5d8b22e52b24dd1: function(arg0, arg1) {
            const ret = arg0 in arg1;
            return ret;
        },
        __wbg___wbindgen_is_bigint_ec25c7f91b4d9e93: function(arg0) {
            const ret = typeof(arg0) === 'bigint';
            return ret;
        },
        __wbg___wbindgen_is_function_3baa9db1a987f47d: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_object_63322ec0cd6ea4ef: function(arg0) {
            const val = arg0;
            const ret = typeof(val) === 'object' && val !== null;
            return ret;
        },
        __wbg___wbindgen_is_string_6df3bf7ef1164ed3: function(arg0) {
            const ret = typeof(arg0) === 'string';
            return ret;
        },
        __wbg___wbindgen_is_undefined_29a43b4d42920abd: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_jsval_eq_d3465d8a07697228: function(arg0, arg1) {
            const ret = arg0 === arg1;
            return ret;
        },
        __wbg___wbindgen_jsval_loose_eq_cac3565e89b4134c: function(arg0, arg1) {
            const ret = arg0 == arg1;
            return ret;
        },
        __wbg___wbindgen_number_get_c7f42aed0525c451: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_string_get_7ed5322991caaec5: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_6b64449b9b9ed33c: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_call_14b169f759b26747: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.call(arg1);
            return ret;
        }, arguments); },
        __wbg_done_9158f7cc8751ba32: function(arg0) {
            const ret = arg0.done;
            return ret;
        },
        __wbg_entries_e0b73aa8571ddb56: function(arg0) {
            const ret = Object.entries(arg0);
            return ret;
        },
        __wbg_error_a6fa202b58aa1cd3: function(arg0, arg1) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.error(getStringFromWasm0(arg0, arg1));
            } finally {
                wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_get_1affdbdd5573b16a: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_8360291721e2339f: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_unchecked_17f53dad852b9588: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_with_ref_key_6412cf3094599694: function(arg0, arg1) {
            const ret = arg0[arg1];
            return ret;
        },
        __wbg_instanceof_ArrayBuffer_7c8433c6ed14ffe3: function(arg0) {
            let result;
            try {
                result = arg0 instanceof ArrayBuffer;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Map_1b76fd4635be43eb: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Map;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Uint8Array_152ba1f289edcf3f: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Uint8Array;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_isArray_c3109d14ffc06469: function(arg0) {
            const ret = Array.isArray(arg0);
            return ret;
        },
        __wbg_isSafeInteger_4fc213d1989d6d2a: function(arg0) {
            const ret = Number.isSafeInteger(arg0);
            return ret;
        },
        __wbg_iterator_013bc09ec998c2a7: function() {
            const ret = Symbol.iterator;
            return ret;
        },
        __wbg_length_3d4ecd04bd8d22f1: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_9f1775224cf1d815: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_new_0c7403db6e782f19: function(arg0) {
            const ret = new Uint8Array(arg0);
            return ret;
        },
        __wbg_new_227d7c05414eb861: function() {
            const ret = new Error();
            return ret;
        },
        __wbg_new_682678e2f47e32bc: function() {
            const ret = new Array();
            return ret;
        },
        __wbg_new_aa8d0fa9762c29bd: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_next_0340c4ae324393c3: function() { return handleError(function (arg0) {
            const ret = arg0.next();
            return ret;
        }, arguments); },
        __wbg_next_7646edaa39458ef7: function(arg0) {
            const ret = arg0.next;
            return ret;
        },
        __wbg_now_a9b7df1cbee90986: function() {
            const ret = Date.now();
            return ret;
        },
        __wbg_prototypesetcall_a6b02eb00b0f4ce2: function(arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
        },
        __wbg_random_ce7f6871aed001dd: function() {
            const ret = Math.random();
            return ret;
        },
        __wbg_set_3bf1de9fab0cd644: function(arg0, arg1, arg2) {
            arg0[arg1 >>> 0] = arg2;
        },
        __wbg_set_6be42768c690e380: function(arg0, arg1, arg2) {
            arg0[arg1] = arg2;
        },
        __wbg_stack_3b0d974bbf31e44f: function(arg0, arg1) {
            const ret = arg1.stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_value_ee3a06f4579184fa: function(arg0) {
            const ret = arg0.value;
            return ret;
        },
        __wbindgen_cast_0000000000000001: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000002: function(arg0) {
            // Cast intrinsic for `I64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000003: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_cast_0000000000000004: function(arg0) {
            // Cast intrinsic for `U64 -> Externref`.
            const ret = BigInt.asUintN(64, arg0);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./ironsmith_bg.js": import0,
    };
}

const WasmGameFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmgame_free(ptr >>> 0, 1));

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasm;
function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('ironsmith_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
