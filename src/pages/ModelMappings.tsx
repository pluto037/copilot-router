import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { NavLink, Navigate, useParams } from "react-router-dom";
import { Save } from "lucide-react";
import {
  tauriApi,
  type AppConfig,
  type ModelMappingTestResult,
} from "@/lib/tauri";
import { cn } from "@/lib/utils";

const BUILTIN_COPILOT_MODELS = [
  { label: "Claude Haiku 4.5", value: "claude-haiku-4-5" },
  { label: "Claude Opus 4.5", value: "claude-opus-4-5" },
  { label: "Claude Opus 4.6", value: "claude-opus-4-6" },
  { label: "Claude Sonnet 4", value: "claude-sonnet-4" },
  { label: "Claude Sonnet 4.5", value: "claude-sonnet-4-5" },
  { label: "Claude Sonnet 4.6", value: "claude-sonnet-4-6" },
  { label: "Gemini 2.5 Pro", value: "gemini-2.5-pro" },
  { label: "Gemini 3 Flash (Preview)", value: "gemini-3-flash-preview" },
  { label: "Gemini 3 Pro (Preview)", value: "gemini-3-pro-preview" },
  { label: "Gemini 3.1 Pro (Preview)", value: "gemini-3.1-pro-preview" },
  { label: "GPT-4.1", value: "gpt-4.1" },
  { label: "GPT-4o", value: "gpt-4o" },
  { label: "GPT-5 mini", value: "gpt-5-mini" },
  { label: "GPT-5.1", value: "gpt-5.1" },
  { label: "GPT-5.1-Codex", value: "gpt-5.1-codex" },
  { label: "GPT-5.1-Codex-Max", value: "gpt-5.1-codex-max" },
  { label: "GPT-5.1-Codex-Mini (Preview)", value: "gpt-5.1-codex-mini-preview" },
  { label: "GPT-5.2", value: "gpt-5.2" },
  { label: "GPT-5.2-Codex", value: "gpt-5.2-codex" },
  { label: "GPT-5.3-Codex", value: "gpt-5.3-codex" },
  { label: "Grok Code Fast 1", value: "grok-code-fast-1" },
] as const;

const CUSTOM_MODEL = "__custom_model__";

const defaultConfig: AppConfig = {
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
};

