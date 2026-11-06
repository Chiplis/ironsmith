import { useLayoutEffect, useRef } from "react";
import { BACK_FX, registerKeywordAura, setKeywordFxQuality } from "@/lib/keyword-fx/keyword-fx";
import useGraphicsQuality from "@/hooks/useGraphicsQuality";

// GPU keyword aura for one battlefield card. The canvases are filled by the
// shared keyword-fx renderer; this component only owns their DOM slots.
export default function KeywordAura({ flags, seed = 0, protection = null }) {
  const [quality] = useGraphicsQuality();
  const back = useRef(null), front = useRef(null), handle = useRef(null);
  const hasBack = Boolean(flags & BACK_FX);
  const enabled = quality !== "off" && flags > 0;
  setKeywordFxQuality(quality);

  useLayoutEffect(() => {
    if (!enabled) return undefined;
    const card = front.current?.closest(".game-card");
    handle.current = registerKeywordAura({
      card, back: back.current, front: front.current, spec: { flags, seed, protection },
    });
    return () => { handle.current?.dispose(); handle.current = null; };
    // Re-register only when a canvas slot appears or disappears.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled, hasBack]);

  useLayoutEffect(() => {
    handle.current?.update({ flags, seed, protection });
  }, [flags, seed, protection]);

  if (!enabled) return null;
  return (
    <>
      {hasBack && <canvas ref={back} className="keyword-aura keyword-aura--back" aria-hidden="true" />}
      <canvas ref={front} className="keyword-aura keyword-aura--front" aria-hidden="true" data-keyword-aura={flags} />
    </>
  );
}
