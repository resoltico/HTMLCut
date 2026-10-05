#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Correctness-qualified current-contract economics; named tokenizer sizes are proxies."""
import argparse,hashlib,importlib.metadata,json,platform,statistics,subprocess,sys,time
from pathlib import Path
from bs4 import BeautifulSoup,NavigableString,Comment,Doctype
ROOT=Path(__file__).resolve().parents[1];CORPUS=ROOT/'evaluation/corpus'

def compact(value):return json.dumps(value,ensure_ascii=False,separators=(',',':'),sort_keys=True)
def record_values(soup):
    return [dict(title=n.select_one('a')['title'],price=n.select_one('.price').get_text(),stock=n.select_one('.stock').get_text(),
                 rating=int(n.select_one('.rating').get_text()),url=n.select_one('a')['href']) for n in soup.select('article.book')]
def reference(task,text):
    soup=BeautifulSoup(text,'lxml')
    if task=='titles':return [n['title'] for n in soup.select('a.item')]
    if task=='urls':return [n['href'] for n in soup.select('a.item')]
    if task=='mapping':return record_values(soup)
    if task=='technical-literal':return [''.join(str(v) for v in soup.select_one('#policy').descendants if isinstance(v,NavigableString) and not isinstance(v,(Comment,Doctype)))]
    if task=='technical-text':
        # Authored expected answer for the fixed fixture, independent of extraction/parser code.
        return ["Free-threading The global interpreter lock affects Windows and macOS. "
                "--disable-gil\nPYTHON_GIL\nsys.version\nPy_mod_gil\nPyUnstable_Module_SetGIL "
                "Hidden source note remains included. Charges Amount EUR 180"]
    if task=='guarded':
        labels=soup.select('#label');values=soup.select('#amount')
        if len(labels)!=1 or labels[0].get_text()!='Repair cost' or len(values)!=1:raise ValueError('declared context/cardinality failed')
        return [values[0].get_text()]
    raise ValueError(task)
