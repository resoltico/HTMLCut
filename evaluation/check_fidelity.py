#!/usr/bin/env python3
"""Content-fidelity regression checks. Known failures are expected on HTMLCut 14.0.0.
Run: python check_fidelity.py --htmlcut /path/to/htmlcut
Exit 0 means all checks passed; exit 1 means a fidelity check failed.
No network, third-party input, or Python dependencies are required.
"""
import argparse,json,subprocess,sys
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--htmlcut',default='htmlcut')
a=p.parse_args()
cases=[
 ('ordinary link', '<p>Before <a href="relative.html">IMPORTANT</a> after.</p>', 'p', ['IMPORTANT']),
 ('reference-class link', '<p>Before <a class="reference internal" href="relative.html">IMPORTANT</a> after.</p>', 'p', ['IMPORTANT']),
 ('ordinary nested section', '<article><p>BEGINNING</p><section id="section-one"><h2>Policy terms</h2><p>IMPORTANT POLICY CONTENT</p></section><p>ENDING</p></article>', 'article', ['BEGINNING','IMPORTANT POLICY CONTENT','ENDING']),
 ('policy-id nested section', '<article><p>BEGINNING</p><section id="policy"><h2>Policy terms</h2><p>IMPORTANT POLICY CONTENT</p></section><p>ENDING</p></article>', 'article', ['BEGINNING','IMPORTANT POLICY CONTENT','ENDING']),
 ('image alternative text', '<p>Damage: <img src="mirror.jpg" alt="Broken mirror"> end.</p>', 'p', ['Broken mirror']),
]
rows=[]
for name,html,css,required in cases:
    proc=subprocess.run([a.htmlcut,'select','--input-html',html,'--css',css,'--match','single','--base-url','https://example.test/'],capture_output=True,text=True,timeout=15)
    missing=[term for term in required if term not in proc.stdout]
    rows.append({'case':name,'exit_code':proc.returncode,'missing':missing,'stdout':proc.stdout,'stderr':proc.stderr,'passed':proc.returncode==0 and not missing})
print(json.dumps(rows,ensure_ascii=False,indent=2))
sys.exit(0 if all(row['passed'] for row in rows) else 1)
