# tick — 本地 Agent-First Issue Tracker

## Context

构建一个本地优先的需求管理工具，类似 GitHub Issues 但完全离线运行。**专为单人多 Agent 场景设计**，不面向团队协作。核心设计理念是 **Agent-First**：AI Coding Agent 是一等公民，CLI 输出默认机器可读（JSON），人类可读模式为可选项。数据存储使用 SQLite，接口为 CLI。

**使用场景**：一个开发者同时使用多个 AI Agent（Claude、Copilot 等）协同开发，需要统一的 issue 追踪来协调工作。不涉及用户认证、权限管理等团队功能。

**核心差异化：Serverless**。与 Gitea/Forgejo 等自部署平台相比，tick 无服务进程、无端口、无运行时——单二进制直接读写 SQLite，用完即走。tick 是结构化的需求数据库，不是 git 平台。Review 是状态流转，不是 PR 系统。

项目路径：`/Users/wangjiyuan/dev/tick`

## 语言选择：Rust

**理由：**

- `clap` derive 宏使 CLI 定义极其简洁，自动生成 help/completion
- `serde` 让 JSON 输出零成本实现（Agent-First 的核心需求）
- `rusqlite` 成熟稳定，无需 ORM
- 强类型系统天然适合建模 issue 状态机等领域模型
- 编译为单二进制文件，零运行时依赖，分发简单
- 错误处理生态优秀（`anyhow` + `thiserror`）

## 核心依赖

| 用途        | crate                                     |
| ----------- | ----------------------------------------- |
| CLI 解析    | `clap` (derive)                           |
| JSON 序列化 | `serde` + `serde_json`                    |
| SQLite      | `rusqlite` (bundled feature)              |
| 错误处理    | `anyhow` + `thiserror`                    |
| 时间处理    | `chrono`                                  |
| 表格输出    | `comfy-table`                             |
| MCP Server  | `tokio` + `serde_json`（stdio transport） |
| 测试：CLI 集成 | `assert_cmd` + `predicates` |
| 测试：临时数据库 | `tempfile` |
| 测试：JSON 断言 | `assert_json_diff` |

## 设计决策

### ID 策略

- **仅使用自增整数 ID**，不支持自定义 ID
- 理由：消除 ID 歧义，Agent 调用行为完全确定性

### 不可删除原则

- **Issue 不允许删除，只允许 close**
- 理由：防止数据丢失，Agent 误操作代价降到最低
- Comment 等附属数据随 issue 永久保留

### 输出模式

- **默认输出 JSON**（Agent-First）
- `--pretty`：人类友好的表格/彩色输出
- 不提供 `--json` flag（JSON 就是默认行为，无需显式指定）

### Issue 依赖

- 仅保留 `depends-on`/`depended-by` 有向依赖关系
- 创建 `A depends-on B` 时，自动在同一事务内创建 `B depended-by A`
- 弱关联（related）不建模，在 issue 描述中自然语言提及即可
- **依赖与状态流转**：
  - A depends-on B 时，A 进入 `in-progress` 要求 B 为 `closed(resolved)`
  - B 被 `closed(wontfix)` 时，所有 depends-on B 的 issue 自动级联 `closed(wontfix)`，递归传播
  - 级联关闭时自动添加 comment（role: system）说明原因
- **循环检测**：创建依赖时检查是否会形成环，拒绝循环依赖

### Issue 类型

- 使用 `type` 字段替代 label 系统，硬编码有限种类：`bug`、`feature`、`refactor`、`docs`、`test`、`chore`
- 与 `status`/`priority` 一样通过 CHECK 约束保证合法值
- 一个 issue 一个 type，不需要多对多关系

### 状态机

```
                    ┌──────────── reopen ─────────────┐
                    ▼                                  │
                 [open] ─────────────────────┐         │
                    │                        │         │
              start work                  wontfix      │
                    │                        │         │
                    ▼                        │         │
              [in-progress] ──── wontfix ─┐  │         │
                  │     ▲                 │  │         │
          complete│     │rejected         │  │         │
                  │     │                 │  │         │
                  ▼     │                 │  │         │
                [done] ──── wontfix ───┐  │  │         │
                  │                    │  │  │         │
                 pass                  │  │  │         │
                  │                    ▼  ▼  ▼         │
                  └──────────────► [closed(R)] ────────┘
                                   R = resolved | wontfix
```

