//! Note history and methods for signing a note against reserve
use std::collections::HashSet;

use ergo_avltree_rust::{
    authenticated_tree_ops::AuthenticatedTreeOps,
    batch_avl_prover::BatchAVLProver,
    batch_node::{AVLTree, Node, NodeHeader, SerializedAdProof},
    operation::{ADDigest, ADKey, ADValue, KeyValue, Operation},
};
use ergo_lib::{
    ergo_chain_types::{Digest, EcPoint},
    ergotree_interpreter::sigma_protocol::wscalar::Wscalar,
    ergotree_ir::{
        chain::token::TokenId,
        mir::avl_tree_data::{AvlTreeData, AvlTreeFlags},
        serialization::{sigma_byte_writer::SigmaByteWriter, SigmaSerializable},
    },
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NoteHistoryError {
    #[error("Attempted to insert duplicate reserve key for note")]
    DuplicateReserveKey,
}

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct Signature {
    a: EcPoint,
    z: Wscalar,
}

impl Signature {
    /// Get public image of randomness
    pub fn a(&self) -> &EcPoint {
        &self.a
    }
    /// Get z (computed from randomness and private key)
    pub fn z(&self) -> &Wscalar {
        &self.z
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(EcPoint::GROUP_SIZE + 32);
        self.a_bytes(&mut buf);
        self.z_bytes(&mut buf);
        buf
    }

    fn a_bytes(&self, buf: &mut Vec<u8>) {
        let mut writer = SigmaByteWriter::new(buf, None);
        self.a.sigma_serialize(&mut writer).unwrap();
    }

    fn z_bytes(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(self.z.as_scalar_ref().to_bytes().as_slice());
    }
}

pub struct NoteHistory {
    signatures: Vec<(TokenId, Signature)>,
}

impl NoteHistory {
    pub fn new() -> Self {
        NoteHistory { signatures: vec![] }
    }
    // Return prover built from signatures. Since BatchAVLProver isn't thread-safe we have to rebuild it each time
    // TODO: make BatchAVLProver thread-safe, then prover can be memoized
    fn prover(&self) -> Result<BatchAVLProver, NoteHistoryError> {
        build_prover(&self.signatures)
    }
    /// Add a signature and generate insertion proof
    pub fn add_signature(
        &mut self,
        reserve_id: TokenId,
        signature: Signature,
    ) -> Result<SerializedAdProof, NoteHistoryError> {
        let mut prover = self.prover()?;
        let key = reserve_id.sigma_serialize_bytes().unwrap().into();
        let value = signature.serialize().into();
        let insert_op = Operation::Insert(KeyValue { key, value });
        prover
            .perform_one_operation(&insert_op)
            .map_err(|_| NoteHistoryError::DuplicateReserveKey)?;
        self.signatures.push((reserve_id, signature));
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
    signatures: impl IntoIterator<Item = &'a (TokenId, Signature)>,
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
        .map(|(reserve_id, signature)| {
            // Unwrap will only fail if OOM
            let key: ADKey = reserve_id.sigma_serialize_bytes().unwrap().into();
            let value: ADValue = signature.serialize().into();
            Operation::Insert(KeyValue { key, value })
        })
        .try_for_each(|op| {
            prover
                .perform_one_operation(&op)
                .map_err(|_| NoteHistoryError::DuplicateReserveKey)
                .map(|_| ())
        })?;
    prover.generate_proof();
    Ok(prover)
}

#[cfg(test)]
mod test {
    use ergo_avltree_rust::authenticated_tree_ops::AuthenticatedTreeOps;
    use ergo_avltree_rust::batch_avl_verifier::BatchAVLVerifier;
    use ergo_avltree_rust::batch_node::{AVLTree, Node, NodeHeader};
    use ergo_avltree_rust::operation::{KeyValue, Operation};
    use ergo_lib::ergotree_ir::chain::token::{arbitrary::ArbTokenIdParam, TokenId};
    use ergo_lib::ergotree_ir::serialization::SigmaSerializable;
    use proptest::arbitrary::{any, any_with};
    use proptest::collection::vec;
    use proptest::proptest;

    use crate::note_history::{build_prover, NoteHistory, Signature};
    proptest! {
        #[test]
        fn test_prover_verifier(signatures in vec((any_with::<TokenId>(ArbTokenIdParam::Arbitrary), any::<Signature>()), 0..100)) {
            let prover = build_prover(signatures.iter()).unwrap();
            let mut note_history = NoteHistory::new();
            for (reserve_id, signature) in signatures {
                let digest = note_history.digest();
                let proof = note_history.add_signature(reserve_id, signature.clone()).unwrap();
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
                bv.perform_one_operation(&Operation::Insert(KeyValue { key: reserve_id.sigma_serialize_bytes().unwrap().into(), value: signature.serialize().into() })).unwrap();
            }
            assert_eq!(note_history.digest(), prover.digest().unwrap().to_vec().try_into().unwrap())
        }
    }
}
