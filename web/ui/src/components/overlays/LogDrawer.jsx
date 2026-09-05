import { useMemo, useState } from "react";
import { useGame } from "@/context/GameContext";
import { Sheet, SheetContent, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { ScrollArea } from "@/components/ui/scroll-area";
import { SymbolText } from "@/lib/mana-symbols";

export default function LogDrawer({ open, onOpenChange }) {
  const { logEntries } = useGame();
  const [showRoutine, setShowRoutine] = useState(false);
  const visibleEntries = useMemo(() => {
    if (showRoutine) return logEntries;
    return logEntries.filter((entry) => (
      entry.isError
      || (
        !/^(Refreshed|Preparing|WASM|Registry|Compiled|Auto-pass (enabled|disabled|held))/i.test(
          String(entry.message || "").trim()
        )
        && !/^Pass priority\b.*\bauto-passed\b/i.test(String(entry.message || "").trim())
      )
    ));
  }, [logEntries, showRoutine]);
  const displayMessage = (message) => (
    showRoutine
      ? message
      : String(message || "").replace(/\s*•\s*auto-passed\s+x\d+\s*$/i, "")
  );

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side="right" className="fantasy-sheet fantasy-sheet--log w-[min(92vw,400px)]">
        <SheetHeader className="fantasy-sheet-header pr-12">
          <SheetTitle className="text-[22px] tracking-[0.08em] text-foreground">
            Game Log
          </SheetTitle>
          <div className="fantasy-log-toolbar">
            <span className="fantasy-sheet-subtitle text-[13px]">
              {visibleEntries.length} of {logEntries.length} recent entries
            </span>
            <button
              type="button"
              className="stone-pill fantasy-log-filter px-2 py-1 text-[12px]"
              aria-pressed={showRoutine}
              onClick={() => setShowRoutine((current) => !current)}
            >
              {showRoutine ? "Hide system events" : "Show system events"}
            </button>
          </div>
        </SheetHeader>
        <ScrollArea className="mt-1 min-h-0 flex-1 px-4 pb-4">
          <ul className="m-0 flex list-none flex-col gap-2 p-0">
            {visibleEntries.map((entry, i) => (
              <li
                key={i}
                className={`fantasy-log-entry text-[14px] leading-tight ${
                  entry.isError ? "fantasy-log-entry--error text-destructive" : "text-foreground"
                }`}
              >
                <small className="fantasy-log-time mr-2">{entry.time}</small>
                <SymbolText text={displayMessage(entry.message)} style={{ whiteSpace: "inherit" }} />
              </li>
            ))}
            {visibleEntries.length === 0 && (
              <li className="fantasy-sheet-empty p-4 text-center text-[15px] italic">
                {logEntries.length === 0 ? "No log entries yet" : "No gameplay entries yet"}
              </li>
            )}
          </ul>
        </ScrollArea>
      </SheetContent>
    </Sheet>
  );
}
