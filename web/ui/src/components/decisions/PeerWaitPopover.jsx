import { createContext, useContext } from "react";
import { ComicTooltip } from "@/components/ui/comic-tooltip";
import { cn } from "@/lib/utils";

const PeerWaitContext = createContext(null);

function defaultPeerWaitTitle(kind) {
  switch (kind) {
    case "crypto_material": return "Waiting for cryptographic material";
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
  const who = names.length > 0
    ? names.join(", ")
    : "one or more peers";
  return `${who} must respond before the game state can advance.`;
}

function peerWaitButtonLabel(peerWait) {
  const names = peerNames(peerWait);
  const singlePeer = names.length === 1 ? names[0] : "";
  const subject = singlePeer || "Peers";
  switch (peerWait?.kind) {
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

export function PeerWaitButtonContent({ className = "", peerWait: peerWaitProp = null }) {
  const contextPeerWait = useContext(PeerWaitContext);
  const peerWait = peerWaitProp || contextPeerWait;
  return (
    <span
      className={cn(
        "peer-wait-button-content inline-flex h-full w-full min-w-0 items-center justify-center text-center",
        className
      )}
    >
      <span className="peer-wait-button-label min-w-0 truncate text-[11px] font-bold uppercase leading-tight">
        {peerWaitButtonLabel(peerWait)}
      </span>
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
  if (!peerWait) return children;

  return (
    <PeerWaitContext.Provider value={peerWait}>
      <ComicTooltip
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
