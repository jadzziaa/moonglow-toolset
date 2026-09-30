import struct,json,re
d=json.load(open('tlk/dialog.json'))
T={e['id']:e.get('text','') for e in d['entries']}
b=open('exe/.data','rb').read()
pat=b'\x00'*12+b'\xff'*12
starts=[m.start() for m in re.finditer(re.escape(pat),b) if m.start()%4==0]
tables=[]
for s in starts:
    recs=[]
    o=s
    while o+12<=len(b):
        a=struct.unpack('<3i',b[o:o+12])
        if all((x==-1) or (0<=x<=200000) for x in a) and (len(recs)<2 or any(x!=0 for x in a)):
            recs.append(a); o+=12
        else: break
    tables.append((s,recs))
print(len(starts))
json.dump([(s,r) for s,r in tables],open('loctables.json','w'))
for s,r in tables[:5]: print(s,len(r))
print(sorted(len(r) for s,r in tables))
