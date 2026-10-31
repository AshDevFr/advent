import { type FormEvent, useEffect, useId, useRef, useState } from "react";

import { api, type EventInfo } from "@/api";
import { Button } from "@/components/ui/button";
import {
  InsecureContextError,
  openFinal,
  type Certificate as Payload,
} from "@/crypto";
import { total, useEventProgress, useProgress } from "@/store";
import { download, svgToPng } from "./exportPng";

/** Shown once every word is collected: unlock and render the certificate. */
export function CertificateSection({ event }: { event: EventInfo }) {
  const progress = useEventProgress(event.id);
  const days = Array.from({ length: event.days }, (_, i) => i + 1);
  const words = days.map((d) => progress.words[d]);
  if (words.some((w) => !w)) return null;
  return <Unlock event={event} words={words as string[]} />;
}

function Unlock({ event, words }: { event: EventInfo; words: string[] }) {
  const progress = useEventProgress(event.id);
  const [name, setName] = useState(progress.name);
  const [payload, setPayload] = useState<Payload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const inputId = useId();

  async function unlock(e: FormEvent) {
    e.preventDefault();
    if (!name.trim()) return;
    useProgress.getState().setName(event.id, name.trim());
    try {
      const opened = await openFinal(words, await api.final(event.id));
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
      aria-labelledby="certificate-heading"
      className="flex flex-col gap-4 rounded-lg border bg-card p-4 sm:p-6"
    >
      <h2 id="certificate-heading" className="text-2xl font-semibold">
        You found every word
      </h2>
      {!payload && (
        <form onSubmit={unlock} className="flex flex-col gap-2">
          <label htmlFor={inputId}>Name for the certificate</label>
          <div className="flex gap-2">
            <input
              id={inputId}
              value={name}
              onChange={(e) => setName(e.target.value)}
              autoComplete="name"
              className="h-9 min-w-0 flex-1 rounded-md border bg-background px-3 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            />
            <Button type="submit">Unlock</Button>
          </div>
          {error && (
            <p role="alert" className="text-sm">
              {error}
            </p>
          )}
        </form>
      )}
      {payload && (
        <CertificateView event={event} payload={payload} name={name.trim()} />
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
  payload,
  name,
  date = new Date(),
}: {
  event: EventInfo;
  payload: Payload;
  name: string;
  date?: Date;
}) {
  const progress = useEventProgress(event.id);
  const svgRef = useRef<SVGSVGElement>(null);
  const [brand, setBrand] = useState<{ name: string; logo: string | null }>({
    name: "",
    logo: null,
  });

  useEffect(() => {
    api
      .site()
      .then(async (site) =>
        setBrand({
          name: site.brand.name,
          logo: site.brand.logo ? await asDataUrl(site.brand.logo) : null,
        }),
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
  const hints = total(progress.hintsUsed);
  const attempts = total(progress.attempts);
  const when = date.toLocaleDateString(undefined, {
    year: "numeric",
    month: "long",
    day: "numeric",
  });
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
        {brand.logo && (
          <image href={brand.logo} x="560" y="70" width="80" height="80" />
        )}
        <text
          x="600"
          y="190"
          textAnchor="middle"
          fontSize="28"
          fill={colours.muted}
        >
          {brand.name} · {event.title}
        </text>
        <text
          x="600"
          y="280"
          textAnchor="middle"
          fontSize="56"
          fontWeight="700"
          fill={colours.primary}
        >
          {payload.title}
        </text>
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
          Completed {when} · {hints} hint{hints === 1 ? "" : "s"} used ·{" "}
          {attempts} attempt{attempts === 1 ? "" : "s"}
        </text>
      </svg>
      <Button type="button" onClick={exportImage} className="self-start">
        Download as PNG
      </Button>
    </div>
  );
}
