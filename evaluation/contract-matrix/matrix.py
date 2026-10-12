#!/usr/bin/env python3
"""Independent, correctness-checked agent-use comparison, NOT a universal speed test.

Run: python3 generate.py
     python3 matrix.py --output baseline.json
     python3 matrix.py --binary /absolute/path/to/htmlcut --output paired.json
Requires beautifulsoup4 and lxml; optional: Chromium through Playwright and jq/pandoc.
The HTMLCut binary is NEVER downloaded by this script. The caller selects and verifies it.
"""
import argparse,hashlib,importlib.metadata,json,platform,random,shutil,statistics,subprocess,sys,time
from pathlib import Path
from urllib.parse import urljoin
from bs4 import BeautifulSoup
from lxml import html as lh
R=Path(__file__).resolve().parent
BASE='https://example.invalid/catalog/'
EXPECTED=json.loads((R/'expected.json').read_text())
TASKS={
 'catalog':('catalog.html','catalog.json'),
 'guarded':('guarded.html','guarded.json'),
 'guarded-drift':('guarded-drift.html','guarded.json'),
 'catalog-missing-field':('catalog-missing-field.html','catalog.json'),
 'ambiguous':('ambiguous.html','ambiguous.json'),
 'technical':('technical.html','technical.json'),
 'links':('links.html','links.json'),
 'article':('article.html','article.json'),
 'jsonld':('jsonld.html','jsonld.json'),
 'unrendered':('app-unrendered.html','rendered.json'),
 'rendered':('app-rendered.html','rendered.json'),
}
FAILURES={'guarded-drift','catalog-missing-field','ambiguous','unrendered'}

def bs4(task, source):
    s=BeautifulSoup(source,'lxml')
    if task in ('catalog','catalog-missing-field'):
        out=[]
        rows=s.select('article.book');assert len(rows)==30,'root cardinality'
        for a in rows:
            for sel in ['a.item','.price','.stock','.rating']:assert len(a.select(sel))==1,'field cardinality'
            assert a.select_one('a.item').has_attr('title') and a.select_one('a.item').has_attr('href'),'required attributes'
            out.append({'title':a.select_one('a.item')['title'],'price':a.select_one('.price').get_text(),
                        'stock':a.select_one('.stock').get_text(),'rating':a.select_one('.rating').get_text(),
                        'url':a.select_one('a.item')['href']})
        return out
    if task in ('guarded','guarded-drift'):
        assert len(s.select('#section'))==1 and s.select_one('#section').get_text()=='Repair Costs','context guard'
        nodes=s.select('#repair-total');assert len(nodes)==1
        return [nodes[0].get_text()]
    if task=='ambiguous':
        a=s.select('.amount');assert len(a)==1,'cardinality';return [a[0].get_text()]
    if task=='technical':return [s.select_one('#policy').get_text(separator='')]
    if task=='links':return [urljoin(BASE,s.select_one('a.link')['href'])]
    if task=='article':return [s.select_one('article#story').get_text(separator=' ',strip=True)]
    if task=='jsonld':return [s.select_one('script[type="application/ld+json"]').string]
    if task in ('unrendered','rendered'):
        a=s.select('.quote');assert len(a)==1,'no rendered quote';return [a[0].get_text()]
    raise KeyError(task)

def lxml(task, source):
    s=lh.fromstring(source)
    q=lambda x:s.xpath(x)
    if task in ('catalog','catalog-missing-field'):
        out=[]
        rows=q('//article[contains(concat(" ",normalize-space(@class)," ")," book ")]');assert len(rows)==30,'root cardinality'
        for a in rows:
            for sel in ['.//a[@class="item"]','.//span[@class="price"]','.//span[@class="stock"]','.//span[@class="rating"]']:assert len(a.xpath(sel))==1,'field cardinality'
            txt=lambda sel:a.xpath(sel)[0].text_content()
            out.append({'title':a.xpath('.//a[@class="item"]/@title')[0],
                        'price':txt('.//span[@class="price"]'),
                        'stock':txt('.//span[@class="stock"]'),
                        'rating':txt('.//span[@class="rating"]'),
                        'url':a.xpath('.//a[@class="item"]/@href')[0]})
        return out
    if task in ('guarded','guarded-drift'):
        a=q('//*[@id="section"]'); assert len(a)==1 and a[0].text_content()=='Repair Costs','context guard'
        a=q('//*[@id="repair-total"]');assert len(a)==1
        return [a[0].text_content()]
    if task=='ambiguous':
        a=q('//*[contains(concat(" ",normalize-space(@class)," ")," amount ")]'); assert len(a)==1,'cardinality'
        return [a[0].text_content()]
    if task=='technical':return [q('//*[@id="policy"]')[0].text_content()]
    if task=='links':return [urljoin(BASE,q('//a[@class="link"]/@href')[0])]
    if task=='article':return [' '.join(' '.join(q('//article[@id="story"]')[0].itertext()).split())]
    if task=='jsonld':return [q('//script[@type="application/ld+json"]')[0].text]
    if task in ('unrendered','rendered'):
        a=q('//*[contains(concat(" ",normalize-space(@class)," ")," quote ")]');assert len(a)==1,'no rendered quote';return [a[0].text_content()]
    raise KeyError(task)

