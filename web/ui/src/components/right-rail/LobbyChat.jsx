import { useCallback, useEffect, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import useUiText from "@/i18n/useUiText";

const EMPTY_MESSAGES = [];
const MAX_CHAT_LENGTH = 120;
const CHAT_CLOSE_DELAY_MS = 4000;
const CHAT_EMOJI_PATTERNS = [
  new RegExp("\\p{Extended_Pictographic}", "u"),
  new RegExp("\\p{Emoji_Presentation}", "u"),
  new RegExp("\\p{Emoji_Modifier}", "u"),
  new RegExp("\\p{Regional_Indicator}", "u"),
];

function cleanChatDraft(value) {
  return Array.from(String(value ?? "").normalize("NFKC"))
    .filter((character) => {
      const codePoint = character.codePointAt(0);
      if (codePoint <= 0x1f || codePoint === 0x7f
        || codePoint === 0x200b || codePoint === 0x200c || codePoint === 0x200d
        || codePoint === 0xfe0f || codePoint === 0xfeff
        || character === "<" || character === ">") return false;
      return !CHAT_EMOJI_PATTERNS.some((pattern) => pattern.test(character));
    })
    .join("")
    .replace(/\s+/g, " ")
    .slice(0, MAX_CHAT_LENGTH);
}

function hasOpenLocalZone() {
  return Boolean(document.querySelector('[data-local-zone-strip="true"][data-state="open"]'));
}

function isLocalZoneSurface(target) {
  return target instanceof Element && Boolean(target.closest(
    '[data-local-zone-piles="true"] [data-zone-pile="graveyard"], '
      + '[data-local-zone-piles="true"] [data-zone-pile="exile"], '
      + '[data-local-zone-strip="true"]',
  ));
}

export default function LobbyChat() {
  const { multiplayer, sendLobbyChat } = useGame();
  const ui = useUiText();
  const [draft, setDraft] = useState("");
  const [error, setError] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const [seenMessageId, setSeenMessageId] = useState("");
  const chatRef = useRef(null);
  const closeTimerRef = useRef(null);
  const blockedOpenRef = useRef(false);
  const listRef = useRef(null);
  const followRef = useRef(true);
  const messages = multiplayer?.chatMessages || EMPTY_MESSAGES;
  const latestMessageId = messages.at(-1)?.id || "";
  const hasUnread = !expanded && Boolean(latestMessageId && latestMessageId !== seenMessageId);
  const collapsed = !expanded;

  const cancelCollapse = useCallback(() => {
    if (closeTimerRef.current) window.clearTimeout(closeTimerRef.current);
    closeTimerRef.current = null;
  }, []);
  const scheduleCollapse = useCallback(() => {
    cancelCollapse();
    closeTimerRef.current = window.setTimeout(() => {
      setExpanded(false);
      if (latestMessageId) setSeenMessageId(latestMessageId);
      closeTimerRef.current = null;
    }, CHAT_CLOSE_DELAY_MS);
  }, [cancelCollapse, latestMessageId]);
  const closeImmediately = useCallback(() => {
    cancelCollapse();
    setExpanded(false);
    if (latestMessageId) setSeenMessageId(latestMessageId);
  }, [cancelCollapse, latestMessageId]);
  useEffect(() => () => cancelCollapse(), [cancelCollapse]);
  useEffect(() => {
    const handlePointerDown = (event) => {
      const insideChat = Boolean(chatRef.current?.contains(event.target));
      const zoneOpen = hasOpenLocalZone();
      blockedOpenRef.current = zoneOpen && insideChat;
      if (zoneOpen && insideChat && expanded) closeImmediately();
      else if (expanded && !insideChat) closeImmediately();
    };
    const handlePointerOver = (event) => {
      if (expanded && isLocalZoneSurface(event.target)) closeImmediately();
    };
    document.addEventListener("pointerdown", handlePointerDown, true);
    document.addEventListener("pointerover", handlePointerOver, true);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown, true);
      document.removeEventListener("pointerover", handlePointerOver, true);
    };
  }, [expanded, closeImmediately]);
  useEffect(() => {
    if (followRef.current && listRef.current) {
      listRef.current.scrollTop = listRef.current.scrollHeight;
    }
  }, [messages, collapsed]);
  if (!multiplayer?.role) return null;
  return <section className="lobby-chat" data-collapsed={collapsed} aria-label={ui("Lobby chat")}
    ref={chatRef}
    onKeyDown={(event) => event.stopPropagation()}
    onPointerDown={(event) => { event.stopPropagation(); cancelCollapse(); }}
    onPointerEnter={cancelCollapse}
    onPointerLeave={() => { if (expanded) scheduleCollapse(); }}
    onFocus={cancelCollapse}
    onBlur={(event) => {
      if (!event.currentTarget.contains(event.relatedTarget)) scheduleCollapse();
    }}>
    <button type="button" className="lobby-chat-header" aria-expanded={!collapsed}
      onClick={() => {
        const blocked = blockedOpenRef.current || hasOpenLocalZone();
        blockedOpenRef.current = false;
        if (blocked) return;
        cancelCollapse();
        setExpanded((current) => {
          const next = !current;
          if (latestMessageId) setSeenMessageId(latestMessageId);
          return next;
        });
      }}>
      {ui("Lobby chat")}
      <span className="lobby-chat-header-actions">
        {hasUnread && <span className="lobby-chat-unread" aria-label={ui("Unread messages")} />}
        <span aria-hidden="true">{collapsed ? "▴" : "▾"}</span>
      </span>
    </button>
    <div className="lobby-chat-body" inert={collapsed}>
    <div className="lobby-chat-body-inner">
    <div ref={listRef} className="lobby-chat-messages" role="log" aria-live="polite"
      aria-relevant="additions" onScroll={(event) => {
        const el = event.currentTarget;
        followRef.current = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
      }}>
      {messages.length ? messages.map((message) => <p key={message.id}>
        <strong data-self={message.peerId === multiplayer.localPeerId}>{message.name}</strong>
        <span>{message.text}</span>
      </p>) : <p className="lobby-chat-empty">{ui("Say hello to the table.")}</p>}
    </div>
    <form className="lobby-chat-compose" onSubmit={(event) => {
      event.preventDefault();
      const message = cleanChatDraft(draft);
      if (!message) return;
      if (sendLobbyChat(message)) {
        setDraft("");
        setError(false);
        followRef.current = true;
      } else setError(true);
    }}>
      <input aria-label={ui("Chat message")} placeholder={ui("Message…")} maxLength={MAX_CHAT_LENGTH}
        value={draft} onChange={(event) => setDraft(cleanChatDraft(event.target.value))} />
      <button type="submit" disabled={!draft.trim()} aria-label={ui("Send message")}>{ui("Send")}</button>
    </form>
    {error && <p role="alert">{ui("Unable to send. Try again when connected.")}</p>}
    </div>
    </div>
  </section>;
}
