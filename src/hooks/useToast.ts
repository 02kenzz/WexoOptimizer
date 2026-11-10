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
function ToastProviderInner({ value, children }: { value: ToastContextValue; children?: ReactNode }) {
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
        { className: "toast-stack" },
        toasts.map((toast) =>
          createElement(
            "div",
            {
              key: toast.id,
              className: `toast toast-${toast.tone}`,
              role: "status",
            },
            createElement("span", {
              className: "history-dot",
              style: {
                width: 8,
                height: 8,
                marginTop: 5,
                borderRadius: "50%",
                flex: "none",
                background:
                  toast.tone === "danger"
                    ? "var(--danger)"
                    : toast.tone === "warn"
                      ? "var(--warn)"
                      : "var(--accent)",
                boxShadow: "0 0 12px currentColor",
              },
            }),
            createElement(
              "div",
              { style: { flex: 1, minWidth: 0 } },
              createElement("b", null, toast.title),
              toast.description ? createElement("p", null, toast.description) : null,
            ),
            createElement(
              "button",
              {
                onClick: () => dismissToast(toast.id),
                className: "toast-x",
                "aria-label": "Dismiss",
              },
              createElement(
                "svg",
                {
                  width: 14,
                  height: 14,
                  fill: "none",
                  stroke: "currentColor",
                  viewBox: "0 0 24 24",
                  strokeWidth: 2,
                  strokeLinecap: "round",
                },
                createElement("path", { d: "M6 18L18 6M6 6l12 12" }),
              ),
            ),
          ),
        ),
      ),
    ),
  );
}

export function useToast() {
  const context = useContext(ToastContext);
  if (!context) {
    throw new Error("useToast must be used within a ToastProvider");
  }
  return context;
}
