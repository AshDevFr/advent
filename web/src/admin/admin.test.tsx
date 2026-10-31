import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import AdminApp, { parseRoute, routePath } from "./AdminApp";

type Handler = (
  method: string,
  url: string,
  body: unknown,
) => Response | undefined;

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status });

function server(handle: Handler) {
  const calls: { method: string; url: string; body: unknown }[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      const method = init?.method ?? "GET";
      const body = init?.body ? JSON.parse(String(init.body)) : undefined;
      calls.push({ method, url, body });
      return handle(method, url, body) ?? json({}, 404);
    }),
  );
  return calls;
}

const event = {
  id: "2026",
  title: "Advent 2026",
  days: [
    { day: 1, date: "2026-12-01", title: "Capitals" },
    { day: 2, date: "2026-12-02", title: "Numbers" },
    { day: 3, date: "2026-12-03", title: null },
  ],
};

const stats = {
  players: 4,
  invited: 3,
  signed_in: 3,
  finished: 1,
  days: [
    { day: 1, solved: 2, completion: 2 / 3, median_guesses: 2, hint_out: 1 },
    { day: 2, solved: 1, completion: 1 / 3, median_guesses: 1, hint_out: 0 },
    { day: 3, solved: 0, completion: 0, median_guesses: null, hint_out: 0 },
  ],
};

/** A signed-in admin with one event. */
function signedIn(extra: Handler = () => undefined) {
  return server((method, url, body) => {
    if (url === "api/admin/session") return new Response(null, { status: 204 });
    if (url === "api/admin/events") return json([event]);
    if (url === "api/admin/events/2026/stats") return json(stats);
    if (url === "api/admin/invitations" && method === "GET")
      return json({ running: false, total: 0, sent: 0, failed: [] });
    return extra(method, url, body);
  });
}

beforeEach(() => {
  document.head.innerHTML = '<meta name="advent-mode" content="server">';
});

afterEach(() => {
  vi.unstubAllGlobals();
  document.head.innerHTML = "";
  window.history.replaceState(null, "", "/");
});

