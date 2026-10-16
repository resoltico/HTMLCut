# SPDX-License-Identifier: MPL-2.0
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
$context=[ordered]@{sid=$sid.Value;backup_before=[ParentAccessControl]::DisableExisting('SeBackupPrivilege');restore_before=[ParentAccessControl]::DisableExisting('SeRestorePrivilege');security_before=[ParentAccessControl]::DisableExisting('SeSecurityPrivilege');backup_enabled=$false;restore_enabled=$false;security_enabled=$false}
New-Item -ItemType Directory -Path $Parent | Out-Null
$saved=Get-Acl -LiteralPath $Parent
[System.IO.File]::WriteAllBytes($PolicyBackup,$saved.GetSecurityDescriptorBinaryForm())
function RestoreParentDacl() {
    $acl=[System.Security.AccessControl.DirectorySecurity]::new()
    $acl.SetSecurityDescriptorBinaryForm([System.IO.File]::ReadAllBytes($PolicyBackup),[System.Security.AccessControl.AccessControlSections]::Access)
    ([System.IO.DirectoryInfo]::new($Parent)).SetAccessControl($acl)
}
function ParentRule([int]$Rights) {
    $acl=[System.Security.AccessControl.DirectorySecurity]::new()
    $acl.SetSecurityDescriptorBinaryForm([System.IO.File]::ReadAllBytes($PolicyBackup),[System.Security.AccessControl.AccessControlSections]::Access)
    $rule=[System.Security.AccessControl.FileSystemAccessRule]::new($sid,[System.Security.AccessControl.FileSystemRights]$Rights,[System.Security.AccessControl.InheritanceFlags]::None,[System.Security.AccessControl.PropagationFlags]::None,[System.Security.AccessControl.AccessControlType]::Deny)
    $acl.AddAccessRule($rule)
    ([System.IO.DirectoryInfo]::new($Parent)).SetAccessControl($acl)
}
function InvokeDeliveredBinary([string]$Path) {
    $info=[System.Diagnostics.ProcessStartInfo]::new()
    $info.FileName=$Binary;$info.Arguments=('get h1 "{0}" -o "{1}"' -f $Page,$Path)
    $info.UseShellExecute=$false;$info.RedirectStandardOutput=$true;$info.RedirectStandardError=$true
    $child=[System.Diagnostics.Process]::Start($info)
    $failure=$null;$outcome=$null;$cleanup=@()
    try {
        $privileges=[ordered]@{backup_enabled=[ParentAccessControl]::PrivilegeEnabled($child.Handle,'SeBackupPrivilege');restore_enabled=[ParentAccessControl]::PrivilegeEnabled($child.Handle,'SeRestorePrivilege');security_enabled=[ParentAccessControl]::PrivilegeEnabled($child.Handle,'SeSecurityPrivilege');traverse_bypass_enabled=[ParentAccessControl]::PrivilegeEnabled($child.Handle,'SeChangeNotifyPrivilege')}
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
    $results.existing_policy_before_priming=[ParentAccessControl]::Policy($existing)
    # Initialize this owned child's native inherited DACL before the baseline.
    # The actual listing-denial update must still leave that policy unchanged.
    RestoreParentDacl
    $results.existing_policy_after_priming=[ParentAccessControl]::Policy($existing)
    $results.existing_policy_before_setup=[ParentAccessControl]::Policy($existing)
    ParentRule 1 # ListDirectory only; no child inheritance.
    $results.existing_policy_after_setup=[ParentAccessControl]::Policy($existing)
    if($results.existing_policy_after_setup-ne$results.existing_policy_before_setup) {throw 'Parent-only DACL fixture altered existing child policy'}
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
        try {RestoreParentDacl} catch {$cleanup+="Parent ACL restoration: $($_.Exception.Message)"}
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
