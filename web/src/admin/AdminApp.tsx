import { type FormEvent, useCallback, useEffect, useId, useState } from "react";

import { basePath, themeCssUrl } from "@/api";
import { hostingMode } from "@/client";
import { ThemeToggle } from "@/components/ThemeToggle";
import { Button } from "@/components/ui/button";
import { Message } from "@/pages/Landing";
import { useThemeCss } from "@/useThemeCss";
import { type AdminEvent, admin, Unauthorized } from "./api";
import { ImportPage } from "./Import";
import { Overview } from "./Overview";
import { PlayerPage } from "./Player";
import { PlayersPage } from "./Players";

/** Where the admin section is, relative to the base path. */
export interface Route {
  page: "overview" | "players" | "player" | "import";
  email?: string;
}

export function parseRoute(path: string, base = basePath()): Route {
  const relative = path.startsWith(base) ? path.slice(base.length) : path;
  const [, page, email] = relative.split("/").filter(Boolean);
  if (page === "players" && email)
    return { page: "player", email: decodeURIComponent(email) };
  if (page === "players") return { page: "players" };
  if (page === "import") return { page: "import" };
  return { page: "overview" };
}

export function routePath(route: Route): string {
  switch (route.page) {
    case "players":
      return "admin/players";
    case "player":
      return `admin/players/${encodeURIComponent(route.email ?? "")}`;
    case "import":
      return "admin/import";
    default:
      return "admin";
  }
}

export type Navigate = (route: Route) => void;

/** `/admin`: the organisers' section of a server-hosted calendar. */
export default function AdminApp({
  path,
  base,
}: {
  path: string;
  base?: string;
}) {
  useThemeCss(themeCssUrl());
  const [route, setRoute] = useState(() => parseRoute(path, base));
  const [state, setState] = useState<"checking" | "out" | "in">("checking");
  const [events, setEvents] = useState<AdminEvent[]>([]);
  const [event, setEvent] = useState<string | null>(null);

  const navigate: Navigate = useCallback(
    (next) => {
      window.history.pushState(
        null,
        "",
        `${base ?? basePath()}${routePath(next)}`,
      );
      setRoute(next);
    },
    [base],
  );

  useEffect(() => {
    const back = () => setRoute(parseRoute(window.location.pathname, base));
    window.addEventListener("popstate", back);
    return () => window.removeEventListener("popstate", back);
  }, [base]);

  useEffect(() => {
    if (state !== "checking") return;
    admin
      .session()
      .then(() => setState("in"))
      .catch(() => setState("out"));
  }, [state]);

  useEffect(() => {
    if (state !== "in") return;
    admin
      .events()
      .then((list) => {
        setEvents(list);
        setEvent((current) => current ?? list.at(-1)?.id ?? null);
      })
      .catch((e) => e instanceof Unauthorized && setState("out"));
  }, [state]);

  if (hostingMode() !== "server")
    return (
      <Message text="The admin section exists only when the calendar is hosted by advent serve." />
    );
  if (state === "checking") return <Message text="Loading…" />;
  if (state === "out") return <Login onSignedIn={() => setState("in")} />;

  const signedOut = () => setState("out");
  const current = events.find((e) => e.id === event) ?? null;
  const tabs: { label: string; route: Route }[] = [
    { label: "Overview", route: { page: "overview" } },
    { label: "Players", route: { page: "players" } },
    { label: "Invite", route: { page: "import" } },
  ];

  return (
    <main className="mx-auto flex min-h-svh max-w-6xl flex-col gap-6 px-4 py-6 sm:px-8">
      <header className="flex flex-wrap items-center justify-between gap-4">
        <div className="flex flex-wrap items-center gap-4">
          <h1 className="font-display text-2xl font-semibold">Admin</h1>
          <nav aria-label="Admin" className="flex gap-1">
            {tabs.map((tab) => (
              <Button
                key={tab.label}
                type="button"
                size="sm"
                variant={
                  route.page === tab.route.page ||
                  (route.page === "player" && tab.route.page === "players")
                    ? "default"
                    : "ghost"
                }
                onClick={() => navigate(tab.route)}
              >
                {tab.label}
              </Button>
            ))}
          </nav>
        </div>
        <div className="flex items-center gap-2">
          {events.length > 1 && (
            <label className="flex items-center gap-2 text-sm">
              Calendar
              <select
                value={event ?? ""}
                onChange={(e) => setEvent(e.target.value)}
                className="h-8 rounded-md border bg-card px-2"
              >
                {events.map((e) => (
                  <option key={e.id} value={e.id}>
                    {e.title}
                  </option>
                ))}
              </select>
            </label>
          )}
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => admin.logout().finally(signedOut)}
          >
            Sign out
          </Button>
          <ThemeToggle />
        </div>
      </header>
      {!current && route.page !== "import" ? (
        <p role="status">No calendar to show.</p>
      ) : route.page === "players" && current ? (
        <PlayersPage
          event={current}
          navigate={navigate}
          onSignedOut={signedOut}
        />
      ) : route.page === "player" && current && route.email ? (
        <PlayerPage
          event={current}
          email={route.email}
          navigate={navigate}
          onSignedOut={signedOut}
        />
      ) : route.page === "import" ? (
        <ImportPage navigate={navigate} onSignedOut={signedOut} />
      ) : current ? (
        <Overview event={current} navigate={navigate} onSignedOut={signedOut} />
      ) : null}
    </main>
  );
}

