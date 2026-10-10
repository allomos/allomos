; Installer hooks for Windows (Task-A07-06). The installer puts alm.exe beside the app; these
; hooks add the app's folder to the person's own PATH, so `alm` works in every new terminal,
; and take it off again on uninstall. No administrator rights are needed.
;
; PATH is changed by PowerShell on the raw registry value (variables such as %USERPROFILE% kept
; as written): NSIS strings stop at 1024 characters, which would cut a long PATH short. Only the
; app's own entry is added or removed; the rest of PATH is left exactly as it was. The folder is
; passed in an environment variable, so a quote in it cannot break the command. `$$` is NSIS's
; way of writing PowerShell's `$`.

; Runs PowerShell `SCRIPT` with $$d (the app's folder), $$k (the person's Environment key),
; $$p (their raw PATH) and $$parts (its entries) set, then announces the change.
!macro ALLOMOS_PATH SCRIPT
  System::Call 'kernel32::SetEnvironmentVariable(t "ALLOMOS_DIR", t "$INSTDIR")'
  nsExec::Exec `powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command "$$d = $$env:ALLOMOS_DIR.TrimEnd('\'); $$k = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Environment'); $$p = [string]$$k.GetValue('Path', '', 'DoNotExpandEnvironmentNames'); $$parts = @($$p -split ';'); ${SCRIPT}; $$k.Close()"`
  Pop $0
  ; Tell running programs (Explorer, so new terminals) that the environment changed.
  SendMessage 0xFFFF 0x001A 0 "STR:Environment" /TIMEOUT=5000
!macroend

; Add the folder at the end, unless it is already there.
!macro NSIS_HOOK_POSTINSTALL
  !insertmacro ALLOMOS_PATH "if (-not ($$parts | Where-Object { $$_.TrimEnd('\') -eq $$d })) { $$new = if ($$p -eq '') { $$d } elseif ($$p.EndsWith(';')) { $$p + $$d } else { $$p + ';' + $$d }; $$k.SetValue('Path', $$new, 'ExpandString') }"
!macroend

; Remove only the folder's own entry.
!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro ALLOMOS_PATH "$$keep = @($$parts | Where-Object { $$_.TrimEnd('\') -ne $$d }); if ($$keep.Count -ne $$parts.Count) { $$k.SetValue('Path', ($$keep -join ';'), 'ExpandString') }"
!macroend
