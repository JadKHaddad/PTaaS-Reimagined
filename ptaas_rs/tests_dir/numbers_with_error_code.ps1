$i = 1
while ($i -le 1) {
    Write-Host $i
    $i++
    Start-Sleep -Seconds 1
}

Write-Error "Error message"

exit 1