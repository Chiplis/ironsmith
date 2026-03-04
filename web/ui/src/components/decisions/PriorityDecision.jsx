import { useGame } from "@/context/GameContext";
import { SymbolText } from "@/lib/mana-symbols";

export default function PriorityDecision() {
  const { state } = useGame();
  const stackSize = state?.stack_size || 0;
  const isResolvingStack = stackSize > 0;
  const topOfStack = (state?.stack_objects || [])[0];
  const resolvingAbilityText = isResolvingStack && topOfStack?.ability_kind
    ? (topOfStack.ability_text || topOfStack.effect_text || null)
    : null;

  if (!resolvingAbilityText) return null;

  return (
    <div className="px-1 pb-0.5">
      <SymbolText
        text={resolvingAbilityText}
        className="text-[12px] text-[#9fc2e4] leading-snug font-[inherit] block"
      />
    </div>
  );
}
