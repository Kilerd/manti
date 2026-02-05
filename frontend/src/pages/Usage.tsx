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
import { getUsage, getUsageStats, type UsageRecord, type UsageStats } from "@/api";

export default function Usage() {
  const [usageHistory, setUsageHistory] = useState<UsageRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [stats, setStats] = useState<UsageStats>({
    active_keys: 0,
    total_requests: 0,
    total_tokens: 0,
    total_cost: 0,
  });

  useEffect(() => {
    fetchUsageData();
  }, []);

  const fetchUsageData = async () => {
    try {
      const [usageResponse, statsData] = await Promise.all([
        getUsage({}),
        getUsageStats(),
      ]);

      setUsageHistory(usageResponse.data ?? []);
      setStats(statsData ?? {
        active_keys: 0,
        total_requests: 0,
        total_tokens: 0,
        total_cost: 0,
      });
    } catch (err) {
      console.error("Failed to fetch usage data:", err);
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

      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Total Requests</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {(stats.total_requests ?? 0).toLocaleString()}
            </div>
            <p className="text-xs text-muted-foreground">All time</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Total Tokens</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {(stats.total_tokens ?? 0).toLocaleString()}
            </div>
            <p className="text-xs text-muted-foreground">All time</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Total Cost</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              ${(stats.total_cost ?? 0).toFixed(4)}
            </div>
            <p className="text-xs text-muted-foreground">All time</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Active Keys</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {stats.active_keys ?? 0}
            </div>
            <p className="text-xs text-muted-foreground">API keys</p>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Recent Requests</CardTitle>
          <CardDescription>Your recent API requests</CardDescription>
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
                  <TableHead>Provider</TableHead>
                  <TableHead>Prompt Tokens</TableHead>
                  <TableHead>Completion Tokens</TableHead>
                  <TableHead>Total Tokens</TableHead>
                  <TableHead>Cost</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {usageHistory.map((record) => (
                  <TableRow key={record.id}>
                    <TableCell className="text-sm">
                      {new Date(record.created_at).toLocaleString()}
                    </TableCell>
                    <TableCell className="font-mono text-sm">
                      {record.model}
                    </TableCell>
                    <TableCell>{record.provider}</TableCell>
                    <TableCell>{record.prompt_tokens.toLocaleString()}</TableCell>
                    <TableCell>{record.completion_tokens.toLocaleString()}</TableCell>
                    <TableCell>{record.total_tokens.toLocaleString()}</TableCell>
                    <TableCell>${parseFloat(record.cost).toFixed(4)}</TableCell>
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
