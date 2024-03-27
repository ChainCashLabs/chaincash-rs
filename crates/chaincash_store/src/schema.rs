// @generated automatically by Diesel CLI.

diesel::table! {
    denominations (id) {
        id -> Integer,
        #[sql_name = "type"]
        type_ -> Integer,
        nanoerg_per_unit -> Nullable<Integer>,
    }
}

diesel::table! {
    ergo_boxes (id) {
        id -> Integer,
        ergo_id -> Text,
        bytes -> Binary,
    }
}

diesel::table! {
    notes (id) {
        id -> Integer,
        box_id -> Integer,
        denomination_id -> Integer,
        value -> Integer,
        owner -> Text,
        issuer -> Text,
    }
}

diesel::table! {
    ownership_entries (id) {
        id -> Integer,
        note_id -> Integer,
        reserve_nft_id -> Text,
        a -> Binary,
        z -> Integer,
    }
}

diesel::table! {
    reserves (id) {
        id -> Integer,
        owner -> Text,
        box_id -> Integer,
        denomination_id -> Integer,
        identifier -> Text,
    }
}

diesel::joinable!(notes -> denominations (denomination_id));
diesel::joinable!(notes -> ergo_boxes (box_id));
diesel::joinable!(ownership_entries -> notes (note_id));
diesel::joinable!(reserves -> denominations (denomination_id));
diesel::joinable!(reserves -> ergo_boxes (box_id));

diesel::allow_tables_to_appear_in_same_query!(
    denominations,
    ergo_boxes,
    notes,
    ownership_entries,
    reserves,
);