export default function ModelMappings() {
  const { client } = useParams();
  const queryClient = useQueryClient();

  const { data: config } = useQuery({
    queryKey: ["config"],
    queryFn: () => tauriApi.getConfig(),
  });

  const [form, setForm] = useState<AppConfig>(defaultConfig);
  const [saved, setSaved] = useState(false);
  const [testInput, setTestInput] = useState("claude-sonnet-4-6");
  const [testResult, setTestResult] = useState<ModelMappingTestResult | null>(null);

  useEffect(() => {
    if (!config) return;

    const next: AppConfig = {
      ...defaultConfig,
      ...config,
      client_model_targets: {
        ...defaultConfig.client_model_targets,
        ...config.client_model_targets,
      },
      client_model_profiles: {
        claude_code: {
          ...defaultConfig.client_model_profiles.claude_code,
          ...config.client_model_profiles?.claude_code,
        },
        codex: {
          ...defaultConfig.client_model_profiles.codex,
          ...config.client_model_profiles?.codex,
        },
        generic: {
          ...defaultConfig.client_model_profiles.generic,
          ...config.client_model_profiles?.generic,
        },
      },
    };

    setForm(next);
  }, [config]);

  const saveMutation = useMutation({
    mutationFn: (c: AppConfig) => tauriApi.saveConfig(c),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["config"] });
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    },
  });

  const testMutation = useMutation({
    mutationFn: (requestedModel: string) => tauriApi.testModelMapping(requestedModel),
    onSuccess: (result) => setTestResult(result),
  });

  const builtinSet = useMemo(
    () => new Set<string>(BUILTIN_COPILOT_MODELS.map((model) => model.value)),
    []
  );

  const clientMeta = {
    claude: {
      title: "Claude Code 配置页面",
      desc: "用于 Claude Code 相关请求的详细模型路由",
      placeholder: "例如 claude-sonnet-4-6",
    },
    codex: {
      title: "Codex 配置页面",
      desc: "用于 Codex 相关请求的详细模型路由",
      placeholder: "例如 gpt-5.2-codex",
    },
    generic: {
      title: "通用插件配置页面",
      desc: "用于其他兼容客户端的默认模型路由",
      placeholder: "例如 gpt-4o",
    },
  } as const;

  const isValidClient = client === "claude" || client === "codex" || client === "generic";
  if (!isValidClient) {
    return <Navigate to="/mappings/claude" replace />;
  }

  const pageMeta = clientMeta[client];

  function updateClaudeField(
    key: keyof AppConfig["client_model_profiles"]["claude_code"],
    value: string
  ) {
    setForm((prev) => ({
      ...prev,
      client_model_profiles: {
        ...prev.client_model_profiles,
        claude_code: {
          ...prev.client_model_profiles.claude_code,
          [key]: value,
        },
      },
      client_model_targets: {
        ...prev.client_model_targets,
        claude_code:
          key === "default" ? value : prev.client_model_targets.claude_code,
      },
    }));
  }

  function updateCodexField(
    key: keyof AppConfig["client_model_profiles"]["codex"],
    value: string
  ) {
    setForm((prev) => ({
      ...prev,
      client_model_profiles: {
        ...prev.client_model_profiles,
        codex: {
          ...prev.client_model_profiles.codex,
          [key]: value,
        },
      },
      client_model_targets: {
        ...prev.client_model_targets,
        codex: key === "default" ? value : prev.client_model_targets.codex,
      },
    }));
  }

  function updateGenericField(value: string) {
    setForm((prev) => ({
      ...prev,
      client_model_profiles: {
        ...prev.client_model_profiles,
        generic: {
          ...prev.client_model_profiles.generic,
          default: value,
        },
      },
      client_model_targets: {
        ...prev.client_model_targets,
        generic: value,
      },
    }));
  }

  function renderModelField(
    title: string,
    desc: string,
    value: string,
    onChange: (value: string) => void
  ) {
    const normalized = value.trim();
    const selectValue = builtinSet.has(normalized) ? normalized : CUSTOM_MODEL;

    return (
      <div className="rounded-lg border border-border bg-card p-3">
        <p className="text-sm font-semibold text-foreground">{title}</p>
        <p className="mt-1 text-xs text-muted-foreground">{desc}</p>

        <div className="mt-2 space-y-2">
          <p className="text-[11px] font-medium text-muted-foreground">预设模型</p>
          <select
            value={selectValue}
            onChange={(e) => {
              if (e.target.value !== CUSTOM_MODEL) {
                onChange(e.target.value);
              }
            }}
            className="w-full rounded-md border border-border bg-secondary px-3 py-1.5 text-sm text-foreground outline-none focus:ring-1 focus:ring-ring"
          >
            {BUILTIN_COPILOT_MODELS.map((model) => (
              <option key={model.value} value={model.value}>
                {model.label}
              </option>
            ))}
            <option value={CUSTOM_MODEL}>自定义模型...</option>
          </select>

          <p className="text-[11px] font-medium text-muted-foreground">最终模型 ID</p>
          <input
            value={value}
            onChange={(e) => onChange(e.target.value)}
            placeholder="输入模型 ID（可手动覆盖）"
            className="w-full rounded-md border border-border bg-secondary px-3 py-1.5 text-sm font-mono text-foreground outline-none focus:ring-1 focus:ring-ring"
          />

          {selectValue === CUSTOM_MODEL && (
            <p className="text-[11px] text-muted-foreground">
              当前为自定义模式，以上输入框内容将作为实际路由模型。
            </p>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="h-full overflow-y-auto p-4">
      <div className="mx-auto max-w-5xl space-y-4 pb-6">
        <div className="rounded-xl border border-border bg-card p-4">
          <h3 className="mb-2 text-sm font-semibold text-foreground">客户端模型配置</h3>
          <p className="text-xs text-muted-foreground">
            每个客户端使用单独配置页面：Claude Code、Codex、通用插件。当前页面仅编辑对应客户端模型。
          </p>
          <div className="mt-3 flex flex-wrap gap-2">
            <NavLink
              to="/mappings/claude"
              className={({ isActive }) =>
                cn(
                  "rounded-md px-3 py-1.5 text-xs transition-colors",
                  isActive
                    ? "bg-primary text-primary-foreground"
                    : "border border-border bg-secondary text-foreground hover:bg-secondary/80"
                )
              }
            >
              Claude Code
            </NavLink>
            <NavLink
              to="/mappings/codex"
              className={({ isActive }) =>
                cn(
                  "rounded-md px-3 py-1.5 text-xs transition-colors",
                  isActive
                    ? "bg-primary text-primary-foreground"
                    : "border border-border bg-secondary text-foreground hover:bg-secondary/80"
                )
              }
            >
              Codex
            </NavLink>
            <NavLink
              to="/mappings/generic"
              className={({ isActive }) =>
                cn(
                  "rounded-md px-3 py-1.5 text-xs transition-colors",
                  isActive
                    ? "bg-primary text-primary-foreground"
                    : "border border-border bg-secondary text-foreground hover:bg-secondary/80"
                )
              }
            >
              通用插件
            </NavLink>
          </div>
        </div>

        {client === "claude" && (
          <div className="rounded-xl border border-border bg-card p-4">
            <p className="text-sm font-medium text-foreground">{pageMeta.title}</p>
            <p className="mb-3 mt-1 text-xs text-muted-foreground">{pageMeta.desc}</p>
            <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
              {renderModelField(
                "默认模型",
                "普通 Claude 请求的默认目标",
                form.client_model_profiles.claude_code.default,
                (value) => updateClaudeField("default", value)
              )}
              {renderModelField(
                "Haiku 默认模型",
                "当请求包含 haiku 关键词时命中",
                form.client_model_profiles.claude_code.haiku,
                (value) => updateClaudeField("haiku", value)
              )}
              {renderModelField(
                "Sonnet 默认模型",
                "当请求包含 sonnet 关键词时命中",
                form.client_model_profiles.claude_code.sonnet,
                (value) => updateClaudeField("sonnet", value)
              )}
              {renderModelField(
                "Opus 默认模型",
                "当请求包含 opus 关键词时命中",
                form.client_model_profiles.claude_code.opus,
                (value) => updateClaudeField("opus", value)
              )}
              {renderModelField(
                "Reasoning 模型",
                "当请求包含 reason 关键词时命中",
                form.client_model_profiles.claude_code.reasoning,
                (value) => updateClaudeField("reasoning", value)
              )}
              {renderModelField(
                "Small/Fast 模型",
                "当请求包含 fast 关键词时命中",
                form.client_model_profiles.claude_code.small_fast,
                (value) => updateClaudeField("small_fast", value)
              )}
            </div>
          </div>
        )}

        {client === "codex" && (
          <div className="rounded-xl border border-border bg-card p-4">
            <p className="text-sm font-medium text-foreground">{pageMeta.title}</p>
            <p className="mb-3 mt-1 text-xs text-muted-foreground">{pageMeta.desc}</p>
            <div className="grid grid-cols-1 gap-3 md:grid-cols-3">
              {renderModelField(
                "默认模型",
                "Codex 常规请求",
                form.client_model_profiles.codex.default,
                (value) => updateCodexField("default", value)
              )}
              {renderModelField(
                "Reasoning 模型",
                "当请求包含 reason 关键词时命中",
                form.client_model_profiles.codex.reasoning,
                (value) => updateCodexField("reasoning", value)
              )}
              {renderModelField(
                "Small/Fast 模型",
                "当请求包含 mini/fast 关键词时命中",
                form.client_model_profiles.codex.small_fast,
                (value) => updateCodexField("small_fast", value)
              )}
            </div>
          </div>
        )}

        {client === "generic" && (
          <div className="rounded-xl border border-border bg-card p-4">
            <p className="text-sm font-medium text-foreground">{pageMeta.title}</p>
            <p className="mb-3 mt-1 text-xs text-muted-foreground">{pageMeta.desc}</p>
            <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
              {renderModelField(
                "默认模型",
                "其他所有客户端/插件请求",
                form.client_model_profiles.generic.default,
                updateGenericField
              )}
            </div>
          </div>
        )}

        <div className="rounded-xl border border-border bg-card p-4">
          <p className="mb-2 text-sm font-medium text-foreground">路由自检</p>
          <p className="mb-3 text-[11px] text-muted-foreground">
            输入客户端传入模型，查看会命中哪个目标模型，并尝试调用上游验证。
          </p>
          <div className="flex flex-col gap-2 md:flex-row">
            <input
              value={testInput}
              onChange={(e) => setTestInput(e.target.value)}
              placeholder={pageMeta.placeholder}
              className="flex-1 rounded-md border border-border bg-secondary px-3 py-1.5 text-sm font-mono text-foreground outline-none focus:ring-1 focus:ring-ring"
            />
            <button
              type="button"
              onClick={() => testMutation.mutate(testInput.trim())}
              disabled={testMutation.isPending || !testInput.trim()}
              className="rounded-md bg-secondary px-3 py-1.5 text-xs font-medium text-foreground transition-colors hover:bg-secondary/80 disabled:opacity-50"
            >
              {testMutation.isPending ? "检测中..." : "开始自检"}
            </button>
          </div>

          {testResult && (
            <div className="mt-3 rounded-md border border-border bg-secondary/40 p-3 text-xs text-muted-foreground">
              <p>
                请求模型：<span className="font-mono text-foreground">{testResult.requested_model}</span>
              </p>
              <p className="mt-1">
                路由目标：<span className="font-mono text-foreground">{testResult.resolved_model}</span>
              </p>
              <p className="mt-1">
                上游检测：
                {testResult.upstream_checked
                  ? testResult.upstream_ok
                    ? `成功 (${testResult.upstream_status ?? "-"})`
                    : `失败 (${testResult.upstream_status ?? "-"})`
                  : "未执行（缺少可用 token）"}
              </p>
              {testResult.upstream_error && (
                <p className="mt-1 break-all text-red-400">{testResult.upstream_error}</p>
              )}
            </div>
          )}
        </div>

        <div className="sticky bottom-0 z-10 rounded-xl border border-border bg-card/95 p-3 backdrop-blur">
          <button
            onClick={() =>
              saveMutation.mutate({
                ...form,
                client_model_targets: {
                  ...form.client_model_targets,
                  claude_code: form.client_model_profiles.claude_code.default,
                  codex: form.client_model_profiles.codex.default,
                  generic: form.client_model_profiles.generic.default,
                },
              })
            }
            disabled={saveMutation.isPending}
            className={cn(
              "flex w-full items-center justify-center gap-2 rounded-lg py-2.5 text-sm font-medium transition-colors",
              saved
                ? "bg-green-500/20 text-green-400"
                : "bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
            )}
          >
            <Save className="h-4 w-4" />
            {saveMutation.isPending ? "保存中..." : saved ? "已保存" : "保存客户端模型配置"}
          </button>
        </div>
      </div>
    </div>
  );
}
