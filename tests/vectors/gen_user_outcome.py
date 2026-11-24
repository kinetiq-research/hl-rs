"""Writes tests/vectors/user_outcome.json, the userOutcome vectors pinned by tests/user_outcome_vectors.rs.

    python3 tests/vectors/gen_user_outcome.py /path/to/hyperliquid-python-sdk > tests/vectors/user_outcome.json

The SDK (0.21.0, `be7523d`) has no method for `userOutcome`, so each case is the action dict as
HyperCore testnet received it on 2026-11-23, key order included:

- `splitOutcome` and `mergeOutcome` (amount `null`) were signed by an approved agent and accepted:
  the split credited the master's spot with both side tokens of outcome 11074, and the merge
  returned the USDC to it.
- `mergeQuestion` (amount `null`) and `negateOutcome` were signed by a key approved nowhere, and
  the answer named that key: HyperCore recovered the signer from these bytes, so they hash as sent.

The remaining cases (`mergeOutcome` and `mergeQuestion` with an amount) follow the documented
shapes and were not sent. Each hash is the SDK's own `action_hash`, `construct_phantom_agent` and
`l1_payload`, hashed with eth_account, at nonce 1700000000000 with vault and expiry unset. Nothing is
signed and no key is involved.
"""
import json
import sys

NONCE = 1_700_000_000_000

CASES = [
    ("split, accepted", "accepted",
     {"type": "userOutcome", "splitOutcome": {"outcome": 11074, "amount": "1.0"}}),
    ("merge all, accepted", "accepted",
     {"type": "userOutcome", "mergeOutcome": {"outcome": 11074, "amount": None}}),
    ("merge question all, signer recovered", "signer-recovered",
     {"type": "userOutcome", "mergeQuestion": {"question": 1283, "amount": None}}),
    ("negate, signer recovered", "signer-recovered",
     {"type": "userOutcome", "negateOutcome": {"question": 1283, "outcome": 17209, "amount": "1.0"}}),
    ("merge an amount, documented shape", "documented",
     {"type": "userOutcome", "mergeOutcome": {"outcome": 20115, "amount": "0.5"}}),
    ("merge question an amount, documented shape", "documented",
     {"type": "userOutcome", "mergeQuestion": {"question": 820, "amount": "123.0"}}),
]


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    sys.path.insert(0, sys.argv[1])
    import msgpack
    from eth_account.messages import encode_typed_data
    from eth_utils import keccak

    from hyperliquid.utils import signing

    def digest(connection_id, is_mainnet):
        payload = signing.l1_payload(signing.construct_phantom_agent(connection_id, is_mainnet))
        m = encode_typed_data(full_message=payload)
        return keccak(b"\x19" + m.version + m.header + m.body)

    out = []
    for name, source, action in CASES:
        connection_id = signing.action_hash(action, None, NONCE, None)
        out.append({
            "name": name,
            "source": source,
            "action": action,
            "nonce": NONCE,
            "msgpack": "0x" + msgpack.packb(action).hex(),
            "connection_id": "0x" + connection_id.hex(),
            "digest_mainnet": "0x" + digest(connection_id, True).hex(),
            "digest_testnet": "0x" + digest(connection_id, False).hex(),
        })
    json.dump({
        "provenance": "tests/vectors/gen_user_outcome.py; see its docstring",
        "sdk": "hyperliquid-python-sdk 0.21.0 (be7523d)",
        "cases": out,
    }, sys.stdout, indent=2)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
