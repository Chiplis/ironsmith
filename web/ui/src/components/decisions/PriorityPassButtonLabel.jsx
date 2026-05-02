import { cn } from "@/lib/utils";

export default function PriorityPassButtonLabel({
  currentLabel,
  advanceLabel,
  className = "",
}) {
  return (
    <span
      className={cn(
        "flex min-w-0 flex-col items-start justify-center leading-none transition-transform duration-200 group-hover:translate-x-0.5",
        className
      )}
    >
      <span className="block max-w-full truncate text-[1em] font-bold leading-[1.05] uppercase">
        {currentLabel || "Priority"}
      </span>
      {advanceLabel ? (
        <span className="mt-0.5 block max-w-full truncate text-[0.68em] font-semibold leading-none normal-case opacity-75">
          {advanceLabel}
        </span>
      ) : null}
    </span>
  );
}
