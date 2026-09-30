# DonutSMP market tracker - PowerShell (built into Windows, nothing to install)
# Reads the DonutSMP auction house through the official API (https://api.donutsmp.net)
# and keeps donut_market.html updated like a stock app, with two tabs:
#   Rising  - items whose AH sale price is going up, with price charts
#   Orders  - what items are selling for on the AH right now, and what to /order them at
# Close this window to stop.

# ---- Settings (change these if you want) ----
$TxPages        = 10    # pages of recent AH sales to read each update
$ListPages      = 15    # pages of current AH listings to read each update
$RefreshSeconds = 60    # how often to update (API allows 250 requests/min)
$AhTaxPercent   = 0     # % the AH takes from your sale (set this if DonutSMP charges a fee)
$TargetProfit   = 25    # % profit you want when ordering (lower = more orders get filled)
$MinSales       = 4     # ignore items with fewer recent sales than this
$HistoryHours   = 24    # how much price history to keep for the charts

$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$Here        = Split-Path -Parent $MyInvocation.MyCommand.Path
$KeyFile     = Join-Path $Here 'donut_api_key.txt'
$HtmlFile    = Join-Path $Here 'donut_market.html'
$HistoryFile = Join-Path $Here 'donut_history.json'
$Api         = 'https://api.donutsmp.net/v1'

# ---- API key (made in-game with /api) ----
if (Test-Path $KeyFile) { $ApiKey = (Get-Content $KeyFile -Raw).Trim() }
if (-not $ApiKey) {
    Write-Host "You need a DonutSMP API key. In Minecraft on DonutSMP, type /api and copy the key."
    $ApiKey = (Read-Host "Paste your API key here").Trim()
    Set-Content -Path $KeyFile -Value $ApiKey
}
$Headers = @{ Authorization = "Bearer $ApiKey"; Accept = 'application/json' }

function Get-Page($path) {
    try {
        $r = Invoke-RestMethod -Uri "$Api/$path" -Headers $Headers -Method Get -TimeoutSec 20
        if ($r.result) { return @($r.result) }
        return @()
    } catch {
        $code = $null
        if ($_.Exception.Response) { $code = [int]$_.Exception.Response.StatusCode }
        if ($code -eq 401 -or $code -eq 403) {
            Remove-Item $KeyFile -ErrorAction SilentlyContinue
            throw "The API key was rejected. Run the tracker again and paste a new key from /api."
        }
        if ($code -eq 429) { Write-Host "  Rate limited, waiting 30s..."; Start-Sleep 30; return $null }
        Write-Host "  Couldn't read $path : $($_.Exception.Message)"
        return $null
    }
}

# Plain items only: skip anything enchanted, since those prices vary too much to compare
function Get-ItemKey($item) {
    if (-not $item -or -not $item.id) { return $null }
    $ench = $item.enchants
    if ($ench -and $ench.enchantments -and $ench.enchantments.levels) {
        if (@($ench.enchantments.levels.PSObject.Properties).Count -gt 0) { return $null }
    }
    return ($item.id -replace '^minecraft:', '')
}

function Get-UnitPrice($entry) {
    $count = 1
    if ($entry.item.count) { $count = [double]$entry.item.count }
    if (-not $entry.price -or $count -le 0) { return $null }
    return [double]$entry.price / $count
}

function Get-Median($values) {
    $s = @($values | Sort-Object)
    if ($s.Count -eq 0) { return $null }
    $m = [math]::Floor($s.Count / 2)
    if ($s.Count % 2) { return $s[$m] }
    return ($s[$m - 1] + $s[$m]) / 2
}

# ---- Price history (saved between runs so the charts build up) ----
$History = @{}
if (Test-Path $HistoryFile) {
    try {
        $h = Get-Content $HistoryFile -Raw | ConvertFrom-Json
        foreach ($p in $h.PSObject.Properties) {
            $list = New-Object System.Collections.ArrayList
            foreach ($pt in $p.Value) { [void]$list.Add(@([double]$pt[0], [double]$pt[1])) }
            $History[$p.Name] = $list
        }
    } catch { Write-Host "Couldn't read old price history, starting fresh." }
}

