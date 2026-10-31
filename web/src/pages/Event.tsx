import { useEffect, useState } from "react";

import {
  api,
  type EventInfo,
  NotFound,
  type SiteIndex,
  themeCssUrl,
} from "@/api";
import { CertificatePanel } from "@/certificate/Certificate";
import { Countdown } from "@/components/Countdown";
import { type Door, DoorGrid } from "@/components/DoorGrid";
import { KeyStrip } from "@/components/KeyStrip";
import { ThemeToggle } from "@/components/ThemeToggle";
import { DoorPanel } from "@/door/DoorPanel";
import { Progress } from "@/door/Progress";
import { useEventProgress } from "@/store";
import { useThemeCss } from "@/useThemeCss";
import { Message } from "./Landing";

/** "Dec 7" for day `day` of an event starting on `start` (YYYY-MM-DD). */
export function openingDate(start: string, day: number): string {
  const date = new Date(`${start}T12:00:00Z`);
  date.setUTCDate(date.getUTCDate() + day - 1);
  return date.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    timeZone: "UTC",
  });
}

/** `/<event>/`: the calendar itself. */
export function EventPage({ id }: { id: string }) {
  useThemeCss(themeCssUrl(id));
  const [event, setEvent] = useState<EventInfo | null>(null);
  const [brand, setBrand] = useState<SiteIndex["brand"] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [openDay, setOpenDay] = useState<number | null>(null);
  const progress = useEventProgress(id);

  useEffect(() => {
    api
      .event(id)
      .then(setEvent)
      .catch((e) =>
        setError(
          e instanceof NotFound
            ? "There is no calendar here."
            : "The calendar could not be loaded.",
        ),
      );
    api
      .site()
      .then((site) => setBrand(site.brand))
      .catch(() => {});
  }, [id]);

  if (error) return <Message text={error} />;
  if (!event) return <Message text="Loading…" />;

  const released = new Set(event.released);
  const doors: Door[] = Array.from({ length: event.days }, (_, i) => {
    const day = i + 1;
    if (!released.has(day)) {
      return { day, state: "locked", opens: openingDate(event.start, day) };
    }
    const word = progress.words[day];
    return {
      day,
      state: word ? "solved" : "open",
      type: event.types[day],
      word,
    };
  });
  const opensAt = event.opens_at ? new Date(event.opens_at) : null;
  const eyebrow = [brand?.name, event.headline ? event.title : null]
    .filter(Boolean)
    .join(" · ");

  return (
    <main className="mx-auto flex min-h-svh max-w-6xl flex-col gap-8 px-4 py-6 sm:px-8 sm:py-10">
      <header className="flex flex-col gap-3">
        <div className="flex items-center justify-between gap-4">
          <a
            href="/"
            className="text-xs font-semibold tracking-widest text-muted-foreground uppercase hover:text-foreground"
          >
            {eyebrow || "All calendars"}
          </a>
          <ThemeToggle />
        </div>
        <h1 className="font-display text-4xl font-semibold tracking-tight sm:text-5xl">
          {event.headline ?? event.title}
        </h1>
        {event.intro && (
          <p className="max-w-2xl text-muted-foreground">{event.intro}</p>
        )}
      </header>
      {released.size === 0 && opensAt && opensAt.getTime() > Date.now() && (
        <Countdown target={opensAt} />
      )}
      <KeyStrip days={event.days} words={progress.words} />
      <DoorGrid doors={doors} onOpen={setOpenDay} />
      <CertificatePanel event={event} brandName={brand?.name ?? ""} />
      <Progress event={id} />
      {openDay !== null && (
        <DoorPanel
          event={id}
          day={openDay}
          type={event.types[openDay]}
          onClose={() => setOpenDay(null)}
        />
      )}
    </main>
  );
}
