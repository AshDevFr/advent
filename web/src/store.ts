/**
 * Player progress, kept in localStorage per event. Nothing here is secret
 * from the player: a faked word cannot open the certificate, and attempts
 * are a statistic for the certificate, not a limit.
 */
import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

export interface EventProgress {
  /** Collected words by day number. */
  words: Record<number, string>;
  hintsUsed: Record<number, number>;
  attempts: Record<number, number>;
  name: string;
}

interface ProgressState {
  events: Record<string, EventProgress>;
  solve: (event: string, day: number, word: string) => void;
  attempt: (event: string, day: number) => void;
  revealHint: (event: string, day: number, available: number) => void;
  setName: (event: string, name: string) => void;
  replace: (event: string, progress: EventProgress) => void;
}

export const emptyProgress = (): EventProgress => ({
  words: {},
  hintsUsed: {},
  attempts: {},
  name: "",
});

/** localStorage, tolerating browsers where it is missing or throws. */
const safeStorage = createJSONStorage(() => ({
  getItem: (key) => {
    try {
      return localStorage.getItem(key);
    } catch {
      return null;
    }
  },
  setItem: (key, value) => {
    try {
      localStorage.setItem(key, value);
    } catch {
      // Private mode or quota: progress lasts for this session only.
    }
  },
  removeItem: (key) => {
    try {
      localStorage.removeItem(key);
    } catch {
      // Same as above.
    }
  },
}));

export const useProgress = create<ProgressState>()(
  persist(
    (set) => {
      const update = (
        event: string,
        change: (p: EventProgress) => Partial<EventProgress>,
      ) =>
        set((state) => {
          const current = state.events[event] ?? emptyProgress();
          return {
            events: {
              ...state.events,
              [event]: { ...current, ...change(current) },
            },
          };
        });
      return {
        events: {},
        solve: (event, day, word) =>
          update(event, (p) => ({ words: { ...p.words, [day]: word } })),
        attempt: (event, day) =>
          update(event, (p) => ({
            attempts: { ...p.attempts, [day]: (p.attempts[day] ?? 0) + 1 },
          })),
        revealHint: (event, day, available) =>
          update(event, (p) => ({
            hintsUsed: {
              ...p.hintsUsed,
              [day]: Math.min(available, (p.hintsUsed[day] ?? 0) + 1),
            },
          })),
        setName: (event, name) => update(event, () => ({ name })),
        replace: (event, progress) => update(event, () => progress),
      };
    },
    { name: "advent-progress", version: 1, storage: safeStorage },
  ),
);

const EMPTY = emptyProgress();

export function useEventProgress(event: string): EventProgress {
  return useProgress((state) => state.events[event]) ?? EMPTY;
}

/** Sum of a per-day counter, for the certificate. */
export const total = (counts: Record<number, number>) =>
  Object.values(counts).reduce((a, b) => a + b, 0);

const CODE_VERSION = 1;

/** A progress code: base64 JSON, versioned so it survives engine upgrades. */
export function exportCode(event: string, progress: EventProgress): string {
  const json = JSON.stringify({ v: CODE_VERSION, event, ...progress });
  return btoa(String.fromCharCode(...new TextEncoder().encode(json)));
}

/** Parse a progress code for `event`; null if it is not a valid one. */
export function importCode(event: string, code: string): EventProgress | null {
  try {
    const bytes = Uint8Array.from(atob(code.trim()), (c) => c.charCodeAt(0));
    const data = JSON.parse(new TextDecoder().decode(bytes));
    if (data?.v !== CODE_VERSION || data.event !== event) return null;
    const record = (value: unknown) =>
      value && typeof value === "object" && !Array.isArray(value)
        ? (value as Record<number, never>)
        : null;
    const words = record(data.words);
    const hintsUsed = record(data.hintsUsed);
    const attempts = record(data.attempts);
    if (!words || !hintsUsed || !attempts) return null;
    return {
      words,
      hintsUsed,
      attempts,
      name: typeof data.name === "string" ? data.name : "",
    };
  } catch {
    return null;
  }
}
