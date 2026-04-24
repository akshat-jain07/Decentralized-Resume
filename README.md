# 📄 Decentralized Resume

> **Verifiable, employer-signed work history on the Stellar blockchain — no middlemen, no forgery.**

---

## Project Description

**Decentralized Resume** is a Soroban smart contract deployed on the Stellar network that lets employers issue cryptographically verifiable work-history credentials directly to employees' Stellar addresses.

Traditional resumes are self-reported and trivially falsified. LinkedIn endorsements are social signals, not proofs. Decentralized Resume flips this: credentials are **signed on-chain by the employer**, permanently recorded, and publicly verifiable by anyone — a future hiring manager, a background-check service, or an automated HR pipeline — without trusting a centralised authority.

---

## What It Does

```
Employer                          Smart Contract                     Anyone
   │                                    │                               │
   ├─── register_employer() ───────────►│                               │
   │                                    │                               │
   ├─── issue_credential(employee, …) ─►│                               │
   │         (employer signs tx)        │ stores credential on-chain    │
   │                                    │                               │
   │                                    │◄── verify_credential(id) ─────┤
   │                                    │◄── get_resume(employee) ──────┤
   │                                    │                               │
   ├─── revoke_credential(id) ─────────►│                               │
   │         (if role was misrepresented)│                              │
```

1. **Employers register** once with a name (their Stellar address is their identity).
2. **Employers issue credentials** — each record contains job title, description, start/end dates, and the employer's on-chain signature.
3. **Anyone can verify** a credential by ID, or pull a full resume for any employee address.
4. **Employers can revoke** credentials if they were issued in error or the employment record is disputed.

---

## Features

### 🏢 Employer Registry
- Employers self-register with a display name.
- Registration is tied to a Stellar address — no central whitelist needed.
- Profiles are stored in persistent contract storage and queryable by anyone.

### 🪪 Credential Issuance
- Each credential has a **unique ID** chosen by the employer (e.g. `"acme-eng-2024"`).
- Stores: job title, free-form description, start date, end date (0 = current role).
- Records the **ledger sequence number** at issuance — an immutable timestamp proxy.
- Emits an `ISSUED` event for off-chain indexers.

### ✅ On-Chain Verification
- `verify_credential(id)` → `bool` — single-call check used by integrations.
- `get_credential(id)` → full `WorkCredential` struct for detailed inspection.
- All data is public and permissionlessly readable.

### 📋 Resume View
- `get_resume(employee_address)` returns a **map of all active credentials** for an employee in one call — revoked entries are automatically filtered out.
- `get_employee_credential_ids(employee)` returns the raw list of credential IDs.

### 🚫 Revocation
- Issuing employer can revoke any credential they issued.
- Revoked credentials remain in storage (audit trail) but are excluded from the resume view and fail the `verify_credential` check.
- Emits a `REVOKED` event.

### 🔐 Auth & Access Control
- Every mutating function uses `Address::require_auth()` — no action can be taken without a valid Stellar signature.
- Only the credential issuer can revoke that credential; third parties cannot interfere.
- A contract `admin` is set at deployment (extendable for future governance).

### 🧪 Test Suite
Included unit tests (using `soroban-sdk` testutils) cover:
- Issuing and retrieving a credential
- Verifying and then revoking a credential
- Confirming the resume view excludes revoked entries

---

## Project Structure

```
decentralized-resume/
├── Cargo.toml                        # workspace
└── contracts/
    └── resume/
        ├── Cargo.toml
        └── src/
            └── lib.rs                # full contract + tests
```

---

## Getting Started

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add the WASM target
rustup target add wasm32-unknown-unknown

# Install the Stellar CLI
cargo install --locked stellar-cli --features opt
```

### Build

```bash
stellar contract build
# Output: target/wasm32-unknown-unknown/release/decentralized_resume.wasm
```

### Run Tests

```bash
cargo test
```

### Deploy to Testnet

```bash
# 1. Create / fund a testnet identity
stellar keys generate --global alice --network testnet
stellar keys fund alice --network testnet

# 2. Deploy
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/decentralized_resume.wasm \
  --source alice \
  --network testnet

# 3. Initialise (replace CONTRACT_ID and ADMIN_ADDRESS)
stellar contract invoke \
  --id CONTRACT_ID \
  --source alice \
  --network testnet \
  -- initialize \
  --admin ADMIN_ADDRESS
```

### Invoke Examples

```bash
# Register employer
stellar contract invoke --id $CONTRACT_ID --source employer_key --network testnet \
  -- register_employer \
  --employer $(stellar keys address employer_key) \
  --name '"Acme Corp"'

# Issue a credential
stellar contract invoke --id $CONTRACT_ID --source employer_key --network testnet \
  -- issue_credential \
  --employer  $(stellar keys address employer_key) \
  --employee  EMPLOYEE_ADDRESS \
  --credential_id '"acme-swe-2024"' \
  --job_title '"Senior Software Engineer"' \
  --description '"Led backend platform team, shipped 3 major features"' \
  --start_date 1700000000 \
  --end_date 0

# Verify a credential
stellar contract invoke --id $CONTRACT_ID --network testnet \
  -- verify_credential \
  --credential_id '"acme-swe-2024"'

# Get full resume
stellar contract invoke --id $CONTRACT_ID --network testnet \
  -- get_resume \
  --employee EMPLOYEE_ADDRESS
```

---

## Contract Interface Summary

| Function | Auth Required | Description |
|---|---|---|
| `initialize(admin)` | admin | One-time setup |
| `register_employer(employer, name)` | employer | Register / update employer profile |
| `get_employer(employer)` | — | Read employer profile |
| `issue_credential(employer, employee, id, title, desc, start, end)` | employer | Issue new credential |
| `revoke_credential(employer, credential_id)` | employer (issuer) | Revoke existing credential |
| `get_credential(credential_id)` | — | Fetch single credential |
| `get_employee_credential_ids(employee)` | — | List all credential IDs for employee |
| `get_resume(employee)` | — | Map of all active credentials |
| `verify_credential(credential_id)` | — | Returns `true` if valid and not revoked |
| `get_admin()` | — | Returns admin address |

---

## Roadmap Ideas

- **Employee endorsements** — employees can add notes or accept/reject credentials.
- **Skill tags** — structured skill list per credential for search/filtering.
- **Expiry** — time-bounded credentials (certifications, contracts).
- **Multi-sig revocation** — require both employer and admin to revoke.
- **Frontend dApp** — React UI using the Stellar SDK for a full resume builder experience.

---
wallet address: GD6TFBA5WUI6GOIMHFPNBJG4TKQQYF7BX2KNN4ZZVWFBVPVS23A2IBMJ
contract address: CBA7BYPL5V2VFLKF77QMHC45XT4TQTZDWEBYDU4HKWLHMRVTC7WZCHS7
<img width="1918" height="967" alt="image" src="https://github.com/user-attachments/assets/42cc3d9a-52e5-429b-a9b2-651ed22dc939" />
