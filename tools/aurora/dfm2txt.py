#!/usr/bin/env python3
"""Convert binary Delphi/C++Builder DFM (TPF0) resources to text form."""
import struct
import sys
from pathlib import Path


class R:
    def __init__(self, b):
        self.b = b
        self.p = 0

    def u8(self):
        v = self.b[self.p]
        self.p += 1
        return v

    def peek(self):
        return self.b[self.p]

    def take(self, n):
        v = self.b[self.p:self.p + n]
        self.p += n
        return v

    def sstr(self):
        n = self.u8()
        return self.take(n).decode('cp1252', 'replace')

    def i32(self):
        return struct.unpack('<i', self.take(4))[0]


def value(r, ind):
    t = r.u8()
    if t == 0:
        return 'null'
    if t == 1:
        items = []
        while r.peek() != 0:
            items.append(value(r, ind + 1))
        r.u8()
        return '(' + ' '.join(items) + ')'
    if t == 2:
        return str(struct.unpack('<b', r.take(1))[0])
    if t == 3:
        return str(struct.unpack('<h', r.take(2))[0])
    if t == 4:
        return str(r.i32())
    if t == 5:
        r.take(10)
        return '<ext>'
    if t in (6,):
        return repr(r.sstr())
    if t == 7:
        return r.sstr()
    if t == 8:
        return 'False'
    if t == 9:
        return 'True'
    if t == 10:
        n = r.i32()
        r.take(n)
        return f'{{binary {n} bytes}}'
    if t == 11:
        items = []
        while True:
            s = r.sstr()
            if not s:
                break
            items.append(s)
        return '[' + ', '.join(items) + ']'
    if t == 12:
        n = r.i32()
        return repr(r.take(n).decode('cp1252', 'replace'))
    if t == 13:
        return 'nil'
    if t == 14:
        out = ['<']
        while r.peek() != 0:
            if r.peek() in (2, 3, 4):
                value(r, ind)
            assert r.u8() == 1
            out.append('  ' * (ind + 1) + 'item')
            while r.peek() != 0:
                name = r.sstr()
                out.append('  ' * (ind + 2) + f'{name} = {value(r, ind + 2)}')
            r.u8()
            out.append('  ' * (ind + 1) + 'end')
        r.u8()
        return '\n'.join(out) + '>'
    if t == 15:
        return str(struct.unpack('<f', r.take(4))[0])
    if t in (16, 17, 21):
        return str(struct.unpack('<d', r.take(8))[0])
    if t == 18:
        n = r.i32()
        return repr(r.take(n * 2).decode('utf-16-le', 'replace'))
    if t == 19:
        return str(struct.unpack('<q', r.take(8))[0])
    if t == 20:
        n = r.i32()
        return repr(r.take(n).decode('utf-8', 'replace'))
    raise ValueError(f'unknown value type {t} at {r.p}')


def component(r, ind, out):
    b = r.peek()
    prefix = ''
    if b & 0xF0 == 0xF0:
        flags = r.u8() & 0x0F
        if flags & 2:
            value(r, ind)
        prefix = 'inherited ' if flags & 1 else ('inline ' if flags & 4 else '')
    cls = r.sstr()
    name = r.sstr()
    out.append('  ' * ind + f'{prefix or "object "}{name}: {cls}')
    while r.peek() != 0:
        pname = r.sstr()
        out.append('  ' * (ind + 1) + f'{pname} = {value(r, ind + 1)}')
    r.u8()
    while r.peek() != 0:
        component(r, ind + 1, out)
    r.u8()
    out.append('  ' * ind + 'end')


def convert(data):
    assert data[:4] == b'TPF0', data[:4]
    r = R(data)
    r.p = 4
    out = []
    component(r, 0, out)
    return '\n'.join(out) + '\n'


if __name__ == '__main__':
    src, dst = Path(sys.argv[1]), Path(sys.argv[2])
    dst.mkdir(parents=True, exist_ok=True)
    for f in sorted(src.iterdir()):
        data = f.read_bytes()
        if data[:4] != b'TPF0':
            continue
        try:
            (dst / (f.name + '.dfm')).write_text(convert(data))
        except Exception as e:
            print(f.name, 'FAILED', e, file=sys.stderr)
