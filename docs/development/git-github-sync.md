# Git/GitHub 同步操作流程

> 本地 Rust 项目「tongpin-todo」同步到 GitHub 的完整操作流程。

## 一、当前状态（实测）

| 项目 | 状态 |
|------|------|
| 本地 Git 仓库 | ✅ 已初始化（`D:\HuangYZ\Documents\WorkBuddy\2026-08-27-12-43-58`） |
| 远程仓库 | ✅ 已配置 `origin = https://github.com/hyz-666/tongpin-todo.git` |
| 本地领先远程 | **26 个提交**（Plan 2 + Plan 3 全部工作） |
| 远程可达性 | ✅ 可达（`git ls-remote` 成功） |
| 认证凭据 | ❌ 已失效（详见下文「认证」） |

## 二、标准操作流程

### 1. 初始化本地 Git 仓库

```bash
cd <项目目录>
git init                      # 初始化仓库
git add .                     # 暂存所有文件
git commit -m "initial commit" # 首次提交
```

> 本项目已完成此步（`.git` 存在，`main` 分支已建立）。

### 2. 配置远程仓库地址

```bash
# 方式 A：HTTPS（推荐，走代理稳定）
git remote add origin https://github.com/hyz-666/tongpin-todo.git

# 方式 B：SSH（需配置 SSH 密钥）
git remote add origin git@github.com:hyz-666/tongpin-todo.git

# 查看当前配置
git remote -v
```

> 本项目已配置 HTTPS remote，并设置了全局 `insteadOf` 规则让 SSH 地址自动走 HTTPS。

### 3. 推送到 GitHub

```bash
# 首次推送（建立 main 分支并设置上游）
git push -u origin main

# 后续推送
git push origin main
```

### 4. 保持本地与远程同步

```bash
# 拉取远程最新（合并）
git pull origin main

# 仅拉取（不合并，先查看差异）
git fetch origin

# 查看本地领先/落后情况
git status -sb

# 查看领先的提交数
git rev-list --count origin/main..main
```

## 三、认证（当前阻塞点）

**现状**：`~/.gitconfig` 中 `credential.helper` 被设为 `<no helper>`，且 GitHub PAT 已失效/丢失。GCM store 为空、`gh` 未登录。

**解决方案（任选其一）**：

```bash
# 方案 A：用 gh CLI 重新登录（浏览器授权，会刷新 GCM 凭据）
gh auth login

# 方案 B：配置 PAT 并存入凭据管理器
git config --global credential.helper manager
# 首次 push 时输入用户名 + PAT（PAT 会存入 Windows 凭据管理器）

# 方案 C：临时用带 token 的 URL 推送（一次性）
git push https://<用户名>:<PAT>@github.com/hyz-666/tongpin-todo.git main
```

**生成 PAT**：GitHub → Settings → Developer settings → Personal access tokens → 勾选 `repo` 权限。

## 四、注意事项（本项目踩坑经验）

1. **HTTPS 优先**：SSH 22 端口在本网络被 GFW 干扰，443/HTTPS 走 Clash 代理稳定。
2. **`insteadOf` 规则**：`git config --global url."https://github.com/".insteadOf "git@github.com:"` 让所有 SSH 地址自动走 HTTPS。
3. **push 遇 `could not read Username`**：凭据失效，需重新 `git credential approve` 或方案 A/B/C。
4. **`git fetch` 偶发 `[gone]`**：PortableGit 怪癖，手动 `echo <sha> > .git/refs/remotes/origin/main` 修复。
