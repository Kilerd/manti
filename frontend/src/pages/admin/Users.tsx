import { useState, useEffect } from "react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { listUsers, updateUserGroups, getUserBalance, addUserBalance, type AdminUser, type UserBalance } from "@/api";
import { toast } from "@/hooks/use-toast";
import { Pencil, X, Check, Shield, User, Plus } from "lucide-react";

export default function AdminUsers() {
  const [users, setUsers] = useState<AdminUser[]>([]);
  const [balances, setBalances] = useState<Record<string, UserBalance>>({});
  const [loading, setLoading] = useState(true);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editGroups, setEditGroups] = useState("");
  const [saving, setSaving] = useState(false);
  const [topUpUserId, setTopUpUserId] = useState<string | null>(null);
  const [topUpAmount, setTopUpAmount] = useState("");

  useEffect(() => {
    fetchUsers();
  }, []);

  const fetchUsers = async () => {
    try {
      const response = await listUsers({});
      const userList = response.data ?? [];
      setUsers(userList);

      // Fetch balances for all users
      const balancePromises = userList.map(async (user) => {
        try {
          const balance = await getUserBalance(user.id);
          return [user.id, balance] as const;
        } catch {
          return [user.id, null] as const;
        }
      });
      const balanceResults = await Promise.all(balancePromises);
      const balanceMap: Record<string, UserBalance> = {};
      for (const [id, balance] of balanceResults) {
        if (balance) {
          balanceMap[id] = balance;
        }
      }
      setBalances(balanceMap);
    } catch (err) {
      console.error("Failed to fetch users:", err);
      toast({
        title: "Error",
        description: "Failed to fetch users.",
        variant: "destructive",
      });
    } finally {
      setLoading(false);
    }
  };

  const handleEdit = (user: AdminUser) => {
    setEditingId(user.id);
    setEditGroups(user.user_groups.join(", "));
  };

  const handleCancel = () => {
    setEditingId(null);
    setEditGroups("");
  };

  const handleSave = async (userId: string) => {
    setSaving(true);
    try {
      const groups = editGroups
        .split(",")
        .map((g) => g.trim())
        .filter(Boolean);

      await updateUserGroups({
        user_id: userId,
        user_groups: groups,
      });

      toast({ title: "Success", description: "User groups updated." });
      setEditingId(null);
      setEditGroups("");
      fetchUsers();
    } catch (err) {
      console.error("Failed to update user groups:", err);
      toast({
        title: "Error",
        description: "Failed to update user groups.",
        variant: "destructive",
      });
    } finally {
      setSaving(false);
    }
  };

  const handleTopUp = async (userId: string) => {
    if (!topUpAmount || parseFloat(topUpAmount) <= 0) {
      toast({
        title: "Error",
        description: "Please enter a valid amount.",
        variant: "destructive",
      });
      return;
    }

    setSaving(true);
    try {
      const updatedBalance = await addUserBalance(userId, topUpAmount);
      setBalances((prev) => ({ ...prev, [userId]: updatedBalance }));
      toast({
        title: "Success",
        description: `Added $${topUpAmount} to user's balance.`,
      });
      setTopUpUserId(null);
      setTopUpAmount("");
    } catch (err) {
      console.error("Failed to add balance:", err);
      toast({
        title: "Error",
        description: "Failed to add balance.",
        variant: "destructive",
      });
    } finally {
      setSaving(false);
    }
  };

  const cancelTopUp = () => {
    setTopUpUserId(null);
    setTopUpAmount("");
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-muted-foreground">Loading...</div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">User Management</h1>
        <p className="text-muted-foreground">
          View users and manage their group assignments
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Users</CardTitle>
          <CardDescription>
            {users.length} user{users.length !== 1 ? "s" : ""} registered
          </CardDescription>
        </CardHeader>
        <CardContent>
          {users.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              No users found.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>User</TableHead>
                  <TableHead>Email</TableHead>
                  <TableHead>Role</TableHead>
                  <TableHead>Groups</TableHead>
                  <TableHead>Balance</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {users.map((user) => (
                  <TableRow key={user.id}>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        {user.is_admin ? (
                          <Shield className="h-4 w-4 text-yellow-500" />
                        ) : (
                          <User className="h-4 w-4 text-muted-foreground" />
                        )}
                        <span className="font-medium">{user.username}</span>
                      </div>
                    </TableCell>
                    <TableCell className="text-sm">{user.email}</TableCell>
                    <TableCell>
                      {user.is_admin ? (
                        <span className="text-yellow-600 font-medium">Admin</span>
                      ) : (
                        <span className="text-muted-foreground">User</span>
                      )}
                    </TableCell>
                    <TableCell>
                      {editingId === user.id ? (
                        <div className="flex items-center gap-2">
                          <Input
                            value={editGroups}
                            onChange={(e) => setEditGroups(e.target.value)}
                            placeholder="default, premium"
                            className="h-8 w-48"
                          />
                        </div>
                      ) : (
                        <div className="flex flex-wrap gap-1">
                          {user.user_groups.map((group) => (
                            <span
                              key={group}
                              className="text-xs bg-secondary px-2 py-0.5 rounded"
                            >
                              {group}
                            </span>
                          ))}
                          {user.user_groups.length === 0 && (
                            <span className="text-muted-foreground text-sm">
                              No groups
                            </span>
                          )}
                        </div>
                      )}
                    </TableCell>
                    <TableCell>
                      {topUpUserId === user.id ? (
                        <div className="flex items-center gap-2">
                          <Input
                            type="number"
                            step="0.01"
                            min="0"
                            value={topUpAmount}
                            onChange={(e) => setTopUpAmount(e.target.value)}
                            placeholder="0.00"
                            className="h-8 w-24"
                          />
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8"
                            onClick={cancelTopUp}
                            disabled={saving}
                          >
                            <X className="h-4 w-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8 text-green-600"
                            onClick={() => handleTopUp(user.id)}
                            disabled={saving}
                          >
                            <Check className="h-4 w-4" />
                          </Button>
                        </div>
                      ) : (
                        <div className="flex items-center gap-2">
                          <span className="font-mono">
                            ${parseFloat(balances[user.id]?.balance ?? "0").toFixed(2)}
                          </span>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-6 w-6"
                            onClick={() => setTopUpUserId(user.id)}
                            title="Add balance"
                          >
                            <Plus className="h-3 w-3" />
                          </Button>
                        </div>
                      )}
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {new Date(user.created_at).toLocaleDateString()}
                    </TableCell>
                    <TableCell className="text-right">
                      {editingId === user.id ? (
                        <div className="flex justify-end space-x-1">
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8"
                            onClick={handleCancel}
                            disabled={saving}
                          >
                            <X className="h-4 w-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8 text-green-600"
                            onClick={() => handleSave(user.id)}
                            disabled={saving}
                          >
                            <Check className="h-4 w-4" />
                          </Button>
                        </div>
                      ) : (
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8"
                          onClick={() => handleEdit(user)}
                        >
                          <Pencil className="h-4 w-4" />
                        </Button>
                      )}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
