# SPDX-License-Identifier: MPL-2.0
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
