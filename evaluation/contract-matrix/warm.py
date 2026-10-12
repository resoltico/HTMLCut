#!/usr/bin/env python3
"""Warm in-process parser baseline, excludes process startup, file I/O, setup and model tokens."""
import json,statistics,time,sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parent))
from matrix import bs4,lxml,TASKS,FAILURES,R,EXPECTED
r=[]
for task,(file,_) in TASKS.items():
 source=(R/'fixtures'/file).read_text()
 row={'task':task,'bytes':len(source.encode()),'results':{}}
 for name,fn in [('bs4',bs4),('lxml',lxml)]:
  times=[]
  for _ in range(50):
   start=time.perf_counter_ns()
   try:
    result=fn(task,source)
    if task in FAILURES:raise RuntimeError('expected refusal, received success')
    if task in EXPECTED:assert result==EXPECTED[task]
   except (AssertionError,IndexError,KeyError,AttributeError) as e:
    if task not in FAILURES:raise
   times.append((time.perf_counter_ns()-start)/1e6)
  row['results'][name]={'median_ms':statistics.median(times),'p95_ms':sorted(times)[47],'samples':50}
 r.append(row)
out={'scope':'warm in-process parse and extract only, input string preloaded, 50 unrandomized iterations; per-task observed for this environment only','tasks':r}
(R/'warm.json').write_text(json.dumps(out,indent=2)+'\n')
for x in r:print(x['task'], ' '.join(f"{k} {v['median_ms']:.2f}ms" for k,v in x['results'].items()))
