import * as React from "react";
import { KeyRound, Plus } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useApp } from "@/lib/app-context";
import { api, errorMessage } from "@/lib/api";
import type { Role, User } from "@/lib/types";

export function UsersScreen() {
  const { t, toast } = useApp();
  const [users, setUsers] = React.useState<User[]>([]);
  const [addOpen, setAddOpen] = React.useState(false);
  const [username, setUsername] = React.useState("");
  const [pin, setPin] = React.useState("");
  const [role, setRole] = React.useState<Role>("cashier");
  const [reset, setReset] = React.useState<User | null>(null);
  const [newPin, setNewPin] = React.useState("");

  const load = React.useCallback(async () => {
    try {
      setUsers(await api.listUsers());
    } catch (e) {
      toast(errorMessage(e), "destructive");
    }
  }, [toast]);

  React.useEffect(() => {
    void load();
  }, [load]);

  const create = async () => {
    if (!username.trim()) return toast(t("required"), "destructive");
    try {
      await api.createUser(username.trim(), pin, role);
      setAddOpen(false);
      setUsername("");
      setPin("");
      setRole("cashier");
      await load();
      toast(t("saved"));
    } catch (e) {
      toast(errorMessage(e), "destructive");
    }
  };

  const savePin = async () => {
    if (!reset) return;
    try {
      await api.resetPin(reset.id, newPin);
      setReset(null);
      setNewPin("");
      toast(t("saved"));
    } catch (e) {
      toast(errorMessage(e), "destructive");
    }
  };

  return (
    <div className="flex h-full flex-col gap-3 p-4">
      <div className="flex items-center gap-3">
        <h2 className="text-lg font-semibold">{t("users")}</h2>
        <Badge variant="secondary">{users.length}</Badge>
        <Button className="ms-auto" onClick={() => setAddOpen(true)}>
          <Plus className="size-4" />
          {t("addUser")}
        </Button>
      </div>

      <div className="min-h-0 flex-1 overflow-auto rounded-lg border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("username")}</TableHead>
              <TableHead>{t("role")}</TableHead>
              <TableHead>{t("active")}</TableHead>
              <TableHead className="w-64" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {users.map((u) => (
              <TableRow key={u.id}>
                <TableCell className="font-medium">{u.username}</TableCell>
                <TableCell>
                  <Badge variant={u.role === "admin" ? "default" : "secondary"}>
                    {u.role === "admin" ? t("admin") : t("cashier")}
                  </Badge>
                </TableCell>
                <TableCell>
                  <Badge variant={u.isActive ? "success" : "outline"}>
                    {u.isActive ? t("yes") : t("no")}
                  </Badge>
                </TableCell>
                <TableCell>
                  <div className="flex gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        setReset(u);
                        setNewPin("");
                      }}
                    >
                      <KeyRound className="size-3.5" />
                      {t("resetPin")}
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={async () => {
                        try {
                          await api.setUserActive(u.id, !u.isActive);
                          await load();
                        } catch (e) {
                          toast(errorMessage(e), "destructive");
                        }
                      }}
                    >
                      {u.isActive ? t("deactivate") : t("activate")}
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      <Dialog open={addOpen} onOpenChange={setAddOpen}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>{t("addUser")}</DialogTitle>
          </DialogHeader>
          <div className="flex flex-col gap-4">
            <div className="flex flex-col gap-2">
              <Label>{t("username")}</Label>
              <Input
                autoFocus
                value={username}
                onChange={(e) => setUsername(e.target.value)}
              />
            </div>
            <div className="flex flex-col gap-2">
              <Label>{t("pin")}</Label>
              <Input
                type="password"
                inputMode="numeric"
                value={pin}
                onChange={(e) => setPin(e.target.value)}
              />
            </div>
            <div className="flex flex-col gap-2">
              <Label>{t("role")}</Label>
              <Select value={role} onValueChange={(v) => setRole(v as Role)}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="cashier">{t("cashier")}</SelectItem>
                  <SelectItem value="admin">{t("admin")}</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setAddOpen(false)}>
              {t("cancel")}
            </Button>
            <Button onClick={create}>{t("create")}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={reset !== null} onOpenChange={(o) => !o && setReset(null)}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>
              {t("resetPin")} · {reset?.username}
            </DialogTitle>
          </DialogHeader>
          <div className="flex flex-col gap-2">
            <Label>{t("newPin")}</Label>
            <Input
              autoFocus
              type="password"
              inputMode="numeric"
              value={newPin}
              onChange={(e) => setNewPin(e.target.value)}
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setReset(null)}>
              {t("cancel")}
            </Button>
            <Button onClick={savePin}>{t("save")}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
