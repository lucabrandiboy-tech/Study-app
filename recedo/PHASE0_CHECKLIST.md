# Recedo Phase 0: pen test checklist

This is a **test program**, not the painting app yet. It checks that your XP-Pen Artist 12
talks to Recedo properly through both of Windows' pen systems (Wintab and Windows Ink):
pressure, tilt, buttons, and how many pen readings per second arrive. Recedo itself is
built on top of this once you confirm it works.

It takes about 15 minutes. You don't need to understand any of the numbers: the report at
the end collects them for me.

## Before you start

1. Unzip `Recedo-TabletTest-Phase0.zip` (right-click it, **Extract All**).
2. Open the **XP-Pen driver** (the "Pentablet" app). On the pen settings page, make sure
   **Windows Ink** is ticked. (Needed for steps 4-10. If Wintab shows a red error later,
   try unticking it, restart the test app, and note whether that changed anything.)
3. Double-click **`recedo-tablet-test.exe`**.
   * If Windows shows **"Windows protected your PC"**: click **More info**, then
     **Run anyway**. (This appears for any program that isn't signed by a company.)
   * The window opens maximised on the monitor your mouse is on. If that's not the
     Artist 12, open **Screen and display** in the left panel and click **Move here**
     next to the Artist 12 (listed by name if the monitor reports one, otherwise as
     `\\.\DISPLAY…` with its size, 1920×1080), or drag the window there.

## Windows Ink

4. At the top left, **Tablet input API** should say **Windows Ink** (pick it if not).
   *Expected:* under it, a line "Windows Ink pen: …" naming your tablet.
5. Hold the pen just above the white area without touching.
   *Expected:* **Live pen** says "Windows Ink: hovering", the numbers change as you move,
   and a small **red cross sits exactly under the pen tip**.
6. Draw a line, pressing very lightly at first and harder towards the end.
   *Expected:* the line goes from thin to thick, smoothly. The **Pressure** bar moves
   from near 0 to near 1. Nothing is drawn while you only hover.
7. Hover and lean the pen: top of the pen to the right, then left, then toward you,
   then away from you.
   *Expected:* **Tilt X** is positive (+) when leaning right and negative (−) left;
   **Tilt Y** is positive leaning toward you and negative away. The blue line in the
   circle points the way the pen leans. Values up to about ±60° are normal.
   *Note:* not every Artist 12 pen has a tilt sensor. The newer models (Artist 12
   2nd Gen, Artist 12 Pro) do; the original 2018 Artist 12 most likely does not. If
   **Tilt X / Y** says "not reported" in both Windows Ink and Wintab, tell me which
   model you have (it's on the label on the back) and we'll check it.
8. Draw a few **fast** scribbles and S-curves.
   *Expected:* lines with **no gaps** between dots. (Small corners on very fast curves
   are expected here: this test draws the raw pen readings with no smoothing. The
   smoothing/stabilizer comes in the next phase.)
   Optional: tick **Show every sample as a red dot** to see each pen reading.
9. Look at **Event rate**, row **Ink**, while you move the pen.
   *Expected:* the **now** number is roughly **200 or more** while the pen moves.
   **lost** stays **0**. Note "per msg": above 1.0 is fine.
10. Press each side button on the pen while hovering.
    *Expected:* **Buttons** shows "lower side button" / "upper side button". If it shows
    "none", the XP-Pen driver has given the buttons another job (like right-click):
    just note what happens.

## Wintab

11. At the top, pick **Wintab**.
    *Expected:* a green line "Wintab: open (…your tablet's name…)". If it's **red**,
    copy the red text into the chat and skip to step 16.
12. Repeat steps 5-10 with Wintab.
    *Expected:* same results. In **Live pen**, pressure shows a "raw" number that goes
    up to about 8191 (the pen's full pressure range).
13. Check the **red cross** is right under the pen tip everywhere on the screen,
    including the corners.
    *If it's off:* open **Wintab details** and note what "In use:" says. Try
    **Monitor under the pen** in the "Screen mapping" list, and say which one works.
14. Draw continuously for about 30 seconds.
    *Expected:* **lost** in the Wintab row stays **0**.

## Both

15. If your pen has an **eraser end**: flip it and rub over a line.
    *Expected:* "ERASER end" appears in **Live pen**, and the line is erased (a grey
    checkerboard shows through). If your pen has no eraser end, skip this.
16. Is the text in the left panel **sharp** (not blurry)? Say what Windows display
    scaling you use (Settings → System → Display → Scale, e.g. 100%, 125%, 150%).
17. In **Recording and report**: tick **Record pen data**. Pick **Windows Ink**, draw one
    slow and one fast stroke; pick **Wintab**, do the same. Then click
    **Save recording + report**, and then **Copy report**.
18. **Paste the report into the chat** (Ctrl+V). If you can, also attach the two files it
    saved: they are in your **Documents\Recedo** folder (**Open folder** opens it).

## What I need from you

* Did **pressure** work with **Windows Ink**? With **Wintab**?
* Did **tilt** work with **Windows Ink**? With **Wintab**?
* Anything that looked wrong, in your own words (a screenshot helps).
* The pasted report.

When pressure and tilt work through both, say **go** and I'll start Phase 1 (the real
canvas: brush with the stabilizer, layers, undo, saving `.kra` files).