function Update-Market {
    $now = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    Write-Host ("[{0:HH:mm:ss}] Reading recent AH sales..." -f (Get-Date))
    # Sales come newest first, so remember the order to split "recent" vs "earlier"
    $sold = @{}
    for ($p = 1; $p -le $TxPages; $p++) {
        $page = Get-Page "auction/transactions/$p"
        if ($null -eq $page -or $page.Count -eq 0) { break }
        foreach ($e in $page) {
            $k = Get-ItemKey $e.item; $u = Get-UnitPrice $e
            if (-not $k -or -not $u) { continue }
            if (-not $sold.ContainsKey($k)) { $sold[$k] = New-Object System.Collections.ArrayList }
            [void]$sold[$k].Add($u)
        }
    }

    Write-Host ("[{0:HH:mm:ss}] Reading current AH listings..." -f (Get-Date))
    $listed = @{}
    for ($p = 1; $p -le $ListPages; $p++) {
        $page = Get-Page "auction/list/$p"
        if ($null -eq $page -or $page.Count -eq 0) { break }
        foreach ($e in $page) {
            $k = Get-ItemKey $e.item; $u = Get-UnitPrice $e
            if (-not $k -or -not $u) { continue }
            if (-not $listed.ContainsKey($k)) { $listed[$k] = New-Object System.Collections.ArrayList }
            [void]$listed[$k].Add($u)
        }
    }

    $keep   = 1 - ($AhTaxPercent / 100)
    $cutoff = $now - $HistoryHours * 3600 * 1000
    $items  = New-Object System.Collections.ArrayList
    foreach ($k in $sold.Keys) {
        $sales = $sold[$k]
        if ($sales.Count -lt $MinSales) { continue }
        $half   = [math]::Floor($sales.Count / 2)
        $recent = Get-Median ($sales[0..($half - 1)])            # newer half of sales
        $older  = Get-Median ($sales[$half..($sales.Count - 1)]) # older half of sales
        $price  = Get-Median $sales

        if (-not $History.ContainsKey($k)) { $History[$k] = New-Object System.Collections.ArrayList }
        [void]$History[$k].Add(@([double]$now, [double]$recent))
        while ($History[$k].Count -gt 0 -and $History[$k][0][0] -lt $cutoff) { $History[$k].RemoveAt(0) }

        # Change: compare to the oldest saved price if we have history, else newer vs older sales
        $base = $older
        if ($History[$k].Count -ge 3) { $base = $History[$k][0][1] }
        $change = 0
        if ($base -gt 0) { $change = ($recent - $base) / $base * 100 }

        $lowest = $null; $supply = 0
        if ($listed.ContainsKey($k)) { $lowest = ($listed[$k] | Measure-Object -Minimum).Minimum; $supply = $listed[$k].Count }
        $sellAt = $recent
        if ($lowest -and ($lowest * 0.99) -lt $sellAt) { $sellAt = $lowest * 0.99 }
        $orderAt = $sellAt * $keep * (1 - $TargetProfit / 100)

        $pts = @($History[$k] | ForEach-Object { ,@([long]$_[0], [math]::Round($_[1], 2)) })
        [void]$items.Add([ordered]@{
            id = $k; price = [math]::Round($recent, 2); change = [math]::Round($change, 1)
            sales = $sales.Count; lowest = $lowest; supply = $supply
            sellAt = [math]::Round($sellAt, 2); orderAt = [math]::Round($orderAt, 2)
            profit = [math]::Round($sellAt * $keep - $orderAt, 2); history = $pts
        })
    }
    # Drop history for items that stopped trading
    foreach ($k in @($History.Keys)) {
        while ($History[$k].Count -gt 0 -and $History[$k][0][0] -lt $cutoff) { $History[$k].RemoveAt(0) }
        if ($History[$k].Count -eq 0) { $History.Remove($k) }
    }
    ($History | ConvertTo-Json -Depth 5 -Compress) | Set-Content -Path $HistoryFile -Encoding UTF8

    Write-Report $items $now
    Write-Host ("[{0:HH:mm:ss}] Updated {1} items. Next update in {2}s." -f (Get-Date), $items.Count, $RefreshSeconds)
}

function Write-Report($items, $now) {
    $data = [ordered]@{ updated = $now; refresh = $RefreshSeconds; items = @($items) }
    $json = ($data | ConvertTo-Json -Depth 6 -Compress) -replace '</', '<\/'
    $html = $Template.Replace('__DATA__', $json)
    [System.IO.File]::WriteAllText($HtmlFile, $html, (New-Object System.Text.UTF8Encoding $false))
}

