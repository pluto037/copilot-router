import { useState, useEffect } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { open } from "@tauri-apps/plugin-shell";
import { tauriApi, type AppConfig, type DeviceAuthInfo } from "@/lib/tauri";
import { cn } from "@/lib/utils";
import { Save, RefreshCw, Eye, EyeOff, Github } from "lucide-react";

export default function Settings() {
  const queryClient = useQueryClient();
  const { data: config } = useQuery({
    queryKey: ["config"],
    queryFn: () => tauriApi.getConfig(),
  });

  const [form, setForm] = useState<AppConfig>({
    proxy_port: 3100,
    proxy_enabled: true,
    auth_mode: "auto",
    github_token: null,
    client_model_targets: {
      claude_code: "claude-sonnet-4-6",
      codex: "gpt-5.2-codex",
      generic: "gpt-4o",
    },
    client_model_profiles: {
      claude_code: {
        default: "claude-sonnet-4-6",
        haiku: "claude-haiku-4-5",
        sonnet: "claude-sonnet-4-6",
        opus: "claude-opus-4-6",
        reasoning: "claude-sonnet-4-6",
        small_fast: "claude-haiku-4-5",
      },
      codex: {
        default: "gpt-5.2-codex",
        reasoning: "gpt-5.2-codex",
        small_fast: "gpt-5.1-codex-mini-preview",
      },
      generic: {
        default: "gpt-4o",
      },
    },
    model_mappings: [],
    start_on_login: false,
    start_minimized: false,
  });
  const [showToken, setShowToken] = useState(false);
  const [saved, setSaved] = useState(false);
  const [deviceInfo, setDeviceInfo] = useState<DeviceAuthInfo | null>(null);
  const [copyStatus, setCopyStatus] = useState<"idle" | "success" | "error">("idle");
  const [loginStatus, setLoginStatus] = useState<{
    type: "idle" | "progress" | "success" | "error";
    text: string;
  }>({ type: "idle", text: "" });

  useEffect(() => {
    if (config) setForm(config);
  }, [config]);

  const saveMutation = useMutation({
    mutationFn: (c: AppConfig) => tauriApi.saveConfig(c),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["config"] });
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    },
  });

  const detectMutation = useMutation({
    mutationFn: () => tauriApi.autoDetectToken(),
    onSuccess: (token) => {
      if (token) {
        setForm((prev) => ({ ...prev, github_token: token }));
      }
    },
  });

  // We separate the modal visibility from the mutation lifecycle
  const [isLoginModalOpen, setIsLoginModalOpen] = useState(false);

  async function copyCode(code: string) {
    try {
      await tauriApi.copyToClipboard(code);
      setCopyStatus("success");
    } catch {
      setCopyStatus("error");
    }

    setTimeout(() => setCopyStatus("idle"), 2000);
  }

  const startLoginFlow = async () => {
    try {
      setLoginStatus({ type: "progress", text: "正在请求 GitHub 设备授权码..." });
      const info = await tauriApi.requestGithubDeviceCode();
      setDeviceInfo(info);
      setIsLoginModalOpen(true);
      setLoginStatus({ type: "progress", text: "请在浏览器完成授权，应用正在等待回调..." });
      
      // Delay to let UI correctly mount the verification code
      await new Promise(resolve => setTimeout(resolve, 800));

      await copyCode(info.user_code);
      
      // Wait a moment for to user to see the code clearly and then open browser automatically
      setTimeout(() => {
        open(info.verification_uri).catch(() => {});
      }, 1500);

    } catch (err) {
      setLoginStatus({ type: "error", text: `登录初始化失败：${String(err)}` });
      alert(String(err));
    }
  };

  const githubLoginMutation = useMutation({
    mutationFn: async (deviceCode: string) => {
      return tauriApi.waitGithubDeviceToken(deviceCode);
    },
    onSuccess: (token) => {
      setDeviceInfo(null);
      setIsLoginModalOpen(false);
      setForm((prev) => ({ ...prev, github_token: token }));
      queryClient.invalidateQueries({ queryKey: ["config"] });
      setLoginStatus({ type: "success", text: "GitHub 登录成功，Copilot Token 已生效。" });
    },
    onError: (err) => {
      setLoginStatus({ type: "error", text: `登录失败：${String(err)}` });
    },
  });

  // Start polling when device info gets set
  useEffect(() => {
    if (deviceInfo && !githubLoginMutation.isPending) {
       githubLoginMutation.mutate(deviceInfo.device_code);
    }
  }, [deviceInfo]);

  return (
    <div className="h-full overflow-y-auto p-4">
      <div className="mx-auto max-w-2xl space-y-4">

        {/* Proxy settings */}
        <Section title="代理服务器">
          <Field label="代理开关">
            <div className="flex items-center gap-2">
              <Toggle
                checked={form.proxy_enabled}
                onChange={(v) =>
                  setForm((prev) => ({ ...prev, proxy_enabled: v }))
                }
              />
              <span className="text-xs text-muted-foreground">
                {form.proxy_enabled
                  ? "已开启：Claude Code / Codex / 插件会走本地代理"
                  : "已关闭：所有请求会被代理拒绝（503）"}
              </span>
            </div>
          </Field>
          <Field label="监听端口">
            <input
              type="number"
              min={1024}
              max={65535}
              value={form.proxy_port}
              onChange={(e) =>
                setForm((prev) => ({
                  ...prev,
                  proxy_port: Number(e.target.value),
                }))
              }
              className="w-32 rounded-md border border-border bg-secondary px-3 py-1.5 text-sm text-foreground outline-none focus:ring-1 focus:ring-ring"
            />
            <span className="text-xs text-muted-foreground">
              本地 API 地址: http://localhost:{form.proxy_port}
            </span>
          </Field>
        </Section>

        {/* Auth settings */}
        <Section title="认证配置">
          <Field label="认证方式">
            <div className="flex gap-2">
              {(["auto", "manual"] as const).map((mode) => (
                <button
                  key={mode}
                  onClick={() =>
                    setForm((prev) => ({ ...prev, auth_mode: mode }))
                  }
                  className={cn(
                    "rounded-md px-3 py-1.5 text-sm transition-colors",
                    form.auth_mode === mode
                      ? "bg-primary text-primary-foreground"
                      : "border border-border bg-secondary text-muted-foreground hover:text-foreground"
                  )}
                >
                  {mode === "auto" ? "自动读取" : "手动输入"}
                </button>
              ))}
            </div>
          </Field>

          {form.auth_mode === "auto" && (
            <Field label="自动检测">
              <button
                onClick={() => detectMutation.mutate()}
                disabled={detectMutation.isPending}
                className="flex items-center gap-1.5 rounded-md border border-border bg-secondary px-3 py-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground disabled:opacity-50"
              >
                <RefreshCw
                  className={cn(
                    "h-3.5 w-3.5",
                    detectMutation.isPending && "animate-spin"
                  )}
                />
                从 VS Code / JetBrains 检测 Token
              </button>
              {detectMutation.data && (
                <span className="text-xs text-green-400">
                  ✓ 已检测到 Token
                </span>
              )}
              {detectMutation.data === null && (
                <span className="text-xs text-yellow-400">
                  未检测到 Token，请切换为手动输入
                </span>
              )}
            </Field>
          )}

          {(form.auth_mode === "manual" || form.github_token) && (
            <>
              <Field label="快捷登录">
                <div className="flex flex-col items-start gap-2">
                  <button
                    onClick={startLoginFlow}
                    disabled={githubLoginMutation.isPending}
                    className="flex items-center gap-1.5 rounded-md border border-border bg-secondary px-3 py-1.5 text-sm text-foreground transition-colors hover:bg-secondary/80 disabled:opacity-50"
                  >
                    <Github className="h-4 w-4" />
                    {githubLoginMutation.isPending ? "请求中..." : "通过 GitHub 授权登录"}
                  </button>

                  {loginStatus.type !== "idle" && (
                    <div
                      className={cn(
                        "text-xs",
                        loginStatus.type === "success" && "text-green-400",
                        loginStatus.type === "error" && "text-red-400",
                        loginStatus.type === "progress" && "text-yellow-300"
                      )}
                    >
                      {loginStatus.text}
                    </div>
                  )}
                  
                  {isLoginModalOpen && deviceInfo && (
                    <div className="rounded-md border border-yellow-500/50 bg-yellow-500/10 p-3 text-sm text-yellow-200">
                      <p className="mb-2 font-semibold">稍后将在浏览器中打开授权页面...</p>
                      <p className="mb-3 text-muted-foreground">
                        如果未自动打开，请手动访问：
                        <a 
                          href={deviceInfo.verification_uri} 
                          target="_blank" 
                          rel="noreferrer"
                          className="text-blue-400 hover:underline ml-1"
                          onClick={(e) => {
                            e.preventDefault();
                            open(deviceInfo.verification_uri).catch(() => {});
                          }}
                        >
                          点击打开 GitHub 授权页面
                        </a>
                      </p>
                      <p>
                        在弹出的页面上输入以下 8 位验证码：
                        <span className="ml-2 rounded bg-background px-2 py-1 font-mono text-lg font-bold tracking-widest text-primary cursor-pointer hover:bg-background/80" title="点击复制" onClick={() => copyCode(deviceInfo.user_code)}>
                          {deviceInfo.user_code}
                        </span>
                        <span className="ml-2 text-xs text-muted-foreground">(点击可重新复制)</span>
                        {copyStatus === "success" && (
                          <span className="ml-2 text-xs text-green-400">复制成功</span>
                        )}
                        {copyStatus === "error" && (
                          <span className="ml-2 text-xs text-red-400">复制失败，请手动复制</span>
                        )}
                      </p>
                    </div>
                  )}
                </div>
              </Field>
              <Field label="GitHub Token">
                <div className="flex gap-2">
                  <div className="relative flex-1">
                    <input
                      type={showToken ? "text" : "password"}
                    value={form.github_token ?? ""}
                    placeholder="ghp_xxxxxxxxxxxx"
                    onChange={(e) =>
                      setForm((prev) => ({
                        ...prev,
                        github_token: e.target.value || null,
                      }))
                    }
                    className="w-full rounded-md border border-border bg-secondary px-3 py-1.5 pr-8 text-sm font-mono text-foreground outline-none focus:ring-1 focus:ring-ring"
                  />
                  <button
                    type="button"
                    onClick={() => setShowToken((v) => !v)}
                    className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                  >
                    {showToken ? (
                      <EyeOff className="h-3.5 w-3.5" />
                    ) : (
                      <Eye className="h-3.5 w-3.5" />
                    )}
                  </button>
                </div>
              </div>
              <p className="text-xs text-muted-foreground">
                需要具有 GitHub Copilot 权限的 OAuth token
              </p>
            </Field>
            </>
          )}
        </Section>

        <Section title="模型映射">
          <p className="text-xs text-muted-foreground">
            已拆分到独立页面，便于为 Claude Code / Codex / 通用插件分别管理映射策略。
          </p>
          <Link
            to="/mappings/claude"
            className="inline-flex rounded-md border border-border bg-secondary px-3 py-1.5 text-sm text-foreground transition-colors hover:bg-secondary/80"
          >
            前往模型映射页面
          </Link>
        </Section>

        {/* System settings */}
        <Section title="系统">
          <Field label="开机自启">
            <Toggle
              checked={form.start_on_login}
              onChange={(v) =>
                setForm((prev) => ({ ...prev, start_on_login: v }))
              }
            />
          </Field>
          <Field label="启动时最小化">
            <Toggle
              checked={form.start_minimized}
              onChange={(v) =>
                setForm((prev) => ({ ...prev, start_minimized: v }))
              }
            />
          </Field>
        </Section>

        {/* Save button */}
        <button
          onClick={() => saveMutation.mutate(form)}
          disabled={saveMutation.isPending}
          className={cn(
            "flex w-full items-center justify-center gap-2 rounded-lg py-2.5 text-sm font-medium transition-colors",
            saved
              ? "bg-green-500/20 text-green-400"
              : "bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          )}
        >
          <Save className="h-4 w-4" />
          {saved ? "已保存" : "保存设置"}
        </button>
      </div>
    </div>
  );
}

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="rounded-xl border border-border bg-card p-4">
      <h3 className="mb-3 text-sm font-semibold text-foreground">{title}</h3>
      <div className="space-y-3">{children}</div>
    </div>
  );
}

function Field({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex flex-col gap-1.5">
      <label className="text-xs font-medium text-muted-foreground">
        {label}
      </label>
      <div className="flex flex-wrap items-center gap-2">{children}</div>
    </div>
  );
}

function Toggle({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      onClick={() => onChange(!checked)}
      className={cn(
        "relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors",
        checked ? "bg-primary" : "bg-border"
      )}
    >
      <span
        className={cn(
          "pointer-events-none block h-4 w-4 rounded-full bg-white shadow-lg ring-0 transition-transform",
          checked ? "translate-x-4" : "translate-x-0"
        )}
      />
    </button>
  );
}
