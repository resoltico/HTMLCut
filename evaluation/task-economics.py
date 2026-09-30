#!/usr/bin/env python3
"""Correctness-qualified offline task economics; tokenizer counts are named proxies."""
import argparse
import hashlib
import importlib.metadata
import json
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import time

from bs4 import BeautifulSoup, NavigableString, Comment, Doctype

ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / 'evaluation/corpus'
ENCODING = None

def compact(value):
    return json.dumps(value, ensure_ascii=False, separators=(',', ':'))

def records(soup):
    return [dict(title=node.select_one('a')['title'], price=node.select_one('.price').get_text(),
                 stock=node.select_one('.stock').get_text(), rating=int(node.select_one('.rating').get_text()),
                 url=node.select_one('a')['href']) for node in soup.select('article.book')]

def text_nodes(node):
    return ''.join(str(value) for value in node.descendants if isinstance(value, NavigableString) and not isinstance(value, (Comment, Doctype)))

def reference(task, text):
    return prepared_reference(task, BeautifulSoup(text, 'lxml'))

def prepared_reference(task, soup):
    if task == 'titles':
        return [node['title'] for node in soup.select('a.item')]
    if task == 'urls':
        return [node['href'] for node in soup.select('a.item')]
    if task == 'technical':
        return [text_nodes(soup.select_one('#policy'))]
    if task == 'guarded':
        labels = soup.select('#label')
        values = soup.select('#amount')
        if len(labels) != 1 or labels[0].get_text() != 'Repair cost' or len(values) != 1:
            raise ValueError('declared context/cardinality failed')
        return [values[0].get_text()]
    if task == 'mapping':
        return records(soup)
    raise ValueError('unknown task')

