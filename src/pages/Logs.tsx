import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { tauriApi, type LogEntry } from "@/lib/tauri";
import { cn, formatDate, formatLatency, formatNumber } from "@/lib/utils";
import { RefreshCw, Trash2, CheckCircle, XCircle } from "lucide-react";

export default function Logs() {
  const [limit, setLimit] = useState(100);
  const queryClient = useQueryClient();

  const { data: logs = [], isFetching } = useQuery({
    queryKey: ["recentLogs", limit],
    queryFn: () => tauriApi.getRecentLogs(limit),
    refetchInterval: 3000,
  });

  const clearMutation = useMutation({
    mutationFn: () => tauriApi.clearLogs(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["recentLogs"] });
    },
  });

  return (
    <div className="flex h-full flex-col">
      {/* Toolbar */}
      <div className="flex items-center justify-between border-b border-border px-4 py-2">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-foreground">请求日志</span>
          <span className="rounded-full bg-secondary px-2 py-0.5 text-xs text-muted-foreground">
            {logs.length}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <select
            value={limit}
            onChange={(e) => setLimit(Number(e.target.value))}
            className="rounded-md border border-border bg-secondary px-2 py-1 text-xs text-foreground outline-none"
          >
            <option value={50}>最近 50 条</option>
            <option value={100}>最近 100 条</option>
            <option value={500}>最近 500 条</option>
          </select>
          <button
            onClick={() => queryClient.invalidateQueries({ queryKey: ["recentLogs"] })}
            className="flex items-center gap-1 rounded-md border border-border bg-secondary px-2 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
          >
            <RefreshCw
              className={cn("h-3 w-3", isFetching && "animate-spin")}
            />
            刷新
          </button>
          <button
            onClick={() => clearMutation.mutate()}
            className="flex items-center gap-1 rounded-md border border-red-500/30 bg-red-500/10 px-2 py-1 text-xs text-red-400 transition-colors hover:bg-red-500/20"
          >
            <Trash2 className="h-3 w-3" />
            清空
          </button>
        </div>
      </div>

      {/* Table */}
      <div className="flex-1 overflow-auto">
        <table className="w-full text-xs">
          <thead className="sticky top-0 bg-card">
            <tr className="border-b border-border text-left text-muted-foreground">
              <th className="px-3 py-2 font-medium">时间</th>
              <th className="px-3 py-2 font-medium">路径</th>
              <th className="px-3 py-2 font-medium">请求模型</th>
              <th className="px-3 py-2 font-medium">映射后模型</th>
              <th className="px-3 py-2 font-medium">状态</th>
              <th className="px-3 py-2 font-medium text-right">提示词</th>
              <th className="px-3 py-2 font-medium text-right">补全</th>
              <th className="px-3 py-2 font-medium text-right">延迟</th>
            </tr>
          </thead>
          <tbody>
            {logs.length === 0 ? (
              <tr>
                <td
                  colSpan={8}
                  className="py-12 text-center text-muted-foreground"
                >
                  暂无请求日志
                </td>
              </tr>
            ) : (
              logs.map((log: LogEntry) => (
                <LogRow key={log.id} log={log} />
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function LogRow({ log }: { log: LogEntry }) {
  const success = log.status_code >= 200 && log.status_code < 300;

  return (
    <tr
      className={cn(
        "border-b border-border/50 transition-colors hover:bg-secondary/50",
        !success && "bg-red-500/5"
      )}
    >
      <td className="px-3 py-1.5 font-mono text-[10px] text-muted-foreground">
        {formatDate(log.timestamp)}
      </td>
      <td className="px-3 py-1.5 font-mono text-[10px]">{log.path}</td>
      <td className="px-3 py-1.5">
        <span className="rounded-sm bg-secondary px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
          {log.requested_model || "-"}
        </span>
      </td>
      <td className="px-3 py-1.5">
        <span className="rounded-sm bg-secondary px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
          {log.mapped_model || log.model || "-"}
        </span>
      </td>
      <td className="px-3 py-1.5">
        <div className="flex items-center gap-1">
          {success ? (
            <CheckCircle className="h-3 w-3 text-green-400" />
          ) : (
            <XCircle className="h-3 w-3 text-red-400" />
          )}
          <span
            className={cn(
              "font-mono text-[10px]",
              success ? "text-green-400" : "text-red-400"
            )}
          >
            {log.status_code}
          </span>
        </div>
      </td>
      <td className="px-3 py-1.5 text-right font-mono text-[10px] text-muted-foreground">
        {formatNumber(log.prompt_tokens)}
      </td>
      <td className="px-3 py-1.5 text-right font-mono text-[10px] text-muted-foreground">
        {formatNumber(log.completion_tokens)}
      </td>
      <td className="px-3 py-1.5 text-right font-mono text-[10px]">
        {formatLatency(log.latency_ms)}
      </td>
    </tr>
  );
}
