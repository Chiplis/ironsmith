import { useEffect, useState } from "react";
import { useI18n } from "./I18nContext";
import { loadTranslatedCardView } from "./cardTranslations";

const faceViews = (card) => (Array.isArray(card?.card_faces) && card.card_faces.length
  ? card.card_faces
  : [card]);

const withView = (face, view) => (view ? {
  ...face,
  name: view.name || face.name,
  ...(view.typeLine ? { type_line: view.typeLine } : {}),
  ...(view.rulesText ? { oracle_text: String(view.rulesText).replace(/\\n/g, "\n") } : {}),
} : face);

// Hand frames print the card, so in a non-English locale they show the
// official localized name, type line and rules text. English stays the card's
// identity (art, lookups); only the frame's printed fields are replaced, and
// the English frame renders while a translation loads or when none exists.
export function useLocalizedHandFrameCard(card, enabled = true) {
  const { locale } = useI18n();
  const faces = faceViews(card);
  const key = [locale, card?.oracle_id || card?.oracleId || "", ...faces.map((f) =>
    `${f?.name || ""}\u0001${f?.type_line || ""}\u0001${f?.oracle_text || ""}`)].join("\u0002");
  const [loaded, setLoaded] = useState(null);

  useEffect(() => {
    if (!enabled || locale === "en" || !card?.name) return undefined;
    let cancelled = false;
    const multi = Array.isArray(card.card_faces) && card.card_faces.length > 0;
    Promise.all(faces.map((face) => loadTranslatedCardView(locale, {
      name: face.name,
      typeLine: face.type_line,
      rulesText: face.oracle_text,
      oracleId: multi ? "" : card.oracle_id || card.oracleId || "",
    }).catch(() => null))).then((views) => {
      if (!cancelled) setLoaded({ key, views });
    });
    return () => { cancelled = true; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, enabled, locale]);

  if (!enabled || locale === "en" || loaded?.key !== key || !loaded.views.some(Boolean)) return card;
  if (Array.isArray(card.card_faces) && card.card_faces.length) {
    const translatedFaces = card.card_faces.map((face, i) => withView(face, loaded.views[i]));
    const top = card.card_faces.findIndex((face) => face.name === card.name);
    return { ...card, card_faces: translatedFaces, ...(top >= 0 ? { name: translatedFaces[top].name } : {}) };
  }
  return withView(card, loaded.views[0]);
}
