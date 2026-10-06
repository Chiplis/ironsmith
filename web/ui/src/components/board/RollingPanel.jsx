import { useState } from "react";
import { cn } from "@/lib/utils";

// Retain the last visible content while the panel rolls closed.
export default function RollingPanel({ open, children, className, retainContent = true, ...props }) {
  const [visibleContent, setVisibleContent] = useState(children);
  if (retainContent && open && children !== visibleContent) setVisibleContent(children);
  return (
    <div {...props} className={cn("battlefield-rolling-panel", className)} data-open={open ? "true" : "false"} inert={!open} aria-hidden={!open}>
      <div className="battlefield-rolling-panel-clip">
        {open || !retainContent ? children : visibleContent}
      </div>
    </div>
  );
}