**状态语义：**

- `open`：待处理
- `in-progress`：进行中（暂停也保持此状态）
- `done`：工作完成，等待 reviewer agent review
- `closed`：已关闭，需附带 `resolution`：
  - `resolved`：问题已解决（默认值）
  - `wontfix`：放弃/不修复

**合法转换（其余一律拒绝）：**

| 从            | 到                 | 场景                                    |
| ------------- | ------------------ | --------------------------------------- |
| `open`        | `in-progress`      | Agent 开始工作                          |
| `open`        | `closed(wontfix)`  | 创建后发现不需要做                      |
| `in-progress` | `done`             | Agent 完成工作，提交 review             |
| `in-progress` | `closed(wontfix)`  | 放弃此方案，未经 review 不允许 resolved |
| `done`        | `in-progress`      | Reviewer 打回，继续修改                 |
| `done`        | `closed(resolved)` | Reviewer 通过                           |
| `done`        | `closed(wontfix)`  | Reviewer 判定不应修复                   |
| `closed`      | `open`             | Reopen（清除 branch + resolution）      |

**状态 Invariants：**

| 状态          | `branch` | `resolution` |
| ------------- | -------- | ------------ |
| `open`        | NULL     | NULL         |
| `in-progress` | 非空     | NULL         |
| `done`        | 非空     | NULL         |
| `closed`      | 可空     | 非空         |

### 校验规则汇总（`src/validators.rs`）

- **状态转换合法性**：仅允许上表中的 8 种用户转换
- **状态 invariants**：按 invariants 表校验 branch 和 resolution（转换时自动维护，如 reopen 清除两者）
- **并发保护**：`version` 整数字段作为乐观锁版本号
  - 写操作可通过 `--expect-version` 启用 CAS 校验（`WHERE id = ? AND version = ?`），不匹配返回 `CONFLICT`
  - 未提供 `--expect-version` 时，直接执行，不做并发检查
- **Parent 循环检测**：设置 `parent_id` 时遍历祖先链，发现循环则拒绝
- **Link 自引用防护**：`from_issue_id` 不得等于 `to_issue_id`
- **依赖前置校验**：两个入口都校验——(1) 进入 `in-progress` 时，所有 depends-on 的 issue 必须为 `closed(resolved)`；(2) 新增依赖时，如果 from issue 已非 `open` 状态，则被依赖的 issue 必须为 `closed(resolved)`
- **依赖循环检测**：创建依赖时遍历依赖链，拒绝形成环
- **Wontfix 级联**：issue 被 wontfix 时，递归关闭所有依赖它的 issue（不受状态转换表限制，可从任何非 closed 状态直接进入 `closed(wontfix)`），附 system comment
- 所有校验逻辑集中于此文件，commands 层调用，不内联散落

### 无身份管理，有角色视角

- tick 不做身份管理、不做调度——专注结构化数据管理
- 人是路由器：由人决定哪个 Agent 做什么，不通过 tick 分配
- 操作审计依赖 git（blame、commit message），不在 tick 内重复建设
- Comment 不追踪"谁写的"，但通过 `role` 字段标记**以什么视角写的**：
  - `worker`：实现者视角——方案描述、技术决策、变更说明
  - `reviewer`：代码审查视角——代码质量、安全漏洞、架构问题
  - `pm`：产品视角——需求是否满足、功能范围、用户体验
  - `qa`：质量视角——边界情况、测试覆盖、回归风险
  - `user`：人类直接输入（默认值）
  - `system`：系统自动生成（如 wontfix 级联关闭说明）
- 同一个 Agent 可以用不同 role 发表 comment，role 是视角而非身份

### Sub-issue 层级

- 在 `issues` 表添加 `parent_id` 自引用外键，而非复用 `issue_links`
- 理由：一个 issue 最多一个 parent，schema 层直接约束；`issue_links` 用于依赖关系建模，语义不同
- 允许多层嵌套，不限制深度
- 各 issue 状态独立，parent wontfix 不级联到 children（子任务可能仍有独立价值）
- Wontfix 级联仅通过依赖关系（depends-on）传播，不通过父子关系传播

