use ergo_lib::{
    ergo_chain_types::EcPoint,
    ergotree_ir::{
        chain::ergo_box::{BoxId, ErgoBox, NonMandatoryRegisterId, RegisterValueError},
        mir::constant::TryExtractInto,
        types::stype::SType,
    },
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to access register")]
    BadRegister(#[from] RegisterValueError),

    #[error("Box field was unexpectedly empty: {0}")]
    FieldNotSet(&'static str),

    #[error("Box field '{field}' was set to incorrect type: {tpe}")]
    InvalidType { field: String, tpe: SType },
}

pub struct ReserveBoxSpec {
    pub owner: EcPoint,
    pub refund_height: i64,
    pub identifier: String,
    inner: ErgoBox,
}

impl ReserveBoxSpec {
    pub fn box_id(&self) -> BoxId {
        self.inner.box_id()
    }
    pub fn ergo_box(&self) -> &ErgoBox {
        &self.inner
    }
}

impl TryFrom<&ErgoBox> for ReserveBoxSpec {
    type Error = Error;

    fn try_from(value: &ErgoBox) -> Result<Self, Self::Error> {
        let owner = value
            .get_register(NonMandatoryRegisterId::R4.into())?
            .ok_or_else(|| Error::FieldNotSet("owner"))
            .and_then(|reg| {
                if reg.tpe == SType::SGroupElement {
                    Ok(reg.v.try_extract_into::<EcPoint>().unwrap())
                } else {
                    Err(Error::InvalidType {
                        field: "owner".to_owned(),
                        tpe: reg.tpe,
                    })
                }
            })?;
        let refund_height = value
            .get_register(NonMandatoryRegisterId::R5.into())?
            .ok_or_else(|| Error::FieldNotSet("refund_height"))
            .and_then(|reg| {
                if reg.tpe == SType::SLong {
                    Ok(reg.v.try_extract_into::<i64>().unwrap())
                } else {
                    Err(Error::InvalidType {
                        field: "refund_height".to_owned(),
                        tpe: reg.tpe,
                    })
                }
            })?;
        let identifier = value
            .tokens
            .as_ref()
            .ok_or_else(|| Error::FieldNotSet("reserve box missing NFT"))?
            .get(0)
            .ok_or_else(|| Error::FieldNotSet("token at index 0 missing, no identifier nft"))?
            .token_id;

        Ok(Self {
            owner,
            refund_height,
            identifier: String::from(identifier),
            inner: value.clone(),
        })
    }
}
