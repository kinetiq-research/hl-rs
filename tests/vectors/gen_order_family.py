"""Writes the order-family cases of tests/vectors/l1_wire.json from hyperliquid-python-sdk.

    python3 tests/vectors/gen_order_family.py /path/to/hyperliquid-python-sdk

Pinned against SDK 0.21.0 (`be7523d`). Covers `order`, `cancel`, `cancelByCloid`, `batchModify`
and `borrowLend`. Each dict is built by the SDK's own `Exchange` method where one exists: the
method runs with `sign_l1_action` and `_post_action` replaced by recorders, so the dict and nonce
are the ones the SDK would have signed and posted. Nothing is signed or sent and no key is
involved. The hash and the EIP-712 `Agent` digest are the SDK's own `action_hash`,
`construct_phantom_agent` and `l1_payload`, hashed with eth_account.

Two forms have no SDK method and are written here, labelled `hand-built`. Their cases show
that hl-rs signs the dict written here; they are not themselves HyperCore measurements. What
HyperCore did with each form is listed with it:

- `cancel` and `cancelByCloid` with the fast flag: the SDK dict with `"f": true` after
  `cancels`, the only form the flag takes (absent when unset). Measured for single cancels on
  testnet: four `f: true` cancels of resting orders went through (2026-10-25, cw-enclave
  `LATENCY.md` §3.3); `cancelByCloid` with `f: true` against a trigger order answered
  `Order not eligible for fast cancel` (2026-10-26); explorer `userDetails` records
  `{"type":"cancel","cancels":[{"a":…,"o":…}],"f":true}` with `error: null` (2026-11-20,
  cw-enclave `crates/cw-executor/tests/fixtures/coreheight`). Multi-cancel bodies with the flag
  are unmeasured.
- `borrowLend`: `{type, operation, token, amount}`, with `amount` a normalised decimal string or
  `null` (the whole balance). Measured on testnet: agent-signed supply and withdraw reached the
  lending module (2026-10-28), and `"1.5"`, `"1.50000000"` and `null` each recovered the
  signer (2026-11-20, cw-enclave `AGENT_AUTHORITY_MATRIX.md` Phase 0).

Every case is built by parsing the dict, so none exercises building an action from CoreWriter
scale; cw-enclave's wire vectors build from `fields` for that.

The script replaces every case of the kinds it writes and leaves the other cases as they are, so
running it twice gives the same file. `fields` holds each case's inputs at CoreWriter scale
(prices, sizes and token amounts in units of 1e-8; cloids as 0x-prefixed hex; `null` for none).
"""
import json
import os
import sys

NONCE = 1_700_000_000_000
HERE = os.path.dirname(os.path.abspath(__file__))
VECTORS = os.path.join(HERE, "l1_wire.json")
ASSETS = {"BTC": 0, "ETH": 4, "HYPE": 159, "@107": 10_107}
# Smallest, mixed, and widest cloids.
C1 = "0x00000000000000000000000000000001"
C2 = "0x0123456789abcdef0123456789abcdef"
C3 = "0xffffffffffffffffffffffffffffffff"
OID = 63_638_909_939
OID_MAX = 2**64 - 1
ORDER_KINDS = {
    "order", "batch-order", "cancel", "cancel-fast", "cancel-by-cloid", "cancel-by-cloid-fast",
    "modify-by-cloid", "modify-by-oid", "batch-modify", "borrow-lend", "borrow-lend-all",
}
# borrowLend supply 1.5 USDC at nonce 42 on testnet, as cw-enclave pins it
# (core_action::a_borrow_lend_hashes_the_normalised_amount). That digest comes from hl-rs, so
# this checks the generator and the enclave build the same dict, not that HyperCore accepts it.
BORROW_LEND_TESTNET_42 = "0xcecdb72d090283d610439b437ca16bba88ca83deb5fb867438ae3c0c72cb46cd"


def scaled(x: str) -> int:
    whole, _, frac = x.partition(".")
    return int(whole) * 10**8 + int((frac + "0" * 8)[:8])


def order_fields(coin, is_buy, sz, px, tif, reduce_only, cloid):
    return {"asset": ASSETS[coin], "is_buy": is_buy, "limit_px": scaled(px), "sz": scaled(sz),
            "reduce_only": reduce_only, "tif": tif, "cloid": cloid}


