## Data Model

Below is a diagram of the data model used for the chaincash server.

```mermaid
erDiagram
    NOTE {
        int id PK
        int box_id FK
        string owner "Hex encoded public key of the current owner"
        string issuer "Hex encoded public key of the notes issuer"
    }
    RESERVE {
        int id PK
        int box_id FK
        string owner "Hex encoded public key of the owner of the reserves"
    }
    ERGO_BOX {
        int id PK
        string[32] ergo_id "Modifier id of the box on the Ergo network"
        byte[] bytes "Serialized box bytes"
    }
    OWNERSHIP_ENTRY {
        int id PK
        int note_id FK
        string owner "Hex encoded public key of the note owner at this point in time"
    }
    SIGNATURE {
        int id PK
        int note_id FK
    }
    NOTE ||--o{ RESERVE : "backed by"
    NOTE ||--|{ OWNERSHIP_ENTRY : "has"
    NOTE ||--|{ SIGNATURE : "has"
    NOTE ||--|| ERGO_BOX : "is a"
    RESERVE ||--|| ERGO_BOX : "is a"
```
