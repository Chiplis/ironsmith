import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { I18nProvider } from "../src/i18n/I18nContext";
import RandomGameSheet from "../src/components/layout/RandomGameSheet";
import "../src/index.css";

window.__generated = [];
window.__statuses = [];

export function Fixture() {
  return (
    <I18nProvider>
      <GameContext.Provider value={{
        state: { players: [], perspective: 0 },
        multiplayer: { mode: "idle" },
        setStatus: (message, isError) => window.__statuses.push({ message, isError: Boolean(isError) }),
      }}>
        <main style={{ padding: 24 }}>
          <RandomGameSheet
            onGenerate={(payload, message) => {
              window.__generated.push({ payload, message });
              return true;
            }}
            trigger={<button type="button" data-random-game-trigger>Random Game</button>}
          />
        </main>
      </GameContext.Provider>
    </I18nProvider>
  );
}

createRoot(document.getElementById("root")).render(<Fixture />);
