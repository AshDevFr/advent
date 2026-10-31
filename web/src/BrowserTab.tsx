import { useEffect } from "react";

/** The browser tab's title and icon: the brand's, not a generic name. */
export function BrowserTab({
  title,
  icon,
}: {
  title: string;
  icon?: string | null;
}) {
  useEffect(() => {
    if (title) document.title = title;
  }, [title]);
  useEffect(() => {
    if (!icon) return;
    let link = document.querySelector<HTMLLinkElement>('link[rel="icon"]');
    if (!link) {
      link = document.createElement("link");
      link.rel = "icon";
      document.head.appendChild(link);
    }
    link.href = icon;
  }, [icon]);
  return null;
}
