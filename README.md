# Nova PC Suite

Application Windows 10/11 x64 de diagnostic, pilotes et maintenance. Interface française, thèmes clair/sombre, moteur Rust/Tauri et installateur NSIS. Licence MIT.

## Télécharger

[Releases GitHub](https://github.com/Rub19/nova-pc-suite/releases) — **0.1.1 preview**, thème sombre graphite.

- `Nova-PC-Suite-0.1.1-setup.exe` : véritable installateur avec désinstallation Windows.
- `Nova-PC-Suite-0.1.1.exe` : exécutable autonome, WebView2 requis.
- `SHA256SUMS.txt` : empreintes de contrôle.

Binaires non signés Authenticode : Windows peut afficher un avertissement d’éditeur inconnu. La preview doit encore être validée sur plusieurs PC et VM Windows 10 avant une version stable.

## Fonctions

- Diagnostic CPU, mémoire, système, matériel, uptime et redémarrage en attente.
- Inventaire réel des pilotes : version, fabricant, date et signature.
- Recherche des pilotes applicables via Windows Update Agent, y compris les pilotes optionnels proposés par le service configuré.
- Installation groupée de la liste confirmée : nouvelle vérification des identifiants, lecture et acceptation explicite des licences nécessaires, téléchargement, résultat par pilote et indication de redémarrage.
- Création et vérification d’un point de restauration.
- Réparation Windows par DISM puis SFC ; codes de sortie affichés.
- ReTrim du SSD système si pris en charge.
- Profils Équilibré et Performances élevées si disponibles.
- Volumes, espace libre, processus, mémoire consommée, applications au démarrage, inventaire antivirus et profils pare-feu.
- Accès aux outils de nettoyage, démarrage et sécurité Windows.
- Recommandations locales ; IA optionnelle via Ollama avec le premier modèle installé sur le PC.
- Historique local des 100 dernières opérations et export JSON ; recherche et pagination des tableaux.

Un résultat Windows Update vide ne signifie pas qu’aucune version constructeur plus récente n’existe : les catalogues NVIDIA/AMD/Intel/OEM ne sont pas agrégés. Nettoyage et modifications du démarrage s’effectuent dans les paramètres Windows. Aucun gain universel de performances n’est garanti.

## Utiliser Nova

1. Installez puis ouvrez Nova. Le diagnostic initial est en lecture seule.
2. Lancez l’inventaire et la recherche de pilotes.
3. Pour une maintenance, utilisez « Ouvrir en administrateur » puis créez un point de restauration. L’élévation ouvre une nouvelle fenêtre ; relancez-y la recherche.
4. Vérifiez la liste et les licences avant « Tout installer ».
5. Consultez les résultats et redémarrez manuellement si Windows le demande.

Un mutex Windows global détenu par le contrôleur PowerShell sérialise les maintenances entre les fenêtres Nova. Une réparation ou installation n’est pas interrompue par un délai arbitraire. Fermer Nova ne garantit pas l’arrêt d’une maintenance : attendez son résultat.

## Confidentialité

Aucune télémétrie, aucun compte ni clé cloud. L’historique local contient l’action, sa date et son succès ; les inventaires restent en mémoire pendant la session. Le thème reste dans le stockage WebView local. Windows Update communique avec le service Microsoft/entreprise configuré. L’IA reçoit votre question et un résumé des mesures uniquement sur `127.0.0.1:11434`.

## Compiler

Windows x64, Node 24, npm 11, Rust stable MSVC, outils C++ Visual Studio, SDK Windows et WebView2 requis.

```powershell
npm ci --ignore-scripts
npm test
npm run build
powershell -NoProfile -File tests/system-tests.ps1
powershell -NoProfile -File tests/backend-safety-tests.ps1
cargo test --manifest-path src-tauri/Cargo.toml --release --locked
npm run tauri:build
```

Les scripts des dépendances sont désactivés dans `.npmrc`. Les paquets natifs nécessaires sont précompilés. Icônes déjà générées ; régénération : `node scripts/icons.mjs`.

Exécutable : `src-tauri/target/release/`. Installateur : `src-tauri/target/release/bundle/nsis/`. Développement natif : `npm run tauri:dev`. `npm run dev` seul fournit un aperçu de l’interface qui refuse les diagnostics natifs.

Voir [docs/VALIDATION.md](docs/VALIDATION.md) pour la portée des tests. Les tests n’installent pas de pilotes et n’appliquent pas de maintenance au PC hôte.

Sources : [Windows Update Search](https://learn.microsoft.com/en-us/windows/win32/api/wuapi/nf-wuapi-iupdatesearcher-search), [installateur Windows Tauri](https://v2.tauri.app/distribute/windows-installer/).
