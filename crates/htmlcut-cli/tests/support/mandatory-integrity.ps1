# SPDX-License-Identifier: MPL-2.0
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
