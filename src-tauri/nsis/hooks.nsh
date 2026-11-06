!macro NSIS_HOOK_PREUNINSTALL
  ; Updates keep the running tray app, which still owns the proxy snapshot, so
  ; only a real uninstall restores. Exit code 2 means an external program owns
  ; the registry now; that configuration is intentionally left alone.
  ${If} $UpdateMode <> 1
    ; Close the tray app first so it cannot rewrite the registry while we
    ; restore. The template repeats this check after the hook.
    !insertmacro CheckIfAppIsRunning "$INSTDIR\${MAINBINARYNAME}.exe" "${PRODUCTNAME}"
    DetailPrint "LightProxy: 正在恢复接管前的系统代理..."
    ExecWait '"$INSTDIR\LightProxy.exe" --restore-snapshot' $0
    ${If} $0 == 1
      DetailPrint "LightProxy: 自动恢复失败，请手动检查系统代理设置。"
    ${ElseIf} $0 == 2
      DetailPrint "LightProxy: 系统代理已由其他程序修改，已保留当前设置。"
    ${EndIf}
  ${EndIf}
!macroend
