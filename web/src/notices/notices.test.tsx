import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { App } from "@/App";
import { event, serve } from "@/test/fixtures";
import { Notices } from "./Notices";

const notices = [
  { id: "aaaaaaaaaaaa", text: "Hints arrive at midnight." },
  { id: "bbbbbbbbbbbb", text: "Day 2 was fixed." },
];

afterEach(() => {
  vi.unstubAllGlobals();
  localStorage.clear();
});

describe("notices", () => {
  it("shows every published notice on the event page", async () => {
    serve({ "/2026/event.json": event({ notices }) });
    render(<App path="/2026/" />);
    const region = await screen.findByRole("region", { name: "Announcements" });
    expect(within(region).getByText("Hints arrive at midnight.")).toBeVisible();
    expect(within(region).getByText("Day 2 was fixed.")).toBeVisible();
  });

  it("renders nothing without notices", async () => {
    serve({ "/2026/event.json": event({ notices: [] }) });
    render(<App path="/2026/" />);
    await screen.findByRole("list", { name: "Calendar doors" });
    expect(screen.queryByRole("region", { name: "Announcements" })).toBeNull();
  });

  it("remembers a dismissal across reloads, per notice", async () => {
    const user = userEvent.setup();
    const { unmount } = render(<Notices event="2026" notices={notices} />);
    const first = screen.getByText("Hints arrive at midnight.")
      .parentElement as HTMLElement;
    await user.click(
      within(first).getByRole("button", { name: "Dismiss this notice" }),
    );
    expect(screen.queryByText("Hints arrive at midnight.")).toBeNull();
    unmount();

    render(<Notices event="2026" notices={notices} />);
    expect(screen.queryByText("Hints arrive at midnight.")).toBeNull();
    expect(screen.getByText("Day 2 was fixed.")).toBeVisible();
  });

  it("shows an edited notice again", () => {
    localStorage.setItem(
      "advent-dismissed-notices:2026",
      JSON.stringify(["aaaaaaaaaaaa"]),
    );
    render(
      <Notices
        event="2026"
        notices={[{ id: "cccccccccccc", text: "Hints arrive at 1 a.m. now." }]}
      />,
    );
    expect(screen.getByText("Hints arrive at 1 a.m. now.")).toBeVisible();
  });
});
