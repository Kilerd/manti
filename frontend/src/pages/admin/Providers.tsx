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
  listAllProviders,
  createProvider,
  updateProvider,
  deleteProvider,
  listProviderModels,
  createModel,
  deleteModel,
  type ProviderConfig,
  type ModelInfo,
} from "@/api";
import { toast } from "@/hooks/use-toast";
import {
  Plus,
  Pencil,
  Trash2,
  ChevronDown,
  ChevronRight,
  Server,
  X,
} from "lucide-react";

const PROVIDER_TYPES = ["openai", "anthropic", "google"];

interface ProviderFormData {
  name: string;
  provider_type: string;
  api_key: string;
  base_url: string;
  priority: number;
  rate_limit: string;
  monthly_quota: string;
  allowed_groups: string;
}

const emptyFormData: ProviderFormData = {
  name: "",
  provider_type: "openai",
  api_key: "",
  base_url: "",
  priority: 0,
  rate_limit: "",
  monthly_quota: "",
  allowed_groups: "",
};

interface ModelFormData {
  model_id: string;
  display_name: string;
  input_cost_per_1k: string;
  output_cost_per_1k: string;
  max_context: string;
  supports_tools: boolean;
  supports_vision: boolean;
}

const emptyModelFormData: ModelFormData = {
  model_id: "",
  display_name: "",
  input_cost_per_1k: "",
  output_cost_per_1k: "",
  max_context: "",
  supports_tools: false,
  supports_vision: false,
};

