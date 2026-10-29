# Recedo Phase 1: setting it up and trying it

This is the first version of **Recedo itself**: a real painting window with a round brush,
eraser, eyedropper, layers, undo, Krita files, PNG/JPEG export and autosave, and since then
32 tools in all (fill, shapes, gradient, smudge and other retouch brushes, selections, move,
crop, text; part 5). Filters and the rest come in later phases (list at the end).

## 1. Get it running (5 minutes)

1. **Download** `Recedo-Phase1.zip` from the chat (tap/click it, then Download).
2. **Unzip it.** In your Downloads folder, right-click `Recedo-Phase1.zip` and pick
   **Extract All...**, then **Extract**. You get a folder with three files:
   `recedo.exe`, `READ ME FIRST.txt` (this guide) and `LICENCES.txt`.
   Move the folder wherever you like (Desktop is fine). There is no installer, and nothing
   else to download.
3. **Start it:** double-click **`recedo.exe`**.
   * If Windows says **"Windows protected your PC"**: click **More info**, then
     **Run anyway**. It says this about any program that is not signed by a company.
   * Optional: right-click `recedo.exe` > **Show more options** > **Pin to Start** or
     **Send to > Desktop (create shortcut)**.
4. The window opens maximised on the screen your mouse is on. To move it to the Artist 12:
   click the window, then press **Windows key + Shift + Left arrow** (or Right arrow)
   until it is there. Next time it opens where you left it.
5. **XP-Pen driver:** open the XP-Pen "Pentablet" app and make sure **Windows Ink** is
   ticked on the pen page (it normally is). Recedo uses Windows Ink unless you change it
   (see 3. below).

Nothing is sent anywhere: Recedo has no internet code at all.

## 2. Try painting (15-20 minutes)

The status bar at the bottom shows the painting's name, size, zoom, the tool, and at the
right which pen system is in use (it should name your tablet).

Each step says what you should see. If something is different, note the step number.

1. Hover the pen over the white canvas without touching.
   *Expected:* a thin circle showing the brush size follows the pen tip exactly (for a
   very small brush, a small cross is drawn too).
2. Draw a line pressing lightly, then harder.
   *Expected:* thin to thick, smooth, no gaps, no delay you notice.
3. Draw fast scribbles. *Expected:* smooth lines, no dots or corners.
4. Press **[** and **]**. *Expected:* the circle shrinks/grows. **I** / **O** lower/raise the
   opacity (Tool property shows the numbers).
5. Press **E**, paint over a line. *Expected:* it erases. **E** again: back to the brush.
   If your pen has an eraser end, flip it: it erases even while the brush is chosen.
6. Hold **Ctrl** and tap a colour on the canvas. *Expected:* the colour is picked
   (the Colour panel and the swatch under the tools change).
7. Moving around:
   * **Space** + drag: pan. **Shift + Space** + drag: rotate. **Ctrl + Space** + drag (up/down): zoom.
   * Mouse wheel: zoom where the mouse is. Middle mouse button + drag: pan.
   * **4** / **6**: rotate 15°, **5**: straight again, **M**: mirror, **1**: 100 %, **2**: fit.
   *Expected:* the brush still paints exactly under the pen when rotated/mirrored/zoomed.
8. **Stability:** drag **Stability** on the toolbar (next to the brush size) to about 50.
   Draw shaky lines slowly, then quick ones. *Expected:* slow lines come out smooth, quick
   ones barely trail the pen (a thin blue string shows how far the line is behind), and
   every line ends exactly where you lift. Then draw a wobbly, roughly straight line and keep
   the pen still at its end for half a second: it snaps straight, and moving the pen before
   lifting moves its end. Tell me which Stability number feels right for you.
9. **Layers** (right side): click **New**, paint, then click the **eye** of a layer to hide it,
   the **lock** to lock it (painting on a locked layer shows a message instead), drag the
   **Opacity** slider, try a blend mode like **Multiply**. Double-click a layer's name to
   rename it.
10. **Undo:** **F** or **Ctrl+Z** undoes, **Ctrl+Shift+Z** redoes (50 steps). The fill tool is **Shift+F**. The **History** tab lists
    every step: click one to jump back to it.
11. **Panels:** drag a panel's tab (e.g. "Colour") to another group or onto
    "Drop here for a new group". Right-click a tab to **Float** it as its own window. Close
    panels and bring them back from the **Window** menu. The layout is kept for next time.
12. **Interface size:** **Edit > Settings... > Interface size** if the panels are too small or
    too big on the Artist 12.

## 3. Wintab (the other pen system)

1. **Edit > Settings... > Tablet: Wintab.**
   *Expected:* the status bar says "Wintab: (your tablet's name)". If a red message says
   Wintab could not be opened, Recedo keeps using Windows Ink; note the message.
2. Repeat steps 1-3 of part 2. Note which of the two felt better, or if one has no
   pressure.
3. Switch back to whichever you prefer.

## 4. Files

1. **File > Save as...**, save as `test.kra` in Documents. *Expected:* the title shows
   `test.kra` and the star (unsaved changes) goes away.
2. If you have Krita: open `test.kra` in Krita. *Expected:* the same layers, names,
   opacities and picture.
