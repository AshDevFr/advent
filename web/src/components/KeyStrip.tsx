/** One slot per day, filled with the words collected so far. */
export function KeyStrip({
  days,
  words,
}: {
  days: number;
  words: Record<number, string>;
}) {
  return (
    <ol
      aria-label="Key words collected"
      className="flex flex-wrap justify-center gap-1.5"
    >
      {Array.from({ length: days }, (_, i) => i + 1).map((day) => (
        <li
          key={day}
          aria-label={
            words[day] ? `Day ${day}: ${words[day]}` : `Day ${day}: missing`
          }
          className={
            words[day]
              ? "rounded-md bg-accent px-2 py-1 text-sm font-medium text-accent-foreground"
              : "rounded-md border border-dashed px-2 py-1 text-sm text-muted-foreground"
          }
        >
          {words[day] ?? day}
        </li>
      ))}
    </ol>
  );
}
