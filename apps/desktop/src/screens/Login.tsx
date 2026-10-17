import * as React from "react";
import { Delete, LogIn } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useApp } from "@/lib/app-context";
import { api, errorMessage } from "@/lib/api";
import type { User } from "@/lib/types";
import { cn } from "@/lib/utils";

const PAD = [
  ["1", "2", "3"],
  ["4", "5", "6"],
  ["7", "8", "9"],
  ["", "0", ""],
];

export function Login() {
  const { t, setUser, toast } = useApp();
  const [users, setUsers] = React.useState<User[]>([]);
  const [username, setUsername] = React.useState("");
  const [pin, setPin] = React.useState("");
  const [busy, setBusy] = React.useState(false);

  React.useEffect(() => {
    api
      .loginUsers()
      .then((list) => {
        setUsers(list);
        if (list.length) setUsername((u) => u || list[0].username);
      })
      .catch((e) => toast(errorMessage(e), "destructive"));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const submit = async () => {
    if (!username) return toast(t("required"), "destructive");
    setBusy(true);
    try {
      const u = await api.login(username, pin);
      setUser(u);
      setPin("");
    } catch {
      toast(t("wrongCredentials"), "destructive");
      setPin("");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex h-full items-center justify-center p-6">
      <Card className="w-full max-w-sm">
        <CardHeader className="items-center">
          <CardTitle className="text-2xl">{t("app")}</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col items-center gap-4">
          {users.length > 1 && (
            <div className="flex flex-wrap justify-center gap-2">
              {users.map((u) => (
                <button
                  key={u.id}
                  onClick={() => {
                    setUsername(u.username);
                    setPin("");
                  }}
                  className={cn(
                    "rounded-full border px-3 py-1 text-sm transition-colors",
                    username === u.username
                      ? "border-primary bg-primary/10 text-primary"
                      : "hover:bg-accent",
                  )}
                >
                  {u.username}
                </button>
              ))}
            </div>
          )}

          <div className="text-sm text-muted-foreground">{username}</div>

          <div className="flex h-9 items-center gap-1 text-2xl tracking-[0.4em]">
            {pin ? "*".repeat(pin.length) : <span className="text-muted-foreground">—</span>}
          </div>

          <div className="grid grid-cols-3 gap-2">
            {PAD.flat().map((label, i) =>
              label === "" ? (
                <div key={i} />
              ) : (
                <Button
                  key={i}
                  variant="outline"
                  className="size-16 text-xl"
                  onClick={() => setPin((p) => (p.length < 12 ? p + label : p))}
                >
                  {label}
                </Button>
              ),
            )}
          </div>

          <div className="grid w-full grid-cols-2 gap-2">
            <Button variant="secondary" onClick={() => setPin("")}>
              <Delete className="size-4" />
              {t("clear")}
            </Button>
            <Button onClick={submit} disabled={busy}>
              <LogIn className="size-4" />
              {t("enter")}
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
