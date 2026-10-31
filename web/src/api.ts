/**
 * The only module that knows the site's paths. Everything here is a static
 * file written by `advent build`; unreleased days simply do not exist.
 * Paths are relative: the browser resolves them against the page's
 * `<base href>`, so a calendar works at a domain root or under a sub-path.
 */
import type { SealedFinal, WordLock } from "./crypto";

export interface SiteIndex {
  brand: {
    name: string;
    logo: string | null;
    /** The logo for dark pages, when the brand has one. */
    logo_dark?: string | null;
  };
  events: EventInfo[];
}

export interface EventInfo {
  id: string;
  title: string;
  timezone: string;
  start: string;
  days: number;
  /** The edition's display headline, if the story sets one. */
  headline: string | null;
  intro: string | null;
  /** Opening date of every day (YYYY-MM-DD), released or not. */
  dates: string[];
  /** When day 1 opens (RFC 3339), or null when it is already open. */
  opens_at: string | null;
  released: number[];
  /** Announcements published so far; `id` changes when the text does. */
  notices: { id: string; text: string }[];
  /** Type label of each released day, keyed by day number. */
  types: Record<string, string>;
  background: string | null;
  /** The background for dark pages, when the theme has one. */
  background_dark?: string | null;
  /** Door artwork, taken in turn: door N uses entry (N - 1) % length. */
  doors: string[];
}

export interface DayPage {
  day: number;
  title: string;
  type: string;
  markdown: string;
}

export class NotFound extends Error {}

async function fetchJson<T>(path: string): Promise<T> {
  const response = await fetch(path, { cache: "no-cache" });
  if (response.status === 404) throw new NotFound(path);
  if (!response.ok) throw new Error(`${path}: HTTP ${response.status}`);
  return (await response.json()) as T;
}

export const api = {
  site: () => fetchJson<SiteIndex>("site.json"),
  event: (id: string) => fetchJson<EventInfo>(`${id}/event.json`),
  day: (id: string, day: number) =>
    fetchJson<DayPage>(`${id}/api/day/${day}.json`),
  hints: (id: string, day: number) =>
    fetchJson<{ hints: string[] }>(`${id}/api/day/${day}/hint.json`),
  locks: (id: string, day: number) =>
    fetchJson<{ word_locks: WordLock[] }>(`${id}/api/day/${day}/locks.json`),
  final: (id: string) => fetchJson<SealedFinal>(`${id}/api/final.json`),
};

/** URL of an image referenced from a day's markdown. */
export function assetUrl(id: string, day: number, name: string): string {
  return `${id}/api/day/${day}/${name}`;
}

/** A sealed reward file, decrypted in the browser with its key. */
export function rewardUrl(id: string, index: number): string {
  return `${id}/api/rewards/${index}.bin`;
}

export function themeCssUrl(id?: string): string {
  return id ? `${id}/theme.css` : "theme.css";
}

/** The site's base path from `<base href>`, e.g. "/" or "/demo/". */
export function basePath(): string {
  return new URL(document.baseURI).pathname;
}

/** The event id a location path points at, relative to the base path. */
export function routeEventId(
  pathname: string,
  base = basePath(),
): string | null {
  const relative = pathname.startsWith(base)
    ? pathname.slice(base.length)
    : pathname;
  const id = relative.split("/").filter(Boolean)[0];
  return id ? decodeURIComponent(id) : null;
}
