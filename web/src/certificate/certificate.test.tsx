import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useProgress } from "@/store";
import { event, serve, site } from "@/test/fixtures";
import vectors from "../../../test-vectors.json";
import { CertificatePanel, wrap } from "./Certificate";
import { svgToPng } from "./exportPng";

// The first final vector: three words open {"title":"Well done",...}.
const sealed = vectors.final[0];
const threeDays = event({ days: 3, released: [1, 2, 3] });

function solveAll(count: number) {
  sealed.words.slice(0, count).forEach((word, i) => {
    useProgress.getState().solve("2026", i + 1, word);
  });
  useProgress.setState((state) => ({
    events: {
      ...state.events,
      "2026": {
        ...state.events["2026"],
        hintsUsed: { 1: 1, 2: 2 },
        attempts: { 1: 4 },
      },
    },
  }));
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  useProgress.setState({ events: {} });
});

describe("certificate", () => {
  it("unlocks with every word and shows the name, title and year only", async () => {
    serve({
      "/2026/api/final.json": {
        salt: sealed.salt,
        iv: sealed.iv,
        ct: sealed.ct,
      },
      "/site.json": site([threeDays]),
    });
    solveAll(3);
    const user = userEvent.setup();
    render(<CertificatePanel event={threeDays} brandName="Acme Advent" />);

    await user.type(
      screen.getByLabelText("Name for the certificate"),
      "Ada Lovelace",
    );
    await user.click(
      screen.getByRole("button", { name: "Unlock certificate" }),
    );

    const svg = await screen.findByRole("img", {
      name: "Certificate for Ada Lovelace: Well done",
    });
    expect(svg).toHaveTextContent("Ada Lovelace");
    expect(svg).toHaveTextContent("Well done");
    expect(svg).toHaveTextContent("You made it.");
    expect(svg).toHaveTextContent("Acme Advent");
    expect(svg).toHaveTextContent("Advent 2026");
    // Progress stats can be reset in localStorage, so they are not shown.
    expect(svg).not.toHaveTextContent(/hint|attempt|Completed/);
    expect(useProgress.getState().events["2026"].name).toBe("Ada Lovelace");
  });

  it("is visible from the start, with unlocking disabled until every word is in", () => {
    solveAll(2);
    render(<CertificatePanel event={threeDays} brandName="Acme" />);
    expect(screen.getByLabelText("Name for the certificate")).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Unlock certificate" }),
    ).toBeDisabled();
  });

  it("explains when the words do not open it", async () => {
    serve({
      "/2026/api/final.json": {
        salt: sealed.salt,
        iv: sealed.iv,
        ct: sealed.ct,
      },
    });
    solveAll(3);
    useProgress.getState().solve("2026", 2, "wrongword");
    const user = userEvent.setup();
    render(<CertificatePanel event={threeDays} brandName="Acme" />);
    await user.type(screen.getByLabelText("Name for the certificate"), "Ada");
    await user.click(
      screen.getByRole("button", { name: "Unlock certificate" }),
    );
    expect(await screen.findByRole("alert")).toHaveTextContent("do not open");
  });
});

describe("PNG export", () => {
  it("draws the SVG on a canvas and returns an image/png blob", async () => {
    // jsdom has no canvas or image decoding: stand in for both.
    const drawImage = vi.fn();
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue({
      drawImage,
    } as unknown as CanvasRenderingContext2D);
    vi.spyOn(HTMLCanvasElement.prototype, "toBlob").mockImplementation(
      function (this: HTMLCanvasElement, callback, type) {
        callback(
          new Blob([new Uint8Array([0x89, 0x50, 0x4e, 0x47])], { type }),
        );
      },
    );
    vi.stubGlobal(
      "Image",
      class {
        onload: (() => void) | null = null;
        set src(_: string) {
          queueMicrotask(() => this.onload?.());
        }
      },
    );
    URL.createObjectURL = vi.fn(() => "blob:svg");
    URL.revokeObjectURL = vi.fn();

    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("viewBox", "0 0 1200 850");
    const png = await svgToPng(svg);

    expect(png.type).toBe("image/png");
    expect(png.size).toBeGreaterThan(0);
    expect(drawImage).toHaveBeenCalledOnce();
  });
});

it("wraps long messages on word boundaries", () => {
  expect(wrap("one two three four", 9)).toEqual(["one two", "three", "four"]);
});
