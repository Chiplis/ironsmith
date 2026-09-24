import { useEffect, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import useUiText from "@/i18n/useUiText";

const EMPTY_MESSAGES = [];
const MAX_CHAT_LENGTH = 240;

export default function LobbyChat() {
  const { multiplayer, sendLobbyChat } = useGame();
  const ui = useUiText();
  const [draft, setDraft] = useState("");
  const [error, setError] = useState(false);
  const [collapsedMessageId, setCollapsedMessageId] = useState("");
  const [reopenCount, setReopenCount] = useState(0);
  const listRef = useRef(null);
  const followRef = useRef(true);
  const messages = multiplayer?.chatMessages || EMPTY_MESSAGES;
  const latestMessageId = messages.at(-1)?.id || "";
  const collapsed = collapsedMessageId === latestMessageId;
  useEffect(() => {
    if (!multiplayer?.role) return;
    const timeout = window.setTimeout(() => setCollapsedMessageId(latestMessageId), 5000);
    return () => window.clearTimeout(timeout);
  }, [latestMessageId, multiplayer?.role, reopenCount]);
  useEffect(() => {
    if (followRef.current && listRef.current) {
      listRef.current.scrollTop = listRef.current.scrollHeight;
    }
  }, [messages, collapsed]);
  if (!multiplayer?.role) return null;
  return <section className="lobby-chat" data-collapsed={collapsed} aria-label={ui("Lobby chat")}
    onKeyDown={(event) => event.stopPropagation()}
    onPointerDown={(event) => event.stopPropagation()}>
    <button type="button" className="lobby-chat-header" aria-expanded={!collapsed}
      onClick={() => {
        setCollapsedMessageId(collapsed ? null : latestMessageId);
        setReopenCount((count) => count + 1);
      }}>{ui("Lobby chat")}<span aria-hidden="true">{collapsed ? "▴" : "▾"}</span></button>
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
      if (!draft.trim()) return;
      if (sendLobbyChat(draft)) {
        setDraft("");
        setError(false);
        followRef.current = true;
      } else setError(true);
    }}>
      <input aria-label={ui("Chat message")} placeholder={ui("Message…")} maxLength={MAX_CHAT_LENGTH}
        value={draft} onChange={(event) => setDraft(event.target.value)} />
      <button type="submit" disabled={!draft.trim()} aria-label={ui("Send message")}>{ui("Send")}</button>
    </form>
    {error && <p role="alert">{ui("Unable to send. Try again when connected.")}</p>}
    </div>
    </div>
  </section>;
}
