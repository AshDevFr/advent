import { render, screen } from "@testing-library/react";
import { createElement } from "react";
import { Progress } from "@/door/Progress";
import { useProgress } from "@/store";
import { serve } from "@/test/fixtures";
import vectors from "../../../test-vectors.json";
import { type Client, client, hostingMode, setClient } from ".";
import { staticClient } from "./static";

// The first word vector: "Paris" opens a lock holding "light".
const lock = vectors.word[0];

beforeEach(() => {
  serve({
    "/2026/api/day/2/locks.json": {
      word_locks: [{ salt: lock.salt, iv: lock.iv, ct: lock.ct }],
    },
  });
});

afterEach(() => {
  vi.unstubAllGlobals();
  useProgress.setState({ events: {} });
  setClient(null);
  document.head.innerHTML = "";
});

const progress = () => useProgress.getState().events["2026"];

describe("static client", () => {
  it("is the client unless the page says it is served", () => {
    expect(hostingMode()).toBe("static");
    expect(client()).toBe(staticClient);
    document.head.innerHTML = '<meta name="advent-mode" content="server">';
    expect(hostingMode()).toBe("server");
  });

  it("solves a day with a right answer and counts every guess", async () => {
    expect(await staticClient.guess("2026", 2, "Lyon")).toEqual({
      kind: "wrong",
      remaining: null,
    });
    expect(progress().words).toEqual({});
    expect(await staticClient.guess("2026", 2, " PARIS! ")).toEqual({
      kind: "solved",
      word: "light",
    });
    expect(progress().words).toEqual({ 2: "light" });
    expect(progress().attempts).toEqual({ 2: 2 });
  });

  it("keeps hints and the name in the browser", async () => {
    staticClient.revealHint("2026", 2, 2);
    staticClient.revealHint("2026", 2, 2);
    staticClient.revealHint("2026", 2, 2);
    await staticClient.setName("2026", "Ada");
    expect(progress().hintsUsed).toEqual({ 2: 2 });
    expect(progress().name).toBe("Ada");
  });

  it("moves progress with a code", () => {
    useProgress.getState().solve("2026", 1, "light");
    const code = staticClient.transfer?.exportCode("2026") ?? "";
    useProgress.setState({ events: {} });
    expect(staticClient.transfer?.importCode("2026", code)).toBe(true);
    expect(progress().words).toEqual({ 1: "light" });
    expect(staticClient.transfer?.importCode("2026", "junk")).toBe(false);
  });

  it("offers no progress code where the client cannot move progress", () => {
    const { transfer: _, ...withoutTransfer } = staticClient;
    setClient(withoutTransfer as Client);
    const { container } = render(createElement(Progress, { event: "2026" }));
    expect(container).toBeEmptyDOMElement();
    expect(
      screen.queryByText("Move your progress to another device"),
    ).toBeNull();
  });
});
