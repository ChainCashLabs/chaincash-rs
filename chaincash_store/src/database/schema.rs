// @generated automatically by Diesel CLI.

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
        owner -> Binary,
        box_id -> Integer,
    }
}

diesel::table! {
    reserves (id) {
        id -> Integer,
        issuer -> Binary,
        box_id -> Integer,
    }
}

diesel::joinable!(notes -> ergo_boxes (box_id));
diesel::joinable!(reserves -> ergo_boxes (box_id));

diesel::allow_tables_to_appear_in_same_query!(
    ergo_boxes,
    notes,
    reserves,
);
