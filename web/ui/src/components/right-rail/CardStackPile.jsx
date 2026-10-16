import useUiText from "@/i18n/useUiText";
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useGame } from "@/context/GameContext";
import StackCard from "@/components/cards/StackCard";
import { stackEntryKindLabel } from "@/lib/stack-entry-kind";
import MiniatureCardFrame, {
  CARD_FRAME_RENDER_HEIGHT,
  CARD_FRAME_RENDER_WIDTH,
} from "@/components/cards/MiniatureCardFrame";
import useStackPresence from "@/hooks/useStackPresence";
import useScryfallImageUrl from "@/hooks/useScryfallImageUrl";
import { requestInspectorDetails } from "@/lib/inspector-details-cache";
import { findObjectCardInState, resolveStackInspectObjectId } from "@/lib/inspector-selection";
import { getVisibleStackObjects, stackEntryAimedObjectIds, stackEntryTargetObjectIds } from "@/lib/stack-targets";
import { visibleFrameCards } from "@/lib/stack-frame-preview";
import { selectMseStyle } from "@/lib/mse/style-selection.mjs";
import { cn } from "@/lib/utils";
import { DEFAULT_PLAYER_ACCENT, getPlayerAccent } from "@/lib/player-colors";
import { launchComet } from "@/lib/motion/stack-fx";
import {
  cancelMotion,
  forgetArrivals,
  PILE_EXIT_MS,
  playArrival,
  playExit,
  playPromotion,
  playViewPop,
} from "./card-stack-motion";
import { ChevronLeft, ChevronRight } from "lucide-react";

const CARD_ASPECT = CARD_FRAME_RENDER_WIDTH / CARD_FRAME_RENDER_HEIGHT;
const STANDARD_MSE_STYLE = "magic-m15-altered.mse-style";
// Without the board's stack dock the front card takes this share of the
// table, within these bounds.
const CARD_MIN_HEIGHT = 150;
const CARD_MAX_HEIGHT = 290;
const CARD_TABLE_SHARE = 0.32;
// How far each older entry rises behind the one in front: at least its title
// line, and further when the pointer fans the pile out.
const STEP_SHARE = 0.22;
const MIN_STEP_SHARE = 0.09;
// Older cards rise at most this share of a card above the front one.
const MAX_HEADROOM_SHARE = 0.75;
// Below the front card: the viewed entry's position and the chevrons.
const PAGER_HEIGHT = 32;
const FANNED_STEP_SHARE = 0.34;
// Older entries drift this share of a card width towards the board.
const DRIFT_SHARE = 0.04;
const ABILITY_SCALE = 1;
// An entry the top one targets leaves the pile and sits beside it, this
// share of a card width away, so the arrow between them has room.
const PULL_GAP_SHARE = 0.34;
const PULL_SCALE = 0.94;
// Stack frames render this much larger than the dock/hover-frame size,
// growing upward into the board.
const FRAME_SCALE = 1.33;

// The pile sits where the Stack rail used to: the board's stack dock
// (MyZone keeps an empty .my-zone-stack-rail for it). Layouts without that
// dock fall back to the table.
function measureFrame() {
  const dock = document.querySelector("[data-card-stack-anchor]");
  const dockRect = dock?.getBoundingClientRect();

  const table = document.querySelector(".table-shell")?.getBoundingClientRect();
  const tableRect = table && table.width > 0 && table.height > 0
    ? table
    : new DOMRect(0, 0, window.innerWidth, window.innerHeight);
  let dockBox = dockRect && dockRect.width > 0 && dockRect.height > 0 ? dockRect : null;
  // The front card stops above the local player's header (name, life),
  // where the hover frame ends too. Only things beside the pile count, and
  // none that come and go with decisions (the phase track), so its size
  // holds still.
  const floor = dockBox
    ? [...document.querySelectorAll(".table-shared-player-header, .local-player-mana-dock")]
      .map((element) => element.getBoundingClientRect())
      .filter((rect) => rect.width > 0 && rect.top > dockBox.top
        && rect.left < dockBox.right && rect.right > dockBox.left)
      .reduce((top, rect) => Math.min(top, rect.top - 6), Infinity)
    : Infinity;
  if (dockBox && floor < dockBox.bottom) {
    dockBox = new DOMRect(dockBox.left, dockBox.top, dockBox.width, Math.max(0, floor - dockBox.top));
  }
  return { dock: dockBox, table: tableRect };
}

