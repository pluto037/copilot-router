import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { execSync } from "node:child_process";

const cwd = process.cwd();
const rawArgs = process.argv.slice(2);
const flags = new Set(rawArgs.filter((arg) => arg.startsWith("--")));
const argTag = rawArgs.find((arg) => !arg.startsWith("--"));

function fail(message) {
  console.error(`❌ ${message}`);
  process.exit(1);
}

if (!argTag) {
  fail("请提供版本号，例如: npm run release:prepare -- v0.2.0");
}

if (!/^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(argTag)) {
  fail("版本号格式错误，应为 vX.Y.Z 或 vX.Y.Z-xxx");
}

const version = argTag.slice(1);
const shouldPush = flags.has("--push");
const skipChecks = flags.has("--skip-checks");

const files = {
  packageJson: resolve(cwd, "package.json"),
  tauriConf: resolve(cwd, "src-tauri/tauri.conf.json"),
  cargoToml: resolve(cwd, "src-tauri/Cargo.toml"),
};

function updatePackageJson(path, nextVersion) {
  const raw = readFileSync(path, "utf-8");
  const data = JSON.parse(raw);
  data.version = nextVersion;
  writeFileSync(path, `${JSON.stringify(data, null, 2)}\n`, "utf-8");
}

function updateTauriConf(path, nextVersion) {
  const raw = readFileSync(path, "utf-8");
  const data = JSON.parse(raw);
  data.version = nextVersion;
  writeFileSync(path, `${JSON.stringify(data, null, 2)}\n`, "utf-8");
}

function updateCargoToml(path, nextVersion) {
  const raw = readFileSync(path, "utf-8");
  const updated = raw.replace(
    /(\[package\][\s\S]*?\nversion\s*=\s*")[^"]+("[\s\S]*?)/,
    `$1${nextVersion}$2`
  );

  if (updated === raw) {
    fail("未找到 src-tauri/Cargo.toml 的 package.version 字段");
  }

  writeFileSync(path, updated, "utf-8");
}

function run(cmd) {
  execSync(cmd, { stdio: "inherit" });
}

try {
  console.log(`🚀 准备发布 ${argTag}`);

  updatePackageJson(files.packageJson, version);
  updateTauriConf(files.tauriConf, version);
  updateCargoToml(files.cargoToml, version);

  console.log("✅ 已同步版本号: package.json / src-tauri/tauri.conf.json / src-tauri/Cargo.toml");

  if (!skipChecks) {
    console.log("🔍 运行构建检查...");
    run("npm run build");
    run("cargo check --manifest-path src-tauri/Cargo.toml");
  } else {
    console.log("⏭️ 已跳过构建检查 (--skip-checks)");
  }

  run("git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml");
  run(`git commit -m \"chore(release): ${argTag}\"`);
  run(`git tag ${argTag}`);

  if (shouldPush) {
    run("git push");
    run(`git push origin ${argTag}`);
    console.log(`🎉 发布触发完成: ${argTag}`);
  } else {
    console.log("✅ 已完成本地发布准备（commit + tag）");
    console.log(`➡️ 下一步执行: git push && git push origin ${argTag}`);
  }
} catch (error) {
  fail(`发布流程失败: ${error.message ?? String(error)}`);
}
