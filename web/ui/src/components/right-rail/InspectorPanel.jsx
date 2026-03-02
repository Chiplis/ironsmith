import { useEffect, useState, useCallback } from "react";
import { useGame } from "@/context/GameContext";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";

export default function InspectorPanel({ selectedObjectId }) {
  const { game } = useGame();
  const [details, setDetails] = useState(null);

  const loadDetails = useCallback(
    async (id) => {
      if (!game || !id) {
        setDetails(null);
        return;
      }
      try {
        const d = await game.objectDetails(BigInt(id));
        setDetails(d);
      } catch (err) {
        console.warn("objectDetails failed:", err);
        setDetails(null);
      }
    },
    [game]
  );

  useEffect(() => {
    loadDetails(selectedObjectId);
  }, [selectedObjectId, loadDetails]);

  return (
    <section className="p-2 border-b border-game-line-2 flex flex-col gap-1.5 min-h-0 overflow-hidden">
      <h3 className="m-0 text-sm font-bold">Object Inspector</h3>
      {!details ? (
        <div className="text-muted-foreground text-[12px] italic">
          Click an object to inspect it
        </div>
      ) : (
        <ScrollArea className="min-h-0 flex-1">
          <div className="grid gap-1.5 text-[11px] pr-1">
            <div className="font-bold text-[13px]">{details.name || "Unknown"}</div>
            {details.type_line && (
              <div className="text-muted-foreground">{details.type_line}</div>
            )}
            {details.power_toughness && (
              <div>P/T: {details.power_toughness}</div>
            )}
            {details.zone && <div>Zone: {details.zone}</div>}
            {details.controller != null && <div>Controller: Player {details.controller}</div>}

            {details.oracle_text && (
              <Collapsible defaultOpen>
                <CollapsibleTrigger className="text-[10px] uppercase tracking-wider text-muted-foreground font-bold cursor-pointer hover:text-foreground">
                  Oracle Text
                </CollapsibleTrigger>
                <CollapsibleContent>
                  <pre className="whitespace-pre-wrap text-[11px] text-[#c0d8f0] bg-[#0a1118] border border-[#1e3044] p-1.5 mt-0.5 font-[inherit]">
                    {details.oracle_text}
                  </pre>
                </CollapsibleContent>
              </Collapsible>
            )}

            {details.compiled_text && (
              <Collapsible>
                <CollapsibleTrigger className="text-[10px] uppercase tracking-wider text-muted-foreground font-bold cursor-pointer hover:text-foreground">
                  Compiled Text
                </CollapsibleTrigger>
                <CollapsibleContent>
                  <pre className="whitespace-pre-wrap text-[11px] text-[#a8c8e4] bg-[#0a1118] border border-[#1e3044] p-1.5 mt-0.5 font-[inherit]">
                    {details.compiled_text}
                  </pre>
                </CollapsibleContent>
              </Collapsible>
            )}

            {details.abilities && details.abilities.length > 0 && (
              <Collapsible>
                <CollapsibleTrigger className="text-[10px] uppercase tracking-wider text-muted-foreground font-bold cursor-pointer hover:text-foreground">
                  Abilities ({details.abilities.length})
                </CollapsibleTrigger>
                <CollapsibleContent>
                  <div className="grid gap-1 mt-0.5">
                    {details.abilities.map((ab, i) => (
                      <div key={i} className="bg-[#0a1118] border border-[#1e3044] p-1 text-[11px]">
                        {ab.text || ab.kind || `Ability ${i + 1}`}
                      </div>
                    ))}
                  </div>
                </CollapsibleContent>
              </Collapsible>
            )}
          </div>
        </ScrollArea>
      )}
    </section>
  );
}
