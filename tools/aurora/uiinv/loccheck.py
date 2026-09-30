import json,struct,re,sys
d=json.load(open('tlk/dialog.json'))
T={e['id']:e.get('text','') for e in d['entries']}
b=open('../exe/.data','rb').read()
L=json.load(open('locbases.json'))
forms=json.load(open('forms.json'))
def norm(s): return re.sub(r'[^a-z0-9]','',s.replace('&','').lower())
def walk(c,acc):
    p=c['props']; t=p.get('Tag')
    if isinstance(t,int) and t>=1: acc.append((t,p.get('Caption', p.get('Hint')),c['name'],c['cls']))
    for k in c['children']: walk(k,acc)
for fn in sys.argv[1:]:
    base=L[fn]['base']; acc=[]; walk(forms[fn],acc)
    print('=====',fn,L[fn])
    for t,cap,nm,cls in sorted(acc,key=lambda x:(x[0],x[2])):
        r=struct.unpack_from('<3i',b,base+t*12)
        tc=T.get(r[0],'') if r[0]>0 else ''
        hint=T.get(r[1],'') if r[1]>0 else ''
        flag='' if (cap and norm(cap)==norm(tc)) else ' <<'
        print(f'  T{t:<3} {nm:28s} dfm={cap!r:34.34} tlk={tc!r:34.34} hint={hint[:50]!r}{flag}')
