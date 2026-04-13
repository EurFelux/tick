# tick Phase 2 开发 Spec

## 概述

Phase 2 在 Phase 1 的 MVP 基础上补全完整功能：Comment 独立命令、Issue Link 系统（含级联 wontfix）、并发保护（--expect-version）、输出控制（--fields/--quiet）、预检（--dry-run）、全文搜索、批量操作、Config 系统。

## 已有基础（Phase 1）

- comments 表 + DB 层 create/list（close -c 内部使用）
- issue_links 表 + DB 层 read（show 展示用）
- version 字段，每次 UPDATE 递增
- `--offset` 已在 list 中实现

## 功能清单

### 1. Comment 系统

**新增命令：**

```
tick comment add <issue-id> <body> [--role worker|reviewer|pm|qa|user]  # 默认 user
tick comment list <issue-id> [--role R]
```

**行为：**
- `comment add`：创建 comment，返回创建的 comment 完整 JSON
- `comment list`：返回 JSON 数组，支持 `--role` 过滤
- `--role` 过滤：DB 查询加 `WHERE role = ?` 条件
- `--pretty` 模式：add 输出 key:value，list 输出 `[role] body (created_at)` 每行一条

**新增文件：**
- `src/cli/comment.rs`：替换 Phase 1 的空 stub
- `src/commands/comment.rs`：替换 Phase 1 的空 stub

**修改：**
- `src/db/comments.rs`：`list_by_issue` 增加 `role` 过滤参数
- `src/cli/mod.rs`：添加 Comment 子命令到 Commands enum
- `src/main.rs`：添加 Comment 命令分发

### 2. Issue Link 系统

**新增命令：**

```
tick issue link <from-id> depends-on <to-id>
tick issue unlink <from-id> <to-id>
```

**行为：**

`link`：
- 在同一事务内创建两行：`(from, to, depends-on)` + `(to, from, depended-by)`
- 校验：
  - 自引用防护：from_id != to_id
  - 循环检测：遍历依赖链，拒绝形成环
  - 依赖前置校验：如果 from issue 已非 open 状态，to issue 必须为 closed(resolved)
- 返回创建的 link JSON

`unlink`：
- 删除两行对称记录
- 返回 `{"unlinked": true}`

**级联 wontfix：**
- 当 issue 通过 `close --resolution wontfix` 关闭时，递归处理所有 depended-by 的 issue
- 级联不受状态转换表限制，可从任何非 closed 状态直接进入 closed(wontfix)
- 每个被级联关闭的 issue 自动附加 system comment：`"Closed by cascade: dependency #N was abandoned"`
- 递归传播：被级联关闭的 issue 继续触发其下游的级联

**新增文件：**
- `src/db/links.rs`：扩展，添加 create + delete + 依赖链查询

**修改：**
- `src/cli/issue.rs`：添加 Link 和 Unlink 子命令
- `src/commands/issue.rs`：添加 link/unlink 处理逻辑 + close 中集成级联 wontfix
- `src/validators.rs`：添加 link 校验函数（自引用、循环、依赖前置）

### 3. --expect-version

**新增全局 flag：**

所有写操作命令（update、start、done、close、reopen）增加 `--expect-version V` 可选参数。

**行为：**
- 传了：SQL 加 `AND version = ?`，不匹配返回 CONFLICT（exit code 6）
- 不传：不检查，直接执行

**修改：**
- `src/cli/issue.rs`：Update/Start/Done/Close/Reopen 加 `expect_version: Option<i64>` 字段
- `src/db/issues.rs`：`update_fields` 和 `update_status_atomic` 增加 `expect_version` 参数
- `src/db/mod.rs`：更新 convenience methods 签名
- `src/commands/issue.rs`：传递 expect_version 参数

### 4. --fields 字段过滤

**新增全局 flag：**

```
tick [--fields id,title,status] issue show 1
tick [--fields id,title] issue list
```

**行为：**
- 对 JSON 输出进行 key 过滤：序列化完整 JSON 后，只保留指定的 key
- 对嵌套对象（如 IssueDetail 的 parent/children/comments）：--fields 只过滤顶层字段
- `--pretty` 模式下也生效
- 仅对 issue show/list/create/update/start/done/close/reopen 的输出生效

**实现：**
- 在 output 层实现 filter 函数，对 `serde_json::Value` 做 key 过滤
- `--fields` 与 `--quiet` 互斥

### 5. --quiet 模式

**新增全局 flag：**

```
tick [--quiet] issue create "Test"   # 输出: 1
tick [--quiet] issue start 1 --branch fix  # 输出: 1
```

**行为：**
- 仅输出资源 ID（issue id 或 comment id），一行一个
- 对所有写操作生效
- 对 list 命令：每行一个 id
- 与 `--fields`、`--pretty` 互斥

### 6. --dry-run 预检

**新增全局 flag：**

```
tick [--dry-run] issue start 1 --branch fix
```

**行为：**
- 执行所有校验（状态转换、依赖检查、parent 循环等）但不写入 DB
- 成功输出：`{"dry_run": true, "would_succeed": true}`
- 失败输出：正常的错误 JSON + 对应退出码
- 对所有写操作生效

**实现：**
- 在 commands 层，校验通过后检查 dry_run flag，如果为 true 直接返回成功而不调用 DB 写入

