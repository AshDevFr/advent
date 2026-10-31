import { type ChangeEvent, useId, useState } from "react";

import { Button } from "@/components/ui/button";
import type { Navigate } from "./AdminApp";
import { admin, type ImportPreview, Unauthorized } from "./api";

/** Add players from a pasted list or a CSV file, after a preview. */
export function ImportPage({
  navigate,
  onSignedOut,
}: {
  navigate: Navigate;
  onSignedOut: () => void;
}) {
  const [text, setText] = useState("");
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const textId = useId();
  const fileId = useId();

  function failed(e: unknown) {
    if (e instanceof Unauthorized) onSignedOut();
    else setMessage(e instanceof Error ? e.message : String(e));
  }

  async function check() {
    setMessage(null);
    try {
      setPreview(await admin.preview(text));
    } catch (e) {
      failed(e);
    }
  }

  async function confirm() {
    try {
      const report = await admin.import(text);
      setMessage(
        `${report.created.length} player(s) added, ${report.updated.length} updated. Send their invitations from the overview.`,
      );
      setPreview(null);
      setText("");
    } catch (e) {
      failed(e);
    }
  }

  async function readFile(e: ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    setText(await file.text());
    setPreview(null);
  }

  return (
    <div className="flex flex-col gap-4">
      <h2 className="font-display text-3xl font-semibold">Invite players</h2>
      <p className="text-muted-foreground">
        One email address per line, optionally followed by a name:{" "}
        <code className="font-mono">alice@acme.com, Alice Martin</code>. A CSV
        file with <code className="font-mono">email</code> and{" "}
        <code className="font-mono">name</code> columns works too. Importing
        sends nothing: invitations go out from the overview.
      </p>
      <label htmlFor={textId} className="font-medium">
        Email addresses
      </label>
      <textarea
        id={textId}
        value={text}
        onChange={(e) => {
          setText(e.target.value);
          setPreview(null);
        }}
        rows={10}
        className="rounded-md border bg-card p-3 font-mono text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      />
      <div className="flex flex-wrap items-center gap-3">
        <label htmlFor={fileId} className="text-sm">
          Or a CSV file
        </label>
        <input
          id={fileId}
          type="file"
          accept=".csv,.txt,text/csv,text/plain"
          onChange={readFile}
          className="text-sm"
        />
      </div>
      <div className="flex gap-2">
        <Button
          type="button"
          variant="outline"
          onClick={check}
          disabled={!text.trim()}
        >
          Preview
        </Button>
        {preview && preview.new.length + preview.existing.length > 0 && (
          <Button type="button" onClick={confirm}>
            Import {preview.new.length} new player
            {preview.new.length === 1 ? "" : "s"}
          </Button>
        )}
      </div>
      {preview && (
        <section aria-label="Preview" className="grid gap-4 sm:grid-cols-3">
          <List
            title={`New (${preview.new.length})`}
            items={preview.new.map(label)}
          />
          <List
            title={`Already invited (${preview.existing.length})`}
            items={preview.existing.map(label)}
            note="Their name is updated if a new one is given."
          />
          <List
            title={`Not an address (${preview.invalid.length})`}
            items={preview.invalid}
          />
        </section>
      )}
      {message && (
        <p role="status">
          {message}{" "}
          <Button
            type="button"
            variant="link"
            className="px-0"
            onClick={() => navigate({ page: "overview" })}
          >
            Go to the overview
          </Button>
        </p>
      )}
    </div>
  );
}

const label = (i: { email: string; name: string | null }) =>
  i.name ? `${i.email} (${i.name})` : i.email;

function List({
  title,
  items,
  note,
}: {
  title: string;
  items: string[];
  note?: string;
}) {
  return (
    <div className="rounded-lg border bg-card p-3">
      <h3 className="font-medium">{title}</h3>
      {note && <p className="text-xs text-muted-foreground">{note}</p>}
      <ul className="mt-2 max-h-60 overflow-auto font-mono text-xs">
        {items.map((item) => (
          <li key={item}>{item}</li>
        ))}
      </ul>
    </div>
  );
}
