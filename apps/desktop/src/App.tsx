import * as React from "react";
import { LogOut } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { useApp } from "@/lib/app-context";
import { api, errorMessage } from "@/lib/api";
import { LANGS, langName, type Lang } from "@/lib/i18n";
import { cn } from "@/lib/utils";
import { InventoryScreen } from "@/screens/Inventory";
import { Login } from "@/screens/Login";
import { ProductsScreen } from "@/screens/Products";
import { Register } from "@/screens/Register";
import { ReportsScreen } from "@/screens/Reports";
import { SettingsScreen } from "@/screens/Settings";
import { Setup } from "@/screens/Setup";
import { UsersScreen } from "@/screens/Users";

type ScreenKey =
  | "register"
  | "products"
  | "inventory"
  | "reports"
  | "settings"
  | "users";

const NAV: { key: ScreenKey; label: string; admin: boolean }[] = [
  { key: "register", label: "register", admin: false },
  { key: "products", label: "products", admin: true },
  { key: "inventory", label: "inventory", admin: true },
  { key: "reports", label: "reports", admin: true },
  { key: "settings", label: "settings", admin: true },
  { key: "users", label: "users", admin: true },
];

export default function App() {
  const { t, user, setUser, lang, setLang, toast } = useApp();
  const [hasUsers, setHasUsers] = React.useState<boolean | null>(null);
  const [screen, setScreen] = React.useState<ScreenKey>(
    (import.meta.env.VITE_INITIAL_SCREEN as ScreenKey | undefined) ?? "register",
  );

  React.useEffect(() => {
    api
      .hasUsers()
      .then(setHasUsers)
      .catch((e) => toast(errorMessage(e), "destructive"));
    // Restore a session that the Rust side still holds (e.g. after a reload).
    api
      .currentUser()
      .then((u) => u && setUser(u))
      .catch(() => {});
    api
      .settings()
      .then((s) => setLang((s.language as Lang) === "ar" ? "ar" : "en"))
      .catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (hasUsers === null) {
    return (
      <div className="flex h-full items-center justify-center text-muted-foreground">
        {t("app")}…
      </div>
    );
  }

  if (!user) {
    return hasUsers ? <Login /> : <Setup />;
  }

  const isAdmin = user.role === "admin";
  const items = NAV.filter((n) => !n.admin || isAdmin);

  return (
    <div className="flex h-full min-h-0 flex-col">
      <header className="flex items-center gap-3 border-b bg-card px-4 py-2">
        <div className="text-lg font-semibold">{t("app")}</div>
        <Separator orientation="vertical" className="mx-1 h-6" />
        <nav className="flex items-center gap-1">
          {items.map((n) => (
            <button
              key={n.key}
              onClick={() => setScreen(n.key)}
              className={cn(
                "rounded-md px-3 py-1.5 text-sm font-medium transition-colors",
                screen === n.key
                  ? "bg-primary text-primary-foreground"
                  : "hover:bg-accent",
              )}
            >
              {t(n.label)}
            </button>
          ))}
        </nav>

        <div className="ms-auto flex items-center gap-2">
          {LANGS.map((l) => (
            <Button
              key={l}
              size="sm"
              variant={lang === l ? "default" : "ghost"}
              onClick={async () => {
                setLang(l);
                try {
                  await api.setLanguage(l);
                } catch {
                  /* non-fatal */
                }
              }}
            >
              {langName(l)}
            </Button>
          ))}
          <Separator orientation="vertical" className="mx-1 h-6" />
          <span className="text-sm">
            <span className="font-medium">{user.username}</span>{" "}
            <span className="text-muted-foreground">
              ({user.role === "admin" ? t("admin") : t("cashier")})
            </span>
          </span>
          <Button
            variant="outline"
            size="sm"
            onClick={async () => {
              try {
                await api.logout();
              } catch {
                /* ignore */
              }
              setUser(null);
              setScreen("register");
            }}
          >
            <LogOut className="size-4" />
            {t("signOut")}
          </Button>
        </div>
      </header>

      <main className="min-h-0 flex-1 overflow-hidden">
        {screen === "register" && <Register />}
        {screen === "products" && <ProductsScreen />}
        {screen === "inventory" && <InventoryScreen />}
        {screen === "reports" && <ReportsScreen />}
        {screen === "settings" && <SettingsScreen />}
        {screen === "users" && <UsersScreen />}
      </main>
    </div>
  );
}