function sameRect(a, b) {
  if (!a || !b) return a === b;
  return ["left", "top", "width", "height"].every((key) => Math.round(a[key]) === Math.round(b[key]));
}

// Polled each frame while the pile is up: the dock moves with decision bars
// and zone piles without resizing.
function usePileFrame(active) {
  const [frame, setFrame] = useState(null);
  useLayoutEffect(() => {
    if (!active) return undefined;
    let id = null;
    const tick = () => {
      const next = measureFrame();
      setFrame((previous) => (
        previous && sameRect(previous.dock, next.dock) && sameRect(previous.table, next.table)
          ? previous
          : next
      ));
      id = requestAnimationFrame(tick);
    };
    tick();
    return () => cancelAnimationFrame(id);
  }, [active]);
  return frame;
}

// The size FloatingCardPreview gives a hovered card (.floating-card-preview):
// the pile shows each entry at that size, so no second frame is needed.
function hoverFrameHeight() {
  const natural = window.innerWidth <= 1180
    ? 460
    : Math.max(520, Math.min(680, window.innerWidth * 0.32));
  return Math.min(natural, window.innerHeight - 16);
}

function pileGeometry(frame, count, fanned) {
  const { dock, table } = frame;
  let height;
  let left;
  let bottom;
  if (dock) {
    // The bottom strip of the dock holds the position pager.
    height = Math.min(hoverFrameHeight(), Math.max(0, dock.height - PAGER_HEIGHT));
    // Grow upward from the dock's floor, as far as the table allows.
    height = Math.min(height * FRAME_SCALE, Math.max(0, dock.bottom - PAGER_HEIGHT - (Math.max(table.top, 0) + 8)));
    // A gutter keeps the card off the window edge the dock starts at.
    left = Math.max(dock.left, 0) + 12;
    bottom = dock.bottom - PAGER_HEIGHT;
  } else {
    height = Math.max(CARD_MIN_HEIGHT, Math.min(CARD_MAX_HEIGHT, table.height * CARD_TABLE_SHARE)) * FRAME_SCALE;
    left = Math.min(table.left + table.width * 0.78, table.right - (height * CARD_ASPECT) / 2 - 24) - (height * CARD_ASPECT) / 2;
    bottom = table.top + table.height * 0.47 + height / 2;
  }
  height = Math.round(height);
  const width = Math.round(height * CARD_ASPECT);
  // Older entries rise behind the front card, above the dock into the board.
  const tableRoom = Math.max(0, bottom - height - (table.top + 8));
  const headroom = Math.min(tableRoom, height * MAX_HEADROOM_SHARE);
  const gaps = Math.max(1, count - 1);
  const step = fanned
    ? Math.min(height * FANNED_STEP_SHARE, tableRoom / gaps)
    : Math.min(height * STEP_SHARE, Math.max(height * MIN_STEP_SHARE, headroom / gaps), tableRoom / gaps);
  return {
    width,
    height,
    step: count > 1 ? step : 0,
    left: Math.round(left),
    top: Math.round(bottom - height),
  };
}

// The live entries the viewed one targets: they leave the pile to sit beside
// it, so its arrow has somewhere to land.
function pulledEntryKeys(entries, viewedKey) {
  const live = entries.filter((entry) => !entry.__leaving);
  const viewed = live.find((entry) => entry.__timeline_key === viewedKey) || live[0];
  if (!viewed) return [];
  const aimed = new Set(stackEntryAimedObjectIds(viewed).map(String));
  if (aimed.size === 0) return [];
  return live
    .filter((entry) => entry !== viewed && (
      aimed.has(String(entry.id))
      || stackEntryTargetObjectIds(entry).some((id) => aimed.has(String(id)))
    ))
    .map((entry) => entry.__timeline_key);
}

function abilityKindLabel(entry) {
  const kind = String(entry?.ability_kind || "");
  return `${kind.charAt(0).toUpperCase()}${kind.slice(1).toLowerCase()} ability`;
}

