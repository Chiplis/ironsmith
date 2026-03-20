"use client"

import * as React from "react"
import { Popover as PopoverPrimitive } from "radix-ui"

import { cn } from "@/lib/utils"

function ComicTooltip({
  children,
  title,
  description,
  side = "top",
  align = "center",
  sideOffset = 8,
  className,
  contentClassName,
  ...props
}) {
  const [open, setOpen] = React.useState(false);

  if (!title && !description) {
    return children;
  }

  return (
    <PopoverPrimitive.Root open={open} onOpenChange={setOpen} {...props}>
      <PopoverPrimitive.Trigger asChild className={className}>
        {children}
      </PopoverPrimitive.Trigger>
      <PopoverPrimitive.Portal>
        <PopoverPrimitive.Content
          side={side}
          align={align}
          sideOffset={sideOffset}
          collisionPadding={16}
          onPointerDown={() => setOpen(false)}
          className={cn(
            "z-[360] max-w-[260px] origin-(--radix-popover-content-transform-origin) rounded-[16px] border border-[rgba(14,18,24,0.18)] bg-white px-3 py-2 text-[#111827] shadow-[0_14px_28px_rgba(0,0,0,0.28),0_0_0_1px_rgba(255,255,255,0.55)] outline-none data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95 data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-1 data-[side=left]:slide-in-from-right-1 data-[side=right]:slide-in-from-left-1 data-[side=top]:slide-in-from-bottom-1",
            contentClassName
          )}
        >
          <div className="pointer-events-none relative grid gap-1">
            {title ? (
              <div className="text-[11px] font-black uppercase tracking-[0.18em] text-[#334155]">
                {title}
              </div>
            ) : null}
            {description ? (
              <div className="text-[12px] font-semibold leading-[1.3] text-[#111827]">
                {description}
              </div>
            ) : null}
          </div>
        </PopoverPrimitive.Content>
      </PopoverPrimitive.Portal>
    </PopoverPrimitive.Root>
  );
}

export { ComicTooltip }
