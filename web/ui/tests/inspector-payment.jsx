import React, { useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { I18nProvider } from "../src/i18n/I18nContext";
import HoverArtOverlay from "../src/components/right-rail/HoverArtOverlay";
import "../src/index.css";

const card = { id: 1, name: "Payment fixture", type_line: "Artifact", oracle_text: "{U}: Draw a card." };
const action = { index: 0, object_id: 1, ability_index: 0, kind: "activate_ability", label: "Activate Payment fixture: {U}: Draw a card." };
const twoAbilities = new URLSearchParams(window.location.search).has("two");
const dualLand = new URLSearchParams(window.location.search).has("dual");
const actions = dualLand ? [
  { ...action, kind: "activate_mana_ability", label: "Activate Payment fixture: {T}: Add {G}." },
  { ...action, index: 1, ability_index: 1, kind: "activate_mana_ability", label: "Activate Payment fixture: {T}: Add {U}." },
] : twoAbilities ? [
  { ...action, label: "Activate Payment fixture: Pay 1 life: Gain vigilance." },
  { ...action, index: 1, ability_index: 1 },
] : [action];
if (twoAbilities) card.oracle_text = "Pay 1 life: Gain vigilance.\n{U}: Draw a card.";
if (dualLand) {
  card.type_line = "Land — Forest Island";
  // Reverse the lines so an ordinal fallback cannot hide an incorrect match.
  card.oracle_text = "{T}: Add {U}.\n{T}: Add {G}.";
}
window.activatedAbilities = [];
let paymentAvailable = false;
window.paymentRequests = [];
window.resolveNextPayment = () => window.paymentRequests.find(request => !request.resolved)?.resolve();
const game = {
  objectDetails: async () => card,
  inspectorActions: (id, ability) => new Promise(resolve => {
    const available = dualLand || (twoAbilities && ability === 0) ? true : paymentAvailable;
    const request = { source: String(id), resolved: false, resolve: () => {
      request.resolved = true;
      resolve([{ ...actions.find(action => action.ability_index === ability), mana_payment_available: available }]);
    } };
    window.paymentRequests.push(request);
  }),
};

export default function Fixture() {
  const [revision, setRevision] = useState(0);
  const [open, setOpen] = useState(false);
  const state = useMemo(() => ({ players: [{ id: 0, battlefield: [card] }], perspective: 0, revision }), [revision]);
  const [activations, setActivations] = useState(0);
  const [mode, setMode] = useState("card-frame");
  return <GameContext.Provider value={{ state, game }}>
    <button onClick={() => setOpen(!open)}>Toggle inspector</button>
    <button onClick={() => { paymentAvailable = !paymentAvailable; setRevision(value => value + 1); }}>Toggle payment</button>
    <button onClick={() => setMode(mode === "card-frame" ? "inspector" : "card-frame")}>Toggle layout</button>
    <output>{activations}</output>
    <div style={{ position: "relative", width: 700, height: 600 }}>
      {open && <HoverArtOverlay objectId={1} displayMode={mode}
        transientPreview={{ card }}
        interactiveActions={actions}
        onInteractiveAction={action => {
          window.activatedAbilities.push(action.ability_index);
          setActivations(value => value + 1);
        }} />}
    </div>
  </GameContext.Provider>;
}

createRoot(document.getElementById("root")).render(<I18nProvider><Fixture /></I18nProvider>);