def sorted_deep(v):
    if isinstance(v, dict):
        return {k: sorted_deep(v[k]) for k in sorted(v)}
    if isinstance(v, list):
        return [sorted_deep(x) for x in v]
    return v


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    sys.path.insert(0, sys.argv[1])
    import msgpack
    from eth_account.messages import encode_typed_data
    from eth_utils import keccak

    import hyperliquid.exchange as ex
    from hyperliquid.utils import signing
    from hyperliquid.utils.constants import MAINNET_API_URL, TESTNET_API_URL
    from hyperliquid.utils.types import Cloid

    recorded = []

    def fake_sign(wallet, action, vault_address, nonce, expires_after, is_mainnet):
        recorded.append((action, vault_address, nonce, expires_after, is_mainnet))
        return {"r": "0x0", "s": "0x0", "v": 27}

    ex.sign_l1_action = fake_sign
    ex.get_timestamp_ms = lambda: NONCE

    class Info:
        @staticmethod
        def name_to_asset(name):
            return ASSETS[name]

    def exchange(base_url):
        e = ex.Exchange.__new__(ex.Exchange)
        e.wallet = None
        e.vault_address = None
        e.account_address = None
        e.expires_after = None
        e.base_url = base_url
        e.info = Info()
        e._post_action = lambda action, signature, nonce: None
        return e

    def capture(call, base_url):
        recorded.clear()
        call(exchange(base_url))
        assert len(recorded) == 1, recorded
        action, vault, nonce, expires, _ = recorded[0]
        assert vault is None and expires is None and nonce == NONCE
        return action

    def digest(connection_id, is_mainnet):
        payload = signing.l1_payload(signing.construct_phantom_agent(connection_id, is_mainnet))
        m = encode_typed_data(full_message=payload)
        return keccak(b"\x19" + m.version + m.header + m.body)

    def cl(c):
        return Cloid.from_str(c) if c else None

    def req(coin, is_buy, sz, px, tif, reduce_only, cloid):
        r = {"coin": coin, "is_buy": is_buy, "sz": float(sz), "limit_px": float(px),
             "order_type": {"limit": {"tif": tif}}, "reduce_only": reduce_only}
        if cloid:
            r["cloid"] = cl(cloid)
        return r

    def order_case(name, *o):
        coin, is_buy, sz, px, tif, reduce_only, cloid = o
        return (name, "order", order_fields(*o),
                lambda e: e.order(coin, is_buy, float(sz), float(px), {"limit": {"tif": tif}},
                                  reduce_only, cl(cloid)))

    def modify_call(target, o):
        coin, is_buy, sz, px, tif, reduce_only, cloid = o
        target = cl(target) if isinstance(target, str) else target
        return lambda e: e.modify_order(target, coin, is_buy, float(sz), float(px),
                                        {"limit": {"tif": tif}}, reduce_only, cl(cloid))

    def modify_req(target, o):
        r = req(*o)
        r["cloid"] = cl(o[6])
        return {"oid": cl(target) if isinstance(target, str) else target, "order": r}

    # (coin, is_buy, sz, px, tif, reduce_only, cloid)
    o_alo_buy = ("ETH", True, "1.5", "3000", "Alo", False, None)
    o_alo_sell = ("HYPE", False, "1.5", "0.1", "Alo", False, C2)
    o_gtc_buy = ("@107", True, "1.5", "0.1", "Gtc", False, None)
    o_gtc_sell = ("ETH", False, "1.5", "3000", "Gtc", False, C1)
    o_ioc_buy = ("BTC", True, "1.5", "3000", "Ioc", True, None)
    o_ioc_sell = ("HYPE", False, "1.5", "0.1", "Ioc", True, C3)
    batch3 = [("ETH", True, "1.5", "3000", "Alo", False, C1),
              ("HYPE", False, "1.5", "0.1", "Gtc", False, C2),
              ("BTC", True, "1.5", "3000", "Ioc", True, C3)]
    batch2 = [("ETH", False, "1.5", "3000", "Alo", False, C2),
              ("@107", True, "1.5", "0.1", "Gtc", False, None)]
    m_alo = ("ETH", True, "1.5", "3000", "Alo", False, C2)
    m_gtc = ("HYPE", False, "1.5", "0.1", "Gtc", False, None)
    m_ioc = ("BTC", False, "1.5", "3000", "Ioc", True, C3)
    mods = [(C1, ("ETH", True, "1.5", "3000", "Alo", False, C2)),
            (C3, ("HYPE", False, "1.5", "0.1", "Gtc", True, C1))]

    cases = [
        order_case("order, Alo buy at 3000, no cloid", *o_alo_buy),
        order_case("order, Alo sell at 0.1, cloid", *o_alo_sell),
        order_case("order, Gtc buy at 0.1 on a spot asset, no cloid", *o_gtc_buy),
        order_case("order, Gtc sell at 3000, smallest cloid", *o_gtc_sell),
        order_case("order, Ioc reduce-only buy at 3000, no cloid", *o_ioc_buy),
        order_case("order, Ioc reduce-only sell at 0.1, widest cloid", *o_ioc_sell),
        ("bulk orders, one per tif, every one with a cloid", "batch-order",
         {"orders": [order_fields(*o) for o in batch3]},
         lambda e: e.bulk_orders([req(*o) for o in batch3])),
        ("bulk orders, two, one without a cloid", "batch-order",
         {"orders": [order_fields(*o) for o in batch2]},
         lambda e: e.bulk_orders([req(*o) for o in batch2])),
        ("cancel by oid", "cancel", {"cancels": [{"asset": 4, "oid": OID}]},
         lambda e: e.cancel("ETH", OID)),
        ("bulk cancel by oid, two, the widest oid", "cancel",
         {"cancels": [{"asset": 4, "oid": OID}, {"asset": 159, "oid": OID_MAX}]},
         lambda e: e.bulk_cancel([{"coin": "ETH", "oid": OID}, {"coin": "HYPE", "oid": OID_MAX}])),
        ("cancel by cloid", "cancel-by-cloid", {"cancels": [{"asset": 159, "cloid": C2}]},
         lambda e: e.cancel_by_cloid("HYPE", cl(C2))),
        ("bulk cancel by cloid, two", "cancel-by-cloid",
         {"cancels": [{"asset": 4, "cloid": C1}, {"asset": 10_107, "cloid": C3}]},
         lambda e: e.bulk_cancel_by_cloid([{"coin": "ETH", "cloid": cl(C1)},
                                           {"coin": "@107", "cloid": cl(C3)}])),
        ("modify by cloid, Alo, new cloid", "modify-by-cloid",
         {"target": C1, "order": order_fields(*m_alo)},
         modify_call(C1, m_alo)),
        ("modify by cloid, Gtc, no new cloid", "modify-by-cloid",
         {"target": C3, "order": order_fields(*m_gtc)},
         modify_call(C3, m_gtc)),
        ("modify by oid, Ioc reduce-only", "modify-by-oid",
         {"target": OID, "order": order_fields(*m_ioc)},
         modify_call(OID, m_ioc)),
        ("bulk modify, two, by cloid", "batch-modify",
         {"modifies": [{"target": t, "order": order_fields(*o)} for t, o in mods]},
         lambda e: e.bulk_modify_orders_new([modify_req(t, o) for t, o in mods])),
    ]

    out = []
    for name, kind, fields, call in cases:
        a_main = capture(call, MAINNET_API_URL)
        a_test = capture(call, TESTNET_API_URL)
        assert a_main == a_test, name
        out.append((name, kind, fields, a_main, "sdk-method"))

    # The fast flag: the SDK's own dict with `f: true` appended after `cancels`.
    for name, kind, call in [
        ("cancel by oid, fast", "cancel-fast", lambda e: e.cancel("ETH", OID)),
        ("bulk cancel by oid, two, fast", "cancel-fast",
         lambda e: e.bulk_cancel([{"coin": "ETH", "oid": OID}, {"coin": "HYPE", "oid": OID_MAX}])),
        ("cancel by cloid, fast", "cancel-by-cloid-fast",
         lambda e: e.cancel_by_cloid("HYPE", cl(C2))),
        ("bulk cancel by cloid, two, fast", "cancel-by-cloid-fast",
         lambda e: e.bulk_cancel_by_cloid([{"coin": "ETH", "cloid": cl(C1)},
                                           {"coin": "@107", "cloid": cl(C3)}])),
    ]:
        action = dict(capture(call, MAINNET_API_URL))
        assert list(action) == ["type", "cancels"], action
        action["f"] = True
        cancels = [({"asset": c["a"], "oid": c["o"]} if "o" in c
                    else {"asset": c["asset"], "cloid": c["cloid"]}) for c in action["cancels"]]
        out.append((name, kind, {"cancels": cancels}, action, "hand-built"))

    # borrowLend: no SDK method. Token 0 is USDC (8 wei decimals, CoreWriter scale).
    for name, kind, op, amount in [
        ("borrow lend, supply 1.5", "borrow-lend", "supply", "1.5"),
        ("borrow lend, withdraw 0.25", "borrow-lend", "withdraw", "0.25"),
        ("borrow lend, supply the whole balance (null)", "borrow-lend-all", "supply", None),
        ("borrow lend, withdraw the whole balance (null)", "borrow-lend-all", "withdraw", None),
    ]:
        out.append((name, kind,
                    {"operation": op, "token": 0, "wei": scaled(amount) if amount else 0},
                    {"type": "borrowLend", "operation": op, "token": 0, "amount": amount},
                    "hand-built"))

    check = {"type": "borrowLend", "operation": "supply", "token": 0, "amount": "1.5"}
    got = "0x" + digest(signing.action_hash(check, None, 42, None), False).hex()
    assert got == BORROW_LEND_TESTNET_42, got

    new = []
    for name, kind, fields, action, source in out:
        conn = signing.action_hash(action, None, NONCE, None)
        new.append(sorted_deep({
            "name": name,
            "kind": kind,
            "fields": fields,
            "source": source,
            "nonce": NONCE,
            "action": action,
            "msgpack": "0x" + msgpack.packb(action).hex(),
            "connection_id": "0x" + conn.hex(),
            "digest_mainnet": "0x" + digest(conn, True).hex(),
            "digest_testnet": "0x" + digest(conn, False).hex(),
        }))

    with open(VECTORS) as f:
        file = json.load(f)
    file["cases"] = [c for c in file["cases"] if c["kind"] not in ORDER_KINDS] + new
    with open(VECTORS, "w") as f:
        # Case keys sorted; `action` key order is not the wire order, `msgpack` is.
        f.write(json.dumps(file, indent=2, ensure_ascii=False) + "\n")
    print(f"wrote {len(new)} order-family cases, {len(file['cases'])} in all")


if __name__ == "__main__":
    main()
