# Nova PC Suite 0.2.10 preview

Installeur : `Nova-PC-Suite-0.2.10-setup.exe`. Exécutable autonome : `Nova-PC-Suite-0.2.10.exe` (WebView2 requis).

## Mémoire plus simple à utiliser

- Le tri des applications utilise un menu sombre Nova au lieu du menu natif Windows, avec une option cochée et des contrôles accessibles au clavier.
- Après une mesure, la surveillance peut être activée immédiatement. Sans application autorisée, elle ne modifie rien et affiche clairement le mode observation.
- Une application peut être cochée ou retirée pendant la surveillance. Nova ne réduit jamais le working set d’un processus qui n’a pas été explicitement autorisé.

## Limites de la preview

- Une réduction de working set ne résout pas une fuite mémoire et peut entraîner le rechargement de données par l’application concernée.
- Les binaires ne sont pas encore signés Authenticode. Windows peut donc afficher un avertissement d’éditeur inconnu.
