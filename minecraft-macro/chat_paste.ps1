# Minecraft chat macro - PowerShell (built into Windows, nothing to install)
# Press U in Minecraft. Does exactly:
# 1,T,paste,Enter  2,T,paste,Enter  ...  8,T,paste,Enter  9
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
    public static void Down(byte vk) { keybd_event(vk, (byte)MapVirtualKey(vk, 0), 0, UIntPtr.Zero); }
    public static void Up(byte vk)   { keybd_event(vk, (byte)MapVirtualKey(vk, 0), 2, UIntPtr.Zero); }
    public static void Tap(byte vk)  { Down(vk); System.Threading.Thread.Sleep(20); Up(vk); }
    public static string Title() { var sb = new StringBuilder(256); GetWindowText(GetForegroundWindow(), sb, 256); return sb.ToString(); }
}
"@

# ---- Timing (milliseconds). Raise these if steps get skipped. ----
$AfterNumber = 40    # after pressing 1-9
$ChatOpen    = 120   # wait for chat box to open before pasting
$AfterPaste  = 40    # before pressing Enter
$AfterEnter  = 250   # wait for chat to close before next slot

$VK_LCTRL = 0xA2; $VK_V = 0x56; $VK_T = 0x54; $VK_ENTER = 0x0D

Write-Host "Macro running. Copy your text, then press U in Minecraft. F9 = stop."
$wasDown = $false
while ($true) {
    if ([K]::GetAsyncKeyState(0x78) -band 0x8000) { break }      # F9 = quit
    $isDown = ([K]::GetAsyncKeyState(0x55) -band 0x8000) -ne 0     # U
    if ($isDown -and -not $wasDown -and ([K]::Title() -like "*Minecraft*")) {
        while ([K]::GetAsyncKeyState(0x55) -band 0x8000) { Start-Sleep -Milliseconds 10 }  # wait until U is let go
        # --- 1, T, paste, Enter ---
        [K]::Tap(0x31); Start-Sleep -Milliseconds $AfterNumber
        [K]::Tap($VK_T); Start-Sleep -Milliseconds $ChatOpen
        [K]::Down($VK_LCTRL); Start-Sleep -Milliseconds 20; [K]::Tap($VK_V); Start-Sleep -Milliseconds 20; [K]::Up($VK_LCTRL); Start-Sleep -Milliseconds $AfterPaste
        [K]::Tap($VK_ENTER); Start-Sleep -Milliseconds $AfterEnter
        # --- 2, T, paste, Enter ---
        [K]::Tap(0x32); Start-Sleep -Milliseconds $AfterNumber
        [K]::Tap($VK_T); Start-Sleep -Milliseconds $ChatOpen
        [K]::Down($VK_LCTRL); Start-Sleep -Milliseconds 20; [K]::Tap($VK_V); Start-Sleep -Milliseconds 20; [K]::Up($VK_LCTRL); Start-Sleep -Milliseconds $AfterPaste
        [K]::Tap($VK_ENTER); Start-Sleep -Milliseconds $AfterEnter
        # --- 3, T, paste, Enter ---
        [K]::Tap(0x33); Start-Sleep -Milliseconds $AfterNumber
        [K]::Tap($VK_T); Start-Sleep -Milliseconds $ChatOpen
        [K]::Down($VK_LCTRL); Start-Sleep -Milliseconds 20; [K]::Tap($VK_V); Start-Sleep -Milliseconds 20; [K]::Up($VK_LCTRL); Start-Sleep -Milliseconds $AfterPaste
        [K]::Tap($VK_ENTER); Start-Sleep -Milliseconds $AfterEnter
        # --- 4, T, paste, Enter ---
        [K]::Tap(0x34); Start-Sleep -Milliseconds $AfterNumber
        [K]::Tap($VK_T); Start-Sleep -Milliseconds $ChatOpen
        [K]::Down($VK_LCTRL); Start-Sleep -Milliseconds 20; [K]::Tap($VK_V); Start-Sleep -Milliseconds 20; [K]::Up($VK_LCTRL); Start-Sleep -Milliseconds $AfterPaste
        [K]::Tap($VK_ENTER); Start-Sleep -Milliseconds $AfterEnter
        # --- 5, T, paste, Enter ---
        [K]::Tap(0x35); Start-Sleep -Milliseconds $AfterNumber
        [K]::Tap($VK_T); Start-Sleep -Milliseconds $ChatOpen
        [K]::Down($VK_LCTRL); Start-Sleep -Milliseconds 20; [K]::Tap($VK_V); Start-Sleep -Milliseconds 20; [K]::Up($VK_LCTRL); Start-Sleep -Milliseconds $AfterPaste
        [K]::Tap($VK_ENTER); Start-Sleep -Milliseconds $AfterEnter
        # --- 6, T, paste, Enter ---
        [K]::Tap(0x36); Start-Sleep -Milliseconds $AfterNumber
        [K]::Tap($VK_T); Start-Sleep -Milliseconds $ChatOpen
        [K]::Down($VK_LCTRL); Start-Sleep -Milliseconds 20; [K]::Tap($VK_V); Start-Sleep -Milliseconds 20; [K]::Up($VK_LCTRL); Start-Sleep -Milliseconds $AfterPaste
        [K]::Tap($VK_ENTER); Start-Sleep -Milliseconds $AfterEnter
        # --- 7, T, paste, Enter ---
        [K]::Tap(0x37); Start-Sleep -Milliseconds $AfterNumber
        [K]::Tap($VK_T); Start-Sleep -Milliseconds $ChatOpen
        [K]::Down($VK_LCTRL); Start-Sleep -Milliseconds 20; [K]::Tap($VK_V); Start-Sleep -Milliseconds 20; [K]::Up($VK_LCTRL); Start-Sleep -Milliseconds $AfterPaste
        [K]::Tap($VK_ENTER); Start-Sleep -Milliseconds $AfterEnter
        # --- 8, T, paste, Enter ---
        [K]::Tap(0x38); Start-Sleep -Milliseconds $AfterNumber
        [K]::Tap($VK_T); Start-Sleep -Milliseconds $ChatOpen
        [K]::Down($VK_LCTRL); Start-Sleep -Milliseconds 20; [K]::Tap($VK_V); Start-Sleep -Milliseconds 20; [K]::Up($VK_LCTRL); Start-Sleep -Milliseconds $AfterPaste
        [K]::Tap($VK_ENTER); Start-Sleep -Milliseconds $AfterEnter
        # --- 9 ---
        [K]::Tap(0x39)
        Write-Host "Done."
    }
    $wasDown = $isDown
    Start-Sleep -Milliseconds 15
}
Write-Host "Macro stopped."
