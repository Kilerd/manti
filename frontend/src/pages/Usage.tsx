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
import { client, type UsageRecord, type UsageSummary } from "@/api";
import { cn } from "@/lib/utils";

export default function Usage() {
  const [usageHistory, setUsageHistory] = useState<UsageRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [summary, setSummary] = useState<UsageSummary>({
    totalRequests: 0,
    totalTokens: 0,
    totalCost: 0,
  });

  useEffect(() => {
    fetchUsageHistory();
  }, []);

  const fetchUsageHistory = async () => {
    try {
      const { data, error } = await client.GET("/usage/history", {
        params: { query: { limit: 100 } },
      });

      if (error || !data) {
        throw new Error("Failed to fetch usage history");
      }

      setUsageHistory(data.history || []);
      setSummary(
        data.summary || {
          totalRequests: 0,
          totalTokens: 0,
          totalCost: 0,
        }
      );
    } catch (err) {
      console.error("Failed to fetch usage history:", err);
    } finally {
      setLoading(false);
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
      <div>
        <h1 className="text-3xl font-bold">Usage History</h1>
        <p className="text-muted-foreground">
          Track your API usage and costs over time
        </p>
      </div>

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Total Requests</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {summary.totalRequests.toLocaleString()}
            </div>
            <p className="text-xs text-muted-foreground">This month</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Total Tokens</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {summary.totalTokens.toLocaleString()}
            </div>
            <p className="text-xs text-muted-foreground">This month</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Total Cost</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              ${summary.totalCost.toFixed(4)}
            </div>
            <p className="text-xs text-muted-foreground">This month</p>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Recent Requests</CardTitle>
          <CardDescription>Your last 100 API requests</CardDescription>
        </CardHeader>
        <CardContent>
          {usageHistory.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              No usage history yet. Start making API calls to see them here.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Timestamp</TableHead>
                  <TableHead>Model</TableHead>
                  <TableHead>Input Tokens</TableHead>
                  <TableHead>Output Tokens</TableHead>
                  <TableHead>Total Tokens</TableHead>
                  <TableHead>Cost</TableHead>
                  <TableHead>Status</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {usageHistory.map((record) => (
                  <TableRow key={record.id}>
                    <TableCell className="text-sm">
                      {new Date(record.timestamp).toLocaleString()}
                    </TableCell>
                    <TableCell className="font-mono text-sm">
                      {record.model}
                    </TableCell>
                    <TableCell>{record.inputTokens.toLocaleString()}</TableCell>
                    <TableCell>{record.outputTokens.toLocaleString()}</TableCell>
                    <TableCell>{record.totalTokens.toLocaleString()}</TableCell>
                    <TableCell>${record.cost.toFixed(4)}</TableCell>
                    <TableCell>
                      <span
                        className={cn(
                          "px-2 py-1 text-xs rounded-full",
                          record.status === "success"
                            ? "bg-green-100 text-green-700"
                            : "bg-red-100 text-red-700"
                        )}
                      >
                        {record.status}
                      </span>
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
