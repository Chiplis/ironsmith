import { useState, useCallback, useRef } from "react";

function nowLabel() {
  const dt = new Date();
  return dt.toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

export function useActivityLog() {
  const [entries, setEntries] = useState([]);
  const entriesRef = useRef([]);

  const pushLog = useCallback((message, isError = false) => {
    const entry = { time: nowLabel(), message, isError };
    entriesRef.current = [entry, ...entriesRef.current].slice(0, 120);
    setEntries(entriesRef.current);
  }, []);

  return { entries, pushLog };
}