// The object the renderer reads: the spell itself, or for an ability its
// source, with the ability's own text in the text box.
// Card frames remembered by stable id from every visible zone, so a spell on
// the stack (or an ability whose source has left) keeps its layout, faces
// and printing -- objectDetails carries none of those.
const rememberedCards = new Map();
const REMEMBERED_LIMIT = 600;

function rememberVisibleCards(state) {
  for (const card of visibleFrameCards(state)) {
    if (card?.stable_id == null) continue;
    const key = String(card.stable_id);
    rememberedCards.delete(key);
    rememberedCards.set(key, card);
  }
  while (rememberedCards.size > REMEMBERED_LIMIT) {
    rememberedCards.delete(rememberedCards.keys().next().value);
  }
}

const SPECIAL_LAYOUTS = new Set([
  "saga", "class", "case", "flip", "leveler", "prototype", "planar", "scheme",
  "battle", "split", "adventure", "meld", "mutate",
]);

// Sagas, planeswalkers, battles, adventures and their kin have frames of
// their own: an ability of one is shown on that frame, never re-dressed.
function usesSpecialFrame(card) {
  if (!card) return false;
  if (SPECIAL_LAYOUTS.has(String(card.layout || "").toLowerCase())) return true;
  return selectMseStyle(card) !== STANDARD_MSE_STYLE;
}

function isLandscapeCard(card) {
  const faces = card?.card_faces;
  const front = Array.isArray(faces) && faces.length ? faces[0] : card;
  return /\b(Battle|Plane|Phenomenon)\b/.test(String(front?.type_line ?? card?.type_line ?? ""));
}

function frameCardForEntry(entry, source) {
  const base = {
    ...(source || {}),
    source_kind: entry.source_kind,
    name: entry.name || source?.name || "",
  };
  if (!entry.ability_kind) {
    return {
      ...base,
      mana_cost: entry.mana_cost ?? source?.mana_cost ?? null,
      oracle_text: source?.oracle_text ?? entry.effect_text ?? "",
    };
  }
  if (usesSpecialFrame(source)) return base;
  const typeLine = abilityKindLabel(entry);
  return {
    ...base,
    mana_cost: null,
    type_line: typeLine,
    type_line_display: typeLine,
    oracle_text: entry.source_ability_text || entry.ability_text || entry.effect_text || "",
    flavor_text: null,
    power: null,
    toughness: null,
    power_toughness: null,
    loyalty: null,
    defense: null,
    card_faces: undefined,
    abilities: undefined,
    compiled_text: undefined,
    ability_text: undefined,
    effect_text: undefined,
  };
}

function useStackEntrySource(entry) {
  const { game, state } = useGame();
  const stateRef = useRef(state);
  stateRef.current = state;
  const leaving = Boolean(entry.__leaving);
  const liveInspectId = resolveStackInspectObjectId(state, entry);
  // A resolving card keeps the face it had: by now its source may be a
  // different object in another zone.
  const inspectIdRef = useRef(liveInspectId);
  if (!leaving) inspectIdRef.current = liveInspectId;
  const inspectId = inspectIdRef.current;
  const [details, setDetails] = useState(null);
  const zoneCard = useMemo(
    () => (inspectId == null ? null : findObjectCardInState(state, inspectId)),
    // Only the source's identity matters here; a fresh snapshot of the same
    // object must not re-render its frame.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [inspectId],
  );
  useEffect(() => {
    if (leaving || !game || inspectId == null) return undefined;
    let active = true;
    requestInspectorDetails(game, stateRef.current, inspectId)
      .then((value) => {
        if (active && value) setDetails({ id: inspectId, value });
      })
      .catch(() => {});
    return () => {
      active = false;
    };
  }, [game, inspectId, leaving]);
  const rememberedKey = String(entry.source_stable_id ?? entry.stable_id ?? "");
  const remembered = useMemo(
    () => rememberedCards.get(rememberedKey) || null,
    // The snapshot taken when the entry (or its source) was last seen.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [rememberedKey, inspectId],
  );
  // The component lives as long as the stack occurrence: when the engine
  // re-issues the entry under a new id, the details already fetched still
  // describe this card, so they stay on screen until the new ones arrive.
  const value = details?.value ?? null;
  const source = value || zoneCard || remembered
    ? { ...(remembered || {}), ...(zoneCard || {}), ...(value || {}) }
    : null;
  return { inspectId, source };
}

