import { useState } from "react";
import { cn } from "@/lib/utils";

// Retain the last visible content while the panel rolls closed.
export default function RollingPanel({ open, children, className, ...props }) {
  const [visibleContent, setVisibleContent] = useState(children);
  if (open && children !== visibleContent) setVisibleContent(children);
  return (
    <div {...props} className={cn("battlefield-rolling-panel", className)} data-open={open ? "true" : "false"} inert={!open} aria-hidden={!open}>
      <div className="battlefield-rolling-panel-clip">
        {open ? children : visibleContent}
      </div>
    </div>
  );
}
