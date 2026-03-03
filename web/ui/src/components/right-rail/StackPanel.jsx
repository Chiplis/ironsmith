import { useGame } from "@/context/GameContext";
import { ScrollArea } from "@/components/ui/scroll-area";
import StackCard from "@/components/cards/StackCard";

export default function StackPanel({ onInspect }) {
  const { state, status, setStatus } = useGame();
  const objects = state?.stack_objects || [];
  const previews = state?.stack_preview || [];

  return (
    <section className="flex-1 min-h-0 p-2 flex flex-col gap-1.5 overflow-hidden">
      <h4 className="m-0 text-[#8ec4ff] uppercase tracking-widest text-[16px] font-bold shrink-0">
        The Stack
      </h4>
      <ScrollArea className="flex-1 min-h-0">
        <div className="grid gap-1.5 pr-0.5">
          {objects.length > 0
            ? objects.map((entry) => (
                <StackCard key={entry.id} entry={entry} onClick={onInspect} />
              ))
            : previews.length > 0
              ? previews.map((name, i) => (
                  <div
                    key={i}
                    className="game-card w-full min-w-0 min-h-[60px] text-[14px] border-[#80a8d7] bg-gradient-to-b from-[#132237] to-[#0d1726] p-1.5"
                  >
                    <span className="relative z-2 leading-[1.12] text-shadow-[0_1px_1px_rgba(0,0,0,0.85)]">{name}</span>
                  </div>
                ))
              : null}
        </div>
      </ScrollArea>
      {status.msg && (
        <div
          className="text-[14px] shrink-0 px-1 py-0.5 break-words relative"
          style={{ color: status.isError ? "#ffb5c5" : "#d5e4f8" }}
        >
          {status.msg}
          {status.isError && (
            <span
              className="absolute top-0 right-0 cursor-pointer text-[#f76969] hover:text-[#ff9999] px-1 leading-none text-[16px]"
              onClick={() => setStatus("")}
            >
              ×
            </span>
          )}
        </div>
      )}
    </section>
  );
}
