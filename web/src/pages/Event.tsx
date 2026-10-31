import { useEffect, useState } from "react";
import {
  api,
  type EventInfo,
  NotFound,
  type SiteIndex,
  themeCssUrl,
} from "@/api";
import { BrowserTab } from "@/BrowserTab";
import { CertificatePanel } from "@/certificate/Certificate";
import { client, useEventProgress } from "@/client";
import { BrandLogo } from "@/components/BrandLogo";
import { Countdown } from "@/components/Countdown";
import { type Door, DoorGrid } from "@/components/DoorGrid";
import { KeyStrip } from "@/components/KeyStrip";
import { ThemeToggle } from "@/components/ThemeToggle";
import { Button } from "@/components/ui/button";
import { DoorPanel } from "@/door/DoorPanel";
import { Progress } from "@/door/Progress";
import { cn } from "@/lib/utils";
import { Notices } from "@/notices/Notices";
import { useThemeCss } from "@/useThemeCss";
import { Message } from "./Landing";

/** "Dec 7" for a date given as YYYY-MM-DD (the event's own calendar date). */
export function shortDate(isoDate: string): string {
  const date = new Date(`${isoDate}T12:00:00Z`);
  return date.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    timeZone: "UTC",
  });
}

/**
 * The theme's background image, fixed behind the page under a wash of the
 * page colour so text stays readable in light and dark mode.
 */
function Backdrop({ image, dark }: { image: string; dark?: string | null }) {
  return (
    <div aria-hidden="true" className="pointer-events-none fixed inset-0 -z-10">
      <div
        data-testid="backdrop"
        className={cn(
          "absolute inset-0 bg-cover bg-center",
          dark && "when-light",
        )}
        style={{ backgroundImage: `url("${image}")` }}
      />
      {dark && (
        <div
          data-testid="backdrop"
          className="when-dark absolute inset-0 bg-cover bg-center"
          style={{ backgroundImage: `url("${dark}")` }}
        />
      )}
      <div className="absolute inset-0 bg-background/78" />
    </div>
  );
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
    // A server keeps progress itself; bring it in (the static site has it
    // in this browser already).
    client()
      .load?.(id)
      .catch(() => {});
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

  // The dialog is not opened by a Radix trigger, so Radix cannot restore
  // focus; put keyboard users back on the door they opened.
  function closeDoor(day: number) {
    setOpenDay(null);
    requestAnimationFrame(() =>
      document.querySelector<HTMLElement>(`button[data-day="${day}"]`)?.focus(),
    );
  }

  if (error) return <Message text={error} />;
  if (!event) return <Message text="Loading…" />;

  const released = new Set(event.released);
  const doors: Door[] = Array.from({ length: event.days }, (_, i) => {
    const day = i + 1;
    if (!released.has(day)) {
      const date = event.dates[day - 1];
      return {
        day,
        state: "locked",
        opens: date ? shortDate(date) : undefined,
      };
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
  const tabTitle = [event.headline ?? event.title, brand?.name]
    .filter(Boolean)
    .join(" · ");
  const eyebrow = [brand?.name, event.headline ? event.title : null]
    .filter(Boolean)
    .join(" · ");

  return (
    <main className="mx-auto flex min-h-svh max-w-6xl flex-col gap-8 px-4 py-6 sm:px-8 sm:py-10">
      <BrowserTab title={tabTitle} icon={brand?.logo} />
      {event.background && (
        <Backdrop image={event.background} dark={event.background_dark} />
      )}
      <header className="flex flex-col gap-3">
        <div className="flex items-center justify-between gap-4">
          <a
            href="./"
            className="flex items-center gap-3 text-xs font-semibold tracking-widest text-muted-foreground uppercase hover:text-foreground"
          >
            {brand?.logo && (
              <BrandLogo
                logo={brand.logo}
                dark={brand.logo_dark}
                className="size-8 shrink-0"
              />
            )}
            {eyebrow || "All calendars"}
          </a>
          <div className="flex items-center gap-2">
            <SignOut />
            <ThemeToggle />
          </div>
        </div>
        <h1 className="font-display text-4xl font-semibold tracking-tight sm:text-5xl">
          {event.headline ?? event.title}
        </h1>
        {event.intro && (
          <p className="max-w-2xl text-muted-foreground">{event.intro}</p>
        )}
      </header>
      <Notices event={id} notices={event.notices} />
      {released.size === 0 && opensAt && opensAt.getTime() > Date.now() && (
        <Countdown target={opensAt} />
      )}
      <KeyStrip days={event.days} words={progress.words} />
      <DoorGrid doors={doors} onOpen={setOpenDay} artwork={event.doors} />
      <CertificatePanel event={event} brandName={brand?.name ?? ""} />
      <Progress event={id} />
      {openDay !== null && (
        <DoorPanel
          event={id}
          day={openDay}
          type={event.types[openDay]}
          onClose={() => closeDoor(openDay)}
        />
      )}
    </main>
  );
}

/** On a server-hosted calendar: end this browser's session. */
function SignOut() {
  const signOut = client().signOut;
  if (!signOut) return null;
  return (
    <Button type="button" variant="ghost" size="sm" onClick={() => signOut()}>
      Sign out
    </Button>
  );
}
