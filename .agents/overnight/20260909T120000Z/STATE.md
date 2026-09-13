# STATE 20260909T143000Z
HEAD ecda842 branch main version 0.0.8
DIRTY: ~64 M files waves1-3, preserved, no reset
J01 DONE VERIFIED | J02-J06/J07/J09/J10/J11/J13 PATCH_APPLIED wave gates GREEN | J12 VERIFY-PASS | J08 PATCH_APPLIED wave3 GREEN
SOAK33: .performance/20260909T165510Z-1545976 (.keep pinned), 13:55-14:28 UTC ~33min idle, gdb=shell
  NO shell SEGV (P0_NOT_REPRODUCED idle); crash.txt only wm TERM secondary teardown; shell.gdb.log = X :97 broken at shutdown, no backtrace
  PSS: wm 4577->4721 (+144, slope +0.05), desktop 13114->13146 (+32, +0.01), shell 5498->6100 (+602, +36.4 unreliable, n=37 broken t=1 stamps)
  combined warm ~24MB <=40MB target on idle window
READY: stress soak with workload (icon/popup/workspace loops from §21/§25), G01-G06, pixel proofs, perf before/after
HOST: :0 untouched, NO-FLAME-PROC before runs, nested :97 only
