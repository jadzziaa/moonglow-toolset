#!/usr/bin/env python3
"""Generate the Aurora Toolset UI inventory markdown from parsed DFMs + TLK tables + hand mapping."""
import datetime, json, struct, re, sys, collections
from pathlib import Path

HERE = Path(__file__).resolve().parent
W = HERE.parent
sys.path.insert(0, str(HERE))
import mapping as M

forms = json.load(open(W / 'forms.json'))
tlk = {e['id']: e.get('text', '') for e in json.load(open(W / 'tlk/dialog.json'))['entries']}
data = open(W.parent / 'exe/.data', 'rb').read()
locb = json.load(open(W / 'locbases.json'))

VK = {8: 'Backspace', 9: 'Tab', 13: 'Enter', 27: 'Esc', 32: 'Space', 33: 'PgUp', 34: 'PgDn', 35: 'End', 36: 'Home',
      37: 'Left', 38: 'Up', 39: 'Right', 40: 'Down', 45: 'Ins', 46: 'Del'}
for i in range(1, 25): VK[111 + i] = f'F{i}'
for c in range(48, 58): VK[c] = chr(c)
for c in range(65, 91): VK[c] = chr(c)


def sc(v):
    if not v: return ''
    m = []
    if v & 0x4000: m.append('Ctrl')
    if v & 0x2000: m.append('Shift')
    if v & 0x8000: m.append('Alt')
    m.append(VK.get(v & 0xFF, f'VK{v & 0xFF}'))
    return '+'.join(m)


def norm(s): return re.sub(r'[^a-z0-9]', '', (s or '').replace('&', '').lower())


def P(c, k, d=None): return c['props'].get(k, d)


def ident(v):
    if isinstance(v, dict) and 'ident' in v: return v['ident']
    return v


def items(c):
    it = P(c, 'Items.Strings')
    if isinstance(it, dict) and 'list' in it: return [str(x) for x in it['list']]
    return None


# ---------- localisation tables ----------
def base_for(fn):
    if fn in M.LOC_OVERRIDE: return M.LOC_OVERRIDE[fn]
    r = locb.get(fn)
    if not r: return None
    if r['votes'] >= 3 and r['votes'] * 2 >= r['n'] and r['votes'] > r['second']: return r['base']
    return None


def loc(fn, tag):
    b = base_for(fn)
    if b is None or not isinstance(tag, int) or tag < 1: return None
    o = b + tag * 12
    if o + 12 > len(data): return None
    r = struct.unpack_from('<3i', data, o)
    if not all(x == -1 or 0 <= x < 200000 for x in r): return None
    return tuple(tlk.get(x, '') if x > 0 else '' for x in r)


def clean(s):
    s = (s or '').replace('&&', '\0').replace('&', '').replace('\0', '&')
    return s.replace('|', '/').replace('\n', ' ').replace('\r', '').strip()


def runtime_caption(fn, c):
    """Return (caption, dfm_caption, hint)."""
    dfm = P(c, 'Caption')
    hint = P(c, 'Hint')
    l = loc(fn, P(c, 'Tag'))
    cap = dfm
    if l:
        if l[0] and l[0] != '-': cap = l[0]
        if l[1]: hint = l[1]
    if cap is not None and dfm is not None and norm(cap) == norm(dfm): cap = dfm
    return clean(cap) if cap is not None else None, clean(dfm) if dfm is not None else None, clean(hint) if hint else ''


