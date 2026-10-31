import { routeEventId } from "@/api";
import { EventPage } from "@/pages/Event";
import { Landing } from "@/pages/Landing";

/** The base path is the landing page; `<base>/<event>/` is that event. */
export function App({
  path = window.location.pathname,
  base,
}: {
  path?: string;
  base?: string;
}) {
  const id = routeEventId(path, base);
  return id ? <EventPage id={id} /> : <Landing />;
}
