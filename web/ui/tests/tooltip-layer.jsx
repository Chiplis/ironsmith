import React from "react";
import { createRoot } from "react-dom/client";
import { ComicTooltip } from "../src/components/ui/comic-tooltip";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "../src/components/ui/tooltip";
import "../src/index.css";

createRoot(document.getElementById("root")).render(
  <TooltipProvider>
    <div style={{position:"fixed",inset:60,zIndex:30010,background:"#333",padding:100}}>
      <ComicTooltip title="Flying" description="Rules tooltip above the inspector." open>
        <button>Flying</button>
      </ComicTooltip>
      <Tooltip open><TooltipTrigger style={{marginLeft:300}}>Standard tooltip</TooltipTrigger>
        <TooltipContent>Tooltip above the inspector</TooltipContent>
      </Tooltip>
    </div>
  </TooltipProvider>
);