### 数据库存储（仅限 Git 项目）

- 数据库存储在 `<git-common-dir>/tick/tick.db`
  - 通过 `git rev-parse --git-common-dir` 获取共享 git 目录路径
  - 子 worktree 的 `.git` 是文件（指回主仓库），不是目录——不能直接查找 `.git/tick/`
  - `--git-common-dir` 在主 worktree 和所有子 worktree 中返回同一个路径，天然共享
  - 不在工作目录中，不被 git track，不受 checkout/stash/merge 影响
  - WAL 模式保障 worktree 间的并发安全
- **`--db` flag**：可手动覆盖路径
- **DB 发现机制**：执行 `git rev-parse --git-common-dir`，在返回路径下查找 `tick/tick.db`，非 git 仓库则报错 `NOT_INITIALIZED`
- `tick init` 必须在 git 仓库内执行，否则报错

## CLI 设计

### 全局 Flag

```
tick [--pretty] [--fields <f1,f2,...>] [--quiet] [--dry-run] [--db <path>] <command>
```

- 默认输出：JSON（Agent-First）
- `--pretty`：人类友好的表格/彩色输出
- `--fields`：指定返回字段（减少 token 浪费）（Phase 2）
- `--quiet`：仅输出 ID（适合管道操作）（Phase 2）
- `--dry-run`：预检操作结果，不实际执行（Agent 友好）（Phase 2）
- `--db`：指定数据库路径（默认 `<git-common-dir>/tick/tick.db`）

### 命令树

```
tick init                                    # 初始化（必须在 git 仓库内）
tick version                                 # 输出版本号和 schema 版本
tick status                                  # 各状态 issue 计数概览

tick issue create <title> [-d desc] [-t type] [-p priority] [--parent <id>]
tick issue list [--status S] [--type T] [--priority P] [--parent <id>] [--root] [--limit N] [--offset O]  # limit 默认 50
tick issue show <id>                         # 输出包含 parent、children、branch、depends-on/depended-by 信息
tick issue update <id> [--title T] [--desc D] [--status S] [--type T] [--priority P] [--parent <id>] [--branch B] [--expect-version V]
tick issue close <id> [-c comment] [--role R] [--resolution resolved|wontfix] [--expect-version V]  # 默认 resolved
tick issue reopen <id> [--expect-version V]
# 注：update --status 仅接受 in-progress 和 done；closed 必须用 close，closed→open 必须用 reopen
tick issue search <query>                    # 全文搜索 title + description

tick comment add <issue-id> <body> [--role worker|reviewer|pm|qa|user]  # 默认 user
tick comment list <issue-id> [--role R]

tick issue link <from-id> depends-on <to-id>   # 自动创建对称关系
tick issue unlink <from-id> <to-id>            # 自动删除对称关系

tick config [--set K=V] [--get K] [--list]       # 项目配置
tick serve --mcp                                 # 启动 MCP Server（Phase 3）
tick import github <owner/repo> [--token T]      # 从 GitHub 导入（Phase 3）
```

### 错误码定义

| 退出码 | 含义       | JSON error code    |
| ------ | ---------- | ------------------ |
| 0      | 成功       | -                  |
| 1      | 一般错误   | `INTERNAL_ERROR`   |
| 2      | 未找到     | `NOT_FOUND`        |
| 3      | 参数错误   | `INVALID_ARGUMENT` |
| 4      | 数据库错误 | `DB_ERROR`         |
| 5      | 未初始化   | `NOT_INITIALIZED`  |
| 6      | 状态冲突   | `CONFLICT`         |

错误输出格式：`{"error": "...", "code": "NOT_FOUND"}`

### Agent 友好特性

