import { useEffect, useState } from "react";

function parts(ms: number) {
  const total = Math.max(0, Math.floor(ms / 1000));
  return {
    days: Math.floor(total / 86400),
    hours: Math.floor((total % 86400) / 3600),
    minutes: Math.floor((total % 3600) / 60),
  };
}

/** Time left until `target`, refreshed every 30 seconds. */
export function Countdown({ target }: { target: Date }) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(timer);
  }, []);
  const { days, hours, minutes } = parts(target.getTime() - now);
  const label = `Opens in ${days} days, ${hours} hours and ${minutes} minutes`;
  return (
    <div
      role="timer"
      aria-label={label}
      className="flex justify-center gap-4 tabular-nums"
    >
      {[
        [days, "days"],
        [hours, "hours"],
        [minutes, "minutes"],
      ].map(([value, unit]) => (
        <span key={unit} aria-hidden="true" className="flex flex-col">
          <span className="text-3xl font-semibold text-primary">{value}</span>
          <span className="text-xs text-muted-foreground">{unit}</span>
        </span>
      ))}
    </div>
  );
}
