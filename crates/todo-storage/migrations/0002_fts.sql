-- Schema version 2: FTS5 trigram index for task search.

CREATE VIRTUAL TABLE task_fts USING fts5(
    task_id UNINDEXED,
    title,
    description,
    tags,
    tokenize='trigram'
);
