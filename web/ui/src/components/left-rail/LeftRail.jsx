import PlayersPane from "./PlayersPane";
import DecisionPanel from "./DecisionPanel";

export default function LeftRail() {
  return (
    <aside className="rail-gradient rounded p-2 flex flex-col gap-2 min-h-0 overflow-hidden">
      <PlayersPane />
      <DecisionPanel />
    </aside>
  );
}
