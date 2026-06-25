import { useGame } from "@/context/GameContext";
import { DEFAULT_PLAYER_ACCENT, getPlayerAccent } from "@/lib/player-colors";
import { cn } from "@/lib/utils";
import { playerDisplayName } from "@/lib/player-display";
import ManaPool from "./ManaPool";

export default function PlayerCard({ player, isActive, isPerspective }) {
  const { state, playerAccentOverrides } = useGame();
  const playerAccent = getPlayerAccent(
    state?.players || [],
    player?.id,
    state?.perspective,
    playerAccentOverrides
  ) || DEFAULT_PLAYER_ACCENT;
  const exileCards = Array.isArray(player.exile_cards) ? player.exile_cards : [];
  const commandCards = Array.isArray(player.command_cards) ? player.command_cards : [];
  const sideboardCards = Array.isArray(player.sideboard_cards) ? player.sideboard_cards : [];

  const battlefieldCount = (player.battlefield || []).reduce((total, card) => {
    const count = Number(card.count);
    return total + (Number.isFinite(count) && count > 1 ? count : 1);
  }, 0);

  return (
    <section
      className={cn(
        "p-2 grid gap-2 rounded border border-transparent",
        "bg-gradient-to-b from-secondary to-card",
        isActive && "shadow-[0_0_8px_rgba(127,184,106,0.30),0_0_0_1px_rgba(127,184,106,0.45)_inset]",
      )}
      data-player-id={player.id}
      style={{
        "--player-accent": playerAccent.hex,
        "--player-accent-rgb": playerAccent.rgb,
        ...(isPerspective
          ? {
            borderColor: playerAccent.hex,
            boxShadow: `inset 0 0 10px rgba(${playerAccent.rgb}, 0.34)`,
          }
          : null),
      }}
    >
      <div className="flex items-center gap-2 min-w-0">
        <h2 className="text-[15px] font-bold m-0 truncate" style={{ color: playerAccent.hex }}>
          {playerDisplayName(state?.players || [], player)}
        </h2>
        <ManaPool
          pool={player.mana_pool}
          alwaysVisible
          compact
          className="player-name-mana"
        />
      </div>

      <div className="flex flex-wrap gap-1 text-[11px] text-muted-foreground">
        <span className="bg-background/70 px-1.5 rounded-sm" title="Library">
          Lib <span className="font-bold text-foreground">{player.library_size}</span>
        </span>
        <span className="bg-background/70 px-1.5 rounded-sm" title="Hand">
          Hand <span className="font-bold text-foreground">{player.hand_size}</span>
        </span>
        <span className="bg-background/70 px-1.5 rounded-sm" title="GY">
          GY <span className="font-bold text-foreground">{player.graveyard_size}</span>
        </span>
        <span className="bg-background/70 px-1.5 rounded-sm" title="Exile">
          Exl <span className="font-bold text-foreground">{exileCards.length}</span>
        </span>
        <span className="bg-background/70 px-1.5 rounded-sm" title="CZ">
          Cmd <span className="font-bold text-foreground">{player.command_size ?? commandCards.length}</span>
        </span>
        {sideboardCards.length > 0 && (
          <span className="bg-background/70 px-1.5 rounded-sm" title="Sideboard">
            SB <span className="font-bold text-foreground">{sideboardCards.length}</span>
          </span>
        )}
        <span className="bg-background/70 px-1.5 rounded-sm" title="Battlefield">
          BF <span className="font-bold text-foreground">{battlefieldCount}</span>
        </span>
      </div>

    </section>
  );
}
