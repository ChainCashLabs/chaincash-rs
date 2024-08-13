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
// This is compiled with reserve/receipt hashes set to "". TODO: change later when testing redeem note
pub const NOTE_ADDRESS: &'static str = "6PS7aYjk1ifeqe4Am4FMbfJcUEpwPvTZeqYTBHTS1CiWhc7RSVQ9UorZtniUwETAgXWYowVSSbD574kiDZhbH2zAuVXzyX5GG6t3gvWJRMgG9BcqGUk7e8c9bQN3AAMwrFzMii8uKBcmRUtBNt1ARAG6ge213msyUaN4vKttkWxyxZdCidUAAdMwbpuMyrUnwKWdkqr1UtHEuAVqLRGHX62rhD5vsBw9GdEvhTCMceeXNiciKhVufRo9FwR7hoQvKWbRpunceF38FHUMMtfa6ik1yPUrn2hFuj5X6xvb9jwBKtqJLqUCmcAcV3uWaLiQRDENLMA6EbPPUNy7W2Pn9G5fBisTknszvs9umrznGnqAUGFdqbXEgGShMYoGzkbPjiFPkzEU3nDNv3W2VNh2fdJ52qPJis2SNyGSP6o8gmuMhXw9QV4jbhrHr3bUJnV1vxUy3NSMXjPiMSpCnfok2pGnfUtTkqzSGtA5anQS1G2gW3ZjSDn81pTWdrbCfjcMuL7aAdFfDpH5WR5eF3xGKgscKCgq67eGG4daHd9x1qh2R74vmJmvMmAqgUZnRyLzPb6kHCBk";
pub const RESERVE_ADDRESS: &'static str = "V5MPcyRpCbWqj3x9wuTCnnDiX4BLX8XunofJYxSN6MD81fGcMePFoiRFUpCBYRQV7BUV7o6cD5Amv3JcxRRSjM12D3JaonJtzaoPEc4R1TS3NTjYysGAtenRuDnJ65sDsUMNMpmW7jtwYqivo8PrwuB93yK83vEBBnAkWqwyFGnsMS44dmkoQZfXprE2cBgG8Wgpv2UN4AN3NfxQ1HZ9gBbfGsLxphnvYdpWLX85XAuZHXZv1rbwZGKaSoZYyYgWpRkVuj3kjNWVbiQLaJP17xw8bv1yXWkR7w26JhhTSfq3JTxp8j1PZJd5n7hfZaf3vJTyYinqM8hwgU9aYcBVRpoSs7HCLQoBBe4QqbEk6txJXYdw75UFpbSYj6t2cBV9Y393c9X7h16DUm8Q9sotJs8kCL1tKqoxpQUP4dXxwTSqqZPGPKzE95mVkRg6UoyRKNpBJsoptash7QK9jro8S8ZebeUMHnTidMwjgGYk91DSPQwfat6FkHLkkEXuwVGaRnwWoqnaN4unqVC4a8B3Zp6iL8UgQFfbqABw5xMrvaZaLMV8aqsnA8H9E2HjSpv1KF73MuNco6geKYm2X6z6QDSBmgxQgeeeSigB4aJNZuATKp8oyg6t2DNWfDvTV8fiwvjqm8dMZ48FpxiXecU5RyRJTgxmmo4fabZN6T7CASFPkAVkgbAbs9RHoFa1b2LAAZUqKQQeAbSfEXZRL8WW5ttv4JYjZhBmg1TBgcdAvvozv22HMKymyYnrTdFp4tAP6KSCoL7joWx6xUvNrWa3txqsEihwXc6a8h7DqkThYerVP9hPjNPnf2eh5PAtBZZRDmo4tJ3zJvutnVHjPbqCjPWeYYtYxYQ4sGoZS25xvc97mCo35xZ9Ld7kynTHsEY6KfYPSM7ivdzxPQjVzQZHPcKTaTY4divEKWeg6jpW4daNBYWgqxUxNJZSo5ErvmVMaiAVhKGHXzrw54fcwSY6s";

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
