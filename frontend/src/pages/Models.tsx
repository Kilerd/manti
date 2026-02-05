import { useState, useEffect } from "react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { listAvailableModels, type AvailableModel } from "@/api";
import { toast } from "@/hooks/use-toast";
import { Wrench, Eye } from "lucide-react";

export default function Models() {
  const [models, setModels] = useState<AvailableModel[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetchModels();
  }, []);

  const fetchModels = async () => {
    try {
      const response = await listAvailableModels({});
      setModels(response.data ?? []);
    } catch (err) {
      console.error("Failed to fetch models:", err);
      toast({
        title: "Error",
        description: "Failed to fetch available models.",
        variant: "destructive",
      });
    } finally {
      setLoading(false);
    }
  };

  const formatCost = (cost: number | null | undefined) => {
    if (cost == null) return "-";
    return `$${cost.toFixed(4)}`;
  };

  const formatContext = (context: number | null | undefined) => {
    if (context == null) return "-";
    if (context >= 1000000) return `${(context / 1000000).toFixed(1)}M`;
    if (context >= 1000) return `${(context / 1000).toFixed(0)}K`;
    return context.toString();
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
        <h1 className="text-3xl font-bold">Available Models</h1>
        <p className="text-muted-foreground">
          Models you can access based on your user groups
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Models</CardTitle>
          <CardDescription>
            {models.length} model{models.length !== 1 ? "s" : ""} available
          </CardDescription>
        </CardHeader>
        <CardContent>
          {models.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              No models available. Contact your administrator to get access.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Model</TableHead>
                  <TableHead>Capabilities</TableHead>
                  <TableHead>Input Cost</TableHead>
                  <TableHead>Output Cost</TableHead>
                  <TableHead>Max Context</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {models.map((model) => (
                  <TableRow key={model.id}>
                    <TableCell>
                      <div>
                        <div className="font-medium font-mono">
                          {model.model_id}
                        </div>
                        {model.display_name && (
                          <div className="text-sm text-muted-foreground">
                            {model.display_name}
                          </div>
                        )}
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="flex gap-1">
                        {model.supports_tools && (
                          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-secondary text-secondary-foreground text-xs font-medium">
                            <Wrench className="h-3 w-3" />
                            Tools
                          </span>
                        )}
                        {model.supports_vision && (
                          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-secondary text-secondary-foreground text-xs font-medium">
                            <Eye className="h-3 w-3" />
                            Vision
                          </span>
                        )}
                        {!model.supports_tools && !model.supports_vision && (
                          <span className="text-muted-foreground text-sm">-</span>
                        )}
                      </div>
                    </TableCell>
                    <TableCell className="font-mono text-sm">
                      {formatCost(model.input_cost_per_1k)}
                      {model.input_cost_per_1k != null && (
                        <span className="text-muted-foreground">/1K</span>
                      )}
                    </TableCell>
                    <TableCell className="font-mono text-sm">
                      {formatCost(model.output_cost_per_1k)}
                      {model.output_cost_per_1k != null && (
                        <span className="text-muted-foreground">/1K</span>
                      )}
                    </TableCell>
                    <TableCell className="font-mono text-sm">
                      {formatContext(model.max_context)}
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
