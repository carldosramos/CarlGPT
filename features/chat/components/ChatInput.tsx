import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  ChangeEvent,
  MouseEvent as ReactMouseEvent,
} from "react";
import { IoAddOutline } from "react-icons/io5";
import { RiSendPlane2Line } from "react-icons/ri";
import { AiOutlineLoading3Quarters } from "react-icons/ai";
import { PendingAttachment, CompletionParams } from "../types";
import { FILES_OPTIONS, MAX_INPUT_LINES, MODEL_OPTIONS } from "../constants";
import { ModelSettings } from "./ModelSettings";

interface ChatInputProps {
  prompt: string;
  setPrompt: (value: string) => void;
  onSubmit: (e: React.FormEvent) => void;
  isChatBusy: boolean;
  canUseChat: boolean;
  pendingAttachments: PendingAttachment[];
  pendingUploadCount: number;
  onRemoveAttachment: (clientId: string) => void;
  onFilesSelected: (files: FileList | null) => void;
  selectedModelId: string;
  setSelectedModelId: (id: string) => void;
  completionParams: CompletionParams;
  setCompletionParams: (params: CompletionParams) => void;
  chatError: { chatId: string; message: string } | null;
  currentChatId?: string;
}

export function ChatInput({
  prompt,
  setPrompt,
  onSubmit,
  isChatBusy,
  canUseChat,
  pendingAttachments,
  pendingUploadCount,
  onRemoveAttachment,
  onFilesSelected,
  selectedModelId,
  setSelectedModelId,
  completionParams,
  setCompletionParams,
  chatError,
  currentChatId,
}: ChatInputProps) {
  const [isInputExpanded, setIsInputExpanded] = useState(false);
  const [isFilesMenuOpen, setIsFilesMenuOpen] = useState(false);
  const [isModelMenuOpen, setIsModelMenuOpen] = useState(false);

  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const filesMenuRef = useRef<HTMLDivElement | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const imageInputRef = useRef<HTMLInputElement | null>(null);
  const modelPickerRef = useRef<HTMLDivElement | null>(null);
  const isInputExpandedRef = useRef(false);

  const selectedModel = useMemo(
    () =>
      MODEL_OPTIONS.find((option) => option.id === selectedModelId) ??
      MODEL_OPTIONS[0],
    [selectedModelId]
  );

  useEffect(() => {
    if (!isFilesMenuOpen) {
      return;
    }
    const handleClick = (event: MouseEvent) => {
      if (
        filesMenuRef.current &&
        !filesMenuRef.current.contains(event.target as Node)
      ) {
        setIsFilesMenuOpen(false);
      }
    };
    const handleKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setIsFilesMenuOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClick);
    document.addEventListener("keydown", handleKey);
    return () => {
      document.removeEventListener("mousedown", handleClick);
      document.removeEventListener("keydown", handleKey);
    };
  }, [isFilesMenuOpen]);

  useEffect(() => {
    if (!isModelMenuOpen) {
      return;
    }
    const handleClick = (event: MouseEvent) => {
      if (
        modelPickerRef.current &&
        !modelPickerRef.current.contains(event.target as Node)
      ) {
        setIsModelMenuOpen(false);
      }
    };
    const handleKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setIsModelMenuOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClick);
    document.addEventListener("keydown", handleKey);
    return () => {
      document.removeEventListener("mousedown", handleClick);
      document.removeEventListener("keydown", handleKey);
    };
  }, [isModelMenuOpen]);

  useEffect(() => {
    if (pendingAttachments.length > 0 && !isInputExpandedRef.current) {
      isInputExpandedRef.current = true;
      setIsInputExpanded(true);
    }
  }, [pendingAttachments]);

  useEffect(() => {
    isInputExpandedRef.current = isInputExpanded;
  }, [isInputExpanded]);

  const resizeTextarea = useCallback(() => {
    const el = textareaRef.current;
    if (!el) {
      return;
    }

    const style = window.getComputedStyle(el);
    const lineHeight =
      parseFloat(style.lineHeight || style.fontSize || "18") || 18;
    const paddingY =
      parseFloat(style.paddingTop || "0") +
      parseFloat(style.paddingBottom || "0");
    const baseHeight = lineHeight + paddingY;

    el.style.height = "auto";
    el.style.overflowY = "hidden";
    const scrollHeight = el.scrollHeight;

    const text = el.value;
    const trimmed = text.trim();
    if (trimmed.length === 0) {
      el.style.height = `${baseHeight}px`;
      el.style.overflowY = "hidden";
      setIsInputExpanded(false);
      isInputExpandedRef.current = false;
      return;
    }

    const hasManualBreak = text.includes("\n");
    const overflow = scrollHeight - baseHeight;
    if (hasManualBreak || overflow > 2) {
      if (!isInputExpandedRef.current) {
        isInputExpandedRef.current = true;
        setIsInputExpanded(true);
      }
    }

    if (isInputExpandedRef.current || hasManualBreak || overflow > 2) {
      const maxHeight = baseHeight + lineHeight * (MAX_INPUT_LINES - 1);
      const targetHeight = Math.min(
        Math.max(scrollHeight, baseHeight),
        maxHeight
      );
      el.style.height = `${targetHeight}px`;
      el.style.overflowY = scrollHeight > maxHeight ? "auto" : "hidden";
    } else {
      el.style.height = `${baseHeight}px`;
      el.style.overflowY = "hidden";
    }
  }, []);

  useLayoutEffect(() => {
    resizeTextarea();
  }, [prompt, resizeTextarea, isInputExpanded]);

  const handleAddFileClick = useCallback(() => {
    if (isChatBusy || !canUseChat) {
      return;
    }
    fileInputRef.current?.click();
  }, [canUseChat, isChatBusy]);

  const handleAddImageClick = useCallback(() => {
    if (isChatBusy || !canUseChat) {
      return;
    }
    imageInputRef.current?.click();
  }, [canUseChat, isChatBusy]);

  return (
    <div className="sticky bottom-0 left-0 w-full pt-4 px-4 md:px-8 pb-6">
      <form onSubmit={onSubmit} className="space-y-3">
        {chatError && chatError.chatId === currentChatId && (
          <div className="text-sm text-[var(--color-danger)]">
            {chatError.message}
          </div>
        )}
        <div className="border border-[var(--color-border)] rounded-4xl [background-image:var(--color-surface-gradient-muted)] p-3 flex flex-col gap-3">
          <div
            className={`grid gap-x-3 grid-cols-[auto_1fr_auto] ${
              isInputExpanded
                ? "grid-rows-[auto_auto]"
                : "grid-rows-[0px_auto] items-center"
            }`}
          >
            <textarea
              ref={textareaRef}
              placeholder={
                canUseChat ? "Pose ta question…" : "Chargement des discussions…"
              }
              className={`resize-none border-none bg-transparent text-base text-[var(--color-text)] focus:outline-none disabled:bg-[var(--color-surface)] py-2 ${
                isInputExpanded
                  ? "col-start-1 col-end-4 row-start-1"
                  : "col-start-2 row-start-2 w-full"
              }`}
              rows={1}
              value={prompt}
              onChange={(e) => {
                setPrompt(e.target.value);
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  onSubmit(e);
                }
              }}
              disabled={isChatBusy || !canUseChat}
              required
            />
            <div
              ref={filesMenuRef}
              className={`relative col-start-1 ${
                isInputExpanded ? "row-start-2 self-end" : "row-start-2"
              }`}
            >
              <button
                type="button"
                className="text-[var(--color-text)] font-medium inline-flex items-center gap-1 px-3 py-1 rounded-full bg-[var(--color-surface)] border border-[var(--color-border)] hover:border-[var(--color-primary)] transition"
                disabled={isChatBusy || !canUseChat}
                onClick={() => setIsFilesMenuOpen((prev) => !prev)}
              >
                <IoAddOutline className="w-5 h-5" />
              </button>
              {isFilesMenuOpen && (
                <div className="absolute left-0 bottom-0 mt-2 w-56 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface-muted)] shadow-lg z-20 overflow-hidden">
                  {FILES_OPTIONS.map((option) => (
                    <button
                      type="button"
                      key={option.id}
                      className="w-full text-left px-4 py-2 text-sm transition text-[var(--color-text)] hover:bg-[var(--color-surface)]"
                      onClick={() => {
                        setIsFilesMenuOpen(false);
                        if (option.id === "image") {
                          handleAddImageClick();
                        } else {
                          handleAddFileClick();
                        }
                      }}
                    >
                      {option.label}
                    </button>
                  ))}
                </div>
              )}
            </div>

            <button
              type="submit"
              className={`bg-[var(--color-primary)] text-white hover:bg-[var(--color-primary-strong)] disabled:opacity-60 transition ${
                isInputExpanded
                  ? "col-start-3 row-start-2 justify-self-end rounded-2xl px-6 py-2"
                  : "col-start-3 row-start-2 ml-auto rounded-full px-4 py-4"
              }`}
              disabled={
                isChatBusy ||
                !canUseChat ||
                prompt.trim().length === 0 ||
                pendingUploadCount > 0
              }
            >
              {isChatBusy ? (
                <AiOutlineLoading3Quarters className="animate-spin" />
              ) : (
                <RiSendPlane2Line />
              )}
            </button>
          </div>
          {pendingAttachments.length > 0 && (
            <div className="flex flex-wrap gap-2">
              {pendingAttachments.map((attachment) => (
                <span
                  key={attachment.clientId}
                  className="inline-flex items-center gap-2 rounded-full border border-[var(--color-border)] bg-[var(--color-surface)] px-3 py-1 text-xs text-[var(--color-text)]"
                >
                  <span className="truncate max-w-[8rem]">
                    {attachment.file_name}
                  </span>
                  <button
                    type="button"
                    className="text-[var(--color-text-muted)] hover:text-[var(--color-danger)]"
                    onClick={() => onRemoveAttachment(attachment.clientId)}
                  >
                    ✕
                  </button>
                </span>
              ))}
            </div>
          )}
          {pendingUploadCount > 0 && (
            <p className="text-xs text-[var(--color-text-muted)]">
              Upload de fichiers en cours…
            </p>
          )}
        </div>
        <input
          ref={fileInputRef}
          type="file"
          className="hidden"
          multiple
          onChange={(event: ChangeEvent<HTMLInputElement>) => {
            onFilesSelected(event.target.files);
            if (event.target) {
              event.target.value = "";
            }
          }}
        />
        <input
          ref={imageInputRef}
          type="file"
          accept="image/*"
          className="hidden"
          multiple
          onChange={(event: ChangeEvent<HTMLInputElement>) => {
            onFilesSelected(event.target.files);
            if (event.target) {
              event.target.value = "";
            }
          }}
        />
        <div className="text-xs text-[var(--color-text-muted)] flex items-center gap-2">
          <span>Modèle :</span>
          <div ref={modelPickerRef} className="relative">
            <button
              type="button"
              className="text-[var(--color-text)] font-medium inline-flex items-center gap-1 px-3 py-1 rounded-full bg-[var(--color-surface)] border border-[var(--color-border)] hover:border-[var(--color-primary)] transition"
              onClick={() => setIsModelMenuOpen((prev) => !prev)}
            >
              {selectedModel.label}
              <span className="text-[var(--color-text-muted)] text-xs">▾</span>
            </button>
            {isModelMenuOpen && (
              <div className="absolute left-0 bottom-0 mt-2 w-56 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface-muted)] shadow-lg z-20 overflow-hidden">
                {MODEL_OPTIONS.map((option) => (
                  <button
                    type="button"
                    key={option.id}
                    className={`w-full text-left px-4 py-2 text-sm transition ${
                      option.id === selectedModel.id
                        ? "bg-[var(--color-surface)] text-[var(--color-primary)]"
                        : "text-[var(--color-text)] hover:bg-[var(--color-surface-muted)]"
                    }`}
                    onClick={() => {
                      setSelectedModelId(option.id);
                      setIsModelMenuOpen(false);
                    }}
                  >
                    {option.label}
                  </button>
                ))}
              </div>
            )}
          </div>
          <ModelSettings
            params={completionParams}
            onChange={setCompletionParams}
          />
        </div>
      </form>
    </div>
  );
}
