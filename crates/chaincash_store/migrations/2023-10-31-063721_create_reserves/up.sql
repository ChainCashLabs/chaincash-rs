CREATE TABLE reserves (
  id INTEGER PRIMARY KEY NOT NULL,
  identifier CHAR(32) UNIQUE NOT NULL,
  owner CHAR(32) NOT NULL,
  box_id INTEGER NOT NULL,
  denomination_id INTEGER,
  FOREIGN KEY (box_id) REFERENCES ergo_boxes (id)
    ON DELETE CASCADE,
  FOREIGN KEY (denomination_id) REFERENCES denominations (id)
);

CREATE UNIQUE INDEX reserve_identifier_idx ON reserves(identifier);
