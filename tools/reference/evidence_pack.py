#!/usr/bin/env python3
"""Bounded, deterministic packing for new test evidence, never gameplay/save data.

DREFLZ1: eight-byte magic, u32 unpacked size, groups of eight LSB-first flag
bits. Zero is one literal byte; one is a little-endian u16 backward distance
and a u8 length-minus-three. References may overlap. Window65535, length258.
Unused flag bits are zero; trailing bytes reject. No historical files migrate.
"""
import argparse
import collections
import struct
from pathlib import Path

MAGIC = b'DREFLZ1\0'
LIMIT = 16 * 1024 * 1024


def pack(data):
    if len(data) > LIMIT:
        raise ValueError('evidence limit')
    out = bytearray(MAGIC + struct.pack('<I', len(data)))
    chains = {}
    at = 0
    while at < len(data):
        flag_at = len(out)
        out.append(0)
        for bit in range(8):
            if at == len(data):
                break
            key = data[at:at + 3]
            best, distance = 0, 0
            if len(key) == 3:
                for previous in reversed(chains.get(key, ())):
                    if at - previous > 65535:
                        break
                    length = 3
                    while length < min(258, len(data) - at) and data[previous + length] == data[at + length]:
                        length += 1
                    if length > best:
                        best, distance = length, at - previous
            if best >= 4:
                out[flag_at] |= 1 << bit
                out += struct.pack('<HB', distance, best - 3)
                count = best
            else:
                out.append(data[at])
                count = 1
            for index in range(at, at + count):
                key = data[index:index + 3]
                if len(key) == 3:
                    chains.setdefault(key, collections.deque(maxlen=16)).append(index)
            at += count
    return bytes(out)


def unpack(data):
    if len(data) < 12 or data[:8] != MAGIC or len(data) > LIMIT + LIMIT // 8 + 12:
        raise ValueError('evidence framing')
    size = struct.unpack_from('<I', data, 8)[0]
    if size > LIMIT:
        raise ValueError('evidence limit')
    out, at = bytearray(), 12
    while len(out) < size:
        if at == len(data):
            raise ValueError('truncated flags')
        flags = data[at]
        at += 1
        for bit in range(8):
            if len(out) == size:
                if flags >> bit:
                    raise ValueError('unused flag bits')
                break
            if flags & (1 << bit):
                if len(data) - at < 3:
                    raise ValueError('truncated reference')
                distance, count = struct.unpack_from('<HB', data, at)
                at += 3
                count += 3
                if not 0 < distance <= len(out) or count > size - len(out):
                    raise ValueError('invalid reference')
                for _ in range(count):
                    out.append(out[-distance])
            else:
                if at == len(data):
                    raise ValueError('truncated literal')
                out.append(data[at])
                at += 1
    if at != len(data):
        raise ValueError('trailing bytes')
    return bytes(out)


def self_check():
    golden = MAGIC + struct.pack('<I', 9) + b'\x08abc\x03\x00\x03'
    assert unpack(golden) == b'abcabcabc'
    assert pack(b'abcabcabc') == golden
    for data in [b'', bytes(range(256)), b'x' * 1000, bytes(range(256)) * 400]:
        assert unpack(pack(data)) == data
    for invalid in [golden[:-1], golden + b'\0', MAGIC + struct.pack('<I', LIMIT + 1),
                    MAGIC + struct.pack('<I', 3) + b'\1\0\0\0']:
        try:
            unpack(invalid)
        except ValueError:
            pass
        else:
            raise AssertionError('invalid evidence accepted')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--unpack', type=Path)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    self_check()
    if args.unpack:
        if not args.output:
            parser.error('--output is required with --unpack')
        args.output.write_bytes(unpack(args.unpack.read_bytes()))
    else:
        print('Evidence packing schema checks passed')
