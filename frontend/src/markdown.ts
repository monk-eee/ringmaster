import { createElement, Fragment, type ReactNode } from "react";

// ADR-0092: evidence quotes and reasons are real transcript/meeting text,
// which sometimes already contains **bold** markdown from the source. Left
// un-rendered it reads as literal asterisks -- noisy, not "modern." This
// renders only **bold** segments as <strong>, nothing else (no headings,
// links, lists, or raw HTML) -- returns React nodes directly, never
// dangerouslySetInnerHTML, so there is no HTML-injection surface regardless
// of what the source text contains.
export function renderBoldSegments(text: string): ReactNode {
  const parts = text.split(/(\*\*[^*]+\*\*)/g).filter((part) => part.length > 0);
  if (parts.length <= 1 && !parts[0]?.startsWith("**")) return text;
  return createElement(
    Fragment,
    null,
    ...parts.map((part, index) => {
      if (part.startsWith("**") && part.endsWith("**") && part.length > 4) {
        return createElement("strong", { key: index }, part.slice(2, -2));
      }
      return part;
    }),
  );
}
