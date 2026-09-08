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
export function saveRelayLobby(session) {
  if (!isRelayId(session.lobbyId) || !session.localPeerId) return;
  const room = session.lobbyId.split('-')[1];
  const saved = readRelaySession(room);
  if (saved?.peerId !== session.localPeerId) return;
  saveRelayIdentity(room, undefined, { session });
}
function database() {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open('ironsmith-relay-resume-v1', 1);
    request.onupgradeneeded = () => request.result.createObjectStore('checkpoints');
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}
export async function relayCheckpoint(lobbyId, value) {
  const db = await database();
  try {
    return await new Promise((resolve, reject) => {
      const tx = db.transaction('checkpoints', value === undefined ? 'readonly' : 'readwrite');
      const store = tx.objectStore('checkpoints');
      const request = value === undefined ? store.get(key(lobbyId)) : store.put(value, key(lobbyId));
      tx.oncomplete = () => resolve(request.result);
      tx.onerror = () => reject(tx.error);
      tx.onabort = () => reject(tx.error || new Error('Could not save reconnect checkpoint'));
    });
  } finally { db.close(); }
}
