import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { I18nProvider } from "../src/i18n/I18nContext";
import PlayerZonePiles from "../src/components/board/PlayerZonePiles";
import { LOOK_DONE_EVENT } from "../src/lib/look-pile";
import "../src/index.css";

function Fixture() {
  const [permission, setPermission] = useState(false);
  const [view, setView] = useState({ cards: [{id: 1, name: "Island"}, {id: 2, name: "Plains"}] });
  const player = {id: 0, name: "Alice", graveyard_cards: [], exile_cards: [], persistent_look_cards: permission ? [{id: 3, name: "Forest"}] : []};
  return <I18nProvider><GameContext.Provider value={{state: {perspective: 0, players: [player], viewed_cards: view}}}>
    <button onClick={() => window.dispatchEvent(new Event(LOOK_DONE_EVENT))}>Done</button>
    <button onClick={() => setPermission(!permission)}>Toggle permission</button>
    <button onClick={() => setView({cards: [{id: 4, name: "Mountain"}]})}>New view</button>
    <div className="has-zone-piles" style={{position: "relative", width: 700, height: 500, margin: 40}}><PlayerZonePiles player={player} /></div>
  </GameContext.Provider></I18nProvider>;
}
createRoot(document.getElementById("root")).render(<Fixture />);
