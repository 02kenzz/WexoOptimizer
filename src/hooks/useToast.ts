import { createContext, useContext, useState, useCallback, ReactNode, createElement } from "react";

interface Toast {
  id: number;
  title: string;
  description?: string;
  tone: "accent" | "ok" | "warn" | "danger";
}

interface ToastContextValue {
  toasts: Toast[];
  showToast: (toast: Omit<Toast, "id">) => void;
  dismissToast: (id: number) => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

// Use createElement to avoid esbuild JSX parsing issues with Context.Provider
function ToastProviderInner({ value, children }: { value: ToastContextValue; children: ReactNode }) {
  return createElement(ToastContext.Provider, { value }, children);
}

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  let nextId = 0;

  const showToast = useCallback((toast: Omit<Toast, "id">) => {
    const id = nextId++;
    setToasts((prev) => [...prev, { ...toast, id }]);
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 4000);
  }, []);

  const dismissToast = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const contextValue: ToastContextValue = { toasts, showToast, dismissToast };

  return createElement(
    ToastProviderInner,
    { value: contextValue },
    createElement(
      "div",
      null,
      children,
      createElement(
        "div",
        { className: "fixed bottom-6 right-6 z-50 flex flex-col gap-2 pointer-events-none" },
        toasts.map((toast) =>
          createElement(
            "div",
            {
              key: toast.id,
              className: `pointer-events-auto flex items-center gap-3 rounded-xl border px-4 py-3 min-w-[280px] max-w-[400px] animate-slide-up ${
                toast.tone === "accent" ? "border-accent/50 bg-accent/10" :
                toast.tone === "ok" ? "border-ok/50 bg-ok/10" :
                toast.tone === "warn" ? "border-warn/50 bg-warn/10" :
                "border-danger/50 bg-danger/10"
              }`,
            },
            createElement(
              "div",
              { className: "flex-1" },
              createElement("p", { className: "text-sm font-medium text-text" }, toast.title),
              toast.description && createElement("p", { className: "text-xs text-mut" }, toast.description)
            ),
            createElement(
              "button",
              {
                onClick: () => dismissToast(toast.id),
                className: "text-mut hover:text-text",
                "aria-label": "Dismiss",
              },
              createElement(
                "svg",
                { className: "size-4", fill: "none", stroke: "currentColor", viewBox: "0 0 24 24" },
                createElement("path", {
                  strokeLinecap: "round",
                  strokeLinejoin: "round",
                  strokeWidth: 2,
                  d: "M6 18L18 6M6 6l12 12",
                })
              )
            )
          )
        )
      )
    )
  );
}

export function useToast() {
  const context = useContext(ToastContext);
  if (!context) {
    throw new Error("useToast must be used within a ToastProvider");
  }
  return context;
}