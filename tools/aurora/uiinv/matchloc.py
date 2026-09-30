import struct,json,re,collections
d=json.load(open('tlk/dialog.json'))
T={e['id']:e.get('text','') for e in d['entries']}
def norm(s): return re.sub(r'[^a-z0-9]','',s.replace('&','').lower())
idx=collections.defaultdict(set)
for k,v in T.items():
    n=norm(v)
    if n and len(v)<200: idx[n].add(k)
b=open('exe/.data','rb').read()
# int32 occurrences
occ=collections.defaultdict(list)
for o in range(0,len(b)-3,4):
    v=struct.unpack_from('<i',b,o)[0]
    if 0<v<130000: occ[v].append(o)
forms=json.load(open('forms.json'))
def walk(c,acc):
    p=c['props']
    t=p.get('Tag')
    cap=p.get('Caption')
    if cap is None: cap=p.get('Hint')
    if isinstance(t,int) and t>=2 and isinstance(cap,str) and norm(cap):
        acc.append((t,cap,c['name']))
    for k in c['children']: walk(k,acc)
res={}
for fn,f in forms.items():
    acc=[]; walk(f,acc)
    votes=collections.Counter()
    for t,cap,nm in acc:
        for s in idx.get(norm(cap),()):
            for o in occ.get(s,()):
                base=o-t*12
                if base>=0: votes[base]+=1
    if not votes: res[fn]=None; print(fn,'no votes',len(acc)); continue
    (base,v),*rest=votes.most_common(2)+[(None,0)]
    second=rest[0][1] if rest else 0
    res[fn]={'base':base,'votes':v,'n':len(acc),'second':second}
    print(f'{fn:32s} base={base:8d} votes={v:3d}/{len(acc):3d} second={second}')
json.dump(res,open('locbases.json','w'),indent=1)
