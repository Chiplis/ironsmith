import { translateUiText } from "./catalog.js";
import { useCallback, useEffect, useState } from "react";
import { useGame } from "@/context/GameContext";
import { findObjectCardInState } from "@/lib/inspector-selection";
import { translatePrintedPrompt } from "@/lib/decision-text-translation";
import { localizeEnginePhrase } from "./decisionPhrases";
import { useI18n } from "./I18nContext";
import { loadTranslatedCardView } from "./cardTranslations";

// Decision prompts are quoted from the source card's printed text, so they can
// be shown in the player's language by taking the same sentence from that
// card's localized printing. Returns a translate function; it hands back the
// English text unchanged while the translation loads, when the locale is
// English, or whenever the localized text does not line up.
export function useTranslatedDecisionText(decision) {
  const { locale, t } = useI18n();
  const { state } = useGame();
  const source = findObjectCardInState(state, decision?.source_id);
  const cardName = String(source?.name || decision?.source_name || "").trim();
  const englishText = String(source?.oracle_text || "").trim();
  const oracleId = String(source?.oracle_id || source?.oracleId || "").trim();
  const translationKey = `${locale}|${oracleId}|${cardName}|${englishText}`;
  const [translation, setTranslation] = useState(null);

  useEffect(() => {
    if (locale === "en" || !englishText || !cardName) return undefined;

    let cancelled = false;
    loadTranslatedCardView(locale, {
      name: cardName,
      typeLine: String(source?.type_line || "").trim(),
      rulesText: englishText,
      oracleId,
    }).then((view) => {
      if (cancelled || !view) return;
      setTranslation({ key: translationKey, text: view.rulesText || "", name: view.name || "" });
    });

    return () => {
      cancelled = true;
    };
    // `source` is re-derived every render; the fields it contributes are in the key.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [locale, cardName, englishText, oracleId, translationKey]);

  const active = translation?.key === translationKey ? translation : null;
  const translatedText = active?.text || null;
  const translatedName = active?.name || "";

  return useCallback(
    (text) => {
      if (!text || locale === "en") return text;
      // The card's own wording first, then the engine's frame vocabulary.
      const quote = (value) => (
        (translatedText
          && translatePrintedPrompt({ prompt: value, englishText, translatedText, locale }))
        || value
      );
      const quoted = quote(text);
      if (quoted !== text) return quoted;
      return (
        localizeEnginePhrase(text, {
          t,
          cardName: translatedName,
          englishCardName: cardName,
          quote,
        }) || translateUiText(text, null, locale)
      );
    },
    [cardName, englishText, locale, t, translatedName, translatedText]
  );
}

export default useTranslatedDecisionText;
