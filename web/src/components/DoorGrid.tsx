import { cn } from "@/lib/utils";

export type DoorState = "locked" | "open" | "solved";

/** The calendar: one door per day. Locked doors are not interactive. */
export function DoorGrid({
  days,
  stateOf,
  onOpen,
}: {
  days: number;
  stateOf: (day: number) => DoorState;
  onOpen: (day: number) => void;
}) {
  return (
    <ul
      aria-label="Calendar doors"
      className="grid grid-cols-4 gap-2 sm:grid-cols-6"
    >
      {Array.from({ length: days }, (_, i) => i + 1).map((day) => {
        const state = stateOf(day);
        const base =
          "flex aspect-square w-full flex-col items-center justify-center rounded-lg border text-lg font-semibold";
        return (
          <li key={day}>
            {state === "locked" ? (
              <div
                role="img"
                aria-label={`Day ${day}, locked`}
                className={cn(
                  base,
                  "bg-muted text-muted-foreground opacity-60",
                )}
              >
                {day}
              </div>
            ) : (
              <button
                type="button"
                aria-label={`Day ${day}, ${state}`}
                onClick={() => onOpen(day)}
                className={cn(
                  base,
                  "transition-transform hover:scale-105 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                  state === "solved"
                    ? "bg-accent text-accent-foreground"
                    : "bg-primary text-primary-foreground",
                )}
              >
                {day}
                {state === "solved" && (
                  <span aria-hidden="true" className="text-xs">
                    ✓
                  </span>
                )}
              </button>
            )}
          </li>
        );
      })}
    </ul>
  );
}
