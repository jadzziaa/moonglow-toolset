#!/usr/bin/env python3
import json, sys
from pathlib import Path

VK={8:'Backspace',9:'Tab',13:'Enter',27:'Esc',32:'Space',33:'PgUp',34:'PgDn',35:'End',36:'Home',37:'Left',38:'Up',39:'Right',40:'Down',45:'Ins',46:'Del',
    107:'Num+',109:'Num-',106:'Num*',111:'Num/',187:'=',189:'-',188:',',190:'.',191:'/',219:'[',221:']',186:';',222:"'",192:'`',220:'\\'}
for i in range(1,25): VK[111+i]=f'F{i}'
for c in range(48,58): VK[c]=chr(c)
for c in range(65,91): VK[c]=chr(c)
for i in range(10): VK[96+i]=f'Num{i}'
def sc(v):
    if not v: return ''
    k=v&0xFF; m=[]
    if v&0x4000: m.append('Ctrl')
    if v&0x2000: m.append('Shift')
    if v&0x8000: m.append('Alt')
    m.append(VK.get(k,f'VK{k}'))
    return '+'.join(m)

INPUT={'TEdit','TComboBox','TCheckBox','TRadioButton','TMemo','TListBox','TTreeView','TTrackBar','TUpDown','TMaskEdit',
       'TStringGrid','TDrawGrid','TListView','TCheckListBox','TValueListEditor','TRadioGroup','TColorPicker','TScrollBox','TImage','TShape','TStaticText','TProgressBar'}
BTN={'TButton','TBitBtn','TSpeedButton','TToolButton'}
CONT={'TPageControl','TTabSheet','TGroupBox','TPanel','TScrollBox','TToolBar','TControlBar','TAuroraPanel','TStatusBar'}
LAB={'TLabel','TStaticText'}

def P(c,k,d=None): return c['props'].get(k,d)
def s(v):
    if isinstance(v,dict):
        if 'ident' in v: return v['ident']
        if 'set' in v: return '['+','.join(v['set'])+']'
        if 'list' in v: return '('+' | '.join(s(x) for x in v['list'])+')'
        if 'binary' in v: return f'<bin{v["binary"]}>'
        if 'coll' in v: return '<coll %d>'%len(v['coll'])
    return repr(v) if isinstance(v,str) else str(v)

def events(c):
    return ' '.join(f'{k}={s(v)}' for k,v in c['props'].items() if k.startswith('On'))

def geo(c):
    return (P(c,'Left',0),P(c,'Top',0),P(c,'Width',0),P(c,'Height',0))

def pair_labels(parent, allnames):
    labs=[k for k in parent['children'] if k['cls'] in ('TLabel',)]
    res={}
    ctrls=[k for k in parent['children'] if k['cls'] not in LAB and k['cls'] not in ('TBevel',)]
    # explicit
    for l in labs:
        fc=P(l,'FocusControl')
        if fc: res.setdefault(s(fc),[]).append(l)
    used=set(id(l) for v in res.values() for l in v)
    for c in ctrls:
        if c['cls'] in ('TCheckBox','TRadioButton','TButton','TBitBtn','TSpeedButton','TGroupBox','TTabSheet','TPageControl','TPanel','TRadioGroup'): continue
        if c['name'] in res: continue
        x,y,w,h=geo(c)
        best=None;bs=1e9
        for l in labs:
            if P(l,'FocusControl'): continue
            lx,ly,lw,lh=geo(l)
            # same row, left
            cy=y+h/2; ly2=ly+lh/2
            if abs(cy-ly2)<=max(10,h/2) and lx+lw<=x+8 and x-(lx+lw)<220:
                sc_=x-(lx+lw)+abs(cy-ly2)*2
            elif ly+lh<=y+4 and y-(ly+lh)<=22 and abs(lx-x)<=40:
                sc_=(y-(ly+lh))*3+abs(lx-x)+30
            else: continue
            if sc_<bs: bs=sc_; best=l
        if best is not None: res.setdefault(c['name'],[]).append(best)
    return res

def cap(c):
    v=P(c,'Caption')
    if v is None: return None
    return v

class Ctx:
    def __init__(s,actions): s.actions=actions

def fmt_items(c):
    it=P(c,'Items.Strings') or P(c,'Lines.Strings')
    if isinstance(it,dict) and 'list' in it:
        xs=[x for x in it['list']]
        t=' | '.join(str(x) for x in xs)
        if len(t)>300: t=t[:300]+'…(%d items)'%len(xs)
        return t
    return None

def extra(c):
    out=[]
    for k in ('MaxLength','ReadOnly','Min','Max','Associate','Position','Increment','Style','Enabled','Visible','ColCount','RowCount','FixedRows','FixedCols','Checked','State','AllowGrayed','MultiSelect','Sorted','EditMask','Frequency','ShowCheckboxes','Checkboxes','ViewStyle','WantReturns','ScrollBars','DragMode','Default','Cancel','ModalResult','Hint','Kind','GroupIndex','Down','PopupMenu','Action','DefaultRowHeight','Options','RowSelect','DropDownCount','Text','Filter','DefaultExt','Title','FileName'):
        if k in c['props']:
            v=c['props'][k]
            if k=='Style' and c['cls'] not in ('TComboBox','TToolButton','TListBox'): continue
            if k=='Text' and not v: continue
            if k=='Options' and c['cls'] not in ('TStringGrid','TDrawGrid'): continue
            out.append(f'{k}={s(v)}')
    it=fmt_items(c)
    if it: out.append(f'Items=[{it}]')
    cols=P(c,'Columns')
    if isinstance(cols,dict) and 'coll' in cols:
        out.append('Columns=['+', '.join(repr(x.get('Caption','')) for x in cols['coll'])+']')
    pn=P(c,'Panels')
    if isinstance(pn,dict) and 'coll' in pn:
        out.append('Panels=%d'%len(pn['coll']))
    return ' '.join(out)

