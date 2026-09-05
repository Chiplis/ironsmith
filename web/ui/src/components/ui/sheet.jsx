import * as React from "react"
import { XIcon } from "lucide-react"
import { Dialog as SheetPrimitive } from "radix-ui"

import { cn } from "@/lib/utils"

function Sheet({
  ...props
}) {
  return <SheetPrimitive.Root data-slot="sheet" {...props} />;
}

function SheetTrigger({
  ...props
}) {
  return <SheetPrimitive.Trigger data-slot="sheet-trigger" {...props} />;
}

function SheetClose({
  ...props
}) {
  return <SheetPrimitive.Close data-slot="sheet-close" {...props} />;
}

function SheetPortal({
  ...props
}) {
  return <SheetPrimitive.Portal data-slot="sheet-portal" {...props} />;
}

function SheetOverlay({
  className,
  style,
  ...props
}) {
  return (
    <SheetPrimitive.Overlay
      data-slot="sheet-overlay"
      className={cn(
        "fixed inset-0 bg-black/50",
        className
      )}
      style={{ zIndex: 1000, ...style }}
      {...props} />
  );
}

function SheetContent({
  className,
  children,
  side = "right",
  showCloseButton = true,
  onOpenAutoFocus,
  onCloseAutoFocus,
  style,
  ...props
}) {
  const returnFocusRef = React.useRef(null);
  const centeredStyle = side === "center"
    ? {
      left: "50%",
      top: "50%",
      maxHeight: "calc(100dvh - 32px)",
      transform: "translate3d(-50%, -50%, 0)",
    }
    : null;

  return (
    <SheetPortal>
      <SheetOverlay />
      <SheetPrimitive.Content
        data-slot="sheet-content"
        data-side={side}
        onOpenAutoFocus={(event) => {
          returnFocusRef.current = document.activeElement;
          onOpenAutoFocus?.(event);
        }}
        onCloseAutoFocus={(event) => {
          onCloseAutoFocus?.(event);
          const previous = returnFocusRef.current;
          if (!event.defaultPrevented && previous?.isConnected && previous !== document.body) {
            event.preventDefault();
            previous.focus();
          }
        }}
        className={cn(
          "fixed flex min-h-0 flex-col gap-4 overflow-y-auto overscroll-contain bg-background shadow-lg",
          side === "right" &&
            "inset-y-0 right-0 h-full w-3/4 border-l sm:max-w-sm",
          side === "left" &&
            "inset-y-0 left-0 h-full w-3/4 border-r sm:max-w-sm",
          side === "top" &&
            "inset-x-0 top-0 h-auto border-b",
          side === "bottom" &&
            "inset-x-0 bottom-0 h-auto border-t",
          side === "center" &&
            "w-[calc(100vw-2rem)] max-w-[1120px] border",
          className
        )}
        style={centeredStyle ? { zIndex: 1001, ...centeredStyle, ...style } : { zIndex: 1001, ...style }}
        {...props}>
        {children}
        {showCloseButton && (
          <SheetPrimitive.Close
            data-slot="sheet-close-button"
            className="absolute top-3 right-3 z-10 inline-flex size-9 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none">
            <XIcon className="size-4" />
            <span className="sr-only">Close</span>
          </SheetPrimitive.Close>
        )}
      </SheetPrimitive.Content>
    </SheetPortal>
  );
}

function SheetHeader({
  className,
  ...props
}) {
  return (
    <div
      data-slot="sheet-header"
      className={cn("flex shrink-0 flex-col gap-1.5 p-4 pr-14", className)}
      {...props} />
  );
}

function SheetFooter({
  className,
  ...props
}) {
  return (
    <div
      data-slot="sheet-footer"
      className={cn("mt-auto flex flex-col gap-2 p-4", className)}
      {...props} />
  );
}

function SheetTitle({
  className,
  ...props
}) {
  return (
    <SheetPrimitive.Title
      data-slot="sheet-title"
      className={cn("font-semibold text-foreground", className)}
      {...props} />
  );
}

function SheetDescription({
  className,
  ...props
}) {
  return (
    <SheetPrimitive.Description
      data-slot="sheet-description"
      className={cn("text-sm text-muted-foreground", className)}
      {...props} />
  );
}

export {
  Sheet,
  SheetTrigger,
  SheetClose,
  SheetContent,
  SheetHeader,
  SheetFooter,
  SheetTitle,
  SheetDescription,
}