# ---------- geometry / labels ----------
def label_pairs(fn, root):
    """Map control name -> label text using FocusControl, same-parent geometry, and sibling name/value panels."""
    parent_of = {}
    absy = {}
    absx = {}

    def walk(c, par, ox, oy):
        x = ox + (P(c, 'Left', 0) or 0)
        y = oy + (P(c, 'Top', 0) or 0)
        absx[id(c)] = x; absy[id(c)] = y
        parent_of[id(c)] = par
        for k in c['children']:
            walk(k, c, x if c is not root else 0, y if c is not root else 0)
    walk(root, None, 0, 0)
    res = {}
    labels_all = []

    def collect(c):
        if c['cls'] in ('TLabel', 'TStaticText'): labels_all.append(c)
        for k in c['children']: collect(k)
    collect(root)

    def ltext(l):
        cap, dfm, _ = runtime_caption(fn, l)
        return cap
    for l in labels_all:
        fc = ident(P(l, 'FocusControl'))
        if fc: res.setdefault(fc, ltext(l))

    def visit(c):
        kids = c['children']
        labs = [k for k in kids if k['cls'] in ('TLabel', 'TStaticText') and P(k, 'Visible') is not False
                and not re.fullmatch(r'\s*(min|max|\+|=|-|gp|[-0-9.]+)?\s*', (P(k, 'Caption') or '').lower())]
        for k in kids:
            if k['cls'] in ('TLabel', 'TStaticText', 'TBevel', 'TTabSheet', 'TPageControl', 'TGroupBox',
                            'TRadioButton', 'TBitBtn', 'TSpeedButton', 'TRadioGroup', 'TToolBar', 'TToolButton'):
                continue
            if k['cls'] == 'TCheckBox' and (P(k, 'Caption') or '').strip(): continue
            if k['cls'] == 'TButton' and (P(k, 'Caption') or '').strip() not in ('...', 'E'): continue
            if k['name'] in res: continue
            x, y, w, h = P(k, 'Left', 0) or 0, P(k, 'Top', 0) or 0, P(k, 'Width', 0) or 0, P(k, 'Height', 0) or 0
            best = None; bs = 1e9
            for l in labs:
                if ident(P(l, 'FocusControl')): continue
                lx, ly, lw, lh = P(l, 'Left', 0) or 0, P(l, 'Top', 0) or 0, P(l, 'Width', 0) or 0, P(l, 'Height', 0) or 0
                cy = y + min(h, 24) / 2; ly2 = ly + lh / 2
                if abs(cy - ly2) <= 10 and lx + lw <= x + 8 and x - (lx + lw) < 240:
                    s = x - (lx + lw) + abs(cy - ly2) * 2
                elif ly + lh <= y + 4 and y - (ly + lh) <= 22 and abs(lx - x) <= 40:
                    s = (y - (ly + lh)) * 3 + abs(lx - x) + 30
                else:
                    continue
                if s < bs: bs = s; best = l
            if best is not None:
                res[k['name']] = ltext(best)
            elif k['cls'] in ('TEdit', 'TComboBox', 'TMaskEdit', 'TCheckBox', 'TTrackBar', 'TPanel', 'TMemo', 'TListBox', 'TUpDown'):
                # sibling name/value panel: look for labels in sibling containers at same absolute Y
                par = parent_of.get(id(c))
                cands = []
                if par is not None and c['cls'] in ('TPanel', 'TScrollBox'):
                    for sib in par['children']:
                        if sib is c or sib['cls'] not in ('TPanel',): continue
                        for l in sib['children']:
                            if l['cls'] in ('TLabel', 'TStaticText'):
                                cands.append(l)
                for sub in kids:
                    if sub['cls'] == 'TPanel' and sub is not k:
                        for l in sub['children']:
                            if l['cls'] in ('TLabel', 'TStaticText'):
                                cands.append(l)
                if cands:
                    ay = absy[id(k)] + min(h, 24) / 2
                    bb = None; bd = 1e9
                    for l in cands:
                        if P(l, 'Visible') is False: continue
                        lt = absy[id(l)]; lh = P(l, 'Height', 13) or 13
                        if lh > 20:
                            d = 0 if lt - 4 <= absy[id(k)] <= lt + lh - 8 else 99
                            d += abs(absy[id(k)] - lt) / 100
                        else:
                            d = abs(lt + lh / 2 - ay)
                        if d < bd: bd = d; bb = l
                    if bb is not None and bd <= 9: res[k['name']] = ltext(bb)
            for kk in [k]:
                pass
        for k in kids: visit(k)
    visit(root)
    res.update(getattr(M, 'LABEL', {}).get(fn, {}))
    return res


