!include "WinMessages.nsh"

Function BroadcastEnvironmentChange
  System::Call 'user32::SendMessageTimeout(p 0xffff, i ${WM_SETTINGCHANGE}, p 0, t "Environment", i 2, i 5000, *p .r0)'
FunctionEnd

Function un.BroadcastEnvironmentChange
  System::Call 'user32::SendMessageTimeout(p 0xffff, i ${WM_SETTINGCHANGE}, p 0, t "Environment", i 2, i 5000, *p .r0)'
FunctionEnd

Function PathContainsEntry
  StrCpy $2 "0"
  StrCpy $3 $0

  loop:
    StrCmp $3 "" done
    StrCpy $4 0

    find_separator:
      StrCpy $5 $3 1 $4
      StrCmp $5 "" segment_done
      StrCmp $5 ";" segment_done
      IntOp $4 $4 + 1
      Goto find_separator

    segment_done:
      StrCpy $5 $3 $4
      StrCmp $5 "" advance
      StrCmp $5 $1 found advance

    advance:
      StrCmp $5 "" 0 +2
      IntOp $4 $4 + 1
      StrCpy $3 $3 "" $4
      Goto loop

    found:
      StrCpy $2 "1"

    done:
FunctionEnd

Function RemovePathEntry
  StrCpy $2 ""
  StrCpy $3 $0

  loop:
    StrCmp $3 "" done
    StrCpy $4 0

    find_separator:
      StrCpy $5 $3 1 $4
      StrCmp $5 "" segment_done
      StrCmp $5 ";" segment_done
      IntOp $4 $4 + 1
      Goto find_separator

    segment_done:
      StrCpy $5 $3 $4
      StrCmp $5 "" advance
      StrCmp $5 $1 advance
      StrCmp $2 "" 0 append
      StrCpy $2 $5
      Goto advance
      append:
        StrCpy $2 "$2;$5"

    advance:
      StrCmp $5 "" 0 +2
      IntOp $4 $4 + 1
      StrCpy $3 $3 "" $4
      Goto loop

    done:
FunctionEnd

Function un.RemovePathEntry
  StrCpy $2 ""
  StrCpy $3 $0

  loop:
    StrCmp $3 "" done
    StrCpy $4 0

    find_separator:
      StrCpy $5 $3 1 $4
      StrCmp $5 "" segment_done
      StrCmp $5 ";" segment_done
      IntOp $4 $4 + 1
      Goto find_separator

    segment_done:
      StrCpy $5 $3 $4
      StrCmp $5 "" advance
      StrCmp $5 $1 advance
      StrCmp $2 "" 0 append
      StrCpy $2 $5
      Goto advance
      append:
        StrCpy $2 "$2;$5"

    advance:
      StrCmp $5 "" 0 +2
      IntOp $4 $4 + 1
      StrCpy $3 $3 "" $4
      Goto loop

    done:
FunctionEnd

Function AddInstallDirToUserPath
  ClearErrors
  ReadRegStr $0 HKCU "Environment" "Path"
  StrCmp $0 "" 0 +3
  ClearErrors
  ReadRegStr $0 HKCU "Environment" "PATH"
  StrCpy $1 "$INSTDIR"

  StrCmp $0 "" write_only

  Call PathContainsEntry
  StrCmp $2 "1" done

  WriteRegExpandStr HKCU "Environment" "Path" "$0;$1"
  Goto refresh

  write_only:
    WriteRegExpandStr HKCU "Environment" "Path" "$1"

  refresh:
    Call BroadcastEnvironmentChange

  done:
FunctionEnd

Function RemoveInstallDirFromUserPath
  ClearErrors
  ReadRegStr $0 HKCU "Environment" "Path"
  StrCmp $0 "" 0 +3
  ClearErrors
  ReadRegStr $0 HKCU "Environment" "PATH"
  StrCmp $0 "" done

  StrCpy $1 "$INSTDIR"
  Call RemovePathEntry

  StrCmp $2 "" clear_path
  WriteRegExpandStr HKCU "Environment" "Path" "$2"
  Goto refresh

  clear_path:
    DeleteRegValue HKCU "Environment" "Path"

  refresh:
    Call BroadcastEnvironmentChange

  done:
FunctionEnd

Function un.RemoveInstallDirFromUserPath
  ClearErrors
  ReadRegStr $0 HKCU "Environment" "Path"
  StrCmp $0 "" 0 +3
  ClearErrors
  ReadRegStr $0 HKCU "Environment" "PATH"
  StrCmp $0 "" done

  StrCpy $1 "$INSTDIR"
  Call un.RemovePathEntry

  StrCmp $2 "" clear_path
  WriteRegExpandStr HKCU "Environment" "Path" "$2"
  Goto refresh

  clear_path:
    DeleteRegValue HKCU "Environment" "Path"

  refresh:
    Call un.BroadcastEnvironmentChange

  done:
FunctionEnd

!macro NSIS_HOOK_POSTINSTALL
  Call AddInstallDirToUserPath
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  Call un.RemoveInstallDirFromUserPath
!macroend
