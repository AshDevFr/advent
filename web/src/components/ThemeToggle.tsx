import { useState } from "react";

import { cn } from "@/lib/utils";

export type ThemeChoice = "system" | "light" | "dark";
const KEY = "advent-theme";

export function readTheme(): ThemeChoice {
  try {
    const value = localStorage.getItem(KEY);
    return value === "light" || value === "dark" ? value : "system";
  } catch {
    return "system";
  }
}

export function applyTheme(choice: ThemeChoice) {
  const root = document.documentElement;
  if (choice === "system") delete root.dataset.theme;
  else root.dataset.theme = choice;
  try {
    if (choice === "system") localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, choice);
  } catch {
    // Not persisted; the choice still applies to this visit.
  }
}

const CHOICES: { value: ThemeChoice; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

/** System / Light / Dark, defaulting to the system setting. */
export function ThemeToggle() {
  const [choice, setChoice] = useState<ThemeChoice>(readTheme);
  return (
    <fieldset className="flex shrink-0 rounded-md border bg-card p-0.5 text-xs">
      <legend className="sr-only">Colour theme</legend>
      {CHOICES.map(({ value, label }) => (
        <label
          key={value}
          className={cn(
            "cursor-pointer rounded px-2 py-1 font-medium text-muted-foreground has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-ring",
            choice === value && "bg-primary text-primary-foreground",
          )}
        >
          <input
            type="radio"
            name="theme"
            value={value}
            checked={choice === value}
            onChange={() => {
              setChoice(value);
              applyTheme(value);
            }}
            className="sr-only"
          />
          {label}
        </label>
      ))}
    </fieldset>
  );
}