### 7. 全文搜索

**新增命令：**

```
tick issue search <query> [--limit N] [--offset O]
```

**行为：**
- 对 title + description 做全文搜索
- 返回匹配的 issue JSON 数组（同 list 格式）
- 支持 --limit（默认 50）和 --offset

**实现：**
- 新增 migration `002_fts.sql`：创建 FTS5 虚拟表 + 初始数据填充
- issue 创建/更新时同步更新 FTS 索引（在 db/issues.rs 的 create 和 update_fields 中）
- 搜索用 FTS5 的 MATCH 语法

**Schema（002_fts.sql）：**
```sql
CREATE VIRTUAL TABLE issues_fts USING fts5(title, description, content=issues, content_rowid=id);

-- 填充已有数据
INSERT INTO issues_fts(rowid, title, description) SELECT id, title, description FROM issues;

-- 自动同步触发器
CREATE TRIGGER trg_issues_fts_insert AFTER INSERT ON issues BEGIN
    INSERT INTO issues_fts(rowid, title, description) VALUES (NEW.id, NEW.title, NEW.description);
END;

CREATE TRIGGER trg_issues_fts_update AFTER UPDATE OF title, description ON issues BEGIN
    INSERT INTO issues_fts(issues_fts, rowid, title, description) VALUES('delete', OLD.id, OLD.title, OLD.description);
    INSERT INTO issues_fts(rowid, title, description) VALUES (NEW.id, NEW.title, NEW.description);
END;
```

注意：issue 不可删除，不需要 DELETE 触发器。FTS 同步用 SQLite 触发器而非应用层，因为这里不涉及 version/updated_at 的同表递归问题。

### 8. 批量操作

**新增 flag：**

```
tick issue create --batch < issues.jsonl
```

**行为：**
- 从 stdin 读取 JSONL，每行一个 JSON 对象：`{"title": "...", "description": "...", "type": "bug", "priority": "high", "parent": 1}`
- 所有字段除 title 外可选，使用默认值
- 输出：JSON 数组，每个元素是创建结果或错误
  - 成功：完整 issue JSON
  - 失败：`{"error": "...", "code": "...", "line": N}`
- 失败的行不阻塞后续行
- 退出码：全部成功=0，有失败=1

**实现：**
- 在 commands/issue.rs 中实现 batch_create 函数
- 逐行 parse JSON，调用现有 create 逻辑

### 9. Config 系统

**新增命令：**

```
tick config --set key=value
tick config --get key
tick config --list
```

**行为：**
- `--set`：写入 config 表（INSERT OR REPLACE）
- `--get`：读取并输出 `{"key": "...", "value": "..."}`，不存在返回 NOT_FOUND
- `--list`：输出所有配置的 JSON 数组
- `--pretty` 模式：key: value 格式

**新增文件：**
- `src/db/config.rs`：config 表 CRUD
- `src/commands/config.rs`：替换 Phase 1 的空位（cli/config.rs 可能已有定义）

## 新增/修改文件汇总

| 文件 | 操作 |
|------|------|
| `migrations/002_fts.sql` | 新增 |
| `src/cli/comment.rs` | 重写（替换 stub） |
| `src/cli/issue.rs` | 修改（加 Link/Unlink/Search 子命令 + expect-version） |
| `src/cli/mod.rs` | 修改（加 Comment 到 Commands + 全局 flags） |
| `src/cli/config.rs` | 新增（或重写 stub） |
| `src/commands/comment.rs` | 重写（替换 stub） |
| `src/commands/issue.rs` | 修改（link/unlink/search/batch + 级联 wontfix + dry-run + expect-version） |
| `src/commands/config.rs` | 新增 |
| `src/db/comments.rs` | 修改（role 过滤） |
| `src/db/links.rs` | 修改（create/delete + 依赖链查询） |
| `src/db/issues.rs` | 修改（expect-version + FTS 搜索） |
| `src/db/config.rs` | 新增 |
| `src/db/mod.rs` | 修改（新方法 + migrate 版本） |
| `src/db/migrate.rs` | 修改（002_fts migration） |
| `src/validators.rs` | 修改（link 校验 + 级联逻辑） |
| `src/output/json.rs` | 修改（fields 过滤） |
| `src/output/pretty.rs` | 修改（comment 输出 + config 输出） |
| `src/main.rs` | 修改（新命令分发 + 全局 flags） |
| `tests/cli_tests.rs` | 修改（大量新测试） |
| `tests/db_tests.rs` | 修改（新测试） |

## 测试策略

每个功能对应的测试场景：

1. **Comment**：add + list、role 过滤、issue 不存在报错
2. **Link**：link + show 展示、unlink、自引用拒绝、循环拒绝、级联 wontfix（含递归）
3. **--expect-version**：正确版本通过、错误版本 CONFLICT、不传不检查
4. **--fields**：过滤输出、嵌套字段
5. **--quiet**：create/start/list 仅输出 ID
6. **--dry-run**：start 成功预检、start 失败预检（依赖未满足）
7. **search**：创建多个 issue、搜索匹配、搜索无结果
8. **batch**：多行成功、部分失败、空 stdin
9. **config**：set + get + list、get 不存在报错
