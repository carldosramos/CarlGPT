import { ChatMessage } from "../types";
import MarkdownRenderer from "../../../components/MarkdownRenderer";

interface MessageBubbleProps {
  message: ChatMessage;
  isLastAssistant: boolean;
  isChatBusy: boolean;
  isStreaming: boolean;
  copiedMessageId: string | null;
  onCopy: (message: ChatMessage) => void;
  onRegenerate: () => void;
}

export function MessageBubble({
  message,
  isLastAssistant,
  isChatBusy,
  isStreaming,
  copiedMessageId,
  onCopy,
  onRegenerate,
}: MessageBubbleProps) {
  const isUser = message.role === "user";

  return (
    <div className={`flex flex-col ${isUser ? "items-end" : "items-start"}`}>
      <span className="text-xs text-[var(--color-text-muted)] mb-1">
        {isUser && "Toi"}
      </span>
      <div className="relative group w-full max-w-3xl">
        {message.role === "assistant" && (
          <div className="absolute -top-3 right-0 flex gap-2 opacity-0 group-hover:opacity-100 transition text-xs">
            <button
              type="button"
              onClick={() => onCopy(message)}
              className="px-2 py-1 rounded-full bg-[var(--color-surface-muted)] border border-[var(--color-border)] text-[var(--color-text)] hover:bg-[var(--color-surface)]"
            >
              {copiedMessageId === message.id ? "Copié !" : "Copier"}
            </button>
            {isLastAssistant && (
              <button
                type="button"
                onClick={onRegenerate}
                className="px-2 py-1 rounded-full bg-[var(--color-surface-muted)] border border-[var(--color-border)] text-[var(--color-text)] hover:bg-[var(--color-surface)] disabled:opacity-60"
                disabled={isChatBusy}
              >
                {isStreaming ? "Régén..." : "Régénérer"}
              </button>
            )}
          </div>
        )}
        <div
          className={`rounded-2xl px-4 py-3 text-sm leading-relaxed w-full ${
            isUser && "bg-[var(--color-primary)] text-white"
          }`}
        >
          {isUser ? (
            <p className="whitespace-pre-wrap break-words">{message.content}</p>
          ) : (
            <MarkdownRenderer content={message.content} />
          )}

          {message.attachments.length > 0 && (
            <div className="mt-3 flex flex-wrap gap-2 text-xs text-[var(--color-text-muted)]">
              {message.attachments.map((attachment) => (
                <a
                  key={attachment.id}
                  href={attachment.url}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex items-center gap-1 rounded-full border border-[var(--color-border)] px-2 py-1 bg-[var(--color-surface)] hover:border-[var(--color-primary)] transition"
                >
                  <span>📎</span>
                  <span className="truncate max-w-[10rem]">
                    {attachment.file_name}
                  </span>
                </a>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
