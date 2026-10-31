import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App, signInLink } from "@/App";
import { setClient } from "@/client";
import { useSession } from "@/client/session";
import { useProgress } from "@/store";
import { event, site } from "@/test/fixtures";

type Handler = (url: string, init?: RequestInit) => Response | undefined;

/** A fake server: each request goes to `handle`, else 404. */
function server(handle: Handler) {
  const calls: { url: string; method: string }[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      const path = url.replace(/^\//, "");
      calls.push({ url: path, method: init?.method ?? "GET" });
      return handle(path, init) ?? new Response("{}", { status: 404 });
    }),
  );
  return calls;
}

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status });

// Reset before each test rather than after: testing-library unmounts the
// previous page after the hooks of this file run, and a still-mounted page
// would react to a reset session by checking it again.
beforeEach(() => {
  document.head.innerHTML = '<meta name="advent-mode" content="server">';
  setClient(null);
  useSession.setState({ session: { status: "checking" } });
});

afterEach(() => {
  vi.unstubAllGlobals();
  document.head.innerHTML = "";
  setClient(null);
  useProgress.setState({ events: {} });
  window.history.replaceState(null, "", "/");
});

describe("server-hosted calendar", () => {
  it("finds the token in a sign-in path", () => {
    expect(signInLink("/signin/abc", "/")).toBe("abc");
    expect(signInLink("/demo/signin/a%2Bb", "/demo/")).toBe("a+b");
    expect(signInLink("/signin/", "/")).toBeNull();
    expect(signInLink("/2026/", "/")).toBeNull();
  });

  it("only signs in from a link once the button is clicked", async () => {
    const calls = server((url, init) => {
      if (url === "api/signin/tok") return json({ email: "alice@acme.com" });
      if (url === "api/signin" && init?.method === "POST")
        return json({ email: "alice@acme.com", name: "Alice" });
      if (url === "site.json") return json(site([event()]));
    });
    const user = userEvent.setup();
    render(<App path="/signin/tok" base="/" />);
    const button = await screen.findByRole("button", {
      name: "Sign in as alice@acme.com",
    });
    expect(calls.some((c) => c.method === "POST")).toBe(false);
    expect(useSession.getState().session.status).not.toBe("in");

    await user.click(button);
    await waitFor(() =>
      expect(useSession.getState().session.status).toBe("in"),
    );
    expect(calls.filter((c) => c.method === "POST")).toEqual([
      { url: "api/signin", method: "POST" },
    ]);
    expect(window.location.pathname).toBe("/");
  });

  it("explains an invalid link and offers to send the current one", async () => {
    server(() => undefined);
    render(<App path="/signin/old" base="/" />);
    expect(await screen.findByRole("alert")).toHaveTextContent("not valid");
    expect(screen.getByLabelText("Your email address")).toBeInTheDocument();
  });

  it("asks visitors without a session for their address", async () => {
    const calls = server((url) => {
      if (url === "api/session") return json({ error: "not signed in" }, 401);
      if (url === "api/signin/request")
        return json(
          {
            message:
              "If this address is invited, a sign-in link is on its way.",
          },
          202,
        );
    });
    const user = userEvent.setup();
    render(<App path="/2026/" base="/" />);
    await user.type(
      await screen.findByLabelText("Your email address"),
      "alice@acme.com",
    );
    await user.click(screen.getByRole("button", { name: "Send my link" }));
    expect(await screen.findByRole("status")).toHaveTextContent(
      "a sign-in link is on its way",
    );
    expect(calls).toContainEqual({ url: "api/signin/request", method: "POST" });
  });

  it("shows the calendar with the server's progress once signed in", async () => {
    const calls = server((url) => {
      if (url === "api/session")
        return json({ email: "alice@acme.com", name: "Alice" });
      if (url === "api/events/2026/progress")
        return json({
          words: { 1: "light" },
          guesses: { 1: 1 },
          name: "Alice",
        });
      if (url === "2026/event.json") return json(event());
      if (url === "site.json") return json(site([event()]));
    });
    render(<App path="/2026/" base="/" />);
    expect(
      await screen.findByRole("button", { name: "Sign out" }),
    ).toBeInTheDocument();
    await waitFor(() =>
      expect(useProgress.getState().events["2026"]?.words).toEqual({
        1: "light",
      }),
    );
    expect(await screen.findByDisplayValue("Alice")).toBeInTheDocument();
    expect(
      screen.queryByText("Move your progress to another device"),
    ).toBeNull();
    expect(calls.some((c) => c.url.includes("locks"))).toBe(false);
  });
});
