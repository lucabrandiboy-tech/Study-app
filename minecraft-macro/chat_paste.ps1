# Minecraft chat macro - PowerShell (built into Windows, nothing to install)
# Press U in Minecraft: T (open chat) -> Ctrl+V (paste) -> Enter (send)
# Press F9 (or close this window) to stop.

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class K {
    [DllImport("user32.dll")] public static extern short GetAsyncKeyState(int vKey);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
    public static void Down(byte vk) { keybd_event(vk, 0, 0, UIntPtr.Zero); }
    public static void Up(byte vk)   { keybd_event(vk, 0, 2, UIntPtr.Zero); }
    public static void Tap(byte vk)  { Down(vk); System.Threading.Thread.Sleep(30); Up(vk); }
}
"@

function Test-MinecraftFocused {
    $procId = [uint32]0
    [void][K]::GetWindowThreadProcessId([K]::GetForegroundWindow(), [ref]$procId)
    $p = Get-Process -Id $procId -ErrorAction SilentlyContinue
    return $p -and ($p.ProcessName -eq 'javaw' -or $p.ProcessName -eq 'java')
}

Write-Host "Macro running. Press U in Minecraft. Press F9 to stop."
$wasDown = $false
while ($true) {
    if ([K]::GetAsyncKeyState(0x78) -band 0x8000) { break }   # F9 = quit
    $isDown = ([K]::GetAsyncKeyState(0x55) -band 0x8000) -ne 0  # U
    if ($isDown -and -not $wasDown -and (Test-MinecraftFocused)) {
        Start-Sleep -Milliseconds 80
        [K]::Tap(0x54)                 # T  - open chat
        Start-Sleep -Milliseconds 150  # wait for chat box (raise to 250 if paste gets lost)
        [K]::Down(0x11); [K]::Tap(0x56); [K]::Up(0x11)   # Ctrl+V - paste
        Start-Sleep -Milliseconds 50
        [K]::Tap(0x0D)                 # Enter - send
    }
    $wasDown = $isDown
    Start-Sleep -Milliseconds 15
}
Write-Host "Macro stopped."
