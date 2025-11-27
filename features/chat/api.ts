import { AttachmentMetadata, ChatSession, CompletionParams } from "./types";

const API_BASE = "http://127.0.0.1:4000/api";

export async function createChatSession(title?: string): Promise<ChatSession> {
  const res = await fetch(`${API_BASE}/chat/sessions`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ title }),
  });

  if (!res.ok) {
    throw new Error(await res.text());
  }

  return res.json();
}

export async function fetchChatSessions(): Promise<ChatSession[]> {
  const res = await fetch(`${API_BASE}/chat/sessions`);
  if (!res.ok) {
    throw new Error(await res.text());
  }
  return res.json();
}

export async function archiveChatSession(chatId: string): Promise<void> {
  const res = await fetch(`${API_BASE}/chat/sessions/${chatId}/archive`, {
    method: "POST",
  });
  if (!res.ok) {
    throw new Error(await res.text());
  }
}

export async function deleteChatSession(chatId: string): Promise<void> {
  const res = await fetch(`${API_BASE}/chat/sessions/${chatId}`, {
    method: "DELETE",
  });
  if (!res.ok) {
    throw new Error(await res.text());
  }
}

export async function sendMessageStream(
  chatId: string,
  content: string,
  model: string,
  attachments: Omit<AttachmentMetadata, "storage_key">[],
  completion_params?: CompletionParams
): Promise<Response> {
  const res = await fetch(
    `${API_BASE}/chat/sessions/${chatId}/messages/stream`,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Accept: "text/event-stream",
      },
      body: JSON.stringify({
        content,
        model,
        attachments,
        completion_params,
      }),
    }
  );

  if (!res.ok) {
    throw new Error(await res.text());
  }

  return res;
}

export async function regenerateMessageStream(
  chatId: string,
  messageId: string,
  model: string,
  completion_params?: CompletionParams
): Promise<Response> {
  const res = await fetch(
    `${API_BASE}/chat/sessions/${chatId}/regenerate/stream`,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Accept: "text/event-stream",
      },
      body: JSON.stringify({
        message_id: messageId,
        model,
        completion_params,
      }),
    }
  );

  if (!res.ok) {
    throw new Error(await res.text());
  }

  return res;
}

export async function uploadFile(file: File): Promise<AttachmentMetadata> {
  const formData = new FormData();
  formData.append("file", file);

  const res = await fetch(`${API_BASE}/uploads`, {
    method: "POST",
    body: formData,
  });

  if (!res.ok) {
    throw new Error(await res.text());
  }

  return res.json();
}
