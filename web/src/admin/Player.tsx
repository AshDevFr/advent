import { useState } from "react";

import { Button } from "@/components/ui/button";
import {
  formatTime,
  type Navigate,
  ProgressBar,
  useAdminData,
} from "./AdminApp";
import { type AdminEvent, admin, Unauthorized } from "./api";

/** One player: each day's guesses, and what can be done for them. */
export function PlayerPage({
  event,
  email,
  navigate,
  onSignedOut,
}: {
  event: AdminEvent;
  email: string;
  navigate: Navigate;
  onSignedOut: () => void;
}) {
  const {
    data: player,
    error,
    reload,
  } = useAdminData(() => admin.player(event.id, email), onSignedOut, [
    event.id,
    email,
  ]);
  const [message, setMessage] = useState<string | null>(null);
  const [confirming, setConfirming] = useState<"reset" | "remove" | null>(null);

  async function act(
    action: () => Promise<void>,
    done: string,
    after?: () => void,
  ) {
    setMessage(null);
    setConfirming(null);
    try {
      await action();
      setMessage(done);
      after ? after() : reload();
    } catch (e) {
      if (e instanceof Unauthorized) onSignedOut();
      else setMessage(e instanceof Error ? e.message : String(e));
    }
  }

  if (error) return <p role="alert">{error}</p>;
  if (!player) return <p role="status">Loading…</p>;
  const byDay = new Map(player.days.map((d) => [d.day, d]));

  return (
    <div className="flex flex-col gap-5">
      <div>
        <Button
          type="button"
          variant="link"
          className="px-0"
          onClick={() => navigate({ page: "players" })}
        >
          ← All players
        </Button>
        <h2 className="font-display text-3xl font-semibold">
          {player.name ?? player.email}
        </h2>
        {player.name && <p className="text-muted-foreground">{player.email}</p>}
      </div>
      <dl className="grid gap-2 text-sm sm:grid-cols-4">
        <div>
          <dt className="text-muted-foreground">Invited</dt>
          <dd>{formatTime(player.invited_at) || "Not yet"}</dd>
        </div>
        <div>
          <dt className="text-muted-foreground">Last seen</dt>
          <dd>{formatTime(player.last_seen) || "Never"}</dd>
        </div>
        <div>
          <dt className="text-muted-foreground">Progress</dt>
          <dd>
            <ProgressBar value={player.solved} max={event.days.length} />
          </dd>
        </div>
        <div>
          <dt className="text-muted-foreground">Finished</dt>
          <dd>{formatTime(player.finished_at) || "Not yet"}</dd>
        </div>
      </dl>
      <div className="overflow-x-auto rounded-lg border bg-card">
        <table className="w-full text-sm">
          <thead className="text-left text-muted-foreground">
            <tr>
              <th className="p-3">Day</th>
              <th className="p-3">Solved</th>
              <th className="p-3">Guesses</th>
              <th className="p-3">Hint out</th>
            </tr>
          </thead>
          <tbody>
            {event.days.map((d) => {
              const day = byDay.get(d.day);
              return (
                <tr key={d.day} className="border-t">
                  <td className="p-3 font-mono">{d.day}</td>
                  <td className="p-3">{formatTime(day?.solved_at ?? null)}</td>
                  <td className="p-3">{day?.guesses ?? ""}</td>
                  <td className="p-3">
                    {day?.solved_at ? (day.hint_out ? "Yes" : "No") : ""}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      <section aria-label="Actions" className="flex flex-wrap gap-2">
        <Button
          type="button"
          variant="outline"
          onClick={() => act(() => admin.resend(email), "Invitation sent.")}
        >
          Resend invitation
        </Button>
        {confirming === "reset" ? (
          <Button
            type="button"
            onClick={() =>
              act(
                () => admin.reset(email),
                "Link reset: the old link and its sessions no longer work. Resend the invitation to give them the new one.",
              )
            }
          >
            Confirm: reset the link
          </Button>
        ) : (
          <Button
            type="button"
            variant="outline"
            onClick={() => setConfirming("reset")}
          >
            Reset link
          </Button>
        )}
        {confirming === "remove" ? (
          <Button
            type="button"
            onClick={() =>
              act(
                () => admin.remove(email),
                "Player removed.",
                () => navigate({ page: "players" }),
              )
            }
          >
            Confirm: remove the player and their progress
          </Button>
        ) : (
          <Button
            type="button"
            variant="outline"
            onClick={() => setConfirming("remove")}
          >
            Remove player
          </Button>
        )}
      </section>
      {message && (
        <p role="status" className="text-sm">
          {message}
        </p>
      )}
    </div>
  );
}
