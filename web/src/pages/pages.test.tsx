import { render, screen, within } from "@testing-library/react";

import { App } from "@/App";
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
  it("shows locked, open and solved doors", async () => {
    serve({ "/2026/event.json": event() });
    useProgress.getState().solve("2026", 1, "light");
    render(<App path="/2026/" />);

    const grid = await screen.findByRole("list", { name: "Calendar doors" });
    expect(
      within(grid).getByRole("button", { name: "Day 1, solved" }),
    ).toBeVisible();
    expect(
      within(grid).getByRole("button", { name: "Day 2, open" }),
    ).toBeVisible();
    expect(
      within(grid).getByRole("button", { name: "Day 3, open" }),
    ).toBeVisible();
    for (const day of [4, 5, 6, 7]) {
      expect(
        within(grid).getByRole("img", { name: `Day ${day}, locked` }),
      ).toBeVisible();
      expect(
        within(grid).queryByRole("button", { name: `Day ${day}, locked` }),
      ).not.toBeInTheDocument();
    }
    expect(within(grid).getAllByRole("listitem")).toHaveLength(7);
  });

  it("fills the key strip with collected words", async () => {
    serve({ "/2026/event.json": event() });
    useProgress.getState().solve("2026", 2, "returns");
    render(<App path="/2026/" />);

    const strip = await screen.findByRole("list", {
      name: "Key words collected",
    });
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
