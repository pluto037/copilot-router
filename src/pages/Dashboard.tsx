import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { open } from "@tauri-apps/plugin-shell";
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  PieChart,
  Pie,
  Cell,
  ResponsiveContainer,
} from "recharts";
import { tauriApi } from "@/lib/tauri";
import { cn, formatNumber } from "@/lib/utils";
import {
  Zap,
  Hash,
  Clock,
  TrendingUp,
  ShieldCheck,
  ShieldX,
  RefreshCw,
  Moon,
  Sparkles,
  Github,
} from "lucide-react";

const PIE_COLORS = ["#3b82f6", "#10b981", "#f59e0b", "#6366f1", "#ef4444"];
type AuthHintType = "info" | "success" | "error";

export default function Dashboard() {
  const queryClient = useQueryClient();
  const [theme, setTheme] = useState<"default" | "midnight">(() => {
    if (typeof document === "undefined") return "default";
    return document.documentElement.dataset.theme === "midnight" ? "midnight" : "default";
  });
  const [authHint, setAuthHint] = useState("");
  const [authHintType, setAuthHintType] = useState<AuthHintType>("info");

  const { data: stats = [] } = useQuery({
    queryKey: ["usageStats", 14],
    queryFn: () => tauriApi.getUsageStats(14),
    refetchInterval: 10000,
  });

  const { data: proxyStatus } = useQuery({
    queryKey: ["proxyStatus"],
    queryFn: () => tauriApi.getProxyStatus(),
    refetchInterval: 5000,
  });

  const { data: config } = useQuery({
    queryKey: ["config"],
    queryFn: () => tauriApi.getConfig(),
  });

  const { data: tokenStatus } = useQuery({
    queryKey: ["tokenStatus"],
    queryFn: () => tauriApi.getTokenStatus(),
    refetchInterval: 5000,
  });

  const { data: takeoverStatus } = useQuery({
    queryKey: ["claudeTakeoverStatus"],
    queryFn: () => tauriApi.getClaudeTakeoverStatus(),
    refetchInterval: 5000,
  });

  const toggleProxyMutation = useMutation({
    mutationFn: async (enabled: boolean) => {
      if (!config) return;
      await tauriApi.saveConfig({
        ...config,
        proxy_enabled: enabled,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["config"] });
      queryClient.invalidateQueries({ queryKey: ["proxyStatus"] });
      queryClient.invalidateQueries({ queryKey: ["claudeTakeoverStatus"] });
    },
  });

  const refreshTokenMutation = useMutation({
    mutationFn: () => tauriApi.refreshToken(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["tokenStatus"] });
      queryClient.invalidateQueries({ queryKey: ["proxyStatus"] });
      queryClient.invalidateQueries({ queryKey: ["claudeTakeoverStatus"] });
    },
  });

  const repairTakeoverMutation = useMutation({
    mutationFn: () => tauriApi.repairClaudeTakeover(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["claudeTakeoverStatus"] });
      queryClient.invalidateQueries({ queryKey: ["config"] });
    },
  });

  const githubAuthMutation = useMutation({
    mutationFn: async () => {
      const info = await tauriApi.requestGithubDeviceCode();
      setAuthHintType("info");
      setAuthHint(`验证码 ${info.user_code} 已复制，等待浏览器授权...`);
      await tauriApi.copyToClipboard(info.user_code);
      await open(info.verification_uri).catch(() => {});
      return tauriApi.waitGithubDeviceToken(info.device_code);
    },
    onSuccess: () => {
      setAuthHintType("success");
      setAuthHint("GitHub 认证成功");
      queryClient.invalidateQueries({ queryKey: ["tokenStatus"] });
      queryClient.invalidateQueries({ queryKey: ["config"] });
      queryClient.invalidateQueries({ queryKey: ["proxyStatus"] });
      queryClient.invalidateQueries({ queryKey: ["claudeTakeoverStatus"] });
      setTimeout(() => setAuthHint(""), 3000);
    },
    onError: (error) => {
      setAuthHintType("error");
      setAuthHint(`认证失败: ${String(error)}`);
      setTimeout(() => setAuthHint(""), 5000);
    },
  });

  const { data: copilotUsage } = useQuery({
    queryKey: ["copilotUsageOverview"],
    queryFn: () => tauriApi.getCopilotUsageOverview(),
    refetchInterval: 15000,
  });

  const applyTheme = (nextTheme: "default" | "midnight") => {
    setTheme(nextTheme);
    document.documentElement.dataset.theme = nextTheme;
    localStorage.setItem("copilot-router-theme", nextTheme);
  };

  const totalTokens = stats.reduce((s, r) => s + r.total_tokens, 0);
  const totalRequests = stats.reduce((s, r) => s + r.request_count, 0);

  // Model distribution
  const modelMap = new Map<string, number>();
  for (const s of stats) {
    modelMap.set(s.model, (modelMap.get(s.model) ?? 0) + s.request_count);
  }
  const modelData = Array.from(modelMap.entries()).map(([name, value]) => ({
    name,
    value,
  }));

  // Daily chart data (deduplicate by date, take last 7 days)
  const dateMap = new Map<string, { tokens: number; requests: number }>();
  for (const s of stats) {
    const existing = dateMap.get(s.date) ?? { tokens: 0, requests: 0 };
    dateMap.set(s.date, {
      tokens: existing.tokens + s.total_tokens,
      requests: existing.requests + s.request_count,
    });
  }
  const chartData = Array.from(dateMap.entries())
    .sort(([a], [b]) => a.localeCompare(b))
    .slice(-7)
    .map(([date, { tokens, requests }]) => ({
      date: date.slice(5), // MM-DD
      tokens,
      requests,
    }));

  const statCards = [
    {
      label: "今日请求",
      value: formatNumber(proxyStatus?.requests_today ?? 0),
      icon: Zap,
      color: "text-blue-400",
    },
    {
      label: "累计请求 (14d)",
      value: formatNumber(totalRequests),
      icon: Hash,
      color: "text-green-400",
    },
    {
      label: "累计 Tokens (14d)",
      value: formatNumber(totalTokens),
      icon: TrendingUp,
      color: "text-purple-400",
    },
    {
      label: "代理端口",
      value: String(proxyStatus?.port ?? 3100),
      icon: Clock,
      color: "text-yellow-400",
    },
  ];

  return (
    <div className="h-full overflow-y-auto p-4">
      {authHint && (
        <div className="pointer-events-none fixed right-4 top-4 z-50">
          <div
            className={cn(
              "max-w-sm rounded-md border px-3 py-2 text-xs shadow-md",
              authHintType === "success" && "border-green-500/30 bg-green-500/10 text-green-500",
              authHintType === "error" && "border-red-500/30 bg-red-500/10 text-red-500",
              authHintType === "info" && "border-border bg-card text-foreground"
            )}
          >
            {authHint}
          </div>
        </div>
      )}

      {/* Stat cards */}
      <div className="mb-4 rounded-xl border border-border bg-card p-4">
        <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
          <div className="space-y-2">
            <div>
            <p className="text-sm font-medium text-foreground">代理状态</p>
            <p className="mt-1 text-xs text-muted-foreground">
              {config?.proxy_enabled
                ? `已开启 · ${proxyStatus?.running ? "运行中" : "启动中"} · http://127.0.0.1:${proxyStatus?.port ?? config?.proxy_port ?? 3100}`
                : "已关闭 · 请求会返回 503"}
            </p>
            </div>

            <div className="flex flex-wrap items-center gap-2 text-xs">
              <span
                className={cn(
                  "inline-flex items-center gap-1 rounded-md border px-2 py-1",
                  tokenStatus?.is_valid
                    ? "border-green-500/30 text-green-400"
                    : "border-red-500/30 text-red-400"
                )}
              >
                {tokenStatus?.is_valid ? (
                  <ShieldCheck className="h-3.5 w-3.5" />
                ) : (
                  <ShieldX className="h-3.5 w-3.5" />
                )}
                {tokenStatus?.is_valid ? "Token 有效" : "Token 无效"}
              </span>

              <span className="rounded-md border border-border px-2 py-1 text-foreground">
                来源: {tokenStatus?.token_source ?? "未知"}
              </span>

              <span className="rounded-md border border-border px-2 py-1 text-foreground">
                过期: {tokenStatus?.expires_at ?? "未知"}
              </span>

              <span
                className={cn(
                  "rounded-md border px-2 py-1",
                  takeoverStatus?.using_local_proxy
                    ? "border-green-500/30 text-green-400"
                    : "border-yellow-500/30 text-yellow-300"
                )}
              >
                Claude 接管: {takeoverStatus?.using_local_proxy ? "已命中本地代理" : "未命中"}
              </span>

              <span
                className={cn(
                  "rounded-md border px-2 py-1",
                  tokenStatus?.is_valid
                    ? "border-green-500/30 text-green-400"
                    : "border-yellow-500/30 text-yellow-300"
                )}
              >
                GitHub 认证: {tokenStatus?.is_valid ? "已认证" : "未认证"}
              </span>
            </div>

            <div className="space-y-1 text-[11px] text-muted-foreground">
              <p>
                配置文件: <span className="font-mono text-foreground">{takeoverStatus?.settings_path ?? "~/.claude/settings.json"}</span>
              </p>
              <p>
                ANTHROPIC_BASE_URL: <span className="font-mono text-foreground">{takeoverStatus?.anthropic_base_url ?? "(未设置)"}</span>
              </p>
            </div>
          </div>

          <div className="flex w-full flex-col gap-2 md:w-auto md:items-end">
            <div className="flex flex-wrap items-center gap-2">
              <div className="flex items-center rounded-md border border-border bg-secondary p-1">
                <button
                  type="button"
                  onClick={() => applyTheme("default")}
                  className={cn(
                    "inline-flex items-center gap-1 rounded px-2 py-1 text-xs transition-colors",
                    theme === "default"
                      ? "bg-primary text-primary-foreground"
                      : "text-muted-foreground hover:text-foreground"
                  )}
                >
                  <Sparkles className="h-3.5 w-3.5" />
                  白天
                </button>
                <button
                  type="button"
                  onClick={() => applyTheme("midnight")}
                  className={cn(
                    "inline-flex items-center gap-1 rounded px-2 py-1 text-xs transition-colors",
                    theme === "midnight"
                      ? "bg-primary text-primary-foreground"
                      : "text-muted-foreground hover:text-foreground"
                  )}
                >
                  <Moon className="h-3.5 w-3.5" />
                  晚上
                </button>
              </div>

              <button
                type="button"
                disabled={!config || toggleProxyMutation.isPending}
                onClick={() => toggleProxyMutation.mutate(!(config?.proxy_enabled ?? false))}
                className={cn(
                  "rounded-md px-3 py-1.5 text-xs font-medium transition-colors disabled:opacity-50",
                  config?.proxy_enabled
                    ? "bg-green-500/20 text-green-400 hover:bg-green-500/30"
                    : "bg-primary text-primary-foreground hover:bg-primary/90"
                )}
              >
                {toggleProxyMutation.isPending
                  ? "更新中..."
                  : config?.proxy_enabled
                    ? "关闭代理"
                    : "开启代理"}
              </button>
            </div>

            <div className="flex flex-wrap items-center gap-2">
              <button
                type="button"
                disabled={githubAuthMutation.isPending}
                onClick={() => githubAuthMutation.mutate()}
                className={cn(
                  "inline-flex items-center gap-1 rounded-md border px-3 py-1.5 text-xs transition-colors disabled:opacity-50",
                  tokenStatus?.is_valid
                    ? "border-green-500/30 bg-green-500/10 text-green-400 hover:bg-green-500/20"
                    : "border-border bg-secondary text-foreground hover:bg-secondary/80"
                )}
              >
                <Github className="h-3.5 w-3.5" />
                {githubAuthMutation.isPending
                  ? "认证中..."
                  : tokenStatus?.is_valid
                    ? "重新认证 GitHub"
                    : "GitHub 认证"}
              </button>

              <button
                type="button"
                disabled={refreshTokenMutation.isPending}
                onClick={() => refreshTokenMutation.mutate()}
                className="inline-flex items-center gap-1 rounded-md border border-border bg-secondary px-3 py-1.5 text-xs text-foreground transition-colors hover:bg-secondary/80 disabled:opacity-50"
              >
                <RefreshCw className={cn("h-3.5 w-3.5", refreshTokenMutation.isPending && "animate-spin")} />
                刷新认证
              </button>

              <button
                type="button"
                disabled={repairTakeoverMutation.isPending}
                onClick={() => repairTakeoverMutation.mutate()}
                className={cn(
                  "inline-flex items-center gap-1 rounded-md border px-3 py-1.5 text-xs transition-colors disabled:opacity-50",
                  takeoverStatus?.using_local_proxy
                    ? "border-border bg-secondary text-foreground hover:bg-secondary/80"
                    : "border-yellow-500/30 bg-yellow-500/10 text-yellow-300 hover:bg-yellow-500/20"
                )}
              >
                <RefreshCw className={cn("h-3.5 w-3.5", repairTakeoverMutation.isPending && "animate-spin")} />
                {repairTakeoverMutation.isPending ? "修复中..." : "一键修复接管"}
              </button>
            </div>
          </div>
        </div>
      </div>

      <div className="mb-4 grid grid-cols-2 gap-3 lg:grid-cols-4">
        {statCards.map(({ label, value, icon: Icon, color }) => (
          <div
            key={label}
            className="rounded-xl border border-border bg-card p-4"
          >
            <div className="flex items-center justify-between">
              <p className="text-xs text-muted-foreground">{label}</p>
              <Icon className={`h-4 w-4 ${color}`} />
            </div>
            <p className="mt-2 text-2xl font-bold text-foreground">{value}</p>
          </div>
        ))}
      </div>

      {/* Charts */}
      <div className="grid grid-cols-1 gap-3 lg:grid-cols-3">
        {/* Token trend */}
        <div className="rounded-xl border border-border bg-card p-4 lg:col-span-2">
          <p className="mb-3 text-sm font-medium text-foreground">
            Token 消耗趋势（最近 7 天）
          </p>
          <ResponsiveContainer width="100%" height={200}>
            <AreaChart data={chartData}>
              <defs>
                <linearGradient id="tokenGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#3b82f6" stopOpacity={0.3} />
                  <stop offset="95%" stopColor="#3b82f6" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
              <XAxis
                dataKey="date"
                tick={{ fill: "hsl(var(--muted-foreground))", fontSize: 11 }}
                axisLine={false}
                tickLine={false}
              />
              <YAxis
                tick={{ fill: "hsl(var(--muted-foreground))", fontSize: 11 }}
                axisLine={false}
                tickLine={false}
                tickFormatter={(v: number) => formatNumber(v)}
              />
              <Tooltip
                contentStyle={{
                  background: "hsl(var(--card))",
                  border: "1px solid hsl(var(--border))",
                  borderRadius: "8px",
                  fontSize: "12px",
                }}
                formatter={(v: number) => [formatNumber(v), "Tokens"]}
              />
              <Area
                type="monotone"
                dataKey="tokens"
                stroke="#3b82f6"
                strokeWidth={2}
                fill="url(#tokenGrad)"
              />
            </AreaChart>
          </ResponsiveContainer>
        </div>

        {/* Model distribution */}
        <div className="rounded-xl border border-border bg-card p-4">
          <p className="mb-3 text-sm font-medium text-foreground">
            模型分布
          </p>
          {modelData.length === 0 ? (
            <div className="flex h-[200px] items-center justify-center text-xs text-muted-foreground">
              暂无数据
            </div>
          ) : (
            <>
              <ResponsiveContainer width="100%" height={150}>
                <PieChart>
                  <Pie
                    data={modelData}
                    cx="50%"
                    cy="50%"
                    innerRadius={40}
                    outerRadius={65}
                    paddingAngle={3}
                    dataKey="value"
                  >
                    {modelData.map((_, index) => (
                      <Cell
                        key={index}
                        fill={PIE_COLORS[index % PIE_COLORS.length]}
                      />
                    ))}
                  </Pie>
                  <Tooltip
                    contentStyle={{
                      background: "hsl(var(--card))",
                      border: "1px solid hsl(var(--border))",
                      borderRadius: "8px",
                      fontSize: "12px",
                    }}
                  />
                </PieChart>
              </ResponsiveContainer>
              <div className="mt-2 space-y-1">
                {modelData.slice(0, 4).map(({ name, value }, i) => (
                  <div key={name} className="flex items-center gap-2 text-xs">
                    <div
                      className="h-2 w-2 shrink-0 rounded-full"
                      style={{ background: PIE_COLORS[i % PIE_COLORS.length] }}
                    />
                    <span className="flex-1 truncate text-muted-foreground">
                      {name}
                    </span>
                    <span className="font-medium">{value}</span>
                  </div>
                ))}
              </div>
            </>
          )}
        </div>
      </div>

      <div className="mt-4 rounded-xl border border-border bg-card p-4">
        <div className="mb-3 flex items-center justify-between">
          <p className="text-sm font-medium text-foreground">Copilot 使用量</p>
          {copilotUsage?.remote_source && (
            <span className="text-xs text-muted-foreground">
              来源: {copilotUsage.remote_source}
            </span>
          )}
        </div>

        <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
          <MetricItem
            label="近 7 天请求"
            value={formatNumber(copilotUsage?.requests_7d ?? 0)}
          />
          <MetricItem
            label="近 7 天 Tokens"
            value={formatNumber(copilotUsage?.tokens_7d ?? 0)}
          />
          <MetricItem
            label="成功率"
            value={`${(copilotUsage?.success_rate_7d ?? 0).toFixed(1)}%`}
          />
          <MetricItem
            label="平均延迟"
            value={`${copilotUsage?.avg_latency_ms_7d ?? 0} ms`}
          />
        </div>

        <div className="mt-3 flex flex-wrap items-center gap-3 text-xs">
          <span className="rounded-md border border-border px-2 py-1 text-foreground">
            Premium 使用: {copilotUsage?.premium_usage_percent != null
              ? `${copilotUsage.premium_usage_percent.toFixed(1)}%`
              : "未知"}
          </span>
          <span className="rounded-md border border-border px-2 py-1 text-foreground">
            重置时间: {copilotUsage?.allowance_reset_at ?? "未知"}
          </span>
        </div>

        {copilotUsage?.remote_error && (
          <p className="mt-2 text-xs text-muted-foreground">
            官方配额接口暂不可用: {copilotUsage.remote_error}
          </p>
        )}
      </div>
    </div>
  );
}

function MetricItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-border bg-secondary px-3 py-2">
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="mt-1 text-sm font-semibold text-foreground">{value}</p>
    </div>
  );
}
