param([string]$Leader='http://127.0.0.1:7070', [string[]]$Followers=@('http://127.0.0.1:7071','http://127.0.0.1:7072'), [string]$Token='letmein')
function Add-Peer($from,$to){ irm -Method Post "$from/peer/add?token=$Token" -ContentType 'application/json' -Body (@{url=$to}|ConvertTo-Json) | Out-Null }
# mesh
foreach($f in $Followers){ Add-Peer $Leader $f; Add-Peer $f $Leader }
if($Followers.Count -gt 1){ Add-Peer $Followers[0] $Followers[1]; Add-Peer $Followers[1] $Followers[0] }
# catch-up
$srcH=[int](irm "$Leader/height")
foreach($dst in $Followers){
  $dstH=[int](irm "$dst/height")
  for($i=$dstH+1;$i -le $srcH;$i++){
    $b=irm "$Leader/block/$i"
    $resp=irm -Method Post "$dst/gossip/block" -ContentType 'application/json' -Body (@{block=$b}|ConvertTo-Json -Depth 100)
    if($resp.status -ne 'accepted'){ Write-Host "[REJECT] $dst block $i -> $($resp.error)" -f Red; break }
  }
}
