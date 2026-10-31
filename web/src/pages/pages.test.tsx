import { render, screen, waitFor, within } from "@testing-library/react";

import userEvent from "@testing-library/user-event";

import { App } from "@/App";
import { readTheme } from "@/components/ThemeToggle";
import { useProgress } from "@/store";
import { event, serve, site } from "@/test/fixtures";

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  useProgress.setState({ events: {} });
});

describe("landing page", () => {
  it("shows an upcoming event with a countdown", async () => {
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(new Date("2026-11-30T12:00:00Z"));
    serve({
      "/site.json": site([
        event({ opens_at: "2026-12-01T00:00:00Z", released: [] }),
      ]),
    });
    render(<App path="/" />);

    expect(
      await screen.findByRole("heading", { name: "Advent 2026" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Acme Advent" })).toBeVisible();
    expect(
      screen.getByLabelText("Opens in 0 days, 12 hours and 0 minutes"),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("link", { name: "Open the calendar" }),
    ).not.toBeInTheDocument();
  });

  it("links to an event that has opened", async () => {
    serve({ "/site.json": site([event({ opens_at: null })]) });
    render(<App path="/" />);
    const link = await screen.findByRole("link", { name: "Open the calendar" });
    expect(link).toHaveAttribute("href", "2026/");
  });

  it("scopes each event's card for its theme colours", async () => {
    serve({ "/site.json": site([event({ opens_at: null })]) });
    render(<App path="/" />);
    const heading = await screen.findByRole("heading", { name: "Advent 2026" });
    const card = heading.closest("section");
    expect(card).toHaveAttribute("data-event", "2026");
    expect(card).toHaveClass("text-card-foreground");
  });

  it("puts a themed event's background behind its card", async () => {
    serve({
      "/site.json": site([
        event({ opens_at: null, background: "2026/theme/snow.png" }),
      ]),
    });
    render(<App path="/" />);
    const heading = await screen.findByRole("heading", { name: "Advent 2026" });
    const card = heading.closest("section") as HTMLElement;
    expect(within(card).getByTestId("card-backdrop")).toHaveStyle({
      backgroundImage: 'url("2026/theme/snow.png")',
    });
  });

  it("explains when the site cannot be loaded", async () => {
    serve({});
    render(<App path="/" />);
    expect(
      await screen.findByText("The calendar could not be loaded."),
    ).toBeInTheDocument();
  });
});

describe("event page", () => {
  it("shows each door's word, type or opening date", async () => {
    serve({ "/2026/event.json": event() });
    useProgress.getState().solve("2026", 1, "light");
    render(<App path="/2026/" />);

    const grid = await screen.findByRole("list", { name: "Calendar doors" });
    const solved = within(grid).getByRole("button", { name: "Day 1, solved" });
    expect(solved).toHaveTextContent("light");
    expect(solved).toHaveTextContent("teaser");
    expect(
      within(grid).getByRole("button", { name: "Day 2, open" }),
    ).toHaveTextContent("code");
    expect(
      within(grid).getByRole("button", { name: "Day 3, open" }),
    ).toHaveTextContent("company");
    for (const day of [4, 5, 6, 7]) {
      const door = within(grid).getByRole("img", {
        name: `Day ${day}, locked, opens Dec ${day}`,
      });
      expect(door).toHaveTextContent(`Opens Dec ${day}`);
      expect(
        within(grid).queryByRole("button", { name: new RegExp(`Day ${day},`) }),
      ).not.toBeInTheDocument();
    }
    expect(within(grid).getAllByRole("listitem")).toHaveLength(7);
  });

  it("returns focus to the door when its dialog closes", async () => {
    serve({
      "/2026/event.json": event(),
      "/2026/api/day/2.json": {
        day: 2,
        title: "Count",
        type: "code",
        markdown: "x",
      },
      "/2026/api/day/2/locks.json": { word_locks: [] },
      "/2026/api/day/2/hint.json": { hints: [] },
    });
    const user = userEvent.setup();
    render(<App path="/2026/" />);
    const door = await screen.findByRole("button", { name: "Day 2, open" });
    await user.click(door);
    await screen.findByRole("dialog", { name: "Count" });
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await waitFor(() => expect(door).toHaveFocus());
  });

  it("labels locked doors with their own date when days are spread out", async () => {
    const spread = Array.from({ length: 12 }, (_, i) => {
      const day = 1 + i * 2;
      return `2026-12-${String(day).padStart(2, "0")}`;
    });
    serve({
      "/2026/event.json": event({
        days: 12,
        dates: spread,
        released: [1],
        types: { 1: "teaser" },
      }),
    });
    render(<App path="/2026/" />);
    const grid = await screen.findByRole("list", { name: "Calendar doors" });
    expect(
      within(grid).getByRole("img", { name: "Day 2, locked, opens Dec 3" }),
    ).toBeVisible();
    expect(
      within(grid).getByRole("img", { name: "Day 12, locked, opens Dec 23" }),
    ).toHaveTextContent("Opens Dec 23");
  });

  it("fills the key strip with collected words", async () => {
    serve({ "/2026/event.json": event() });
    useProgress.getState().solve("2026", 2, "returns");
    render(<App path="/2026/" />);

    const strip = await screen.findByRole("list", {
      name: "Key words collected",
    });
    expect(screen.getByText("1 / 7 words")).toBeVisible();
    expect(within(strip).getByLabelText("Day 2: returns")).toHaveTextContent(
      "returns",
    );
    expect(within(strip).getByLabelText("Day 1: missing")).toBeVisible();
  });

  it("says when there is no such calendar", async () => {
    serve({});
    render(<App path="/nope/" />);
    expect(
      await screen.findByText("There is no calendar here."),
    ).toBeInTheDocument();
  });
});

describe("theme toggle", () => {
  it("defaults to the system setting and remembers an explicit choice", async () => {
    localStorage.clear();
    delete document.documentElement.dataset.theme;
    serve({ "/site.json": site([event({ opens_at: null })]) });
    const user = userEvent.setup();
    render(<App path="/" />);

    expect(await screen.findByRole("radio", { name: "System" })).toBeChecked();
    expect(document.documentElement.dataset.theme).toBeUndefined();

    await user.click(screen.getByRole("radio", { name: "Dark" }));
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(localStorage.getItem("advent-theme")).toBe("dark");
    expect(readTheme()).toBe("dark");

    await user.click(screen.getByRole("radio", { name: "System" }));
    expect(document.documentElement.dataset.theme).toBeUndefined();
    expect(localStorage.getItem("advent-theme")).toBeNull();
  });
});

describe("base path", () => {
  it("routes and links relative to a sub-path", async () => {
    serve({ "/site.json": site([event({ opens_at: null })]) });
    render(<App path="/demo/" base="/demo/" />);
    const link = await screen.findByRole("link", { name: "Open the calendar" });
    expect(link).toHaveAttribute("href", "2026/");
  });

  it("opens an event under a sub-path", async () => {
    serve({ "/2026/event.json": event() });
    render(<App path="/demo/2026/" base="/demo/" />);
    expect(
      await screen.findByRole("list", { name: "Calendar doors" }),
    ).toBeInTheDocument();
  });

  it("requests site data by relative URL", async () => {
    serve({ "/site.json": site([event({ opens_at: null })]) });
    render(<App path="/" base="/" />);
    await screen.findByRole("link", { name: "Open the calendar" });
    const requested = (fetch as ReturnType<typeof vi.fn>).mock.calls.map(
      ([url]) => String(url),
    );
    expect(requested.every((url) => !url.startsWith("/"))).toBe(true);
  });
});

describe("branding on the calendar", () => {
  it("shows the logo, the background and door artwork, and names the tab", async () => {
    serve({
      "/2026/event.json": event({
        headline: "Light the way",
        background: "2026/theme/background.svg",
        doors: ["2026/theme/fox.svg", "2026/theme/owl.svg"],
      }),
      "/site.json": site([event()]),
    });
    render(<App path="/2026/" />);

    const grid = await screen.findByRole("list", { name: "Calendar doors" });
    await waitFor(() =>
      expect(document.title).toBe("Light the way · Acme Advent"),
    );
    expect(document.querySelector('link[rel="icon"]')).toHaveAttribute(
      "href",
      "/brand/logo.svg",
    );
    expect(
      document.querySelector('header img[src="/brand/logo.svg"]'),
    ).not.toBeNull();
    expect(screen.getByTestId("backdrop").style.backgroundImage).toContain(
      "2026/theme/background.svg",
    );

    // Doors take the artwork in turn: day 4 the second, day 5 the first.
    const day4 = within(grid).getByRole("img", { name: /Day 4, locked/ });
    expect(day4.style.backgroundImage).toContain("2026/theme/owl.svg");
    const day5 = within(grid).getByRole("img", { name: /Day 5, locked/ });
    expect(day5.style.backgroundImage).toContain("2026/theme/fox.svg");
    expect(within(day5).getByText("5")).toHaveClass("text-foreground");
    const open = within(grid).getByRole("button", { name: "Day 2, open" });
    expect(open.style.backgroundImage).toBe("");
  });

  it("stays plain without theme decoration", async () => {
    serve({ "/2026/event.json": event() });
    render(<App path="/2026/" />);
    await screen.findByRole("list", { name: "Calendar doors" });
    expect(screen.queryByTestId("backdrop")).toBeNull();
  });
});
