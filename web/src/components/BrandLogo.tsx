import { cn } from "@/lib/utils";

/** The brand logo, with its dark version on dark pages when there is one. */
export function BrandLogo({
  logo,
  dark,
  className,
}: {
  logo: string;
  dark?: string | null;
  className?: string;
}) {
  if (!dark) return <img src={logo} alt="" className={className} />;
  return (
    <>
      <img src={logo} alt="" className={cn("when-light", className)} />
      <img src={dark} alt="" className={cn("when-dark", className)} />
    </>
  );
}
