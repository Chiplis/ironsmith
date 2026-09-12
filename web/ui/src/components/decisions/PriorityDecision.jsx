import useUiText from "@/i18n/useUiText";
import { useGame } from "@/context/GameContext";
import { SymbolText } from "@/lib/mana-symbols";
import { getVisibleTopStackObject } from "@/lib/stack-targets";

export default function PriorityDecision({ decision }) {
  const ui = useUiText();
  const { state } = useGame();
  const topOfStack = getVisibleTopStackObject(state);
  const resolvingAbilityText = topOfStack?.ability_kind
    ? (topOfStack.ability_text || topOfStack.effect_text || null)
    : null;

  const checking = decision?.analysis_complete === false;
  if (!resolvingAbilityText && !checking) return null;

  return (
    <div className="px-1 pb-0.5">
      {checking && <span role="status" className="text-[12px] text-[#9fc2e4] block">{ui("Checking available actions…")}</span>}
      {resolvingAbilityText && <SymbolText
        text={resolvingAbilityText}
        className="text-[12px] text-[#9fc2e4] leading-snug font-[inherit] block"
      />}
    </div>
  );
}
