import { useEffect, useRef, useState } from "react";

function buildLocalEntryIds(entries = [], perspective = null) {
  return entries
    .filter((entry) => Number(entry?.controller) === Number(perspective))
    .map((entry) => String(entry.id));
}

export default function useStackStartAlert(entries = [], perspective = null) {
  const previousLocalIdsRef = useRef([]);
  const [alertEntryId, setAlertEntryId] = useState(null);

  useEffect(() => {
    const previousLocalIds = previousLocalIdsRef.current || [];
    const currentLocalIds = buildLocalEntryIds(entries, perspective);
    const hadLocalStackEntries = previousLocalIds.length > 0;
    const hasLocalStackEntries = currentLocalIds.length > 0;

    if (!hadLocalStackEntries && hasLocalStackEntries) {
      const previousIdSet = new Set(previousLocalIds);
      const firstNewLocalId = currentLocalIds.find((id) => !previousIdSet.has(id)) || currentLocalIds[0];
      setAlertEntryId(firstNewLocalId || null);
    } else if (!hasLocalStackEntries) {
      setAlertEntryId(null);
    } else if (alertEntryId != null && !currentLocalIds.includes(String(alertEntryId))) {
      setAlertEntryId(null);
    }

    previousLocalIdsRef.current = currentLocalIds;
  }, [alertEntryId, entries, perspective]);

  return {
    alertEntryId,
    dismissAlert: () => setAlertEntryId(null),
  };
}
