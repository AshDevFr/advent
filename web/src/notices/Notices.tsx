import { useState } from "react";

const key = (event: string) => `advent-dismissed-notices:${event}`;

function readDismissed(event: string): string[] {
  try {
    const value = JSON.parse(localStorage.getItem(key(event)) ?? "[]");
    return Array.isArray(value) ? value : [];
  } catch {
    return [];
  }
}

function saveDismissed(event: string, ids: string[]) {
  try {
    localStorage.setItem(key(event), JSON.stringify(ids));
  } catch {
    // Not persisted; the notice stays hidden for this visit only.
  }
}

/**
 * The organisers' announcements. Dismissing one hides it in this browser;
 * ids come from the text, so an edited notice shows up again.
 */
export function Notices({
  event,
  notices,
}: {
  event: string;
  notices: { id: string; text: string }[];
}) {
  const [dismissed, setDismissed] = useState(() => readDismissed(event));
  const visible = notices.filter((n) => !dismissed.includes(n.id));
  if (visible.length === 0) return null;

  function dismiss(id: string) {
    // Only ids still published are kept, so storage does not grow forever.
    const next = [...dismissed, id].filter((d) =>
      notices.some((n) => n.id === d),
    );
    setDismissed(next);
    saveDismissed(event, next);
  }

  return (
    <section aria-label="Announcements" className="flex flex-col gap-2">
      {visible.map((notice) => (
        <div
          key={notice.id}
          className="flex max-w-2xl items-start gap-3 rounded-lg border border-dashed px-4 py-3 text-sm text-muted-foreground"
        >
          <p className="flex-1 whitespace-pre-line">{notice.text}</p>
          <button
            type="button"
            aria-label="Dismiss this notice"
            onClick={() => dismiss(notice.id)}
            className="-mr-1 rounded px-1.5 text-lg leading-none hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          >
            ×
          </button>
        </div>
      ))}
    </section>
  );
}
