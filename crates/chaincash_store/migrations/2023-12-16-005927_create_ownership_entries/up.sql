CREATE TABLE ownership_entries (
  id INTEGER PRIMARY KEY NOT NULL,
  note_id INTEGER NOT NULL,
  amount BIGINT NOT NULL,
  position BIGINT NOT NULL,
  reserve_nft_id CHAR(32) NOT NULL,
  signature BLOB NOT NULL,
  FOREIGN KEY (note_id) REFERENCES notes (id) ON DELETE CASCADE
);