// One key per stack occurrence that survives the engine re-issuing an entry:
// a spell by its card, an ability by its source and text, numbered from the
// bottom so identical triggers stay apart.
function stackOccurrenceKeys(liveEntries) {
  const counts = new Map();
  const keys = new Array(liveEntries.length);
  for (let index = liveEntries.length - 1; index >= 0; index -= 1) {
    const entry = liveEntries[index];
    const base = entry.ability_kind
      ? `ability:${entry.source_stable_id ?? entry.inspect_object_id ?? ""}:${entry.ability_text ?? entry.source_ability_text ?? ""}`
      : `spell:${entry.stable_id ?? entry.id}:${entry.name ?? ""}`;
    const count = counts.get(base) || 0;
    counts.set(base, count + 1);
    keys[index] = `${base}#${count}`;
  }
  return keys;
}

function CardStackPileEntry({
  entry,
  occurrenceKey,
  placement,
  isTop,
  isViewed,
  geometry,
  isActive,
  isCasting,
  onInspectObject,
}) {
  const { state, playerAccentOverrides } = useGame();
  const motionRef = useRef(null);
  const animationRef = useRef(null);
  const { inspectId, source } = useStackEntrySource(entry);
  const frameCard = useMemo(() => frameCardForEntry(entry, source), [entry, source]);
  const artUrl = useScryfallImageUrl(entry.source_kind ? "" : entry.name || source?.name || "", "art_crop");
  const isAbility = Boolean(entry.ability_kind);
  const owner = source?.owner ?? entry.controller;
  const accent = getPlayerAccent(state?.players || [], owner, state?.perspective, playerAccentOverrides);
  const cardEntry = useMemo(() => ({ ...entry, __frame_owner: owner }), [entry, owner]);
  const leaving = Boolean(entry.__leaving);

  const rgb = accent?.rgb || DEFAULT_PLAYER_ACCENT.rgb;
  const motionArgs = useRef(null);
  motionArgs.current = { entry, state, inspectId, owner, rgb, occurrenceKey };
  const track = (animation) => {
    cancelMotion(animationRef.current);
    animationRef.current = animation;
  };

  // Arrival: a comet (bolt, wisp, spiral...) from whatever put the entry on
  // the stack, then the frame ignites where it lands.
  useLayoutEffect(() => {
    const node = motionRef.current;
    if (!node || leaving) return undefined;
    const cancel = playArrival({
      node,
      ...motionArgs.current,
      pileNode: node.closest("[data-card-stack-pile]"),
      onMotion: track,
    });
    return () => {
      cancel();
      cancelMotion(animationRef.current);
      animationRef.current = null;
    };
    // Mount only: the arrival plays once per stack occurrence.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Departure, by destination: onto the battlefield, burnt to a pile,
  // shattered by a counter, or (an ability) dispersed.
  useLayoutEffect(() => {
    const node = motionRef.current;
    if (!node || !leaving) return;
    cancelMotion(animationRef.current);
    playExit({ node, ...motionArgs.current, onMotion: track });
  }, [leaving]);

  // Brought into view, or left on top: spring forward / rise with a ring.
  const wasViewed = useRef(isViewed);
  useLayoutEffect(() => {
    if (isViewed && !wasViewed.current && !leaving) {
      const animation = playViewPop(motionRef.current);
      if (animation) track(animation);
    }
    wasViewed.current = isViewed;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isViewed]);
  const wasTop = useRef(isTop);
  useLayoutEffect(() => {
    if (isTop && !wasTop.current && !leaving) playPromotion(motionRef.current, rgb);
    wasTop.current = isTop;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isTop]);

  // Landscape frames (battles, planes) lie on their side in the pile. The
  // type predicts it; a rendered canvas's own aspect settles it.
  const faceRef = useRef(null);
  const [renderedLandscape, setRenderedLandscape] = useState(null);
  useEffect(() => {
    const face = faceRef.current;
    if (!face) return undefined;
    const read = () => {
      const canvas = face.querySelector('canvas[data-mse-frame="ready"]');
      if (canvas?.width > 0 && canvas.height > 0) setRenderedLandscape(canvas.width > canvas.height);
    };
    read();
    const observer = new MutationObserver(read);
    observer.observe(face, { subtree: true, attributes: true, attributeFilter: ["data-mse-frame", "width", "height"] });
    return () => observer.disconnect();
  }, []);
  const landscape = renderedLandscape ?? isLandscapeCard(frameCard);
  const { x, y, depth, pulled } = placement;
  const width = landscape ? geometry.height : geometry.width;
  const height = landscape ? geometry.width : geometry.height;
  // A landscape card keeps the portrait card's center line.
  const landscapeShift = landscape ? (geometry.height - geometry.width) / 2 : 0;
  const scale = (pulled ? PULL_SCALE : 1 - Math.min(depth, 6) * 0.035) * (isAbility ? ABILITY_SCALE : 1);
  const slotStyle = {
    width: `${width}px`,
    height: `${height}px`,
    // The viewed entry comes forward over the cards in front of it.
    zIndex: pulled ? 120 : isViewed ? 110 : 100 - depth,
    transform: `translate3d(${x}px, ${y + landscapeShift}px, 0) scale(${scale})`,
    "--card-stack-depth": pulled ? 0 : depth,
  };

  return (
    <div
      className="card-stack-slot"
      style={slotStyle}
      data-top={isTop ? "true" : undefined}
      data-pulled={pulled ? "true" : undefined}
      data-viewed={isViewed ? "true" : undefined}
      data-leaving={leaving ? "true" : undefined}
      data-ability={isAbility ? "true" : undefined}
      data-casting={isCasting ? "true" : undefined}
      data-landscape={landscape ? "true" : undefined}
      data-stack-key={entry.__timeline_key}
      data-stack-occurrence={occurrenceKey}
      inert={leaving || undefined}
      aria-hidden={leaving || undefined}
    >
      <div ref={motionRef} className="card-stack-motion">
        <div className="card-stack-float">
          <StackCard
            entry={cardEntry}
            variant="frame"
            isLeaving={leaving}
            isActive={isActive}
            onClick={leaving ? undefined : onInspectObject}
            className="card-stack-card"
            arrowRoute={pulled ? "direct" : undefined}
          >
            <span className="card-stack-glow" aria-hidden="true" />
            <span ref={faceRef} className="card-stack-face">
              <MiniatureCardFrame
                card={frameCard}
                imageUrl={artUrl}
                renderWidth={landscape ? CARD_FRAME_RENDER_HEIGHT : CARD_FRAME_RENDER_WIDTH}
                renderHeight={landscape ? CARD_FRAME_RENDER_WIDTH : CARD_FRAME_RENDER_HEIGHT}
              />
            </span>
            <span className="card-stack-sheen" aria-hidden="true" />
          </StackCard>
        </div>
      </div>
    </div>
  );
}

// The desktop layouts’ stack: the entries as a literal pile of rendered
// cards, newest in front, each glowing with its owner's color. One entry is
// "viewed" -- the top one unless a click or the chevrons picked another -- and
// the pager under the pile shows where it sits.
export default function CardStackPile({
  selectedObjectId = null,
  focusedStackObjectId = null,
  onInspectObject = null,
}) {
  const ui = useUiText();
  const { state } = useGame();
  const stackObjects = getVisibleStackObjects(state);
  rememberVisibleCards(state);
  // The engine re-issues an entry under a new id as its casting completes
  // or its targets are chosen. Same occurrence, same card: the old one goes
  // quietly and the new one takes its place without a second entrance.
  // Leaving entries are copies of their last live selves, so they keep the
  // occurrence key stamped here.
  const keyedObjects = useMemo(() => {
    const keys = stackOccurrenceKeys(stackObjects);
    return stackObjects.map((entry, index) => ({ ...entry, __occurrence: keys[index] }));
  }, [stackObjects]);
  const presentEntries = useStackPresence(keyedObjects, PILE_EXIT_MS);
  const entries = useMemo(() => {
    const live = new Set(keyedObjects.map((entry) => entry.__occurrence));
    return presentEntries.filter((entry) => !(entry.__leaving && live.has(entry.__occurrence)));
  }, [keyedObjects, presentEntries]);
  const active = entries.length > 0;
  const frame = usePileFrame(active);
  const [fanned, setFanned] = useState(false);
  const live = useMemo(() => entries.filter((entry) => !entry.__leaving), [entries]);
  const liveCount = live.length;
  useEffect(() => {
    if (stackObjects.length === 0) forgetArrivals();
  }, [stackObjects.length]);
  const viewedIndex = useMemo(() => {
    const focused = focusedStackObjectId == null ? null : String(focusedStackObjectId);
    const index = focused == null ? -1 : live.findIndex((entry) => String(entry.id) === focused);
    return Math.max(0, index);
  }, [focusedStackObjectId, live]);
  const viewedEntry = live[viewedIndex] ?? null;
  const viewedKey = viewedEntry?.__timeline_key ?? null;
  // Stepping through the stack: a spark hops from the card left behind to
  // the one now in view, and the pager's number rolls the same way.
  const previousView = useRef({ key: viewedKey, index: viewedIndex });
  const [viewDirection, setViewDirection] = useState(0);
  useLayoutEffect(() => {
    const previous = previousView.current;
    previousView.current = { key: viewedKey, index: viewedIndex };
    if (previous.key == null || viewedKey == null || previous.key === viewedKey) return;
    setViewDirection(Math.sign(viewedIndex - previous.index));
    const slot = (key) => document.querySelector(`[data-card-stack-pile] [data-stack-key="${CSS.escape(key)}"] .card-stack-motion`);
    const from = slot(previous.key);
    const to = slot(viewedKey);
    if (!from || !to || from.closest("[data-leaving]")) return;
    const accent = to.querySelector(".stack-card--frame");
    const rgb = accent ? getComputedStyle(accent).getPropertyValue("--glow-rgb").trim() : "";
    launchComet({
      from: from.getBoundingClientRect(),
      to: () => to.getBoundingClientRect(),
      rgb: rgb || DEFAULT_PLAYER_ACCENT.rgb,
      style: "spark",
    });
  }, [viewedIndex, viewedKey]);
  // The spell whose casting is underway (targets, costs): its rim races.
  const decision = state?.decision;
  const castingId = decision && decision.kind !== "priority" ? decision.source_id ?? null : null;
  const pulledKeys = useMemo(() => pulledEntryKeys(entries, viewedKey), [entries, viewedKey]);
  const pileCount = entries.length - pulledKeys.length;
  // A lone card has nothing to fan.
  const pileFanned = fanned && pileCount > 1;
  const geometry = useMemo(
    () => (frame ? pileGeometry(frame, pileCount, pileFanned) : null),
    [frame, pileCount, pileFanned],
  );
  // Pile entries count up from the front card; pulled ones line up beside
  // the viewed card, to the board side of it.
  const placements = useMemo(() => {
    if (!geometry) return new Map();
    const result = new Map();
    let depth = 0;
    for (const entry of entries) {
      if (pulledKeys.includes(entry.__timeline_key)) continue;
      result.set(entry.__timeline_key, {
        pulled: false,
        depth,
        x: depth * geometry.width * DRIFT_SHARE,
        y: -depth * geometry.step,
      });
      depth += 1;
    }
    const viewedY = result.get(viewedKey)?.y ?? 0;
    pulledKeys.forEach((key, index) => {
      result.set(key, {
        pulled: true,
        depth: 0,
        x: geometry.width * (1 + PULL_GAP_SHARE),
        y: viewedY + index * geometry.height * 0.22,
      });
    });
    return result;
  }, [entries, geometry, pulledKeys, viewedKey]);

  // The stack frame (FloatingCardPreview) sits right of the dock box: widen
  // that box to everything the pile covers, pulled-out cards included.
  const extent = geometry && frame?.dock
    ? Math.ceil(geometry.left - frame.dock.left + Math.max(
      geometry.width * (1 + Math.max(0, pileCount - 1) * DRIFT_SHARE),
      pulledKeys.length > 0 ? geometry.width * (1 + PULL_GAP_SHARE + (1 + PULL_SCALE) / 2) : 0,
    ) + 12)
    : null;
  useLayoutEffect(() => {
    const dock = document.querySelector("[data-card-stack-anchor]");
    if (!dock) return undefined;
    if (extent == null) dock.style.removeProperty("--card-stack-extent");
    else dock.style.setProperty("--card-stack-extent", `${extent}px`);
    return () => dock.style.removeProperty("--card-stack-extent");
  }, [extent]);

  const selectedKey = useMemo(() => {
    const selected = selectedObjectId == null ? null : String(selectedObjectId);
    const match = selected != null && live.find((entry) => (
      String(entry.inspect_object_id ?? entry.id) === selected || String(entry.id) === selected
    ));
    return match ? match.__timeline_key : null;
  }, [live, selectedObjectId]);

  if (!active || !geometry || typeof document === "undefined") return null;

  const topKey = live[0]?.__timeline_key;
  const countLabel = ui("Stack entries: {0}", { 0: liveCount });
  // A chevron only opens the entry: it never switches the hotseat seat or
  // picks a target the way a click on the card can.
  const viewEntry = (index) => {
    const entry = live[index];
    if (!entry) return;
    onInspectObject?.(entry.inspect_object_id ?? entry.id, {
      source: "stack",
      stackEntry: entry,
      detailOnly: true,
    });
  };
  const place = liveCount > 1
    ? (viewedIndex === 0 ? "Top" : viewedIndex === liveCount - 1 ? "Bottom" : null)
    : null;

  return createPortal(
    <section
      className={cn("card-stack-pile", pileFanned && "card-stack-pile--fanned")}
      style={{
        left: `${geometry.left}px`,
        top: `${geometry.top}px`,
        width: `${geometry.width}px`,
        height: `${geometry.height}px`,
      }}
      data-card-stack-pile="true"
      data-stack-preview-anchor="true"
      data-inspector-stack-timeline
      aria-label={ui("Stack")}
    >
      <span className="sr-only">{countLabel}</span>
      <div
        className="card-stack-cards"
        onMouseEnter={() => setFanned(true)}
        onMouseLeave={() => setFanned(false)}
      >
        {entries.map((entry) => (
          <CardStackPileEntry
            // By occurrence, not by the engine's id: a spell re-issued as its
            // casting completes keeps this component and its rendered frame,
            // so finishing a payment changes nothing on screen.
            key={entry.__occurrence}
            entry={entry}
            occurrenceKey={entry.__occurrence}
            placement={placements.get(entry.__timeline_key)}
            isTop={entry.__timeline_key === topKey}
            isViewed={entry.__timeline_key === viewedKey}
            geometry={geometry}
            isActive={selectedKey != null && entry.__timeline_key === selectedKey}
            isCasting={castingId != null && !entry.ability_kind && (
              String(castingId) === String(entry.id) || String(castingId) === String(entry.inspect_object_id)
            )}
            onInspectObject={onInspectObject}
          />
        ))}
      </div>
      {entries.length > 0 ? (
        <nav
          className="card-stack-pager"
          aria-label={ui("Stack position")}
          data-empty={liveCount === 0 ? "true" : undefined}
        >
          <button
            type="button"
            className="card-stack-pager-button"
            disabled={viewedIndex <= 0}
            onClick={() => viewEntry(viewedIndex - 1)}
            aria-label={ui("Previous stack entry")}
            title={ui("Toward the top of the stack")}
          >
            <ChevronLeft aria-hidden="true" />
          </button>
          <span className="card-stack-pager-label" aria-live="polite">
            {viewedEntry ? (
              <span key={`kind-${viewedEntry.__occurrence}`} className="card-stack-pager-kind">
                {ui(stackEntryKindLabel(viewedEntry))}
              </span>
            ) : null}
            <span className="card-stack-pager-roll">
              <span key={viewedIndex} className="card-stack-pager-index" data-direction={viewDirection}>
                {viewedIndex + 1}
              </span>
            </span>
            <span key={`total-${liveCount}`} className="card-stack-pager-total">/ {Math.max(liveCount, 1)}</span>
            {place ? <span key={place} className="card-stack-pager-place">{ui(place)}</span> : null}
          </span>
          <button
            type="button"
            className="card-stack-pager-button"
            disabled={viewedIndex >= liveCount - 1}
            onClick={() => viewEntry(viewedIndex + 1)}
            aria-label={ui("Next stack entry")}
            title={ui("Toward the bottom of the stack")}
          >
            <ChevronRight aria-hidden="true" />
          </button>
        </nav>
      ) : null}
    </section>,
    document.body,
  );
}
