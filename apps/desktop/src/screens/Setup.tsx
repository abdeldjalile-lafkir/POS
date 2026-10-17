import * as React from "react";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useApp } from "@/lib/app-context";
import { api, errorMessage } from "@/lib/api";

export function Setup() {
  const { t, setUser, toast } = useApp();
  const [username, setUsername] = React.useState("admin");
  const [pin, setPin] = React.useState("");
  const [pin2, setPin2] = React.useState("");
  const [busy, setBusy] = React.useState(false);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!username.trim()) return toast(t("required"), "destructive");
    if (pin !== pin2) return toast(t("pinMismatch"), "destructive");
    setBusy(true);
    try {
      const u = await api.bootstrapAdmin(username.trim(), pin);
      setUser(u);
      toast(t("saved"));
    } catch (err) {
      toast(errorMessage(err), "destructive");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex h-full items-center justify-center p-6">
      <Card className="w-full max-w-md">
        <CardHeader>
          <CardTitle className="text-2xl">{t("firstRun")}</CardTitle>
          <CardDescription>{t("createAdmin")}</CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={submit} className="flex flex-col gap-4">
            <div className="flex flex-col gap-2">
              <Label htmlFor="username">{t("username")}</Label>
              <Input
                id="username"
                value={username}
                autoFocus
                onChange={(e) => setUsername(e.target.value)}
              />
            </div>
            <div className="flex flex-col gap-2">
              <Label htmlFor="pin">{t("pin")}</Label>
              <Input
                id="pin"
                type="password"
                inputMode="numeric"
                value={pin}
                onChange={(e) => setPin(e.target.value)}
              />
            </div>
            <div className="flex flex-col gap-2">
              <Label htmlFor="pin2">{t("confirmPin")}</Label>
              <Input
                id="pin2"
                type="password"
                inputMode="numeric"
                value={pin2}
                onChange={(e) => setPin2(e.target.value)}
              />
              <p className="text-xs text-muted-foreground">{t("choosePin")}</p>
            </div>
            <Button type="submit" size="lg" disabled={busy}>
              {t("create")}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
