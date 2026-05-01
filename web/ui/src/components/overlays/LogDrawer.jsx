import { useGame } from "@/context/GameContext";
import { Sheet, SheetContent, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { ScrollArea } from "@/components/ui/scroll-area";
import { SymbolText } from "@/lib/mana-symbols";

export default function LogDrawer({ open, onOpenChange }) {
  const { logEntries } = useGame();

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side="right" className="fantasy-sheet fantasy-sheet--log w-[min(92vw,400px)]">
        <SheetHeader className="fantasy-sheet-header pr-12">
          <SheetTitle className="text-[22px] uppercase tracking-[0.18em] text-foreground">
            Game Log
          </SheetTitle>
          <span className="fantasy-sheet-subtitle text-[13px] uppercase tracking-[0.18em]">
            Latest 120 entries
          </span>
        </SheetHeader>
        <ScrollArea className="mt-1 h-[calc(100vh-108px)] px-4 pb-4">
          <ul className="m-0 flex list-none flex-col gap-2 p-0">
            {logEntries.map((entry, i) => (
              <li
                key={i}
                className={`fantasy-log-entry text-[14px] leading-tight ${
                  entry.isError ? "fantasy-log-entry--error text-destructive" : "text-foreground"
                }`}
              >
                <small className="fantasy-log-time mr-2">{entry.time}</small>
                <SymbolText text={entry.message} style={{ whiteSpace: "inherit" }} />
              </li>
            ))}
            {logEntries.length === 0 && (
              <li className="fantasy-sheet-empty p-4 text-center text-[15px] italic">
                No log entries yet
              </li>
            )}
          </ul>
        </ScrollArea>
      </SheetContent>
    </Sheet>
  );
}
