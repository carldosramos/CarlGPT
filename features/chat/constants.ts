export const MODEL_OPTIONS = [
  { id: "llama-3.1-8b-instant", label: "Llama 3.1 8B (Groq)" },
  { id: "gpt-5.1", label: "GPT-5.1 (OpenAI)" },
  { id: "gpt-5-mini", label: "GPT-5 mini (OpenAI)" },
  { id: "gpt-5-nano", label: "GPT-5 nano (OpenAI)" },
  { id: "gpt-5-pro", label: "GPT-5 pro (OpenAI)" },
  { id: "gpt-5", label: "GPT-5 (OpenAI)" },
  { id: "gpt-4.1", label: "GPT-4.1 (OpenAI)" },
] as const;

export const FILES_OPTIONS = [
  { id: "file", label: "Fichier" },
  { id: "image", label: "Image" },
] as const;

export const MAX_INPUT_LINES = 8;
