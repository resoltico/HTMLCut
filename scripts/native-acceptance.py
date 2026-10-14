#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Native delivered-binary lifetime, representation and authorization acceptance.

Missing native prerequisites are recorded as unavailable and fail acceptance.
This script creates/removes only its own temporary directory and records exact binary identity.
"""
import argparse
import hashlib
import importlib.util
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tempfile
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--results', type=Path, required=True)
    args = parser.parse_args()
    if args.results.exists():
        parser.error('candidate result already exists')
    binary = args.binary.resolve()
    results = []
    with tempfile.TemporaryDirectory(prefix='htmlcut-native-') as directory:
        root = Path(directory)
        root.chmod(0o755)
        page = root / 'page.html'
        page.write_bytes(b'<h1>complete</h1><pre>a\nb</pre><pre>c</pre>')
        destination = root / 'result.json'

        def run(argv, **kwargs):
            return subprocess.run([str(binary), *map(str, argv)], capture_output=True, timeout=4, **kwargs)

        def check(name, action):
            started = time.monotonic()
            try:
                details = action() or {}
                result = dict(id=name, status='pass', details=details)
            except FileNotFoundError as error:
                result = dict(id=name, status='unavailable', detail=str(error))
            except Exception as error:
                result = dict(id=name, status='fail', detail=str(error))
            result['seconds'] = time.monotonic() - started
            results.append(result)

        def admission_and_framing():
            destination.write_bytes(b'old')
            result = run(['get', 'pre', page, '--all', '--raw', '-o', destination])
            assert result.returncode == 3 and not result.stdout, result
            error = json.loads(result.stderr)
            assert error['error'] == 'unrepresentable_output' and error['row'] == [1], error
            assert destination.read_bytes() == b'old'
            result = run(['get', 'pre:first-of-type', page, '--raw'])
            assert result.returncode == 0 and result.stdout == b'a\nb\n' and not result.stderr, result
            result = run(['run', '--from-file', '-', page], input=b'"h1"')
            assert result.returncode == 0 and json.loads(result.stdout) == 'complete', result
            result = run(['run', '--from-file', '-'], input=b'"h1"')
            assert result.returncode == 2 and json.loads(result.stderr)['error'] == 'invalid_arguments', result
            result = run(['get', 'h1', page, '--timeout', '0'])
            assert result.returncode == 4 and not result.stdout, result
            for command in ['schema', 'get', 'outline', 'run']:
                result = run([command, '--help'])
                assert result.returncode == 0 and b'Usage:' in result.stdout, result
                if command == 'schema':
                    assert b'--output' in result.stdout and b'--timeout' in result.stdout
                    assert b'--max-nodes' not in result.stdout and b'--raw' not in result.stdout
            result = run(['schema', '-o', destination])
            assert result.returncode == 0 and not result.stdout and json.loads(destination.read_bytes())['$defs'], result
            return {'raw_refusal': error}

        check('representation-stdin-schema-help', admission_and_framing)

        def bounded_input_and_reporting():
            # Keep producers open; the command itself must terminate, rather than an external kill.
            for argv, expected in [(['get', 'h1', '--timeout', '.05'], 4), (['run', '{'], 2)]:
                reader, writer = os.pipe()
                capacity = 0
                os.set_blocking(writer, False)
                try:
                    while True:
                        capacity += os.write(writer, b'x' * 4096)
                except BlockingIOError:
                    pass
                os.set_blocking(writer, True)
                child = subprocess.Popen([str(binary), *argv], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=writer)
                try:
                    child.wait(timeout=2)
                    assert child.returncode == expected, child.returncode
                finally:
                    if child.poll() is None:
                        child.kill()
                        child.wait()
                    if child.stdin:
                        child.stdin.close()
                    if child.stdout:
                        child.stdout.close()
                    os.close(writer)
                    os.close(reader)
            # A writable diagnostic stream gets exactly one complete JSON line.
            child = subprocess.Popen([str(binary), 'get', 'h1', '--timeout', '.05'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            try:
                child.wait(timeout=2)
                output = child.stdout.read()
                error = child.stderr.read()
                assert child.returncode == 4 and not output and error.count(b'\n') == 1
                assert json.loads(error)['limit'] == 'time'
            finally:
                if child.poll() is None:
                    child.kill()
                    child.wait()
                child.stdin.close()
                child.stdout.close()
                child.stderr.close()
            return {'blocked_stderr_capacity': capacity, 'exit_classes': [4, 2]}

        check('blocked-input-and-stderr', bounded_input_and_reporting)

        if os.name == 'posix':
            def fd_conflicts_and_fifos():
                aliases = ['/dev/stdin', '/dev/fd/0']
                alias = root / 'stdin-alias'
                alias.symlink_to('/dev/stdin')
                aliases.append(str(alias))
                if Path('/proc/self/fd/0').exists():
                    aliases.append('/proc/self/fd/0')
                for source in aliases:
                    child = subprocess.Popen([str(binary), 'run', '--from-file', '-', source], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                    try:
                        child.wait(timeout=2)
                        assert child.returncode == 2 and not child.stdout.read()
                        assert json.loads(child.stderr.read())['error'] == 'invalid_arguments'
                    finally:
                        if child.poll() is None:
                            child.kill()
                            child.wait()
                        child.stdin.close()
                        child.stdout.close()
                        child.stderr.close()
                fifo = root / 'fifo'
                os.mkfifo(fifo)
                for argv in [['get', 'h1', fifo], ['run', '--from-file', fifo, page], ['get', 'h1', page, '-o', fifo]]:
                    result = run([*argv, '--timeout', '.05'])
                    assert result.returncode == 4 and not result.stdout and json.loads(result.stderr)['limit'] == 'time', result
                return {'aliases_checked': aliases, 'unconnected_fifo_cases': 3}
            check('fd0-aliases-and-unconnected-fifos', fd_conflicts_and_fifos)

        if os.name == 'nt':
            def named_pipe_lifetime():
                outcomes=[]
                for reading in [True,False]:
                    name='htmlcut-'+os.urandom(12).hex();ready=root/'pipe-ready'
                    ready.unlink(missing_ok=True)
                    server_script=root/'pipe-server.ps1'
                    direction='Out' if reading else 'In'
                    server_script.write_text("param([string]$Name,[string]$Ready);$p=[System.IO.Pipes.NamedPipeServerStream]::new($Name,[System.IO.Pipes.PipeDirection]::"+direction+");[System.IO.File]::WriteAllText($Ready,'ready');try{$p.WaitForConnection();"+("$b=[System.Text.Encoding]::UTF8.GetBytes('<h1>prefix');$p.Write($b,0,$b.Length);$p.Flush();" if reading else "")+"Start-Sleep -Seconds 5}finally{$p.Dispose()}")
                    server=subprocess.Popen(['powershell.exe','-NoProfile','-File',str(server_script),name,str(ready)],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
                    try:
                        started=time.monotonic()
                        while not ready.exists() and time.monotonic()-started<3:time.sleep(.01)
                        assert ready.exists(),'native named-pipe server was not ready'
                        pipe='\\\\.\\pipe\\'+name
                        if reading:
                            argv=['get','h1',pipe,'--timeout','.1']
                        else:
                            large=root/'large.html';large.write_bytes(b'<p>'+b'x'*1024*1024+b'</p>')
                            argv=['get','p',large,'-o',pipe,'--timeout','.1']
                        result=run(argv)
                        assert result.returncode==4 and json.loads(result.stderr)['limit']=='time',result
                        outcomes.append(result.returncode)
                    finally:
                        server.kill();server.communicate(timeout=3)
                return {'native_named_pipe_timeouts':outcomes}
            check('native-connected-named-pipe-lifetime',named_pipe_lifetime)

        def authorization():
            destination.write_bytes(b'old-private')
            destination.chmod(0o644)
            if sys.platform == 'darwin':
                subprocess.run(['chmod', '+a', 'everyone allow read,file_inherit', str(root)], check=True, capture_output=True)
                subprocess.run(['chmod', '+a', 'everyone deny read', str(destination)], check=True, capture_output=True)
                try:
                    try:
                        destination.read_bytes()
                    except PermissionError:
                        pass
                    else:
                        raise AssertionError('native ACL negative control did not deny reading')
                    result = run(['get', 'h1', page, '-o', destination])
                    assert result.returncode == 0 and not result.stdout and not result.stderr, result
                    try:
                        destination.read_bytes()
                    except PermissionError:
                        pass
                    else:
                        raise AssertionError('published result loosened native ACL')
                    acl = subprocess.check_output(['ls', '-le', str(destination)], text=True)
                    assert 'deny read' in acl, acl
                    subprocess.run(['chmod', '+a', 'everyone deny read,directory_inherit,only_inherit', str(root)], check=True, capture_output=True)
                    control = root / 'inherited-directory-control'
                    control.mkdir()
                    try:
                        try:
                            list(control.iterdir())
                        except PermissionError:
                            pass
                        else:
                            raise AssertionError('inherited directory ACL must deny listing')
                    finally:
                        subprocess.run(['chmod', '-N', str(control)], check=True, capture_output=True)
                        control.rmdir()
                    before = set(root.iterdir())
                    result = run(['get', 'h1', page, '-o', destination])
                    assert result.returncode == 0 and not result.stdout and not result.stderr, result
                    assert set(root.iterdir()) == before, 'inherited denial stranded a staging directory'
                    return {'acl': acl, 'actual_denied_reads': 2, 'inherited_directory_denial_cleaned': True}
                finally:
                    subprocess.run(['chmod', '-N', str(destination)], check=True, capture_output=True)
                    subprocess.run(['chmod', '-N', str(root)], check=True, capture_output=True)
            if sys.platform.startswith('linux'):
                if not shutil.which('setfacl') or not shutil.which('sudo'):
                    raise FileNotFoundError('native Linux acceptance needs setfacl and sudo -n -u nobody')
                def reading():
                    return subprocess.run(['sudo', '-n', '-u', 'nobody', 'cat', str(destination)], capture_output=True, timeout=3)
                assert reading().returncode == 0, 'authorized native read control must succeed'
                subprocess.run(['setfacl', '-m', 'd:u:nobody:r-x', str(root)], check=True, capture_output=True)
                subprocess.run(['setfacl', '-m', 'u:nobody:---', str(destination)], check=True, capture_output=True)
                assert reading().returncode != 0, 'named-user deny control must reject actual reading'
                result = run(['get', 'h1', page, '-o', destination])
                assert result.returncode == 0 and not result.stderr, result
                assert reading().returncode != 0, 'published result loosened named-user ACL'
                assert destination.read_bytes() == b'"complete"\n'
                return {'acl': subprocess.check_output(['getfacl', '-cp', str(destination)], text=True), 'actual_denied_reads': 2}
            if os.name == 'nt':
                command = "param([string]$p);$a=Get-Acl -LiteralPath $p;$a.SetAccessRuleProtection($true,$true);$u=[System.Security.Principal.WindowsIdentity]::GetCurrent().Name;$r=[System.Security.AccessControl.FileSystemAccessRule]::new($u,[System.Security.AccessControl.FileSystemRights]::ReadData,[System.Security.AccessControl.AccessControlType]::Deny);$a.AddAccessRule($r);Set-Acl -LiteralPath $p -AclObject $a"
                ps=root/'authorization.ps1';ps.write_text(command)
                subprocess.run(['powershell.exe', '-NoProfile', '-File', str(ps), str(destination)], check=True, capture_output=True)
                subprocess.run(['icacls.exe',str(destination),'/setintegritylevel','L'],check=True,capture_output=True)
                before_label=subprocess.check_output(['icacls.exe',str(destination)],text=True)
                try:
                    try:
                        destination.read_bytes()
                    except PermissionError:
                        pass
                    else:
                        raise AssertionError('native protected DACL negative control did not deny reading')
                    result = run(['get', 'h1', page, '-o', destination])
                    assert result.returncode == 0 and not result.stderr, result
                    try:
                        destination.read_bytes()
                    except PermissionError:
                        pass
                    else:
                        raise AssertionError('published result loosened protected DACL')
                    after_label=subprocess.check_output(['icacls.exe',str(destination)],text=True)
                    assert 'Low Mandatory Level' in before_label and 'Low Mandatory Level' in after_label, (before_label,after_label)
                    return {'actual_denied_reads': 2,'mandatory_label':after_label}
                finally:
                    restore = "param([string]$p);$a=Get-Acl -LiteralPath $p;$u=[System.Security.Principal.WindowsIdentity]::GetCurrent().Name;foreach($r in @($a.Access)){if($r.IdentityReference.Value -eq $u -and $r.AccessControlType -eq 'Deny'){$a.RemoveAccessRuleSpecific($r)}};Set-Acl -LiteralPath $p -AclObject $a"
                    ps.write_text(restore)
                    subprocess.run(['powershell.exe', '-NoProfile', '-File', str(ps), str(destination)], check=True, capture_output=True)
            raise FileNotFoundError('no native authorization adapter for this host')
        check('native-authorization-and-inheritance', authorization)

        if os.name == 'nt':
            def parent_acl_propagation():
                # Keep the script outside the directory whose child access is changed.
                script = root / 'parent-acl-propagation.ps1'
                script.write_text(r'''
param([string]$Operation,[string]$Parent,[string]$Control,[string]$Published,[int]$Protected)
$ErrorActionPreference='Stop'
$everyone=[System.Security.Principal.SecurityIdentifier]::new('S-1-1-0')
$deny=[System.Security.AccessControl.FileSystemAccessRule]::new(
    $everyone,[System.Security.AccessControl.FileSystemRights]::ReadData,
    [System.Security.AccessControl.InheritanceFlags]::ObjectInherit,
    [System.Security.AccessControl.PropagationFlags]::InheritOnly,
    [System.Security.AccessControl.AccessControlType]::Deny)
function Facts([string]$Path) {
    $acl=Get-Acl -LiteralPath $Path
    $raw=[System.Security.AccessControl.RawSecurityDescriptor]::new($acl.GetSecurityDescriptorBinaryForm(),0)
    $aces=@(foreach($ace in $raw.DiscretionaryAcl) {
        $bytes=[byte[]]::new($ace.BinaryLength)
        $ace.GetBinaryForm($bytes,0)
        [ordered]@{type=[int]$ace.AceType;flags=[int]$ace.AceFlags;
            mask=$ace.AccessMask;sid=$ace.SecurityIdentifier.Value;
            binary=[Convert]::ToBase64String($bytes)}
    })
    [ordered]@{owner=$raw.Owner.Value;group=$raw.Group.Value;
        protected=$acl.AreAccessRulesProtected;
        dacl_present=([int]$raw.ControlFlags -band 4) -ne 0;
        dacl_null=($null -eq $raw.DiscretionaryAcl);aces=$aces}
}
switch($Operation) {
    'setup' {
        foreach($path in @($Control,$Published)) {
            [System.IO.File]::WriteAllText($path,'original')
            $acl=Get-Acl -LiteralPath $path
            $acl.SetAccessRuleProtection(($Protected -eq 1),$true)
            if($Protected -eq 0) {
                # Explicit allows precede inherited denies. Use an inherited-only
                # fixture so the parent's deny is an actual ReadData refusal.
                foreach($rule in @($acl.GetAccessRules($true,$false,[System.Security.Principal.SecurityIdentifier]))) {
                    $acl.RemoveAccessRuleSpecific($rule)
                }
            }
            Set-Acl -LiteralPath $path -AclObject $acl
        }
    }
    'deny' {
        $acl=Get-Acl -LiteralPath $Parent
        $acl.AddAccessRule($deny)
        Set-Acl -LiteralPath $Parent -AclObject $acl
    }
    'remove-deny' {
        $acl=Get-Acl -LiteralPath $Parent
        $acl.RemoveAccessRuleSpecific($deny)
        Set-Acl -LiteralPath $Parent -AclObject $acl
    }
    'facts' {
        [ordered]@{control=(Facts $Control);published=(Facts $Published)} | ConvertTo-Json -Depth 8 -Compress
    }
    default {throw "Unknown parent ACL operation: $Operation"}
}
''')
                outcomes = []
                for protected in [False, True]:
                    parent = root / ('protected-parent' if protected else 'unprotected-parent')
                    parent.mkdir()
                    control = parent / 'control.json'
                    published = parent / 'published.json'

                    def powershell(operation):
                        result = subprocess.run(
                            ['powershell.exe', '-NoProfile', '-File', str(script), operation,
                             str(parent), str(control), str(published), str(int(protected))],
                            capture_output=True, text=True, timeout=4)
                        assert result.returncode == 0, (operation, result.stdout, result.stderr)
                        return json.loads(result.stdout) if operation == 'facts' else None

                    def readable(path):
                        try:
                            path.read_bytes()
                        except PermissionError:
                            return False
                        return True

                    try:
                        powershell('setup')
                        before = powershell('facts')
                        assert before['control'] == before['published'], before
                        assert before['control']['protected'] == protected, before
                        assert before['control']['dacl_present'] and not before['control']['dacl_null'], before
                        assert before['control']['aces'], 'native control must have a nonempty DACL'
                        if not protected:
                            assert all(ace['flags'] & 0x10 for ace in before['control']['aces']), before
                        assert readable(control) and readable(published), 'initial native reads must succeed'
                        result = run(['get', 'h1', page, '-o', published])
                        assert result.returncode == 0 and not result.stdout and not result.stderr, result
                        assert control.read_bytes() == b'original' and published.read_bytes() == b'"complete"\n'
                        after = powershell('facts')
                        assert after == before, {'before': before, 'after': after}
                        powershell('deny')
                        with_deny = powershell('facts')
                        assert with_deny['control'] == with_deny['published'], with_deny
                        inherited_read_deny = any(
                            ace['type'] == 1 and ace['flags'] & 0x10 and ace['mask'] & 1
                            and ace['sid'] == 'S-1-1-0'
                            for ace in with_deny['control']['aces'])
                        assert inherited_read_deny == (not protected), with_deny
                        denied = [readable(control), readable(published)]
                        assert denied == [protected, protected], {'protected': protected, 'reads': denied}
                        powershell('remove-deny')
                        restored = [readable(control), readable(published)]
                        assert restored == [True, True], restored
                        final = powershell('facts')
                        assert final == before, {'before': before, 'final': final}
                        outcomes.append({'protected': protected, 'before': before,
                                         'after_publication': after, 'reads_with_parent_deny': denied,
                                         'with_parent_deny': with_deny,
                                         'reads_after_removal': restored})
                    finally:
                        # The rule never denies parent/script access or child deletion.
                        # Always restore it before removing this test's own files.
                        try:
                            powershell('remove-deny')
                        finally:
                            shutil.rmtree(parent)
                return {'protected_and_unprotected_controls': outcomes}
            check('native-windows-parent-acl-propagation', parent_acl_propagation)

        def acquisition_boundary():
            if not shutil.which('curl'):raise FileNotFoundError('native acquisition acceptance requires curl')
            example=Path(__file__).resolve().parents[1]/'examples/acquire_snapshot.py'
            spec=importlib.util.spec_from_file_location('htmlcut_acquisition',example);recipe=importlib.util.module_from_spec(spec);spec.loader.exec_module(recipe)
            class Handler(BaseHTTPRequestHandler):
                def log_message(self,*args):pass
                def do_GET(self):
                    import gzip
                    status=200;body=b'<h1>complete</h1>';content_type='text/html; charset=utf-8'
                    if self.path=='/redirect':
                        self.send_response(302);self.send_header('Location','/moved/page');self.end_headers();return
                    if self.path=='/moved/page':body=b'<a href="child">link</a>'
                    if self.path=='/latin1':body=b'<h1>Caf\xe9</h1>';content_type='text/html; charset=windows-1252'
                    if self.path=='/javascript':body=b'<h1>Shell</h1><script>document.body.innerHTML="Rendered"</script>'
                    if self.path=='/error':status=500
                    if self.path=='/not-found':status=404
                    if self.path=='/gzip':body=gzip.compress(body)
                    self.send_response(status);self.send_header('Content-Type',content_type)
                    if self.path=='/gzip':self.send_header('Content-Encoding','gzip')
                    self.send_header('Content-Length',str(len(body)+(100 if self.path=='/truncated' else 0)));self.end_headers();self.wfile.write(body)
            server=ThreadingHTTPServer(('127.0.0.1',0),Handler);worker=threading.Thread(target=server.serve_forever,daemon=True);worker.start()
            endpoint='http://127.0.0.1:'+str(server.server_port);contract=root/'acquisition-contract.json';outcomes=[]
            try:
                for route in ['/error','/not-found','/truncated','/gzip','/redirect','/latin1','/javascript']:
                    destination.write_bytes(b'old');contract.write_text('{"select":"a","read":"url:href"}' if route=='/redirect' else '"h1"')
                    status,report=recipe.acquire_and_extract(binary,endpoint+route,contract,destination)
                    if route in ['/error','/not-found','/truncated']:
                        assert status!=0 and not report['extract_invoked'] and destination.read_bytes()==b'old',report
                    else:
                        expected={'/gzip':'complete','/redirect':endpoint+'/moved/child','/latin1':'Café','/javascript':'Shell'}[route]
                        assert status==0 and report['extract_invoked'] and json.loads(destination.read_bytes())==expected,report
                    outcomes.append({'route':route,'acquisition_exit':report['acquisition_exit'],'extract_invoked':report['extract_invoked']})
                loose=run(['run','"h1"'],input=b'<h1>complete</h1>')
                assert loose.returncode==0 and json.loads(loose.stdout)=='complete'
                return {'transfers':outcomes,'loose_partial_snapshot_control':'succeeds, so transport completion must precede extraction'}
            finally:server.shutdown();server.server_close();worker.join(timeout=2)
        check('two-phase-acquisition',acquisition_boundary)

        def clean_stages():
            assert not list(root.glob('.tmp*')), 'owned stage or directory remained'
        check('owned-stage-cleanup', clean_stages)

    report = dict(binary=str(binary), binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), host=platform.platform(), machine=platform.machine(), cases=results)
    report['counts'] = {status: sum(c['status'] == status for c in results) for status in ['pass', 'fail', 'unavailable']}
    args.results.parent.mkdir(parents=True, exist_ok=True)
    args.results.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report['counts']))
    return 0 if results and all(c['status'] == 'pass' for c in results) else 1


if __name__ == '__main__':
    raise SystemExit(main())
