import * as React from "react";

import { isRtl, t as translate, type Lang } from "@/lib/i18n";
import type { User } from "@/lib/types";
import { cn } from "@/lib/utils";

type ToastVariant = "default" | "destructive";

interface ToastItem {
  id: number;
  message: string;
  variant: ToastVariant;
}

interface AppCtx {
  lang: Lang;
  setLang: (l: Lang) => void;
  rtl: boolean;
  t: (key: string) => string;
  user: User | null;
  setUser: (u: User | null) => void;
  toast: (message: string, variant?: ToastVariant) => void;
}

const Ctx = React.createContext<AppCtx | null>(null);

export function useApp(): AppCtx {
  const c = React.useContext(Ctx);
  if (!c) throw new Error("useApp must be used inside AppProvider");
  return c;
}

export function AppProvider({ children }: { children: React.ReactNode }) {
  const [lang, setLang] = React.useState<Lang>("en");
  const [user, setUser] = React.useState<User | null>(null);
  const [toasts, setToasts] = React.useState<ToastItem[]>([]);
  const idRef = React.useRef(0);

  const rtl = isRtl(lang);

  React.useEffect(() => {
    document.documentElement.dir = rtl ? "rtl" : "ltr";
    document.documentElement.lang = lang;
  }, [lang, rtl]);

  const toast = React.useCallback(
    (message: string, variant: ToastVariant = "default") => {
      const id = ++idRef.current;
      setToasts((prev) => [...prev, { id, message, variant }]);
      window.setTimeout(
        () => setToasts((prev) => prev.filter((x) => x.id !== id)),
        4000,
      );
    },
    [],
  );

  const value = React.useMemo<AppCtx>(
    () => ({
      lang,
      setLang,
      rtl,
      t: (k: string) => translate(lang, k),
      user,
      setUser,
      toast,
    }),
    [lang, rtl, user, toast],
  );

  return (
    <Ctx.Provider value={value}>
      {children}
      <div className="pointer-events-none fixed bottom-4 end-4 z-[100] flex w-80 flex-col gap-2">
        {toasts.map((x) => (
          <div
            key={x.id}
            className={cn(
              "pointer-events-auto rounded-lg border px-4 py-3 text-sm shadow-lg",
              x.variant === "destructive"
                ? "border-destructive/40 bg-destructive/10 text-destructive"
                : "border-primary/40 bg-primary/10 text-foreground",
            )}
          >
            {x.message}
          </div>
        ))}
      </div>
    </Ctx.Provider>
  );
}