# ---------- row building ----------
ROWCLS = {'TEdit': 'edit', 'TMaskEdit': 'masked edit', 'TComboBox': 'combo', 'TCheckBox': 'checkbox',
          'TRadioButton': 'radio', 'TMemo': 'memo', 'TListBox': 'listbox', 'TCheckListBox': 'check-listbox',
          'TTreeView': 'tree', 'TListView': 'listview', 'TStringGrid': 'grid', 'TDrawGrid': 'draw-grid',
          'TValueListEditor': 'key/value grid', 'TTrackBar': 'slider', 'TRadioGroup': 'radio group',
          'TButton': 'button', 'TBitBtn': 'button', 'TSpeedButton': 'speed button', 'TToolButton': 'tool button',
          'TUpDown': 'spin', 'TImage': 'image', 'TAuroraPanel': '3D view', 'TOpenGLPanel': 'GL view',
          'TProgressBar': 'progress', 'TStatusBar': 'status bar', 'TPanel': 'color swatch/panel',
          'TScrollBox': 'viewport', 'TShape': 'shape'}
CONTAINERS = {'TPageControl', 'TTabSheet', 'TGroupBox', 'TPanel', 'TScrollBox', 'TToolBar', 'TControlBar'}


def evsummary(c):
    ev = []
    for k, v in c['props'].items():
        if k in ('OnClick', 'OnChange', 'OnDblClick', 'OnClickCheck', 'OnDragDrop', 'OnEdited', 'OnSelectItem',
                 'OnSetEditText', 'OnDrawItem', 'OnColumnClick', 'OnKeyDown', 'OnMouseDown', 'OnExit'):
            ev.append(f'{k}={ident(v)}')
    a = ident(P(c, 'Action'))
    if a: ev.insert(0, f'Action={a}')
    return ev


