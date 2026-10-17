// Marks <html data-full-cards> while the "Full cards" appearance is chosen.
// Theme rules that must hold before a battlefield renders (the navigation
// in the top-right corner, for instance) key off this flag instead of the
// battlefield row, so they apply from the first paint rather than flashing
// the compact layout while a game prepares.
const KEY = "ironsmith.battlefieldAppearance";

export function syncFullCardsFlag(appearance) {
  if (typeof document === "undefined") return;
  document.documentElement.toggleAttribute("data-full-cards", appearance?.compactCards === false);
}

function readSavedAppearance() {
  try {
    return JSON.parse(window.localStorage.getItem(KEY) || "{}");
  } catch {
    return {};
  }
}

// Applied when the module loads, before React renders anything.
if (typeof window !== "undefined") syncFullCardsFlag(readSavedAppearance());
