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
