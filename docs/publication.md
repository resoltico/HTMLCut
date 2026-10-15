# Output publication

The CLI computes and validates a complete result before publishing it. `-o DEST` or
`--output DEST` selects the destination; `-` means standard output. Extraction refusals
and limits before publication leave the destination unchanged, including a FIFO without
a reader. Temporary staging can still require cleanup as described below.

## Destination behavior

Symbolic links are followed to the destination a write would reach. A writable regular
file is replaced atomically after staging the complete result; an absent path is created
atomically. The caller must be able to open an existing file for writing without
truncation and create and rename entries in its containing directory. Directory listing
permission is not required, but write and search/traversal authorization is required.
A directory destination is refused.

Pipes, devices and descriptor aliases such as `/dev/stdout` and `/dev/fd/N` receive direct
writes. Local descriptor aliases duplicate the inherited descriptor, preserving its
cursor, append flag and access mode. Foreign-process descriptor paths are opened as path
streams rather than duplicated local descriptors. Direct output has no atomicity
promise: write or flush failure can expose a prefix. A reader closing a pipe is silent
success, exit 0; other delivery failures are `output_unwritable`, exit 5.

Regular-file replacement changes the named file's identity. Other hard links and already
open handles retain the old object. Publication does not promise fsync durability.

## Authorization and private staging

Replacement preserves inspected owner, group, read/write/execute permissions,
discretionary ACLs and applicable mandatory access labels. Privileged executable bits,
file capabilities and unrelated content extended attributes are not copied. A new Unix
file follows ordinary mode `0666` creation policy, with umask or an inherited default ACL
as applicable. Replacement discards set-user-ID, set-group-ID and sticky bits. A new
destination follows the caller's and parent directory's ownership policy. An empty policy probe captures
inherited authorization for an absent destination; it never contains result bytes.

Owner, group and applicable access labels are established and verified under restrictive
staging before any result byte. Unix result files begin at mode zero in a verified private
directory with inherited access/default ACLs removed. Darwin additionally uses a private
deny-read ACL during metadata inspection. Final ACLs precede mode grants. Only complete
content receives the final discretionary policy.

Windows result files are created with the captured mandatory SACL and its control flags,
and a protected DACL granting the owner only writer, metadata and deletion rights, without
data-read access. Through the retained writer handle, an empty protected DACL is installed
before bytes are written. A pathname reader cannot acquire a data handle before restriction.
A distinct empty policy probe verifies exact reproduction before result bytes, including
native inheritance processing; a retained inherited ACL that would gain different parent
permissions refuses. Readers of that probe retain a separate empty object. Final policy
readback must still match exactly. Inspection or reproduction failure refuses before
replacement.

### Windows requirements and restrictions

Preserving a protected SACL requires caller-assigned `SeSecurityPrivilege`. HTMLCut
activates it only on a duplicate effective token on the current thread during private
creation and mandatory initialization, then restores the exact prior thread context.
It does not assign the privilege or alter the process/original token. Missing assignment
refuses before result bytes. Irrecoverable context-restoration failure aborts; after
creation this can leave an empty protected, unconfirmed stage with no result bytes and
potentially no diagnostic.

Unprotected present-null SACLs are unsupported by this publication path and refuse before
private result-file creation or result bytes because the path cannot faithfully reproduce
their inheritance controls. Protected null and supported non-null mandatory policies
remain subject to exact reproduction and native access requirements.

Resource attributes, central-policy scope, process trust and access-filter SACL components
are inspected. Detected access-bearing policy beyond supported mandatory integrity labels
refuses before result bytes; it is not discarded. Unsupported security queries also
refuse, including on kernels that lack the required query capabilities. Audit-only SACL
backup is excluded. Privilege activation does not bypass data-read ACLs or copy audit SACLs.

## Deadline, cleanup and concurrent changes

The [invocation deadline](design.md#limits-and-time) includes acquisition, stream writes
and flushes, regular-file staging and metadata. The regular-file commit boundary checks
the deadline and atomically competes with expiry. Once commit wins, the actual rename outcome governs; there is no
contradictory timeout or rollback promise. A stuck filesystem operation can outlast the
deadline and cleanup grace.

Staging ownership is reserved before exclusive creation, confirmed before result bytes,
and checked when creation returns. Ordinary and timeout cleanup remove only objects whose
ownership is established, using identity checks or a retained handle to the newly created object. Detected foreign
replacements are left untouched. Creation can succeed while access denial or a failed identity query prevents confirmation.
That can leave an empty unconfirmed file or directory; available diagnostics name possible
residue. Hard termination and cleanup run separately, so extraordinary blocked syscalls
can also leave a candidate or stage beyond the cleanup grace.

Containing-parent identity, target identity and policy, original path resolution, and
stage identity and policy are revalidated immediately before commit. The guarantee assumes
these identities and policies remain stable through publication. Native reconstruction,
replacement and unlink cannot eliminate every hostile namespace or policy mutation after
revalidation; this is not isolation from concurrent changes by other processes.
