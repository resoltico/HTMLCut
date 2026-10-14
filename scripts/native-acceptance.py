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
                    assert b'-o - selects stdout' in result.stdout
                    assert b'FILE is a local snapshot' not in result.stdout
                else:
                    assert b"FILE is a local snapshot; '-' is standard input." in result.stdout
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
            def search_without_listing():
                if os.geteuid() == 0:
                    raise FileNotFoundError('directory permission controls need an unprivileged identity')
                parent = root / 'search-only-publication'
                parent.mkdir()
                existing = parent / 'existing.json'
                absent = parent / 'absent.json'
                control = parent / 'os-control.json'
                existing.write_bytes(b'old')
                existing.chmod(0o640)
                alias = root / 'search-only-parent-alias'
                alias.symlink_to(parent, target_is_directory=True)

                def policy(path):
                    stat = path.stat()
                    return [stat.st_uid, stat.st_gid, stat.st_mode & 0o777]

                def must_deny(operation):
                    try:
                        operation()
                    except PermissionError:
                        return
                    raise AssertionError('native permission negative control did not refuse')

                try:
                    prior = policy(existing)
                    parent.chmod(0o311)
                    must_deny(lambda: list(parent.iterdir()))
                    # Native open/create/replace controls use only write+search.
                    descriptor = os.open(existing, os.O_WRONLY)
                    os.close(descriptor)
                    stage = parent / 'os-stage.json'
                    stage.write_bytes(b'os-control')
                    descriptor = os.open(control, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o666)
                    os.close(descriptor)
                    os.replace(stage, control)
                    assert control.read_bytes() == b'os-control'
                    default_policy = policy(control)
                    for path, expected_policy in [(existing, prior), (alias / absent.name, default_policy)]:
                        result = run(['get', 'h1', page, '-o', path])
                        assert result.returncode == 0 and not result.stdout and not result.stderr, result
                        assert path.read_bytes() == b'"complete"\n'
                        assert policy(path) == expected_policy, (policy(path), expected_policy)
                    for mode, operation in [
                        (0o511, lambda: (parent / 'must-refuse.json').write_bytes(b'control')),
                        (0o600, lambda: existing.read_bytes()),
                    ]:
                        parent.chmod(mode)
                        must_deny(operation)
                        for path in [existing, parent / 'must-refuse.json']:
                            result = run(['get', 'h1', page, '-o', path])
                            assert result.returncode == 5 and not result.stdout, result
                            assert json.loads(result.stderr)['error'] == 'output_unwritable', result
                        parent.chmod(0o700)
                        assert existing.read_bytes() == b'"complete"\n'
                        assert not (parent / 'must-refuse.json').exists()
                    parent.chmod(0o700)
                    assert {p.name for p in parent.iterdir()} == {'existing.json', 'absent.json', 'os-control.json'}
                    return {'mode': '0311', 'existing_policy': prior, 'absent_policy': default_policy,
                            'native_create_open_replace': True, 'parent_alias_followed': True,
                            'missing_write_search_refusals': 4, 'staging_residue': []}
                finally:
                    parent.chmod(0o700)
                    shutil.rmtree(parent)
                    alias.unlink()
            check('publication-without-directory-listing', search_without_listing)

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
            def publication_without_parent_listing():
                # All operations run in one owned helper process. Its children
                # inherit disabled bypass privileges; Python/product contexts
                # are never changed. The script stays outside the denied parent.
                script = root / 'parent-listing-control.ps1'
                parent = root / 'delivery-parent'
                alias = root / 'delivery-alias'
                policy_backup = root / 'delivery-parent-policy.bin'
                script.write_text(r'''
param([string]$Binary,[string]$Page,[string]$Parent,[string]$Alias,[string]$PolicyBackup)
$ErrorActionPreference='Stop'
Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
public static class ParentAccessControl {
    [StructLayout(LayoutKind.Sequential)] struct Luid { public uint Low; public int High; }
    [StructLayout(LayoutKind.Sequential)] struct LuidAttributes { public Luid Luid; public uint Attributes; }
    [StructLayout(LayoutKind.Sequential)] struct Privileges { public uint Count; public LuidAttributes Entry; }
    [StructLayout(LayoutKind.Sequential)] struct FileTime { public uint Low; public uint High; }
    [StructLayout(LayoutKind.Sequential)] struct FileInfo { public uint Attributes; public FileTime Creation,Access,Write; public uint Volume,SizeHigh,SizeLow,Links,IndexHigh,IndexLow; }
    [DllImport("kernel32.dll")] static extern IntPtr GetCurrentProcess();
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool CloseHandle(IntPtr handle);
    [DllImport("kernel32.dll")] static extern IntPtr LocalFree(IntPtr pointer);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern bool MoveFileExW(string source,string target,uint flags);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern IntPtr CreateFileW(string path,uint access,uint share,IntPtr attributes,uint creation,uint flags,IntPtr template);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool GetFileInformationByHandle(IntPtr file,out FileInfo information);
    [DllImport("advapi32.dll", SetLastError=true)] static extern bool OpenProcessToken(IntPtr process,uint access,out IntPtr token);
    [DllImport("advapi32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern bool LookupPrivilegeValueW(string system,string name,out Luid luid);
    [DllImport("advapi32.dll", SetLastError=true)] static extern bool GetTokenInformation(IntPtr token,int kind,IntPtr buffer,uint length,out uint required);
    [DllImport("advapi32.dll", SetLastError=true)] static extern bool AdjustTokenPrivileges(IntPtr token,bool all,ref Privileges state,uint length,IntPtr previous,IntPtr required);
    [DllImport("advapi32.dll")] static extern uint GetSecurityInfo(IntPtr handle,int type,uint information,IntPtr owner,IntPtr group,IntPtr dacl,IntPtr sacl,out IntPtr descriptor);
    [DllImport("advapi32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern bool ConvertSecurityDescriptorToStringSecurityDescriptorW(IntPtr descriptor,uint revision,uint information,out IntPtr text,IntPtr size);
    static void Require(bool success) { if(!success) throw new Win32Exception(Marshal.GetLastWin32Error()); }
    public static int DisableExisting(string name) {
        IntPtr token=IntPtr.Zero,buffer=IntPtr.Zero;
        try {
            Require(OpenProcessToken(GetCurrentProcess(),0x28,out token));
            Luid luid; Require(LookupPrivilegeValueW(null,name,out luid));
            uint required; GetTokenInformation(token,3,IntPtr.Zero,0,out required);
            buffer=Marshal.AllocHGlobal((int)required);
            Require(GetTokenInformation(token,3,buffer,required,out required));
            uint count=(uint)Marshal.ReadInt32(buffer); int prior=-1;
            for(int index=0;index<count;index++) {
                LuidAttributes entry=(LuidAttributes)Marshal.PtrToStructure(IntPtr.Add(buffer,4+index*Marshal.SizeOf(typeof(LuidAttributes))),typeof(LuidAttributes));
                if(entry.Luid.Low==luid.Low&&entry.Luid.High==luid.High) prior=(int)entry.Attributes;
            }
            if(prior<0) return -1; // Already absent; never assign a privilege.
            Privileges state=new Privileges(); state.Count=1; state.Entry.Luid=luid; state.Entry.Attributes=0;
            Require(AdjustTokenPrivileges(token,false,ref state,0,IntPtr.Zero,IntPtr.Zero));
            int error=Marshal.GetLastWin32Error(); if(error!=0) throw new Win32Exception(error);
            Require(GetTokenInformation(token,3,buffer,required,out required));
            count=(uint)Marshal.ReadInt32(buffer);
            for(int index=0;index<count;index++) {
                LuidAttributes entry=(LuidAttributes)Marshal.PtrToStructure(IntPtr.Add(buffer,4+index*Marshal.SizeOf(typeof(LuidAttributes))),typeof(LuidAttributes));
                if(entry.Luid.Low==luid.Low&&entry.Luid.High==luid.High&&(entry.Attributes&2)!=0) throw new InvalidOperationException("Bypass privilege remained enabled");
            }
            return prior;
        } finally { if(buffer!=IntPtr.Zero) Marshal.FreeHGlobal(buffer); if(token!=IntPtr.Zero) CloseHandle(token); }
    }
    public static bool PrivilegeEnabled(IntPtr process,string name) {
        IntPtr token=IntPtr.Zero,buffer=IntPtr.Zero;
        try {
            Require(OpenProcessToken(process,8,out token));
            Luid luid;Require(LookupPrivilegeValueW(null,name,out luid));
            uint required;GetTokenInformation(token,3,IntPtr.Zero,0,out required);
            buffer=Marshal.AllocHGlobal((int)required);Require(GetTokenInformation(token,3,buffer,required,out required));
            uint count=(uint)Marshal.ReadInt32(buffer);
            for(int index=0;index<count;index++) {
                LuidAttributes entry=(LuidAttributes)Marshal.PtrToStructure(IntPtr.Add(buffer,4+index*Marshal.SizeOf(typeof(LuidAttributes))),typeof(LuidAttributes));
                if(entry.Luid.Low==luid.Low&&entry.Luid.High==luid.High) return (entry.Attributes&2)!=0;
            }
            return false;
        } finally {if(buffer!=IntPtr.Zero) Marshal.FreeHGlobal(buffer);if(token!=IntPtr.Zero) CloseHandle(token);}
    }
    public static void Replace(string source,string target) { Require(MoveFileExW(source,target,1)); }
    public static int GenericParentReadError(string path) {
        IntPtr file=CreateFileW(path,0x80000000,7,IntPtr.Zero,3,0x02000000,IntPtr.Zero);
        if(file==new IntPtr(-1)) return Marshal.GetLastWin32Error();
        CloseHandle(file);return 0;
    }
    public static string ParentIdentity(string path) {
        IntPtr file=CreateFileW(path,0,7,IntPtr.Zero,3,0x02000000,IntPtr.Zero);
        if(file==new IntPtr(-1)) throw new Win32Exception(Marshal.GetLastWin32Error());
        try {FileInfo info;Require(GetFileInformationByHandle(file,out info));return info.Volume+":"+info.IndexHigh+":"+info.IndexLow;}
        finally {CloseHandle(file);}
    }
    public static string Policy(string path) {
        IntPtr file=IntPtr.Zero,descriptor=IntPtr.Zero,text=IntPtr.Zero;
        try {
            file=CreateFileW(path,0x20000,7,IntPtr.Zero,3,0,IntPtr.Zero);
            if(file==new IntPtr(-1)) {file=IntPtr.Zero;throw new Win32Exception(Marshal.GetLastWin32Error());}
            uint status=GetSecurityInfo(file,1,0x17,IntPtr.Zero,IntPtr.Zero,IntPtr.Zero,IntPtr.Zero,out descriptor);
            if(status!=0) throw new Win32Exception((int)status);
            Require(ConvertSecurityDescriptorToStringSecurityDescriptorW(descriptor,1,0x17,out text,IntPtr.Zero));
            return Marshal.PtrToStringUni(text);
        } finally { if(text!=IntPtr.Zero) LocalFree(text); if(descriptor!=IntPtr.Zero) LocalFree(descriptor); if(file!=IntPtr.Zero) CloseHandle(file); }
    }
}
'@
$sid=[System.Security.Principal.WindowsIdentity]::GetCurrent().User
$context=[ordered]@{sid=$sid.Value;backup_before=[ParentAccessControl]::DisableExisting('SeBackupPrivilege');restore_before=[ParentAccessControl]::DisableExisting('SeRestorePrivilege');backup_enabled=$false;restore_enabled=$false}
New-Item -ItemType Directory -Path $Parent | Out-Null
$saved=Get-Acl -LiteralPath $Parent
[System.IO.File]::WriteAllBytes($PolicyBackup,$saved.GetSecurityDescriptorBinaryForm())
function ParentRule([int]$Rights) {
    $acl=[System.Security.AccessControl.DirectorySecurity]::new()
    $acl.SetSecurityDescriptorBinaryForm([System.IO.File]::ReadAllBytes($PolicyBackup))
    $rule=[System.Security.AccessControl.FileSystemAccessRule]::new($sid,[System.Security.AccessControl.FileSystemRights]$Rights,[System.Security.AccessControl.InheritanceFlags]::None,[System.Security.AccessControl.PropagationFlags]::None,[System.Security.AccessControl.AccessControlType]::Deny)
    $acl.AddAccessRule($rule)
    Set-Acl -LiteralPath $Parent -AclObject $acl
}
function InvokeDeliveredBinary([string]$Path) {
    $info=[System.Diagnostics.ProcessStartInfo]::new()
    $info.FileName=$Binary;$info.Arguments=('get h1 "{0}" -o "{1}"' -f $Page,$Path)
    $info.UseShellExecute=$false;$info.RedirectStandardOutput=$true;$info.RedirectStandardError=$true
    $child=[System.Diagnostics.Process]::Start($info)
    $failure=$null;$outcome=$null;$cleanup=@()
    try {
        $privileges=[ordered]@{backup_enabled=[ParentAccessControl]::PrivilegeEnabled($child.Handle,'SeBackupPrivilege');restore_enabled=[ParentAccessControl]::PrivilegeEnabled($child.Handle,'SeRestorePrivilege');traverse_bypass_enabled=[ParentAccessControl]::PrivilegeEnabled($child.Handle,'SeChangeNotifyPrivilege')}
        if(!$child.WaitForExit(4000)) {throw 'Delivered binary exceeded external test deadline'}
        $outcome=[ordered]@{path=$Path;exit=$child.ExitCode;stdout=$child.StandardOutput.ReadToEnd();stderr=$child.StandardError.ReadToEnd();privileges=$privileges}
    } catch {$failure=$_.Exception.Message}
    finally {
        try {if(!$child.HasExited) {$child.Kill()}} catch {$cleanup+=$_.Exception.Message}
        try {
            if(!$child.WaitForExit(4000)) {$script:unreapedChild=$true;throw 'Owned CLI child did not exit after termination'}
        } catch {$script:unreapedChild=$true;$cleanup+=$_.Exception.Message}
        try {$child.Dispose()} catch {$cleanup+=$_.Exception.Message}
    }
    if($failure) {throw "CLI helper failure: $failure; owned child cleanup: $($cleanup -join '; ')"}
    if($cleanup.Count) {throw "Owned CLI child cleanup failed: $($cleanup -join '; ')"}
    $outcome
}
$results=[ordered]@{context=$context;positive=@();negative=@()}
$script:unreapedChild=$false
$failure=$null;$cleanup=@()
try {
    $existing=Join-Path $Parent 'existing.json';$absent=Join-Path $Parent 'absent.json';$osControl=Join-Path $Parent 'os-control.json'
    [System.IO.File]::WriteAllText($existing,'old')
    ParentRule 1 # ListDirectory only; no child inheritance.
    try {[System.IO.Directory]::GetFileSystemEntries($Parent) | Out-Null;throw 'Directory enumeration unexpectedly succeeded'} catch [System.UnauthorizedAccessException] {}
    [System.IO.File]::WriteAllText($osControl,'old')
    $replacement=Join-Path $Parent 'os-stage.json'
    [System.IO.File]::WriteAllText($replacement,'os-complete')
    [ParentAccessControl]::Replace($replacement,$osControl)
    if([System.IO.File]::ReadAllText($osControl)-ne'os-complete') {throw 'Native atomic replacement failed'}
    $writer=[System.IO.File]::Open($osControl,[System.IO.FileMode]::Open,[System.IO.FileAccess]::Write);$writer.Dispose()
    $results.native_create_write_open_replace=$true;$results.enumeration_denied=$true
    $results.generic_parent_read_error=[ParentAccessControl]::GenericParentReadError($Parent)
    $results.zero_access_parent_identity=[ParentAccessControl]::ParentIdentity($Parent)
    $existingPolicy=[ParentAccessControl]::Policy($existing);$defaultPolicy=[ParentAccessControl]::Policy($osControl)
    New-Item -ItemType Junction -Path $Alias -Target $Parent | Out-Null
    $results.zero_access_alias_identity=[ParentAccessControl]::ParentIdentity($Alias)
    foreach($path in @($existing,$absent,(Join-Path $Alias 'existing.json'))) {$results.positive+=InvokeDeliveredBinary $path}
    $results.existing_policy_before=$existingPolicy;$results.existing_policy_after=[ParentAccessControl]::Policy($existing)
    $results.default_policy=$defaultPolicy
    if(Test-Path -LiteralPath $absent) {$results.absent_policy=[ParentAccessControl]::Policy($absent)}
    $results.existing_bytes=[System.IO.File]::ReadAllText($existing)
    if(Test-Path -LiteralPath $absent) {$results.absent_bytes=[System.IO.File]::ReadAllText($absent)}
    foreach($rights in @(2,32)) {
        if($rights-eq32) {$results.traverse_before=[ParentAccessControl]::DisableExisting('SeChangeNotifyPrivilege');$results.traverse_bypass_enabled=$false}
        ParentRule $rights
        $osDenied=Join-Path $Parent ('os-denied-'+$rights+'.json')
        try {[System.IO.File]::WriteAllText($osDenied,'must-refuse');throw "Native create unexpectedly allowed under deny $rights"} catch [System.UnauthorizedAccessException] {}
        foreach($name in @('existing.json',('denied-'+$rights+'.json'))) {
            $path=Join-Path $Parent $name
            $entry=InvokeDeliveredBinary $path;$entry.native_create_refused=$true;$entry.denied_parent_right=$rights;$results.negative+=$entry
        }
    }
} catch {$failure=$_.Exception.Message}
finally {
    if(!$script:unreapedChild) {
        try {Set-Acl -LiteralPath $Parent -AclObject $saved} catch {$cleanup+="Parent ACL restoration: $($_.Exception.Message)"}
        try {if(Test-Path -LiteralPath $Alias) {[System.IO.Directory]::Delete($Alias)}} catch {$cleanup+="Junction cleanup: $($_.Exception.Message)"}
    }
}
if($script:unreapedChild) {
    $results.helper_failure=$failure;$results.cleanup_failures=$cleanup
    $results | ConvertTo-Json -Depth 8 -Compress
    # Keep this exact owned parent alive so the Python supervisor can terminate
    # its process tree before independent ACL restoration; never orphan a child.
    Start-Sleep -Seconds 30
    exit 1
}
if($failure -or $cleanup.Count) {
    $results.helper_failure=$failure;$results.cleanup_failures=$cleanup
    $results | ConvertTo-Json -Depth 8 -Compress
    exit 1
}
$results.existing_bytes_after_negatives=[System.IO.File]::ReadAllText((Join-Path $Parent 'existing.json'))
$results.inventory=@([System.IO.Directory]::GetFileSystemEntries($Parent) | ForEach-Object {[System.IO.Path]::GetFileName($_)} | Sort-Object)
$results | ConvertTo-Json -Depth 8 -Compress
''')
                helper = None
                primary_failure = None
                details = None
                stdout = stderr = ''
                cleanup_failures = []
                try:
                    helper = subprocess.Popen(
                        ['powershell.exe', '-NoProfile', '-File', str(script), str(binary), str(page),
                         str(parent), str(alias), str(policy_backup)],
                        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
                    stdout, stderr = helper.communicate(timeout=25)
                    assert helper.returncode == 0, (stdout, stderr)
                    details = json.loads(stdout)
                    assert details['enumeration_denied'] and details['native_create_write_open_replace'], details
                    assert details['generic_parent_read_error'] == 5, details
                    assert details['zero_access_parent_identity'] == details['zero_access_alias_identity'], details
                    assert all(not item['privileges']['backup_enabled'] and not item['privileges']['restore_enabled']
                               for item in details['positive'] + details['negative']), details
                    assert all(not item['privileges']['traverse_bypass_enabled']
                               for item in details['negative'] if item['denied_parent_right'] == 32), details
                    assert len(details['positive']) == 3 and all(
                        item['exit'] == 0 and not item['stdout'] and not item['stderr']
                        for item in details['positive']), details
                    assert details['existing_policy_after'] == details['existing_policy_before'], details
                    assert details['absent_policy'] == details['default_policy'], details
                    assert details['existing_bytes'] == details['absent_bytes'] == '"complete"\n', details
                    assert len(details['negative']) == 4 and all(
                        item['exit'] == 5 and not item['stdout']
                        and json.loads(item['stderr'])['error'] == 'output_unwritable'
                        for item in details['negative']), details
                    assert details['existing_bytes_after_negatives'] == '"complete"\n', details
                    assert details['inventory'] == ['absent.json', 'existing.json', 'os-control.json'], details
                except Exception as error:
                    primary_failure = error
                finally:
                    # Stop the exact owned process tree before restoring ACLs;
                    # forced termination cannot run the helper's finally block.
                    if helper is not None and helper.poll() is None:
                        try:
                            stopped = subprocess.run(
                                ['taskkill.exe', '/PID', str(helper.pid), '/T', '/F'],
                                capture_output=True, text=True, timeout=5)
                            if stopped.returncode != 0:
                                cleanup_failures.append(f'owned helper tree termination: {stopped.stdout} {stopped.stderr}')
                        except Exception as error:
                            cleanup_failures.append(f'owned helper tree termination: {error}')
                        try:
                            if helper.poll() is None:
                                helper.kill()
                            stdout, stderr = helper.communicate(timeout=5)
                        except Exception as error:
                            cleanup_failures.append(f'owned helper reap: {error}')
                    elif helper is not None:
                        helper.wait()
                    if policy_backup.exists() and parent.exists():
                        restore = root / 'restore-delivery-parent.ps1'
                        restore.write_text(
                            "param([string]$Parent,[string]$Backup);$ErrorActionPreference='Stop';"
                            "$a=[System.Security.AccessControl.DirectorySecurity]::new();"
                            "$a.SetSecurityDescriptorBinaryForm([System.IO.File]::ReadAllBytes($Backup));"
                            "Set-Acl -LiteralPath $Parent -AclObject $a")
                        try:
                            restored = subprocess.run(
                                ['powershell.exe', '-NoProfile', '-File', str(restore), str(parent), str(policy_backup)],
                                capture_output=True, text=True, timeout=5)
                            if restored.returncode != 0:
                                cleanup_failures.append(f'independent parent ACL restoration: {restored.stdout} {restored.stderr}')
                        except Exception as error:
                            cleanup_failures.append(f'independent parent ACL restoration: {error}')
                    for path, cleanup in [(alias, os.rmdir), (parent, shutil.rmtree)]:
                        try:
                            if path.exists():
                                cleanup(path)
                        except Exception as error:
                            cleanup_failures.append(f'owned path cleanup {path}: {error}')
                if primary_failure is not None or cleanup_failures:
                    raise AssertionError({'primary_failure': str(primary_failure) if primary_failure else None,
                                          'stdout': stdout, 'stderr': stderr, 'cleanup_failures': cleanup_failures}) from primary_failure
                return details
            check('publication-without-directory-listing', publication_without_parent_listing)

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

            def mandatory_integrity_enforcement():
                script = root / 'mandatory-integrity.ps1'
                target = root / 'integrity.json'
                target.write_bytes(b'original')
                script.write_text(r'''
param([string]$Operation,[string]$Path)
$ErrorActionPreference='Stop'
Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
public static class IntegrityAcceptance {
    [StructLayout(LayoutKind.Sequential)] struct SidAttributes { public IntPtr Sid; public uint Attributes; }
    [StructLayout(LayoutKind.Sequential)] struct MandatoryLabel { public SidAttributes Label; }
    public sealed class ReadResult { public bool allowed; public int error; public string text; }
    [DllImport("kernel32.dll")] static extern IntPtr GetCurrentProcess();
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool CloseHandle(IntPtr handle);
    [DllImport("kernel32.dll")] static extern IntPtr LocalFree(IntPtr value);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern IntPtr CreateFileW(string path,uint access,uint sharing,IntPtr security,uint disposition,uint flags,IntPtr template);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool ReadFile(IntPtr file,byte[] buffer,uint length,out uint read,IntPtr overlapped);
    [DllImport("advapi32.dll", SetLastError=true)] static extern bool OpenProcessToken(IntPtr process,uint access,out IntPtr token);
    [DllImport("advapi32.dll", SetLastError=true)] static extern bool DuplicateTokenEx(IntPtr token,uint access,IntPtr attributes,int level,int type,out IntPtr duplicate);
    [DllImport("advapi32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern bool ConvertStringSidToSidW(string text,out IntPtr sid);
    [DllImport("advapi32.dll")] static extern uint GetLengthSid(IntPtr sid);
    [DllImport("advapi32.dll", SetLastError=true)] static extern bool CopySid(uint length,IntPtr destination,IntPtr source);
    [DllImport("advapi32.dll", SetLastError=true)] static extern bool SetTokenInformation(IntPtr token,int kind,IntPtr information,uint length);
    [DllImport("advapi32.dll", SetLastError=true)] static extern bool ImpersonateLoggedOnUser(IntPtr token);
    [DllImport("advapi32.dll", SetLastError=true)] static extern bool RevertToSelf();
    [DllImport("advapi32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern bool ConvertStringSecurityDescriptorToSecurityDescriptorW(string text,uint revision,out IntPtr descriptor,IntPtr size);
    [DllImport("advapi32.dll", SetLastError=true)] static extern bool GetSecurityDescriptorSacl(IntPtr descriptor,out bool present,out IntPtr sacl,out bool defaulted);
    [DllImport("advapi32.dll")] static extern uint SetSecurityInfo(IntPtr handle,int type,uint information,IntPtr owner,IntPtr group,IntPtr dacl,IntPtr sacl);
    [DllImport("advapi32.dll")] static extern uint GetSecurityInfo(IntPtr handle,int type,uint information,IntPtr owner,IntPtr group,IntPtr dacl,IntPtr sacl,out IntPtr descriptor);
    [DllImport("advapi32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern bool ConvertSecurityDescriptorToStringSecurityDescriptorW(IntPtr descriptor,uint revision,uint information,out IntPtr text,IntPtr size);
    static void Require(bool success) { if(!success) throw new Win32Exception(Marshal.GetLastWin32Error()); }
    static IntPtr Open(string path,uint access) {
        IntPtr handle=CreateFileW(path,access,7,IntPtr.Zero,3,0,IntPtr.Zero);
        if(handle==new IntPtr(-1)) throw new Win32Exception(Marshal.GetLastWin32Error());
        return handle;
    }
    public static ReadResult Read(string path,bool low) {
        IntPtr processToken=IntPtr.Zero,token=IntPtr.Zero,sid=IntPtr.Zero,label=IntPtr.Zero,file=IntPtr.Zero;
        bool impersonating=false;
        try {
            if(low) {
                Require(OpenProcessToken(GetCurrentProcess(),0x0002|0x0008,out processToken));
                Require(DuplicateTokenEx(processToken,0x0004|0x0008|0x0080,IntPtr.Zero,2,2,out token));
                Require(ConvertStringSidToSidW("S-1-16-4096",out sid));
                int header=Marshal.SizeOf(typeof(MandatoryLabel));
                uint sidLength=GetLengthSid(sid);
                label=Marshal.AllocHGlobal(checked(header+(int)sidLength));
                IntPtr copiedSid=IntPtr.Add(label,header);
                Require(CopySid(sidLength,copiedSid,sid));
                MandatoryLabel information=new MandatoryLabel();
                information.Label.Sid=copiedSid; information.Label.Attributes=0x20;
                Marshal.StructureToPtr(information,label,false);
                Require(SetTokenInformation(token,25,label,checked((uint)header+sidLength)));
                Require(ImpersonateLoggedOnUser(token)); impersonating=true;
            }
            // Request and exercise data-read access, without backup intent or privilege bypass.
            file=CreateFileW(path,1,7,IntPtr.Zero,3,0,IntPtr.Zero);
            if(file==new IntPtr(-1)) { file=IntPtr.Zero; return new ReadResult {allowed=false,error=Marshal.GetLastWin32Error()}; }
            byte[] bytes=new byte[256]; uint count;
            if(!ReadFile(file,bytes,(uint)bytes.Length,out count,IntPtr.Zero)) return new ReadResult {allowed=false,error=Marshal.GetLastWin32Error()};
            return new ReadResult {allowed=true,error=0,text=Encoding.UTF8.GetString(bytes,0,(int)count)};
        } finally {
            bool reverted=!impersonating||RevertToSelf();
            int revertError=reverted?0:Marshal.GetLastWin32Error();
            if(file!=IntPtr.Zero) CloseHandle(file);
            if(token!=IntPtr.Zero) CloseHandle(token);
            if(processToken!=IntPtr.Zero) CloseHandle(processToken);
            if(label!=IntPtr.Zero) Marshal.FreeHGlobal(label);
            if(sid!=IntPtr.Zero) LocalFree(sid);
            if(!reverted) throw new Win32Exception(revertError,"Failed to revert low-integrity impersonation");
        }
    }
    public static void SetMediumNoReadUp(string path) {
        IntPtr file=IntPtr.Zero,descriptor=IntPtr.Zero;
        try {
            file=Open(path,0x00020000|0x00080000);
            Require(ConvertStringSecurityDescriptorToSecurityDescriptorW("S:(ML;;NR;;;ME)",1,out descriptor,IntPtr.Zero));
            bool present,defaulted; IntPtr sacl;
            Require(GetSecurityDescriptorSacl(descriptor,out present,out sacl,out defaulted));
            if(!present||sacl==IntPtr.Zero) throw new InvalidOperationException("Mandatory label fixture is absent");
            uint status=SetSecurityInfo(file,1,0x10,IntPtr.Zero,IntPtr.Zero,IntPtr.Zero,sacl);
            if(status!=0) throw new Win32Exception((int)status);
        } finally {
            if(descriptor!=IntPtr.Zero) LocalFree(descriptor);
            if(file!=IntPtr.Zero) CloseHandle(file);
        }
    }
    public static string Policy(string path,uint information) {
        IntPtr file=IntPtr.Zero,descriptor=IntPtr.Zero,text=IntPtr.Zero;
        try {
            file=Open(path,0x00020000);
            uint status=GetSecurityInfo(file,1,information,IntPtr.Zero,IntPtr.Zero,IntPtr.Zero,IntPtr.Zero,out descriptor);
            if(status!=0) throw new Win32Exception((int)status);
            Require(ConvertSecurityDescriptorToStringSecurityDescriptorW(descriptor,1,information,out text,IntPtr.Zero));
            return Marshal.PtrToStringUni(text);
        } finally {
            if(text!=IntPtr.Zero) LocalFree(text);
            if(descriptor!=IntPtr.Zero) LocalFree(descriptor);
            if(file!=IntPtr.Zero) CloseHandle(file);
        }
    }
}
'@
$initial=$null
$initialDacl=$null
if($Operation -eq 'setup') {
    $initial=[IntegrityAcceptance]::Read($Path,$true)
    $initialDacl=[IntegrityAcceptance]::Policy($Path,0x7)
    [IntegrityAcceptance]::SetMediumNoReadUp($Path)
} elseif($Operation -ne 'after') {throw "Unknown integrity operation: $Operation"}
[ordered]@{initial_low=$initial;normal=[IntegrityAcceptance]::Read($Path,$false);
    low=[IntegrityAcceptance]::Read($Path,$true);initial_dacl=$initialDacl;
    dacl=[IntegrityAcceptance]::Policy($Path,0x7);
    policy=[IntegrityAcceptance]::Policy($Path,0x17)} | ConvertTo-Json -Depth 5 -Compress
''')

                def probe(operation):
                    result = subprocess.run(
                        ['powershell.exe', '-NoProfile', '-File', str(script), operation, str(target)],
                        capture_output=True, text=True, timeout=8)
                    assert result.returncode == 0, (operation, result.stdout, result.stderr)
                    return json.loads(result.stdout)

                before = probe('setup')
                assert before['initial_low']['allowed'] and before['initial_low']['text'] == 'original', before
                assert before['normal']['allowed'] and before['normal']['text'] == 'original', before
                assert before['initial_dacl'] == before['dacl'], before
                assert not before['low']['allowed'] and before['low']['error'] == 5, before
                assert '(ML;;NR;;;ME)' in before['policy'], before
                result = run(['get', 'h1', page, '-o', target])
                assert result.returncode == 0 and not result.stdout and not result.stderr, result
                after = probe('after')
                assert after['normal']['allowed'] and after['normal']['text'] == '"complete"\n', after
                assert not after['low']['allowed'] and after['low']['error'] == 5, after
                assert after['dacl'] == before['dacl'], {'before': before, 'after': after}
                assert after['policy'] == before['policy'], {'before': before, 'after': after}
                return {'before': before, 'after': after}
            check('native-windows-mandatory-integrity-enforcement', mandatory_integrity_enforcement)

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
    args.results.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(report['counts']))
    return 0 if results and all(c['status'] == 'pass' for c in results) else 1


if __name__ == '__main__':
    raise SystemExit(main())
