/**
 * Player progress, kept in localStorage per event. Nothing here is secret
 * from the player: a faked word cannot open the certificate.
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
    (set) => ({
      events: {},
      solve: (event, day, word) =>
        set((state) => {
          const current = state.events[event] ?? emptyProgress();
          return {
            events: {
              ...state.events,
              [event]: { ...current, words: { ...current.words, [day]: word } },
            },
          };
        }),
    }),
    { name: "advent-progress", version: 1, storage: safeStorage },
  ),
);

export function useEventProgress(event: string): EventProgress {
  return useProgress((state) => state.events[event]) ?? EMPTY;
}

const EMPTY = emptyProgress();
