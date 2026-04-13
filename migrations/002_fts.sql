CREATE VIRTUAL TABLE issues_fts USING fts5(title, description, content=issues, content_rowid=id);

INSERT INTO issues_fts(rowid, title, description) SELECT id, title, description FROM issues;

CREATE TRIGGER trg_issues_fts_insert AFTER INSERT ON issues BEGIN
    INSERT INTO issues_fts(rowid, title, description) VALUES (NEW.id, NEW.title, NEW.description);
END;

CREATE TRIGGER trg_issues_fts_update AFTER UPDATE OF title, description ON issues BEGIN
    INSERT INTO issues_fts(issues_fts, rowid, title, description) VALUES('delete', OLD.id, OLD.title, OLD.description);
    INSERT INTO issues_fts(rowid, title, description) VALUES (NEW.id, NEW.title, NEW.description);
END;
