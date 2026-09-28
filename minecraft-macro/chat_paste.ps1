# Minecraft chat macro - PowerShell (built into Windows, nothing to install)
# Press U in Minecraft: for slots 1-9: number, T, paste, Enter (~4.5 sec total)
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
    // Types the whole text into the chat box in one go (fast)
    public static void TypeText(string text) {
        text = text.Replace("\r", "").Replace("\n", "");
        INPUT[] ins = new INPUT[text.Length * 2];
        for (int i = 0; i < text.Length; i++) {
            ins[2*i].type = 1;   ins[2*i].ki.wScan = text[i];   ins[2*i].ki.dwFlags = 4;       // KEYEVENTF_UNICODE
            ins[2*i+1].type = 1; ins[2*i+1].ki.wScan = text[i]; ins[2*i+1].ki.dwFlags = 4 | 2; // + KEYUP
        }
        if (ins.Length > 0) SendInput((uint)ins.Length, ins, Marshal.SizeOf(typeof(INPUT)));
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
        $text = Get-Clipboard -Raw
        for ($n = 1; $n -le 9; $n++) {
            [K]::Tap([byte](0x30 + $n))    # 1..9 - hotbar slot
            Start-Sleep -Milliseconds 40
            [K]::Tap(0x54)                 # T - open chat
            Start-Sleep -Milliseconds 120  # wait for chat box (raise if text gets lost)
            if ($text) { [K]::TypeText($text) }   # paste
            Start-Sleep -Milliseconds 30
            [K]::Tap(0x0D)                 # Enter - send
            Start-Sleep -Milliseconds 250  # let chat close before next slot
        }
    }
    $wasDown = $isDown
    Start-Sleep -Milliseconds 15
}
Write-Host "Macro stopped."
