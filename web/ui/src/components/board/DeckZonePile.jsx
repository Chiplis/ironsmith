function normalizeCount(value) {
  const count = Number(value);
  return Number.isFinite(count) ? Math.max(0, Math.floor(count)) : 0;
}

export default function DeckZonePile({ count }) {
  const deckCount = normalizeCount(count);
  const stackDepth = Math.min(Math.max(deckCount, 1), 4);

  return (
    <section className="deck-zone-pile relative h-full min-h-[148px] overflow-hidden rounded-none border border-[#34485f] bg-[linear-gradient(180deg,rgba(12,20,31,0.98),rgba(7,12,19,0.98))] shadow-[inset_0_1px_0_rgba(170,208,245,0.08),0_10px_24px_rgba(0,0,0,0.22)]">
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top,rgba(112,171,232,0.12),transparent_58%)]" />
      <div className="flex h-full min-h-0 flex-col">
        <div className="flex flex-1 items-center justify-center px-4 pb-12 pt-3">
          <div
            className="relative"
            style={{ width: "min(100%, 122px)", aspectRatio: "63 / 88" }}
            aria-hidden="true"
          >
            {Array.from({ length: stackDepth }).map((_, index) => {
              const offset = (stackDepth - index - 1) * 5;
              const rotation = (stackDepth - index - 1) * 1.2;
              const isFront = index === stackDepth - 1;
              return (
                <div
                  key={index}
                  className="absolute inset-0 overflow-hidden rounded-none border border-[#8eb1d7]/20 bg-[linear-gradient(180deg,rgba(18,30,45,0.98),rgba(7,12,18,0.98)),repeating-linear-gradient(145deg,rgba(255,255,255,0.05)_0_2px,transparent_2px_6px)] shadow-[0_8px_20px_rgba(0,0,0,0.3),inset_0_0_0_1px_rgba(255,255,255,0.04)]"
                  style={{
                    transform: `translate(${offset}px, ${offset * -0.85}px) rotate(${rotation}deg)`,
                    transformOrigin: "50% 100%",
                  }}
                >
                  <div className="absolute inset-[10px] rounded-none border border-[#d8ebff]/10 bg-[linear-gradient(180deg,rgba(18,34,51,0.76),rgba(8,14,22,0.92))]" />
                  <div className="absolute inset-x-[13px] top-[14px] h-[18%] rounded-none border border-[#9cc4ed]/12 bg-[linear-gradient(180deg,rgba(32,61,89,0.84),rgba(13,24,36,0.96))]" />
                  <div className="absolute inset-x-[13px] top-[37%] h-px bg-[rgba(158,196,232,0.14)]" />
                  <div className="absolute inset-x-[13px] top-[48%] h-px bg-[rgba(158,196,232,0.11)]" />
                  <div className="absolute inset-x-[13px] top-[59%] h-px bg-[rgba(158,196,232,0.09)]" />
                  {isFront && (
                    <div className="absolute inset-x-0 bottom-[10px] text-center text-[12px] font-bold uppercase tracking-[0.28em] text-[#d5e7fb]">
                      Deck
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>
        <div className="relative z-[1] border-t border-[#32485f]/85 bg-[linear-gradient(180deg,rgba(7,12,18,0.82),rgba(4,8,13,0.96))] px-3 py-2">
          <div className="flex items-end justify-between gap-3">
            <div className="min-w-0">
              <div className="text-[11px] font-semibold uppercase tracking-[0.22em] text-[#8eb1d7]">
                Deck
              </div>
              <div className="text-[12px] text-[#a9c1dc]">
                {deckCount === 0 ? "Empty" : `${deckCount} card${deckCount === 1 ? "" : "s"}`}
              </div>
            </div>
            <div className="deck-zone-count rounded-none border border-[#4d6987] bg-[rgba(10,18,27,0.94)] px-2.5 py-1 text-[18px] font-bold leading-none text-[#e3efff] shadow-[inset_0_1px_0_rgba(255,255,255,0.05)]">
              {deckCount}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
