!macro NSIS_HOOK_PREUNINSTALL
  ; LightProxy restores the proxy snapshot from state.json on quit. The uninstaller
  ; cannot call the app, so this hook only leaves a marker for manual recovery.
  DetailPrint "LightProxy: 若系统代理仍指向 LightProxy，请在卸载后手动恢复。"
!macroend
