import { useState, useEffect, type KeyboardEvent } from "react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  listApiKeys,
  createApiKey,
  revokeApiKey,
  getApiKeyStats,
  type ApiKeyListItem,
  type ApiKeyStats,
} from "@/api";
import { toast } from "@/hooks/use-toast";
import { Copy, Trash2, Plus, Eye, EyeOff } from "lucide-react";

export default function ApiKeys() {
  const [apiKeys, setApiKeys] = useState<ApiKeyListItem[]>([]);
  const [keyStats, setKeyStats] = useState<Map<string, ApiKeyStats>>(new Map());
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [newKeyName, setNewKeyName] = useState("");
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [keyToDelete, setKeyToDelete] = useState<{
    id: string;
    name: string;
  } | null>(null);
  const [visibleKeys, setVisibleKeys] = useState<Set<string>>(new Set());

  useEffect(() => {
    fetchApiKeys();
  }, []);

  const fetchApiKeys = async () => {
    try {
      const [keysResponse, statsData] = await Promise.all([
        listApiKeys({}),
        getApiKeyStats().catch(() => []),
      ]);
      setApiKeys(keysResponse.data);

      const statsMap = new Map<string, ApiKeyStats>();
      for (const stat of statsData) {
        statsMap.set(stat.api_key_id, stat);
      }
      setKeyStats(statsMap);
    } catch {
      toast({
        title: "Error",
        description: "Failed to fetch API keys. Please try again.",
        variant: "destructive",
      });
    } finally {
      setLoading(false);
    }
  };

  const handleCreateKey = async () => {
    if (!newKeyName.trim()) return;

    setCreating(true);
    try {
      const response = await createApiKey({ name: newKeyName });
      const data = response.data;

      setApiKeys([...apiKeys, data]);
      setNewKeyName("");
      // Auto-show the new key
      setVisibleKeys((prev) => new Set(prev).add(data.id));

      toast({
        title: "API Key Created",
        description: `Successfully created API key "${data.name}"`,
      });
    } catch (err) {
      toast({
        title: "Error",
        description:
          err instanceof Error
            ? err.message
            : "Failed to create API key. Please try again.",
        variant: "destructive",
      });
    } finally {
      setCreating(false);
    }
  };

  const confirmDeleteKey = (keyId: string, keyName: string) => {
    setKeyToDelete({ id: keyId, name: keyName });
    setDeleteDialogOpen(true);
  };

  const handleDeleteKey = async () => {
    if (!keyToDelete) return;

    try {
      await revokeApiKey({ id: keyToDelete.id });

      setApiKeys(apiKeys.filter((key) => key.id !== keyToDelete.id));

      toast({
        title: "API Key Deleted",
        description: `Successfully deleted API key "${keyToDelete.name}"`,
      });
    } catch (err) {
      toast({
        title: "Error",
        description:
          err instanceof Error
            ? err.message
            : "Failed to delete API key. Please try again.",
        variant: "destructive",
      });
    } finally {
      setDeleteDialogOpen(false);
      setKeyToDelete(null);
    }
  };

  const copyToClipboard = (text: string, keyName: string) => {
    navigator.clipboard
      .writeText(text)
      .then(() => {
        toast({
          title: "Copied",
          description: `API key "${keyName}" copied to clipboard`,
        });
      })
      .catch(() => {
        toast({
          title: "Error",
          description: "Failed to copy to clipboard",
          variant: "destructive",
        });
      });
  };

  const toggleKeyVisibility = (keyId: string) => {
    setVisibleKeys((prev) => {
      const newSet = new Set(prev);
      if (newSet.has(keyId)) {
        newSet.delete(keyId);
      } else {
        newSet.add(keyId);
      }
      return newSet;
    });
  };

  const handleKeyPress = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      handleCreateKey();
    }
  };

  const maskKey = (key: string) => {
    // Show first 12 chars and last 4 chars
    if (key.length <= 20) return key;
    return `${key.slice(0, 12)}...${key.slice(-4)}`;
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
        <h1 className="text-3xl font-bold">API Keys</h1>
        <p className="text-muted-foreground">
          Manage your API keys for accessing the LLM Gateway
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Create New API Key</CardTitle>
          <CardDescription>
            Generate a new API key for your applications
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex space-x-2">
            <div className="flex-1">
              <Label htmlFor="keyName" className="sr-only">
                Key Name
              </Label>
              <Input
                id="keyName"
                placeholder="Enter a name for this key"
                value={newKeyName}
                onChange={(e) => setNewKeyName(e.target.value)}
                onKeyPress={handleKeyPress}
              />
            </div>
            <Button
              onClick={handleCreateKey}
              disabled={creating || !newKeyName.trim()}
            >
              <Plus className="h-4 w-4 mr-2" />
              Create Key
            </Button>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Your API Keys</CardTitle>
          <CardDescription>Active API keys for your account</CardDescription>
        </CardHeader>
        <CardContent>
          {apiKeys.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              No API keys yet. Create one to get started.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Key</TableHead>
                  <TableHead className="text-right">Requests</TableHead>
                  <TableHead className="text-right">Tokens</TableHead>
                  <TableHead className="text-right">Cost</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead>Last Used</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {apiKeys.map((apiKey) => {
                  const stats = keyStats.get(apiKey.id);
                  return (
                    <TableRow key={apiKey.id}>
                      <TableCell className="font-medium">{apiKey.name}</TableCell>
                      <TableCell className="font-mono text-sm">
                        <div className="flex items-center space-x-2">
                          <span className="max-w-[200px] truncate">
                            {visibleKeys.has(apiKey.id)
                              ? apiKey.key
                              : maskKey(apiKey.key)}
                          </span>
                        </div>
                      </TableCell>
                      <TableCell className="text-right tabular-nums">
                        {stats?.total_requests.toLocaleString() ?? "0"}
                      </TableCell>
                      <TableCell className="text-right tabular-nums">
                        {stats?.total_tokens.toLocaleString() ?? "0"}
                      </TableCell>
                      <TableCell className="text-right tabular-nums">
                        ${stats?.total_cost.toFixed(4) ?? "0.0000"}
                      </TableCell>
                      <TableCell>
                        {new Date(apiKey.created_at).toLocaleDateString()}
                      </TableCell>
                      <TableCell>
                        {apiKey.last_used
                          ? new Date(apiKey.last_used).toLocaleDateString()
                          : "Never"}
                      </TableCell>
                      <TableCell className="text-right">
                        <div className="flex justify-end space-x-2">
                          <Button
                            size="icon"
                            variant="ghost"
                            onClick={() => toggleKeyVisibility(apiKey.id)}
                            title={
                              visibleKeys.has(apiKey.id) ? "Hide key" : "Show key"
                            }
                          >
                            {visibleKeys.has(apiKey.id) ? (
                              <EyeOff className="h-4 w-4" />
                            ) : (
                              <Eye className="h-4 w-4" />
                            )}
                          </Button>
                          <Button
                            size="icon"
                            variant="ghost"
                            onClick={() => copyToClipboard(apiKey.key, apiKey.name)}
                            title="Copy to clipboard"
                          >
                            <Copy className="h-4 w-4" />
                          </Button>
                          <Button
                            size="icon"
                            variant="ghost"
                            onClick={() =>
                              confirmDeleteKey(apiKey.id, apiKey.name)
                            }
                            title="Delete key"
                          >
                            <Trash2 className="h-4 w-4" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      <AlertDialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete API Key</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete the API key "{keyToDelete?.name}"?
              This action cannot be undone and any applications using this key
              will stop working.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDeleteKey}
              className="bg-destructive text-destructive-foreground"
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
