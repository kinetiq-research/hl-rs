"""Writes tests/vectors/signed_body.json: full /exchange bodies signed by hyperliquid-python-sdk.

    python3 tests/vectors/gen_signed_body.py /path/to/hyperliquid-python-sdk

`l1_wire.json` pins the action, its msgpack and its digest, but carries a placeholder
signature, so nothing there pins how the signature itself is written into the body. The SDK
writes `r` and `s` with eth_utils `to_hex`: lowercase, `0x`-prefixed, no leading zeros. This
signs one `cancel` with the SDK's `sign_l1_action` and builds the body as `Exchange._post_action`
does, for the first nonces from NONCE on whose `r`, and then whose `s`, has a leading zero
nibble, so a padded rendering would differ.

The key is the constant 0x1111...11, a test key that holds nothing. Nothing is sent.
"""
import json
import os
import sys

NONCE = 1_700_000_000_000
KEY = "0x" + "11" * 32
ACTION = {"type": "cancel", "cancels": [{"a": 0, "o": 63_638_909_939}]}
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "signed_body.json")


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    sys.path.insert(0, sys.argv[1])
    from eth_account import Account

    from hyperliquid.utils import signing

    wallet = Account.from_key(KEY)
    cases = []
    for field in ("r", "s"):
        nonce = NONCE
        while True:
            sig = signing.sign_l1_action(wallet, ACTION, None, nonce, None, True)
            if len(sig[field]) < 2 + 64:
                break
            nonce += 1
        body = {
            "action": ACTION,
            "nonce": nonce,
            "signature": sig,
            "vaultAddress": None,
            "expiresAfter": None,
        }
        cases.append({"name": f"short-{field}", "chain": "mainnet", "body": body})
    with open(OUT, "w") as f:
        json.dump({"signer": wallet.address, "cases": cases}, f, indent=2)
        f.write("\n")


if __name__ == "__main__":
    main()
