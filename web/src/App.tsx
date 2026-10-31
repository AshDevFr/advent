import { useEffect } from "react";

import { basePath, routeEventId } from "@/api";
import { hostingMode } from "@/client";
import { useSession } from "@/client/session";
import { EventPage } from "@/pages/Event";
import { Landing, Message } from "@/pages/Landing";
import { SignIn } from "@/pages/SignIn";
import { SignInLink } from "@/pages/SignInLink";

/** The base path is the landing page; `<base>/<event>/` is that event. */
export function App({ path, base }: { path?: string; base?: string }) {
  if (hostingMode() === "server") return <ServerApp path={path} base={base} />;
  return <Calendar path={path ?? window.location.pathname} base={base} />;
}

function Calendar({ path, base }: { path: string; base?: string }) {
  const id = routeEventId(path, base);
  return id ? <EventPage id={id} /> : <Landing />;
}

/** The link token in `<base>/signin/<token>`, if that is the path. */
export function signInLink(path: string, base = basePath()): string | null {
  const relative = path.startsWith(base) ? path.slice(base.length) : path;
  const [first, token] = relative.split("/").filter(Boolean);
  return first === "signin" && token ? decodeURIComponent(token) : null;
}

/**
 * A server-hosted calendar is for invited players: their sign-in link page,
 * the sign-in form, or the calendar once signed in.
 */
function ServerApp({ path, base }: { path?: string; base?: string }) {
  const session = useSession((s) => s.session);
  const current = path ?? window.location.pathname;
  const link = session.status === "in" ? null : signInLink(current, base);

  useEffect(() => {
    if (session.status !== "checking" || link) return;
    fetch("api/session", { credentials: "same-origin", cache: "no-cache" })
      .then(async (response) => {
        if (!response.ok) throw new Error(String(response.status));
        const me = (await response.json()) as {
          email: string;
          name: string | null;
        };
        useSession.getState().set({ status: "in", ...me });
      })
      .catch(() => useSession.getState().set({ status: "out" }));
  }, [session.status, link]);

  if (link) return <SignInLink link={link} />;
  if (session.status === "checking") return <Message text="Loading…" />;
  if (session.status === "out") return <SignIn />;
  // Just signed in from a link page: the calendar is at the base path.
  const calendar = signInLink(current, base) ? (base ?? basePath()) : current;
  return <Calendar path={calendar} base={base} />;
}
