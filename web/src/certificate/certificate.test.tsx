import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useProgress } from "@/store";
import { event, serve, site } from "@/test/fixtures";
import vectors from "../../../test-vectors.json";
import { CertificatePanel, CertificateView, wrap } from "./Certificate";
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

describe("certificate options", () => {
  it("shows the subtitle above the name and the signature, and the image on top", () => {
    render(
      <CertificateView
        event={threeDays}
        brandName="Acme"
        name="Ada"
        payload={{
          title: "Well done",
          message: "Every word.",
          subtitle: "is hereby recognised for",
          signature: "The Holiday Committee",
          image: "UklGRg==",
          image_position: "top",
          background: "UklGRg==",
        }}
      />,
    );
    const svg = screen.getByRole("img", {
      name: "Certificate for Ada: Well done",
    });
    const texts = [...svg.querySelectorAll("text")].map((t) => ({
      text: t.textContent,
      y: Number(t.getAttribute("y")),
    }));
    const y = (s: string) => texts.find((t) => t.text === s)?.y ?? -1;
    expect(y("is hereby recognised for")).toBeGreaterThan(y("Well done"));
    expect(y("Ada")).toBeGreaterThan(y("is hereby recognised for"));
    expect(y("The Holiday Committee")).toBeGreaterThan(y("Every word."));
    const images = [...svg.querySelectorAll("image")].map((i) => ({
      href: i.getAttribute("href") ?? "",
      y: Number(i.getAttribute("y") ?? 0),
    }));
    // The background covers the certificate; the keeper's image is at the top.
    expect(
      images.filter((i) => i.href.startsWith("data:image/webp")),
    ).toHaveLength(2);
    expect(images.some((i) => i.y === 60)).toBe(true);
  });
});

describe("certificate frame and print", () => {
  const payload = {
    title: "Well done",
    message: "Every word.",
    background: "UklGRg==",
  };
  const view = (over = {}) =>
    render(
      <CertificateView
        event={threeDays}
        brandName="Acme"
        name="Ada"
        payload={{ ...payload, ...over }}
      />,
    ).container.querySelector("svg") as SVGSVGElement;

  it("washes the background and draws a border by default", () => {
    const svg = view();
    expect(svg.querySelector('rect[opacity="0.8"]')).not.toBeNull();
    expect(svg.querySelector("rect[stroke]")).not.toBeNull();
    expect(svg.querySelector("ellipse[data-glow]")).toBeNull();
  });

  it("shows a frame at full strength, without its own border", () => {
    const svg = view({ background_wash: false });
    expect(svg.querySelector('rect[opacity="0.8"]')).toBeNull();
    expect(svg.querySelector("rect[stroke]")).toBeNull();
    // The middle fades to the page colour and every line of text has a
    // halo, so it reads on any frame without hiding it.
    expect(svg.querySelector("ellipse[data-glow]")).not.toBeNull();
    for (const text of svg.querySelectorAll("text")) {
      expect(text).toHaveAttribute("paint-order", "stroke");
    }
  });

  it("prints the certificate on its own page", async () => {
    const print = vi.fn();
    vi.stubGlobal("print", print);
    view();
    await userEvent.click(screen.getByRole("button", { name: "Print" }));
    expect(print).toHaveBeenCalledOnce();
    // Only while the dialog is open: the page prints as usual afterwards.
    expect(document.documentElement.dataset.printing).toBe("one");
    window.dispatchEvent(new Event("afterprint"));
    expect(document.documentElement.dataset.printing).toBeUndefined();
    // The print stylesheet shows only what carries this marker.
    expect(
      screen
        .getByRole("img", { name: /^Certificate for Ada/ })
        .closest(`[data-print="one"]`),
    ).not.toBeNull();
  });
});

