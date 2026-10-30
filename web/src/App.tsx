import { EventPage } from "@/pages/Event";
import { Landing } from "@/pages/Landing";

/** `/` is the landing page; `/<event>/` is that event's calendar. */
export function App({ path = window.location.pathname }: { path?: string }) {
  const id = path.split("/").filter(Boolean)[0];
  return id ? <EventPage id={decodeURIComponent(id)} /> : <Landing />;
}
