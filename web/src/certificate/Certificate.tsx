import { type FormEvent, useEffect, useId, useRef, useState } from "react";

import { api, type EventInfo } from "@/api";
import { Button } from "@/components/ui/button";
import {
  InsecureContextError,
  openFinal,
  type Certificate as Payload,
} from "@/crypto";
import { useEventProgress, useProgress } from "@/store";
import { download, svgToPng } from "./exportPng";

/**
 * Always shown, so players know what the words are for; unlocking needs
 * every word. The key is derived from the words in the browser and opens
 * the certificate there: nothing is checked on a server.
 */
export function CertificatePanel({
  event,
  brandName,
}: {
  event: EventInfo;
  brandName: string;
}) {
  const progress = useEventProgress(event.id);
  const words = Array.from(
    { length: event.days },
    (_, i) => progress.words[i + 1],
  );
  const complete = words.every(Boolean);
  const [name, setName] = useState(progress.name);
  const [payload, setPayload] = useState<Payload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const inputId = useId();

  async function unlock(e: FormEvent) {
    e.preventDefault();
    if (!complete || !name.trim()) return;
    useProgress.getState().setName(event.id, name.trim());
    try {
      const opened = await openFinal(
        words as string[],
        await api.final(event.id),
      );
      if (opened) setPayload(opened);
      else
        setError(
          "Your words do not open the certificate. One may have been stored wrongly: reopen the days and check each word.",
        );
    } catch (e) {
      setError(
        e instanceof InsecureContextError
          ? e.message
          : "The certificate could not be loaded.",
      );
    }
  }

  return (
    <section
      aria-label="Certificate"
      className="flex flex-col gap-4 rounded-lg border bg-card p-4 sm:p-5"
    >
      {!payload && (
        <form
          onSubmit={unlock}
          className="flex flex-col gap-4 md:flex-row md:items-center"
        >
          <p className="flex-1 text-sm text-muted-foreground">
            When every word is in, enter the name that goes on your certificate.
            The key is derived from your words in your browser, and it decrypts
            the certificate. Nothing is checked on a server.
          </p>
          <div className="flex flex-col gap-2 sm:flex-row">
            <label htmlFor={inputId} className="sr-only">
              Name for the certificate
            </label>
            <input
              id={inputId}
              value={name}
              onChange={(e) => setName(e.target.value)}
              autoComplete="name"
              placeholder="Name on certificate"
              className="h-9 w-full min-w-0 rounded-md border bg-background px-3 font-mono text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring md:w-56"
            />
            <Button type="submit" disabled={!complete}>
              Unlock certificate
            </Button>
          </div>
        </form>
      )}
      {error && (
        <p role="alert" className="text-sm">
          {error}
        </p>
      )}
      {payload && (
        <CertificateView
          event={event}
          brandName={brandName}
          payload={payload}
          name={name.trim()}
        />
      )}
    </section>
  );
}

/** Read the live theme so the exported image matches what is on screen. */
function themeColour(name: string, fallback: string): string {
  const value = getComputedStyle(document.documentElement)
    .getPropertyValue(`--${name}`)
    .trim();
  return value || fallback;
}

/** Split text into lines of at most `width` characters, on word boundaries. */
export function wrap(text: string, width: number): string[] {
  const lines: string[] = [];
  let line = "";
  for (const word of text.split(/\s+/).filter(Boolean)) {
    if (line && line.length + 1 + word.length > width) {
      lines.push(line);
      line = word;
    } else {
      line = line ? `${line} ${word}` : word;
    }
  }
  if (line) lines.push(line);
  return lines;
}

async function asDataUrl(path: string): Promise<string | null> {
  try {
    const blob = await (await fetch(path)).blob();
    return await new Promise((resolve) => {
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result as string);
      reader.onerror = () => resolve(null);
      reader.readAsDataURL(blob);
    });
  } catch {
    return null;
  }
}

export function CertificateView({
  event,
  brandName,
  payload,
  name,
}: {
  event: EventInfo;
  brandName: string;
  payload: Payload;
  name: string;
}) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [logo, setLogo] = useState<string | null>(null);

  useEffect(() => {
    api
      .site()
      .then(async (site) =>
        setLogo(site.brand.logo ? await asDataUrl(site.brand.logo) : null),
      )
      .catch(() => {});
  }, []);

  const colours = {
    surface: themeColour("surface", "#fffaf3"),
    text: themeColour("text", "#1f1b16"),
    muted: themeColour("text-muted", "#5c544b"),
    primary: themeColour("primary", "#b3261e"),
    accent: themeColour("accent", "#2e7d32"),
  };
  // Only the year: progress stats live in localStorage and can be reset,
  // so they would prove nothing on a certificate.
  const year = event.start.slice(0, 4);
  const heading = event.headline ?? event.title;
  const edition = heading.includes(year) ? heading : `${heading} · ${year}`;
  const title = wrap(payload.title, 30)
    .slice(0, 2)
    .map((line, i, all) => ({ line, y: 280 - (all.length - 1) * 30 + i * 62 }));
  const message = wrap(payload.message, 60)
    .slice(0, 6)
    .map((line, i) => ({ line, y: 440 + i * 40 }));

  async function exportImage() {
    if (svgRef.current) {
      download(await svgToPng(svgRef.current), `${event.id}-certificate.png`);
    }
  }

  return (
    <div className="flex flex-col gap-3">
      <svg
        ref={svgRef}
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 1200 850"
        role="img"
        aria-label={`Certificate for ${name}: ${payload.title}`}
        // Web fonts do not load inside an exported SVG image; the stack
        // falls back to a system serif there.
        fontFamily="'Fraunces Variable', Georgia, serif"
        className="h-auto w-full rounded-md border"
      >
        <rect width="1200" height="850" fill={colours.surface} />
        <rect
          x="30"
          y="30"
          width="1140"
          height="790"
          fill="none"
          stroke={colours.primary}
          strokeWidth="6"
          rx="24"
        />
        {logo && <image href={logo} x="560" y="70" width="80" height="80" />}
        <text
          x="600"
          y="190"
          textAnchor="middle"
          fontSize="28"
          fill={colours.muted}
        >
          {brandName}
        </text>
        {title.map(({ line, y }) => (
          <text
            key={y}
            x="600"
            y={y}
            textAnchor="middle"
            fontSize="56"
            fontWeight="700"
            fill={colours.primary}
          >
            {line}
          </text>
        ))}
        <text
          x="600"
          y="360"
          textAnchor="middle"
          fontSize="44"
          fill={colours.text}
        >
          {name}
        </text>
        {message.map(({ line, y }) => (
          <text
            key={y}
            x="600"
            y={y}
            textAnchor="middle"
            fontSize="28"
            fill={colours.text}
          >
            {line}
          </text>
        ))}
        {payload.image && (
          <image
            href={`data:image/webp;base64,${payload.image}`}
            x="500"
            y="640"
            width="200"
            height="120"
            preserveAspectRatio="xMidYMid meet"
          />
        )}
        <text
          x="600"
          y="790"
          textAnchor="middle"
          fontSize="24"
          fill={colours.accent}
        >
          {edition}
        </text>
      </svg>
      <Button type="button" onClick={exportImage} className="self-start">
        Download as PNG
      </Button>
    </div>
  );
}
