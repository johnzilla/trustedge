<!--
Copyright (c) 2025 John Turner
MPL-2.0: https://mozilla.org/MPL/2.0/
Project: trustedge — Privacy and trust at the edge.
GitHub: https://github.com/johnzilla/trustedge
-->
# TrustEdge CLI Reference

Complete command-line interface documentation for TrustEdge.

## Table of Contents
- [CLI Options](#cli-options)
- [Backend Management](#backend-management)
- [Error Handling](#error-handling)
- [Complete Workflows](#complete-workflows)
- [Network Operations](#network-operations)
- [Verification](#verification)

---

## CLI Options

### Core Options

| Option | Description | Example |
|--------|-------------|---------|
| `-i, --input <INPUT>` | Input file (opaque bytes) | `--input document.txt` |
| `-o, --out <OUT>` | Output file for round-tripped plaintext (encrypt mode) or decrypt target (decrypt mode) | `--out decrypted.txt` |
| `--chunk <CHUNK>` | Chunk size in bytes [default: 4096] | `--chunk 8192` |
| `--envelope <ENVELOPE>` | Optional: write envelope (header + records) to this .trst file | `--envelope encrypted.trst` |
| `--no-plaintext` | Skip writing plaintext during encrypt (still verifies+envelopes) | `--no-plaintext` |
| `--decrypt` | Decrypt mode: read .trst from --input and write plaintext to --out | `--decrypt` |

### Key Management Options

| Option | Description | Example |
|--------|-------------|---------|
| `--key-hex <KEY_HEX>` | 64 hex chars (32 bytes) AES-256 key | `--key-hex 0123456789abcdef...` |
| `--key-out <KEY_OUT>` | Where to store generated key (encrypt mode) as hex | `--key-out mykey.hex` |
| `--set-passphrase <PASSPHRASE>` | Store passphrase in OS keyring (one-time setup) | `--set-passphrase "my_secure_passphrase"` |
| `--salt-hex <SALT_HEX>` | Salt for key derivation (32 hex chars = 16 bytes) | `--salt-hex "abcdef1234567890abcdef1234567890"` |
| `--use-keyring` | Use key derived from keyring passphrase + salt instead of --key-hex | `--use-keyring` |

### Backend Options

| Option | Description | Example |
|--------|-------------|---------|
| `--backend <BACKEND>` | Key management backend to use (keyring, tpm, hsm, matter) [default: keyring] | `--backend keyring` |
| `--list-backends` | List available key management backends | `--list-backends` |
| `--backend-config <CONFIG>` | Backend-specific configuration (format: key=value) | `--backend-config "iterations=150000"` |

### Network Options

| Option | Description | Example |
|--------|-------------|---------|
| `--server <ADDRESS>` | Server address for network mode (client) | `--server 127.0.0.1:8080` |
| `--port <PORT>` | Port to listen on (server mode) | `--port 8080` |
| `--output-dir <DIR>` | Directory to save decrypted chunks (server mode) | `--output-dir ./chunks` |

---

## Backend Management

### List Available Backends

```bash
$ trustedge-audio --list-backends
Available key management backends:
  ✓ keyring - OS keyring with PBKDF2 key derivation
    Required config: passphrase, salt

Usage examples:
  --backend keyring --use-keyring --salt-hex <salt>
  --backend tpm --backend-config device_path=/dev/tpm0
  --backend hsm --backend-config pkcs11_lib=/usr/lib/libpkcs11.so
```

### Backend Management Examples

#### Keyring Backend (Default)

```bash
# Set up passphrase (one-time setup)
$ trustedge-audio --set-passphrase "my_secure_passphrase_123"
Passphrase stored in system keyring

# Use keyring with salt for encryption
$ trustedge-audio \
    --input document.txt \
    --out roundtrip.txt \
    --envelope encrypted.trst \
    --backend keyring \
    --salt-hex "abcdef1234567890abcdef1234567890" \
    --use-keyring

# Use specific backend configuration
$ trustedge-audio --backend keyring --backend-config "iterations=150000"
```

#### Future Backends (Planned)

```bash
# TPM 2.0 backend (planned)
$ trustedge-audio --backend tpm --backend-config "device_path=/dev/tpm0"

# HSM backend (planned)  
$ trustedge-audio --backend hsm --backend-config "pkcs11_lib=/usr/lib/libpkcs11.so"

# Matter/Thread ecosystem (planned)
$ trustedge-audio --backend matter --backend-config "device_id=12345"
```

---

## Error Handling

### Common Error Scenarios

#### Unsupported Backend
```bash
$ trustedge-audio --backend tpm
Backend 'tpm' not yet implemented. Available: keyring. Future backends: tpm, hsm, matter. Use --list-backends to see all options
```

#### Missing File
```bash
$ trustedge-audio --decrypt --input nonexistent.trst
Error: open envelope. Caused by: No such file or directory (os error 2)
```

#### Invalid Salt Format
```bash
$ trustedge-audio --salt-hex 'invalid'
Error: salt_hex decode. Caused by: Odd number of digits
```

#### Wrong File Type for Decryption
```bash
$ trustedge-audio --decrypt --input input.mp3 --out output.wav --backend keyring --salt-hex "abcdef1234567890abcdef1234567890"
Error: bad magic
```

#### Wrong Passphrase/Salt Combination
```bash
$ trustedge-audio --decrypt --input test_examples.trst --out output.txt --backend keyring --salt-hex "deadbeefdeadbeefdeadbeefdeadbeef" --use-keyring
Error: AES-GCM decrypt/verify failed
```

---

## Complete Workflows

### Basic Encryption and Decryption

#### 1. Set up keyring passphrase (one-time setup)
```bash
$ trustedge-audio --set-passphrase "my_secure_passphrase_123" --backend keyring
Passphrase stored in system keyring
```

#### 2. Encrypt a file
```bash
$ echo "Hello TrustEdge!" > document.txt
$ trustedge-audio \
    --input document.txt \
    --out roundtrip.txt \
    --envelope encrypted.trst \
    --backend keyring \
    --salt-hex "abcdef1234567890abcdef1234567890" \
    --use-keyring
Round-trip complete. Read 18 bytes, wrote 18 bytes.
```

#### 3. Decrypt the file
```bash
$ trustedge-audio \
    --decrypt \
    --input encrypted.trst \
    --out decrypted.txt \
    --backend keyring \
    --salt-hex "abcdef1234567890abcdef1234567890" \
    --use-keyring
Decrypt complete. Wrote 18 bytes.
```

#### 4. Verify the content
```bash
$ diff document.txt decrypted.txt
(no output = files are identical)
```

### Using Raw Hex Keys

#### Generate and save a key
```bash
$ trustedge-audio \
    --input document.txt \
    --out roundtrip.txt \
    --envelope encrypted.trst \
    --key-out generated_key.hex
Generated AES-256 key: a1b2c3d4e5f6...
Round-trip complete. Read 18 bytes, wrote 18 bytes.
```

#### Use the saved key for decryption
```bash
$ trustedge-audio \
    --decrypt \
    --input encrypted.trst \
    --out decrypted.txt \
    --key-hex $(cat generated_key.hex)
Decrypt complete. Wrote 18 bytes.
```

### Large File Processing

#### Process large files with custom chunk size
```bash
$ trustedge-audio \
    --input large_audio.wav \
    --out large_roundtrip.wav \
    --envelope large_encrypted.trst \
    --chunk 8192 \
    --backend keyring \
    --salt-hex "1234567890abcdef1234567890abcdef" \
    --use-keyring
Round-trip complete. Read 10485760 bytes, wrote 10485760 bytes.
```

#### Encrypt without writing plaintext (envelope only)
```bash
$ trustedge-audio \
    --input sensitive_data.bin \
    --envelope secure.trst \
    --no-plaintext \
    --backend keyring \
    --salt-hex "fedcba0987654321fedcba0987654321" \
    --use-keyring
Envelope created. Input processed but plaintext not written.
```

---

## Network Operations

### Server Mode

#### Start a decrypting server
```bash
$ trustedge-audio \
    --port 8080 \
    --decrypt \
    --use-keyring \
    --salt-hex "networkkey1234567890abcdef1234" \
    --output-dir ./received_chunks
Server listening on 0.0.0.0:8080
Waiting for encrypted chunks...
```

### Client Mode

#### Send encrypted data to server
```bash
$ trustedge-audio \
    --server 127.0.0.1:8080 \
    --input audio_chunk.wav \
    --use-keyring \
    --salt-hex "networkkey1234567890abcdef1234"
Connecting to TrustEdge server at 127.0.0.1:8080
Connected successfully!
Sent chunk 1/1 (4096 bytes)
```

## Verification

The `trustedge-verify` binary checks Ed25519 signatures and AES-GCM tags in a `.trst`
envelope. It reconstructs the additional authenticated data (AAD) for each record
and validates the stream without emitting plaintext. Verification semantics are
defined in [PROTOCOL.md](./PROTOCOL.md).

### Example

```bash
$ trustedge-verify --input encrypted.trst --key-hex 012345...cdef
Record 1 verified (ts_ms=1723372036854)
Verified 1 records.
```

### JSON output

```bash
$ trustedge-verify --input encrypted.trst --key-hex 012345...cdef --json
[
  {"seq":1,"ts_ms":1723372036854}
]
```

---

## Key Management

### Passphrase Management
- `--set-passphrase`: Store a passphrase in the system keyring (run once).
- `--use-keyring`: Use the keyring passphrase for key derivation (PBKDF2). **Mutually exclusive** with `--key-hex`.
- `--salt-hex`: 32-char hex salt for PBKDF2 key derivation (required with `--use-keyring`, must be 16 bytes).

### Key Derivation
- In decrypt mode, you must provide either `--key-hex` or `--use-keyring` (random key is not allowed).
- In encrypt mode, if neither is provided, a random key is generated and optionally saved with `--key-out`.
- **PBKDF2 parameters:** SHA-256, 100,000 iterations, 16-byte (32 hex char) salt.

### Security Notes
- Keys are zeroized after use
- Passphrases are stored securely in OS keyring
- Salt must be consistent between encrypt/decrypt operations
- Different salts produce different keys from the same passphrase

---

## Advanced Configuration

### Backend-Specific Configuration

#### Keyring Backend
```bash
# Custom iteration count
--backend-config "iterations=200000"

# Multiple parameters (future)
--backend-config "iterations=150000,timeout=30"
```

#### TPM Backend (Planned)
```bash
# Specify TPM device
--backend-config "device_path=/dev/tpm0"

# Use specific key handle
--backend-config "key_handle=0x81000001"
```

#### HSM Backend (Planned)
```bash
# PKCS#11 library path
--backend-config "pkcs11_lib=/usr/lib/softhsm/libsofthsm2.so"

# Slot and pin
--backend-config "slot=0,pin=1234"
```

---

For more technical details about the underlying protocol and formats, see [PROTOCOL.md](./PROTOCOL.md).

For examples and use cases, see [EXAMPLES.md](./EXAMPLES.md).
