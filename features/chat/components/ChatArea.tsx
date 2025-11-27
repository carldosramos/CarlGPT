import { useEffect, useRef } from "react";
import { ChatMessage, ChatSession } from "../types";
import { MessageBubble } from "./MessageBubble";
import { ReasoningIndicator } from "./ReasoningIndicator";

interface ChatAreaProps {
  currentChat?: ChatSession;
  isLoading: boolean;
  isChatBusy: boolean;
  streamingState: { chatId: string; messageId: string } | null;
  reasoningContent: string;
  copiedMessageId: string | null;
  onCopyMessage: (msg: ChatMessage) => void;
  onRegenerateMessage: (chat: ChatSession, msgId: string) => void;
}

export function ChatArea({
  currentChat,
  isLoading,
  isChatBusy,
  streamingState,
  reasoningContent,
  copiedMessageId,
  onCopyMessage,
  onRegenerateMessage,
}: ChatAreaProps) {
  const messagesEndRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (messagesEndRef.current) {
      messagesEndRef.current.scrollIntoView({ behavior: "auto" });
    }
  }, [currentChat?.id, currentChat?.messages.length, reasoningContent]);

  return (
    <div className="flex-1 overflow-y-auto">
      <div className="max-w-4xl w-full mx-auto px-4 md:px-8 py-6 flex flex-col gap-4">
        <div className="flex flex-col gap-2">
          <h2 className="text-2xl font-semibold text-[var(--color-text)]">
            {currentChat?.title ?? "Discussion"}
          </h2>
        </div>

        <div className="space-y-4 pr-1">
          {isLoading && (
            <div className="flex items-center gap-2 text-sm text-[var(--color-text-muted)]">
              <span className="h-4 w-4 rounded-full border-2 border-[var(--color-primary)] border-t-transparent animate-spin" />
              Chargement des discussions…
            </div>
          )}

          {!isLoading &&
            currentChat?.messages.map((message) => {
              const isLastAssistant =
                message.role !== "user" &&
                currentChat?.messages[currentChat.messages.length - 1]?.id ===
                  message.id;

              const isStreaming =
                isChatBusy && streamingState?.messageId === message.id;

              return (
                <MessageBubble
                  key={message.id}
                  message={message}
                  isLastAssistant={isLastAssistant}
                  isChatBusy={isChatBusy}
                  isStreaming={isStreaming}
                  copiedMessageId={copiedMessageId}
                  onCopy={onCopyMessage}
                  onRegenerate={() =>
                    onRegenerateMessage(currentChat, message.id)
                  }
                />
              );
            })}

          {!isLoading && currentChat && currentChat.messages.length === 0 && (
            <p className="text-sm text-[var(--color-text-muted)]">
              Pose ta première question à l&apos;IA pour démarrer la discussion.
            </p>
          )}

          {isChatBusy && <ReasoningIndicator content={reasoningContent} />}

          <div ref={messagesEndRef} />
        </div>
      </div>
    </div>
  );
}
