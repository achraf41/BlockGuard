use crate::block::BlockHeader;

pub const BLOCK_HEADER_DOMAIN: &[u8] = b"BLOCKGUARD_BLOCK_V1";

pub const BLOCK_HEADER_ENCODING_LENGTH: usize = BLOCK_HEADER_DOMAIN.len()
        + 2  // version
        + 4  // chain_id
        + 8  // height
        + 32 // previous_block_hash
        + 32 // transactions_root
        + 32 // state_root
        + 8  // timestamp
        + 8; // pow nonce

pub fn encode_block_header_for_hash(header: &BlockHeader) -> [u8; BLOCK_HEADER_ENCODING_LENGTH] {
    let mut output = [0u8; BLOCK_HEADER_ENCODING_LENGTH];
    let mut offset = 0;

    write_bytes(&mut output, &mut offset, BLOCK_HEADER_DOMAIN);

    write_bytes(
        &mut output,
        &mut offset,
        &header.version().value().to_be_bytes(),
    );

    write_bytes(
        &mut output,
        &mut offset,
        &header.chain_id().value().to_be_bytes(),
    );

    write_bytes(
        &mut output,
        &mut offset,
        &header.height().value().to_be_bytes(),
    );

    let previous_block_hash = header.previous_block_hash();

    write_bytes(&mut output, &mut offset, previous_block_hash.as_bytes());

    let transaction_root = header.transaction_root();

    write_bytes(&mut output, &mut offset, transaction_root.as_bytes());

    let state_root = header.state_root();

    write_bytes(&mut output, &mut offset, state_root.as_bytes());

    write_bytes(
        &mut output,
        &mut offset,
        &header.timestamp().value().to_be_bytes(),
    );

    write_bytes(
        &mut output,
        &mut offset,
        &header.pow_nonce().value().to_be_bytes(),
    );

    debug_assert_eq!(offset, BLOCK_HEADER_ENCODING_LENGTH);

    output
}

fn write_bytes(output: &mut [u8], offset: &mut usize, bytes: &[u8]) {
    let end = *offset + bytes.len();

    output[*offset..end].copy_from_slice(bytes);

    *offset = end;
}
