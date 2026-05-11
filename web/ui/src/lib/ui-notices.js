export const UI_NOTICE_EVENT = "ironsmith:ui-notice";

export function emitUiNotice(notice) {
  if (typeof window === "undefined" || typeof window.dispatchEvent !== "function") {
    return;
  }

  window.dispatchEvent(
    new CustomEvent(UI_NOTICE_EVENT, {
      detail: notice,
    })
  );
}

export function emitSyncFailureNotice(title, body) {
  const notice =
    body && typeof body === "object" && !Array.isArray(body)
      ? body
      : { body };
  emitUiNotice({
    tone: "error",
    title,
    ...notice,
  });
}
