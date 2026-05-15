import { LoaderCircle } from "lucide-react";
import { ComicTooltip } from "@/components/ui/comic-tooltip";
import { cn } from "@/lib/utils";

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

export function PeerWaitButtonContent({ className = "" }) {
  return (
    <span className={cn("peer-wait-button-content inline-flex h-full w-full items-center justify-center", className)}>
      <LoaderCircle className="peer-wait-spinner h-4 w-4 animate-spin" aria-hidden="true" />
      <span className="sr-only">Waiting for peers</span>
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
  );
}
