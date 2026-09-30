#!/usr/bin/env python3
"""Parse binary DFM (TPF0) into a python tree."""
import struct, json, sys
from pathlib import Path

class R:
    def __init__(s,b): s.b=b; s.p=0
    def u8(s): v=s.b[s.p]; s.p+=1; return v
    def peek(s): return s.b[s.p]
    def take(s,n): v=s.b[s.p:s.p+n]; s.p+=n; return v
    def sstr(s): n=s.u8(); return s.take(n).decode('cp1252','replace')
    def i32(s): return struct.unpack('<i',s.take(4))[0]

class Ident(str): pass

def value(r):
    t=r.u8()
    if t==0: return None
    if t==1:
        items=[]
        while r.peek()!=0: items.append(value(r))
        r.u8(); return {'list':items}
    if t==2: return struct.unpack('<b',r.take(1))[0]
    if t==3: return struct.unpack('<h',r.take(2))[0]
    if t==4: return r.i32()
    if t==5: r.take(10); return '<ext>'
    if t==6: return r.sstr()
    if t==7: return {'ident':r.sstr()}
    if t==8: return False
    if t==9: return True
    if t==10: n=r.i32(); r.take(n); return {'binary':n}
    if t==11:
        items=[]
        while True:
            s=r.sstr()
            if not s: break
            items.append(s)
        return {'set':items}
    if t==12: n=r.i32(); return r.take(n).decode('cp1252','replace')
    if t==13: return None
    if t==14:
        items=[]
        while r.peek()!=0:
            if r.peek() in (2,3,4): value(r)
            assert r.u8()==1
            it={}
            while r.peek()!=0:
                nm=r.sstr(); it[nm]=value(r)
            r.u8(); items.append(it)
        r.u8(); return {'coll':items}
    if t==15: return struct.unpack('<f',r.take(4))[0]
    if t in (16,17,21): return struct.unpack('<d',r.take(8))[0]
    if t==18: n=r.i32(); return r.take(n*2).decode('utf-16-le','replace')
    if t==19: return struct.unpack('<q',r.take(8))[0]
    if t==20: n=r.i32(); return r.take(n).decode('utf-8','replace')
    raise ValueError(t)

def component(r):
    b=r.peek(); flags=0
    if b&0xF0==0xF0:
        flags=r.u8()&0x0F
        if flags&2: value(r)
    cls=r.sstr(); name=r.sstr()
    props={}
    while r.peek()!=0:
        pn=r.sstr(); props[pn]=value(r)
    r.u8()
    kids=[]
    while r.peek()!=0: kids.append(component(r))
    r.u8()
    return {'cls':cls,'name':name,'flags':flags,'props':props,'children':kids}

def parse(data):
    assert data[:4]==b'TPF0'
    r=R(data); r.p=4
    return component(r)

def load_all(d):
    out={}
    for f in sorted(Path(d).iterdir()):
        data=f.read_bytes()
        if data[:4]!=b'TPF0': continue
        out[f.name]=parse(data)
    return out

if __name__=='__main__':
    t=load_all(sys.argv[1])
    json.dump(t,open(sys.argv[2],'w'),indent=1)
    print(len(t))