1. **结构化输出**：所有命令默认输出 JSON，错误也是 JSON
2. **批量操作**：`tick issue create --batch < issues.jsonl`
3. **明确的退出码**：见上表
4. **可过滤字段**：`--fields id,title,status` 仅返回需要的字段
5. **stdin 支持**：`-d -` 从 stdin 读取描述（适合长文本，读到 EOF 结束）
6. **分页支持**：`--limit` + `--offset` 可靠遍历大量数据
7. **全文搜索**：`tick issue search` 按关键词定位 issue
8. **预检模式**：`--dry-run` 让 Agent 在执行前确认操作结果
9. **版本检测**：`tick version` 输出版本和 schema 版本，Agent 可检测兼容性

## 数据模型

### SQLite Schema

```sql
-- 以下 PRAGMA 在每次打开连接时设置，不是 schema migration 的一部分
-- PRAGMA journal_mode=WAL;
-- PRAGMA busy_timeout=5000;
-- PRAGMA foreign_keys=ON;

-- Schema 版本管理
CREATE TABLE schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
);

-- 项目配置
CREATE TABLE config (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Issues（不可删除，只能 close）
CREATE TABLE issues (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_id   INTEGER DEFAULT NULL REFERENCES issues(id),  -- sub-issue 层级
    title       TEXT NOT NULL,
    description TEXT DEFAULT '',
    type        TEXT NOT NULL DEFAULT 'feature'
                CHECK (type IN ('bug', 'feature', 'refactor', 'docs', 'test', 'chore')),
    status      TEXT NOT NULL DEFAULT 'open'
                CHECK (status IN ('open', 'in-progress', 'done', 'closed')),
    priority    TEXT NOT NULL DEFAULT 'medium'
                CHECK (priority IN ('low', 'medium', 'high', 'critical')),
    resolution  TEXT DEFAULT NULL               -- closed 时必填：resolved, wontfix
                CHECK (resolution IS NULL OR resolution IN ('resolved', 'wontfix')),
    branch      TEXT DEFAULT NULL,              -- 关联的 git 分支名
    version     INTEGER NOT NULL DEFAULT 1,    -- 乐观锁版本号，每次更新 +1
    created_at  TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
);

CREATE INDEX idx_issues_parent_id ON issues(parent_id);

-- 评论
CREATE TABLE comments (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    issue_id   INTEGER NOT NULL REFERENCES issues(id),
    body       TEXT NOT NULL,
    role       TEXT NOT NULL DEFAULT 'user'
               CHECK (role IN ('worker', 'reviewer', 'pm', 'qa', 'user', 'system')),
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
);

-- Issue 依赖（正向 depends-on + 反向 depended-by 同时存储，应用层自动维护对称性）
CREATE TABLE issue_links (
    from_issue_id INTEGER NOT NULL REFERENCES issues(id),
    to_issue_id   INTEGER NOT NULL REFERENCES issues(id),
    relation      TEXT NOT NULL CHECK (relation IN ('depends-on', 'depended-by')),
    PRIMARY KEY (from_issue_id, to_issue_id, relation)
);

-- version 递增 + updated_at 更新：在应用层每条 UPDATE 语句中显式设置
-- SET ..., version = version + 1, updated_at = datetime('now', 'utc')
-- 不使用 trigger 避免同表递归更新风险

-- 索引
CREATE INDEX idx_issues_type ON issues(type);
CREATE INDEX idx_issues_status ON issues(status);
CREATE INDEX idx_issues_priority ON issues(priority);
CREATE INDEX idx_comments_issue_id ON comments(issue_id);
```

## 项目结构

```
tick/
├── Cargo.toml
├── CLAUDE.md
├── .pre-commit-config.yaml  # prek: cargo fmt --check + cargo clippy
├── src/
│   ├── main.rs              # 入口，CLI 解析分发
│   ├── cli/
│   │   ├── mod.rs           # clap App 定义
│   │   ├── issue.rs         # issue 子命令定义
│   │   ├── comment.rs       # comment 子命令定义
│   │   └── config.rs        # config 子命令定义
│   ├── db/
│   │   ├── mod.rs           # 数据库连接管理、migration
│   │   ├── migrate.rs       # Schema 版本管理与迁移执行
│   │   ├── issues.rs        # issue CRUD
│   │   ├── comments.rs      # comment CRUD
│   │   └── links.rs         # issue link CRUD（含对称维护）
│   ├── models/
│   │   ├── mod.rs
│   │   ├── issue.rs         # Issue struct + serde
│   │   └── comment.rs
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── init.rs          # tick init
│   │   ├── issue.rs         # issue 命令逻辑
│   │   ├── comment.rs
│   │   └── config.rs
│   ├── output/
│   │   ├── mod.rs           # OutputFormat enum + 分发
│   │   ├── json.rs          # JSON 输出
│   │   └── pretty.rs        # 人类可读输出
│   ├── validators.rs        # 独立校验函数（状态-分支约束等）
│   └── error.rs             # 统一错误类型 + 错误码映射
├── migrations/
│   └── 001_init.sql         # 初始 schema
└── tests/
    ├── cli_tests.rs         # CLI 集成测试
    └── db_tests.rs          # 数据库单元测试
```