def canonical(v):return json.dumps(v,sort_keys=True,ensure_ascii=False,separators=(',',':'))+'\n'

def reference_cli():
    task=sys.argv[2];which=sys.argv[3];path=R/'fixtures'/TASKS[task][0]
    try:
        data=(bs4 if which=='bs4' else lxml)(task,path.read_text())
        print(canonical(data),end='')
    except (AssertionError,IndexError,AttributeError,KeyError,ValueError) as e:
        print(json.dumps({'error':type(e).__name__,'message':str(e)}),file=sys.stderr)
        sys.exit(3)

def run_command(cmd, expected_error,expected=None):
    start=time.perf_counter_ns()
    p=subprocess.run(cmd,capture_output=True,timeout=20,check=False)
    ns=time.perf_counter_ns()-start
    if expected_error:
        if p.returncode==0 or p.stdout or not p.stderr:raise AssertionError(f'Expected atomic failure: {cmd}: {p.returncode}: {p.stdout[:150]}')
        if Path(cmd[0]).resolve()!=Path(sys.executable).resolve() and p.returncode!=3:raise AssertionError(f'HTMLCut unexpected error class: {p.returncode} {p.stderr[:150]}')
    else:
        if p.returncode:raise AssertionError(f'{cmd}: {p.returncode} {p.stderr[:300]}')
        v=json.loads(p.stdout)
        if expected is None and 'article' in ' '.join(cmd) and (not isinstance(v,list) or len(v)!=1 or 'Small but valuable article' not in v[0] or 'Sponsored' in v[0] or 'NAV' in v[0]):raise AssertionError(f'Article extraction included boilerplate or lost main text: {cmd}')
        if expected is not None and expected[0:1]==['Small but valuable article First verified paragraph. Second verified paragraph.']:
            assert isinstance(v,list) and len(v)==1
            v=[' '.join(v[0].split())]
        if expected is not None and v!=expected:raise AssertionError(f'Wrong extraction: {cmd}: {repr(v)[:300]}')
    return ns,p.stdout,p.stderr,p.returncode

def optional_browser():
    try:
        from playwright.sync_api import sync_playwright
        with sync_playwright() as x:
            b=x.chromium.launch(headless=True,executable_path='/usr/bin/chromium' if Path('/usr/bin/chromium').exists() else None)
            page=b.new_page();page.set_content((R/'fixtures/app-unrendered.html').read_text(),wait_until='load')
            quote=page.locator('.quote').all_text_contents()
            result={'status':'passed' if quote==['Rendered only'] else 'failed','quote':quote,'rendered_input_sha256':hashlib.sha256(page.content().encode()).hexdigest()}
            b.close();return result
    except Exception as e:return {'status':'not_executed','reason':str(e)[:250]}

def optional_pandoc():
    if not shutil.which('pandoc'):return {'status':'not_installed'}
    cmd=['pandoc','-f','html','-t','markdown',str(R/'fixtures/article.html')]
    p=subprocess.run(cmd,capture_output=True,timeout=15)
    if p.returncode:return {'status':'failed','stderr':p.stderr.decode(errors='replace')[:250]}
    txt=p.stdout.decode();return {'status':'passed','markdown_bytes':len(p.stdout),'contains_boilerplate':('Sponsored' in txt or 'NAV' in txt)}

