export const PUBLIC_FORMATS = Object.freeze(Object.fromEntries([
  ['standard', 'Standard'], ['pioneer', 'Pioneer'], ['modern', 'Modern'],
  ['legacy', 'Legacy'], ['vintage', 'Vintage'], ['pauper', 'Pauper'], ['commander', 'Commander'],
].map(([id, label]) => [id, { id, label, startingLife: id === 'commander' ? 40 : 20,
  maxPlayers: id === 'commander' ? 4 : 2, engineFormat: id === 'commander' ? 'commander' : 'normal' }])));
export const isRelayId = (id) => /^ws-[a-f0-9]{32}-[a-f0-9]{32}$/.test(String(id || ''));
export function relayBaseUrl() {
  const raw = import.meta.env?.VITE_LOBBY_RELAY_URL || '';
  return raw.replace(/\/$/, '');
}