## 实现阶段

### 开发方法：TDD

- **测试先行**：每个功能先写测试，再写实现
- 单元测试：validators、DB 操作（用 `tempfile` 创建临时数据库）
- 集成测试：用 `assert_cmd` 调用编译后的二进制，验证 JSON 输出和退出码
- 每个 PR 必须包含对应测试

### Phase 1：核心骨架（MVP）

1. 项目初始化：`cargo init`，配置依赖（含测试 crate），配置代码质量工具链：
   - prek pre-commit：`cargo fmt --check` + `cargo clippy`（commit 兜底）
   - CLAUDE.md hooks：PostToolUseHook 在 Edit/Write 后自动跑 `cargo fmt` + `cargo clippy`（即时反馈）
2. CLI 框架：clap derive 定义命令树（含全局 flag）
3. 数据库层：连接管理、WAL 模式、schema 版本迁移机制、基础 CRUD
4. 核心命令：`init`、`version`、`status`、`issue create/list/show/update/close/reopen`
5. 输出系统：JSON 默认输出 + `--pretty` 模式
6. 错误处理：统一错误类型 + 错误码 + 结构化 JSON 错误输出

### Phase 2：完整功能

7. Comment 系统（含 role 字段）
8. Issue Link 系统（含 depends-on/depended-by 对称维护、unlink、级联 wontfix）
9. `--fields` 字段过滤
10. `--quiet` 模式
11. `--offset` 分页
12. `--dry-run` 预检模式
13. 全文搜索 `tick issue search`（基于 SQLite FTS5）
14. 批量操作（`--batch` 从 stdin 读 JSONL）
15. `tick config` 基础命令（set/get/list）

### Phase 3：高级功能

16. MCP Server 模式（`tick serve --mcp`）
    - 注意：引入 `tokio` 异步运行时，需用 `spawn_blocking` 桥接同步 `rusqlite`
    - 或评估 `tokio-rusqlite` crate
17. 可配置状态机（通过 `tick config`）
18. Git hook 集成
19. GitHub Issues 导入
    - 需要：HTTP client（`reqwest`）、GitHub API 认证、分页、rate limiting
    - 数据映射：GitHub label/milestone/assignee → tick 模型
20. Shell completion 生成

## 验证方式

- 每个 Phase 完成后，通过 CLI 手动验证核心场景
- 使用 `cargo test` 运行集成测试
- Phase 1 验证：
  ```bash
  tick init
  tick issue create "Fix login bug" -d "Users cannot login" -t bug -p high
  tick issue list --status open              # 默认 JSON 输出
  tick issue list --status open --pretty     # 人类可读输出
  tick issue show 1
  tick issue update 1 --status in-progress --branch fix/login-bug --expect-version 1
  tick issue update 1 --status done --expect-version 2
  tick issue close 1 --expect-version 3
  tick status
  tick version
  ```
- 完整 Agent 流程验证（Phase 2+）：
  ```bash
  tick issue create "Fix login bug" -d "Users cannot login" -t bug -p high
  tick issue update 1 --status in-progress --branch fix/login-bug --expect-version 1
  tick comment add 1 --role worker "在 auth.rs:42 加了参数转义，补了单测"
  tick issue update 1 --status done --expect-version 2          # 提交 review
  tick comment add 1 --role reviewer "LGTM"
  tick issue close 1 --role reviewer --expect-version 3         # reviewer 通过
  ```