def render_form(fn, out, parent_note=None, level='####'):
    f = forms[fn]
    root = f
    rp = root['props']
    title_cap, _, _ = runtime_caption(fn, root)
    cls = root['cls']
    b = base_for(fn)
    lb = locb.get(fn) or {}
    out.append(f"{level} `{cls}` — {title_cap or '(no caption)'}")
    meta = [f"DFM `{fn}`"]
    if rp.get('ClientWidth'): meta.append(f"{rp.get('ClientWidth')}×{rp.get('ClientHeight')}")
    if rp.get('BorderStyle'): meta.append(f"border {ident(rp.get('BorderStyle'))}")
    if b is not None:
        meta.append(f"StrRef table .data+0x{b:X} ({lb.get('votes')}/{lb.get('n')} captions matched)")
    else:
        meta.append('no StrRef table resolved' if fn not in M.NOLOC_NOTE else M.NOLOC_NOTE[fn])
    fe = [f"{k}={ident(v)}" for k, v in rp.items() if k.startswith('On')]
    if fe: meta.append('form events: ' + ', '.join(fe))
    if title_cap and rp.get('Caption') and norm(title_cap) != norm(rp.get('Caption')):
        meta.append(f"DFM caption '{clean(rp.get('Caption'))}'")
    out.append('*' + ' · '.join(meta) + '*')
    out.append('')
    if fn in M.PURPOSE: out.append('**Purpose:** ' + M.PURPOSE[fn]); out.append('')
    if parent_note or fn in M.PARENT:
        out.append('**Inherits:** ' + (parent_note or M.PARENT[fn])); out.append('')
    labels = label_pairs(fn, root)
    fmap = M.MAP.get(fn, {})
    # updown associations
    assoc = {}

    def collect_ud(c):
        if c['cls'] == 'TUpDown':
            a = ident(P(c, 'Associate'))
            if a: assoc[a] = c
        for k in c['children']: collect_ud(k)
    collect_ud(root)
    ud_by_name = {}

    def collect_all(c):
        ud_by_name[c['name']] = c
        for k in c['children']: collect_all(k)
    collect_all(root)
    # tabs listing
    tabs = []

    def find_tabs(c, depth):
        if c['cls'] == 'TPageControl':
            names = []
            for t in c['children']:
                if t['cls'] == 'TTabSheet':
                    cap, dfm, _ = runtime_caption(fn, t)
                    cap = getattr(M, 'TABNAME', {}).get(fn, {}).get(t['name'], cap)
                    vis = '' if P(t, 'TabVisible', True) is not False else ' (hidden tab)'
                    names.append((cap or dfm or t['name']) + vis)
            if names: tabs.append((c['name'], names))
        for k in c['children']:
            if (k['flags'] & 4) or (k['cls'].startswith('Tfra') and k is not root): continue
            find_tabs(k, depth + 1)
    find_tabs(root, 0)
    if tabs:
        for pc, names in tabs:
            out.append(f"**Tabs** (`{pc}`): " + ' · '.join(names))
        out.append('')
    rows = []
    menus = []
    actionlists = []
    fold = {}   # button name -> combo name
    folded = collections.defaultdict(list)
    for nm, cc in ud_by_name.items():
        if cc['cls'] not in ('TButton', 'TBitBtn', 'TSpeedButton'): continue
        m = re.match(r'^(?:sb|bb|b)(Browse|Edit)(.+)$', nm)
        if not m: continue
        x = m.group(2)
        for cand in ('cb' + x, 'cbOn' + x, 'cb' + x.replace('Script', '')):
            if cand in ud_by_name and ud_by_name[cand]['cls'] == 'TComboBox':
                fold[nm] = cand
                folded[cand].append(('[…]' if m.group(1) == 'Browse' else '[Edit]'))
                break

    def path_of(stack):
        parts = []
        for c in stack:
            if c['cls'] in ('TTabSheet', 'TGroupBox', 'TRadioGroup'):
                cap, dfm, _ = runtime_caption(fn, c)
                cap = getattr(M, 'TABNAME', {}).get(fn, {}).get(c['name'], cap)
                if cap and not cap.startswith('ts'): parts.append(cap.strip())
                else: parts.append(c['name'])
            elif c['cls'] == 'TPanel' and P(c, 'Caption') and not str(P(c, 'Caption')).startswith('p') and P(c, 'Caption') not in ('Panel1',):
                cap, dfm, _ = runtime_caption(fn, c)
                if cap: parts.append(cap)
            elif c['flags'] & 4 or (c['cls'].startswith('Tfra') and c is not root):
                parts.append(f"[{c['cls']}]")
        return ' › '.join(parts)

    def walk(c, stack):
        cl = c['cls']
        if cl in ('TActionList',):
            actionlists.append(c); return
        if cl in ('TMainMenu', 'TPopupMenu'):
            menus.append(c); return
        if cl in ('TOpenDialog', 'TSaveDialog', 'TColorDialog', 'TFontDialog', 'TPrintDialog'):
            flt = P(c, 'Filter')
            rows.append((path_of(stack), c['name'], 'file dialog' if 'File' in cl or 'Open' in cl or 'Save' in cl else cl[1:],
                         clean(P(c, 'Title') or ''), fmap.get(c['name'], ''),
                         (f"filter `{clean(flt)}`" if flt else '') + (f" ext .{P(c,'DefaultExt').lstrip('.')}" if P(c, 'DefaultExt') else '')))
            return
        is_frame = (c['flags'] & 4) or (cl.startswith('Tfra') or cl.startswith('Tfrm')) and c is not root
        if is_frame and c is not root:
            rows.append((path_of(stack), c['name'], 'embedded frame', f"`{cl}`", fmap.get(c['name'], ''),
                         'see frame section' + ('; overrides: ' + ', '.join(sorted(k['name'] for k in c['children'] if evsummary(k) or P(k, 'Visible') is not None)) if any(evsummary(k) for k in c['children']) else '')))
            return
        show = False
        typ = ROWCLS.get(cl)
        if cl in ('TLabel', 'TStaticText', 'TBevel', 'TSplitter', 'TImageList', 'TTimer', 'TTabSheet', 'TPageControl', 'TGroupBox', 'TToolBar', 'TControlBar'):
            show = False
        elif cl == 'TPanel':
            show = bool(P(c, 'OnClick')) or c['name'] in fmap
        elif cl == 'TScrollBox':
            show = any(k.startswith('OnMouse') for k in c['props']) or c['name'] in fmap
        elif cl == 'TImage':
            show = any(k.startswith('On') for k in c['props']) or c['name'] in fmap
        elif cl == 'TShape':
            show = False
        elif cl == 'TToolButton':
            show = ident(P(c, 'Style')) != 'tbsSeparator'
        elif cl == 'TUpDown':
            a = ident(P(c, 'Associate'))
            show = not a and ('e' + c['name'][2:]) not in ud_by_name
        elif typ:
            show = True
        if c is root: show = False
        if c['name'] in fold: show = False
        if show:
            cap, dfm, hint = runtime_caption(fn, c)
            if cl == 'TToolButton' and ident(P(c, 'Action')):
                an = ident(P(c, 'Action'))
                act = ud_by_name.get(an)
                if act is not None:
                    acap, _, ahint = runtime_caption(fn, act)
                    if not cap or cap == c['name']: cap = acap
                    hint = hint or ahint
            lab = labels.get(c['name'])
            if cl in ('TButton', 'TBitBtn', 'TSpeedButton', 'TCheckBox', 'TRadioButton', 'TToolButton', 'TRadioGroup') or (cap and cl not in ('TEdit', 'TComboBox', 'TMemo', 'TMaskEdit', 'TPanel')):
                lab_text = cap if cap else (lab or '')
                if cl == 'TToolButton' and cap and cap == c['name']: lab_text = ''
                if dfm and cap and norm(dfm) != norm(cap) and dfm != c['name']: lab_text += f" (DFM: '{dfm}')"
                if lab and cl in ('TCheckBox',) and not cap: lab_text = lab
                if not lab_text and lab: lab_text = lab
            else:
                lab_text = lab or ''
            if cl == 'TButton' and cap in ('...', 'E') and lab: lab_text = f'{cap} ({lab})'
            if not lab_text and cl not in ('TButton', 'TBitBtn', 'TSpeedButton', 'TToolButton', 'TCheckBox', 'TRadioButton', 'TImage'):
                for a in reversed(stack):
                    if a['cls'] in ('TGroupBox', 'TTabSheet', 'TPanel', 'TRadioGroup'):
                        acap, _, _ = runtime_caption(fn, a)
                        if acap and not re.match(r'^(ts|p|Panel|gb)[A-Z0-9]', acap) and acap != a['name']:
                            lab_text = f'⟨{acap}⟩'; break
            if not lab_text and cl in ('TButton', 'TBitBtn', 'TSpeedButton', 'TCheckBox') and not cap:
                lab_text = '(no caption; ' + ('glyph' if cl in ('TBitBtn', 'TSpeedButton') else 'runtime') + ')'
            notes = []
            if P(c, 'Visible') is False: notes.append('**hidden**')
            elif any(P(a, 'Visible') is False and a['cls'] != 'TTabSheet' for a in stack): notes.append('**hidden (container)**')
            if P(c, 'Enabled') is False: notes.append('disabled')
            if P(c, 'ReadOnly') is True and cl not in ('TTreeView', 'TListView'): notes.append('read-only')
            if P(c, 'MaxLength'): notes.append(f"max {P(c,'MaxLength')}")
            ud = assoc.get(c['name']) or (ud_by_name.get('ud' + c['name'][1:]) if c['name'].startswith('e') else None)
            if ud is not None and ud['cls'] == 'TUpDown':
                mn, mx = P(ud, 'Min', 0), P(ud, 'Max', 100)
                inc = P(ud, 'Increment')
                notes.append(f"spin {mn}…{mx}" + (f" step {inc}" if inc else ''))
            if cl == 'TTrackBar': notes.append(f"range {P(c,'Min',0)}…{P(c,'Max',10)}")
            if cl == 'TComboBox':
                st = ident(P(c, 'Style'))
                notes.append('list' if st in ('csDropDownList', 'csOwnerDrawFixed') else 'editable')
            if cl in ('TCheckBox', 'TRadioButton') and P(c, 'Checked'): notes.append('default on')
            if cl == 'TToolButton' and ident(P(c, 'Style')) == 'tbsCheck': notes.append('toggle' + (' (down)' if P(c, 'Down') else ''))
            if P(c, 'MultiSelect'): notes.append('multi-select')
            if P(c, 'DragMode') and ident(P(c, 'DragMode')) == 'dmAutomatic': notes.append('drag source')
            its = items(c)
            if its and cl in ('TComboBox', 'TRadioGroup', 'TListBox') and not all(re.fullmatch(r'\d\d', x) for x in its):
                notes.append('items: ' + ' / '.join(clean(x) for x in its[:8]) + (' …' if len(its) > 8 else ''))
            cols = P(c, 'Columns')
            if isinstance(cols, dict) and cols.get('coll'):
                notes.append('cols: ' + ' / '.join(clean(x.get('Caption', '')) for x in cols['coll']))
            if cl in ('TStringGrid', 'TDrawGrid'):
                o = P(c, 'Options')
                if isinstance(o, dict) and 'goEditing' in o.get('set', []): notes.append('editable cells')
            if cl == 'TMemo' and cap and P(c, 'ReadOnly') and len(cap) > 20: notes.append(f"text: “{cap[:160]}”")
            if hint: notes.append(f"tip: “{hint[:70]}”")
            ev = evsummary(c)
            if ev: notes.append('`' + ' '.join(ev) + '`')
            if folded.get(c['name']): notes.insert(0, 'buttons ' + ' '.join(sorted(set(folded[c['name']]), reverse=True)))
            extra = M.NOTE.get(fn, {}).get(c['name'])
            if extra: notes.insert(0, extra)
            if lab_text and len(lab_text) > 90: lab_text = lab_text[:87] + '…'
            mp = fmap.get(c['name'], '')
            if not mp and cl not in ('TButton', 'TBitBtn', 'TSpeedButton', 'TToolButton'):
                mp = getattr(M, 'DEFAULT_MAP', {}).get(fn, '')
            rows.append((path_of(stack), c['name'], typ or cl, lab_text, mp, '; '.join(notes)))
        for k in c['children']:
            walk(k, stack + [c])
    walk(root, [])
    if rows:
        out.append('| Where | Control | Type | Label / caption (runtime) | Maps to | Notes |')
        out.append('|---|---|---|---|---|---|')
        for r in rows:
            out.append('| ' + ' | '.join(str(x).replace('|', '/') for x in (r[0], f"`{r[1]}`", r[2], r[3], r[4], r[5])) + ' |')
        out.append('')
    for al in actionlists:
        out.append(f"**Action list `{al['name']}`**")
        out.append('')
        out.append('| Action | Caption (runtime) | Shortcut | Handler | Default | Tip |')
        out.append('|---|---|---|---|---|---|')
        for a in al['children']:
            cap, dfm, hint = runtime_caption(fn, a)
            st = []
            if P(a, 'Enabled') is False: st.append('disabled')
            if P(a, 'Visible') is False: st.append('hidden')
            if P(a, 'Checked'): st.append('checked')
            cc = cap or ''
            if dfm and cap and norm(dfm) != norm(cap) and not dfm.startswith('act'): cc += f" (DFM '{dfm}')"
            sec = P(a, 'SecondaryShortCuts.Strings')
            s2 = sc(P(a, 'ShortCut', 0))
            if sec: s2 += ' / ' + ', '.join(sec['list'])
            out.append(f"| `{a['name']}` | {cc} | {s2} | {('`' + ident(P(a,'OnExecute')) + '`') if P(a,'OnExecute') else ''} | {', '.join(st)} | {hint[:80]} |")
        out.append('')
    for mn in menus:
        out.append(f"**{'Main menu' if mn['cls']=='TMainMenu' else 'Popup menu'} `{mn['name']}`**" + (f" (OnPopup={ident(P(mn,'OnPopup'))})" if P(mn, 'OnPopup') else ''))
        out.append('')
        out.append('| Item | Caption (runtime) | Shortcut | Action / handler | State |')
        out.append('|---|---|---|---|---|')

        def mi(m, d):
            cap, dfm, hint = runtime_caption(fn, m)
            a = ident(P(m, 'Action'))
            s = sc(P(m, 'ShortCut', 0))
            if a:
                act = ud_by_name.get(a)
                if act is not None:
                    acap, adfm, ahint = runtime_caption(fn, act)
                    if not cap or cap == '(nocap)': cap = acap
                    if not s: s = sc(P(act, 'ShortCut', 0))
            st = []
            if P(m, 'Visible') is False: st.append('hidden')
            if P(m, 'Enabled') is False: st.append('disabled')
            if P(m, 'Checked'): st.append('checked')
            if P(m, 'RadioItem'): st.append('radio')
            h = ident(P(m, 'OnClick'))
            if cap == '-':
                out.append(f"| {'&nbsp;&nbsp;' * d}— | ——— | | | |"); return
            out.append(f"| {'&nbsp;&nbsp;' * d}`{m['name']}` | {cap or ''} | {s} | {('`' + a + '`') if a else ''}{(' `' + h + '`') if h else ''} | {', '.join(st)} |")
            for k in m['children']: mi(k, d + 1)
        for k in mn['children']: mi(k, 0)
        out.append('')
    if fn in M.NOTES:
        out.append('**Notes / behaviors:**')
        for n in M.NOTES[fn]: out.append(f"- {n}")
        out.append('')


