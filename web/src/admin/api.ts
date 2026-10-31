/** The admin API of `advent serve`. */

export class Unauthorized extends Error {}

export interface AdminEvent {
  id: string;
  title: string;
  days: { day: number; date: string; title: string | null }[];
}

export interface DayStats {
  day: number;
  solved: number;
  completion: number;
  median_guesses: number | null;
  hint_out: number;
}

export interface EventStats {
  players: number;
  invited: number;
  signed_in: number;
  finished: number;
  days: DayStats[];
}

export interface PlayerRow {
  email: string;
  name: string | null;
  invited_at: string | null;
  last_seen: string | null;
  solved: number;
  finished_at: string | null;
}

export interface PlayerDetail extends PlayerRow {
  days: {
    day: number;
    guesses: number;
    solved_at: string | null;
    hint_out: boolean;
  }[];
}

export interface Invite {
  email: string;
  name: string | null;
}

export interface ImportPreview {
  new: Invite[];
  existing: Invite[];
  invalid: string[];
}

export interface ImportReport {
  created: string[];
  updated: string[];
  unchanged: string[];
  invalid: string[];
}

export interface InviteJob {
  running: boolean;
  total: number;
  sent: number;
  failed: { email: string; error: string }[];
}

async function call<T>(
  method: string,
  path: string,
  body?: unknown,
): Promise<T> {
  const response = await fetch(`api/admin/${path}`, {
    method,
    credentials: "same-origin",
    cache: "no-cache",
    headers:
      body === undefined ? undefined : { "content-type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (response.status === 401) throw new Unauthorized();
  if (!response.ok) {
    const error = (await response.json().catch(() => ({}))) as {
      error?: string;
    };
    throw new Error(error.error ?? `HTTP ${response.status}`);
  }
  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}

const enc = encodeURIComponent;

export interface AdminMode {
  auth: "password" | "accounts";
  setup_needed: boolean;
}

export interface AdminAccount {
  email: string;
  created_at: string;
}

export const admin = {
  mode: () => call<AdminMode>("GET", "mode"),
  login: (password: string) => call<void>("POST", "login", { password }),
  requestLink: (email: string) =>
    call<{ message: string }>("POST", "login", { email }),
  setup: (code: string, email: string) =>
    call<void>("POST", "setup", { code, email }),
  linkOwner: (link: string) =>
    call<{ email: string }>("GET", `signin/${enc(link)}`),
  redeemLink: (link: string) => call<void>("POST", "signin", { link }),
  admins: () => call<AdminAccount[]>("GET", "admins"),
  addAdmin: (email: string) => call<AdminAccount>("POST", "admins", { email }),
  removeAdmin: (email: string) => call<void>("DELETE", `admins/${enc(email)}`),
  logout: () => call<void>("POST", "logout", {}),
  session: () => call<void>("GET", "session"),
  events: () => call<AdminEvent[]>("GET", "events"),
  stats: (event: string) =>
    call<EventStats>("GET", `events/${enc(event)}/stats`),
  players: (event: string) =>
    call<PlayerRow[]>("GET", `events/${enc(event)}/players`),
  player: (event: string, email: string) =>
    call<PlayerDetail>("GET", `events/${enc(event)}/players/${enc(email)}`),
  preview: (text: string) =>
    call<ImportPreview>("POST", "import/preview", { text }),
  import: (text: string) => call<ImportReport>("POST", "import", { text }),
  invite: (emails?: string[]) =>
    call<InviteJob>("POST", "invitations", emails ? { emails } : {}),
  invitations: () => call<InviteJob>("GET", "invitations"),
  resend: (email: string) =>
    call<void>("POST", `players/${enc(email)}/resend`, {}),
  reset: (email: string) =>
    call<void>("POST", `players/${enc(email)}/reset`, {}),
  remove: (email: string) => call<void>("DELETE", `players/${enc(email)}`),
};

export const exportUrl = (event: string, file: "finishers" | "players") =>
  `api/admin/events/${enc(event)}/${file}.csv`;
