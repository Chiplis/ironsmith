/* eslint-disable react-refresh/only-export-components */
import { createContext, useCallback, useContext, useMemo, useState } from "react";
import { GameContext } from "@/context/GameContext.shared";
import { decisionKey } from "@/lib/decision-key";
import { isObjectChosen, selectionAfterChoice } from "@/lib/object-selection";

const EMPTY_SELECTION = Object.freeze([]);

// Card surfaces render a check for every chosen object, so the choices for a
// select_objects decision cannot live inside the decision panel that collects
// them. They are keyed by decision identity: a new decision, or none at all,
// leaves every card unchecked without anyone having to clear the old set.
const ChosenObjectsContext = createContext(EMPTY_SELECTION);
const ObjectSelectionActionsContext = createContext(null);

export function ObjectSelectionProvider({ children }) {
  // The shared context keeps isolated UI fixtures out of the live game module.
  const { state } = useContext(GameContext) || {};
  const decision = state?.decision;
  const activeKey = decision?.kind === "select_objects" ? decisionKey(decision) : null;
  const [selection, setSelection] = useState(() => ({ key: null, ids: EMPTY_SELECTION }));

  const chosenIds = activeKey != null && selection.key === activeKey
    ? selection.ids
    : EMPTY_SELECTION;

  const applyChoice = useCallback((choice) => {
    setSelection((current) => {
      const base = current.key === activeKey ? current.ids : EMPTY_SELECTION;
      const ids = selectionAfterChoice(base, choice);
      if (ids === base && current.key === activeKey) return current;
      return { key: activeKey, ids };
    });
  }, [activeKey]);

  const clearChoices = useCallback(() => {
    setSelection((current) => (
      current.key === activeKey && current.ids.length === 0
        ? current
        : { key: activeKey, ids: EMPTY_SELECTION }
    ));
  }, [activeKey]);

  const actions = useMemo(() => ({ applyChoice, clearChoices }), [applyChoice, clearChoices]);

  return (
    <ChosenObjectsContext.Provider value={chosenIds}>
      <ObjectSelectionActionsContext.Provider value={actions}>
        {children}
      </ObjectSelectionActionsContext.Provider>
    </ChosenObjectsContext.Provider>
  );
}

export function useChosenObjectIds() {
  return useContext(ChosenObjectsContext);
}

export function useObjectSelectionActions() {
  return useContext(ObjectSelectionActionsContext);
}

/** Grouped permanents answer for their members too, and report which id was chosen. */
export function useChosenObjectIdAmong(objectIds) {
  const chosenIds = useChosenObjectIds();
  if (!chosenIds || chosenIds.length === 0) return null;
  const ids = Array.isArray(objectIds) ? objectIds : [objectIds];
  return ids.find((id) => isObjectChosen(chosenIds, id)) ?? null;
}
