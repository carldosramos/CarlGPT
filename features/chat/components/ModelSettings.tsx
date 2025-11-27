import { useState } from "react";
import { IoSettingsOutline } from "react-icons/io5";
import { CompletionParams } from "../types";

interface ModelSettingsProps {
  params: CompletionParams;
  onChange: (params: CompletionParams) => void;
}

export function ModelSettings({ params, onChange }: ModelSettingsProps) {
  const [isOpen, setIsOpen] = useState(false);

  const handleChange = (
    field: keyof CompletionParams,
    value: number | undefined
  ) => {
    onChange({
      ...params,
      [field]: value,
    });
  };

  return (
    <div className="relative">
      <button
        type="button"
        className="text-[var(--color-text)] font-medium inline-flex items-center gap-1 px-3 py-1 rounded-full bg-[var(--color-surface)] border border-[var(--color-border)] hover:border-[var(--color-primary)] transition"
        onClick={() => setIsOpen(!isOpen)}
      >
        <IoSettingsOutline className="w-4 h-4" />
        <span className="text-xs">Paramètres</span>
      </button>

      {isOpen && (
        <div className="absolute left-0 bottom-full mb-2 w-80 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface-muted)] shadow-lg z-30 p-4 space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-semibold text-[var(--color-text)]">
              Paramètres du modèle
            </h3>
            <button
              type="button"
              className="text-[var(--color-text-muted)] hover:text-[var(--color-text)]"
              onClick={() => setIsOpen(false)}
            >
              ✕
            </button>
          </div>

          {/* Temperature */}
          <div className="space-y-1">
            <div className="flex items-center justify-between">
              <label className="text-xs text-[var(--color-text-muted)]">
                Temperature
              </label>
              <span className="text-xs text-[var(--color-text)] font-mono">
                {params.temperature?.toFixed(1) ?? "0.7"}
              </span>
            </div>
            <input
              type="range"
              min="0"
              max="2"
              step="0.1"
              value={params.temperature ?? 0.7}
              onChange={(e) =>
                handleChange("temperature", parseFloat(e.target.value))
              }
              className="w-full accent-[var(--color-primary)]"
            />
            <p className="text-[10px] text-[var(--color-text-muted)]">
              Contrôle la créativité (0 = déterministe, 2 = très varié)
            </p>
          </div>

          {/* Max Tokens */}
          <div className="space-y-1">
            <div className="flex items-center justify-between">
              <label className="text-xs text-[var(--color-text-muted)]">
                Max Tokens
              </label>
              <span className="text-xs text-[var(--color-text)] font-mono">
                {params.max_tokens ?? "auto"}
              </span>
            </div>
            <input
              type="range"
              min="0"
              max="4000"
              step="100"
              value={params.max_tokens ?? 0}
              onChange={(e) => {
                const val = parseInt(e.target.value);
                handleChange("max_tokens", val === 0 ? undefined : val);
              }}
              className="w-full accent-[var(--color-primary)]"
            />
            <p className="text-[10px] text-[var(--color-text-muted)]">
              Limite de longueur de réponse (0 = automatique)
            </p>
          </div>

          {/* Top P */}
          <div className="space-y-1">
            <div className="flex items-center justify-between">
              <label className="text-xs text-[var(--color-text-muted)]">
                Top P
              </label>
              <span className="text-xs text-[var(--color-text)] font-mono">
                {params.top_p?.toFixed(2) ?? "1.00"}
              </span>
            </div>
            <input
              type="range"
              min="0"
              max="1"
              step="0.05"
              value={params.top_p ?? 1.0}
              onChange={(e) =>
                handleChange("top_p", parseFloat(e.target.value))
              }
              className="w-full accent-[var(--color-primary)]"
            />
            <p className="text-[10px] text-[var(--color-text-muted)]">
              Échantillonnage nucleus (1.0 = désactivé)
            </p>
          </div>

          {/* Presence Penalty */}
          <div className="space-y-1">
            <div className="flex items-center justify-between">
              <label className="text-xs text-[var(--color-text-muted)]">
                Presence Penalty
              </label>
              <span className="text-xs text-[var(--color-text)] font-mono">
                {params.presence_penalty?.toFixed(1) ?? "0.0"}
              </span>
            </div>
            <input
              type="range"
              min="-2"
              max="2"
              step="0.1"
              value={params.presence_penalty ?? 0.0}
              onChange={(e) =>
                handleChange("presence_penalty", parseFloat(e.target.value))
              }
              className="w-full accent-[var(--color-primary)]"
            />
            <p className="text-[10px] text-[var(--color-text-muted)]">
              Encourage nouveaux sujets (positif = plus de nouveauté)
            </p>
          </div>

          {/* Frequency Penalty */}
          <div className="space-y-1">
            <div className="flex items-center justify-between">
              <label className="text-xs text-[var(--color-text-muted)]">
                Frequency Penalty
              </label>
              <span className="text-xs text-[var(--color-text)] font-mono">
                {params.frequency_penalty?.toFixed(1) ?? "0.0"}
              </span>
            </div>
            <input
              type="range"
              min="-2"
              max="2"
              step="0.1"
              value={params.frequency_penalty ?? 0.0}
              onChange={(e) =>
                handleChange("frequency_penalty", parseFloat(e.target.value))
              }
              className="w-full accent-[var(--color-primary)]"
            />
            <p className="text-[10px] text-[var(--color-text-muted)]">
              Réduit les répétitions (positif = moins de répétition)
            </p>
          </div>

          {/* Reset Button */}
          <button
            type="button"
            className="w-full px-3 py-2 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] text-xs text-[var(--color-text)] hover:bg-[var(--color-surface-muted)] transition"
            onClick={() => {
              onChange({
                temperature: 0.7,
                max_tokens: undefined,
                top_p: 1.0,
                presence_penalty: 0.0,
                frequency_penalty: 0.0,
              });
            }}
          >
            Réinitialiser valeurs par défaut
          </button>
        </div>
      )}
    </div>
  );
}
