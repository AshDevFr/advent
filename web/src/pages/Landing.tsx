import { useEffect, useState } from "react";

import { api, type SiteIndex, themeCssUrl } from "@/api";
import { BrowserTab } from "@/BrowserTab";
import { Countdown } from "@/components/Countdown";
import { ThemeToggle } from "@/components/ThemeToggle";
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
    <main className="mx-auto flex min-h-svh max-w-3xl flex-col gap-10 px-4 py-6 sm:px-8 sm:py-10">
      <BrowserTab title={site.brand.name} icon={site.brand.logo} />
      <header className="flex items-start justify-between gap-4">
        <div className="flex items-center gap-4">
          {site.brand.logo && (
            <img src={site.brand.logo} alt="" className="size-12" />
          )}
          <h1 className="font-display text-4xl font-semibold tracking-tight">
            {site.brand.name}
          </h1>
        </div>
        <ThemeToggle />
      </header>
      {events.length === 0 && <p>Nothing yet.</p>}
      {events.map((event) => {
        const opensAt = event.opens_at ? new Date(event.opens_at) : null;
        const upcoming = opensAt !== null && opensAt.getTime() > Date.now();
        return (
          <section
            key={event.id}
            // theme.css carries each themed event's palette under this
            // attribute, so the card shows the event's own colours.
            data-event={event.id}
            aria-labelledby={`event-${event.id}`}
            className="relative isolate flex flex-col gap-4 overflow-hidden rounded-lg border bg-card p-6 text-card-foreground"
          >
            {event.background && (
              <div aria-hidden="true" className="absolute inset-0 -z-10">
                <div
                  data-testid="card-backdrop"
                  className="absolute inset-0 bg-cover bg-center"
                  style={{ backgroundImage: `url("${event.background}")` }}
                />
                <div className="absolute inset-0 bg-card/80" />
              </div>
            )}
            {event.headline && (
              <p className="text-xs font-semibold tracking-widest text-muted-foreground uppercase">
                {event.title}
              </p>
            )}
            <h2
              id={`event-${event.id}`}
              className="font-display text-3xl font-semibold tracking-tight"
            >
              {event.headline ?? event.title}
            </h2>
            {event.intro && (
              <p className="text-muted-foreground">{event.intro}</p>
            )}
            {upcoming && opensAt ? (
              <Countdown target={opensAt} />
            ) : (
              <a
                href={`${event.id}/`}
                className={buttonVariants({ className: "self-start" })}
              >
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
