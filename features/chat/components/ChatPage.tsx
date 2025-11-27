"use client";

import { Suspense } from "react";
import { ChatSidebar } from "./ChatSidebar";
import { ChatArea } from "./ChatArea";
import { ChatInput } from "./ChatInput";
import { useChat } from "../hooks/useChat";
import { ChatSession } from "../types";

declare global {
  interface Window {
    MathJax?: {
      typesetPromise?: () => Promise<void>;
    };
  }
}

interface ChatPageProps {
  initialChats: ChatSession[];
}

export default function ChatPage({ initialChats }: ChatPageProps) {
  const {
    chats,
    currentChat,
    selectedChatId,
    setSelectedChatId,
    prompt,
    setPrompt,
    chatsLoading,
    chatListError,
    chatError,
    chatActionLoadingId,
    creatingChat,
    isChatBusy,
    streamingState,
    reasoningContent,
    copiedMessageId,
    selectedModelId,
    setSelectedModelId,
    completionParams,
    setCompletionParams,
    pendingAttachments,
    pendingUploadCount,
    handleCreateChat,
    handleArchiveChat,
    handleDeleteChat,
    handleAskAI,
    handleCopyMessage,
    handleRegenerateMessage,
    handleFilesSelected,
    handleRemoveAttachment,
  } = useChat(initialChats);

  return (
    <main className="flex h-screen bg-[var(--color-surface-muted)] text-[var(--color-text)] overflow-hidden">
      <Suspense
        fallback={
          <div className="w-full max-w-xs border-r border-[var(--color-border)] bg-[var(--color-surface)] hidden md:flex" />
        }
      >
        <ChatSidebar
          chats={chats}
          currentChatId={selectedChatId}
          onSelectChat={setSelectedChatId}
          onCreateChat={handleCreateChat}
          onDeleteChat={handleDeleteChat}
          onArchiveChat={handleArchiveChat}
          isCreating={creatingChat}
          isLoading={chatsLoading}
          error={chatListError}
          actionLoadingId={chatActionLoadingId}
        />
      </Suspense>

      <section className="flex-1 flex flex-col h-full overflow-hidden">
        <ChatArea
          currentChat={currentChat}
          isLoading={chatsLoading}
          isChatBusy={!!isChatBusy}
          streamingState={streamingState}
          reasoningContent={reasoningContent}
          copiedMessageId={copiedMessageId}
          onCopyMessage={handleCopyMessage}
          onRegenerateMessage={handleRegenerateMessage}
        />

        <ChatInput
          prompt={prompt}
          setPrompt={setPrompt}
          onSubmit={handleAskAI}
          isChatBusy={!!isChatBusy}
          canUseChat={!chatsLoading && !!currentChat}
          pendingAttachments={pendingAttachments}
          pendingUploadCount={pendingUploadCount}
          onRemoveAttachment={handleRemoveAttachment}
          onFilesSelected={handleFilesSelected}
          selectedModelId={selectedModelId}
          setSelectedModelId={setSelectedModelId}
          completionParams={completionParams}
          setCompletionParams={setCompletionParams}
          chatError={chatError}
          currentChatId={currentChat?.id}
        />
      </section>
    </main>
  );
}
