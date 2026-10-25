[CmdletBinding()]
param([ValidateSet('network-audit','network-probe','network-connections')][string]$Action='network-audit',[string]$PayloadBase64='',[string]$DataRoot)
$ErrorActionPreference='Stop'
$ProgressPreference='SilentlyContinue'
[Console]::OutputEncoding=New-Object Text.UTF8Encoding($false)
try {
    if($PayloadBase64){$p=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($PayloadBase64))|ConvertFrom-Json;if(@($p.PSObject.Properties).Count){throw 'InvalidPayload'}}
    $data=@{}; $issues=@()
    switch($Action){
        'network-audit' {
            $adapters=@(Get-NetAdapter -ErrorAction Stop|Select-Object Name,InterfaceIndex,Status,LinkSpeed,InterfaceDescription)
            $stats=$null
            try{$stats=@(Get-NetAdapterStatistics -ErrorAction Stop|Select-Object Name,ReceivedBytes,SentBytes,ReceivedPacketErrors,OutboundPacketErrors,ReceivedDiscardedPackets,OutboundDiscardedPackets)}catch{$issues+='Compteurs réseau indisponibles : Windows peut demander les droits administrateur.'}
            $configs=@(Get-NetIPConfiguration -ErrorAction Stop|ForEach-Object{@{name=$_.InterfaceAlias;index=$_.InterfaceIndex;ipv4=@($_.IPv4Address.IPAddress);ipv6=@($_.IPv6Address.IPAddress);gateway=@($_.IPv4DefaultGateway.NextHop);dns=@($_.DNSServer.ServerAddresses)}})
            $proxy=Get-ItemProperty -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings' -ErrorAction SilentlyContinue
            if(@($stats|Where-Object{[long]$_.ReceivedPacketErrors -gt 0 -or [long]$_.OutboundPacketErrors -gt 0}).Count){$issues+='Des erreurs cumulées figurent dans les compteurs réseau. Comparez deux relevés pour vérifier si elles augmentent.'}
            $vpn=@(Get-NetAdapter -IncludeHidden|Where-Object{$_.Status -eq 'Up' -and ($_.InterfaceDescription+' '+$_.Name) -match '(?i)vpn|wireguard|openvpn|tunnel|tap-|tailscale|zerotier'}|Select-Object Name,InterfaceDescription)
            $data=@{adapters=$adapters;statistics=$stats;configuration=$configs;proxyEnabled=($proxy.ProxyEnable -eq 1);automaticProxyConfigured=([bool]$proxy.AutoConfigURL);vpnHints=$vpn;findings=$issues;leakStatus='Non déterminé : un inventaire local ne prouve ni fuite DNS, ni exfiltration, ni compromission.';privacy='Adresses locales et distantes : conserver le rapport local ou le relire avant partage.'}
        }
        'network-connections' {
            $cache=@{}; foreach($proc in Get-Process){$cache[[uint32]$proc.Id]=$proc.ProcessName}
            $connections=@(Get-NetTCPConnection -ErrorAction Stop|Select-Object -First 1000|ForEach-Object{@{process=$cache[[uint32]$_.OwningProcess];pid=$_.OwningProcess;localAddress=$_.LocalAddress;localPort=$_.LocalPort;remoteAddress=$_.RemoteAddress;remotePort=$_.RemotePort;state=[string]$_.State}})
            $listeners=@($connections|Where-Object{$_.state -eq 'Listen' -and $_.localAddress -in @('0.0.0.0','::')})
            $data=@{items=$connections;listenersAllInterfaces=$listeners;interpretation='Un port en écoute ou une adresse inconnue n’est pas une preuve de fuite. Les débits par application ne sont pas mesurés par cet inventaire.';limit=1000}
        }
        'network-probe' {
            # Fixed destinations only; no scanning or user-controlled target.
            $route=Get-NetRoute -DestinationPrefix '0.0.0.0/0'|Where-Object{$_.NextHop -ne '0.0.0.0'}|Sort-Object RouteMetric|Select-Object -First 1
            $targets=@('1.1.1.1')
            $parsed=$null
            if($route -and [Net.IPAddress]::TryParse([string]$route.NextHop,[ref]$parsed)){$targets=@([string]$route.NextHop)+$targets}
            $tests=@()
            foreach($target in $targets){
                $ping=New-Object Net.NetworkInformation.Ping; $rtts=@();$lost=0
                try{for($i=0;$i -lt 5;$i++){try{$r=$ping.Send($target,1000);if($r.Status -eq 'Success'){$rtts += [double]$r.RoundtripTime}else{$lost++}}catch{$lost++}}}finally{$ping.Dispose()}
                $mean=if($rtts.Count){($rtts|Measure-Object -Average).Average}else{$null}
                $jitter=$null;if($rtts.Count -gt 1){$d=@();for($i=1;$i -lt $rtts.Count;$i++){$d += [math]::Abs($rtts[$i]-$rtts[$i-1])};$jitter=($d|Measure-Object -Average).Average}
                $tests+=@{target=$target;sent=5;received=$rtts.Count;lossPercent=$lost*20;averageMs=$mean;jitterMs=$jitter}
            }
            $watch=[Diagnostics.Stopwatch]::StartNew();$dnsOk=$false
            try{$resolved=@(Resolve-DnsName -Name 'example.com' -Type A -DnsOnly -QuickTimeout -ErrorAction Stop);$dnsOk=$resolved.Count -gt 0}catch{}
            $watch.Stop()
            $data=@{probes=$tests;dnsResolved=$dnsOk;dnsElapsedMs=$watch.ElapsedMilliseconds;dnsName='example.com';limitations='5 paquets par cible : estimation courte. Un pare-feu peut filtrer ICMP. Ceci ne mesure pas le débit internet et ne démontre pas une fuite DNS.'}
        }
    }
    @{ok=$true;data=$data}|ConvertTo-Json -Depth 12 -Compress
}catch{@{ok=$false;data=$null;error='NetworkDiagnosticUnavailable'}|ConvertTo-Json -Compress}
