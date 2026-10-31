import { useState } from "react";

import { rewardUrl } from "@/api";
import { buttonVariants } from "@/components/ui/button";
import { openReward, type Reward } from "@/crypto";
import { download } from "./exportPng";

function size(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** What else finishing unlocks: links, and files decrypted on demand. */
export function Rewards({
  event,
  rewards,
}: {
  event: string;
  rewards: Reward[];
}) {
  if (rewards.length === 0) return null;
  return (
    <section aria-labelledby="rewards-heading" className="flex flex-col gap-3">
      <h3 id="rewards-heading" className="font-display text-xl font-semibold">
        Your rewards
      </h3>
      <ul className="flex flex-col gap-2">
        {rewards.map((reward) => (
          <li
            key={`${reward.label}-${reward.url ?? reward.file?.index}`}
            className="flex flex-wrap items-center justify-between gap-3 rounded-md border bg-background p-3"
          >
            <span className="font-medium">{reward.label}</span>
            {reward.url && (
              <a
                href={reward.url}
                target="_blank"
                rel="noopener noreferrer"
                className={buttonVariants({ variant: "outline", size: "sm" })}
              >
                Open link
              </a>
            )}
            {reward.file && <FileButton event={event} file={reward.file} />}
          </li>
        ))}
      </ul>
    </section>
  );
}

function FileButton({
  event,
  file,
}: {
  event: string;
  file: NonNullable<Reward["file"]>;
}) {
  const [state, setState] = useState<"idle" | "busy" | "failed">("idle");

  async function save() {
    setState("busy");
    try {
      const response = await fetch(rewardUrl(event, file.index));
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const plain = await openReward(file, await response.arrayBuffer());
      if (!plain) throw new Error("does not decrypt");
      download(new Blob([plain]), file.name);
      setState("idle");
    } catch {
      setState("failed");
    }
  }

  return (
    <span className="flex items-center gap-2">
      {state === "failed" && (
        <span role="alert" className="text-sm">
          Download failed, try again.
        </span>
      )}
      <button
        type="button"
        onClick={save}
        disabled={state === "busy"}
        className={buttonVariants({ size: "sm" })}
      >
        {state === "busy"
          ? "Preparing…"
          : `Download ${file.name} (${size(file.size)})`}
      </button>
    </span>
  );
}
