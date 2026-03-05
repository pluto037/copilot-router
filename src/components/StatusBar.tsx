import { useQuery } from "@tanstack/react-query";
import { tauriApi } from "@/lib/tauri";
import { cn } from "@/lib/utils";
import { Activity, Key, Zap } from "lucide-react";

export default function StatusBar() {
  const { data: proxyStatus } = useQuery({
    queryKey: ["proxyStatus"],
    queryFn: () => tauriApi.getProxyStatus(),
    refetchInterval: 3000,
  });

  const { data: tokenStatus } = useQuery({
    queryKey: ["tokenStatus"],
    queryFn: () => tauriApi.getTokenStatus(),
    refetchInterval: 10000,
  });

  return (
    <div className="flex h-7 items-center gap-4 border-t border-border bg-card px-4 text-xs text-muted-foreground">
      {/* Proxy status */}
      <div className="flex items-center gap-1.5">
        <Activity className="h-3 w-3" />
        <span
          className={cn(
            "font-medium",
            proxyStatus?.running ? "text-green-400" : "text-red-400"
          )}
        >
          {proxyStatus?.running ? "运行中" : "已停止"}
        </span>
        {proxyStatus?.running && (
          <span className="text-muted-foreground">
            :{proxyStatus.port}
          </span>
        )}
      </div>

      <div className="h-3 w-px bg-border" />

      {/* Token status */}
      <div className="flex items-center gap-1.5">
        <Key className="h-3 w-3" />
        <span
          className={cn(
            "font-medium",
            tokenStatus?.is_valid ? "text-green-400" : "text-yellow-400"
          )}
        >
          {tokenStatus?.is_valid ? "Token 有效" : "Token 失效"}
        </span>
        {tokenStatus?.token_source && (
          <span className="text-muted-foreground">
            ({tokenStatus.token_source})
          </span>
        )}
      </div>

      <div className="h-3 w-px bg-border" />

      {/* Today requests */}
      <div className="flex items-center gap-1.5">
        <Zap className="h-3 w-3" />
        <span>今日: {proxyStatus?.requests_today ?? 0} 次请求</span>
      </div>

      {/* Right side: local API address */}
      <div className="ml-auto font-mono text-[10px] text-muted-foreground/60">
        http://localhost:{proxyStatus?.port ?? 3100}
      </div>
    </div>
  );
}
