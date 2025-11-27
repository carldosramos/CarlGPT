import ChatPage from "../features/chat/components/ChatPage";
import { getCachedChatSessions } from "../features/chat/server-api";

export default async function Page() {
  const chats = await getCachedChatSessions();

  return <ChatPage initialChats={chats} />;
}
