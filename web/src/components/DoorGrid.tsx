import type { CSSProperties } from "react";

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

/**
 * Doors per row on wide screens: the theme's choice, or else the widest row
 * of 4 to 6 that leaves no short last row, falling back to the one whose last
 * row is fullest. 15 doors get 5 rows of 3 rather than 6, 6 and 3.
 */
export function doorColumns(count: number, columns?: number | null): number {
  if (columns) return columns;
  const fill = (width: number) =>
    count % width === 0 ? 1 : (count % width) / width;
  return [6, 5, 4].reduce((best, width) =>
    fill(width) > fill(best) ? width : best,
  );
}

const track = (width: number) => `repeat(${width}, minmax(0, 1fr))`;

/** The calendar: one card per day. Locked doors are not interactive. */
export function DoorGrid({
  doors,
  onOpen,
  artwork,
  columns,
}: {
  doors: Door[];
  onOpen: (day: number) => void;
  /** The theme's door artwork, shown on locked doors like wrapping paper;
   * the doors take the images in turn. */
  artwork?: string[];
  /** Doors per row on wide screens, from the theme. */
  columns?: number | null;
}) {
  // Narrower screens show fewer doors per row, never more than wide ones.
  const wide = doorColumns(doors.length, columns);
  const widths = {
    "--cols": track(Math.min(3, wide)),
    "--cols-md": track(Math.min(4, wide)),
    "--cols-lg": track(wide),
  } as CSSProperties;
  // A wash of the door colour keeps the number and date readable on top.
  // Open doors keep their picture, faded further so the door reads as opened.
  const wrapped = (day: number, wash: number) => {
    if (!artwork?.length) return undefined;
    const image = artwork[(day - 1) % artwork.length];
    const tint = `color-mix(in srgb, var(--surface-raised) ${wash}%, transparent)`;
    return {
      backgroundImage: `linear-gradient(${tint}, ${tint}), url("${image}")`,
      backgroundSize: "cover",
      backgroundPosition: "center",
    };
  };
  // On artwork, a halo in the door colour lifts the text off the picture.
  const onArt = artwork?.length
    ? "text-foreground [text-shadow:0_0_4px_var(--surface-raised),0_0_10px_var(--surface-raised),0_1px_2px_var(--surface-raised)]"
    : "text-muted-foreground";
  return (
    <ul
      aria-label="Calendar doors"
      data-columns={wide}
      style={widths}
      className="grid grid-cols-(--cols) gap-2 sm:gap-3 md:grid-cols-(--cols-md) lg:grid-cols-(--cols-lg)"
    >
      {doors.map((door) => (
        <li key={door.day}>
          {door.state === "locked" ? (
            <div
              role="img"
              aria-label={`Day ${door.day}, locked${door.opens ? `, opens ${door.opens}` : ""}`}
              data-artwork={artwork?.length ? "" : undefined}
              style={wrapped(door.day, 55)}
              className="flex aspect-square flex-col justify-between rounded-lg border bg-card/60 p-2.5 sm:aspect-[5/6] sm:p-3"
            >
              <span
                className={cn(
                  "font-display text-3xl font-semibold sm:text-4xl",
                  onArt,
                )}
              >
                {door.day}
              </span>
              {door.opens && (
                <span className={cn(label, onArt)}>Opens {door.opens}</span>
              )}
            </div>
          ) : (
            <button
              type="button"
              aria-label={`Day ${door.day}, ${door.state}`}
              data-day={door.day}
              onClick={() => onOpen(door.day)}
              style={wrapped(door.day, 82)}
              className={cn(
                "flex aspect-square w-full flex-col justify-between rounded-lg border p-2.5 text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring sm:aspect-[5/6] sm:p-3",
                door.state === "solved"
                  ? "border-primary bg-primary/10"
                  : "bg-card hover:border-primary",
                artwork?.length && "border-2",
              )}
            >
              <span
                className={cn(
                  "font-display text-3xl font-semibold sm:text-4xl",
                  artwork?.length && onArt,
                )}
              >
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
