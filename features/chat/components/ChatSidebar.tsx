import { useMemo, useState, useEffect, MouseEvent } from "react";
import { ChatSession } from "../types";

interface ChatSidebarProps {
  chats: ChatSession[];
  currentChatId?: string;
  onSelectChat: (id: string) => void;
  onCreateChat: () => void;
  onDeleteChat: (id: string) => void;
  onArchiveChat: (id: string) => void;
  isCreating: boolean;
  isLoading: boolean;
  error: string | null;
  actionLoadingId: string | null;
}

export function ChatSidebar({
  chats,
  currentChatId,
  onSelectChat,
  onCreateChat,
  onDeleteChat,
  onArchiveChat,
  isCreating,
  isLoading,
  error,
  actionLoadingId,
}: ChatSidebarProps) {
  const [sidebarQuery, setSidebarQuery] = useState("");
  const [menuOpenId, setMenuOpenId] = useState<string | null>(null);

  useEffect(() => {
    if (!menuOpenId) {
      return;
    }
    const closeMenu = () => setMenuOpenId(null);
    const handleKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setMenuOpenId(null);
      }
    };
    document.addEventListener("click", closeMenu);
    document.addEventListener("keydown", handleKey);
    return () => {
      document.removeEventListener("click", closeMenu);
      document.removeEventListener("keydown", handleKey);
    };
  }, [menuOpenId]);

  const groupedChats = useMemo(
    () =>
      chats.reduce<{
        today: ChatSession[];
        yesterday: ChatSession[];
        earlier: ChatSession[];
      }>(
        (acc, chat) => {
          const matchesQuery =
            sidebarQuery.trim().length === 0 ||
            chat.title.toLowerCase().includes(sidebarQuery.toLowerCase());

          if (!matchesQuery) {
            return acc;
          }

          const updatedDate = new Date(chat.updated_at);
          const now = new Date();

          const isToday = updatedDate.toDateString() === now.toDateString();

          const yesterday = new Date();
          yesterday.setDate(now.getDate() - 1);
          const isYesterday =
            updatedDate.toDateString() === yesterday.toDateString();

          if (isToday) {
            acc.today.push(chat);
          } else if (isYesterday) {
            acc.yesterday.push(chat);
          } else {
            acc.earlier.push(chat);
          }

          return acc;
        },
        { today: [], yesterday: [], earlier: [] }
      ),
    [chats, sidebarQuery]
  );

  const stopMenuEvent = (event: MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
    event.nativeEvent.stopImmediatePropagation();
  };

  return (
    <aside className="w-full max-w-xs border-r border-[var(--color-border)] bg-[var(--color-surface)] backdrop-blur hidden md:flex flex-col h-full overflow-y-auto">
      <div className="px-6 py-5 border-b border-[var(--color-border)]">
        <h1 className="text-lg font-semibold tracking-tight text-[var(--color-text)]">
          CarlGPT
        </h1>
      </div>
      <div className="p-3 flex flex-col gap-3">
        <button
          type="button"
          onClick={onCreateChat}
          className="w-full rounded-[18px] bg-gradient-to-r from-[#7b36ff] to-[#551ecf] text-white px-3 py-2 font-medium hover:shadow-[var(--shadow-soft)] disabled:opacity-60 transition"
          disabled={isCreating || isLoading}
        >
          {isCreating ? "Création…" : "Nouvelle discussion"}
        </button>
        {error && <p className="text-sm text-[var(--color-danger)]">{error}</p>}
        <input
          type="search"
          placeholder="Rechercher une conversation"
          className="w-full rounded-[18px] bg-[var(--color-surface)] border border-[var(--color-border)] text-[var(--color-text)] px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-[var(--color-primary)] focus:border-transparent"
          value={sidebarQuery}
          onChange={(e) => setSidebarQuery(e.target.value)}
        />
      </div>
      <div className="flex-1 overflow-y-auto px-3 pb-4 space-y-4">
        {isLoading ? (
          <p className="text-sm text-[var(--color-text-muted)] px-2">
            Chargement des chats…
          </p>
        ) : chats.length === 0 ? (
          <p className="text-sm text-[var(--color-text-muted)] px-2">
            Aucune discussion enregistrée.
          </p>
        ) : (
          (["today", "yesterday", "earlier"] as const).map((sectionKey) => {
            const list = groupedChats[sectionKey];

            if (list.length === 0) return null;

            const label =
              sectionKey === "today"
                ? "Aujourd'hui"
                : sectionKey === "yesterday"
                ? "Hier"
                : "Plus tôt";

            return (
              <div key={sectionKey} className="space-y-2">
                <p className="text-xs uppercase tracking-wide text-[var(--color-text-muted)] px-2">
                  {label}
                </p>
                <ul className="space-y-2">
                  {list.map((chat) => {
                    const isCurrent = chat.id === currentChatId;
                    const menuVisible = menuOpenId === chat.id;
                    return (
                      <li key={chat.id} className="relative">
                        <div
                          className={`flex items-center gap-2 [background-image:var(--color-surface-gradient)] rounded-xl border px-3 py-2 transition ${
                            isCurrent
                              ? "border-[var(--color-primary)] bg-[var(--color-surface)]"
                              : "border-[var(--color-border)] hover:border-[var(--color-primary)]"
                          }`}
                        >
                          <button
                            type="button"
                            onClick={() => onSelectChat(chat.id)}
                            className="flex-1 text-left min-w-0"
                          >
                            <p className="font-medium text-sm truncate">
                              {chat.title}
                            </p>
                            <p className="text-[11px] text-[var(--color-text-muted)]">
                              {new Date(chat.updated_at).toLocaleTimeString(
                                "fr-FR",
                                { hour: "2-digit", minute: "2-digit" }
                              )}
                            </p>
                          </button>
                          <button
                            type="button"
                            aria-label="Options de la discussion"
                            onClick={(event) => {
                              stopMenuEvent(event);
                              setMenuOpenId((prev) =>
                                prev === chat.id ? null : chat.id
                              );
                            }}
                            className="px-1 rounded hover:bg-[var(--color-primary-soft)] transition text-[var(--color-text-muted)]"
                            disabled={actionLoadingId === chat.id}
                          >
                            <span aria-hidden="true">⋯</span>
                          </button>
                        </div>
                        {menuVisible && (
                          <div
                            className="absolute right-0 mt-2 w-44 rounded border border-[var(--color-border)] bg-[var(--color)] shadow-lg z-10"
                            onClick={(event) => stopMenuEvent(event)}
                          >
                            <button
                              type="button"
                              className="w-full text-left px-4 py-2 text-sm hover:bg-[var(--color-surface)] disabled:opacity-60"
                              disabled={actionLoadingId === chat.id}
                              onClick={(event) => {
                                stopMenuEvent(event);
                                onArchiveChat(chat.id);
                              }}
                            >
                              {actionLoadingId === chat.id
                                ? "Traitement…"
                                : "Archiver"}
                            </button>
                            <button
                              type="button"
                              className="w-full text-left px-4 py-2 text-sm text-[var(--color-danger)] hover:bg-[var(--color-danger)]/10 disabled:opacity-60"
                              disabled={actionLoadingId === chat.id}
                              onClick={(event) => {
                                stopMenuEvent(event);
                                onDeleteChat(chat.id);
                              }}
                            >
                              {actionLoadingId === chat.id
                                ? "Suppression…"
                                : "Supprimer"}
                            </button>
                          </div>
                        )}
                      </li>
                    );
                  })}
                </ul>
              </div>
            );
          })
        )}
      </div>
    </aside>
  );
}