describe("several backgrounds", () => {
  const payload = {
    title: "Well done",
    message: "Every word.",
    backgrounds: ["AAAA", "BBBB"],
    background_wash: false,
  };
  const hrefs = (svg: Element) =>
    [...svg.querySelectorAll("image")].map((i) => i.getAttribute("href"));

  it("lets the player pick the background", async () => {
    render(
      <CertificateView
        event={threeDays}
        brandName="Acme"
        name="Ada"
        payload={payload}
      />,
    );
    const svg = screen.getByRole("img", { name: /^Certificate for Ada/ });
    expect(hrefs(svg)).toContain("data:image/webp;base64,AAAA");
    const second = screen.getByRole("button", { name: "Background 2" });
    expect(second).toHaveAttribute("aria-pressed", "false");
    await userEvent.click(second);
    expect(second).toHaveAttribute("aria-pressed", "true");
    expect(hrefs(svg)).toContain("data:image/webp;base64,BBBB");
    expect(hrefs(svg)).not.toContain("data:image/webp;base64,AAAA");
  });

  it("prints one page per background", async () => {
    const print = vi.fn();
    vi.stubGlobal("print", print);
    const { container } = render(
      <CertificateView
        event={threeDays}
        brandName="Acme"
        name="Ada"
        payload={payload}
      />,
    );
    await userEvent.click(screen.getByRole("button", { name: "Print all" }));
    expect(print).toHaveBeenCalledOnce();
    expect(document.documentElement.dataset.printing).toBe("all");
    const pages = container.querySelectorAll('[data-print="all"] svg');
    expect([...pages].map((page) => hrefs(page)[0])).toEqual([
      "data:image/webp;base64,AAAA",
      "data:image/webp;base64,BBBB",
    ]);
    window.dispatchEvent(new Event("afterprint"));
  });

  it("offers no choice for a single background", () => {
    render(
      <CertificateView
        event={threeDays}
        brandName="Acme"
        name="Ada"
        payload={{
          title: "Well done",
          message: "Every word.",
          background: "AAAA",
        }}
      />,
    );
    expect(screen.queryByRole("button", { name: "Background 1" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Print all" })).toBeNull();
  });
});

describe("rewards", () => {
  it("lists links and decrypts a file on demand", async () => {
    const bytes = new TextEncoder().encode("wallpaper bytes");
    const rawKey = crypto.getRandomValues(new Uint8Array(32));
    const iv = crypto.getRandomValues(new Uint8Array(12));
    const key = await crypto.subtle.importKey("raw", rawKey, "AES-GCM", false, [
      "encrypt",
    ]);
    const sealed = new Uint8Array(
      await crypto.subtle.encrypt({ name: "AES-GCM", iv }, key, bytes),
    );
    const b64 = (b: Uint8Array) => btoa(String.fromCharCode(...b));
    serve({ "/2026/api/rewards/1.bin": sealed });
    const saved: { blob: Blob; name: string }[] = [];
    vi.spyOn(await import("./exportPng"), "download").mockImplementation(
      (blob, name) => {
        saved.push({ blob, name });
      },
    );

    const { Rewards } = await import("./Rewards");
    const user = userEvent.setup();
    render(
      <Rewards
        event="2026"
        rewards={[
          { label: "Bonus puzzle", url: "https://example.com/bonus" },
          {
            label: "Wallpaper",
            file: {
              name: "wall.png",
              size: bytes.length,
              index: 1,
              key: b64(rawKey),
              iv: b64(iv),
            },
          },
        ]}
      />,
    );
    expect(screen.getByRole("link", { name: "Open link" })).toHaveAttribute(
      "href",
      "https://example.com/bonus",
    );
    await user.click(screen.getByRole("button", { name: /Download wall.png/ }));
    await waitFor(() => expect(saved).toHaveLength(1));
    expect(saved[0].name).toBe("wall.png");
    // Compare as plain arrays: jsdom's and Node's Uint8Array never compare equal.
    expect(
      Array.from(new Uint8Array(await saved[0].blob.arrayBuffer())),
    ).toEqual(Array.from(bytes));
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
