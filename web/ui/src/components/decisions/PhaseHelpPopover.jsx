import { useMemo } from "react";
import { CircleHelp } from "lucide-react";
import { ComicTooltip } from "@/components/ui/comic-tooltip";
import {
  formatPhase,
  formatStep,
  normalizePhaseKey,
  normalizeStepKey,
} from "@/lib/constants";
import { cn } from "@/lib/utils";

function cleanAdvanceLabel(label) {
  return String(label || "the next step")
    .replace(/^→\s*/, "")
    .trim()
    .toLowerCase();
}

function actionKindLabel(kind) {
  switch (kind) {
    case "play_land": return "play a land";
    case "cast_spell": return "cast a spell";
    case "activate_ability": return "activate an ability";
    case "activate_mana_ability": return "activate a mana ability";
    default: return null;
  }
}

function summarizeAvailableActions(decision) {
  const labels = [];
  const seen = new Set();

  for (const action of decision?.actions || []) {
    if (!action || action.legal === false || action.kind === "pass_priority") continue;
    const label = actionKindLabel(action.kind);
    if (!label || seen.has(label)) continue;
    seen.add(label);
    labels.push(label);
  }

  if (labels.length === 0) {
    return "You can pass priority if you do not want to act.";
  }

  if (labels.length === 1) {
    return `You can ${labels[0]} or pass priority.`;
  }

  return `You can ${labels.slice(0, -1).join(", ")}, ${labels.at(-1)}, or pass priority.`;
}

function currentPhaseGuidance(state) {
  const phase = normalizePhaseKey(state?.phase);
  const step = normalizeStepKey(state?.step);
  const stackSize = Number(state?.stack_size || 0);

  if (stackSize > 0) {
    return "The stack is not empty. Players may respond with instant-speed spells or abilities before the top object resolves.";
  }

  switch (step) {
    case "Untap":
      return "Permanents untap automatically here. Priority is only unusual here, so most choices are instant-speed responses.";
    case "Upkeep":
      return "Upkeep triggers happen here. You can respond with instants and activated abilities before moving on.";
    case "Draw":
      return "The active player draws for turn, then players may respond before the first main phase.";
    case "BeginCombat":
      return "This is the last priority window before attackers are declared.";
    case "DeclareAttackers":
      return "Attackers have been declared. Players may respond before blockers are chosen.";
    case "DeclareBlockers":
      return "Blockers have been declared. Players may respond before combat damage.";
    case "CombatDamage":
      return "Combat damage is handled here. Players may respond before combat ends.";
    case "EndCombat":
      return "End-of-combat effects happen here before the postcombat main phase.";
    case "End":
      return "End step triggers happen here. Players may respond before cleanup.";
    case "Cleanup":
      return "Cleanup removes damage and handles discards. Priority appears here only if something triggers or state-based actions matter.";
    default:
      break;
  }

  switch (phase) {
    case "FirstMain":
      return "The active player may play a land and cast sorcery-speed spells while the stack is empty. Any player with priority can use instant-speed actions.";
    case "NextMain":
      return "This is the postcombat main phase. The active player may use sorcery-speed options while the stack is empty.";
    case "Combat":
      return "Combat is in progress. Use priority windows to act before the next combat step.";
    case "Ending":
      return "The turn is ending. Use priority windows for end-step responses before cleanup.";
    case "Beginning":
      return "The turn is in its beginning phase. Resolve upkeep or draw timing before moving to the main phase.";
    default:
      return "Use priority to take a legal action, or pass to continue.";
  }
}

function phaseHelpContent(state, decision, advanceLabel) {
  const phaseLabel = formatPhase(state?.phase);
  const stepLabel = formatStep(state?.step);
  const hasStep = stepLabel && stepLabel !== "None";
  const title = hasStep ? `${phaseLabel}: ${stepLabel}` : phaseLabel;
  const actionSummary = summarizeAvailableActions(decision);
  const guidance = currentPhaseGuidance(state);
  const nextLabel = cleanAdvanceLabel(advanceLabel);

  return {
    title,
    description: `${guidance} ${actionSummary} The main button advances to ${nextLabel}.`,
  };
}

export default function PhaseHelpPopover({
  state,
  decision,
  advanceLabel,
  className = "",
}) {
  const help = useMemo(
    () => phaseHelpContent(state, decision, advanceLabel),
    [advanceLabel, decision, state]
  );

  return (
    <ComicTooltip
      title={help.title}
      description={help.description}
      side="top"
      align="end"
      sideOffset={7}
      contentClassName="max-w-[320px]"
      persistOnOutsideInteraction={(event) => {
        const target = event?.target;
        return target instanceof Element && !!target.closest(".decision-main-button");
      }}
    >
      <button
        type="button"
        className={cn("decision-phase-help-button", className)}
        aria-label="Explain current phase"
        data-decision-phase-help
        onPointerDown={(event) => {
          if (event.button != null && event.button !== 0) return;
          event.stopPropagation();
        }}
        onClick={(event) => event.stopPropagation()}
      >
        <CircleHelp className="h-4 w-4" aria-hidden="true" />
      </button>
    </ComicTooltip>
  );
}
