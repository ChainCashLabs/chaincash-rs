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
        int amount "Maximum amount that can be redeemed from reserve"
        int position "Index of signature in history"
        string[32] reserve_nft_id "Reserve NFT id used as the key for the signed data inserted into the ergo box avltree"
        byte[] signature "signature"
    }
    DENOMINATION {
        int id PK
        int type "Type enum of the denomination, 0 = erg, 1 = gold"
        int nanoerg_per_unit "The conversion rate of this denomination in nanoergs"
    }
    REFUND {
        int id PK
        string[32] reserve_identifier "Reserve NFT id the refund was announced on"
        int amount "nanoErgs announced at initiation, the upper bound the contract enforces"
        int withdrawn_amount "nanoErgs actually withdrawn, set once completed"
        int init_height "Height written into R5 of the reserve box, the waiting period runs from here"
        string status "initiated, completed or cancelled"
        string[64] init_tx_id "Transaction that announced the refund"
        string[64] settle_tx_id "Transaction that completed or cancelled it"
    }
    NOTE ||--|{ OWNERSHIP_ENTRY : "has"
    NOTE ||--|| DENOMINATION : "has"
    NOTE ||--|| ERGO_BOX : "is a"
    RESERVE ||--|| ERGO_BOX : "is a"
```

`REFUND` is deliberately not a foreign key onto `RESERVE`: a reserve row is deleted as soon as its
box is spent, and the refund history has to outlive it. The reserve box carries the *pending*
refund in its registers (R5 initiation height, R6 announced amount) and the contract treats that as
the source of truth; the table adds the history the chain no longer keeps once a refund is settled.
