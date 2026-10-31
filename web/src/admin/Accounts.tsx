import {
  type FormEvent,
  type ReactNode,
  useEffect,
  useId,
  useState,
} from "react";

import { Button } from "@/components/ui/button";
import { admin } from "./api";

const field =
  "h-9 rounded-md border bg-card px-3 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring";

function Frame({ children }: { children: ReactNode }) {
  return (
    <main className="mx-auto flex min-h-svh max-w-sm flex-col justify-center gap-4 px-4">
      <h1 className="font-display text-3xl font-semibold">Admin</h1>
      <div className="flex flex-col gap-3 rounded-lg border bg-card p-5">
        {children}
      </div>
    </main>
  );
}

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

/** Accounts mode: ask for a sign-in link by email. */
export function AccountLogin() {
  const [email, setEmail] = useState("");
  const [sent, setSent] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const inputId = useId();

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!email.trim()) return;
    setProblem(null);
    try {
      setSent((await admin.requestLink(email.trim())).message);
    } catch (e) {
      setProblem(message(e));
    }
  }

  return (
    <Frame>
      <form onSubmit={submit} className="flex flex-col gap-2">
        <label htmlFor={inputId} className="font-medium">
          Your admin email address
        </label>
        <input
          id={inputId}
          type="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          autoComplete="email"
          className={field}
        />
        <Button type="submit">Send me a sign-in link</Button>
      </form>
      {sent && (
        <p role="status" className="text-sm text-muted-foreground">
          {sent} It works once, within 15 minutes.
        </p>
      )}
      {problem && (
        <p role="alert" className="text-sm">
          {problem}
        </p>
      )}
    </Frame>
  );
}

/**
 * The first admin, on a new server: the setup code from the server's log
 * proves whoever fills this in runs the server.
 */
export function Setup({ onSignedIn }: { onSignedIn: () => void }) {
  const [code, setCode] = useState("");
  const [email, setEmail] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  const codeId = useId();
  const emailId = useId();

  async function submit(e: FormEvent) {
    e.preventDefault();
    setProblem(null);
    try {
      await admin.setup(code.trim(), email.trim());
      onSignedIn();
    } catch (e) {
      setProblem(message(e));
    }
  }

  return (
    <Frame>
      <h2 className="font-medium">Create the first admin</h2>
      <p className="text-sm text-muted-foreground">
        The setup code is printed in the server's log when it starts, or is the
        value of ADVENT_ADMIN__SETUP_CODE. It stops working once an admin
        exists.
      </p>
      <form onSubmit={submit} className="flex flex-col gap-2">
        <label htmlFor={codeId} className="font-medium">
          Setup code
        </label>
        <input
          id={codeId}
          value={code}
          onChange={(e) => setCode(e.target.value)}
          autoComplete="off"
          className={`${field} font-mono`}
        />
        <label htmlFor={emailId} className="font-medium">
          Your email address
        </label>
        <input
          id={emailId}
          type="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          autoComplete="email"
          className={field}
        />
        <Button type="submit" disabled={!code.trim() || !email.trim()}>
          Create and sign in
        </Button>
      </form>
      {problem && (
        <p role="alert" className="text-sm">
          {problem}
        </p>
      )}
    </Frame>
  );
}

/** `/admin/signin/<link>`: signs in on the button, never on the visit. */
export function AdminLinkPage({
  link,
  onSignedIn,
}: {
  link: string;
  onSignedIn: () => void;
}) {
  const [email, setEmail] = useState<string | null>(null);
  const [invalid, setInvalid] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    admin
      .linkOwner(link)
      .then((owner) => setEmail(owner.email))
      .catch(() => setInvalid(true));
  }, [link]);

  async function signIn() {
    setProblem(null);
    try {
      await admin.redeemLink(link);
      onSignedIn();
    } catch (e) {
      setProblem(message(e));
    }
  }

  if (invalid) return <AccountLoginAfterInvalid />;
  return (
    <Frame>
      {email === null ? (
        <p role="status">Checking your link…</p>
      ) : (
        <Button type="button" onClick={signIn}>
          Sign in as {email}
        </Button>
      )}
      {problem && (
        <p role="alert" className="text-sm">
          {problem}
        </p>
      )}
    </Frame>
  );
}

function AccountLoginAfterInvalid() {
  return (
    <>
      <p role="alert" className="mx-auto max-w-sm px-4 pt-10 text-sm">
        This sign-in link has been used or has expired. Ask for a new one.
      </p>
      <AccountLogin />
    </>
  );
}
