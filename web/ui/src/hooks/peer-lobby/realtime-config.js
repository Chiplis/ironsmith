export function realtimeLobbyUrl(lobbyId, baseUrl = import.meta.env?.VITE_REALTIME_WS_URL) {
  const lobby = String(lobbyId || "").trim();
  const base = String(baseUrl || "").trim();
  if (!lobby || !base) return "";
  let url;
  try {
    url = new URL(base, globalThis.location?.origin || "http://localhost");
  } catch {
    return "";
  }
  if (!["ws:", "wss:"].includes(url.protocol)) return "";
  url.pathname = url.pathname.replace(/\/$/, "") + "/lobby";
  url.searchParams.set("id", lobby);
  return url.toString();
}
