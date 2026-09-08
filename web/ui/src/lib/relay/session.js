import { isRelayId, relayBaseUrl } from './formats.js';
const prefix = 'ironsmith-relay-session-v1:';
const key = (room, url = relayBaseUrl()) => `${prefix}${url}:${room}`;
export function readRelaySession(room, url) {
  try {
    const value = JSON.parse(localStorage.getItem(key(room, url)));
    return value && isRelayId(value.peerId) && value.peerId.split('-')[1] === room
      && /^[a-f0-9]{32}$/.test(value.token) ? value : null;
  } catch { return null; }
}
export function saveRelayIdentity(room, url, identity) {
  // A failed write must be visible: otherwise closing this tab would lose the seat.
  localStorage.setItem(key(room, url), JSON.stringify({ ...readRelaySession(room, url), ...identity }));
}
const durableFields = ['role', 'lobbyId', 'hostPeerId', 'localPeerId', 'localName',
  'localPlayerIndex', 'desiredPlayers', 'startingLife', 'format', 'securityMode',
  'localDeckText', 'localCommanderText', 'players', 'matchStarted', 'rematch'];
export function durableRelaySession(session) {
  return Object.fromEntries(durableFields.filter(field => session[field] !== undefined)
    .map(field => [field, session[field]]));
}
export function saveRelayLobby(session, previous) {
  if (!isRelayId(session.lobbyId) || !session.localPeerId) return;
  if (previous && durableFields.every(field => previous[field] === session[field])) return;
  const room = session.lobbyId.split('-')[1];
  const saved = readRelaySession(room);
  if (saved?.peerId !== session.localPeerId) return;
  saveRelayIdentity(room, undefined, { session: durableRelaySession(session) });
}
let connection;
function database() {
  if (connection) return connection;
  connection = new Promise((resolve, reject) => {
    const request = indexedDB.open('ironsmith-relay-resume-v1', 2);
    request.onupgradeneeded = () => {
      if (!request.result.objectStoreNames.contains('checkpoints')) request.result.createObjectStore('checkpoints');
      if (!request.result.objectStoreNames.contains('actions')) request.result.createObjectStore('actions');
    };
    request.onsuccess = () => {
      const db = request.result;
      db.onversionchange = () => { db.close(); connection = null; };
      db.onclose = () => { connection = null; };
      resolve(db);
    };
    request.onerror = () => { connection = null; reject(request.error); };
    request.onblocked = () => { /* Existing tabs close on versionchange. */ };
  });
  return connection;
}
export const relayMatchId = match => String(match?.auditMatchId || `${match?.lobbyId}:${match?.seed}`);
function transaction(db, stores, mode, run) {
  return new Promise((resolve, reject) => {
    const tx = db.transaction(stores, mode);
    let result, failure;
    const fail = error => { failure = error; tx.abort(); };
    tx.oncomplete = () => resolve(result);
    tx.onerror = () => reject(failure || tx.error);
    tx.onabort = () => reject(failure || tx.error || new Error('Could not save reconnect checkpoint'));
    try { run(tx, value => { result = value; }, fail); } catch (error) { fail(error); }
  });
}

// Only called at match initialization/recovery. Accepted actions are appended by
// appendRelayAction, never by rewriting the history on the publication path.
export async function initializeRelayMatch(lobbyId, value) {
  const db = await database();
  return transaction(db, ['checkpoints', 'actions'], 'readwrite', (tx, done, fail) => {
    const headers = tx.objectStore('checkpoints'), actions = tx.objectStore('actions');
    const roomKey = key(lobbyId), matchId = relayMatchId(value.match);
    const request = headers.get(roomKey);
    request.onsuccess = () => {
      try {
        const previous = request.result;
        if (previous?.storageVersion === 2 && previous.matchId === matchId) { done(previous.lastSequence); return; }
        // Clear only this room's old journal; other rooms retain their recovery.
        actions.delete(IDBKeyRange.bound([roomKey], [roomKey, []]));
        const entries = value.actions || [];
        for (let i = 0; i < entries.length; i++) {
          if (Number(entries[i].seq) !== i + 1) throw new Error('Saved action transcript is incomplete');
          actions.put(entries[i], [roomKey, matchId, i + 1]);
        }
        headers.put({ storageVersion: 2, matchId, match: value.match,
          session: durableRelaySession(value.session), lastSequence: entries.length,
          replayOnly: true }, roomKey);
        done(entries.length);
      } catch (error) { fail(error); }
    };
  });
}

export async function appendRelayAction(lobbyId, match, session, entry) {
  const db = await database();
  return transaction(db, ['checkpoints', 'actions'], 'readwrite', (tx, done, fail) => {
    const headers = tx.objectStore('checkpoints'), actions = tx.objectStore('actions');
    const roomKey = key(lobbyId), matchId = relayMatchId(match);
    const request = headers.get(roomKey);
    request.onsuccess = () => {
      try {
        const header = request.result;
        if (!header || header.storageVersion !== 2 || header.matchId !== matchId) throw new Error('Match journal is not initialized');
        if (Number(entry.seq) !== header.lastSequence + 1) throw new Error('Accepted action does not extend durable transcript');
        actions.add(entry, [roomKey, matchId, Number(entry.seq)]);
        headers.put({ ...header, session: durableRelaySession(session), lastSequence: Number(entry.seq) }, roomKey);
        done(Number(entry.seq));
      } catch (error) { fail(error); }
    };
  });
}

export async function relayCheckpoint(lobbyId, value) {
  if (value !== undefined) return initializeRelayMatch(lobbyId, value);
  const db = await database();
  return transaction(db, ['checkpoints', 'actions'], 'readonly', (tx, done, fail) => {
    const roomKey = key(lobbyId);
    const request = tx.objectStore('checkpoints').get(roomKey);
    request.onsuccess = () => {
      const header = request.result;
      if (!header || header.storageVersion !== 2) { done(header); return; }
      const entries = tx.objectStore('actions').getAll(IDBKeyRange.bound(
        [roomKey, header.matchId, 1], [roomKey, header.matchId, Number.MAX_SAFE_INTEGER]));
      entries.onsuccess = () => {
        const actions = entries.result;
        if (actions.length !== header.lastSequence || actions.some((entry, i) => Number(entry.seq) !== i + 1)) {
          fail(new Error('Saved action transcript is incomplete')); return;
        }
        done({ ...header, session: { ...header.session, lastAppliedSequence: header.lastSequence },
          match: { ...header.match, currentPlayers: header.session.players }, actions });
      };
    };
  });
}
