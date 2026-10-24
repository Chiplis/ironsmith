import { LearnButton } from "./PlayNavigation";
import SocialLinks from "./SocialLinks";
import TopbarMenuSheet from "./TopbarMenuSheet";

export default function TopbarUtilityControls({
  playerNames,
  setPlayerNames,
  startingLife,
  setStartingLife,
  onReset,
  onRefresh,
  onToggleLog,
  onOpenPuzzleSetup,
  onOpenLobby,
  onGenerateRandomGame,
  puzzleSetupMode = false,
  onAddCardNotice,
  showBrand = false,
  menuFirst = false,
  showLearn = false,
  // The brand itself opens the menu, with the site links in a row beneath it.
  brandMenu = false,
}) {
  const menu = (
    <TopbarMenuSheet
      playerNames={playerNames}
      setPlayerNames={setPlayerNames}
      startingLife={startingLife}
      setStartingLife={setStartingLife}
      onReset={onReset}
      onRefresh={onRefresh}
      onToggleLog={onToggleLog}
      onOpenPuzzleSetup={onOpenPuzzleSetup}
      onOpenLobby={onOpenLobby}
      onGenerateRandomGame={onGenerateRandomGame}
      puzzleSetupMode={puzzleSetupMode}
      onAddCardNotice={onAddCardNotice}
      triggerIcon={brandMenu ? "brand" : "menu"}
    />
  );
  const links = <>
      <SocialLinks />
      {showLearn ? <LearnButton onOpenLobby={onOpenLobby} iconOnly={brandMenu} /> : null}
  </>;
  if (brandMenu) {
    return (
      <div className="topbar-minor-controls topbar-minor-controls--utility topbar-minor-controls--brand-menu">
        {menu}
        <div className="topbar-brand-links">{links}</div>
      </div>
    );
  }
  return (
    <div className="topbar-minor-controls topbar-minor-controls--utility">
      {menuFirst ? menu : null}
      {showBrand ? (
        <h1 className="toolbar-brand topbar-brand m-0 whitespace-nowrap font-bold">Ironsmith</h1>
      ) : null}
      {links}
      {!menuFirst ? menu : null}
    </div>
  );
}
