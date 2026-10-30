import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { exportCode, importCode, useProgress } from "@/store";
import { serve } from "@/test/fixtures";
import vectors from "../../../test-vectors.json";
import { DoorPanel } from "./DoorPanel";
import { Progress } from "./Progress";

// The first word vector: "Paris" opens a lock holding "light".
const lock = vectors.word[0];

function serveDay(hints: string[] = ["Think big.", "Capital city."]) {
  serve({
    "/2026/api/day/2.json": {
      day: 2,
      title: "Capitals",
      type: "teaser",
      markdown:
        "# Where?\n\n```\nprint('hi')\n```\n\n![map](map.webp)\n\n<script>alert(1)</script>",
    },
    "/2026/api/day/2/locks.json": {
      word_locks: [{ salt: lock.salt, iv: lock.iv, ct: lock.ct }],
    },
    "/2026/api/day/2/hint.json": { hints },
  });
}

const progress = () => useProgress.getState().events["2026"];

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  useProgress.setState({ events: {} });
});

describe("door panel", () => {
  it("renders code blocks and images from the day's assets, but no raw HTML", async () => {
    serveDay();
    render(<DoorPanel event="2026" day={2} onClose={() => {}} />);
    const dialog = await screen.findByRole("dialog", {
      name: "Day 2: Capitals",
    });
    expect(dialog.querySelector("pre")).toHaveTextContent("print('hi')");
    expect(within(dialog).getByRole("img", { name: "map" })).toHaveAttribute(
      "src",
      "/2026/api/day/2/map.webp",
    );
    expect(dialog.querySelector("script")).toBeNull();
  });

  it("accepts an answer in any case and punctuation, and stores the word", async () => {
    serveDay();
    const user = userEvent.setup();
    render(<DoorPanel event="2026" day={2} onClose={() => {}} />);
    await user.type(await screen.findByLabelText("Your answer"), "  Paris! ");
    await user.click(screen.getByRole("button", { name: "Check" }));

    expect(await screen.findByText("light")).toBeInTheDocument();
    expect(progress().words[2]).toBe("light");
    expect(progress().attempts[2]).toBe(1);
  });

  it("explains normalization on a wrong answer and counts the attempt", async () => {
    serveDay();
    const user = userEvent.setup();
    render(<DoorPanel event="2026" day={2} onClose={() => {}} />);
    await user.type(await screen.findByLabelText("Your answer"), "London!");
    await user.click(screen.getByRole("button", { name: "Check" }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("we read “London!” as “london”");
    expect(progress().attempts[2]).toBe(1);
    expect(progress().words[2]).toBeUndefined();
  });

  it("reveals due hints one at a time", async () => {
    serveDay();
    const user = userEvent.setup();
    render(<DoorPanel event="2026" day={2} onClose={() => {}} />);
    await user.click(
      await screen.findByRole("button", { name: "Show a hint (2 available)" }),
    );
    expect(screen.getByText("Think big.")).toBeInTheDocument();
    expect(screen.queryByText("Capital city.")).not.toBeInTheDocument();
    await user.click(
      screen.getByRole("button", { name: "Show a hint (1 available)" }),
    );
    expect(screen.getByText("Capital city.")).toBeInTheDocument();
    expect(progress().hintsUsed[2]).toBe(2);
    expect(
      screen.queryByRole("button", { name: /Show a hint/ }),
    ).not.toBeInTheDocument();
  });

  it("still works when localStorage throws", async () => {
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("QuotaExceededError");
    });
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("SecurityError");
    });
    serveDay();
    const user = userEvent.setup();
    render(<DoorPanel event="2026" day={2} onClose={() => {}} />);
    await user.type(await screen.findByLabelText("Your answer"), "paris");
    await user.click(screen.getByRole("button", { name: "Check" }));
    expect(await screen.findByText("light")).toBeInTheDocument();
  });
});

describe("progress code", () => {
  it("round-trips words, hints and attempts into an empty store", async () => {
    const saved = {
      words: { 1: "light", 2: "returns" },
      hintsUsed: { 2: 1 },
      attempts: { 1: 3, 2: 1 },
      name: "Ada",
    };
    const code = exportCode("2026", saved);
    expect(importCode("2026", code)).toEqual(saved);

    const user = userEvent.setup();
    render(<Progress event="2026" />);
    await user.click(screen.getByText("Move your progress to another device"));
    await user.type(screen.getByLabelText("Paste a progress code"), code);
    await user.click(screen.getByRole("button", { name: "Restore" }));

    expect(await screen.findByText("Progress restored.")).toBeInTheDocument();
    expect(progress()).toEqual(saved);
  });

  it("rejects an invalid code and changes nothing", async () => {
    useProgress.getState().solve("2026", 1, "light");
    const user = userEvent.setup();
    render(<Progress event="2026" />);
    await user.click(screen.getByText("Move your progress to another device"));
    await user.type(
      screen.getByLabelText("Paste a progress code"),
      "not-a-code",
    );
    await user.click(screen.getByRole("button", { name: "Restore" }));

    expect(
      await screen.findByText("That is not a progress code for this calendar."),
    ).toBeInTheDocument();
    await waitFor(() => expect(progress().words).toEqual({ 1: "light" }));
    expect(importCode("2027", exportCode("2026", progress()))).toBeNull();
  });
});
