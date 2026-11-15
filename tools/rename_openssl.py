#!/usr/bin/env python3
"""Rename the OpenSSL libraries a binary asks for, keeping the strings' length.

The recovery's own libcrypto is OpenSSL 3, which no longer exports the stack
functions a vendor wpa_supplicant was built against. OpenSSL 1.1.1 is built for
it instead, under names of the same length so that the DT_NEEDED strings can be
rewritten in place.
"""
import sys

RENAMES = [
    (b"libcrypto.so.1.1\0", b"libc11.so\0"),
    (b"libcrypto.so\0", b"libc11.so\0"),
    (b"libssl.so.1.1\0", b"libs11.so\0"),
    (b"libssl.so\0", b"libs11.so\0"),
]


def patch(path: str) -> None:
    data = bytearray(open(path, "rb").read())
    changed = 0
    for old, new in RENAMES:
        start = 0
        while True:
            at = data.find(old, start)
            if at < 0:
                break
            pad = len(old) - len(new)
            data[at:at + len(old)] = new + b"\0" * pad
            changed += 1
            start = at + len(old)
    open(path, "wb").write(data)
    print(f"{path}: {changed} names rewritten")


if __name__ == "__main__":
    for p in sys.argv[1:]:
        patch(p)
