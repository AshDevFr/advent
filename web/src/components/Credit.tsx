const linkClass = "underline underline-offset-2 hover:text-foreground";

/**
 * The "Built with Advent" footer on the landing page and every calendar, so
 * players who enjoyed it can find the engine and make their own.
 */
export function Credit() {
  return (
    <footer className="mt-auto pt-6 text-center text-muted-foreground text-xs">
      Built with{" "}
      <a href="https://github.com/AshDevFr/advent" className={linkClass}>
        Advent
      </a>{" "}
      {__ADVENT_VERSION__} ·{" "}
      <a
        href="https://advent-engine.4sh.dev/start/introduction/"
        className={linkClass}
      >
        Build your own
      </a>
    </footer>
  );
}