$Template = @'
<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>DonutSMP Market</title>
<style>
:root{--bg:#0b0e11;--card:#161a1e;--fg:#eaecef;--muted:#848e9c;--line:#2b3139;--up:#0ecb81;--down:#f6465d;--acc:#fcd535}
@media (prefers-color-scheme:light){:root{--bg:#f5f5f5;--card:#fff;--fg:#1e2329;--muted:#707a8a;--line:#eaecef;--up:#03a66d;--down:#cf304a;--acc:#c99400}}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--fg);font:14px/1.4 system-ui,sans-serif}
header{display:flex;align-items:center;gap:16px;flex-wrap:wrap;padding:14px 16px;border-bottom:1px solid var(--line)}
h1{font-size:20px;margin:0}h1 span{color:var(--acc)}
.live{display:flex;align-items:center;gap:6px;color:var(--muted);font-size:13px}
.dot{width:8px;height:8px;border-radius:50%;background:var(--up);animation:p 1.5s infinite}@keyframes p{50%{opacity:.3}}
.tabs{display:flex;gap:4px;padding:0 16px;border-bottom:1px solid var(--line)}
.tab{background:none;border:0;color:var(--muted);font:inherit;font-weight:600;padding:12px 14px;cursor:pointer;border-bottom:2px solid transparent}
.tab.on{color:var(--fg);border-color:var(--acc)}
.bar{display:flex;gap:10px;padding:12px 16px;flex-wrap:wrap;align-items:center}
input{background:var(--card);border:1px solid var(--line);color:var(--fg);border-radius:6px;padding:8px 10px;font:inherit;min-width:200px}
.note{color:var(--muted);font-size:13px}
.wrap{padding:0 16px 24px;overflow-x:auto}
table{width:100%;border-collapse:collapse;font-variant-numeric:tabular-nums}
th{color:var(--muted);font-weight:500;font-size:12px;text-align:right;padding:8px 10px;cursor:pointer;white-space:nowrap}
td{padding:10px;text-align:right;border-top:1px solid var(--line);white-space:nowrap}
th:first-child,td:first-child{text-align:left}tr:hover td{background:var(--card)}
.name{font-weight:600}.up{color:var(--up)}.down{color:var(--down)}.muted{color:var(--muted)}
.pill{display:inline-block;min-width:70px;text-align:center;padding:3px 8px;border-radius:4px;font-weight:600;color:#fff}
.pill.up{background:var(--up)}.pill.down{background:var(--down)}.pill.flat{background:var(--line);color:var(--fg)}
svg{display:block}
</style></head><body>
<header><h1>Donut<span>SMP</span> Market</h1>
<div class="live"><span class="dot"></span><span id="upd"></span></div></header>
<div class="tabs"><button class="tab" data-t="rising">Rising prices</button><button class="tab" data-t="orders">Orders &rarr; AH sell</button></div>
<div class="bar"><input id="q" placeholder="Search items..." autocomplete="off"><span class="note" id="note"></span></div>
<div class="wrap"><table id="tbl"></table></div>
<script>
const D = __DATA__;
const $ = s => document.querySelector(s);
const money = n => n == null ? "-" : "$" + (Math.abs(n) >= 1e9 ? (n/1e9).toFixed(2)+"B" : Math.abs(n) >= 1e6 ? (n/1e6).toFixed(2)+"M" : Math.abs(n) >= 1e3 ? (n/1e3).toFixed(2)+"K" : (+n).toFixed(2));
const nice = id => id.replace(/_/g, " ").replace(/\b\w/g, c => c.toUpperCase());
const esc = s => String(s).replace(/[&<>"]/g, c => ({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;"}[c]));
function spark(h, up){
  if (!h || h.length < 2) return '<span class="muted">building…</span>';
  const w=110, ht=32, ys=h.map(p=>p[1]), mn=Math.min(...ys), mx=Math.max(...ys), r=(mx-mn)||1;
  const pts=h.map((p,i)=>(i/(h.length-1)*w).toFixed(1)+","+(ht-2-(p[1]-mn)/r*(ht-4)).toFixed(1)).join(" ");
  return `<svg width="${w}" height="${ht}"><polyline fill="none" stroke="var(--${up?"up":"down"})" stroke-width="1.8" points="${pts}"/></svg>`;
}
const pill = c => `<span class="pill ${c>0.05?"up":c<-0.05?"down":"flat"}">${c>0?"+":""}${c.toFixed(1)}%</span>`;
const TABS = {
  rising: { note: "Items whose AH sale price is going up. Change is vs. the oldest saved price (or earlier sales on first run).",
    sort: ["change", -1],
    cols: [["Item","id"],["Price","price"],["Change","change"],["Chart",null],["Sales","sales"],["On AH","supply"],["Cheapest listed","lowest"]],
    row: i => `<td class="name">${esc(nice(i.id))}</td><td>${money(i.price)}</td><td>${pill(i.change)}</td><td>${spark(i.history,i.change>=0)}</td><td>${i.sales}</td><td>${i.supply}</td><td>${money(i.lowest)}</td>` },
  orders: { note: "What items are selling for on the AH right now. Put an /order in at \"Order at\", then /ah sell at \"Sell at\".",
    sort: ["profit", -1],
    cols: [["Item","id"],["Selling for (AH)","price"],["Cheapest listed","lowest"],["Order at","orderAt"],["Sell at","sellAt"],["Profit / item","profit"],["Change","change"],["Chart",null],["Sales","sales"]],
    row: i => `<td class="name">${esc(nice(i.id))}</td><td>${money(i.price)}</td><td>${money(i.lowest)}</td><td class="up">${money(i.orderAt)}</td><td>${money(i.sellAt)}</td><td class="up">+${money(i.profit)}</td><td>${pill(i.change)}</td><td>${spark(i.history,i.change>=0)}</td><td>${i.sales}</td>` }
};
let tab = (location.hash.slice(1) in TABS) ? location.hash.slice(1) : "rising", sortBy = {};
try { $("#q").value = sessionStorage.getItem("q") || ""; } catch(e) {}
function render(){
  const T = TABS[tab], [key, dir] = sortBy[tab] || T.sort, q = $("#q").value.toLowerCase();
  document.querySelectorAll(".tab").forEach(b => b.classList.toggle("on", b.dataset.t === tab));
  $("#note").textContent = T.note;
  let rows = D.items.filter(i => nice(i.id).toLowerCase().includes(q));
  if (tab === "rising" && !q) rows = rows.filter(i => i.change > 0);
  rows.sort((a,b) => (typeof a[key] === "string" ? a[key].localeCompare(b[key]) : ((a[key]??-1e18) - (b[key]??-1e18))) * dir);
  $("#tbl").innerHTML = "<tr>" + T.cols.map(c => `<th data-k="${c[1]||""}">${c[0]}${c[1]===key?(dir>0?" ▲":" ▼"):""}</th>`).join("") + "</tr>" +
    (rows.length ? rows.map(i => "<tr>" + T.row(i) + "</tr>").join("") : `<tr><td colspan="${T.cols.length}" class="muted">Nothing yet — waiting for data.</td></tr>`);
}
document.querySelectorAll(".tab").forEach(b => b.onclick = () => { tab = b.dataset.t; location.hash = tab; render(); });
$("#tbl").onclick = e => { const k = e.target.closest("th")?.dataset.k; if (!k) return;
  const [ck, cd] = sortBy[tab] || TABS[tab].sort; sortBy[tab] = [k, ck === k ? -cd : -1]; render(); };
$("#q").oninput = () => { try { sessionStorage.setItem("q", $("#q").value); } catch(e) {} render(); };
const upd = () => { const s = Math.round((Date.now() - D.updated) / 1000); $("#upd").textContent = `LIVE · updated ${s}s ago`; };
upd(); setInterval(upd, 1000); render();
setTimeout(() => location.reload(), Math.max(15, D.refresh / 2) * 1000);
</script></body></html>
'@

Write-Host "DonutSMP market tracker running. Close this window to stop."
$opened = $false
while ($true) {
    try {
        Update-Market
        if (-not $opened) { Start-Process $HtmlFile; $opened = $true }
    } catch {
        Write-Host "Error: $($_.Exception.Message)"
        if ($_.Exception.Message -like '*API key was rejected*') { break }
    }
    Start-Sleep -Seconds $RefreshSeconds
}
