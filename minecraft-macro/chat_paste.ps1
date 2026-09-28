# Minecraft chat macro - PowerShell (built into Windows, nothing to install)
# Press U in Minecraft. Does: scroll up, T, paste, Enter  (about 0.4 sec)
# Press F9 (or close this window) to stop.

Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class K {
    [DllImport("user32.dll")] public static extern short GetAsyncKeyState(int vKey);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll")] public static extern uint MapVirtualKey(uint code, uint mapType);
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, int dx, int dy, int data, UIntPtr extra);
    [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT { public int dx; public int dy; public int mouseData; public uint dwFlags; public uint time; public IntPtr extra; }
    [StructLayout(LayoutKind.Explicit)] public struct INPUT { [FieldOffset(0)] public uint type; [FieldOffset(8)] public MOUSEINPUT mi; }
    [StructLayout(LayoutKind.Explicit)] public struct INPUT32 { [FieldOffset(0)] public uint type; [FieldOffset(4)] public MOUSEINPUT mi; }
    [DllImport("user32.dll")] public static extern uint SendInput(uint n, INPUT[] i, int size);
    [DllImport("user32.dll", EntryPoint = "SendInput")] public static extern uint SendInput32(uint n, INPUT32[] i, int size);
    // Scroll wheel up one notch (like rolling the mouse wheel forward)
    public static void ScrollUp() {
        uint sent;
        if (IntPtr.Size == 8) {
            INPUT[] a = new INPUT[1]; a[0].type = 0; a[0].mi.mouseData = 120; a[0].mi.dwFlags = 0x0800;
            sent = SendInput(1, a, 40);
        } else {
            INPUT32[] a = new INPUT32[1]; a[0].type = 0; a[0].mi.mouseData = 120; a[0].mi.dwFlags = 0x0800;
            sent = SendInput32(1, a, 28);
        }
        if (sent == 0) mouse_event(0x0800, 0, 0, 120, UIntPtr.Zero);   // fallback
    }
    public static void Down(byte vk) { keybd_event(vk, (byte)MapVirtualKey(vk, 0), 0, UIntPtr.Zero); }
    public static void Up(byte vk)   { keybd_event(vk, (byte)MapVirtualKey(vk, 0), 2, UIntPtr.Zero); }
    public static void Tap(byte vk)  { Down(vk); System.Threading.Thread.Sleep(20); Up(vk); }
    public static string Title() { var sb = new StringBuilder(256); GetWindowText(GetForegroundWindow(), sb, 256); return sb.ToString(); }
}
"@

# ---- Timing (milliseconds). Whole thing takes about 0.4 sec. ----
$AfterScroll = 40    # after scrolling up
$ChatOpen    = 200   # wait for chat box to open before pasting (raise if paste gets lost)
$AfterPaste  = 40    # before pressing Enter

$VK_LCTRL = 0xA2; $VK_V = 0x56; $VK_T = 0x54; $VK_ENTER = 0x0D

# Only allow one copy of the macro to run (two copies = everything happens twice)
$mutex = New-Object System.Threading.Mutex($false, "MinecraftChatMacro")
if (-not $mutex.WaitOne(0)) { Write-Host "The macro is already running in another window. Close this one."; exit }

Write-Host "Macro running. Copy your text, then press U in Minecraft. F9 = stop."
$wasDown = $false
while ($true) {
    if ([K]::GetAsyncKeyState(0x78) -band 0x8000) { break }      # F9 = quit
    $isDown = ([K]::GetAsyncKeyState(0x55) -band 0x8000) -ne 0     # U
    if ($isDown -and -not $wasDown -and ([K]::Title() -like "*Minecraft*")) {
        while ([K]::GetAsyncKeyState(0x55) -band 0x8000) { Start-Sleep -Milliseconds 10 }  # wait until U is let go
        [K]::ScrollUp();              Start-Sleep -Milliseconds $AfterScroll   # scroll up
        [K]::Tap($VK_T);              Start-Sleep -Milliseconds $ChatOpen      # T
        [K]::Down($VK_LCTRL); Start-Sleep -Milliseconds 20                     # paste (Ctrl+V)
        [K]::Tap($VK_V);      Start-Sleep -Milliseconds 20
        [K]::Up($VK_LCTRL);           Start-Sleep -Milliseconds $AfterPaste
        [K]::Tap($VK_ENTER)                                                    # Enter
        Write-Host "Done."
        Start-Sleep -Milliseconds 300          # ignore extra U presses right after
        $isDown = ([K]::GetAsyncKeyState(0x55) -band 0x8000) -ne 0
    }
    $wasDown = $isDown
    Start-Sleep -Milliseconds 15
}
Write-Host "Macro stopped."
