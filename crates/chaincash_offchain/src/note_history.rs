//! Note history and methods for signing a note against reserve

use ergo_avltree_rust::{
    authenticated_tree_ops::AuthenticatedTreeOps,
    batch_avl_prover::BatchAVLProver,
    batch_node::{AVLTree, Node, NodeHeader, SerializedAdProof},
    operation::{ADKey, ADValue, KeyValue, Operation},
};
use ergo_lib::{
    ergo_chain_types::{
        blake2b256_hash,
        ec_point::{exponentiate, generator},
        Digest, EcPoint,
    },
    ergotree_interpreter::sigma_protocol::wscalar::Wscalar,
    ergotree_ir::{
        chain::token::TokenId,
        mir::avl_tree_data::{AvlTreeData, AvlTreeFlags},
        serialization::{sigma_byte_writer::SigmaByteWriter, SigmaSerializable},
    },
};
use k256::{
    elliptic_curve::{ops::Reduce, Field, PrimeField},
    FieldBytes,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NoteHistoryError {
    #[error("Attempted to insert duplicate reserve key for note")]
    DuplicateReserveKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct Signature {
    a: EcPoint,
    z: Wscalar,
}

impl Signature {
    pub const SERIALIZED_SIZE: usize = EcPoint::GROUP_SIZE + 32;
    /// Get public image of randomness
    pub fn a(&self) -> &EcPoint {
        &self.a
    }
    /// Get z (computed from randomness and private key)
    pub fn z(&self) -> &Wscalar {
        &self.z
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(Signature::SERIALIZED_SIZE);
        self.write_a_bytes(&mut buf);
        self.write_z_bytes(&mut buf);
        buf
    }

    fn write_a_bytes(&self, buf: &mut Vec<u8>) {
        let mut writer = SigmaByteWriter::new(buf, None);
        self.a.sigma_serialize(&mut writer).unwrap();
    }

    fn write_z_bytes(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(self.z.as_scalar_ref().to_bytes().as_slice());
    }

    pub fn z_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.write_z_bytes(&mut buf);
        buf
    }
}

impl TryFrom<&[u8]> for Signature {
    type Error = String;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != Signature::SERIALIZED_SIZE {
            return Err(format!(
                "Expected {} bytes, received {}",
                Signature::SERIALIZED_SIZE,
                value.len()
            ));
        }
        let a = EcPoint::sigma_parse_bytes(&value[0..33])
            .map_err(|_| String::from("Parsing EcPoint failed"))?;
        let z_opt: Option<k256::Scalar> = k256::Scalar::from_repr(
            FieldBytes::from_exact_iter(value[33..].iter().copied())
                .ok_or_else(|| String::from("Parsing z bytes failed"))?,
        )
        .into();
        Ok(Signature {
            a,
            z: z_opt
                .ok_or_else(|| String::from("Failed to parse z"))?
                .into(),
        })
    }
}

#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[derive(Debug, Clone)]
pub struct SignedOwnershipEntry {
    pub position: u64,
    #[cfg_attr(
        test,
        proptest(
            strategy = "proptest::arbitrary::any_with::<TokenId>(ergo_lib::ergotree_ir::chain::token::arbitrary::ArbTokenIdParam::Arbitrary)"
        )
    )]
    pub reserve_id: TokenId,
    pub amount: u64,
    pub signature: Signature,
}

#[derive(Clone)]
pub struct NoteHistory {
    signatures: Vec<SignedOwnershipEntry>,
}

impl NoteHistory {
    pub fn new() -> Self {
        NoteHistory { signatures: vec![] }
    }
    // Return prover built from signatures. Since BatchAVLProver isn't thread-safe we have to rebuild it each time
    // TODO: make BatchAVLProver thread-safe, then prover can be memoized, otherwise NoteHistory slows down significantly with more signatures
    fn prover(&self) -> Result<BatchAVLProver, NoteHistoryError> {
        build_prover(&self.signatures)
    }
    /// Add a signature and generate insertion proof
    pub fn add_commitment(
        &mut self,
        commitment: SignedOwnershipEntry,
    ) -> Result<SerializedAdProof, NoteHistoryError> {
        let mut prover = self.prover()?;
        let key = commitment
            .reserve_id
            .sigma_serialize_bytes()
            .unwrap()
            .into();
        let value = commitment.signature.serialize().into();
        let insert_op = Operation::Insert(KeyValue { key, value });
        prover
            .perform_one_operation(&insert_op)
            .map_err(|_| NoteHistoryError::DuplicateReserveKey)?;
        self.signatures.push(commitment);
        Ok(prover.generate_proof())
    }
    pub fn digest(&self) -> Digest<33> {
        self.prover().unwrap().digest().unwrap()[..]
            .try_into()
            .unwrap()
    }
    pub fn to_avltree(&self) -> AvlTreeData {
        let tree_flags = AvlTreeFlags::new(true, false, false);
        AvlTreeData {
            digest: self.digest(),
            tree_flags,
            key_length: 32,
            value_length_opt: None,
        }
    }
}

