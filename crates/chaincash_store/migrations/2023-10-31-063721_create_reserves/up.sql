CREATE TABLE reserves (
  id INTEGER PRIMARY KEY NOT NULL,
  owner CHAR(32) NOT NULL,
  box_id INTEGER NOT NULL,
  denomination_id INTEGER,
  identifier CHAR(32) NOT NULL,
  FOREIGN KEY (box_id) REFERENCES ergo_boxes (id)
    ON DELETE CASCADE,
  FOREIGN KEY (denomination_id) REFERENCES denominations (id)
);
