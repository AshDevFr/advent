/**
 * The static site's client: answers are checked against the word locks in
 * the browser, and progress stays in localStorage. Nothing here is secret
 * from the player: a faked word cannot open the certificate.
 */
import { api } from "@/api";
import { openWord } from "@/crypto";
import { emptyProgress, exportCode, importCode, useProgress } from "@/store";
import type { Client } from ".";

export const staticClient: Client = {
  mode: "static",

  async guess(event, day, answer) {
    const { word_locks } = await api.locks(event, day);
    let found: string | null = null;
    for (const lock of word_locks) {
      found = await openWord(answer, lock);
      if (found) break;
    }
    const progress = useProgress.getState();
    progress.attempt(event, day);
    if (!found) return { kind: "wrong", remaining: null };
    progress.solve(event, day, found);
    return { kind: "solved", word: found };
  },

  revealHint(event, day, available) {
    useProgress.getState().revealHint(event, day, available);
  },

  async setName(event, name) {
    useProgress.getState().setName(event, name);
  },

  transfer: {
    exportCode(event) {
      const progress = useProgress.getState().events[event];
      return exportCode(event, progress ?? emptyProgress());
    },
    importCode(event, code) {
      const imported = importCode(event, code);
      if (!imported) return false;
      useProgress.getState().replace(event, imported);
      return true;
    },
  },
};
