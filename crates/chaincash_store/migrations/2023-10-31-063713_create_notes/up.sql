CREATE TABLE notes (
  id INTEGER PRIMARY KEY NOT NULL,
  owner BLOB NOT NULL,
  box_id INTEGER NOT NULL,
  FOREIGN KEY (box_id) REFERENCES ergo_boxes (id)
    ON DELETE CASCADE
);
