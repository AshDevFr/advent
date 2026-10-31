import {
  type FormEvent,
  type ReactNode,
  useEffect,
  useId,
  useState,
} from "react";

import { api, type SiteIndex, themeCssUrl } from "@/api";
import { BrowserTab } from "@/BrowserTab";
import { BrandLogo } from "@/components/BrandLogo";
import { ThemeToggle } from "@/components/ThemeToggle";
import { Button } from "@/components/ui/button";
import { useThemeCss } from "@/useThemeCss";

/** A centred card in the brand's colours, for the sign-in pages. */
export function SignInFrame({ children }: { children: ReactNode }) {
  useThemeCss(themeCssUrl());
  const [brand, setBrand] = useState<SiteIndex["brand"] | null>(null);
  useEffect(() => {
    api
      .site()
      .then((site) => setBrand(site.brand))
      .catch(() => {});
  }, []);
  return (
    <main className="mx-auto flex min-h-svh max-w-md flex-col justify-center gap-6 px-4 py-10">
      <BrowserTab title={brand?.name ?? ""} icon={brand?.logo} />
      <header className="flex items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          {brand?.logo && (
            <BrandLogo
              logo={brand.logo}
              dark={brand.logo_dark}
              className="size-10"
            />
          )}
          <h1 className="font-display text-3xl font-semibold tracking-tight">
            {brand?.name ?? "Advent calendar"}
          </h1>
        </div>
        <ThemeToggle />
      </header>
      <section className="flex flex-col gap-4 rounded-lg border bg-card p-5">
        {children}
      </section>
    </main>
  );
}

/** Ask for one's personal link again. Invited players only get one. */
export function LinkRequestForm() {
  const [email, setEmail] = useState("");
  const [sending, setSending] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const inputId = useId();

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!email.trim() || sending) return;
    setSending(true);
    try {
      const response = await fetch("api/signin/request", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ email: email.trim() }),
      });
      const body = (await response.json().catch(() => ({}))) as {
        message?: string;
        error?: string;
      };
      setMessage(
        response.ok
          ? (body.message ??
              "If this address is invited, a sign-in link is on its way.")
          : (body.error ?? "Something went wrong. Please try again later."),
      );
    } catch {
      setMessage("Something went wrong. Please try again later.");
    } finally {
      setSending(false);
    }
  }

  return (
    <form onSubmit={submit} className="flex flex-col gap-2">
      <label htmlFor={inputId} className="font-medium">
        Your email address
      </label>
      <div className="flex gap-2">
        <input
          id={inputId}
          type="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          autoComplete="email"
          className="h-9 min-w-0 flex-1 rounded-md border bg-card px-3 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        />
        <Button type="submit" disabled={sending}>
          Send my link
        </Button>
      </div>
      {message && (
        <p role="status" className="text-sm text-muted-foreground">
          {message}
        </p>
      )}
    </form>
  );
}

/** Shown to visitors who are not signed in. */
export function SignIn() {
  return (
    <SignInFrame>
      <p>
        This calendar is for invited players. Open the personal link from your
        invitation email, or enter your address to receive it again.
      </p>
      <LinkRequestForm />
    </SignInFrame>
  );
}