def paired(commands, expected, normalize, warmups=3, repeats=15):
    from measurements import paired as measure
    return measure(commands, expected, lambda key, data: normalize(key, json.loads(data)),
                   warmups=warmups, repeats=repeats)

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--binary');parser.add_argument('--output')
    parser.add_argument('--reference-task');parser.add_argument('--fixture');args=parser.parse_args()
    if args.reference_task:
        print(compact(reference(args.reference_task,Path(args.fixture).read_text())));return
    if not args.binary or not args.output:parser.error('--binary and --output required')
    import tiktoken
    enc=tiktoken.get_encoding('o200k_base');tokens=lambda v:len(enc.encode(v if isinstance(v,str) else compact(v)))
    binary=str(Path(args.binary).resolve());dest=Path(args.output).resolve()
    if dest.is_relative_to(ROOT):parser.error('Execution evidence must be outside the repository')
    dest.parent.mkdir(parents=True,exist_ok=True)
    manifest=json.loads((CORPUS/'manifest.json').read_text())
    for entry in manifest['files']:assert hashlib.sha256((CORPUS/entry['path']).read_bytes()).hexdigest()==entry['sha256']
    plans=dest.parent/'task-plans';plans.mkdir(exist_ok=True);tasks=[];discovery={}
    for name,command in [('index',[binary,'--help']),('extract',[binary,'extract','--help']),
                         ('inspection',[binary,'inspect','--file',str(CORPUS/'books.html'),'--select','article.book'])]:
        raw=subprocess.check_output(command,timeout=30);discovery[name]=dict(command=command,bytes=len(raw),tokens=tokens(raw.decode()))
    for task,fixture,selector,reading,fields in [
        ('titles','books.html','a.item','attr:title',None),
        ('urls','books.html','a.item','attr:href',None),
        ('technical-literal','technical.html','#policy','literal',None),
        ('technical-text','technical.html','#policy','text',None),
        ('guarded','context.html','#amount','literal',None),
        ('mapping','books.html','article.book',None,dict(
            title=dict(select='a',read='attr:title'),
            price=dict(select='.price',read='literal'),
            stock=dict(select='.stock',read='literal'),
            rating=dict(select='.rating',read='literal'),
            url=dict(select='a',read='attr:href')))]:
        plan=dict(version=6,select=selector)
        if task in ['titles','urls','mapping']:plan.update(match='all',min=1)
        if fields is not None:plan['fields']=fields
        elif reading is not None:plan['read']=reading
        if task=='guarded':plan['expect']=[dict(select='#label',read='literal',equals='Repair cost')]
        path=plans/(task+'.json');path.write_text(compact(plan));source=CORPUS/fixture;expected=reference(task,source.read_text())
        command=[binary,'extract','--file',str(source),'--plan',str(path)]
        alternative=[sys.executable,str(Path(__file__).resolve()),'--reference-task',task,'--fixture',str(source)]
        def normalize(key,value):return [dict(n,rating=int(n['rating'])) for n in value] if task=='mapping' and key=='htmlcut' else value
        commands=dict(htmlcut=command) if task=='technical-text' else dict(htmlcut=command,parser=alternative)
        timings,outputs=paired(commands,expected,normalize)
        timing=timings['htmlcut'];other_time=timings.get('parser');raw=outputs['htmlcut'];other_raw=outputs.get('parser');data=json.loads(raw)
        actual=[dict(n,rating=int(n['rating'])) for n in data] if task=='mapping' else data
        assert actual==expected,task
        # The mapper converts one primitive only; no caller HTML reparsing occurs.
        receipt_path=plans/(task+'.receipt.json');bundle_path=plans/(task+'.htmlcut.tar')
        for artifact in [receipt_path,bundle_path]:
            if artifact.exists():artifact.unlink()
        receipt_command=command+['--receipt',str(receipt_path)];receipt_run=subprocess.run(receipt_command,check=True,capture_output=True,timeout=30)
        assert receipt_run.stdout==raw
        receipt_raw=receipt_path.read_bytes();receipt=json.loads(receipt_raw)
        assert receipt['data_sha256']==hashlib.sha256(compact(data).encode()).hexdigest()
        bundle_command=command+['--bundle',str(bundle_path)];bundle_run=subprocess.run(bundle_command,check=True,capture_output=True,timeout=30)
        assert bundle_run.stdout==raw
        replay=subprocess.check_output([binary,'replay',str(bundle_path)],timeout=30);assert replay==raw
        alternative=[sys.executable,str(Path(__file__).resolve()),'--reference-task',task,'--fixture',str(source)]
        if other_raw is not None:assert json.loads(other_raw)==expected
        warm=[]; source_text=source.read_text()
        for _ in range(0 if task=='technical-text' else 100):
            start=time.perf_counter_ns();value=reference(task,source_text);warm.append(time.perf_counter_ns()-start);assert value==expected
        tasks.append(dict(task=task,fixture=fixture,correctness='complete exact equality',values=actual,htmlcut_values=data,
            commands=commands,htmlcut_fresh_process=timing,parser_fresh_process=other_time,
            parser_warm_parse_select=dict(samples_ns=warm,median_ns=statistics.median(warm)) if warm else None,
            payload_bytes=len(raw),payload_tokens=tokens(raw.decode()),parser_output_tokens=tokens(other_raw.decode()) if other_raw is not None else None,
            plan_tokens=tokens(plan),source_tokens=tokens(source.read_text()),caller_mapping='rating string to int only' if task=='mapping' else None,
            receipt=dict(command=receipt_command,bytes=len(receipt_raw),tokens=tokens(receipt_raw.decode()),not_default_stdout=True),
            bundle=dict(command=bundle_command,bytes=bundle_path.stat().st_size,replay_equal=True,not_default_stdout=True),observed_retries=0))
    report=dict(scope='fixed offline synthetic equivalent-correct tasks; not billing or blind agent reasoning',
        tokenizer=dict(name='tiktoken',version=importlib.metadata.version('tiktoken'),encoding='o200k_base'),
        versions={n:importlib.metadata.version(n) for n in ['beautifulsoup4','lxml','tiktoken']},python=platform.python_version(),
        binary_version=subprocess.check_output([binary,'--version']).decode().strip(),binary_sha256=hashlib.sha256(Path(binary).read_bytes()).hexdigest(),
        corpus=manifest,discovery=discovery,tasks=tasks,unmeasured='Actual billed reasoning/context/cache effects; in-process Rust measured separately.')
    dest.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n');print(f'Validated {len(tasks)} tasks; report: {dest}')
if __name__=='__main__':main()
