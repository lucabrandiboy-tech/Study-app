# DonutSMP market tracker - PowerShell (built into Windows, nothing to install)
# Reads the DonutSMP auction house through the official API (https://api.donutsmp.net)
# and every few minutes rewrites donut_market.html with:
#   1. Items worth ordering with /order, then selling with /ah sell
#   2. Items listed on the AH right now far below what they usually sell for
# Close this window to stop.

# ---- Settings (change these if you want) ----
$TxPages        = 20    # pages of recent AH sales to read (more = better prices, slower)
$ListPages      = 30    # pages of current AH listings to read
$RefreshMinutes = 2     # how often to update
$AhTaxPercent   = 0     # % the AH takes from your sale (set this if DonutSMP charges a fee)
$TargetProfit   = 25    # % profit you want when ordering (lower = more orders get filled)
$MinSales       = 5     # ignore items with fewer recent sales than this (not enough data)
$DealPercent    = 70    # a listing is a "deal" if it costs less than this % of the usual price

$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$Here     = Split-Path -Parent $MyInvocation.MyCommand.Path
$KeyFile  = Join-Path $Here 'donut_api_key.txt'
$HtmlFile = Join-Path $Here 'donut_market.html'
$Api      = 'https://api.donutsmp.net/v1'

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

function Format-Money($n) {
    if ($null -eq $n) { return '-' }
    $a = [math]::Abs($n)
    if ($a -ge 1e9) { return ('${0:0.##}B' -f ($n / 1e9)) }
    if ($a -ge 1e6) { return ('${0:0.##}M' -f ($n / 1e6)) }
    if ($a -ge 1e3) { return ('${0:0.##}K' -f ($n / 1e3)) }
    return ('${0:0.##}' -f $n)
}

function Format-Name($key) {
    return (Get-Culture).TextInfo.ToTitleCase(($key -replace '_', ' '))
}

function Esc($s) { return [System.Net.WebUtility]::HtmlEncode([string]$s) }

function Update-Market {
    Write-Host ("[{0:HH:mm:ss}] Reading recent AH sales..." -f (Get-Date))
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
    $listed = @{}; $allListings = New-Object System.Collections.ArrayList
    for ($p = 1; $p -le $ListPages; $p++) {
        $page = Get-Page "auction/list/$p"
        if ($null -eq $page -or $page.Count -eq 0) { break }
        foreach ($e in $page) {
            $k = Get-ItemKey $e.item; $u = Get-UnitPrice $e
            if (-not $k -or -not $u) { continue }
            if (-not $listed.ContainsKey($k)) { $listed[$k] = New-Object System.Collections.ArrayList }
            [void]$listed[$k].Add($u)
            $cnt = 1; if ($e.item.count) { $cnt = [int]$e.item.count }
            [void]$allListings.Add([pscustomobject]@{ Key = $k; Unit = $u; Count = $cnt; Price = [double]$e.price; Seller = $e.seller })
        }
    }

    $keep = 1 - ($AhTaxPercent / 100)
    $orders = foreach ($k in $sold.Keys) {
        $sales = $sold[$k]
        if ($sales.Count -lt $MinSales) { continue }
        $median = Get-Median $sales
        $lowest = $null; $supply = 0
        if ($listed.ContainsKey($k)) { $lowest = ($listed[$k] | Measure-Object -Minimum).Minimum; $supply = $listed[$k].Count }
        # To sell you usually have to beat the cheapest listing, so plan to sell a bit under it
        $sellAt = $median
        if ($lowest -and ($lowest * 0.99) -lt $sellAt) { $sellAt = $lowest * 0.99 }
        $orderAt = $sellAt * $keep * (1 - $TargetProfit / 100)
        $profit  = $sellAt * $keep - $orderAt
        [pscustomobject]@{
            Key = $k; Sales = $sales.Count; Median = $median; Lowest = $lowest; Supply = $supply
            SellAt = $sellAt; OrderAt = $orderAt; Profit = $profit
            Score = $profit * $sales.Count / [math]::Max(1.0, [math]::Sqrt([double]$supply))
        }
    }
    $orders = @($orders | Sort-Object Score -Descending | Select-Object -First 40)

    $deals = foreach ($l in $allListings) {
        if (-not $sold.ContainsKey($l.Key) -or $sold[$l.Key].Count -lt $MinSales) { continue }
        $median = Get-Median $sold[$l.Key]
        if ($l.Unit -ge $median * ($DealPercent / 100)) { continue }
        $resell = $median * $keep * [double]$l.Count
        [pscustomobject]@{ Key = $l.Key; Count = $l.Count; Price = $l.Price; Unit = $l.Unit; Median = $median
                           Seller = $l.Seller; Profit = $resell - $l.Price }
    }
    $deals = @($deals | Sort-Object Profit -Descending | Select-Object -First 30)

    Write-Report $orders $deals $sold.Count $allListings.Count
    Write-Host ("[{0:HH:mm:ss}] Updated: {1} order ideas, {2} AH deals. Next update in {3} min." -f (Get-Date), $orders.Count, $deals.Count, $RefreshMinutes)
}

