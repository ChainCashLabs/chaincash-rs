CREATE TEMPORARY TABLE temp as SELECT scan_id, scan_name, scan_type FROM scans;

DROP TABLE scans;
CREATE TABLE scans (
    scan_id INTEGER PRIMARY KEY NOT NULL,
    scan_type TEXT CHECK (scan_type IN ('reserve', 'receipt', 'note')) NOT NULL UNIQUE,
    scan_name TEXT NOT NULL -- Store both scan id and scan name. If node changes then we can use scan name + id to invalidate this scan
);

INSERT INTO scans (scan_id, scan_type, scan_name) SELECT scan_id, scan_type, scan_name FROM temp;
