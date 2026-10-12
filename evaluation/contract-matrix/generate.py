#!/usr/bin/env python3
"""Generate deterministic, original, offline scenarios; no claims about public site data."""
import hashlib, json
from pathlib import Path
ROOT=Path(__file__).resolve().parent
FIX=ROOT/'fixtures'; PLANS=ROOT/'plans'; PLANS8=ROOT/'plans-v8'
FIX.mkdir(parents=True,exist_ok=True);PLANS.mkdir(parents=True,exist_ok=True);PLANS8.mkdir(parents=True,exist_ok=True)

def put(name, text):
    p=FIX/name;p.write_text(text,encoding='utf-8');return p

def plan(name,value):
    (PLANS/(name+'.json')).write_text(json.dumps(value,sort_keys=True,separators=(',',':'))+'\n')
    converted={**value,'version':8}
    (PLANS8/(name+'.json')).write_text(json.dumps(converted,sort_keys=True,separators=(',',':'))+'\n')

rows=[]
for i in range(1,31):
    rows.append(f'<article class="book" data-id="{i}"><a class="item" href="/books/{i:03}" title="Title {i:03}">Title {i:03}</a><span class="price">EUR {10+i/100:.2f}</span><span class="stock">In stock</span><span class="rating">{1+i%5}</span></article>')
nav='<nav>'+(' <a href="#noise">Navigation · irrelevant to requested records</a>'*230)+'</nav>'
book_html='<html><head><title>Catalogue</title></head><body>'+nav+'<main id="products">'+''.join(rows)+'</main><footer>Unrelated advertisement</footer></body></html>'
put('catalog.html',book_html)
put('catalog-missing-field.html',book_html.replace('<span class="price">EUR 10.17</span>','',1))
put('guarded.html','<html><h2 id="section">Repair Costs</h2><p>Premium: EUR 120</p><p id="repair-total">EUR 3,140.00</p></html>')
put('guarded-drift.html','<html><h2 id="section">Policy Premium</h2><p>Premium: EUR 120</p><p id="repair-total">EUR 3,140.00</p></html>')
put('ambiguous.html','<html><div id="first"><span class="amount">EUR 100</span></div><div id="second"><span class="amount">EUR 900</span></div></html>')
put('technical.html','<div id="policy">Free-threading <code>Py_mod_gil</code> and <em>PYTHON_GIL</em>. <span hidden>Hidden note</span></div>')
put('links.html','<html><a class="link" href="docs/terms.html">Terms</a></html>')
put('article.html','<html><body><nav>'+('NAV '*1000)+'</nav><article id="story"><h1>Small but valuable article</h1><p>First verified paragraph.</p><p>Second verified paragraph.</p></article><aside>'+('Sponsored '*600)+'</aside></body></html>')
put('jsonld.html','<script type="application/ld+json">{"@type":"Product","name":"Kit A","price":19.99}</script>')
put('app-unrendered.html','<html><body><div id="app"></div><script>document.getElementById("app").innerHTML="<span class=\\"quote\\">Rendered only</span>";</script></body></html>')
put('app-rendered.html','<html><body><div id="app"><span class="quote">Rendered only</span></div></body></html>')
(ROOT/'api.json').write_text(json.dumps({'items':[{'id':1,'value':'correct'}]},separators=(',',':'))+'\n')
plan('catalog',{'version':7,'select':'article.book','match':'all','min':30,'max':30,'fields':{
    'title':{'select':'a.item','read':'attr:title'},'price':{'select':'.price','read':'literal'},
    'stock':{'select':'.stock','read':'literal'},'rating':{'select':'.rating','read':'literal'},
    'url':{'select':'a.item','read':'attr:href'}}})
plan('guarded',{'version':7,'select':'#repair-total','read':'literal','expect':[{'select':'#section','read':'literal','equals':'Repair Costs'}]})
plan('ambiguous',{'version':7,'select':'.amount','read':'literal'})
plan('technical',{'version':7,'select':'#policy','read':'literal'})
plan('links',{'version':7,'select':'a.link','read':'url:href'})
plan('article',{'version':7,'select':'article#story','read':'text'})
plan('jsonld',{'version':7,'select':'script[type="application/ld+json"]','read':'literal'})
plan('rendered',{'version':7,'select':'.quote','read':'literal'})
expected_catalog=[{'title':f'Title {i:03}','price':f'EUR {10+i/100:.2f}','stock':'In stock','rating':str(1+i%5),'url':f'/books/{i:03}'} for i in range(1,31)]
expected={'catalog':expected_catalog,'guarded':['EUR 3,140.00'],'technical':['Free-threading Py_mod_gil and PYTHON_GIL. Hidden note'],'links':['https://example.invalid/catalog/docs/terms.html'],'jsonld':['{"@type":"Product","name":"Kit A","price":19.99}'],'rendered':['Rendered only']}
(ROOT/'expected.json').write_text(json.dumps(expected,ensure_ascii=False,sort_keys=True,indent=2)+'\n')
files=sorted([*FIX.glob('*.html'),*PLANS.glob('*.json'),*PLANS8.glob('*.json'),ROOT/'api.json',ROOT/'expected.json'])
file_manifest=[{'path':str(p.relative_to(ROOT)),'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size} for p in files]
(ROOT/'manifest.json').write_text(json.dumps({'nature':'original deterministic realistic scenarios, NOT real-world captures','generator':'evidence/generate.py','files':file_manifest},indent=2)+'\n')
print('generated',len(files),'tracked input files')
