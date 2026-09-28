; Minecraft Java chat macro (AutoHotkey v2)
; Press U in-game: T (open chat) -> Ctrl+V (paste) -> Enter (send)
; Press F9 to exit the script.
#Requires AutoHotkey v2.0
#SingleInstance Force

#HotIf WinActive("ahk_exe javaw.exe") or WinActive("ahk_exe java.exe")
$u:: {
    SendEvent "{t down}"
    Sleep 30
    SendEvent "{t up}"
    Sleep 150            ; give the chat box time to open
    SendEvent "^v"
    Sleep 50
    SendEvent "{Enter}"
}
#HotIf

F9::ExitApp