function Write-Report($orders, $deals, $itemCount, $listingCount) {
    $rows1 = ($orders | ForEach-Object {
        "<tr><td>$(Esc (Format-Name $_.Key))</td><td class=g>$(Format-Money $_.OrderAt)</td><td>$(Format-Money $_.SellAt)</td>" +
        "<td class=g>+$(Format-Money $_.Profit)</td><td>$($_.Sales)</td><td>$($_.Supply)</td><td>$(Format-Money $_.Median)</td></tr>"
    }) -join "`n"
    $rows2 = ($deals | ForEach-Object {
        "<tr><td>$(Esc (Format-Name $_.Key)) x$($_.Count)</td><td>$(Format-Money $_.Price)</td><td>$(Format-Money $_.Unit)</td>" +
        "<td>$(Format-Money $_.Median)</td><td class=g>+$(Format-Money $_.Profit)</td><td>$(Esc $_.Seller)</td></tr>"
    }) -join "`n"
    if (-not $rows1) { $rows1 = '<tr><td colspan=7>No data yet.</td></tr>' }
    if (-not $rows2) { $rows2 = '<tr><td colspan=6>No underpriced listings right now.</td></tr>' }

    $html = @"
<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta http-equiv="refresh" content="30"><title>DonutSMP Market</title>
<style>
:root{--bg:#f4f4f5;--card:#fff;--fg:#18181b;--muted:#52525b;--line:#e4e4e7;--good:#15803d}
@media (prefers-color-scheme:dark){:root{--bg:#18181b;--card:#27272a;--fg:#fafafa;--muted:#a1a1aa;--line:#3f3f46;--good:#4ade80}}
body{margin:0;background:var(--bg);color:var(--fg);font:15px/1.45 system-ui,sans-serif;padding:16px}
.wrap{max-width:1000px;margin:0 auto}.card{background:var(--card);border-radius:12px;padding:16px 20px;margin:16px 0;overflow-x:auto}
h1{margin:8px 0 0;font-size:24px}h2{margin:0 0 4px;font-size:18px}p{color:var(--muted);margin:4px 0 12px}
table{border-collapse:collapse;width:100%;font-variant-numeric:tabular-nums}th,td{text-align:left;padding:6px 10px;border-bottom:1px solid var(--line);white-space:nowrap}
th{color:var(--muted);font-weight:600;font-size:13px}.g{color:var(--good);font-weight:600}
</style></head><body><div class="wrap">
<h1>DonutSMP Market</h1>
<p>Updated $(Get-Date -Format 'HH:mm:ss') from $itemCount items and $listingCount current listings. Page refreshes by itself. Plain (non-enchanted) items only.</p>
<div class="card"><h2>Order these, then /ah sell</h2>
<p>Put up an /order at the <b>Order at</b> price (or lower). When it fills, list it on the AH at <b>Sell at</b>. Profit is per item. Sorted by profit &times; how often it sells.</p>
<table><tr><th>Item</th><th>Order at</th><th>Sell at</th><th>Profit / item</th><th>Recent sales</th><th>On AH now</th><th>Usual price</th></tr>
$rows1
</table></div>
<div class="card"><h2>Cheap on the AH right now</h2>
<p>Listed for less than $DealPercent% of what it usually sells for. Buy it and relist. Profit is for the whole stack.</p>
<table><tr><th>Listing</th><th>Price</th><th>Per item</th><th>Usual / item</th><th>Resell profit</th><th>Seller</th></tr>
$rows2
</table></div>
<p>Prices come from recent AH sales, so they're estimates, not guarantees. Always check the AH before ordering big amounts.</p>
</div></body></html>
"@
    [System.IO.File]::WriteAllText($HtmlFile, $html, (New-Object System.Text.UTF8Encoding $false))
}

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
    Start-Sleep -Seconds ($RefreshMinutes * 60)
}
