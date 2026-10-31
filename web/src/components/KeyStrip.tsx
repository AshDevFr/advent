import { cn } from "@/lib/utils";

/** "Your key": one slot per day, filled with the words collected so far. */
export function KeyStrip({
  days,
  words,
}: {
  days: number;
  words: Record<number, string>;
}) {
  const found = Object.keys(words).length;
  return (
    <section aria-labelledby="key-heading" className="flex flex-col gap-2">
      <div className="flex items-baseline justify-between">
        <h2
          id="key-heading"
          className="text-xs font-semibold tracking-widest text-muted-foreground uppercase"
        >
          Your key
        </h2>
        <p className="font-mono text-sm font-semibold">
          {found} / {days} words
        </p>
      </div>
      <ol
        aria-label="Key words collected"
        className="grid grid-cols-6 gap-1 sm:grid-cols-8 sm:gap-1.5 lg:grid-cols-12"
      >
        {Array.from({ length: days }, (_, i) => i + 1).map((day) => (
          <li
            key={day}
            aria-label={
              words[day] ? `Day ${day}: ${words[day]}` : `Day ${day}: missing`
            }
            className={cn(
              "flex h-8 items-center justify-center overflow-hidden rounded-md border px-1 font-mono text-[11px] sm:text-xs",
              words[day]
                ? "border-primary bg-primary/10 font-semibold"
                : "text-muted-foreground",
            )}
          >
            {words[day] ?? day}
          </li>
        ))}
      </ol>
    </section>
  );
}
