# Plan

Spec : md/specs.md

1. Format réel du webhook : fait (voir spec, protocole v1).
2. Squelette Rust : Cargo.toml, main.rs (axum, /hook, /healthz), config.rs (YAML + secrets env), Makefile (build, test, key, secret, sealed-secret).
3. Classification : normalisation (casse, accents, ponctuation), extraction type + domaine, défauts note/perso. Tests unitaires.
4. ICS : échappement, pliage 75 octets, note / tâche / journal, ajout d'une ligne au journal. Tests unitaires.
5. Webhook : vérification HMAC en temps constant, fenêtre d'horodatage, anti-rejeu, événement de test. Tests avec une requête signée.
6. CalDAV : PUT If-None-Match, GET + PUT If-Match pour le journal, réessai sur 412. Fait ; test serveur simulé non écrit (ouvrirait un port local), validation sur Nextcloud.
7. Conteneur : Dockerfile multi-stage musl, .env.example.
8. deploy/ : base Kustomize générique + exemple de ConfigMap.
9. CI GitHub Actions : build, tests, image publiée sur ghcr.io. LICENSE-MIT + LICENSE-APACHE.
10. Validation réelle : événement de test Pebble, puis dictées sur Nextcloud + jtx Board.
