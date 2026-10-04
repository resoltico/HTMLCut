#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Execute bounded native Windows stream counterfactuals; never substitute for full mutation accounting."""
import argparse,hashlib,json,platform,subprocess,time
from pathlib import Path

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
    if platform.system()!='Windows':parser.error('Matching native Windows runner required')
    root=Path.cwd();dest=args.output.resolve();dest.mkdir(parents=True,exist_ok=False)
    if subprocess.check_output(['git','status','--porcelain'],text=True):raise ValueError('Counterfactual source must begin clean')
    sha=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip();stdout=root/'crates/htmlcut-cli/src/stdio.rs';file_kind=root/'crates/htmlcut-cli/src/input/regular_file.rs';originals={p:p.read_bytes() for p in [stdout,file_kind]};rows=[]
    patches=[('terminal-query-role',stdout,'let terminal = if self.error','let terminal = if !self.error'),('console-flush-omitted',stdout,'console.flush()?;',''),('terminal-routing-inverted',stdout,'(false, true) => ConsoleBackend::Stdout','(false, true) => ConsoleBackend::Stderr'),('regular-kind-inverted',file_kind,'if regular {','if !regular {')]
    def bounded(command, seconds):
        process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            stdout, stderr = process.communicate(timeout=seconds)
        except subprocess.TimeoutExpired:
            subprocess.run(['taskkill', '/PID', str(process.pid), '/T', '/F'], capture_output=True, timeout=30)
            process.communicate(timeout=10)
            raise
        return subprocess.CompletedProcess(command, process.returncode, stdout, stderr)
    def build(label):
        command=['cargo','build','--locked','-p','htmlcut-cli','--message-format=json'];start=time.monotonic();run=bounded(command,900);(dest/(label+'-build.log')).write_bytes(run.stdout+run.stderr)
        binary=None
        if run.returncode==0:
            for line in run.stdout.splitlines():
                try:value=json.loads(line)
                except ValueError:continue
                if value.get('reason')=='compiler-artifact' and value.get('target',{}).get('name')=='htmlcut' and value.get('executable'):binary=Path(value['executable'])
        return run.returncode,binary,time.monotonic()-start
    def test(label,binary):
        evidence=dest/(label+'-stdio.json');command=['powershell.exe','-NoProfile','-ExecutionPolicy','Bypass','-File',str(root/'scripts/native-windows-stdio.ps1'),'-Binary',str(binary),'-Evidence',str(evidence)];run=bounded(command,120);(dest/(label+'-test.log')).write_bytes(run.stdout+run.stderr);proof=json.loads(evidence.read_text(encoding='utf-8')) if evidence.exists() else None
        return run.returncode,proof
    try:
        code,binary,seconds=build('baseline')
        if code or binary is None:raise ValueError('Native counterfactual build baseline failed')
        test_code,proof=test('baseline',binary)
        if test_code or not proof or not proof['passed']:raise ValueError('Native counterfactual test baseline failed')
        rows.append({'case':'baseline','build_exit':code,'test_exit':test_code,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'passed':True})
        for name,path,old,new in patches:
            source=originals[path].decode();assert source.count(old)==1,(name,'source boundary not unique');changed=source.replace(old,new,1);(dest/(name+'.patch.txt')).write_text(json.dumps({'file':str(path.relative_to(root)),'old':old,'new':new},indent=2)+'\n');path.write_text(changed)
            try:
                row={'case':name,'detected':False};code,binary,seconds=build(name);row.update(build_exit=code,build_seconds=seconds)
                if code==0 and binary is not None:
                    test_code,proof=test(name,binary);row.update(test_exit=test_code,binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),failed_rows=[v['case'] for v in proof['rows'] if not v['passed']] if proof else [])
                    row['detected']=test_code!=0 and proof is not None and any(not v['passed'] and 'harness_error' not in v for v in proof['rows'])
                rows.append(row);(dest/'results.json').write_text(json.dumps({'source_commit':sha,'runner_system':platform.platform(),'rows':rows,'passed':len(rows)==5 and all(v.get('detected',v.get('passed',False)) for v in rows)},indent=2)+'\n')
            except subprocess.TimeoutExpired as error:
                rows.append({'case':name,'detected':False,'timeout':True,'command':error.cmd})
            finally:path.write_bytes(originals[path])
    finally:
        for path,data in originals.items():path.write_bytes(data)
        (dest/'results.json').write_text(json.dumps({'source_commit':sha,'runner_system':platform.platform(),'rows':rows,'passed':len(rows)==5 and all(v.get('detected',v.get('passed',False)) for v in rows)},indent=2)+'\n')
    if len(rows)!=5 or not all(v.get('detected',v.get('passed',False)) for v in rows):raise SystemExit(1)

if __name__=='__main__':main()
