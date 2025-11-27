import { useCallback, useEffect, useMemo, useState } from "react";
import {
  ChatSession,
  ChatMessage,
  PendingAttachment,
  StreamEventPayload,
  CompletionParams,
} from "../types";
import {
  createChatSession,
  fetchChatSessions,
  archiveChatSession,
  deleteChatSession,
  sendMessageStream,
  regenerateMessageStream,
  uploadFile,
} from "../api";
import { MODEL_OPTIONS } from "../constants";

const sortChatsDesc = (sessions: ChatSession[]) =>
  [...sessions].sort(
    (a, b) =>
      new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
  );

export function useChat(initialChats: ChatSession[] = []) {
  const [chats, setChats] = useState<ChatSession[]>(initialChats);
  const [selectedChatId, setSelectedChatId] = useState("");
  const [prompt, setPrompt] = useState("");
  const [chatLoadingId, setChatLoadingId] = useState<string | null>(null);
  const [chatsLoading, setChatsLoading] = useState(initialChats.length === 0);
  const [creatingChat, setCreatingChat] = useState(false);
  const [chatListError, setChatListError] = useState<string | null>(null);
  const [chatError, setChatError] = useState<{
    chatId: string;
    message: string;
  } | null>(null);
  const [chatActionLoadingId, setChatActionLoadingId] = useState<string | null>(
    null
  );
  const [streamingState, setStreamingState] = useState<{
    chatId: string;
    messageId: string;
  } | null>(null);
  const [reasoningContent, setReasoningContent] = useState("");
  const [copiedMessageId, setCopiedMessageId] = useState<string | null>(null);
  const [selectedModelId, setSelectedModelId] = useState<string>(
    () => MODEL_OPTIONS[0].id
  );
  const [completionParams, setCompletionParams] = useState<CompletionParams>(
    () => ({
      temperature: 0.7,
      max_tokens: undefined,
      top_p: 1.0,
      presence_penalty: 0.0,
      frequency_penalty: 0.0,
    })
  );
  const [pendingAttachments, setPendingAttachments] = useState<
    PendingAttachment[]
  >([]);
  const [pendingUploadCount, setPendingUploadCount] = useState(0);

  const persistChatSession = useCallback(async (title?: string) => {
    const session = await createChatSession(title);
    setChats((prev) => {
      const withoutCurrent = prev.filter((chat) => chat.id !== session.id);
      return sortChatsDesc([session, ...withoutCurrent]);
    });
    setSelectedChatId(session.id);
    return session;
  }, []);

  const currentChat = useMemo(() => {
    if (chats.length === 0) {
      return undefined;
    }
    return chats.find((chat) => chat.id === selectedChatId) ?? chats[0];
  }, [chats, selectedChatId]);

  const isStreamingCurrent = streamingState?.chatId === currentChat?.id;
  const isChatBusy =
    (chatLoadingId !== null && chatLoadingId === currentChat?.id) ||
    isStreamingCurrent;

  const handleStreamEvent = useCallback(
    (event: StreamEventPayload) => {
      if (!event || typeof event !== "object") {
        return;
      }

      switch (event.type) {
        case "session": {
          const session = event.session as ChatSession | undefined;
          if (!session) {
            return;
          }
          setChats((prev) =>
            sortChatsDesc([
              session,
              ...prev.filter((chat) => chat.id !== session.id),
            ])
          );
          setSelectedChatId((prev) => prev || session.id);
          if (event.chatId && event.messageId) {
            setStreamingState({
              chatId: event.chatId,
              messageId: event.messageId,
            });
          }
          break;
        }
        case "token": {
          const { chatId, messageId, content } = event;
          if (!chatId || !messageId || !content) {
            return;
          }
          setChats((prev) =>
            prev.map((chat) =>
              chat.id === chatId
                ? {
                    ...chat,
                    messages: chat.messages.map((msg) =>
                      msg.id === messageId
                        ? { ...msg, content: `${msg.content}${content}` }
                        : msg
                    ),
                  }
                : chat
            )
          );
          break;
        }
        case "reasoning": {
          const { content } = event;
          console.log("🧠 REASONING EVENT:", content); // DEBUG
          if (content) {
            setReasoningContent((prev) => prev + content);
          }
          break;
        }
        case "final": {
          const session = event.session as ChatSession | undefined;
          if (session) {
            setChats((prev) =>
              sortChatsDesc([
                session,
                ...prev.filter((chat) => chat.id !== session.id),
              ])
            );
          }
          setStreamingState(null);
          setChatLoadingId(null);
          setReasoningContent("");
          break;
        }
        case "error": {
          setChatError({
            chatId:
              event.chatId ?? streamingState?.chatId ?? currentChat?.id ?? "",
            message:
              event.message ??
              "Erreur lors de la génération de la réponse en streaming.",
          });
          setStreamingState(null);
          setChatLoadingId(null);
          setReasoningContent("");
          break;
        }
        default:
          break;
      }
    },
    [currentChat?.id, streamingState]
  );

  const consumeAssistantStream = useCallback(
    async (response: Response) => {
      if (!response.body) {
        throw new Error("Flux SSE indisponible");
      }

      const reader = response.body.getReader();
      const decoder = new TextDecoder("utf-8");
      let buffer = "";

      while (true) {
        const { value, done } = await reader.read();
        if (done) {
          break;
        }
        buffer += decoder.decode(value, { stream: true });
        const events = buffer.split("\n\n");
        buffer = events.pop() ?? "";

        for (const eventChunk of events) {
          const dataLine = eventChunk
            .split("\n")
            .find((line) => line.startsWith("data:"));
          if (!dataLine) continue;
          try {
            const payload = JSON.parse(dataLine.slice(5)) as StreamEventPayload;
            handleStreamEvent(payload);
          } catch (err) {
            console.error("Payload SSE invalide :", err);
          }
        }
      }

      setChatLoadingId(null);
      setStreamingState(null);
      setReasoningContent("");
    },
    [handleStreamEvent]
  );

  const handleAskAI = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!currentChat || prompt.trim().length === 0) {
      return;
    }
    if (pendingUploadCount > 0) {
      setChatError({
        chatId: currentChat.id,
        message: "Patiente, les fichiers se téléversent encore.",
      });
      return;
    }
    if (
      pendingAttachments.length > 0 &&
      selectedModelId === "llama-3.1-8b-instant"
    ) {
      setChatError({
        chatId: currentChat.id,
        message:
          "Les fichiers et images nécessitent un modèle OpenAI (GPT-5.1, GPT-5 mini, etc.).",
      });
      return;
    }

    const messageContent = prompt.trim();
    setPrompt("");
    setChatError(null);
    setChatLoadingId(currentChat.id);
    setReasoningContent("");

    // Optimistic update: Add user message immediately
    const optimisticMessageId = crypto.randomUUID();
    const optimisticMessage: ChatMessage = {
      id: optimisticMessageId,
      session_id: currentChat.id,
      role: "user",
      content: messageContent,
      position: currentChat.messages.length + 1,
      created_at: new Date().toISOString(),
      attachments: pendingAttachments.map((att) => ({
        id: crypto.randomUUID(),
        message_id: optimisticMessageId,
        file_name: att.file_name,
        mime_type: att.mime_type,
        size_bytes: att.size_bytes,
        url: att.url,
        created_at: new Date().toISOString(),
      })),
    };

    setChats((prev) =>
      prev.map((c) =>
        c.id === currentChat.id
          ? { ...c, messages: [...c.messages, optimisticMessage] }
          : c
      )
    );

    const attachmentsPayload = pendingAttachments.map(
      ({ clientId, ...rest }) => rest
    );

    try {
      const res = await sendMessageStream(
        currentChat.id,
        messageContent,
        selectedModelId,
        attachmentsPayload,
        completionParams
      );
      await consumeAssistantStream(res);
      setPendingAttachments([]);
    } catch (error) {
      console.error("Erreur IA :", error);
      setPrompt(messageContent);
      // Remove optimistic message on error
      setChats((prev) =>
        prev.map((c) =>
          c.id === currentChat.id
            ? {
                ...c,
                messages: c.messages.filter(
                  (m) => m.id !== optimisticMessageId
                ),
              }
            : c
        )
      );
      setChatError({
        chatId: currentChat.id,
        message:
          "Impossible d'envoyer le message à l'IA. Vérifie le backend puis réessaie.",
      });
      setChatLoadingId(null);
      setStreamingState(null);
      setReasoningContent("");
    }
  };

  const handleArchiveChat = async (chatId: string) => {
    setChatListError(null);
    setChatActionLoadingId(chatId);
    try {
      await archiveChatSession(chatId);
      setChats((prev) => {
        const next = prev.filter((chat) => chat.id !== chatId);
        if (selectedChatId === chatId) {
          setSelectedChatId(next.length > 0 ? next[0].id : "");
          setPrompt("");
          setChatError(null);
        }
        return next;
      });
      if (chats.length <= 1) {
        try {
          await persistChatSession();
        } catch (error) {
          console.error("Erreur création discussion :", error);
          setChatListError(
            "Impossible de créer une nouvelle discussion pour le moment."
          );
        }
      }
    } catch (error) {
      console.error("Erreur archivage :", error);
      setChatListError("Impossible d'archiver cette discussion.");
    } finally {
      setChatActionLoadingId(null);
    }
  };

  const handleDeleteChat = async (chatId: string) => {
    setChatListError(null);
    setChatActionLoadingId(chatId);
    try {
      await deleteChatSession(chatId);
      setChats((prev) => {
        const next = prev.filter((chat) => chat.id !== chatId);
        if (selectedChatId === chatId) {
          setSelectedChatId(next.length > 0 ? next[0].id : "");
          setPrompt("");
          setChatError(null);
        }
        return next;
      });
      if (chats.length <= 1) {
        try {
          await persistChatSession();
        } catch (error) {
          console.error("Erreur création discussion :", error);
          setChatListError(
            "Impossible de créer une nouvelle discussion pour le moment."
          );
        }
      }
    } catch (error) {
      console.error("Erreur suppression :", error);
      setChatListError("Impossible de supprimer cette discussion.");
    } finally {
      setChatActionLoadingId(null);
    }
  };

  const handleCopyMessage = useCallback(async (message: ChatMessage) => {
    try {
      await navigator.clipboard.writeText(message.content);
      setCopiedMessageId(message.id);
      setTimeout(() => setCopiedMessageId(null), 2000);
    } catch (error) {
      console.error("Impossible de copier le message :", error);
    }
  }, []);

  const handleFilesSelected = useCallback(
    async (fileList: FileList | null) => {
      if (!fileList || fileList.length === 0) {
        return;
      }

      const files = Array.from(fileList);
      for (const file of files) {
        setPendingUploadCount((count) => count + 1);
        try {
          const meta = await uploadFile(file);
          const clientId =
            typeof crypto !== "undefined" && crypto.randomUUID
              ? crypto.randomUUID()
              : `${Date.now()}-${Math.random()}`;
          setPendingAttachments((prev) => [...prev, { ...meta, clientId }]);
        } catch (error) {
          console.error("Upload de fichier :", error);
          setChatError({
            chatId: currentChat?.id ?? "",
            message: "Impossible d'uploader ce fichier.",
          });
        } finally {
          setPendingUploadCount((count) => Math.max(0, count - 1));
        }
      }
    },
    [currentChat?.id]
  );

  const handleRemoveAttachment = useCallback((clientId: string) => {
    setPendingAttachments((prev) =>
      prev.filter((att) => att.clientId !== clientId)
    );
  }, []);

  const handleRegenerateMessage = useCallback(
    async (chat: ChatSession, messageId: string) => {
      setChatError(null);
      setChatLoadingId(chat.id);
      setReasoningContent("");

      setChats((prev) =>
        prev.map((c) =>
          c.id === chat.id
            ? {
                ...c,
                messages: c.messages.map((msg) =>
                  msg.id === messageId ? { ...msg, content: "" } : msg
                ),
              }
            : c
        )
      );

      try {
        const res = await regenerateMessageStream(
          chat.id,
          messageId,
          selectedModelId,
          completionParams
        );
        await consumeAssistantStream(res);
      } catch (error) {
        console.error("Erreur régénération :", error);
        setChatError({
          chatId: chat.id,
          message: "Impossible de régénérer cette réponse.",
        });
        setStreamingState(null);
        setReasoningContent("");
      } finally {
        setChatLoadingId(null);
      }
    },
    [consumeAssistantStream, selectedModelId]
  );

  const handleCreateChat = async () => {
    setCreatingChat(true);
    setChatError(null);
    setChatListError(null);
    try {
      await persistChatSession();
      setPrompt("");
    } catch (error) {
      console.error("Erreur création discussion :", error);
      setChatListError(
        "Impossible de créer une nouvelle discussion pour le moment."
      );
    } finally {
      setCreatingChat(false);
    }
  };

  useEffect(() => {
    const loadChats = async () => {
      setChatsLoading(true);
      try {
        const data = await fetchChatSessions();
        if (data.length === 0) {
          await persistChatSession();
        } else {
          setChats(sortChatsDesc(data));
          setSelectedChatId((prev) => prev || data[0].id);
        }
        setChatListError(null);
      } catch (error) {
        console.error("Erreur chargement des discussions :", error);
        setChatListError(
          "Impossible de charger les discussions IA pour le moment."
        );
      } finally {
        setChatsLoading(false);
      }
    };

    loadChats();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    const stored = window.localStorage.getItem("preferred-model");
    if (stored && MODEL_OPTIONS.some((option) => option.id === stored)) {
      setSelectedModelId(stored);
    }
  }, []);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    window.localStorage.setItem("preferred-model", selectedModelId);
  }, [selectedModelId]);

  useEffect(() => {
    if (!selectedChatId && chats.length > 0) {
      setSelectedChatId(chats[0].id);
    }
  }, [selectedChatId, chats]);

  useEffect(() => {
    setPendingAttachments([]);
    setPendingUploadCount(0);
  }, [currentChat?.id]);

  return {
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
  };
}
