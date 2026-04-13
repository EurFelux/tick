# tick Phase 1 开发 Spec

## 概述

Phase 1 交付 tick 的核心骨架 MVP：CLI 框架、SQLite 数据库层、issue CRUD + 状态机、JSON/pretty 输出、统一错误处理。完成后可以运行完整的 issue 生命周期：create → start → done → close。

## 功能范围

### 包含

| 命令 | 行为 |
|------|------|
| `tick init` | 在当前 git 仓库内初始化数据库，非 git 仓库报错 |
| `tick version` | 输出 tick 版本号 + schema 版本号 |
| `tick status` | 输出各状态的 issue 计数 |
| `tick issue create <title>` | 创建 issue，支持 `-d`/`-t`/`-p`/`--parent`，返回完整 JSON |
| `tick issue list` | 按条件过滤列出 issues，支持 `--status`/`--type`/`--priority`/`--parent`/`--root`/`--limit`/`--offset`，limit 默认 50 |
| `tick issue show <id>` | 展示详情，含 parent、children、depends-on/depended-by、comments |
| `tick issue update <id>` | 修改非状态字段：`--title`/`--desc`/`--type`/`--priority`/`--parent`。不接受 `--status` |
| `tick issue start <id> --branch B` | open → in-progress，`--branch` 必填，原子校验 |
| `tick issue done <id>` | in-progress → done，原子校验 |
| `tick issue close <id>` | 合法前置状态 → closed，支持 `-c`/`--role`/`--resolution`（默认 resolved），原子校验 |
| `tick issue reopen <id>` | closed → open，清除 branch + resolution，原子校验 |

### 不包含（Phase 2+）

- `tick comment add/list`（独立命令）
- `tick issue link/unlink`
- `tick issue search`
- `tick config`
- `--fields`、`--quiet`、`--dry-run`、`--expect-version`
- 批量操作（`--batch`）
- 全文搜索（FTS5）
- 级联 wontfix（依赖 issue link 命令，Phase 2 一起做）

### 预建但不暴露

Schema 中 comments 表、issue_links 表、version 字段在 Phase 1 就建好：
- `show` 需要读取 comments 和 links 来展示
- `close -c` 需要写入 comments 表
- version 字段随每次 update 递增，为 Phase 2 的 `--expect-version` 做准备

## 状态机

### 状态转换命令

所有状态转换走语义化子命令，`update` 不处理状态变更。每条命令内置原子 CAS 校验（`WHERE status = ?`），天然防止并发 race condition。

| 命令 | 转换 | SQL 条件 | 失败返回 |
|------|------|----------|----------|
| `start` | open → in-progress | `WHERE id = ? AND status = 'open'` | `CONFLICT` |
| `done` | in-progress → done | `WHERE id = ? AND status = 'in-progress'` | `CONFLICT` |
| `close` | open/in-progress/done → closed | `WHERE id = ? AND status = ?`（校验合法前置） | `CONFLICT` |
| `reopen` | closed → open | `WHERE id = ? AND status = 'closed'` | `CONFLICT` |

### 校验规则

集中在 `src/validators.rs`，命令层调用：

**start：**
- 前置：status = open
- 必须提供 `--branch`
- 所有 depends-on 的 issue 必须为 closed(resolved)
- 成功后：status = in-progress, branch = 非空

**done：**
- 前置：status = in-progress
- 成功后：status = done, branch 保留

**close：**
- 前置：status 在合法前置集内
- open / in-progress → resolution 只能是 wontfix
- done → resolution 可以是 resolved（默认）或 wontfix
- `-c comment` 时创建 comments 记录
- Phase 1 不做 wontfix 级联（依赖 issue link 命令，Phase 2 一起做）
- 成功后：status = closed, resolution = 非空

**reopen：**
- 前置：status = closed
- 成功后：status = open, branch = NULL, resolution = NULL

**update（非状态字段）：**
- 设置 parent_id 时：检查祖先链防循环

### Invariants

| 状态 | branch | resolution |
|------|--------|------------|
| open | NULL | NULL |
| in-progress | 非空 | NULL |
| done | 非空 | NULL |
| closed | 可空 | 非空 |

