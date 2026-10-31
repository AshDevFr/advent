import { type FormEvent, useId, useState } from "react";
import { client } from "@/client";
import { Button } from "@/components/ui/button";

/**
 * Copy progress to another browser, or restore it, with a code. Only where
 * progress lives in the browser: a server already has it on every device.
 */
export function Progress({ event }: { event: string }) {
  const transfer = client().transfer;
  const [code, setCode] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const inputId = useId();

  if (!transfer) return null;

  async function copyCode() {
    if (!transfer) return;
    const value = transfer.exportCode(event);
    try {
      await navigator.clipboard.writeText(value);
      setMessage("Progress code copied.");
    } catch {
      setCode(value);
      setMessage("Copy the code from the box.");
    }
  }

  function restore(e: FormEvent) {
    e.preventDefault();
    if (!transfer?.importCode(event, code)) {
      setMessage("That is not a progress code for this calendar.");
      return;
    }
    setMessage("Progress restored.");
    setCode("");
  }

  return (
    <details className="rounded-lg border p-4">
      <summary className="cursor-pointer font-medium">
        Move your progress to another device
      </summary>
      <div className="mt-3 flex flex-col gap-3">
        <Button
          type="button"
          variant="outline"
          onClick={copyCode}
          className="self-start"
        >
          Copy progress code
        </Button>
        <form onSubmit={restore} className="flex flex-col gap-2">
          <label htmlFor={inputId} className="text-sm">
            Paste a progress code
          </label>
          <div className="flex gap-2">
            <input
              id={inputId}
              value={code}
              onChange={(e) => setCode(e.target.value)}
              className="h-9 min-w-0 flex-1 rounded-md border bg-card px-3 font-mono text-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            />
            <Button type="submit" variant="outline">
              Restore
            </Button>
          </div>
        </form>
        {message && (
          <p role="status" className="text-sm">
            {message}
          </p>
        )}
      </div>
    </details>
  );
}
