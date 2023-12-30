use ergo_lib::{
    ergo_chain_types::EcPoint,
    ergotree_ir::{
        chain::ergo_box::{ErgoBox, NonMandatoryRegisterId, RegisterValueError},
        mir::constant::TryExtractInto,
        types::stype::SType,
    },
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to access register")]
    BadRegister(#[from] RegisterValueError),

    #[error("Box missing field: {0}")]
    FieldNotSet(String),

    #[error("Field was set to incorrect type: {0}")]
    InvalidType(SType),
}

pub struct ReserveBoxSpec {
    pub owner: EcPoint,
    pub refund_height: i64,
}

impl TryFrom<ErgoBox> for ReserveBoxSpec {
    type Error = Error;

    fn try_from(value: ErgoBox) -> Result<Self, Self::Error> {
        let owner = value
            .get_register(NonMandatoryRegisterId::R4.into())?
            .ok_or_else(|| Error::FieldNotSet("owner".to_owned()))
            .and_then(|reg| {
                if reg.tpe == SType::SGroupElement {
                    Ok(reg.v.try_extract_into::<EcPoint>().unwrap())
                } else {
                    Err(Error::InvalidType(reg.tpe))
                }
            })?;
        let refund_height = value
            .get_register(NonMandatoryRegisterId::R5.into())?
            .ok_or_else(|| Error::FieldNotSet("refund_height".to_owned()))
            .and_then(|reg| {
                if reg.tpe == SType::SLong {
                    Ok(reg.v.try_extract_into::<i64>().unwrap())
                } else {
                    Err(Error::InvalidType(reg.tpe))
                }
            })?;

        Ok(Self {
            owner,
            refund_height,
        })
    }
}
