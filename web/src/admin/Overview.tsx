import { useEffect, useState } from "react";

import { Button, buttonVariants } from "@/components/ui/button";
import { type Navigate, useAdminData } from "./AdminApp";
import {
  type AdminEvent,
  admin,
  exportUrl,
  type InviteJob,
  Unauthorized,
} from "./api";

/** The event at a glance: who plays, and how each day went. */
export function Overview({
  event,
  navigate,
  onSignedOut,
}: {
  event: AdminEvent;
  navigate: Navigate;
  onSignedOut: () => void;
}) {
  const { data: stats, error } = useAdminData(
    () => admin.stats(event.id),
    onSignedOut,
    [event.id],
  );
  if (error) return <p role="alert">{error}</p>;
  if (!stats) return <p role="status">Loading…</p>;

  // The latest released day is today's door.
  const today = [...event.days].reverse().find((d) => d.title !== null);
  const todaySolved = today ? (stats.days[today.day - 1]?.solved ?? 0) : 0;
  const tiles = [
    { label: "Invited", value: stats.invited, of: stats.players },
    { label: "Signed in", value: stats.signed_in },
    {
      label: today ? `Solved day ${today.day}` : "Solved today",
      value: todaySolved,
    },
    { label: "Finished", value: stats.finished },
  ];

  return (
    <div className="flex flex-col gap-6">
      <h2 className="font-display text-3xl font-semibold">{event.title}</h2>
      <dl className="grid grid-cols-2 gap-3 sm:grid-cols-4">
        {tiles.map((tile) => (
          <div key={tile.label} className="rounded-lg border bg-card p-4">
            <dt className="text-sm text-muted-foreground">{tile.label}</dt>
            <dd className="font-display text-3xl font-semibold">
              {tile.value}
              {tile.of !== undefined && (
                <span className="text-base text-muted-foreground">
                  {" "}
                  of {tile.of}
                </span>
              )}
            </dd>
          </div>
        ))}
      </dl>
      <Invitations
        pending={stats.players - stats.invited}
        onSignedOut={onSignedOut}
        onImport={() => navigate({ page: "import" })}
      />
      <section
        aria-label="Days"
        className="overflow-x-auto rounded-lg border bg-card"
      >
        <table className="w-full text-sm">
          <thead className="text-left text-muted-foreground">
            <tr>
              <th className="p-3">Day</th>
              <th className="p-3">Date</th>
              <th className="p-3">Title</th>
              <th className="p-3">Completion</th>
              <th className="p-3">Median guesses</th>
              <th className="p-3">With a hint out</th>
            </tr>
          </thead>
          <tbody>
            {event.days.map((d) => {
              const s = stats.days[d.day - 1];
              return (
                <tr key={d.day} className="border-t">
                  <td className="p-3 font-mono">{d.day}</td>
                  <td className="p-3">{d.date}</td>
                  <td className="p-3">
                    {d.title ?? (
                      <span className="text-muted-foreground">
                        Not open yet
                      </span>
                    )}
                  </td>
                  <td className="p-3">
                    {d.title !== null && s && (
                      <>
                        <strong>{Math.round(s.completion * 100)}%</strong>{" "}
                        <span className="text-muted-foreground">
                          ({s.solved} of {stats.signed_in} signed in,{" "}
                          {stats.players} players)
                        </span>
                      </>
                    )}
                  </td>
                  <td className="p-3">{s?.median_guesses ?? ""}</td>
                  <td className="p-3">
                    {d.title !== null && s ? s.hint_out : ""}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </section>
      <section aria-label="Exports" className="flex flex-wrap gap-2">
        <a
          className={buttonVariants({ variant: "outline" })}
          href={exportUrl(event.id, "finishers")}
          download
        >
          Download finishers (CSV)
        </a>
        <a
          className={buttonVariants({ variant: "outline" })}
          href={exportUrl(event.id, "players")}
          download
        >
          Download all players (CSV)
        </a>
      </section>
    </div>
  );
}

/** Send invitations to everyone not invited yet, and follow the sending. */
function Invitations({
  pending,
  onSignedOut,
  onImport,
}: {
  pending: number;
  onSignedOut: () => void;
  onImport: () => void;
}) {
  const [job, setJob] = useState<InviteJob | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    admin
      .invitations()
      .then(setJob)
      .catch(() => {});
  }, []);

  useEffect(() => {
    if (!job?.running) return;
    const timer = setInterval(() => {
      admin
        .invitations()
        .then(setJob)
        .catch(() => {});
    }, 1000);
    return () => clearInterval(timer);
  }, [job?.running]);

  async function send() {
    setProblem(null);
    try {
      setJob(await admin.invite());
    } catch (e) {
      if (e instanceof Unauthorized) onSignedOut();
      else setProblem(e instanceof Error ? e.message : String(e));
    }
  }

  return (
    <section
      aria-label="Invitations"
      className="flex flex-wrap items-center gap-3 rounded-lg border bg-card p-4"
    >
      {job?.running ? (
        <p role="status">
          Sending invitations: {job.sent} of {job.total}…
        </p>
      ) : pending > 0 ? (
        <>
          <p className="flex-1">
            {pending} player{pending === 1 ? " has" : "s have"} not been invited
            yet.
          </p>
          <Button type="button" onClick={send}>
            Send invitations
          </Button>
        </>
      ) : (
        <>
          <p className="flex-1 text-muted-foreground">
            Every player has been invited.
          </p>
          <Button type="button" variant="outline" onClick={onImport}>
            Add players
          </Button>
        </>
      )}
      {job && !job.running && job.total > 0 && (
        <p role="status" className="w-full text-sm text-muted-foreground">
          Last sending: {job.sent} of {job.total} sent.
          {job.failed.length > 0 &&
            ` Failed: ${job.failed.map((f) => `${f.email} (${f.error})`).join(", ")}.`}
        </p>
      )}
      {problem && (
        <p role="alert" className="w-full text-sm">
          {problem}
        </p>
      )}
    </section>
  );
}
