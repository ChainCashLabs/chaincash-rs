CREATE TABLE scans (
    scan_id INTEGER PRIMARY KEY NOT NULL,
    scan_type TEXT CHECK (scan_type IN ('reserve', 'receipt', 'note')) NOT NULL UNIQUE,
    scan_name TEXT NOT NULL -- Store both scan id and scan name. If node changes then we can use scan name + id to invalidate this scan
);
