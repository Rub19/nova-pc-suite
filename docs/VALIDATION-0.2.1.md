# Validation 0.2.1 preview

## Vérifications locales

- `npm test` : 15 tests passent. Les nouveaux tests utilisent un DOM simulé et un bridge Tauri simulé : annulation sans effet, confirmation ciblée, processus protégé, refus de fermeture et confirmation du démarrage.
- `cargo test --manifest-path src-tauri/Cargo.toml --release --locked` : 14 tests passent, dont la liste blanche qui impose une confirmation à `process-close`.
- Tests PowerShell `health-tests.ps1` et `apps-tests.ps1` réussis. PID invalide refusé avant toute fermeture ; registre et WinGet simulés, aucune modification du démarrage utilisateur.
- Compilation Tauri/NSIS réussie, version des ressources de l’exécutable vérifiée à 0.2.1 et SHA256 fournis dans la release.

## Fonctionnement et limites

La fermeture utilise [`Process.CloseMainWindow`](https://learn.microsoft.com/en-us/dotnet/api/system.diagnostics.process.closemainwindow) après contrôle de l’éligibilité, du propriétaire et de l’heure de création du processus. Le handle reste détenu pendant la demande. Nova attend au maximum 1,5 seconde : si l’application n’a pas quitté, le résultat précise qu’une confirmation ou un enregistrement peut être nécessaire. Une demande envoyée n’est pas présentée comme une fermeture réussie. Aucun recours à Kill/TerminateProcess.

Pas de prise de contrôle du bureau, pas de lancement de l’interface pendant ce développement, pas de fermeture de programme utilisateur. Le chemin de fermeture d’une application réelle et la présentation visuelle de ces nouveaux contrôles restent à valider sur des postes dédiés. Les limites Windows 10/11, de signature et de distribution décrites dans VALIDATION-0.2.md restent applicables.

L’installation/désinstallation 0.2.0 a réussi sur le runner Windows GitHub. Le workflow 0.2.1 exécute les mêmes contrôles et un test d’installation/désinstallation sur runner, sans exécuter de maintenance sur le PC de l’utilisateur.
