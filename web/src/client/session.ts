/** Whether the player is signed in, on a server-hosted calendar. */
import { create } from "zustand";

export type Session =
  | { status: "checking" }
  | { status: "out" }
  | { status: "in"; email: string; name: string | null };

export const useSession = create<{
  session: Session;
  set: (session: Session) => void;
}>()((set) => ({
  session: { status: "checking" },
  set: (session) => set({ session }),
}));

export const signedOut = () => useSession.getState().set({ status: "out" });
