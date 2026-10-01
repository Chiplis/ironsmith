/* eslint-disable react-refresh/only-export-components -- Browser fixture. */
import React, { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import PreservingTextarea from "../src/components/ui/PreservingTextarea";

function Fixture() {
  const [value, setValue] = useState("4 Lightning Bolt\n2 Counterspell\n20 Island");
  const [, setEditCount] = useState(0);
  const pendingValueRef = useRef(null);
  useEffect(() => {
    window.__applyPendingDeckEdit = () => setValue(pendingValueRef.current || value);
    return () => { delete window.__applyPendingDeckEdit; };
  }, [value]);
  return (
    <div>
      <PreservingTextarea
        aria-label="Decklist"
        value={value}
        onChange={(event) => {
          pendingValueRef.current = event.currentTarget.value;
          setEditCount((count) => count + 1);
        }}
      />
      <button type="button" onClick={() => window.__applyPendingDeckEdit()}>
        Apply pending edit
      </button>
    </div>
  );
}

createRoot(document.getElementById("root")).render(<Fixture />);
