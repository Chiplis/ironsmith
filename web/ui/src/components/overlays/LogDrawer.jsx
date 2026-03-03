import { useGame } from "@/context/GameContext";
import { Sheet, SheetContent, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { ScrollArea } from "@/components/ui/scroll-area";

export default function LogDrawer({ open, onOpenChange }) {
  const { logEntries } = useGame();

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side="right" className="w-[400px] bg-[#0b1017] border-game-line">
        <SheetHeader>
          <SheetTitle className="text-foreground">Game Log</SheetTitle>
          <span className="text-muted-foreground text-[14px]">Latest 120 entries</span>
        </SheetHeader>
        <ScrollArea className="h-[calc(100vh-100px)] mt-4">
          <ul className="list-none p-0 m-0 flex flex-col gap-0.5">
            {logEntries.map((entry, i) => (
              <li
                key={i}
                className={`text-[14px] leading-tight py-0.5 px-1 border-b border-game-line-2 ${
                  entry.isError ? "text-destructive" : "text-foreground"
                }`}
              >
                <small className="text-muted-foreground mr-1.5">{entry.time}</small>
                {entry.message}
              </li>
            ))}
            {logEntries.length === 0 && (
              <li className="text-muted-foreground text-[16px] italic p-3 text-center">
                No log entries yet
              </li>
            )}
          </ul>
        </ScrollArea>
      </SheetContent>
    </Sheet>
  );
}
