import type { EventInfo, SiteIndex } from "@/api";

export const event = (over: Partial<EventInfo> = {}): EventInfo => ({
  id: "2026",
  title: "Advent 2026",
  timezone: "UTC",
  start: "2026-12-01",
  days: 7,
  intro: "Seven doors, one key.",
  opens_at: "2026-12-01T00:00:00Z",
  released: [1, 2, 3],
  background: null,
  doors: null,
  ...over,
});

export const site = (events: EventInfo[]): SiteIndex => ({
  brand: { name: "Acme Advent", logo: "/brand/logo.svg" },
  events,
});

/** Serve `files` (path to JSON) through a mocked fetch. */
export function serve(files: Record<string, unknown>) {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (path: string) =>
      path in files
        ? new Response(JSON.stringify(files[path]), { status: 200 })
        : new Response("not found", { status: 404 }),
    ),
  );
}