# docs/ is an Open Knowledge Format bundle: the inventory is a concept in it,
# so it opens with frontmatter ({AT}: when this run wrote it).
FRONTMATTER = """---
type: Reference
title: Aurora Toolset (nwtoolset.exe, NWN:EE) — UI Feature Inventory
description: Every form and user-facing control of the Aurora Toolset (nwtoolset.exe, NWN:EE), generated from its 105 VCL forms - runtime captions, the GFF field, 2DA or behaviour each control maps to, event handlers and keyboard shortcuts. The reimplementation checklist for Moonglow.
tags: [aurora, parity, ui, inventory, generated]
generated: { by: process:aurora-uiinv-gen, at: {AT} }
sources:
  - id: nwtoolset
    resource: nwtoolset.exe of NWN:EE 89.8193.37 (its embedded DFM forms and per-form localisation tables), dialog.tlk, base-game 2DAs and sample GFFs; decoded by tools/aurora/uiinv
    title: The Aurora toolset's executable and the game's data
---
"""


def main():
    out = FRONTMATTER.replace('{AT}', datetime.datetime.now(datetime.timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')).split('\n')
    nres = sum(1 for fn in forms if base_for(fn) is not None)
    out.extend(M.HEADER.strip('\n').replace('{LOCSTATS}', f'The StrRef table was resolved for {nres} of {len(forms)} forms').split('\n'))
    out.append('')
    done = set()
    out.append('## Form index'); out.append('')
    out.append('| § | Class | Runtime title | DFM | Size | Controls (approx.) |'); out.append('|---|---|---|---|---|---|')
    for i, sec in enumerate(M.SECTIONS, 1):
        for fn in sec['forms']:
            f = forms[fn]; cap, _, _ = runtime_caption(fn, f)
            n = [0]
            def cnt(c):
                if c['cls'] in ROWCLS and c['cls'] not in ('TPanel', 'TScrollBox', 'TImage', 'TShape'): n[0] += 1
                for k in c['children']: cnt(k)
            cnt(f)
            sz = f"{f['props'].get('ClientWidth','')}×{f['props'].get('ClientHeight','')}" if f['props'].get('ClientWidth') else ''
            out.append(f"| {i} | `{f['cls']}` | {cap or ''} | {fn} | {sz} | {n[0]} |")
    out.append('')
    for sec in M.SECTIONS:
        out.append(f"## {sec['title']}")
        out.append('')
        if sec.get('intro'):
            out.extend(sec['intro'].strip('\n').split('\n')); out.append('')
        for fn in sec['forms']:
            render_form(fn, out)
            done.add(fn)
    missing = [f for f in forms if f not in done]
    if missing:
        out.append('## Unassigned forms'); out.append('')
        for fn in missing: render_form(fn, out)
    # shortcuts summary
    out.extend(M.TRAILER_PRE.strip('\n').split('\n')); out.append('')
    out.append('| Form | Command | Shortcut |'); out.append('|---|---|---|')
    seen = set()
    for fn, f in forms.items():
        def w(c):
            s = P(c, 'ShortCut')
            if s:
                cap, dfm, hint = runtime_caption(fn, c)
                if (not cap) or cap.startswith('act'): cap = getattr(M, 'SHORTCUT_NAMES', {}).get(c['name']) or hint or c['name']
                key = (fn, cap, s)
                if key not in seen:
                    seen.add(key); out.append(f"| {f['cls']} | {cap} (`{c['name']}`) | {sc(s)} |")
            for k in c['children']: w(k)
        w(f)
    out.append('')
    out.extend(M.TRAILER.strip('\n').split('\n'))
    Path(sys.argv[1]).write_text('\n'.join(out) + '\n')
    print('missing sections for', missing)
    print(len(out), 'lines')


if __name__ == '__main__':
    main()
