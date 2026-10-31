/**
 * What the calendar does with a player's answers and progress, behind one
 * interface. The static client checks answers against the word locks in the
 * browser and keeps progress in localStorage; a server-hosted calendar
 * checks answers and keeps progress on the server. Pages only use this.
 */
import { serverClient } from "./server";
import { staticClient } from "./static";

export { useEventProgress } from "@/store";

export type GuessResult =
  | { kind: "solved"; word: string }
  /** `remaining`: guesses left in this window under an attempt limit. */
  | { kind: "wrong"; remaining: number | null }
  /** Too many guesses; the next one is accepted from `retryAt`. */
  | { kind: "limited"; retryAt: string };

export interface Client {
  readonly mode: "static" | "server";
  /** Check an answer; a right one stores the day's word in the progress. */
  guess(event: string, day: number, answer: string): Promise<GuessResult>;
  /** Show the next of the `available` released hints. */
  revealHint(event: string, day: number, available: number): void;
  /** The name for the certificate. */
  setName(event: string, name: string): Promise<void>;
  /**
   * Moving progress to another browser with a code. Only where progress
   * lives in the browser; a server already has it everywhere.
   */
  /** Bring the event's progress from where it is kept (server only). */
  load?(event: string): Promise<void>;
  signOut?(): Promise<void>;
  transfer?: {
    exportCode(event: string): string;
    /** Replace the event's progress; false if the code is not valid. */
    importCode(event: string, code: string): boolean;
  };
}

/**
 * How this copy of the calendar is hosted: `advent serve` marks its pages
 * with `<meta name="advent-mode" content="server">`, read once at start so
 * the choice never waits on a request.
 */
export function hostingMode(doc: Document = document): Client["mode"] {
  return doc
    .querySelector('meta[name="advent-mode"]')
    ?.getAttribute("content") === "server"
    ? "server"
    : "static";
}

/** A failure to show the player as it is. */
export class ClientError extends Error {}

let chosen: Client | null = null;

export function client(): Client {
  chosen ??= hostingMode() === "server" ? serverClient : staticClient;
  return chosen;
}

/** Use another client (tests), or `null` to choose again. */
export function setClient(next: Client | null) {
  chosen = next;
}