fn build_prover<'a>(
    signatures: impl IntoIterator<Item = &'a SignedOwnershipEntry>,
) -> Result<BatchAVLProver, NoteHistoryError> {
    let mut prover = BatchAVLProver::new(
        AVLTree::new(
            |digest| Node::LabelOnly(NodeHeader::new(Some(*digest), None)),
            32,
            None,
        ),
        true,
    );
    signatures
        .into_iter()
        .map(
            |SignedOwnershipEntry {
                 reserve_id,
                 signature,
                 ..
             }| {
                // Unwrap will only fail if OOM
                let key: ADKey = reserve_id.sigma_serialize_bytes().unwrap().into();
                let value: ADValue = signature.serialize().into();
                Operation::Insert(KeyValue { key, value })
            },
        )
        .try_for_each(|op| {
            prover
                .perform_one_operation(&op)
                .map_err(|_| NoteHistoryError::DuplicateReserveKey)
                .map(|_| ())
        })?;
    prover.generate_proof();
    Ok(prover)
}

pub(crate) fn sign(message: &[u8], private_key: Wscalar) -> Signature {
    let rng = rand::thread_rng();
    let private_key: k256::Scalar = private_key.into();
    let g = generator();

    let public_key = exponentiate(&g, &private_key);
    let r = k256::Scalar::random(rng);
    let a = exponentiate(&g, &r);
    let commitment = [
        &a.sigma_serialize_bytes().unwrap()[..],
        message,
        &public_key.sigma_serialize_bytes().unwrap()[..],
    ]
    .concat();
    let hash: FieldBytes = FieldBytes::clone_from_slice(&blake2b256_hash(&commitment).0);
    let e = <k256::Scalar as Reduce<k256::U256>>::reduce_bytes(&hash);

    let z = r + e * private_key;
    if z.shr_vartime(255) == k256::Scalar::ONE || e.shr_vartime(255) == k256::Scalar::ONE {
        sign(message, Wscalar::from(private_key))
    } else {
        Signature { a, z: z.into() }
    }
}

#[cfg(test)]
mod test {
    use ergo_avltree_rust::authenticated_tree_ops::AuthenticatedTreeOps;
    use ergo_avltree_rust::batch_avl_verifier::BatchAVLVerifier;
    use ergo_avltree_rust::batch_node::{AVLTree, Node, NodeHeader};
    use ergo_avltree_rust::operation::{KeyValue, Operation};
    use ergo_lib::ergotree_ir::serialization::SigmaSerializable;
    use proptest::arbitrary::any;
    use proptest::collection::vec;
    use proptest::proptest;

    use crate::note_history::{build_prover, NoteHistory, Signature, SignedOwnershipEntry};
    proptest! {
        #[test]
        fn test_signature_roundtrip(signature in any::<Signature>()) {
            let serialized = signature.serialize();
            assert_eq!(Signature::try_from(&serialized[..]).unwrap(), signature);
        }
        #[test]
        fn test_prover_verifier(commitments in vec(any::<SignedOwnershipEntry>(), 0..100)) {
            let prover = build_prover(commitments.iter()).unwrap();
            let mut note_history = NoteHistory::new();
            for commitment in commitments {
                let digest = note_history.digest();
                let proof = note_history.add_commitment(commitment.clone()).unwrap();
                let mut bv = BatchAVLVerifier::new(
                            &digest.0.to_vec().into(),
                            &proof,
                            AVLTree::new(
                                |digest| Node::LabelOnly(NodeHeader::new(Some(*digest), None)),
                                32,
                                None,
                            ),
                            None,
                            None,
                        )
                        .unwrap();
                bv.perform_one_operation(&Operation::Insert(KeyValue { key: commitment.reserve_id.sigma_serialize_bytes().unwrap().into(), value: commitment.signature.serialize().into() })).unwrap();
            }
            assert_eq!(note_history.digest(), prover.digest().unwrap().to_vec().try_into().unwrap())
        }
    }
}