def walk(c, ind, lines, labmap, actions, top=False):
    cls=c['cls']; name=c['name']; pad='  '*ind
    tag=P(c,'Tag')
    tg=f' #T{tag}' if tag not in (None,) else ''
    ev=events(c); ex=extra(c)
    capt=cap(c)
    if cls in ('TActionList',):
        lines.append(f'{pad}ACTIONLIST {name}{tg}')
        for a in c['children']:
            lines.append(f'{pad}  ACTION {a["name"]} cap={s(P(a,"Caption",""))} cat={s(P(a,"Category",""))} sc={sc(P(a,"ShortCut",0))} {("sec="+s(P(a,"SecondaryShortCuts.Strings"))) if P(a,"SecondaryShortCuts.Strings") else ""} hint={s(P(a,"Hint",""))} {events(a)} {"Enabled=False" if P(a,"Enabled") is False else ""} {"Visible=False" if P(a,"Visible") is False else ""} {"Checked" if P(a,"Checked") else ""} {"GroupIndex="+str(P(a,"GroupIndex")) if P(a,"GroupIndex") else ""}{" #T"+str(P(a,"Tag")) if P(a,"Tag") is not None else ""}')
        return
    if cls in ('TMainMenu','TPopupMenu'):
        lines.append(f'{pad}{cls.upper()} {name}{tg} {ev}')
        def mi(m,d):
            a=P(m,'Action'); acap=''
            if a:
                an=s(a); act=actions.get(an)
                acap=f' ->{an}' + (f' [{s(P(act,"Caption",""))}{" "+sc(P(act,"ShortCut",0)) if P(act,"ShortCut") else ""}]' if act else '')
            cp=P(m,'Caption')
            extras=[]
            if P(m,'ShortCut'): extras.append('sc='+sc(P(m,'ShortCut')))
            if P(m,'Checked'): extras.append('Checked')
            if P(m,'RadioItem'): extras.append('Radio')
            if P(m,'GroupIndex'): extras.append('GI='+str(P(m,'GroupIndex')))
            if P(m,'Visible') is False: extras.append('Visible=False')
            if P(m,'Enabled') is False: extras.append('Enabled=False')
            if P(m,'Hint'): extras.append('hint='+s(P(m,'Hint')))
            lines.append('  '*(ind+d)+f'MI {m["name"]} {s(cp) if cp is not None else "(nocap)"}{acap} {" ".join(extras)} {events(m)}{" #T"+str(P(m,"Tag")) if P(m,"Tag") is not None else ""}')
            for k in m['children']: mi(k,d+1)
        for k in c['children']: mi(k,1)
        return
    if cls in LAB and not top:
        # label printed only if unpaired
        if id(c) in labmap['used']: return
        lines.append(f'{pad}LBL {name} {s(capt) if capt is not None else "(nocap)"}{tg}{" "+ev if ev else ""}{" "+ex if ex else ""}')
        return
    if cls=='TBevel' or cls=='TImageList' or cls=='TSplitter' or cls=='TTimer': 
        if ev: lines.append(f'{pad}{cls} {name} {ev}')
        return
    lbl=labmap['map'].get(name)
    lt=''
    if lbl: lt=' LABEL=' + '/'.join(repr(P(l,'Caption','')) + (f'#T{P(l,"Tag")}' if P(l,'Tag') is not None else '') for l in lbl)
    head=f'{pad}{cls} {name}'
    if capt is not None: head+=f' cap={s(capt)}'
    elif cls in ('TCheckBox','TRadioButton','TButton','TTabSheet','TGroupBox','TBitBtn'): head+=' cap=(EMPTY)'
    head+=lt+tg
    if ex: head+=' '+ex
    if ev: head+=' '+ev
    if c['flags']&4: head+=' [INLINE FRAME]'
    if c['flags']&1: head+=' [INHERITED]'
    lines.append(head)
    if c['children']:
        pm=pair_labels(c,None)
        used=set(id(l) for v in pm.values() for l in v)
        lm={'map':pm,'used':used}
        for k in c['children']:
            walk(k,ind+1,lines,lm,actions)

def collect_actions(c,acc):
    if c['cls']=='TAction' or c['cls'].endswith('Action'):
        acc[c['name']]=c
    for k in c['children']: collect_actions(k,acc)

def report(fname,form):
    acts={}; collect_actions(form,acts)
    lines=[]
    p=form['props']
    lines.append(f'=== {fname}: {form["name"]}: {form["cls"]} caption={s(p.get("Caption"))} size={p.get("ClientWidth")}x{p.get("ClientHeight")} #T{p.get("Tag")} {events(form)}')
    pm=pair_labels(form,None)
    used=set(id(l) for v in pm.values() for l in v)
    for k in form['children']:
        walk(k,1,lines,{'map':pm,'used':used},acts)
    return lines

if __name__=='__main__':
    forms=json.load(open(sys.argv[1]))
    outd=Path(sys.argv[2]); outd.mkdir(exist_ok=True)
    tot=0
    for fn,f in forms.items():
        L=report(fn,f); tot+=len(L)
        (outd/(fn+'.txt')).write_text('\n'.join(L)+'\n')
    print(tot)
