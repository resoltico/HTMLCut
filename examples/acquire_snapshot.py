#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Two-phase HTML acquisition: never extract a failed partial download."""
import argparse
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

def acquire_and_extract(binary, url, contract, output):
    with tempfile.TemporaryDirectory(prefix='snapshot-acquisition-') as directory:
        root=Path(directory); snapshot=root/'snapshot.html'; headers=root/'headers'
        fetched=subprocess.run(['curl','--fail','--location','--compressed','--silent','--show-error','--max-time','5','--dump-header',str(headers),'--max-filesize','67108864','--output',str(snapshot),'--write-out','%{json}','--url',url],capture_output=True,timeout=7)
        transfer=json.loads(fetched.stdout) if fetched.stdout else {}
        report={'acquisition_exit':fetched.returncode,'acquisition_stderr':fetched.stderr.decode(),'transfer':transfer,'extract_invoked':False}
        if fetched.returncode:
            return 1,report
        # Last response Content-Type is authoritative after redirects; BOM priority
        # and document parsing remain HTMLCut's job. Caller policy requires HTML.
        content_types=re.findall(r'^Content-Type:\s*(.*)$',headers.read_text(encoding='latin-1'),flags=re.I|re.M)
        content_type=content_types[-1].strip() if content_types else ''
        if not 200 <= int(transfer.get('response_code',transfer.get('http_code',0))) < 300:
            report['acquisition_error']='Expected a successful 2xx HTTP response'; return 1,report
        if content_type.split(';',1)[0].strip().lower()!='text/html':
            report['acquisition_error']='Expected a text/html snapshot'; return 1,report
        command=[str(binary),'run','--from-file',str(contract),str(snapshot),'--base-url',transfer['url_effective'],'-o',str(output)]
        charset=re.search(r'charset\s*=\s*[\"\x27]?([^\s;\"\x27]+)',content_type,re.I)
        if charset:command += ['--encoding',charset.group(1)]
        report['extract_invoked']=True
        extracted=subprocess.run(command,capture_output=True,timeout=15)
        report.update(command=command,exit=extracted.returncode,stdout=extracted.stdout.decode(errors='replace'),stderr=extracted.stderr.decode(errors='replace'))
        return extracted.returncode,report

def main():
    parser=argparse.ArgumentParser();parser.add_argument('binary',type=Path);parser.add_argument('url');parser.add_argument('contract',type=Path);parser.add_argument('output',type=Path);parser.add_argument('--report',type=Path)
    args=parser.parse_args()
    status,report=acquire_and_extract(Path(shutil.which(str(args.binary)) or args.binary).resolve(),args.url,args.contract.resolve(),args.output.resolve())
    if args.report:args.report.write_text(json.dumps(report,indent=2)+'\n', encoding='utf-8')
    print(json.dumps({k:v for k,v in report.items() if k not in ['transfer','command']}))
    return status

if __name__=='__main__':raise SystemExit(main())
