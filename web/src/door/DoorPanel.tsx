import { type FormEvent, useEffect, useId, useState } from "react";

import { api, type DayPage } from "@/api";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from "@/components/ui/dialog";
import { normalize, openWord, type WordLock } from "@/crypto";
import { useEventProgress, useProgress } from "@/store";
import { Markdown } from "./Markdown";

interface Loaded {
  page: DayPage;
  locks: WordLock[];
  hints: string[];
}

/** One open door: the puzzle, the answer box, hints, and the word once found. */
export function DoorPanel({
  event,
  day,
  onClose,
}: {
  event: string;
  day: number;
  onClose: () => void;
}) {
  const [data, setData] = useState<Loaded | null>(null);
  const [failed, setFailed] = useState(false);
  const progress = useEventProgress(event);
  const word = progress.words[day];

  useEffect(() => {
    Promise.all([
      api.day(event, day),
      api.locks(event, day),
      api.hints(event, day),
    ])
      .then(([page, locks, hints]) =>
        setData({ page, locks: locks.word_locks, hints: hints.hints }),
      )
      .catch(() => setFailed(true));
  }, [event, day]);

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent aria-describedby={undefined}>
        <DialogTitle className="pr-8 text-2xl font-semibold">
          Day {day}
          {data && `: ${data.page.title}`}
        </DialogTitle>
        {failed && <p role="alert">This door could not be opened.</p>}
        {!data && !failed && <p role="status">Opening…</p>}
        {data && (
          <>
            <Markdown event={event} day={day} text={data.page.markdown} />
            {word ? (
              <Solved word={word} />
            ) : (
              <AnswerForm event={event} day={day} locks={data.locks} />
            )}
            <Hints event={event} day={day} hints={data.hints} />
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}

function AnswerForm({
  event,
  day,
  locks,
}: {
  event: string;
  day: number;
  locks: WordLock[];
}) {
  const [guess, setGuess] = useState("");
  const [checking, setChecking] = useState(false);
  const [wrong, setWrong] = useState<string | null>(null);
  const { solve, attempt } = useProgress.getState();
  const inputId = useId();

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!guess.trim() || checking) return;
    setChecking(true);
    attempt(event, day);
    let found: string | null = null;
    for (const lock of locks) {
      found = await openWord(guess, lock);
      if (found) break;
    }
    setChecking(false);
    if (found) {
      solve(event, day, found);
    } else {
      setWrong(guess);
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
    <div className="flex flex-wrap items-center gap-3 rounded-md bg-accent/15 p-3">
      <p>
        Solved! Your word: <strong className="text-lg">{word}</strong>
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
          onClick={() =>
            useProgress.getState().revealHint(event, day, hints.length)
          }
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
