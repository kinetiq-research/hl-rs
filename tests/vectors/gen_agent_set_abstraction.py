"""Prints the agentSetAbstraction vectors pinned in src/actions/l1_actions/agent_set_abstraction.rs.

    python3 tests/vectors/gen_agent_set_abstraction.py /path/to/hyperliquid-python-sdk

Pinned against hyperliquid-python-sdk `2fdb18f`; `Exchange.agent_set_abstraction` builds the same
dict since 0.22.0 (`b4d2d1b`). Each dict is built by that method, with `sign_l1_action` and
`_post_action` replaced by recorders: nothing is signed or sent and no key is involved. The hash
and the EIP-712 `Agent` digest are the SDK's own `action_hash`, `construct_phantom_agent` and
`l1_payload`, hashed with eth_account; vault address and expiry are unset.
"""
import json
import sys

NONCE = 1_700_000_000_000


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    sys.path.insert(0, sys.argv[1])
    import msgpack
    from eth_account.messages import encode_typed_data
    from eth_utils import keccak

    import hyperliquid.exchange as ex
    from hyperliquid.utils import signing
    from hyperliquid.utils.constants import MAINNET_API_URL

    recorded = []

    def fake_sign(wallet, action, vault_address, nonce, expires_after, is_mainnet):
        recorded.append((action, vault_address, nonce, expires_after))
        return {"r": "0x0", "s": "0x0", "v": 27}

    ex.sign_l1_action = fake_sign
    ex.get_timestamp_ms = lambda: NONCE

    e = ex.Exchange.__new__(ex.Exchange)
    e.wallet = None
    e.vault_address = None
    e.account_address = None
    e.expires_after = None
    e.base_url = MAINNET_API_URL
    e._post_action = lambda action, signature, nonce: None

    def digest(connection_id, is_mainnet):
        payload = signing.l1_payload(signing.construct_phantom_agent(connection_id, is_mainnet))
        m = encode_typed_data(full_message=payload)
        return keccak(b"\x19" + m.version + m.header + m.body)

    out = []
    for mode in ["i", "u", "p"]:
        recorded.clear()
        e.agent_set_abstraction(mode)
        assert len(recorded) == 1, recorded
        action, vault, nonce, expires = recorded[0]
        assert vault is None and expires is None and nonce == NONCE
        connection_id = signing.action_hash(action, None, NONCE, None)
        out.append({
            "action": action,
            "nonce": NONCE,
            "msgpack": msgpack.packb(action).hex(),
            "digest_mainnet": "0x" + digest(connection_id, True).hex(),
            "digest_testnet": "0x" + digest(connection_id, False).hex(),
        })
    print(json.dumps(out, indent=2))


if __name__ == "__main__":
    main()
