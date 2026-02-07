import { useState, useEffect } from "react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { getUsageStats, getMyBalance, type UsageStats, type UserBalance } from "@/api";
import { toast } from "@/hooks/use-toast";
import {
  BarChart3,
  DollarSign,
  Activity,
  Key,
  RefreshCw,
  AlertCircle,
  Wallet,
  type LucideIcon,
} from "lucide-react";

export default function Dashboard() {
  const [stats, setStats] = useState<UsageStats>({
    total_requests: 0,
    total_tokens: 0,
    total_cost: 0,
    active_keys: 0,
  });
  const [balance, setBalance] = useState<UserBalance | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);

  useEffect(() => {
    fetchData();
  }, []);

  const fetchData = async (isRefresh = false) => {
    if (isRefresh) {
      setRefreshing(true);
    } else {
      setLoading(true);
    }
    setError(null);

    try {
      const [statsData, balanceData] = await Promise.all([
        getUsageStats(),
        getMyBalance().catch(() => null),
      ]);

      setStats(statsData ?? {
        total_requests: 0,
        total_tokens: 0,
        total_cost: 0,
        active_keys: 0,
      });
      setBalance(balanceData);

      if (isRefresh) {
        toast({
          title: "Stats Refreshed",
          description: "Dashboard statistics have been updated.",
        });
      }
    } catch (err) {
      const errorMessage =
        err instanceof Error
          ? err.message
          : "Failed to fetch statistics. Please try again.";
      setError(errorMessage);

      toast({
        title: "Error Loading Stats",
        description: errorMessage,
        variant: "destructive",
      });
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  };

  const handleRefresh = () => {
    fetchData(true);
  };

  const statCards: {
    title: string;
    value: string | number;
    icon: LucideIcon;
    description: string;
  }[] = [
    {
      title: "Total Requests",
      value: (stats.total_requests ?? 0).toLocaleString(),
      icon: BarChart3,
      description: "API calls",
    },
    {
      title: "Tokens Used",
      value: (stats.total_tokens ?? 0).toLocaleString(),
      icon: Activity,
      description: "Total tokens consumed",
    },
    {
      title: "Total Cost",
      value: `$${(stats.total_cost ?? 0).toFixed(2)}`,
      icon: DollarSign,
      description: "Usage cost",
    },
    {
      title: "Active API Keys",
      value: stats.active_keys ?? 0,
      icon: Key,
      description: "Currently active keys",
    },
  ];

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-muted-foreground">Loading dashboard...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-64 space-y-4">
        <div className="flex items-center gap-2 text-destructive">
          <AlertCircle className="h-5 w-5" />
          <span>Failed to load dashboard</span>
        </div>
        <p className="text-sm text-muted-foreground">{error}</p>
        <Button onClick={() => fetchData(false)} variant="outline">
          <RefreshCw className="h-4 w-4 mr-2" />
          Try Again
        </Button>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Dashboard</h1>
          <p className="text-muted-foreground">
            Monitor your LLM API usage and costs
          </p>
        </div>
        <Button
          onClick={handleRefresh}
          disabled={refreshing}
          variant="outline"
          size="sm"
        >
          <RefreshCw
            className={`h-4 w-4 mr-2 ${refreshing ? "animate-spin" : ""}`}
          />
          Refresh
        </Button>
      </div>

      {balance && (
        <Card className="bg-gradient-to-r from-primary/10 to-primary/5 border-primary/20">
          <CardHeader className="pb-2">
            <div className="flex items-center gap-2">
              <Wallet className="h-5 w-5 text-primary" />
              <CardTitle className="text-lg">Account Balance</CardTitle>
            </div>
          </CardHeader>
          <CardContent>
            <div className="grid gap-4 md:grid-cols-3">
              <div>
                <p className="text-sm text-muted-foreground">Current Balance</p>
                <p className="text-2xl font-bold text-primary">
                  ${parseFloat(balance.balance).toFixed(2)}
                </p>
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Credit Limit</p>
                <p className="text-2xl font-bold">
                  ${parseFloat(balance.credit_limit).toFixed(2)}
                </p>
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Lifetime Usage</p>
                <p className="text-2xl font-bold">
                  ${parseFloat(balance.lifetime_usage).toFixed(2)}
                </p>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        {statCards.map((stat, index) => (
          <Card key={index}>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">{stat.title}</CardTitle>
              <stat.icon className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">{stat.value}</div>
              <p className="text-xs text-muted-foreground">{stat.description}</p>
            </CardContent>
          </Card>
        ))}
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Recent Activity</CardTitle>
          <CardDescription>Your API usage over the last 7 days</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="h-64 flex items-center justify-center text-muted-foreground">
            <div className="text-center space-y-2">
              <BarChart3 className="h-12 w-12 mx-auto opacity-20" />
              <p>Usage chart will be displayed here</p>
              <p className="text-xs">Chart visualization coming soon</p>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
