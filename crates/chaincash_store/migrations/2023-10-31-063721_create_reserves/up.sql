CREATE TABLE reserves (
  id INTEGER PRIMARY KEY NOT NULL,
  issuer BLOB NOT NULL,
  box_id INTEGER NOT NULL,
  FOREIGN KEY (box_id) REFERENCES ergo_boxes (id)
    ON DELETE CASCADE
);