## 数据库层

### 连接管理

- 通过 `git rev-parse --git-common-dir` 定位 DB 路径：`<git-common-dir>/tick/tick.db`
- 每次连接设置 PRAGMA：
  - `journal_mode=WAL`
  - `busy_timeout=5000`
  - `foreign_keys=ON`
- `--db` 全局 flag 可覆盖路径

### Schema Migration

- `schema_version` 表记录已应用的版本号
- `tick init`：创建 `tick/` 目录 + 执行全部未应用的 migration
- 非 init 命令启动时检查 schema_version，版本不匹配报错提示
- Phase 1 只有 `001_init.sql` 一个 migration

### Schema（001_init.sql）

```sql
CREATE TABLE schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
);

CREATE TABLE config (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE issues (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_id   INTEGER DEFAULT NULL REFERENCES issues(id),
    title       TEXT NOT NULL,
    description TEXT DEFAULT '',
    type        TEXT NOT NULL DEFAULT 'feature'
                CHECK (type IN ('bug', 'feature', 'refactor', 'docs', 'test', 'chore')),
    status      TEXT NOT NULL DEFAULT 'open'
                CHECK (status IN ('open', 'in-progress', 'done', 'closed')),
    priority    TEXT NOT NULL DEFAULT 'medium'
                CHECK (priority IN ('low', 'medium', 'high', 'critical')),
    resolution  TEXT DEFAULT NULL
                CHECK (resolution IS NULL OR resolution IN ('resolved', 'wontfix')),
    branch      TEXT DEFAULT NULL,
    version     INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
);

CREATE TABLE comments (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    issue_id   INTEGER NOT NULL REFERENCES issues(id),
    body       TEXT NOT NULL,
    role       TEXT NOT NULL DEFAULT 'user'
               CHECK (role IN ('worker', 'reviewer', 'pm', 'qa', 'user', 'system')),
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
);

CREATE TABLE issue_links (
    from_issue_id INTEGER NOT NULL REFERENCES issues(id),
    to_issue_id   INTEGER NOT NULL REFERENCES issues(id),
    relation      TEXT NOT NULL CHECK (relation IN ('depends-on', 'depended-by')),
    PRIMARY KEY (from_issue_id, to_issue_id, relation)
);

CREATE INDEX idx_issues_parent_id ON issues(parent_id);
CREATE INDEX idx_issues_type ON issues(type);
CREATE INDEX idx_issues_status ON issues(status);
CREATE INDEX idx_issues_priority ON issues(priority);
CREATE INDEX idx_comments_issue_id ON comments(issue_id);
```

### version + updated_at

- 应用层每条 UPDATE 语句显式 `SET version = version + 1, updated_at = datetime('now', 'utc')`
- 不使用 trigger，避免同表递归更新风险
- Phase 1 不做 CAS 校验（`--expect-version` 推到 Phase 2）

## CLI 定义

### 全局 Flag

```
tick [--pretty] [--db <path>] <command>
```

- 默认输出 JSON
- `--pretty`：人类可读的简单 key: value 文本
- `--db`：覆盖数据库路径（默认 `<git-common-dir>/tick/tick.db`）

Phase 2 再加 `--fields`、`--quiet`、`--dry-run`。

### 输出规则

**成功：**
- JSON 对象或数组，退出码 0
- `tick issue show`：单个 JSON 对象，嵌套 parent、children、depends_on、depended_by、comments
- `tick issue list`：JSON 数组
- `tick status`：`{"open": N, "in-progress": N, "done": N, "closed": N}`

**失败：**
- `{"error": "...", "code": "NOT_FOUND"}`，对应退出码

**--pretty 模式：**
- 成功：简单 key: value 文本，不使用表格库
- 失败：`Error: <message>`

## 错误类型与退出码

