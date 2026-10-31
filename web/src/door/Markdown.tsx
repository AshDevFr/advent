import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

import { assetUrl } from "@/api";

/**
 * A puzzle's markdown, with GitHub's tables and strikethrough. Raw HTML is not rendered: nothing needs it, and
 * leaving it out narrows what an authoring mistake can do. Relative image
 * paths point at the day's published assets.
 */
export function Markdown({
  event,
  day,
  text,
}: {
  event: string;
  day: number;
  text: string;
}) {
  return (
    <div className="prose-puzzle flex flex-col gap-3 text-left leading-relaxed">
      <ReactMarkdown
        skipHtml
        remarkPlugins={[remarkGfm]}
        components={{
          img: ({ src, alt }) => (
            <img
              src={
                typeof src === "string" && !/^[a-z]+:|^\//i.test(src)
                  ? assetUrl(event, day, src)
                  : src
              }
              alt={alt ?? ""}
              className="mx-auto max-w-full rounded-md border"
            />
          ),
          pre: ({ children }) => (
            <pre className="overflow-x-auto rounded-md bg-muted p-3 text-sm">
              {children}
            </pre>
          ),
          code: ({ children }) => <code className="font-mono">{children}</code>,
          h1: ({ children }) => (
            <h3 className="text-xl font-semibold">{children}</h3>
          ),
          h2: ({ children }) => (
            <h4 className="text-lg font-semibold">{children}</h4>
          ),
          ul: ({ children }) => <ul className="list-disc pl-6">{children}</ul>,
          ol: ({ children }) => (
            <ol className="list-decimal pl-6">{children}</ol>
          ),
          table: ({ children }) => (
            <div className="overflow-x-auto">
              <table className="mx-auto border-collapse text-sm">
                {children}
              </table>
            </div>
          ),
          th: ({ children }) => (
            <th className="border bg-muted px-3 py-1.5 text-left font-semibold">
              {children}
            </th>
          ),
          td: ({ children }) => (
            <td className="border px-3 py-1.5">{children}</td>
          ),
          blockquote: ({ children }) => (
            <blockquote className="border-l-4 pl-3 text-muted-foreground">
              {children}
            </blockquote>
          ),
        }}
      >
        {text}
      </ReactMarkdown>
    </div>
  );
}
