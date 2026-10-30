import { Button } from "@/components/ui/button";

export function App() {
  return (
    <main className="mx-auto flex min-h-svh max-w-2xl flex-col items-center justify-center gap-6 p-6 text-center">
      <h1 className="text-3xl font-semibold">Advent calendar</h1>
      <p className="text-muted-foreground">The calendar is on its way.</p>
      <Button type="button">Open a door</Button>
    </main>
  );
}
