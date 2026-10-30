import { useEffect, useState } from "react";

import { api, type SiteIndex, themeCssUrl } from "@/api";
import { Countdown } from "@/components/Countdown";
import { buttonVariants } from "@/components/ui/button";
import { useThemeCss } from "@/useThemeCss";

/** `/`: who runs the calendar, its events, and a countdown to the next. */
export function Landing() {
  useThemeCss(themeCssUrl());
  const [site, setSite] = useState<SiteIndex | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    api
      .site()
      .then(setSite)
      .catch(() => setFailed(true));
  }, []);

  if (failed) return <Message text="The calendar could not be loaded." />;
  if (!site) return <Message text="Loading…" />;

  const events = [...site.events].reverse();
  return (
    <main className="mx-auto flex min-h-svh max-w-2xl flex-col gap-8 p-6">
      <header className="flex flex-col items-center gap-3 text-center">
        {site.brand.logo && (
          <img src={site.brand.logo} alt="" className="size-16" />
        )}
        <h1 className="text-3xl font-semibold">{site.brand.name}</h1>
      </header>
      {events.length === 0 && <p className="text-center">Nothing yet.</p>}
      {events.map((event) => {
        const opensAt = event.opens_at ? new Date(event.opens_at) : null;
        const upcoming = opensAt !== null && opensAt.getTime() > Date.now();
        return (
          <section
            key={event.id}
            aria-labelledby={`event-${event.id}`}
            className="flex flex-col gap-4 rounded-lg border bg-card p-6 text-center"
          >
            <h2 id={`event-${event.id}`} className="text-2xl font-semibold">
              {event.title}
            </h2>
            {event.intro && (
              <p className="text-muted-foreground">{event.intro}</p>
            )}
            {upcoming && opensAt ? (
              <Countdown target={opensAt} />
            ) : (
              <a href={`/${event.id}/`} className={buttonVariants()}>
                Open the calendar
              </a>
            )}
          </section>
        );
      })}
    </main>
  );
}

export function Message({ text }: { text: string }) {
  return (
    <main className="flex min-h-svh items-center justify-center p-6">
      <p role="status">{text}</p>
    </main>
  );
}