def main():
    if len(sys.argv)>1 and sys.argv[1]=='--reference':return reference_cli()
    ap=argparse.ArgumentParser();ap.add_argument('--binary',type=Path);ap.add_argument('--plan-version',type=int,choices=(7,8),default=7);ap.add_argument('--output',type=Path,default=R/'baseline.json');ap.add_argument('--repeats',type=int,default=15);args=ap.parse_args()
    if args.repeats<1:ap.error('--repeats must be positive')
    b=args.binary.resolve() if args.binary else None
    if b and not b.is_file():ap.error('binary not found')
    manifest=json.loads((R/'manifest.json').read_text())
    for f in manifest['files']:
        p=R/f['path'];assert p.stat().st_size==f['bytes'] and hashlib.sha256(p.read_bytes()).hexdigest()==f['sha256'],f['path']
    versions={'matrix_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'python':platform.python_version(),'beautifulsoup4':importlib.metadata.version('beautifulsoup4'),'lxml':importlib.metadata.version('lxml')}
    if b:
        versions['htmlcut']=subprocess.check_output([str(b),'--version'],timeout=10).decode().strip()
        versions['binary_sha256']=hashlib.sha256(b.read_bytes()).hexdigest()
    for fn in [bs4,lxml]:
        catalog=(R/'fixtures/catalog.html').read_text()
        duplicate=catalog.replace('<span class="price">EUR 10.01</span>', '<span class="price">EUR 10.01</span><span class="price">WRONG</span>')
        for source in [duplicate,catalog.replace(' title="Title 001"','',1)]:
            try:fn('catalog',source)
            except (AssertionError,IndexError,KeyError,AttributeError):pass
            else:raise AssertionError('strict reference accepted a duplicate field/missing attribute')
    results=[]
    for task,(file,plan) in TASKS.items():
        commands={'bs4':[sys.executable,str(Path(__file__).resolve()),'--reference',task,'bs4'],
                  'lxml':[sys.executable,str(Path(__file__).resolve()),'--reference',task,'lxml']}
        if b:
            plan_folder=R/('plans' if args.plan_version==7 else 'plans-v8')
            cmd=[str(b),'extract','--file',str(R/'fixtures'/file),'--plan',str(plan_folder/plan)]
            if task=='links':cmd+=['--base-url',BASE]
            commands['htmlcut']=cmd
        expected=EXPECTED.get(task) if task!='article' else ['Small but valuable article First verified paragraph. Second verified paragraph.']
        # These cases deliberately must fail; fail-closed outcome is the oracle, not data parity.
        failed=task in FAILURES
        observations={};samples={k:[] for k in commands}
        seq=list(commands)
        for k in seq:run_command(commands[k],failed,expected)
        rng=random.Random(20261012)
        for _ in range(args.repeats):
            rng.shuffle(seq)
            for k in seq:
                ns,out,err,code=run_command(commands[k],failed,expected)
                samples[k].append(ns/1e6)
                if k not in observations:
                    observations[k]={'exit':code,'stdout_bytes':len(out),'stderr_bytes':len(err),
                                     'output_sha256':hashlib.sha256(out).hexdigest() if code==0 else None}
        for k in commands:
            observations[k].update({'median_fresh_process_ms':statistics.median(samples[k]),'samples_ms':samples[k],'command':commands[k]})
        results.append({'task':task,'source':file,'source_bytes':(R/'fixtures'/file).stat().st_size,
                        'expected':'atomic semantic refusal' if failed else ('complete ordered characters after whitespace normalization' if task=='article' else 'complete JSON equality'),
                        'observations':observations})
    api=subprocess.run(['jq','-c','.items[0].value',str(R/'api.json')],capture_output=True) if shutil.which('jq') else None
    report={'scope':'original synthetic offline workloads, fixed conditions; not universal, not a blinded agent study',
            'repo_commit':'4cd5752740b6d26aa63594fa3ea001d3290db71d',
            'versions':versions,'binary_executed':bool(b),'query_version':args.plan_version,'paired_trials':args.repeats,'results':results,
            'browser':optional_browser(), 'pandoc':optional_pandoc(),
            'json_direct':{'tool':'jq','status':'passed' if api and api.returncode==0 else 'not_executed','output':api.stdout.decode().strip() if api and api.returncode==0 else None},
            'limits':'No model token accounting; no evidence of commercial demand; no trust/semantic oracle; no network capture; no current binary benchmark if binary_executed=false.'}
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(f'validated {len(results)} scenarios, HTMLCut executed={bool(b)}: {args.output}')
    for r in results:
        print(f"  {r['task']:<23} {'FAIL-closed' if r['task'] in FAILURES else 'correct':<11} "+' '.join(f"{k}:{v['median_fresh_process_ms']:.1f}ms" for k,v in r['observations'].items()))
    print('browser',report['browser'],'pandoc',report['pandoc'],'jq',report['json_direct'])
if __name__=='__main__':main()