def measured(command, warmups=3, repeats=10):
    samples = []
    for index in range(warmups + repeats):
        start = time.perf_counter_ns()
        result = subprocess.run(command, capture_output=True, check=True)
        elapsed = time.perf_counter_ns() - start
        if index >= warmups:
            samples.append(elapsed)
    return {'samples_ns': samples, 'median_ns': statistics.median(samples), 'warmups': warmups, 'repeats': repeats}, result.stdout

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary')
    parser.add_argument('--output')
    parser.add_argument('--reference-task')
    parser.add_argument('--fixture')
    parser.add_argument('--htmlcut-mapping')
    parser.add_argument('--plan-file')
    args = parser.parse_args()
    if args.htmlcut_mapping:
        result = json.loads(subprocess.check_output([args.htmlcut_mapping, 'extract', '--file', args.fixture, '--plan', args.plan_file]))
        print(compact(records(BeautifulSoup(''.join(result['values']), 'lxml'))))
        return
    if args.reference_task:
        print(compact(reference(args.reference_task, Path(args.fixture).read_text())))
        return
    global ENCODING
    import tiktoken
    ENCODING = tiktoken.get_encoding('o200k_base')
    if not args.binary or not args.output:
        parser.error('--binary and --output are required for the report')
    binary = str(Path(args.binary).resolve())
    manifest = json.loads((CORPUS / 'manifest.json').read_text())
    for entry in manifest['files']:
        assert hashlib.sha256((CORPUS / entry['path']).read_bytes()).hexdigest() == entry['sha256']
    descriptions = {}
    for operation in [[], ['extract']]:
        command = [binary, 'describe', *operation]
        output = subprocess.check_output(command).decode()
        descriptions[' '.join(command[1:])] = dict(bytes=len(output.encode()), tokens=len(ENCODING.encode(output)))
    tasks = []
    temporary = Path(args.output).resolve().parent / 'task-plans'
    temporary.mkdir(parents=True, exist_ok=True)
    for task, fixture, selector, projection in [
        ('titles','books.html','a.item',{'kind':'attribute','name':'title'}),
        ('urls','books.html','a.item',{'kind':'attribute','name':'href'}),
        ('technical','technical.html','#policy',{'kind':'dom_text'}),
        ('guarded','context.html','#amount',{'kind':'dom_text'}),
        ('mapping','books.html','article.book',{'kind':'outer_html'}),
    ]:
        plan = dict(schema='htmlcut.extraction.plan',version=1,strategy=dict(kind='css',selector=selector),
                    selection=dict(kind='all',min=1) if task in ('titles','urls','mapping') else dict(kind='single'),
                    projection=projection,exclude=[],guards=[],transforms=[])
        if task == 'guarded':
            plan['guards'] = [dict(scope='document',selector='#label',min=1,max=1,read=dict(kind='dom_text'),predicate=dict(kind='exact',value='Repair cost'))]
        path = temporary / (task + '.json')
        path.write_text(compact(plan))
        source = CORPUS / fixture
        expected = reference(task, source.read_text())
        command = [binary, 'extract', '--file', str(source), '--plan', str(path)]
        timing, raw = measured(command)
        result = json.loads(raw)
        actual = result['values']
        if task == 'mapping':
            actual = records(BeautifulSoup(''.join(actual), 'lxml'))
        assert actual == expected, task
        caller_timing = None
        caller_command = None
        if task == 'mapping':
            caller_command = [sys.executable, str(Path(__file__).resolve()), '--htmlcut-mapping', binary, '--fixture', str(source), '--plan-file', str(path)]
            caller_timing, mapped_raw = measured(caller_command)
            assert json.loads(mapped_raw) == expected
        alternative = [sys.executable, str(Path(__file__).resolve()), '--reference-task', task, '--fixture', str(source)]
        other_timing, other_raw = measured(alternative)
        assert json.loads(other_raw) == expected
        soup = BeautifulSoup(source.read_text(), 'lxml')
        start = time.perf_counter_ns()
        for _ in range(100):
            # Prepared parser reuse: selectors/mapping operate in an existing process.
            if task == 'mapping':
                value = records(soup)
            elif task in ('titles','urls'):
                value = [node['title' if task == 'titles' else 'href'] for node in soup.select('a.item')]
            else:
                value = prepared_reference(task, soup)
            assert value == expected
        in_process = (time.perf_counter_ns() - start) / 100
        payload = compact(actual)
        tasks.append(dict(task=task, fixture=fixture, correctness='complete exact equality', values=actual,
                          commands=[command,alternative] + ([caller_command] if caller_command else []), observed_retries=0, repair_steps=['Evaluation setup corrected to use the newly packaged target binary; comparator now reads literal text nodes including payload elements and excludes tokenizer setup from parser timing.'],
                          htmlcut_plus_caller_mapping_fresh_process=caller_timing,
                          htmlcut_fresh_process=timing, parser_fresh_process=other_timing,
                          parser_in_process_ns=in_process,
                          payload_bytes=len(payload.encode()), payload_tokens=len(ENCODING.encode(payload)),
                          default_envelope_bytes=len(raw), default_envelope_tokens=len(ENCODING.encode(raw.decode())),
                          parser_output_tokens=len(ENCODING.encode(other_raw.decode())),
                          command_tokens_proxy=sum(len(ENCODING.encode(compact(c))) for c in [command,alternative])))
    report = dict(scope='offline synthetic equivalent-correct tasks; not agent billing or a blind multi-agent trial',
                  tokenizer={'name':'tiktoken','version':importlib.metadata.version('tiktoken'),'encoding':'o200k_base'},
                  versions={name:importlib.metadata.version(name) for name in ['beautifulsoup4','lxml','tiktoken']},
                  python=platform.python_version(), binary_version=subprocess.check_output([binary,'--version']).decode().strip(),
                  binary_sha256=hashlib.sha256(Path(binary).read_bytes()).hexdigest(),
                  corpus=manifest, discovery=descriptions, tasks=tasks,
                  code_tokens_proxy=len(ENCODING.encode(Path(__file__).read_text())),
                  unmeasured='Actual agent command/code generation, private reasoning and billed usage; script/command counts are proxies only.')
    Path(args.output).write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n')
    print(f'Validated {len(tasks)} tasks; report: {args.output}')

if __name__ == '__main__':
    main()
