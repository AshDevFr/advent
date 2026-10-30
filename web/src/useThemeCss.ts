import { useEffect } from "react";

/** Load a generated theme stylesheet (brand and event colours). */
export function useThemeCss(href: string) {
  useEffect(() => {
    const link = document.createElement("link");
    link.rel = "stylesheet";
    link.href = href;
    link.dataset.theme = "";
    document.head.appendChild(link);
    return () => link.remove();
  }, [href]);
}
