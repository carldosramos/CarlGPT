export type ChatRole = "user" | "assistant";

export type ChatMessage = {
  id: string;
  session_id: string;
  role: ChatRole;
  content: string;
  position: number;
  created_at: string;
  attachments: ChatAttachment[];
};

export type ChatAttachment = {
  id: string;
  message_id: string;
  file_name: string;
  mime_type: string;
  size_bytes: number;
  url: string;
  created_at: string;
};

export type AttachmentMetadata = {
  file_name: string;
  mime_type: string;
  size_bytes: number;
  url: string;
  storage_key: string;
};

export type PendingAttachment = AttachmentMetadata & { clientId: string };

export type ChatSession = {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
  archived: boolean;
  messages: ChatMessage[];
};

export type StreamEventPayload = {
  type: "session" | "token" | "final" | "error" | "reasoning";
  session?: ChatSession;
  chatId?: string;
  messageId?: string;
  content?: string;
  message?: string;
};

export type CompletionParams = {
  temperature?: number;
  max_tokens?: number;
  top_p?: number;
  presence_penalty?: number;
  frequency_penalty?: number;
  seed?: number;
};
