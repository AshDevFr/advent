import type { EventInfo, SiteIndex } from "@/api";

export const event = (over: Partial<EventInfo> = {}): EventInfo => ({
  id: "2026",
  title: "Advent 2026",
  timezone: "UTC",
  start: "2026-12-01",
  days: 7,
  headline: null,
  intro: "Seven doors, one key.",
  dates: Array.from({ length: 7 }, (_, i) => `2026-12-0${i + 1}`),
  opens_at: "2026-12-01T00:00:00Z",
  released: [1, 2, 3],
  notices: [],
  types: { 1: "teaser", 2: "code", 3: "company" },
  background: null,
  doors: [],
  ...over,
});

export const site = (events: EventInfo[]): SiteIndex => ({
  brand: { name: "Acme Advent", logo: "/brand/logo.svg" },
  events,
});

/**
 * Serve `files` (site path to JSON) through a mocked fetch. Keys are written
 * as site paths ("/2026/event.json"); the app requests them relative to the
 * base ("2026/event.json"), so both forms match.
 */
export function serve(files: Record<string, unknown>) {
  const byPath = new Map(
    Object.entries(files).map(([k, v]) => [k.replace(/^\//, ""), v]),
  );
  vi.stubGlobal(
    "fetch",
    vi.fn(async (path: string) => {
      const key = path.replace(/^\//, "");
      if (!byPath.has(key)) return new Response("not found", { status: 404 });
      const body = byPath.get(key);
      // Binary files (sealed rewards) are served as they are.
      return body instanceof Uint8Array
        ? new Response(body.slice().buffer, { status: 200 })
        : new Response(JSON.stringify(body), { status: 200 });
    }),
  );
}
