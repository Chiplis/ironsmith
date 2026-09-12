import useUiText from "@/i18n/useUiText";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { useHover } from "@/context/HoverContext";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/i18n/I18nContext";
import { useTranslatedCardNames } from "@/i18n/useTranslatedCardName";
import { cn } from "@/lib/utils";
import DecisionSummary from "./DecisionSummary";
import HighlightedDecisionText from "./HighlightedDecisionText";
import { decisionOptionAccentVars, getPlayerAccent } from "@/lib/player-colors";
import { buildObjectControllerById } from "@/lib/decision-object-meta";
import {
  SELECT_OBJECT_CHOICE_EVENT,
  isObjectChosen,
} from "@/lib/object-selection";
import {
  useChosenObjectIds,
  useObjectSelectionActions,
} from "@/context/ObjectSelectionContext";

const STRIP_ITEM_BASE_CLASS = "decision-option-row decision-option-row--strip h-8 max-w-[360px] min-w-[120px] shrink-0 justify-start self-stretch px-2.5 text-[12px] font-semibold";
const STRIP_ITEM_ACTIVE_CLASS = "is-selected";
const STRIP_ITEM_DISABLED_CLASS = "is-disabled";

export default function SelectObjectsDecision({
  decision,
  canAct,
  inspectorOracleTextHeight = 0,
  inlineSubmit = true,
  onSubmitActionChange = null,
  hideDescription = false,
  layout = "panel",
}) {
  const ui = useUiText();
  const { dispatch, state, playerAccentOverrides } = useGame();
  const { hoveredObjectId, hoverCard, clearHover, setHoverLinkedObjects, clearHoverLinkedObjects } = useHover();
  useEffect(() => () => clearHoverLinkedObjects(), [clearHoverLinkedObjects]);
  const stripLayout = layout === "strip";
  const candidates = useMemo(() => decision.candidates || [], [decision.candidates]);
  const { t } = useI18n();
  // Candidate rows name cards, so they use the official localized printing.
  const localizeCardName = useTranslatedCardNames(
    useMemo(() => candidates.map((candidate) => candidate?.name), [candidates])
  );
  // Choices live above this panel: every card surface renders a check for them.
  const selected = useChosenObjectIds();
  const { applyChoice, clearChoices } = useObjectSelectionActions() || {};
  const min = decision.min ?? 0;
  const max = decision.max ?? candidates.length;
  const allowPartialCompletion = decision.allow_partial_completion === true;
  const hideTimerRef = useRef(null);
  const optionsMaxHeight = useMemo(() => {
    const oracleHeight = Number(inspectorOracleTextHeight);
    if (!Number.isFinite(oracleHeight) || oracleHeight <= 0) return 360;
    const dynamicMax = Math.round(420 - (oracleHeight * 0.55));
    return Math.max(180, Math.min(360, dynamicMax));
  }, [inspectorOracleTextHeight]);
  const objectControllerById = useMemo(
    () => buildObjectControllerById(state),
    [state]
  );

  const scopedCandidates = useMemo(() => {
    if (stripLayout) return candidates;
    if (hoveredObjectId == null) return candidates;
    const hoveredStr = String(hoveredObjectId);
    const hasHoveredCandidate = candidates.some((c) => String(c.id) === hoveredStr);
    if (!hasHoveredCandidate) return candidates;
    return candidates.filter(
      (c) => String(c.id) === hoveredStr || isObjectChosen(selected, c.id)
    );
  }, [candidates, hoveredObjectId, selected, stripLayout]);
  const showRows = scopedCandidates.length > 0;
  const [visibleCandidates, setVisibleCandidates] = useState(scopedCandidates);
  const focusedToHover = hoveredObjectId != null
    && candidates.some((c) => String(c.id) === String(hoveredObjectId));
  const showHeader = !stripLayout;

  const chooseObject = useCallback((id, mode = "toggle") => {
    applyChoice?.({ objectId: id, mode, max });
  }, [applyChoice, max]);

  useEffect(() => {
    const onExternalObjectChoice = (event) => {
      if (!canAct) return;
      const externalObjectId = event?.detail?.objectId;
      if (externalObjectId == null) return;
      const matchedCandidate = candidates.find(
        (candidate) => String(candidate?.id) === String(externalObjectId)
      );
      if (!matchedCandidate || matchedCandidate.legal === false) return;
      chooseObject(matchedCandidate.id, event?.detail?.mode || "toggle");
    };

    window.addEventListener(SELECT_OBJECT_CHOICE_EVENT, onExternalObjectChoice);
    return () => {
      window.removeEventListener(SELECT_OBJECT_CHOICE_EVENT, onExternalObjectChoice);
    };
  }, [canAct, candidates, chooseObject]);

  const canSubmit = selected.length <= max
    && (allowPartialCompletion || selected.length >= min);
  const selectedIds = useMemo(() => Array.from(selected), [selected]);
  const submitRangeLabel = allowPartialCompletion
    ? `0-${max}`
    : (min === max ? min : `${min}-${max}`);
  const submitLabel = t("decision.submit", { progress: `${selected.length}/${submitRangeLabel}` });
  const handleSubmit = useCallback(() => {
    dispatch(
      { type: "select_objects", object_ids: selectedIds },
      `Selected ${selectedIds.length} object(s)`
    );
    clearChoices?.();
  }, [clearChoices, dispatch, selectedIds]);

  useEffect(() => {
    if (!onSubmitActionChange) return undefined;
    onSubmitActionChange({
      label: submitLabel,
      disabled: !canAct || !canSubmit,
      onSubmit: handleSubmit,
    });
    return () => onSubmitActionChange(null);
  }, [onSubmitActionChange, submitLabel, canAct, canSubmit, handleSubmit]);

  useEffect(() => {
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
      hideTimerRef.current = null;
    }
    hideTimerRef.current = setTimeout(() => {
      setVisibleCandidates(showRows ? scopedCandidates : []);
      hideTimerRef.current = null;
    }, showRows ? 0 : 180);
  }, [scopedCandidates, showRows]);

  useEffect(
    () => () => {
      if (hideTimerRef.current) {
        clearTimeout(hideTimerRef.current);
        hideTimerRef.current = null;
      }
    },
    []
  );

  return (
    <div className="flex w-full min-w-0 flex-col gap-1.5">
      <div
        className={cn(
          stripLayout ? "transition-all duration-200" : "-mx-1.5 transition-all duration-200",
          showRows ? "opacity-100 translate-y-0" : "opacity-0 -translate-y-1 pointer-events-none"
        )}
      >
        {showHeader && (
          <div
            className={cn(
              stripLayout
                ? "decision-strip-header px-1.5 py-1"
                : "decision-panel-header sticky top-0 z-10 px-1.5 py-1"
            )}
          >
            <DecisionSummary
              decision={decision}
              hideDescription={hideDescription}
              layout={layout}
            />
            {!stripLayout && (
              <div className="decision-helper-text text-[13px] leading-snug">
                {allowPartialCompletion
                  ? t("decision.selectUpTo", { max })
                  : t("decision.selectRange", { range: min === max ? min : `${min}-${max}` })}
              </div>
            )}
            {!stripLayout && focusedToHover && (
              <div className="decision-helper-text decision-helper-text--muted text-[12px] italic leading-snug">
                {t("decision.hoveredOptions")}
              </div>
            )}
          </div>
        )}
        <div
          className={cn(
            "w-full transition-[max-height] duration-300 ease-out",
            stripLayout ? "decision-strip-scroll overflow-x-auto overflow-y-hidden pb-1" : "overflow-y-auto overflow-x-hidden"
          )}
          style={stripLayout ? undefined : { maxHeight: `${optionsMaxHeight}px` }}
        >
          <div className={cn(
            stripLayout
              ? "decision-strip-options-row flex w-max min-w-full flex-nowrap items-center gap-1.5 py-0.5 pr-1"
              : "w-full divide-y divide-[rgba(128,107,78,0.28)]"
          )}>
            {visibleCandidates.map((c) => {
              const isSelected = isObjectChosen(selected, c.id);
              const isUnavailable = !isSelected && selected.length >= max;
              const isDisabled = !canAct || !c.legal || isUnavailable;
              const controllerId = c?.object_controller != null
                ? Number(c.object_controller)
                : objectControllerById.get(String(c.id));
              const accent = getPlayerAccent(
                state?.players || [],
                controllerId ?? state?.perspective,
                state?.perspective,
                playerAccentOverrides,
              );
              return (
                <Button
                  key={c.id}
                  aria-pressed={isSelected}
                  variant="ghost"
                  size="sm"
                  className={cn(
                    stripLayout
                      ? STRIP_ITEM_BASE_CLASS
                      : "decision-option-row decision-option-row--panel h-8 w-full min-w-0 justify-start px-2.5 text-[13px]",
                    stripLayout && isSelected && STRIP_ITEM_ACTIVE_CLASS,
                    !stripLayout && isSelected && "is-selected",
                    isDisabled
                      && (stripLayout
                        ? STRIP_ITEM_DISABLED_CLASS
                        : "is-disabled")
                  )}
                  style={decisionOptionAccentVars(accent)}
                  disabled={isDisabled}
                  onPointerDown={(event) => {
                    if (isDisabled || event.button !== 0) return;
                    event.preventDefault();
                    chooseObject(c.id);
                  }}
                  onClick={(event) => {
                    if (isDisabled || event.detail !== 0) return;
                    chooseObject(c.id);
                  }}
                  onMouseEnter={() => {
                    hoverCard(c.id);
                    setHoverLinkedObjects([c.id]);
                  }}
                  onMouseLeave={clearHover}
                  onFocus={() => {
                    hoverCard(c.id);
                    setHoverLinkedObjects([c.id]);
                  }}
                  onBlur={clearHover}
                >
                  <HighlightedDecisionText
                    className="decision-option-label"
                    text={localizeCardName(c.name)}
                    highlightText={localizeCardName(c.name)}
                  />
                </Button>
              );
            })}
            {visibleCandidates.length === 0 && (
              <div className={cn(
                "decision-empty-note text-[12px] italic",
                stripLayout ? "px-2 py-1" : "px-2.5 py-2"
              )}>
                {t("decision.noLegalChoices")}
              </div>
            )}
          </div>
        </div>
      </div>
      {inlineSubmit && (
        <div className="w-full shrink-0 pt-1">
          <Button
            variant="ghost"
            size="sm"
            className={cn(
              "decision-neon-button decision-submit-button h-6 rounded-none px-2 text-[13px] font-semibold uppercase",
              stripLayout ? "w-auto ml-1" : "w-full"
            )}
            disabled={!canAct || !canSubmit}
            onPointerDown={(event) => {
              if (!canAct || !canSubmit || event.button !== 0) return;
              event.preventDefault();
              handleSubmit();
            }}
            onClick={(event) => {
              if (!canAct || !canSubmit || event.detail !== 0) return;
              handleSubmit();
            }}
          >
            {ui(submitLabel)}
          </Button>
        </div>
      )}
    </div>
  );
}