| 错误类型 | 退出码 | JSON code |
|----------|--------|-----------|
| 成功 | 0 | - |
| Internal | 1 | `INTERNAL_ERROR` |
| NotFound | 2 | `NOT_FOUND` |
| InvalidArgument | 3 | `INVALID_ARGUMENT` |
| Db | 4 | `DB_ERROR` |
| NotInitialized | 5 | `NOT_INITIALIZED` |
| Conflict | 6 | `CONFLICT` |

## 项目结构

```
tick/
├── Cargo.toml
├── CLAUDE.md
├── .pre-commit-config.yaml
├── src/
│   ├── main.rs              # 入口，CLI 解析分发，错误输出
│   ├── cli/
│   │   ├── mod.rs           # clap App 定义 + 全局 flag
│   │   ├── issue.rs         # issue 子命令定义（含 start/done/close/reopen）
│   │   └── comment.rs       # comment 子命令定义（Phase 1 仅 close -c 用）
│   ├── db/
│   │   ├── mod.rs           # 连接管理、PRAGMA 设置
│   │   ├── migrate.rs       # schema 版本管理与 migration 执行
│   │   ├── issues.rs        # issue CRUD
│   │   ├── comments.rs      # comment 写入（Phase 1: close -c）
│   │   └── links.rs         # issue link 读取（Phase 1: show 展示用）
│   ├── models/
│   │   ├── mod.rs
│   │   ├── issue.rs         # Issue struct + serde
│   │   └── comment.rs       # Comment struct + serde
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── init.rs          # tick init
│   │   ├── issue.rs         # issue 命令逻辑（含 start/done/close/reopen）
│   │   └── comment.rs       # comment 逻辑（Phase 1 仅内部调用）
│   ├── output/
│   │   ├── mod.rs           # OutputFormat enum + 分发
│   │   ├── json.rs          # JSON 输出
│   │   └── pretty.rs        # key: value 文本输出
│   ├── validators.rs        # 校验函数集合
│   └── error.rs             # TickError + 退出码映射
├── migrations/
│   └── 001_init.sql
└── tests/
    ├── cli_tests.rs         # CLI 集成测试
    └── db_tests.rs          # 数据库单元测试
```

## 测试策略

### TDD 流程

每个功能先写测试，再写实现。

### 单元测试

- **DB 层**（`tests/db_tests.rs`）：用 `tempfile` 创建临时数据库，测试 CRUD
- **Validators**（模块内 `#[cfg(test)]`）：状态转换合法性、invariants、依赖前置、循环检测

### 集成测试

- **CLI 测试**（`tests/cli_tests.rs`）：用 `assert_cmd` 调用编译后的 `tick` 二进制
- 每个测试创建临时目录 + `git init` + `tick init --db <temp_path>`，隔离环境
- 验证 JSON 输出结构（`assert_json_diff`）、退出码、错误码
- 覆盖场景：
  - 完整生命周期：create → start → done → close
  - reopen 后重新走流程
  - close wontfix（从 open、in-progress）
  - race condition：start 已经 in-progress 的 issue → CONFLICT
  - 校验失败：start 没有 branch → INVALID_ARGUMENT
  - sub-issue：创建子 issue、show 展示 parent/children
  - 非 git 目录 init → 报错

### 不测

- `--pretty` 输出的具体格式

## 代码质量工具链

### pre-commit（prek）

- `cargo fmt --check`
- `cargo clippy -- -D warnings`

### CLAUDE.md hooks

- PostToolUseHook 在 Edit/Write 后自动跑 `cargo fmt` + `cargo clippy`

Phase 1 第一步配好，确保从第一行代码开始就有质量保障。

## 依赖清单（Cargo.toml）

| crate | 用途 | dev? |
|-------|------|------|
| `clap` (features: derive) | CLI | |
| `serde` (features: derive) | 序列化 | |
| `serde_json` | JSON 输出 | |
| `rusqlite` (features: bundled) | SQLite | |
| `anyhow` | 错误上下文 | |
| `thiserror` | 错误类型派生 | |
| `chrono` | 时间处理 | |
| `assert_cmd` | CLI 测试 | dev |
| `predicates` | 断言辅助 | dev |
| `tempfile` | 临时文件/目录 | dev |
| `assert_json_diff` | JSON 比对 | dev |
