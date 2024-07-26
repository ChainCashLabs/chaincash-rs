use ergo_lib::{
    chain::{ergo_box::box_builder::ErgoBoxCandidateBuilder, transaction::TxId},
    ergo_chain_types::EcPoint,
    ergotree_ir::{
        chain::{
            address::{Address, AddressEncoder},
            ergo_box::{box_value::BoxValue, ErgoBox, ErgoBoxCandidate, NonMandatoryRegisterId},
            token::Token,
        },
        sigma_protocol::sigma_boolean::ProveDlog,
    },
};
use proptest::arbitrary::Arbitrary;
use proptest::{
    strategy::{Strategy, ValueTree},
    test_runner::TestRunner,
};
use rand::{thread_rng, Rng};

use crate::{
    boxes::{Note, ReserveBoxSpec},
    note_history::NoteHistory,
};

pub fn force_any_val<T: Arbitrary>() -> T {
    let mut runner = TestRunner::default();
    proptest::arbitrary::any::<T>()
        .new_tree(&mut runner)
        .unwrap()
        .current()
}
// Pre-compiled note address since we need node to compile contract otherwise
pub const NOTE_ADDRESS: &'static str = "ZCrKu5bZniW2nkWfpGE5Tiyqk5VGcmb95ZmSbYAeq2jSt1PnP2LoXZBVBiAWRFt5pM6Eo8AtDMNwM4iesCcguSiiPnfnKKCHfcTh5kUCr3ZGXuMmABgPMjeHtkuthEU1coUkh1CYbw1xuiHC2udWCiLLRrUYJiT9i3hWNmgEm8DR3fgj4udrefshDR5capWKM55yeFYeht4wUs9RjBKGpGby89JWbQAa414wNU3BhYoPdhYHZfLfgddDPPCqwdVrwcEoMrfmtZFPUu1Q1xLqpVL5rbwM9mavDTXKpcvRvACW7J3jmUk8mAmH4PBJa14m2tcdxuntQo8GivAcxmEJpd36WSjrc6cHiXzq4R3e6fSNu7QWxAYyHafxR8LTGkss919iUWyzKZtNVJuAu9wGKP9FeJSXGDAopK1nR4dthbLvRVArRbTkhQSSD3MdT1PAZkjxvRZhfVEzf6FbHPocHkqfJf5fpLqqzB7TyP9utee6vAaw2UZiYAaj94PdYpJYxTUDT61zWQsZhG6Wtx5LDm6nVUN5A9xoBqiLEpSYQeuz5vk4ryv3ErXqMNT1Rp";
pub const RESERVE_ADDRESS: &'static str = "3tqwfXjHWgDQB6vacLK2na6r7QgBftHGzwJorVXCRFdtJthTCjd5FXwvtsPENdcRYnbWeF8F5mUkaBLMzrrGUzK4dhHsgGmwrQCz5zLLmrP4iQqXpuVtNXKDqdGZsQYF6fyLsEzHUTGsCBsju6xmbxz6mg7V6dGd5eJkuCjsU728ts4kQiYByUwG2Rb3UwoV8i8o3SqNeVBC6FZVW5YiZJzN95tNrfPyzMS6hz6jzbN4K83gAFLMcpQPjFpodRj35ymftJPCVoCYXn1Rz1kK5UrxDC62ny5YPu5E8k8ZAiGvnHq4KpMnN43L4fe7jiT3KLkQCsyYtXTAasqWq89un3FFBv4sPaVWEA1JPp7k8cHeiFWZewpytgMYZ9qE5NePV5zMYH6xLSvmcQg16thst3NHyUHbDLN8nqFnTJ4i9UW5YNZbYiF9gZYW7WjQhj8XgnFWxaKhoADgW5g8yWHPa3Cm3HvxxUXNjfiTNYhFZ1LVaaigje2mx32dbzJi8XoUdDFkYSxmnau8s14zbjxbpXdFmwirE2YvhYr1ScygFznkyJ3tL8tqcBsNKbSiybZLXxBxQ39aq7jeftztvgBhr3X4RKq2q3VYHZZBkM3AD8j8rD17YSieXNaKscNB8TLrEUFyz2FUyNxpbN895be2qZ5VJtKxWGT3jJ3JxTaPoUzm3rwKWjo3QohHict8WFwiiivQr55uCAZRJR13J9ie9cGhi3UJqSjuQUzqiYkx4HtRNoHYiToNqLhs8r7azgAWir73HYBiMAwgpp6giNkzwng5YzxrWzm4YpWXCksoGz1ukFqSzYKQNhWBnqsoU9MajhRzXW8wKDGrqWbwVqch2jz39XDSMf7Qc1AGYg2NdRU7vGL94LEWcGyN42ZCkWU3gYVjt6MYKJiUmKsT2ExniKUizfhVCqVm3JZKvJtQu9u3uRyBx4Gkp1Z9ctSm3oBfxrvZHsF1x7Dv";

pub fn create_box(box_candidate: ErgoBoxCandidate) -> ErgoBox {
    let mut rng = thread_rng();
    ErgoBox::from_box_candidate(
        &box_candidate,
        TxId::zero(),
        rng.gen_range(0..i16::MAX as u16),
    )
    .unwrap()
}
pub fn create_wallet_box(public_key: EcPoint, amount: u64) -> ErgoBox {
    let tree = Address::P2Pk(ProveDlog::new(public_key)).script().unwrap();
    let box_candidate = ErgoBoxCandidateBuilder::new(BoxValue::new(amount).unwrap(), tree, 0)
        .build()
        .unwrap();
    create_box(box_candidate)
}

pub fn create_reserve(public_key: EcPoint) -> ReserveBoxSpec {
    let mut box_candidate = ErgoBoxCandidateBuilder::new(
        BoxValue::SAFE_USER_MIN,
        AddressEncoder::new(ergo_lib::ergotree_ir::chain::address::NetworkPrefix::Mainnet)
            .parse_address_from_str(RESERVE_ADDRESS)
            .unwrap()
            .script()
            .unwrap(),
        0,
    );
    box_candidate.set_register_value(NonMandatoryRegisterId::R4, public_key.into());
    box_candidate.add_token(Token {
        token_id: serde_json::from_str(
            "\"161A3A5250655368566D597133743677397A24432646294A404D635166546A57\"",
        )
        .unwrap(),
        amount: 1.try_into().unwrap(),
    });
    ReserveBoxSpec::try_from(&create_box(box_candidate.build().unwrap())).unwrap()
}

pub fn create_note(public_key: &EcPoint, amount: u64) -> Note {
    let mut note_box_candidate = ErgoBoxCandidateBuilder::new(
        BoxValue::SAFE_USER_MIN,
        AddressEncoder::new(ergo_lib::ergotree_ir::chain::address::NetworkPrefix::Mainnet)
            .parse_address_from_str(NOTE_ADDRESS)
            .unwrap()
            .script()
            .unwrap(),
        0,
    );
    note_box_candidate.add_token(Token {
        token_id: serde_json::from_str(
            "\"4b2d8b7beb3eaac8234d9e61792d270898a43934d6a27275e4f3a044609c9f2a\"",
        )
        .unwrap(),
        amount: amount.try_into().unwrap(),
    });
    let history = NoteHistory::new();
    note_box_candidate.set_register_value(NonMandatoryRegisterId::R4, history.to_avltree().into());
    note_box_candidate.set_register_value(NonMandatoryRegisterId::R5, (*public_key).clone().into());
    note_box_candidate.set_register_value(NonMandatoryRegisterId::R6, 0i64.into());
    Note::new(create_box(note_box_candidate.build().unwrap()), history).unwrap()
}