3. In Recedo, **File > Open...** a `.kra` you made in Krita (or drag it onto the Recedo
   window). *Expected:* it opens; if it has things Recedo cannot show yet (e.g. vector
   layers, masks), a window lists them in plain words.
4. **File > Export as PNG...** and **Export as JPEG...**: open the files in the Photos app.
5. **Crash test (autosave):** set **Edit > Settings... > Autosave every** to 1 min. Paint
   something, wait 1-2 minutes, then close Recedo the hard way: **Ctrl+Shift+Esc**
   (Task Manager), right-click **recedo** > **End task**. Start Recedo again.
   *Expected:* "Recover your work?" lists the painting; **Open** brings it back.
   Then set autosave back to 5 min.
6. Close Recedo with unsaved changes. *Expected:* "Save changes?" asks first.

## 5. The other tools (10 minutes)

1. Click the **grid button** at the top of the tool icons on the far left. *Expected:* a window lists 31
   tools in groups. Tick **On bar** next to **Spray** and close the window: Spray is now at
   the bottom of the tool strip. (**Up** / **Down** / **Remove** order the bar; **Back to the
   standard tools** resets it.)
2. **Smudge** (**S**): paint two colours side by side, then smudge across them, pressing
   lightly, then harder. *Expected:* the paint is dragged along; pressing harder drags more.
3. **Blur** (on the tool strip: the drop icon): go over the edge of a stroke a few times. *Expected:* the edge
   gets soft.
4. **Rectangle select** (**R**): drag a box over part of a stroke, then paint across it with
   the brush. *Expected:* paint only inside the box (the moving dashed line shows the
   selection). **Delete** clears inside it, **Ctrl+D** deselects. Holding **Shift** when you
   start a new selection adds to it, **Alt** takes away.
5. **Magic wand** (**W**): click inside a filled area; **Backspace** fills it with the colour.
6. **Text** (**T**): click on the canvas, type, click somewhere else to move it, type more,
   then **Ctrl+Enter**. *Expected:* the letters always go into the text (never change the
   tool), and Ctrl+Enter paints it in the colour.
7. **Crop** (**C**): drag over the part to keep, press **Enter**. **Ctrl+Z** brings the whole
   painting back.
8. **Move** (on the tool strip: the four arrows): drag the painting around; the arrow keys move it 1 px,
   **Shift** + arrow 10 px. **Ctrl+Z** undoes.

9. **Reference** (browser version, in the toolbar): search "hand drawing" with Pictures,
   Videos and Articles; click a picture to see it big; under **Watch a link** paste a
   YouTube link. *Expected:* everything shows in the panel on the right while you keep
   drawing; **×** closes it; **Full screen** fills the screen with it (Escape comes back).
   Tell me if a kind of search never shows anything.
10. **G** (click the canvas first): everything but the painting hides, the reference panel
   too; **G** again or **Escape** brings it all back. Typing a g in the reference search
   box must not do this. The gradient is now **Shift+G**.
11. **Lasso transform** (the box-with-handles icon, under the four arrows): paint something,
   draw round it, then drag a corner. *Expected:* it gets bigger or smaller as you drag
   (**Shift** keeps its shape), dragging inside moves it, and only the selected layer
   changes. **Enter** puts it down; **Ctrl+Z** puts it back; **Escape** before Enter
   cancels. Tell me if it feels slow on a big area.
12. **Navigator** (top of the right side): *Expected:* a small picture of the whole painting,
   your local time under it, then **Flip**, **Symmetry X** and **Y**. Zoom in (**+**), then
   click or drag on the small picture: the view goes there. **Flip** mirrors the view (the
   painting itself does not change). Turn on **X** and **Y** and paint in one corner:
   *Expected:* the stroke appears in all four corners, dashed blue lines show the middles,
   and one **Ctrl+Z** takes all four away.

## 6. What to send back

* The step numbers that did not behave as expected, and what happened instead.
* Which pen system felt better (Windows Ink or Wintab), and the status-bar text for each.
* Your Windows display scale for the Artist 12 (Settings > System > Display > Scale), and
  whether everything was readable.
* If Recedo ever crashes, it saves a report in **Documents\Recedo** (`crash-...txt`) and
  says so: send that file.

## Where Recedo keeps things

* Settings: `%APPDATA%\Recedo\recedo.json` (delete it to reset the layout and settings).
* Autosaved copies: `%LOCALAPPDATA%\Recedo\recovery` (only while a painting has unsaved
  changes; removed when you save or close normally).
* Crash reports: `Documents\Recedo`.

To remove Recedo: delete its folder and the two `Recedo` folders above.

## Not in this version yet

The brush editor, textures, the radial pop-up palette, pen button settings, pressure curve
editor (Phase 2); palettes, colour wheel styles, 16-bit painting, colour management
(Phase 3); groups, masks, filters, alpha lock, thumbnails (Phase 4); polygon lasso, feather /
grow / shrink, scaling and rotating, radial gradients (Phase 5); polygon and Bézier shapes,
assistants, radial symmetry, text that stays editable, reference images (Phase 6); shortcut editor,
workspaces, light theme (Phase 7); WebP, PDF, timelapse (Phase 8); animation
(Phase 9); installer (Phase 10).
