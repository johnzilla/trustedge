use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Key, Nonce,
};
use anyhow::{anyhow, Context, Result};
use bincode::deserialize_from;
use clap::Parser;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::Serialize;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::PathBuf;
use trustedge_audio::{
    build_aad,
    read_preamble_and_header,
    FileHeader,
    Manifest,
    Record,
    StreamHeader,
    HEADER_LEN,
};
use zeroize::Zeroize;

/// Verify `.trst` envelopes for integrity and authenticity.
#[derive(Parser, Debug)]
#[command(name = "trustedge-verify", version, about = "Verify .trst streams")] 
struct Args {
    /// Input `.trst` file. Reads stdin if omitted.
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// 64 hex chars (32 bytes) AES-256 key.
    #[arg(long)]
    key_hex: String,

    /// Emit JSON report instead of human-readable text.
    #[arg(long, default_value_t = false)]
    json: bool,
}

#[derive(Serialize)]
struct Report {
    seq: u64,
    ts_ms: u64,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // input reader
    let reader: Box<dyn Read> = match &args.input {
        Some(p) => Box::new(File::open(p).context("open input")?),
        None => Box::new(io::stdin()),
    };
    let mut r = BufReader::new(reader);

    // AES key
    let mut key_bytes_vec = hex::decode(&args.key_hex).context("decode key")?;
    anyhow::ensure!(key_bytes_vec.len() == 32, "--key-hex must be 64 hex chars");
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&key_bytes_vec);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));

    // header
    let sh: StreamHeader = read_preamble_and_header(&mut r).context("read stream header")?;
    anyhow::ensure!(sh.header.len() == HEADER_LEN, "bad stream header length");
    let header_arr: [u8; HEADER_LEN] = sh
        .header
        .as_slice()
        .try_into()
        .context("stream header length != 58")?;
    let fh = FileHeader::from_bytes(&header_arr);
    let hh = blake3::hash(&sh.header);
    anyhow::ensure!(hh.as_bytes() == &sh.header_hash, "header_hash mismatch");

    // record loop
    let mut expected_seq = 1u64;
    let mut reports = Vec::new();
    loop {
        let rec: Record = match deserialize_from(&mut r) {
            Ok(x) => x,
            Err(err) => {
                if let bincode::ErrorKind::Io(ref e) = *err {
                    if e.kind() == io::ErrorKind::UnexpectedEof {
                        break;
                    }
                }
                return Err(err).context("read record");
            }
        };

        // basic invariants
        anyhow::ensure!(
            rec.nonce[..4] == fh.nonce_prefix,
            "record nonce prefix != stream header nonce_prefix"
        );
        anyhow::ensure!(
            rec.nonce[4..] == rec.seq.to_be_bytes(),
            "record nonce counter != record seq"
        );
        anyhow::ensure!(
            rec.seq == expected_seq,
            "non-contiguous sequence: got {}, expected {}",
            rec.seq,
            expected_seq
        );
        expected_seq = expected_seq
            .checked_add(1)
            .ok_or_else(|| anyhow!("seq overflow"))?;

        // manifest signature
        let pubkey_arr: [u8; 32] = rec
            .sm
            .pubkey
            .as_slice()
            .try_into()
            .context("pubkey length != 32")?;
        let sig_arr: [u8; 64] = rec.sm.sig.as_slice().try_into().context("sig len != 64")?;
        VerifyingKey::from_bytes(&pubkey_arr)
            .context("bad pubkey")?
            .verify(&rec.sm.manifest, &Signature::from_bytes(&sig_arr))
            .context("manifest signature verify failed")?;

        let m: Manifest = bincode::deserialize(&rec.sm.manifest).context("manifest decode")?;
        anyhow::ensure!(m.header_hash == sh.header_hash, "manifest.header_hash mismatch");
        anyhow::ensure!(m.key_id == fh.key_id, "manifest.key_id mismatch");
        anyhow::ensure!(m.seq == rec.seq, "manifest.seq != record.seq");

        // AES-GCM tag verify (decrypt and discard)
        let mh = blake3::hash(&rec.sm.manifest);
        let aad = build_aad(&sh.header_hash, rec.seq, &rec.nonce, mh.as_bytes());
        let pt = cipher
            .decrypt(
                Nonce::from_slice(&rec.nonce),
                Payload {
                    msg: &rec.ct,
                    aad: &aad,
                },
            )
            .map_err(|_| anyhow!("AES-GCM decrypt/verify failed"))?;
        let pt_hash = blake3::hash(&pt);
        anyhow::ensure!(pt_hash.as_bytes() == &m.pt_hash, "pt hash mismatch");

        if args.json {
            reports.push(Report { seq: m.seq, ts_ms: m.ts_ms });
        } else {
            println!("Record {} verified (ts_ms={})", m.seq, m.ts_ms);
        }
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&reports)?);
    } else {
        println!("Verified {} records.", reports.len());
    }

    key_bytes_vec.zeroize();
    key_bytes.zeroize();
    Ok(())
}

