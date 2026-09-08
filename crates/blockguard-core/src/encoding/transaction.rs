use crate::transaction::{SignedTransaction, UnsignedTransaction};

pub const SIGNING_DOMAIN: &[u8; 16] = b"BLOCKGUARD_TX_V1";

pub const SIGNED_TRANSACTION_DOMAIN: &[u8; 23] = b"BLOCKGUARD_SIGNED_TX_V1";

const UNSIGNED_TRANSACTION_FIELDS_LENGTH: usize = 2  // version
    + 4  // chain_id
    + 33 // sender public key
    + 20 // recipient
    + 8  // amount
    + 8; // nonce

pub const SIGNING_PREIMAGE_LENGTH: usize =
    SIGNING_DOMAIN.len() + UNSIGNED_TRANSACTION_FIELDS_LENGTH;

pub const SIGNED_TRANSACTION_ENCODING_LENGTH: usize =
    SIGNED_TRANSACTION_DOMAIN.len() + UNSIGNED_TRANSACTION_FIELDS_LENGTH + 64; // signature

fn write_bytes(output: &mut [u8], offset: &mut usize, bytes: &[u8]) {
    let end = *offset + bytes.len();

    output[*offset..end].copy_from_slice(bytes);

    *offset = end;
}

fn write_unsigned_fields(tx: &UnsignedTransaction, output: &mut [u8], offset: &mut usize) {
    write_bytes(output, offset, &tx.version().value().to_be_bytes());
    write_bytes(output, offset, &tx.chain_id().value().to_be_bytes());
    write_bytes(output, offset, tx.sender_public_key().as_bytes());
    write_bytes(output, offset, tx.recipient().as_bytes());
    write_bytes(output, offset, &tx.amount().value().to_be_bytes());
    write_bytes(output, offset, &tx.nonce().as_u64().to_be_bytes());
}

pub fn encode_unsigned_transaction_for_signing(
    tx: &UnsignedTransaction,
) -> [u8; SIGNING_PREIMAGE_LENGTH] {
    let mut output = [0u8; SIGNING_PREIMAGE_LENGTH];

    let mut offset = 0;

    write_bytes(&mut output, &mut offset, SIGNING_DOMAIN);

    write_unsigned_fields(tx, &mut output, &mut offset);
    output
}

pub fn encode_signed_transaction_for_id(
    tx: &SignedTransaction,
) -> [u8; SIGNED_TRANSACTION_ENCODING_LENGTH] {
    let mut output = [0u8; SIGNED_TRANSACTION_ENCODING_LENGTH];

    let mut offset = 0;

    write_bytes(&mut output, &mut offset, SIGNED_TRANSACTION_DOMAIN);

    write_unsigned_fields(tx.payload(), &mut output, &mut offset);

    write_bytes(&mut output, &mut offset, tx.signature().as_bytes());
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        transaction::UnsignedTransaction,
        types::{Address, Amount, ChainId, Nonce, PublicKeyBytes, TransactionVersion},
    };

    fn sample_transaction() -> UnsignedTransaction {
        let mut public_key = [0u8; 33];

        // A compressed secp256k1 key normally starts
        // with 0x02 or 0x03.
        public_key[0] = 0x02;

        for (index, byte) in public_key[1..].iter_mut().enumerate() {
            *byte = (index + 1) as u8;
        }

        let mut recipient = [0u8; 20];

        for (index, byte) in recipient.iter_mut().enumerate() {
            *byte = (0xA0 + index) as u8;
        }

        UnsignedTransaction::new(
            TransactionVersion::V1,
            ChainId::new(1),
            PublicKeyBytes::from_bytes(public_key),
            Address::from_bytes(recipient),
            Amount::new(500),
            Nonce::new(7),
        )
    }

    #[test]
    fn signing_encoding_has_expected_length() {
        let tx = sample_transaction();

        let encoded = encode_unsigned_transaction_for_signing(&tx);

        assert_eq!(encoded.len(), SIGNING_PREIMAGE_LENGTH);

        assert_eq!(encoded.len(), 91);
    }

    #[test]
    fn encoding_is_deterministic() {
        let tx = sample_transaction();

        let first = encode_unsigned_transaction_for_signing(&tx);

        let second = encode_unsigned_transaction_for_signing(&tx);

        assert_eq!(first, second);
    }

    #[test]
    fn changing_amount_changes_encoding() {
        let tx1 = sample_transaction();

        let tx2 = UnsignedTransaction::new(
            tx1.version(),
            tx1.chain_id(),
            *tx1.sender_public_key(),
            *tx1.recipient(),
            Amount::new(501),
            tx1.nonce(),
        );

        let encoded1 = encode_unsigned_transaction_for_signing(&tx1);

        let encoded2 = encode_unsigned_transaction_for_signing(&tx2);

        assert_ne!(encoded1, encoded2);
    }

    #[test]
    fn test_vector_matches_protocol_encoding() {
        let tx = sample_transaction();

        let encoded = encode_unsigned_transaction_for_signing(&tx);

        let mut expected = Vec::new();

        // Domain
        expected.extend_from_slice(b"BLOCKGUARD_TX_V1");

        // version = 1 as u16 big-endian
        expected.extend_from_slice(&1u16.to_be_bytes());

        // chain_id = 1 as u32 big-endian
        expected.extend_from_slice(&1u32.to_be_bytes());

        // public key
        let mut public_key = [0u8; 33];
        public_key[0] = 0x02;

        for (index, byte) in public_key[1..].iter_mut().enumerate() {
            *byte = (index + 1) as u8;
        }

        expected.extend_from_slice(&public_key);

        // recipient
        let mut recipient = [0u8; 20];

        for (index, byte) in recipient.iter_mut().enumerate() {
            *byte = (0xA0 + index) as u8;
        }

        expected.extend_from_slice(&recipient);

        // amount = 500
        expected.extend_from_slice(&500u64.to_be_bytes());

        // nonce = 7
        expected.extend_from_slice(&7u64.to_be_bytes());

        assert_eq!(encoded.as_slice(), expected.as_slice());
    }
}
