// @ts-check
import starlight from "@astrojs/starlight";
import starlightLinksValidator from "starlight-links-validator";
import { defineConfig } from "astro/config";

// The docs are the root of advent-demo.4sh.dev; the demo and the showcases
// are deployed beside them under /demo/, /northwind/ and /christmas/.
export default defineConfig({
  site: "https://advent-demo.4sh.dev",
  integrations: [
    starlight({
      title: "advent",
      description:
        "An engine for puzzle advent calendars: seal puzzles, publish a static site, one door a day.",
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/AshDevFr/advent",
        },
      ],
      customCss: ["./src/styles/theme.css"],
      // Fail the build on a broken internal link. /demo/ is deployed beside
      // the docs, not part of them.
      plugins: [starlightLinksValidator({ exclude: ["/demo/", "/demo/**"] })],
      sidebar: [
        { label: "Start here", items: [{ autogenerate: { directory: "start" } }] },
        { label: "Guides", items: [{ autogenerate: { directory: "guides" } }] },
        { label: "Reference", items: [{ autogenerate: { directory: "reference" } }] },
        { label: "Live demo", link: "/demo/", attrs: { target: "_self" } },
      ],
    }),
  ],
});
