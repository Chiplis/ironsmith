/** Runs Ironsmith's Verified multiplayer protocol (committed decks, jointly shuffled libraries, signed and replayed actions) without a UI framework. */

export interface VerifiedEngine {
  /** Calls a method of the wasm game, wherever it lives (a worker, the same thread). */
  call(method: string, ...args: unknown[]): Promise<unknown>;
  /** The methods this engine has; defaults to every known name. */
  methods?: Iterable<string>;
}

/** PeerJS-compatible peer the protocol talks to. */
export interface VerifiedPeer {
  readonly id: string;
  on(type: string, listener: (...args: any[]) => void): unknown;
  connect(peerId: string, options?: { metadata?: unknown }): unknown;
  destroy(): void;
}

export interface VerifiedSessionOptions {
  engine: VerifiedEngine;
  createPeer(peerId: string, options?: Record<string, unknown>): VerifiedPeer;
  /** Registers card definitions before the engine reads them. */
  ensureCards?(names: string[]): Promise<void>;
  onStatus?(text: string, isError: boolean): void;
  onError?(error: unknown): void;
}

export interface VerifiedSessionState {
  state: any;
  multiplayer: any;
}

export interface VerifiedLobby {
  multiplayer: any;
  canStartHostedMatch: boolean;
  createLobby(options: Record<string, unknown>): Promise<void>;
  joinLobby(options: Record<string, unknown>): Promise<void>;
  leaveLobby(reason?: string): void;
  startHostedMatch(): Promise<void>;
  submitMultiplayerCommand(command: Record<string, unknown>, label?: string): Promise<unknown>;
  exportAuditTranscript(options?: Record<string, unknown>): Promise<unknown>;
  [name: string]: unknown;
}

export interface VerifiedSession {
  readonly game: Record<string, (...args: any[]) => Promise<any>>;
  readonly statuses: Array<{ text: string; isError: boolean }>;
  readonly lobby: VerifiedLobby;
  readonly multiplayer: any;
  readonly state: any;
  subscribe(listener: (update: VerifiedSessionState) => void): () => void;
  publishState(state: unknown): void;
  settled(): Promise<void>;
  dispose(): void;
}

export function createVerifiedSession(options: VerifiedSessionOptions): VerifiedSession;
export function cardNamesInArguments(value: unknown): Set<string>;
export function previewCryptoRequirementsWithMaterial(game: unknown, command: unknown, material: unknown): unknown;
export function registerPeerFactory(factory: (peerId: string, options?: Record<string, unknown>) => VerifiedPeer): void;
export const WORKER_METHODS: string[];
export const ZIFFLE_WORKER_METHODS: Set<string>;
