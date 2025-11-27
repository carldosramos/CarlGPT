import { ChatSession } from "./types";

const API_BASE = "http://127.0.0.1:4000/api";

export async function getCachedChatSessions(): Promise<ChatSession[]> {
  "use cache";
  try {
    const res = await fetch(`${API_BASE}/chat/sessions`);
    if (!res.ok) {
      console.error("Failed to fetch chats:", await res.text());
      return [];
    }
    return res.json();
  } catch (error) {
    console.error("Error fetching cached chats:", error);
    return [];
  }
}
