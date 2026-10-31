import { useProgress } from "@/store";
import { ClientError } from ".";
import { serverClient } from "./server";
import { useSession } from "./session";

type Reply = { status: number; body: unknown };

/** Answer each request with the next reply, recording what was asked. */
function replies(...queue: Reply[]) {
  const calls: { url: string; init?: RequestInit }[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      calls.push({ url, init });
      const next = queue.shift() ?? { status: 500, body: {} };
      return new Response(JSON.stringify(next.body), { status: next.status });
    }),
  );
  return calls;
}

afterEach(() => {
  vi.unstubAllGlobals();
  useProgress.setState({ events: {} });
  useSession.setState({ session: { status: "checking" } });
});

const progress = () => useProgress.getState().events["2026"];

describe("server client", () => {
  it("sends the answer to the server and stores the word it returns", async () => {
    const calls = replies(
      { status: 200, body: { wrong: true, remaining: null } },
      { status: 200, body: { word: "light" } },
    );
    expect(await serverClient.guess("2026", 2, "Lyon")).toEqual({
      kind: "wrong",
      remaining: null,
    });
    expect(await serverClient.guess("2026", 2, "Paris")).toEqual({
      kind: "solved",
      word: "light",
    });
    expect(calls[1].url).toBe("api/events/2026/days/2/guess");
    expect(calls[1].init?.method).toBe("POST");
    expect(JSON.parse(String(calls[1].init?.body))).toEqual({
      answer: "Paris",
    });
    expect(calls.some((c) => c.url.includes("locks"))).toBe(false);
    expect(progress().words).toEqual({ 2: "light" });
    expect(progress().attempts).toEqual({ 2: 2 });
  });

  it("reports the attempt limit with the time to wait", async () => {
    replies(
      { status: 200, body: { wrong: true, remaining: 0 } },
      {
        status: 429,
        body: { error: "too many guesses", retry_at: "2026-12-03T13:00:00Z" },
      },
    );
    expect(await serverClient.guess("2026", 1, "a")).toEqual({
      kind: "wrong",
      remaining: 0,
    });
    expect(await serverClient.guess("2026", 1, "b")).toEqual({
      kind: "limited",
      retryAt: "2026-12-03T13:00:00Z",
    });
  });

  it("passes the server's own refusals on as messages", async () => {
    replies({
      status: 429,
      body: { error: "too many requests; try again in 40 s" },
    });
    await expect(serverClient.guess("2026", 1, "a")).rejects.toThrow(
      "Too many requests; try again in 40 s.",
    );
  });

  it("signs the player out when the session has ended", async () => {
    useSession.setState({
      session: { status: "in", email: "a@b.c", name: null },
    });
    replies({ status: 401, body: { error: "not signed in" } });
    await expect(serverClient.guess("2026", 1, "a")).rejects.toBeInstanceOf(
      ClientError,
    );
    expect(useSession.getState().session.status).toBe("out");
  });

  it("loads progress and the invited name, keeping hints read here", async () => {
    useProgress.getState().revealHint("2026", 1, 2);
    replies({
      status: 200,
      body: {
        words: { 1: "light", 3: "after" },
        guesses: { 1: 1, 2: 4, 3: 2 },
        name: "Alice",
        finished_at: null,
      },
    });
    await serverClient.load?.("2026");
    expect(progress()).toEqual({
      words: { 1: "light", 3: "after" },
      attempts: { 1: 1, 2: 4, 3: 2 },
      hintsUsed: { 1: 1 },
      name: "Alice",
    });
  });

  it("saves the name on the server and offers no progress code", async () => {
    const calls = replies({
      status: 200,
      body: { email: "a@b.c", name: "Ada" },
    });
    await serverClient.setName("2026", "Ada");
    expect(calls[0].url).toBe("api/name");
    expect(calls[0].init?.method).toBe("PUT");
    expect(progress().name).toBe("Ada");
    expect(serverClient.transfer).toBeUndefined();
  });
});
