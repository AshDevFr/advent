import { type FormEvent, useId, useState } from "react";

import { Button } from "@/components/ui/button";
import { exportCode, importCode, useEventProgress, useProgress } from "@/store";

/** Copy progress to another browser, or restore it, with a code. */
export function Progress({ event }: { event: string }) {
  const progress = useEventProgress(event);
  const [code, setCode] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const inputId = useId();

  async function copyCode() {
    const value = exportCode(event, progress);
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
    const imported = importCode(event, code);
    if (!imported) {
      setMessage("That is not a progress code for this calendar.");
      return;
    }
    useProgress.getState().replace(event, imported);
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
