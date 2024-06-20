## Data Model

Below is a diagram of the data model used for the chaincash server.

```mermaid
erDiagram
    NOTE {
        int id PK
        int box_id FK
        int denomination_id FK
        int value "The value of the note in its denomination, this is the amount of tokens in tokens(0)"
        string owner "Hex encoded public key of the current owner"
        string issuer "Hex encoded public key of the notes issuer"
    }
    RESERVE {
        int id PK
        int box_id FK
        int denomination_id FK
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
        string[32] reserve_nft_id "Reserve NFT id used as the key for the signed data inserted into the ergo box avltree"
        byte[] a "'a' value used in signature"
        bigint z "'z' value used in signature"
    }
    DENOMINATION {
        int id PK
        int type "Type enum of the denomination, 0 = erg, 1 = gold"
        int nanoerg_per_unit "The conversion rate of this denomination in nanoergs"
    }
    NOTE ||--|{ OWNERSHIP_ENTRY : "has"
    NOTE ||--|| DENOMINATION : "has"
    NOTE ||--|| ERGO_BOX : "is a"
    RESERVE ||--|| ERGO_BOX : "is a"
```
