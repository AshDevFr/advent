import { cn } from "@/lib/utils";

export interface Door {
  day: number;
  state: "locked" | "open" | "solved";
  /** The day's type label, once released. */
  type?: string;
  /** The collected word, once solved. */
  word?: string;
  /** When a locked door opens, e.g. "Dec 7". */
  opens?: string;
}

const label =
  "text-[11px] font-semibold tracking-wider uppercase leading-tight";

/** The calendar: one card per day. Locked doors are not interactive. */
export function DoorGrid({
  doors,
  onOpen,
}: {
  doors: Door[];
  onOpen: (day: number) => void;
}) {
  return (
    <ul
      aria-label="Calendar doors"
      className="grid grid-cols-3 gap-2 sm:gap-3 md:grid-cols-4 lg:grid-cols-6"
    >
      {doors.map((door) => (
        <li key={door.day}>
          {door.state === "locked" ? (
            <div
              role="img"
              aria-label={`Day ${door.day}, locked${door.opens ? `, opens ${door.opens}` : ""}`}
              className="flex aspect-square flex-col justify-between rounded-lg border bg-card/60 p-2.5 sm:aspect-[5/6] sm:p-3"
            >
              <span className="font-display text-3xl font-semibold text-muted-foreground sm:text-4xl">
                {door.day}
              </span>
              {door.opens && (
                <span className={cn(label, "text-muted-foreground")}>
                  Opens {door.opens}
                </span>
              )}
            </div>
          ) : (
            <button
              type="button"
              aria-label={`Day ${door.day}, ${door.state}`}
              onClick={() => onOpen(door.day)}
              className={cn(
                "flex aspect-square w-full flex-col justify-between rounded-lg border p-2.5 text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring sm:aspect-[5/6] sm:p-3",
                door.state === "solved"
                  ? "border-primary bg-primary/10"
                  : "bg-card hover:border-primary",
              )}
            >
              <span className="font-display text-3xl font-semibold sm:text-4xl">
                {door.day}
              </span>
              <span className="flex flex-col gap-0.5">
                {door.word && (
                  <span className="truncate font-mono text-xs font-semibold text-primary sm:text-sm">
                    {door.word}
                  </span>
                )}
                {door.type && (
                  <span
                    className={cn(
                      label,
                      door.state === "solved"
                        ? "text-primary"
                        : "text-muted-foreground",
                    )}
                  >
                    {door.type}
                  </span>
                )}
              </span>
            </button>
          )}
        </li>
      ))}
    </ul>
  );
}
