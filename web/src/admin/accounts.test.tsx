import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import AdminApp, { parseRoute } from "./AdminApp";

type Handler = (
  method: string,
  url: string,
  body: unknown,
) => Response | undefined;

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status });
const noContent = () => new Response(null, { status: 204 });

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

const event = { id: "2026", title: "Advent 2026", days: [] };
const stats = { players: 0, invited: 0, signed_in: 0, finished: 0, days: [] };

/** A server in accounts mode; `signedIn` flips once a sign-in succeeds. */
function accounts(setupNeeded: boolean, extra: Handler = () => undefined) {
  let signedIn = false;
  const calls = server((method, url, body) => {
    if (url === "api/admin/mode")
      return json({ auth: "accounts", setup_needed: setupNeeded && !signedIn });
    if (url === "api/admin/session")
      return signedIn ? noContent() : json({ error: "not signed in" }, 401);
    if (url === "api/admin/events") return json([event]);
    if (url === "api/admin/events/2026/stats") return json(stats);
    if (url === "api/admin/invitations")
      return json({ running: false, total: 0, sent: 0, failed: [] });
    const reply = extra(method, url, body);
    if (reply?.ok && (url === "api/admin/setup" || url === "api/admin/signin"))
      signedIn = true;
    return reply;
  });
  return calls;
}

beforeEach(() => {
  document.head.innerHTML = '<meta name="advent-mode" content="server">';
});

afterEach(() => {
  vi.unstubAllGlobals();
  document.head.innerHTML = "";
  window.history.replaceState(null, "", "/");
});

describe("admin accounts", () => {
  it("routes sign-in links", () => {
    expect(parseRoute("/admin/signin/tok", "/")).toEqual({
      page: "signin",
      token: "tok",
    });
  });

  it("creates the first admin with the setup code", async () => {
    const calls = accounts(true, (_m, url, body) => {
      if (url === "api/admin/setup")
        return (body as { code: string }).code === "K7Q2-9XPL"
          ? noContent()
          : json({ error: "the setup code is wrong" }, 403);
    });
    const user = userEvent.setup();
    render(<AdminApp path="/admin" base="/" />);
    await user.type(await screen.findByLabelText("Setup code"), "WRONG");
    await user.type(
      screen.getByLabelText("Your email address"),
      "ana@acme.com",
    );
    await user.click(
      screen.getByRole("button", { name: "Create and sign in" }),
    );
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "setup code is wrong",
    );

    await user.clear(screen.getByLabelText("Setup code"));
    await user.type(screen.getByLabelText("Setup code"), "K7Q2-9XPL");
    await user.click(
      screen.getByRole("button", { name: "Create and sign in" }),
    );
    expect(
      await screen.findByRole("heading", { name: "Advent 2026" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Admins" })).toBeInTheDocument();
    expect(calls.filter((c) => c.url === "api/admin/setup")).toHaveLength(2);
  });

  it("asks admins for their address, not a password", async () => {
    const calls = accounts(false, (_m, url) => {
      if (url === "api/admin/login")
        return json(
          {
            message:
              "If this address belongs to an admin, a sign-in link is on its way.",
          },
          202,
        );
    });
    const user = userEvent.setup();
    render(<AdminApp path="/admin" base="/" />);
    await user.type(
      await screen.findByLabelText("Your admin email address"),
      "ana@acme.com",
    );
    await user.click(
      screen.getByRole("button", { name: "Send me a sign-in link" }),
    );
    expect(await screen.findByRole("status")).toHaveTextContent(
      "within 15 minutes",
    );
    expect(screen.queryByLabelText("Admin password")).toBeNull();
    expect(calls).toContainEqual({
      method: "POST",
      url: "api/admin/login",
      body: { email: "ana@acme.com" },
    });
  });

  it("signs in from a link only on the button", async () => {
    const calls = accounts(false, (method, url) => {
      if (url === "api/admin/signin/tok")
        return json({ email: "ana@acme.com" });
      if (url === "api/admin/signin" && method === "POST") return noContent();
    });
    const user = userEvent.setup();
    render(<AdminApp path="/admin/signin/tok" base="/" />);
    const button = await screen.findByRole("button", {
      name: "Sign in as ana@acme.com",
    });
    expect(calls.some((c) => c.method === "POST")).toBe(false);
    await user.click(button);
    expect(
      await screen.findByRole("heading", { name: "Advent 2026" }),
    ).toBeInTheDocument();
    expect(window.location.pathname).toBe("/admin");
  });

  it("explains a used link", async () => {
    accounts(false);
    render(<AdminApp path="/admin/signin/old" base="/" />);
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "used or has expired",
    );
    expect(
      screen.getByLabelText("Your admin email address"),
    ).toBeInTheDocument();
  });

  it("offers single sign-on and shows why it was refused", async () => {
    server((_m, url) => {
      if (url === "api/admin/mode")
        return json({
          auth: "accounts",
          setup_needed: false,
          oidc: "Acme SSO",
        });
      if (url === "api/admin/session") return json({}, 401);
    });
    window.history.replaceState(
      null,
      "",
      "/admin?error=you%20are%20not%20in%20the%20admin%20group",
    );
    render(<AdminApp path="/admin" base="/" />);
    expect(
      await screen.findByRole("link", { name: "Sign in with Acme SSO" }),
    ).toHaveAttribute("href", "api/admin/oidc/start");
    expect(screen.getByRole("alert")).toHaveTextContent(
      "You are not in the admin group.",
    );
    expect(
      screen.getByLabelText("Your admin email address"),
    ).toBeInTheDocument();
  });

  it("removes an admin only after confirmation", async () => {
    const calls = accounts(false, (method, url) => {
      if (url === "api/admin/signin/tok")
        return json({ email: "ana@acme.com" });
      if (url === "api/admin/signin") return noContent();
      if (url === "api/admin/admins")
        return json([
          { email: "ana@acme.com", created_at: "2026-11-20T09:00:00Z" },
          { email: "bo@acme.com", created_at: "2026-11-21T09:00:00Z" },
        ]);
      if (url === "api/admin/admins/bo%40acme.com" && method === "DELETE")
        return noContent();
    });
    const user = userEvent.setup();
    render(<AdminApp path="/admin/signin/tok" base="/" />);
    await user.click(
      await screen.findByRole("button", { name: "Sign in as ana@acme.com" }),
    );
    await user.click(await screen.findByRole("button", { name: "Admins" }));
    const removes = await screen.findAllByRole("button", { name: "Remove" });
    await user.click(removes[1]);
    expect(calls.some((c) => c.method === "DELETE")).toBe(false);
    await user.click(screen.getByRole("button", { name: "Confirm removal" }));
    await waitFor(() =>
      expect(calls.filter((c) => c.method === "DELETE")).toHaveLength(1),
    );
    expect(await screen.findByText("bo@acme.com removed.")).toBeInTheDocument();
  });
});
