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
    expect(link).toHaveAttribute("href", "/2026/");
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
