import { useEffect, useState } from "react";

import { api, type EventInfo, NotFound, themeCssUrl } from "@/api";
import { CertificateSection } from "@/certificate/Certificate";
import { Countdown } from "@/components/Countdown";
import { DoorGrid, type DoorState } from "@/components/DoorGrid";
import { KeyStrip } from "@/components/KeyStrip";
import { DoorPanel } from "@/door/DoorPanel";
import { Progress } from "@/door/Progress";
import { useEventProgress } from "@/store";
import { useThemeCss } from "@/useThemeCss";
import { Message } from "./Landing";

/** `/<event>/`: the calendar itself. */
export function EventPage({ id }: { id: string }) {
  useThemeCss(themeCssUrl(id));
  const [event, setEvent] = useState<EventInfo | null>(null);
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
  }, [id]);

  if (error) return <Message text={error} />;
  if (!event) return <Message text="Loading…" />;

  const released = new Set(event.released);
  const stateOf = (day: number): DoorState =>
    progress.words[day] ? "solved" : released.has(day) ? "open" : "locked";
  const opensAt = event.opens_at ? new Date(event.opens_at) : null;

  return (
    <main className="mx-auto flex min-h-svh max-w-3xl flex-col gap-6 p-4 sm:p-6">
      <header className="flex flex-col gap-2 text-center">
        <a href="/" className="text-sm text-muted-foreground hover:underline">
          ← All calendars
        </a>
        <h1 className="text-3xl font-semibold">{event.title}</h1>
        {event.intro && <p className="text-muted-foreground">{event.intro}</p>}
      </header>
      {released.size === 0 && opensAt && opensAt.getTime() > Date.now() && (
        <Countdown target={opensAt} />
      )}
      <KeyStrip days={event.days} words={progress.words} />
      <DoorGrid days={event.days} stateOf={stateOf} onOpen={setOpenDay} />
      <CertificateSection event={event} />
      <Progress event={id} />
      {openDay !== null && (
        <DoorPanel event={id} day={openDay} onClose={() => setOpenDay(null)} />
      )}
    </main>
  );
}
