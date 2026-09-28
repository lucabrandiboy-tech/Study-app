# Minecraft chat macro - PowerShell (built into Windows, nothing to install)
# Press U in Minecraft: T (open chat) -> paste -> Enter (send) -> scroll wheel up
# Press F9 (or close this window) to stop.

Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class K {
    [DllImport("user32.dll")] public static extern short GetAsyncKeyState(int vKey);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] public static extern uint MapVirtualKey(uint code, uint mapType);
    [DllImport("user32.dll")] public static extern uint SendInput(uint n, INPUT[] inputs, int size);
    [StructLayout(LayoutKind.Sequential)] public struct KEYBDINPUT { public ushort wVk; public ushort wScan; public uint dwFlags; public uint time; public IntPtr extra; }
    [StructLayout(LayoutKind.Explicit, Size = 40)] public struct INPUT { [FieldOffset(0)] public uint type; [FieldOffset(8)] public KEYBDINPUT ki; }
    public static void Down(byte vk) { keybd_event(vk, (byte)MapVirtualKey(vk, 0), 0, UIntPtr.Zero); }
    public static void Up(byte vk)   { keybd_event(vk, (byte)MapVirtualKey(vk, 0), 2, UIntPtr.Zero); }
    // Types text straight into the chat box (works even if Ctrl+V doesn't)
    public static void TypeText(string text) {
        foreach (char c in text) {
            if (c == '\r' || c == '\n') continue;
            INPUT[] ins = new INPUT[2];
            ins[0].type = 1; ins[0].ki.wScan = c; ins[0].ki.dwFlags = 4;       // KEYEVENTF_UNICODE
            ins[1].type = 1; ins[1].ki.wScan = c; ins[1].ki.dwFlags = 4 | 2;   // + KEYUP
            SendInput(2, ins, Marshal.SizeOf(typeof(INPUT)));
        }
    }
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, int dx, int dy, int data, UIntPtr extra);
    public static void ScrollUp() { mouse_event(0x0800, 0, 0, 120, UIntPtr.Zero); }   // mouse wheel up one notch
    public static void Tap(byte vk)  { Down(vk); System.Threading.Thread.Sleep(3); Up(vk); }
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
        $text = Get-Clipboard -Raw     # read clipboard first (saves time)
        Start-Sleep -Milliseconds 9
        [K]::Tap(0x54)                 # T  - open chat
        Start-Sleep -Milliseconds 17   # wait for chat box (raise to 50 if paste gets lost)
        if ($text) { [K]::TypeText($text) }   # paste clipboard text
        Start-Sleep -Milliseconds 6
        [K]::Tap(0x0D)                 # Enter - send
        Start-Sleep -Milliseconds 10   # let chat close so the scroll reaches the hotbar
        [K]::ScrollUp()                # scroll wheel up
    }
    $wasDown = $isDown
    Start-Sleep -Milliseconds 15
}
Write-Host "Macro stopped."
