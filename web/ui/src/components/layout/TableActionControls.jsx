import PlayNavigation from "./PlayNavigation";

export default function TableActionControls({ onOpenLobby, onAddCardNotice }) {
  return <div className="table-zone-action-controls"><PlayNavigation onOpenLobby={onOpenLobby} onAddCardNotice={onAddCardNotice} /></div>;
}
