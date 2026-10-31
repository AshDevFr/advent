import { useEffect, useState } from "react";

import { basePath } from "@/api";
import { useSession } from "@/client/session";
import { Button } from "@/components/ui/button";
import { LinkRequestForm, SignInFrame } from "./SignIn";

/**
 * `/signin/<link>`: the page an invitation opens. It only signs in on the
 * button, never on the visit itself: company mail filters open every link
 * in an email to scan it, and that must not sign anyone in.
 */
export function SignInLink({ link }: { link: string }) {
  const [email, setEmail] = useState<string | null>(null);
  const [invalid, setInvalid] = useState(false);
  const [signingIn, setSigningIn] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    fetch(`api/signin/${encodeURIComponent(link)}`, { cache: "no-cache" })
      .then(async (response) => {
        if (!response.ok) throw new Error(String(response.status));
        const me = (await response.json()) as { email: string };
        setEmail(me.email);
      })
      .catch(() => setInvalid(true));
  }, [link]);

  async function signIn() {
    setSigningIn(true);
    setProblem(null);
    try {
      const response = await fetch("api/signin", {
        method: "POST",
        headers: { "content-type": "application/json" },
        credentials: "same-origin",
        body: JSON.stringify({ link }),
      });
      if (!response.ok) throw new Error(String(response.status));
      const me = (await response.json()) as {
        email: string;
        name: string | null;
      };
      // Leave the link out of the address bar and history.
      window.history.replaceState(null, "", basePath());
      useSession.getState().set({ status: "in", ...me });
    } catch {
      setProblem("Signing in did not work. Please try again.");
      setSigningIn(false);
    }
  }

  return (
    <SignInFrame>
      {invalid ? (
        <>
          <p role="alert">
            This sign-in link is not valid any more. Enter your address to
            receive your current one.
          </p>
          <LinkRequestForm />
        </>
      ) : email === null ? (
        <p role="status">Checking your link…</p>
      ) : (
        <>
          <p>Your personal link to the calendar.</p>
          <Button type="button" onClick={signIn} disabled={signingIn}>
            Sign in as {email}
          </Button>
          {problem && (
            <p role="alert" className="text-sm">
              {problem}
            </p>
          )}
        </>
      )}
    </SignInFrame>
  );
}
