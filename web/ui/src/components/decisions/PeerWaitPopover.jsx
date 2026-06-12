import { createContext, useContext } from "react";
import { Loader2 } from "lucide-react";
import { ComicTooltip } from "@/components/ui/comic-tooltip";
import { useGame } from "@/context/GameContext";
import { cn } from "@/lib/utils";

const PeerWaitContext = createContext(null);

function defaultPeerWaitTitle(kind) {
  switch (kind) {
    case "crypto_material": return "Waiting for cryptographic material";
    case "local_action": return "Working on your action";
    case "local_payload": return "Preparing action payload";
    case "engine_work": return "Resolving action";
    case "local_ziffle_reveal": return "Generating reveal proof";
    case "action_progress": return "Waiting for action payload";
    case "ziffle_shuffle": return "Waiting for shuffle material";
    case "ziffle_reveal": return "Waiting for reveal material";
    case "fair_random_commit": return "Waiting for random commitment";
    case "fair_random_reveal": return "Waiting for random reveal";
    case "action_quorum": return "Waiting for action quorum";
    case "timeout_vote": return "Waiting for peer vote";
    case "peer_resync": return "Waiting for peer resync";
    default: return "Waiting for peers";
  }
}

function peerNames(peerWait) {
  if (Array.isArray(peerWait?.peers) && peerWait.peers.length > 0) {
    return peerWait.peers
      .map((peer) => String(peer?.name || "").trim())
      .filter(Boolean);
  }
  const name = String(peerWait?.peerName || "").trim();
  return name ? [name] : [];
}

function peerWaitTitle(peerWait) {
  return String(peerWait?.title || "").trim() || defaultPeerWaitTitle(peerWait?.kind);
}

function peerWaitDescription(peerWait) {
  const description = String(peerWait?.description || "").trim();
  if (description) return description;

  const names = peerNames(peerWait);
  const who = peerWait?.local
    ? "Your browser"
    : names.length > 0
    ? names.join(", ")
    : "one or more peers";
  return `${who} must respond before the game state can advance.`;
}

function peerWaitButtonLabel(peerWait) {
  const names = peerNames(peerWait);
  const singlePeer = names.length === 1 ? names[0] : "";
  const subject = peerWait?.local ? "You" : (singlePeer || "Peers");
  const operation = String(peerWait?.operation || "").trim();
  switch (peerWait?.kind) {
    case "local_action":
    case "local_payload":
      return operation || "Preparing payload";
    case "engine_work":
      return operation || "Engine resolving";
    case "local_ziffle_reveal":
      return operation || `${subject} proving reveal`;
    case "action_progress":
      return operation ? `${subject} ${operation.toLowerCase()}` : `${subject} syncing action`;
    case "crypto_material":
      return `${subject} generating payloads`;
    case "ziffle_reveal":
      return `${subject} proving reveal`;
    case "ziffle_shuffle":
      return `${subject} proving shuffle`;
    case "fair_random_commit":
      return `${subject} committing randomness`;
    case "fair_random_reveal":
      return `${subject} revealing randomness`;
    case "action_quorum":
      return `${subject} verifying payload`;
    case "timeout_vote":
      return `${subject} signing timeout`;
    case "peer_resync":
      return `${subject} importing state`;
    default:
      return `${subject} responding`;
  }
}

function peerWaitButtonDetail(peerWait) {
  const current = Number(peerWait?.progressCurrent);
  const total = Number(peerWait?.progressTotal);
  const hasProgress = Number.isFinite(current) && Number.isFinite(total) && total > 0;
  const cardName = String(peerWait?.cardName || "").trim();
  const detail = String(peerWait?.detail || "").trim();
  const zone = String(peerWait?.zone || "").trim();
  const timeoutMs = Number(peerWait?.responseTimeoutMs);
  const parts = [];
  if (hasProgress) {
    parts.push(`${Math.max(0, Math.min(total, current))}/${total}`);
  }
  if (cardName) {
    parts.push(cardName);
  } else if (detail) {
    parts.push(detail);
  }
  if (zone) {
    parts.push(zone);
  }
  if (parts.length === 0 && Number.isFinite(timeoutMs) && timeoutMs > 0) {
    parts.push(`Timeout ${Math.ceil(timeoutMs / 1000)}s`);
  }
  return parts.join(" / ");
}

export function PeerWaitButtonContent({ className = "", peerWait: peerWaitProp = null }) {
  const contextPeerWait = useContext(PeerWaitContext);
  const { inspectorDebug } = useGame();
  const peerWait = peerWaitProp || contextPeerWait;
  const detail = peerWaitButtonDetail(peerWait);

  // Outside debug mode the crypto round-trips are an implementation detail:
  // show a plain spinner instead of the per-operation labels.
  if (!inspectorDebug) {
    return (
      <span
        className={cn(
          "peer-wait-button-content inline-flex h-full w-full min-w-0 items-center justify-center",
          className
        )}
      >
        <Loader2 className="size-4 animate-spin" aria-hidden="true" />
        <span className="sr-only">{peerWaitTitle(peerWait)}</span>
      </span>
    );
  }

  return (
    <span
      className={cn(
        "peer-wait-button-content inline-flex h-full w-full min-w-0 flex-col items-center justify-center gap-0.5 text-center",
        className
      )}
    >
      <span className="peer-wait-button-label min-w-0 truncate text-[11px] font-bold uppercase leading-tight">
        {peerWaitButtonLabel(peerWait)}
      </span>
      {detail ? (
        <span className="peer-wait-button-detail max-w-full truncate text-[10px] font-semibold uppercase leading-none opacity-80">
          {detail}
        </span>
      ) : null}
    </span>
  );
}

export default function PeerWaitPopover({
  peerWait,
  children,
  side = "top",
  align = "center",
  sideOffset = 7,
  contentClassName = "max-w-[320px]",
}) {
  // Always render the same wrapper tree: toggling between bare children and a
  // popover-wrapped child remounts the trigger (typically the main decision
  // button), restarting its animations/transitions on every peer round-trip.
  return (
    <PeerWaitContext.Provider value={peerWait || null}>
      <ComicTooltip
        disabled={!peerWait}
        title={peerWaitTitle(peerWait)}
        description={peerWaitDescription(peerWait)}
        side={side}
        align={align}
        sideOffset={sideOffset}
        contentClassName={contentClassName}
      >
        {children}
      </ComicTooltip>
    </PeerWaitContext.Provider>
  );
}
