export const PUBLIC_FORMATS = Object.freeze(Object.fromEntries([
  ['standard', 'Standard'], ['pioneer', 'Pioneer'], ['modern', 'Modern'],
  ['legacy', 'Legacy'], ['vintage', 'Vintage'], ['pauper', 'Pauper'], ['commander', 'Commander'],
  // Pauper EDH (PDH): an uncommon commander, a 99 of commons, 30 life.
  ['paupercommander', 'Pauper EDH', { startingLife: 30, engineFormat: 'pauper_commander' }],
].map(([id, label, overrides = {}]) => {
  const commander = id === 'commander' || id === 'paupercommander';
  return [id, Object.freeze({ id, label, startingLife: commander ? 40 : 20,
    maxPlayers: commander ? 4 : 2, engineFormat: commander ? 'commander' : 'normal', ...overrides })];
})));
// Whether a match or deck format builds around commanders.
const COMMANDER_ENGINE_FORMATS = new Set(['commander', 'pauper_commander']);
// Lobbies in a public format check every deck against that format's card
// legality before the match can start, whatever the transport.
export const isLegalityFormat = (format) => typeof format === 'string' && Object.hasOwn(PUBLIC_FORMATS, format);
export const usesCommanders = (format) => COMMANDER_ENGINE_FORMATS.has(format) || COMMANDER_ENGINE_FORMATS.has(PUBLIC_FORMATS[format]?.engineFormat);
export const isRelayId = (id) => /^ws-[a-f0-9]{32}-[a-f0-9]{32}$/.test(String(id || ''));
export function relayBaseUrl() {
  const raw = import.meta.env?.VITE_LOBBY_RELAY_URL || '';
  return raw.replace(/\/$/, '');
}
