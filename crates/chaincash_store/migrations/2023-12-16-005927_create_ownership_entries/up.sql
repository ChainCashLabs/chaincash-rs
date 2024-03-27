CREATE TABLE ownership_entries (
  id INTEGER PRIMARY KEY NOT NULL,
  note_id INTEGER NOT NULL,
  reserve_nft_id CHAR(32) NOT NULL,
  a BLOB NOT NULL,
  z INTEGER NOT NULL,
  FOREIGN KEY (note_id) REFERENCES notes (id)
);
