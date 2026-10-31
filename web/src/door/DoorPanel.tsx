import { type FormEvent, useEffect, useId, useState } from "react";

import { api, type DayPage } from "@/api";
import { client, useEventProgress } from "@/client";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from "@/components/ui/dialog";
import { InsecureContextError, normalize } from "@/crypto";
import { Markdown } from "./Markdown";

interface Loaded {
  page: DayPage;
  hints: string[];
}

/** One open door: the puzzle, the answer box, hints, and the word once found. */
export function DoorPanel({
  event,
  day,
  type,
  onClose,
}: {
  event: string;
  day: number;
  /** The day's type label, shown above the title. */
  type?: string;
  onClose: () => void;
}) {
  const [data, setData] = useState<Loaded | null>(null);
  const [failed, setFailed] = useState(false);
  const progress = useEventProgress(event);
  const word = progress.words[day];

  useEffect(() => {
    Promise.all([api.day(event, day), api.hints(event, day)])
      .then(([page, hints]) => setData({ page, hints: hints.hints }))
      .catch(() => setFailed(true));
  }, [event, day]);

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent aria-describedby={undefined}>
        <div className="flex flex-col gap-1 pr-8">
          <p className="text-xs font-semibold tracking-widest text-muted-foreground uppercase">
            Day {day}
            {type && ` · ${type}`}
          </p>
          <DialogTitle className="font-display text-3xl font-semibold tracking-tight">
            {data ? data.page.title : `Day ${day}`}
          </DialogTitle>
        </div>
        {failed && <p role="alert">This door could not be opened.</p>}
        {!data && !failed && <p role="status">Opening…</p>}
        {data && (
          <>
            <Markdown event={event} day={day} text={data.page.markdown} />
            {word ? (
              <Solved word={word} />
            ) : (
              <AnswerForm event={event} day={day} />
            )}
            <Hints event={event} day={day} hints={data.hints} />
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}

function AnswerForm({ event, day }: { event: string; day: number }) {
  const [guess, setGuess] = useState("");
  const [checking, setChecking] = useState(false);
  const [wrong, setWrong] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const inputId = useId();

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!guess.trim() || checking) return;
    setChecking(true);
    setProblem(null);
    try {
      const result = await client().guess(event, day, guess);
      if (result.kind === "wrong") {
        setWrong(guess);
      } else if (result.kind === "limited") {
        setWrong(null);
        setProblem(tooMany(result.retryAt));
      }
    } catch (e) {
      setWrong(null);
      setProblem(
        e instanceof InsecureContextError
          ? e.message
          : "Your answer could not be checked. Please try again.",
      );
    } finally {
      setChecking(false);
    }
  }

  return (
    <form onSubmit={submit} className="flex flex-col gap-2">
      <label htmlFor={inputId} className="font-medium">
        Your answer
      </label>
      <div className="flex gap-2">
        <input
          id={inputId}
          value={guess}
          onChange={(e) => setGuess(e.target.value)}
          autoComplete="off"
          className="h-9 min-w-0 flex-1 rounded-md border bg-card px-3 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        />
        <Button type="submit" disabled={checking}>
          {checking ? "Checking…" : "Check"}
        </Button>
      </div>
      {problem && (
        <p role="alert" className="text-sm">
          {problem}
        </p>
      )}
      {wrong !== null && (
        <p role="alert" className="text-sm text-muted-foreground">
          Not quite. Answers are compared in lowercase, ignoring accents, spaces
          and punctuation, so we read “{wrong}” as “
          {normalize(wrong) || "nothing"}”.
        </p>
      )}
    </form>
  );
}

/** The wait before the next guess, in words. */
function tooMany(retryAt: string): string {
  const minutes = Math.max(
    1,
    Math.ceil((Date.parse(retryAt) - Date.now()) / 60_000),
  );
  return `Too many guesses for now. Try again in ${minutes} minute${minutes === 1 ? "" : "s"}.`;
}

function Solved({ word }: { word: string }) {
  const [copied, setCopied] = useState(false);
  async function copy() {
    try {
      await navigator.clipboard.writeText(word);
      setCopied(true);
    } catch {
      setCopied(false);
    }
  }
  return (
    <div className="flex flex-wrap items-center gap-3 rounded-md border border-primary bg-primary/10 p-3">
      <p>
        Solved! Your word:{" "}
        <strong className="font-mono text-lg text-primary">{word}</strong>
      </p>
      <Button type="button" variant="outline" size="sm" onClick={copy}>
        {copied ? "Copied" : "Copy word"}
      </Button>
    </div>
  );
}

function Hints({
  event,
  day,
  hints,
}: {
  event: string;
  day: number;
  hints: string[];
}) {
  const used = useEventProgress(event).hintsUsed[day] ?? 0;
  const shown = hints.slice(0, used);
  const left = hints.length - shown.length;
  return (
    <section aria-label="Hints" className="flex flex-col gap-2">
      {shown.map((hint, i) => (
        <p key={hint} className="rounded-md border border-dashed p-2 text-sm">
          <span className="font-medium">Hint {i + 1}:</span> {hint}
        </p>
      ))}
      {left > 0 && (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="self-start"
          onClick={() => client().revealHint(event, day, hints.length)}
        >
          Show a hint ({left} available)
        </Button>
      )}
      {hints.length === 0 && (
        <DialogDescription className="text-sm text-muted-foreground">
          No hint yet: hints arrive at the next midnights after a door opens.
        </DialogDescription>
      )}
    </section>
  );
}