describe("admin section", () => {
  it("maps paths to pages and back", () => {
    expect(parseRoute("/admin", "/")).toEqual({ page: "overview" });
    expect(parseRoute("/demo/admin/players", "/demo/")).toEqual({
      page: "players",
    });
    expect(parseRoute("/admin/players/a%40b.c", "/")).toEqual({
      page: "player",
      email: "a@b.c",
    });
    expect(routePath({ page: "player", email: "a@b.c" })).toBe(
      "admin/players/a%40b.c",
    );
  });

  it("is not offered on a static site", () => {
    document.head.innerHTML = "";
    render(<AdminApp path="/admin" base="/" />);
    expect(screen.getByRole("status")).toHaveTextContent("advent serve");
  });

  it("asks for the password and says when it is wrong", async () => {
    const calls = server((_method, url, body) => {
      if (url === "api/admin/session") return json({}, 401);
      if (url === "api/admin/login")
        return (body as { password: string }).password === "right"
          ? new Response(null, { status: 204 })
          : json({ error: "wrong password" }, 401);
      if (url === "api/admin/events") return json([event]);
      if (url === "api/admin/events/2026/stats") return json(stats);
      if (url === "api/admin/invitations")
        return json({ running: false, total: 0, sent: 0, failed: [] });
    });
    const user = userEvent.setup();
    render(<AdminApp path="/admin" base="/" />);
    const field = await screen.findByLabelText("Admin password");
    await user.type(field, "wrong");
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Wrong password.",
    );

    await user.clear(field);
    await user.type(field, "right");
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    expect(
      await screen.findByRole("heading", { name: "Advent 2026" }),
    ).toBeInTheDocument();
    expect(calls.filter((c) => c.url === "api/admin/login")).toHaveLength(2);
  });

  it("shows the day completion over signed-in players, and the exports", async () => {
    signedIn();
    render(<AdminApp path="/admin" base="/" />);
    const days = await screen.findByRole("region", { name: "Days" });
    const row = within(days).getAllByRole("row")[1];
    expect(row).toHaveTextContent("67%");
    expect(row).toHaveTextContent("2 of 3 signed in, 4 players");
    expect(within(days).getAllByRole("row")[3]).toHaveTextContent(
      "Not open yet",
    );
    expect(screen.getByText("Solved day 2").nextSibling).toHaveTextContent("1");
    expect(
      screen.getByRole("link", { name: "Download finishers (CSV)" }),
    ).toHaveAttribute("href", "api/admin/events/2026/finishers.csv");
    expect(
      screen.getByText("1 player has not been invited yet."),
    ).toBeInTheDocument();
  });

  it("starts the invitations and follows them", async () => {
    let started = false;
    const calls = signedIn((method, url) => {
      if (url === "api/admin/invitations" && method === "POST") {
        started = true;
        return json({ running: true, total: 1, sent: 0, failed: [] }, 202);
      }
    });
    const user = userEvent.setup();
    render(<AdminApp path="/admin" base="/" />);
    await user.click(
      await screen.findByRole("button", { name: "Send invitations" }),
    );
    expect(
      await screen.findByText("Sending invitations: 0 of 1…"),
    ).toBeInTheDocument();
    expect(started).toBe(true);
    expect(calls).toContainEqual({
      method: "POST",
      url: "api/admin/invitations",
      body: {},
    });
  });

  it("previews an import before adding anyone", async () => {
    const calls = signedIn((_method, url) => {
      if (url === "api/admin/import/preview")
        return json({
          new: [
            { email: "bob@acme.com", name: null },
            { email: "alice@acme.com", name: "Alice" },
          ],
          existing: [],
          invalid: ["not-an-email"],
        });
      if (url === "api/admin/import")
        return json({
          created: ["bob@acme.com", "alice@acme.com"],
          updated: [],
          unchanged: [],
          invalid: ["not-an-email"],
        });
    });
    const user = userEvent.setup();
    render(<AdminApp path="/admin/import" base="/" />);
    await user.type(
      await screen.findByLabelText("Email addresses"),
      "bob@acme.com\nalice@acme.com,Alice\nnot-an-email",
    );
    await user.click(screen.getByRole("button", { name: "Preview" }));
    const preview = await screen.findByRole("region", { name: "Preview" });
    expect(preview).toHaveTextContent("New (2)");
    expect(preview).toHaveTextContent("Not an address (1)");
    expect(calls.some((c) => c.url === "api/admin/import")).toBe(false);

    await user.click(
      screen.getByRole("button", { name: "Import 2 new players" }),
    );
    expect(await screen.findByText(/2 player\(s\) added/)).toBeInTheDocument();
    expect(calls.filter((c) => c.url === "api/admin/import")).toHaveLength(1);
  });

  it("asks for confirmation before resetting a player's link", async () => {
    const calls = signedIn((method, url) => {
      if (url === "api/admin/events/2026/players/alice%40acme.com")
        return json({
          email: "alice@acme.com",
          name: "Alice",
          invited_at: "2026-11-21T09:00:00Z",
          last_seen: null,
          solved: 1,
          finished_at: null,
          days: [
            {
              day: 1,
              guesses: 3,
              solved_at: "2026-12-01T10:00:00Z",
              hint_out: false,
            },
          ],
        });
      if (
        url === "api/admin/players/alice%40acme.com/reset" &&
        method === "POST"
      )
        return json({ reset: true });
    });
    const user = userEvent.setup();
    render(<AdminApp path="/admin/players/alice%40acme.com" base="/" />);
    expect(
      await screen.findByRole("heading", { name: "Alice" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("progressbar", { name: "1 of 3 days" }),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Reset link" }));
    expect(calls.some((c) => c.url.endsWith("/reset"))).toBe(false);
    await user.click(
      screen.getByRole("button", { name: "Confirm: reset the link" }),
    );
    await waitFor(() =>
      expect(calls.filter((c) => c.url.endsWith("/reset"))).toHaveLength(1),
    );
    expect(await screen.findByText(/Link reset/)).toBeInTheDocument();
  });
});
