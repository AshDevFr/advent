import { useState } from "react";

import {
  formatTime,
  type Navigate,
  ProgressBar,
  useAdminData,
} from "./AdminApp";
import { type AdminEvent, admin } from "./api";

/** Every player, with their progress on the selected calendar. */
export function PlayersPage({
  event,
  navigate,
  onSignedOut,
}: {
  event: AdminEvent;
  navigate: Navigate;
  onSignedOut: () => void;
}) {
  const { data: rows, error } = useAdminData(
    () => admin.players(event.id),
    onSignedOut,
    [event.id],
  );
  const [filter, setFilter] = useState("");
  if (error) return <p role="alert">{error}</p>;
  if (!rows) return <p role="status">Loading…</p>;

  const needle = filter.trim().toLowerCase();
  const shown = rows.filter(
    (r) =>
      !needle ||
      r.email.toLowerCase().includes(needle) ||
      r.name?.toLowerCase().includes(needle),
  );

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h2 className="font-display text-3xl font-semibold">
          Players <span className="text-muted-foreground">({rows.length})</span>
        </h2>
        <label className="flex items-center gap-2 text-sm">
          Search
          <input
            type="search"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            className="h-8 rounded-md border bg-card px-2"
          />
        </label>
      </div>
      <div className="overflow-x-auto rounded-lg border bg-card">
        <table className="w-full text-sm">
          <thead className="text-left text-muted-foreground">
            <tr>
              <th className="p-3">Player</th>
              <th className="p-3">Invited</th>
              <th className="p-3">Last seen</th>
              <th className="p-3">Progress</th>
              <th className="p-3">Finished</th>
            </tr>
          </thead>
          <tbody>
            {shown.map((r) => (
              <tr key={r.email} className="border-t">
                <td className="p-3">
                  <button
                    type="button"
                    className="text-left underline-offset-4 hover:underline"
                    onClick={() => navigate({ page: "player", email: r.email })}
                  >
                    <span className="font-medium">{r.name ?? r.email}</span>
                    {r.name && (
                      <span className="block text-xs text-muted-foreground">
                        {r.email}
                      </span>
                    )}
                  </button>
                </td>
                <td className="p-3">{formatTime(r.invited_at) || "Not yet"}</td>
                <td className="p-3">{formatTime(r.last_seen) || "Never"}</td>
                <td className="p-3">
                  <ProgressBar value={r.solved} max={event.days.length} />
                </td>
                <td className="p-3">{formatTime(r.finished_at)}</td>
              </tr>
            ))}
          </tbody>
        </table>
        {shown.length === 0 && (
          <p className="p-4 text-muted-foreground">No players.</p>
        )}
      </div>
    </div>
  );
}
