/**
 * The client for a calendar hosted by `advent serve`: answers are checked
 * and progress is kept on the server, so the browser never holds word locks
 * and an attempt limit cannot be bypassed. Progress is mirrored into the
 * same store the pages read, after each answer and when an event loads.
 */
import { useProgress } from "@/store";
import { type Client, ClientError } from ".";
import { signedOut, useSession } from "./session";

interface Answer {
  word?: string;
  wrong?: boolean;
  remaining?: number | null;
  retry_at?: string;
  error?: string;
}

interface ServerProgress {
  words: Record<number, string>;
  guesses: Record<number, number>;
  name: string | null;
}

/** A request to the server API; a 401 means the session has ended. */
async function call(path: string, init?: RequestInit): Promise<Response> {
  const response = await fetch(path, {
    ...init,
    credentials: "same-origin",
    headers: { "content-type": "application/json" },
  });
  if (response.status === 401) {
    signedOut();
    throw new ClientError("Your session has ended. Please sign in again.");
  }
  return response;
}

async function body<T>(response: Response): Promise<T> {
  try {
    return (await response.json()) as T;
  } catch {
    return {} as T;
  }
}

const eventPath = (event: string) => `api/events/${encodeURIComponent(event)}`;

export const serverClient: Client = {
  mode: "server",

  async guess(event, day, answer) {
    const response = await call(`${eventPath(event)}/days/${day}/guess`, {
      method: "POST",
      body: JSON.stringify({ answer }),
    });
    const result = await body<Answer>(response);
    if (response.status === 429 && result.retry_at) {
      return { kind: "limited", retryAt: result.retry_at };
    }
    if (!response.ok) {
      throw new ClientError(
        result.error
          ? `${result.error[0].toUpperCase()}${result.error.slice(1)}.`
          : "Your answer could not be checked. Please try again.",
      );
    }
    const progress = useProgress.getState();
    progress.attempt(event, day);
    if (result.word) {
      progress.solve(event, day, result.word);
      return { kind: "solved", word: result.word };
    }
    return { kind: "wrong", remaining: result.remaining ?? null };
  },

  // Hints are released on the server's schedule; showing one is only a
  // choice of what to read, so it stays in the browser.
  revealHint(event, day, available) {
    useProgress.getState().revealHint(event, day, available);
  },

  async setName(event, name) {
    const response = await call("api/name", {
      method: "PUT",
      body: JSON.stringify({ name }),
    });
    if (!response.ok) {
      const result = await body<Answer>(response);
      throw new ClientError(result.error ?? "The name could not be saved.");
    }
    useProgress.getState().setName(event, name);
  },

  async load(event) {
    const response = await call(`${eventPath(event)}/progress`);
    if (!response.ok)
      throw new ClientError("Your progress could not be loaded.");
    const server = await body<ServerProgress>(response);
    const local = useProgress.getState().events[event];
    useProgress.getState().replace(event, {
      words: server.words,
      attempts: server.guesses,
      hintsUsed: local?.hintsUsed ?? {},
      name: server.name ?? "",
    });
  },

  async signOut() {
    await call("api/signout", { method: "POST" }).catch(() => {});
    useSession.getState().set({ status: "out" });
  },
};
