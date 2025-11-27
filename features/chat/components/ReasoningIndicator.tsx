import { useEffect, useRef } from "react";
import ReactMarkdown from "react-markdown";

interface ReasoningIndicatorProps {
  content: string;
}

export function ReasoningIndicator({ content }: ReasoningIndicatorProps) {
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [content]);

  return (
    <div className="flex flex-col gap-2 p-3 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] max-w-2xl animate-in fade-in slide-in-from-bottom-2 duration-300">
      <div className="flex items-center gap-2 text-xs font-medium text-[var(--color-text-muted)] uppercase tracking-wider">
        <span className="relative flex h-2 w-2">
          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[var(--color-primary)] opacity-75"></span>
          <span className="relative inline-flex rounded-full h-2 w-2 bg-[var(--color-primary)]"></span>
        </span>
        IA en réflexion
      </div>

      <div
        ref={scrollRef}
        className="text-sm text-[var(--color-text-muted)] max-h-48 overflow-y-auto font-mono bg-[var(--color-bg-secondary)] p-2 rounded border border-[var(--color-border-subtle)]"
      >
        {content ? (
          <ReactMarkdown>{content}</ReactMarkdown>
        ) : (
          <span className="italic">Démarrage du raisonnement...</span>
        )}
      </div>
    </div>
  );
}