export default function AdminProviders() {
  const [providers, setProviders] = useState<ProviderConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [formData, setFormData] = useState<ProviderFormData>(emptyFormData);
  const [saving, setSaving] = useState(false);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [providerToDelete, setProviderToDelete] = useState<ProviderConfig | null>(null);

  // Models state
  const [expandedProvider, setExpandedProvider] = useState<string | null>(null);
  const [models, setModels] = useState<Record<string, ModelInfo[]>>({});
  const [showModelForm, setShowModelForm] = useState(false);
  const [modelFormData, setModelFormData] = useState<ModelFormData>(emptyModelFormData);
  const [savingModel, setSavingModel] = useState(false);

  useEffect(() => {
    fetchProviders();
  }, []);

  const fetchProviders = async () => {
    try {
      const response = await listAllProviders({});
      setProviders(response.data ?? []);
    } catch (err) {
      console.error("Failed to fetch providers:", err);
      toast({
        title: "Error",
        description: "Failed to fetch providers.",
        variant: "destructive",
      });
    } finally {
      setLoading(false);
    }
  };

  const fetchModels = async (providerId: string) => {
    try {
      const response = await listProviderModels({ provider_id: providerId });
      setModels((prev) => ({ ...prev, [providerId]: response.data ?? [] }));
    } catch (err) {
      console.error("Failed to fetch models:", err);
    }
  };

  const toggleExpand = (providerId: string) => {
    if (expandedProvider === providerId) {
      setExpandedProvider(null);
      setShowModelForm(false);
    } else {
      setExpandedProvider(providerId);
      setShowModelForm(false);
      if (!models[providerId]) {
        fetchModels(providerId);
      }
    }
  };

  const handleCreate = () => {
    setEditingId(null);
    setFormData(emptyFormData);
    setShowForm(true);
  };

  const handleEdit = (provider: ProviderConfig) => {
    setEditingId(provider.id);
    setFormData({
      name: provider.name,
      provider_type: provider.provider_type,
      api_key: "", // Don't show existing key
      base_url: provider.base_url ?? "",
      priority: provider.priority,
      rate_limit: provider.rate_limit?.toString() ?? "",
      monthly_quota: provider.monthly_quota?.toString() ?? "",
      allowed_groups: provider.allowed_groups.join(", "),
    });
    setShowForm(true);
  };

  const handleSubmit = async () => {
    if (!formData.name || !formData.provider_type) {
      toast({
        title: "Validation Error",
        description: "Name and provider type are required.",
        variant: "destructive",
      });
      return;
    }

    if (!editingId && !formData.api_key) {
      toast({
        title: "Validation Error",
        description: "API key is required for new providers.",
        variant: "destructive",
      });
      return;
    }

    setSaving(true);
    try {
      const allowedGroups = formData.allowed_groups
        .split(",")
        .map((g) => g.trim())
        .filter(Boolean);

      if (editingId) {
        await updateProvider({
          id: editingId,
          name: formData.name,
          api_key: formData.api_key || undefined,
          base_url: formData.base_url || undefined,
          priority: formData.priority,
          rate_limit: formData.rate_limit ? parseInt(formData.rate_limit) : undefined,
          monthly_quota: formData.monthly_quota || undefined,
          allowed_groups: allowedGroups,
          is_active: true,
        });
        toast({ title: "Success", description: "Provider updated successfully." });
      } else {
        await createProvider({
          name: formData.name,
          provider_type: formData.provider_type,
          api_key: formData.api_key,
          base_url: formData.base_url || undefined,
          priority: formData.priority,
          rate_limit: formData.rate_limit ? parseInt(formData.rate_limit) : undefined,
          monthly_quota: formData.monthly_quota || undefined,
          allowed_groups: allowedGroups.length > 0 ? allowedGroups : undefined,
        });
        toast({ title: "Success", description: "Provider created successfully." });
      }

      setShowForm(false);
      setEditingId(null);
      setFormData(emptyFormData);
      fetchProviders();
    } catch (err) {
      console.error("Failed to save provider:", err);
      toast({
        title: "Error",
        description: err instanceof Error ? err.message : "Failed to save provider.",
        variant: "destructive",
      });
    } finally {
      setSaving(false);
    }
  };

  const confirmDelete = (provider: ProviderConfig) => {
    setProviderToDelete(provider);
    setDeleteDialogOpen(true);
  };

  const handleDelete = async () => {
    if (!providerToDelete) return;

    try {
      await deleteProvider({ id: providerToDelete.id });
      toast({ title: "Success", description: "Provider deleted successfully." });
      fetchProviders();
    } catch (err) {
      console.error("Failed to delete provider:", err);
      toast({
        title: "Error",
        description: "Failed to delete provider.",
        variant: "destructive",
      });
    } finally {
      setDeleteDialogOpen(false);
      setProviderToDelete(null);
    }
  };

  const handleAddModel = () => {
    setModelFormData(emptyModelFormData);
    setShowModelForm(true);
  };

  const handleSubmitModel = async () => {
    if (!expandedProvider || !modelFormData.model_id) {
      toast({
        title: "Validation Error",
        description: "Model ID is required.",
        variant: "destructive",
      });
      return;
    }

    setSavingModel(true);
    try {
      await createModel({
        provider_id: expandedProvider,
        model_id: modelFormData.model_id,
        display_name: modelFormData.display_name || undefined,
        input_cost_per_1k: modelFormData.input_cost_per_1k || undefined,
        output_cost_per_1k: modelFormData.output_cost_per_1k || undefined,
        max_context: modelFormData.max_context
          ? parseInt(modelFormData.max_context)
          : undefined,
        supports_tools: modelFormData.supports_tools,
        supports_vision: modelFormData.supports_vision,
      });
      toast({ title: "Success", description: "Model created successfully." });
      setShowModelForm(false);
      setModelFormData(emptyModelFormData);
      fetchModels(expandedProvider);
    } catch (err) {
      console.error("Failed to create model:", err);
      toast({
        title: "Error",
        description: "Failed to create model.",
        variant: "destructive",
      });
    } finally {
      setSavingModel(false);
    }
  };

  const handleDeleteModel = async (modelId: string) => {
    try {
      await deleteModel({ id: modelId });
      toast({ title: "Success", description: "Model deleted successfully." });
      if (expandedProvider) {
        fetchModels(expandedProvider);
      }
    } catch (err) {
      console.error("Failed to delete model:", err);
      toast({
        title: "Error",
        description: "Failed to delete model.",
        variant: "destructive",
      });
    }
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
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Provider Management</h1>
          <p className="text-muted-foreground">
            Manage LLM provider configurations
          </p>
        </div>
        <Button onClick={handleCreate}>
          <Plus className="h-4 w-4 mr-2" />
          Add Provider
        </Button>
      </div>

      {showForm && (
        <Card>
          <CardHeader>
            <CardTitle>{editingId ? "Edit Provider" : "Add Provider"}</CardTitle>
            <CardDescription>
              {editingId
                ? "Update provider configuration"
                : "Configure a new LLM provider"}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="name">Name *</Label>
                <Input
                  id="name"
                  value={formData.name}
                  onChange={(e) =>
                    setFormData({ ...formData, name: e.target.value })
                  }
                  placeholder="My OpenAI Provider"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="provider_type">Provider Type *</Label>
                <select
                  id="provider_type"
                  value={formData.provider_type}
                  onChange={(e) =>
                    setFormData({ ...formData, provider_type: e.target.value })
                  }
                  disabled={!!editingId}
                  className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                >
                  {PROVIDER_TYPES.map((type) => (
                    <option key={type} value={type}>
                      {type}
                    </option>
                  ))}
                </select>
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="api_key">
                API Key {editingId ? "(leave empty to keep existing)" : "*"}
              </Label>
              <Input
                id="api_key"
                type="password"
                value={formData.api_key}
                onChange={(e) =>
                  setFormData({ ...formData, api_key: e.target.value })
                }
                placeholder="sk-..."
              />
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="base_url">Base URL (optional)</Label>
                <Input
                  id="base_url"
                  value={formData.base_url}
                  onChange={(e) =>
                    setFormData({ ...formData, base_url: e.target.value })
                  }
                  placeholder="https://api.openai.com/v1"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="priority">Priority</Label>
                <Input
                  id="priority"
                  type="number"
                  value={formData.priority}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      priority: parseInt(e.target.value) || 0,
                    })
                  }
                />
              </div>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="rate_limit">Rate Limit (RPM)</Label>
                <Input
                  id="rate_limit"
                  type="number"
                  value={formData.rate_limit}
                  onChange={(e) =>
                    setFormData({ ...formData, rate_limit: e.target.value })
                  }
                  placeholder="60"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="monthly_quota">Monthly Quota ($)</Label>
                <Input
                  id="monthly_quota"
                  type="number"
                  step="0.01"
                  value={formData.monthly_quota}
                  onChange={(e) =>
                    setFormData({ ...formData, monthly_quota: e.target.value })
                  }
                  placeholder="100.00"
                />
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="allowed_groups">
                Allowed Groups (comma-separated, empty = public)
              </Label>
              <Input
                id="allowed_groups"
                value={formData.allowed_groups}
                onChange={(e) =>
                  setFormData({ ...formData, allowed_groups: e.target.value })
                }
                placeholder="default, premium"
              />
            </div>

            <div className="flex justify-end space-x-2 pt-4">
              <Button
                variant="outline"
                onClick={() => {
                  setShowForm(false);
                  setEditingId(null);
                  setFormData(emptyFormData);
                }}
              >
                Cancel
              </Button>
              <Button onClick={handleSubmit} disabled={saving}>
                {saving ? "Saving..." : editingId ? "Update" : "Create"}
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardHeader>
          <CardTitle>Providers</CardTitle>
          <CardDescription>
            {providers.length} provider{providers.length !== 1 ? "s" : ""}{" "}
            configured
          </CardDescription>
        </CardHeader>
        <CardContent>
          {providers.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              No providers configured. Add one to get started.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-8"></TableHead>
                  <TableHead>Name</TableHead>
                  <TableHead>Type</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Priority</TableHead>
                  <TableHead>Allowed Groups</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {providers.map((provider) => (
                  <>
                    <TableRow key={provider.id}>
                      <TableCell>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-6 w-6"
                          onClick={() => toggleExpand(provider.id)}
                        >
                          {expandedProvider === provider.id ? (
                            <ChevronDown className="h-4 w-4" />
                          ) : (
                            <ChevronRight className="h-4 w-4" />
                          )}
                        </Button>
                      </TableCell>
                      <TableCell className="font-medium">
                        <div className="flex items-center gap-2">
                          <Server className="h-4 w-4 text-muted-foreground" />
                          {provider.name}
                        </div>
                      </TableCell>
                      <TableCell>
                        <span className="font-mono text-sm">
                          {provider.provider_type}
                        </span>
                      </TableCell>
                      <TableCell>
                        {provider.is_active ? (
                          <span className="text-green-600">Active</span>
                        ) : (
                          <span className="text-red-600">Inactive</span>
                        )}
                      </TableCell>
                      <TableCell>{provider.priority}</TableCell>
                      <TableCell>
                        {provider.allowed_groups.length === 0 ? (
                          <span className="text-muted-foreground">Public</span>
                        ) : (
                          <span className="text-sm">
                            {provider.allowed_groups.join(", ")}
                          </span>
                        )}
                      </TableCell>
                      <TableCell className="text-right">
                        <div className="flex justify-end space-x-1">
                          <Button
                            variant="ghost"
                            size="icon"
                            onClick={() => handleEdit(provider)}
                          >
                            <Pencil className="h-4 w-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            onClick={() => confirmDelete(provider)}
                          >
                            <Trash2 className="h-4 w-4" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                    {expandedProvider === provider.id && (
                      <TableRow key={`${provider.id}-models`}>
                        <TableCell colSpan={7} className="bg-muted/50">
                          <div className="p-4 space-y-4">
                            <div className="flex items-center justify-between">
                              <h4 className="font-medium">Models</h4>
                              <Button
                                size="sm"
                                variant="outline"
                                onClick={handleAddModel}
                              >
                                <Plus className="h-3 w-3 mr-1" />
                                Add Model
                              </Button>
                            </div>

                            {showModelForm && (
                              <Card>
                                <CardContent className="pt-4 space-y-4">
                                  <div className="grid grid-cols-2 gap-4">
                                    <div className="space-y-2">
                                      <Label>Model ID *</Label>
                                      <Input
                                        value={modelFormData.model_id}
                                        onChange={(e) =>
                                          setModelFormData({
                                            ...modelFormData,
                                            model_id: e.target.value,
                                          })
                                        }
                                        placeholder="gpt-4o"
                                      />
                                    </div>
                                    <div className="space-y-2">
                                      <Label>Display Name</Label>
                                      <Input
                                        value={modelFormData.display_name}
                                        onChange={(e) =>
                                          setModelFormData({
                                            ...modelFormData,
                                            display_name: e.target.value,
                                          })
                                        }
                                        placeholder="GPT-4o"
                                      />
                                    </div>
                                  </div>
                                  <div className="grid grid-cols-3 gap-4">
                                    <div className="space-y-2">
                                      <Label>Input Cost (/1K)</Label>
                                      <Input
                                        type="number"
                                        step="0.0001"
                                        value={modelFormData.input_cost_per_1k}
                                        onChange={(e) =>
                                          setModelFormData({
                                            ...modelFormData,
                                            input_cost_per_1k: e.target.value,
                                          })
                                        }
                                        placeholder="0.0025"
                                      />
                                    </div>
                                    <div className="space-y-2">
                                      <Label>Output Cost (/1K)</Label>
                                      <Input
                                        type="number"
                                        step="0.0001"
                                        value={modelFormData.output_cost_per_1k}
                                        onChange={(e) =>
                                          setModelFormData({
                                            ...modelFormData,
                                            output_cost_per_1k: e.target.value,
                                          })
                                        }
                                        placeholder="0.01"
                                      />
                                    </div>
                                    <div className="space-y-2">
                                      <Label>Max Context</Label>
                                      <Input
                                        type="number"
                                        value={modelFormData.max_context}
                                        onChange={(e) =>
                                          setModelFormData({
                                            ...modelFormData,
                                            max_context: e.target.value,
                                          })
                                        }
                                        placeholder="128000"
                                      />
                                    </div>
                                  </div>
                                  <div className="flex items-center gap-6">
                                    <label className="flex items-center gap-2">
                                      <input
                                        type="checkbox"
                                        checked={modelFormData.supports_tools}
                                        onChange={(e) =>
                                          setModelFormData({
                                            ...modelFormData,
                                            supports_tools: e.target.checked,
                                          })
                                        }
                                      />
                                      Supports Tools
                                    </label>
                                    <label className="flex items-center gap-2">
                                      <input
                                        type="checkbox"
                                        checked={modelFormData.supports_vision}
                                        onChange={(e) =>
                                          setModelFormData({
                                            ...modelFormData,
                                            supports_vision: e.target.checked,
                                          })
                                        }
                                      />
                                      Supports Vision
                                    </label>
                                  </div>
                                  <div className="flex justify-end space-x-2">
                                    <Button
                                      variant="outline"
                                      size="sm"
                                      onClick={() => setShowModelForm(false)}
                                    >
                                      Cancel
                                    </Button>
                                    <Button
                                      size="sm"
                                      onClick={handleSubmitModel}
                                      disabled={savingModel}
                                    >
                                      {savingModel ? "Saving..." : "Add Model"}
                                    </Button>
                                  </div>
                                </CardContent>
                              </Card>
                            )}

                            {models[provider.id]?.length === 0 ? (
                              <div className="text-sm text-muted-foreground py-4 text-center">
                                No models configured for this provider.
                              </div>
                            ) : (
                              <Table>
                                <TableHeader>
                                  <TableRow>
                                    <TableHead>Model ID</TableHead>
                                    <TableHead>Display Name</TableHead>
                                    <TableHead>Capabilities</TableHead>
                                    <TableHead>Costs</TableHead>
                                    <TableHead className="text-right">
                                      Actions
                                    </TableHead>
                                  </TableRow>
                                </TableHeader>
                                <TableBody>
                                  {models[provider.id]?.map((model) => (
                                    <TableRow key={model.id}>
                                      <TableCell className="font-mono text-sm">
                                        {model.model_id}
                                      </TableCell>
                                      <TableCell>
                                        {model.display_name || "-"}
                                      </TableCell>
                                      <TableCell>
                                        <div className="flex gap-1">
                                          {model.supports_tools && (
                                            <span className="text-xs bg-secondary px-1.5 py-0.5 rounded">
                                              Tools
                                            </span>
                                          )}
                                          {model.supports_vision && (
                                            <span className="text-xs bg-secondary px-1.5 py-0.5 rounded">
                                              Vision
                                            </span>
                                          )}
                                        </div>
                                      </TableCell>
                                      <TableCell className="text-sm">
                                        {model.input_cost_per_1k != null && (
                                          <span>
                                            ${model.input_cost_per_1k}/
                                            ${model.output_cost_per_1k}
                                          </span>
                                        )}
                                      </TableCell>
                                      <TableCell className="text-right">
                                        <Button
                                          variant="ghost"
                                          size="icon"
                                          className="h-7 w-7"
                                          onClick={() =>
                                            handleDeleteModel(model.id)
                                          }
                                        >
                                          <X className="h-3 w-3" />
                                        </Button>
                                      </TableCell>
                                    </TableRow>
                                  ))}
                                </TableBody>
                              </Table>
                            )}
                          </div>
                        </TableCell>
                      </TableRow>
                    )}
                  </>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      <AlertDialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Provider</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete "{providerToDelete?.name}"? This
              will also delete all models associated with this provider. This
              action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDelete}
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
