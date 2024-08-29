use ergo_lib::{
    chain::ergo_box::box_builder::ErgoBoxCandidateBuilder,
    ergo_chain_types::EcPoint,
    ergotree_ir::{
        chain::{
            address::{Address, AddressEncoder},
            ergo_box::{box_value::BoxValue, ErgoBox, ErgoBoxCandidate, NonMandatoryRegisterId},
            token::{Token, TokenAmount, TokenId},
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
pub const NOTE_ADDRESS: &'static str = "5ARVo4rkBiy9tNbYHUx2AzdzvkXEuvqD67tZrZjh59MP5vA5GRSHeEJNx2X5Sy1BBfuS365MfVFNTs8x493SwnDSQTnWU4qmJ3psQjfF3z4U9KeeHXzjynm2ecRwjxZkNjNduRv8rFYj9SGMzbmrwQcLrh8QuqA8MdFtQeoaNy9ZDRFoKueVGpqzUmsg3LMceuCSGKXrBwtaXQ28JECE7zrMkDwj17GCAaz5m14hafvUFsuiiurSBsEJpuaA8SnrTdrm1h8tJb49yWNBUm8Knj4eXFJUGdjXQspRMRq6viWhWBuo7XX2jN6pms3Vv16ydsPFcnEoAE1iAEZMEjezfGY3uS1Gpa1ZUrKTehQRuUws4tK3gDhHSTh5xoErzHbj72CgwdqNuWAFYD67KFSibi2WEhKTgCY6zz9joaF4uj69CqSXThkHERoqrgkyoYK8Sp1DCptA9s31PSaaCUHPvdZ6MmXiAX6aFW54BZxp92x8kbnwEJxqc9v2fYNxHi4tKPsC4iKWFUe5TCbLNsabeCD5zMqqJz11dxdVXWY1PyrcVoNgzSPWui25oWzY2PK5FC41sJCAyfdAxL1dfq7nUH4wjAFVC1VRB4Sh5yiQ45BBM4vyPZgquraALtmCc1fiuoFUr2ZAQGyLPBtQ7gzxnRmALE";
pub const RESERVE_ADDRESS: &'static str = "2Ra2gepQYdZrJBu1rYoYXaZtkNKfpQYgRNLba6u5aNpmLqAcU2wXjjT2ouKcVZYkJtbZ42rxDJE1RS9sYVpmsQcci2hbYzW36f4Ei8fMNbsPso4S2vfyMMwqnGy11PYhmpGeFxokibXUcNo5NrmEPsEvY83yHMbMKqBfxbzSbci3JeRLRbLNrcu41yRjcoGdL4t6Uvhqfw62umrxQiJovXre1YdHRbWHafFgQioXgXpioR7TAnpaHtsag7bafXz3DdGC93mPs8SUjpr1VCyoCxhLbK16DbfAxEgDmfCvEp7qTT4RdewpbayMnqnpckiiwxLJhA7DobJz2Ao2o9KqytM4nyMK7SFST1UCdzUCHrN93LugFguTszUqZw3HCTBf3GxFKKDtZqY9YY4gPAUhNMVkHbnRRkpJMQcDywtxgxEWWU3XusoTDXFfXaDp8NrMsjrmmoz11zhSPLd52PVcjUoSyzuxBXXPCUmse3A59EcnVMSet1rNeKfmPJWZQ75Hg4YHEB4u58JF9gyWFDgMdzrXFFBUhRH8rubcxhvP6xytq1cweJhUtAwjaXiyDt997zLVzCzMB6jpdMeCAr8ea65CbYJN8ht39vjCABKMAfUKe3Rmk3HaTJdxXokKDtrBWYECoJ6QGk3XfYXVurS5ZjDkVpekbqSawi48NkRk2qPDW7ZKb6GJ3vpFpj745j3ezhx7VX72MXTUBNUWvEN5TZBhUsM1y2JCaxuFgMd7Ri2ewjK1aKs9WDtjznjbL22VqzxLdSVGtLT7pu6LydzEjSGa7fgYzJKL81mfvkWFhXjsGpu4jA8tXMattmdty2idUdLeSN95RVskRM2Lkrdmua1byWeuAo67MyZMkfkGV5mzm7thtEMjMHhirp2LZeB89bfAso4mtSsFSjCF9sToF895MVcjgq93cZhtXid93mSfrRVXFNxtZz5HHjHM6yaRpRVVHNV7DqXtVmNxBwZ";
pub const RECEIPT_ADDRESS: &'static str = "dW86NzEXDeRHC2ESgUABHri1DfQ1bWHPmkWVMkWvWakZ6PuSQh8MCMXquHDN941XW7kNeYbQtxWtjnSAhKqohVNmNa56TeXS1iev5UM7G71ksMu8HhMAvhv58MZVSffYBqS1YuAZQXUBbPjZLzyQCJnjHjBXZY7s2TMsjYH";
pub const BUYBACK_ADDRESS: &'static str = "43xhWcMKGeNA8eJ1oFuJGzcCyiBH84t5ViuMLx5BddqmAew1JjhEXkiL5bJsdBktBhvFkpyFb5WJ5m24jT1Kz978h8Mb9Z61feYbCZjLTuF2skSCfQWvsrXpcsTGY2pnVpZn4fe32CijGymD2M4UKdc1YMU8fh6TRJJUfHCra9xkX98cajwsrWUE6aFC7Ck7y4rKA9vNSNsBr1sCFt2i5dj7Cci5Ez3F8QZ8xMkZioAGh6MCvupHuPefnkPXtqBtZxwZ6ve5Nk24oudcRagnepk3ipTMRyxA57sfaNwXAij98doHk2mBU7Li51TD3REpjifjUPmA6DX1U3UMCnuDgAZzdSkdgxanPWzRbxbxEoDyxrieN7TQyirHK9dBpp6iJjaE6ave1Y8TZWCizBvWUsXcEuLWGZczDumdstzDs6zvLk4f8MWTVRSkqorwFfvDRSYK26TbsUbD5dLUw4QW8QCYVzER542syvMKp1MNUyM8bo9kRD2Vtb75V3eZV1Camr1h46nMQLLw3bUq2T9P1wRqCHb7kfqKpR1hDLMqmpEeNNig4WJ6BVnBqzfT7MVesYinRco881NDvJnVi22JNx7CXdFKjWm88fG3UcjmBpG8LWedKiybSv3GxZdf9reZrigTZpMyBgwc5YWN4uF9pKWVr25ZHMnqZK8r5FdbrW5x4k8bW1hoLCe5aXLKXzBYyBNt1wZk33qCE4gJ5qu4dyD9eR1F32vY787vBYbSXrsNiF5iN1MhSJysFy4pQVvePMrxePQTEuqYytJCDGFxbaMiU3mai1GwJVTgnQysQWceSE959HgYBxALsTGisWGCjjfYN7QxnjPbbD36ybsei9VhYXu6";

pub fn create_box(box_candidate: ErgoBoxCandidate) -> ErgoBox {
    let mut rng = thread_rng();
    ErgoBox::from_box_candidate(
        &box_candidate,
        force_any_val(),
        rng.gen_range(0..i16::MAX as u16),
    )
    .unwrap()
}

pub fn create_oracle_box(nanoerg_per_kg: i64) -> ErgoBox {
    // script doesn't matter since oracle box is only used for data input
    let mut box_candidate = ErgoBoxCandidateBuilder::new(
        BoxValue::SAFE_USER_MIN,
        AddressEncoder::new(ergo_lib::ergotree_ir::chain::address::NetworkPrefix::Mainnet)
            .parse_address_from_str(RESERVE_ADDRESS)
            .unwrap()
            .script()
            .unwrap(),
        0,
    );
    box_candidate.set_register_value(NonMandatoryRegisterId::R4, nanoerg_per_kg.into());
    create_box(box_candidate.build().unwrap())
}

pub fn create_buyback_box() -> ErgoBox {
    let mut box_candidate = ErgoBoxCandidateBuilder::new(
        BoxValue::SAFE_USER_MIN,
        AddressEncoder::new(ergo_lib::ergotree_ir::chain::address::NetworkPrefix::Mainnet)
            .parse_address_from_str(BUYBACK_ADDRESS)
            .unwrap()
            .script()
            .unwrap(),
        0,
    );
    box_candidate.add_token(Token {
        token_id: TokenId::from_base64("EZoGigEZZw3opdJGfaM99XKQPGSqp7bqTJZo7wz+AyU=").unwrap(),
        amount: TokenAmount::try_from(1).unwrap(),
    });
    create_box(box_candidate.build().unwrap())
}

pub fn create_wallet_box(public_key: EcPoint, amount: u64) -> ErgoBox {
    let tree = Address::P2Pk(ProveDlog::new(public_key)).script().unwrap();
    let box_candidate = ErgoBoxCandidateBuilder::new(BoxValue::new(amount).unwrap(), tree, 0)
        .build()
        .unwrap();
    create_box(box_candidate)
}

pub fn create_reserve(public_key: EcPoint, amount: u64) -> ReserveBoxSpec {
    let mut box_candidate = ErgoBoxCandidateBuilder::new(
        BoxValue::new(amount).unwrap(),
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