function Login({ onSignedIn }: { onSignedIn: () => void }) {
  const [password, setPassword] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const inputId = useId();

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!password || busy) return;
    setBusy(true);
    setProblem(null);
    try {
      await admin.login(password);
      onSignedIn();
    } catch (e) {
      setProblem(
        e instanceof Unauthorized
          ? "Wrong password."
          : e instanceof Error
            ? e.message
            : "Signing in failed.",
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="mx-auto flex min-h-svh max-w-sm flex-col justify-center gap-4 px-4">
      <h1 className="font-display text-3xl font-semibold">Admin</h1>
      <form
        onSubmit={submit}
        className="flex flex-col gap-2 rounded-lg border bg-card p-5"
      >
        <label htmlFor={inputId} className="font-medium">
          Admin password
        </label>
        <input
          id={inputId}
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          autoComplete="current-password"
          className="h-9 rounded-md border bg-card px-3 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        />
        <Button type="submit" disabled={busy}>
          Sign in
        </Button>
        {problem && (
          <p role="alert" className="text-sm">
            {problem}
          </p>
        )}
      </form>
    </main>
  );
}

/** Run an admin request; an expired session goes back to the login. */
export function useAdminData<T>(
  load: () => Promise<T>,
  onSignedOut: () => void,
  deps: unknown[],
): { data: T | null; error: string | null; reload: () => void } {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tick, setTick] = useState(0);
  // biome-ignore lint/correctness/useExhaustiveDependencies: the caller lists what the load depends on
  useEffect(() => {
    let live = true;
    load()
      .then((value) => {
        if (!live) return;
        setData(value);
        setError(null);
      })
      .catch((e) => {
        if (!live) return;
        if (e instanceof Unauthorized) onSignedOut();
        else setError(e instanceof Error ? e.message : String(e));
      });
    return () => {
      live = false;
    };
  }, [...deps, tick]);
  return { data, error, reload: () => setTick((t) => t + 1) };
}

export function formatTime(iso: string | null): string {
  if (!iso) return "";
  return new Date(iso).toLocaleString(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  });
}

export function ProgressBar({ value, max }: { value: number; max: number }) {
  const percent = max ? Math.round((value / max) * 100) : 0;
  return (
    <div className="flex items-center gap-2">
      <div
        role="progressbar"
        aria-valuenow={value}
        aria-valuemin={0}
        aria-valuemax={max}
        aria-label={`${value} of ${max} days`}
        className="h-2 w-24 overflow-hidden rounded-full bg-border"
      >
        <div className="h-full bg-primary" style={{ width: `${percent}%` }} />
      </div>
      <span className="font-mono text-xs">
        {value}/{max}
      </span>
    </div>
  );
}
