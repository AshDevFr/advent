import { type FormEvent, useId, useState } from "react";

import { Button } from "@/components/ui/button";
import { formatTime, useAdminData } from "./AdminApp";
import { admin, Unauthorized } from "./api";

/** Accounts mode: who can sign in to the admin section. */
export function AdminsPage({ onSignedOut }: { onSignedOut: () => void }) {
  const {
    data: admins,
    error,
    reload,
  } = useAdminData(() => admin.admins(), onSignedOut, []);
  const [email, setEmail] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const [confirming, setConfirming] = useState<string | null>(null);
  const inputId = useId();

  async function act(action: () => Promise<unknown>, done: string) {
    setMessage(null);
    setConfirming(null);
    try {
      await action();
      setMessage(done);
      reload();
    } catch (e) {
      if (e instanceof Unauthorized) onSignedOut();
      else setMessage(e instanceof Error ? e.message : String(e));
    }
  }

  async function add(e: FormEvent) {
    e.preventDefault();
    if (!email.trim()) return;
    const address = email.trim();
    await act(
      () => admin.addAdmin(address),
      `${address} added; a sign-in link was emailed to them.`,
    );
    setEmail("");
  }

  if (error) return <p role="alert">{error}</p>;
  if (!admins) return <p role="status">Loading…</p>;

  return (
    <div className="flex flex-col gap-4">
      <h2 className="font-display text-3xl font-semibold">Admins</h2>
      <ul className="flex flex-col divide-y rounded-lg border bg-card">
        {admins.map((a) => (
          <li key={a.email} className="flex flex-wrap items-center gap-3 p-3">
            <span className="flex-1">
              {a.email}
              <span className="block text-xs text-muted-foreground">
                Added {formatTime(a.created_at)}
              </span>
            </span>
            {admins.length > 1 &&
              (confirming === a.email ? (
                <Button
                  type="button"
                  size="sm"
                  onClick={() =>
                    act(() => admin.removeAdmin(a.email), `${a.email} removed.`)
                  }
                >
                  Confirm removal
                </Button>
              ) : (
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={() => setConfirming(a.email)}
                >
                  Remove
                </Button>
              ))}
          </li>
        ))}
      </ul>
      <form onSubmit={add} className="flex flex-col gap-2">
        <label htmlFor={inputId} className="font-medium">
          Add an admin
        </label>
        <div className="flex gap-2">
          <input
            id={inputId}
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            className="h-9 min-w-0 flex-1 rounded-md border bg-card px-3 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          />
          <Button type="submit">Add and email a link</Button>
        </div>
      </form>
      {message && (
        <p role="status" className="text-sm">
          {message}
        </p>
      )}
    </div>
  );
}
