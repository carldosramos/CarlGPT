"use client";

import { Children, isValidElement, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import rehypeKatex from "rehype-katex";
import "katex/dist/katex.min.css";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { oneDark } from "react-syntax-highlighter/dist/esm/styles/prism";

type MarkdownRendererProps = {
  content: string;
};

type CodeProps = React.DetailedHTMLProps<
  React.HTMLAttributes<HTMLElement>,
  HTMLElement
> & {
  inline?: boolean;
  className?: string;
  children?: React.ReactNode;
};

const InlineCode = ({ children }: { children: React.ReactNode }) => (
  <code className="rounded bg-[var(--color-surface-muted)] px-2 py-1 text-[var(--color-text)]">
    {children}
  </code>
);

const CodeBlock = ({ inline, className, children }: CodeProps) => {
  const [copied, setCopied] = useState(false);

  const language =
    className?.replace("language-", "")?.toLowerCase() || "plaintext";
  const rawCode = String(children ?? "");
  const normalizedCode = rawCode.replace(/\n$/, "");

  if (inline) {
    return <InlineCode>{children}</InlineCode>;
  }

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(normalizedCode);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      console.error("Impossible de copier :", error);
    }
  };

  const isPlainText = language === "plaintext" || language === "text";

  if (isPlainText) {
    return (
      <em
        className="text-[var(--color-text)] italic whitespace-pre-wrap bg-[var(--color-surface-muted)] py-1 px-2 rounded-sm"
        data-code-block
      >
        {normalizedCode}
      </em>
    );
  }

  return (
    <div
      className="rounded-2xl border border-[var(--color-border)] bg-[#0d1117] overflow-hidden"
      data-code-block
    >
      <div className="flex items-center justify-between px-4 py-2 text-xs uppercase tracking-wide text-gray-400 bg-[#141821] border-b border-white/5 font-mono">
        <span>{language}</span>
        <button
          type="button"
          onClick={handleCopy}
          className="text-gray-300 hover:text-white transition text-[11px]"
        >
          {copied ? "Copié !" : "Copier"}
        </button>
      </div>
      <SyntaxHighlighter
        language={language}
        style={oneDark}
        PreTag="div"
        customStyle={{
          margin: 0,
          borderRadius: 0,
          background: "transparent",
          padding: "1.25rem",
          fontSize: "0.9rem",
          lineHeight: 1.6,
        }}
        codeTagProps={{
          style: { fontFamily: "var(--font-geist-mono), monospace" },
        }}
      >
        {normalizedCode}
      </SyntaxHighlighter>
    </div>
  );
};

export default function MarkdownRenderer({ content }: MarkdownRendererProps) {
  return (
    <div className="markdown-body">
      <ReactMarkdown
        remarkPlugins={[remarkMath, remarkGfm]}
        rehypePlugins={[
          [
            rehypeKatex,
            { output: "html", strict: false, trust: true, throwOnError: false },
          ],
        ]}
        components={{
          code: CodeBlock,

          // TITRES
          h1: ({ children }) => (
            <h1 className="text-2xl md:text-3xl font-semibold tracking-tight text-[var(--color-text)] mb-4 mt-1">
              {children}
            </h1>
          ),
          h2: ({ children }) => (
            <h2 className="text-xl md:text-2xl font-semibold text-[var(--color-text)] mt-4 mb-2">
              {children}
            </h2>
          ),
          h3: ({ children }) => (
            <h3 className="text-lg md:text-xl font-semibold text-[var(--color-text)] mt-3 mb-2">
              {children}
            </h3>
          ),

          // PARAGRAPHES
          p: ({ children }) => {
            const childArray = Children.toArray(children);
            const containsBlock = childArray.some((child) => {
              if (!isValidElement(child)) {
                return false;
              }

              if (child.type === CodeBlock) {
                return true;
              }

              if (typeof child.type === "string") {
                return ["div", "table", "pre", "code", "ol", "ul"].includes(
                  child.type
                );
              }

              return Boolean(
                (child.props as Record<string, unknown>)?.["data-code-block"]
              );
            });

            if (containsBlock) {
              return (
                <div className="leading-[1.9] text-[var(--color-text)] my-3 space-y-3">
                  {children}
                </div>
              );
            }

            return (
              <p className="leading-[1.9] text-[var(--color-text)] my-3">
                {children}
              </p>
            );
          },

          // LISTES
          ul: ({ children }) => (
            <ul className="list-disc pl-6 space-y-2 text-[var(--color-text)] leading-[1.9]">
              {children}
            </ul>
          ),
          ol: ({ children }) => (
            <ol className="list-decimal pl-6 space-y-2 text-[var(--color-text)] leading-[1.9]">
              {children}
            </ol>
          ),

          // CITATIONS
          blockquote: ({ children }) => (
            <blockquote className="border-l-4 border-[var(--color-border)] pl-4 text-[var(--color-text-muted)] italic leading-[1.9] my-4">
              {children}
            </blockquote>
          ),

          // TABLES
          table: ({ children }) => (
            <div className="overflow-x-auto my-4">
              <table className="w-full border border-[var(--color-border)] text-sm">
                {children}
              </table>
            </div>
          ),
          th: ({ children }) => (
            <th className="border border-[var(--color-border)] bg-[var(--color-surface-muted)] px-3 py-2 text-left text-[var(--color-text)]">
              {children}
            </th>
          ),
          td: ({ children }) => (
            <td className="border border-[var(--color-border)] px-3 py-2 text-[var(--color-text-muted)] leading-[1.8]">
              {children}
            </td>
          ),
        }}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}
