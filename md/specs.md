Pebble Index 01 → jtx Board (Rust)
Petit service Rust open source (licence MIT OR Apache-2.0) qui reçoit le webhook de l'appli Pebble (Index 01), convertit la transcription en entrée iCalendar (VJOURNAL ou VTODO) et l'écrit dans une collection CalDAV (Nextcloud), synchronisée vers jtx Board par DAVx⁵.

Liens de référence
• Guide de démarrage Index 01 : https://help.repebble.com/en/articles/15434751-index-01-getting-started-guide
• Fonctions avancées (MCP et webhooks) : https://help.repebble.com/en/articles/15724406-index-advanced-features-mcp-webhook
• Doc SDK Index 01 (webhooks v1, signature, exemples) : https://github.com/coredevices/sdk-docs/pull/26
• Code de l'appli mobile (source de vérité du format) : https://github.com/coredevices/mobileapp
• Page produit : https://repebble.com/index
• Exemples communautaires de récepteurs :
    ◦ https://github.com/maimon495/pebble-index-webhook (Cloudflare Worker)
    ◦ https://github.com/lab909/pebble-index-01-webhook-hub (PHP)
    ◦ https://github.com/rbartoli/pebble-index
• CalDAV : RFC 4791 — iCalendar : RFC 5545
• jtx Board : https://jtx.techbee.at

Architecture
1. Index 01 : appui sur le bouton, enregistrement audio.
2. Appli Pebble : transcription (locale ou cloud selon réglage), puis POST du webhook.
3. Service Rust (Kubernetes ou simple conteneur) : vérifie la signature, classe le contenu, génère l'ICS.
4. Nextcloud : écriture CalDAV dans la collection du domaine.
5. DAVx⁵ : synchronisation vers jtx Board.

Spécification du webhook entrant (protocole v1, doc SDK confirmée)
Source : sdk-docs source/index-01/webhooks.md (PR #26, fusionnée).
• Réglage appli : un webhook par geste, « What to send » = Transcription, « Sign requests » activé, secret collé par l'utilisateur (généré par make key, ≥ 32 octets hex).
• Méthode : POST multipart/form-data.
• En-têtes : X-Index-Webhook-Version (1), X-Index-Trigger (single-click-hold | double-click-hold | test-event), X-Index-Test (true sur l'événement de test), X-Index-Signature (hex minuscule), X-Index-Timestamp (secondes Unix), X-Index-Delivery (ID stable par enregistrement).
• Champs : transcription (texte), recordedAt (millisecondes Unix), client (ring), test (true sur l'événement de test), audio (ignoré s'il est présent).
• Signature : HMAC-SHA256(secret UTF-8, "v1\n" + timestamp + "\n" + delivery + "\n" + trigger + "\n" + (1|0 test) + "\n" || corps brut). Vérifier sur les octets bruts avant tout parsing multipart, comparaison en temps constant.
• Requête non signée ou version ≠ 1 : refusée.
• Fenêtre d'horodatage : 5 minutes.
• Anti-rejeu : IDs de livraison gardés en mémoire jusqu'à max(timestamp, réception) + 5 min ; doublon → 409 sans écriture. Limite : perdu au redémarrage (1 réplique).
• Événement de test : signature vérifiée, 200 sans rien écrire.
• Réponses : 200 succès, 400 corps invalide, 401 signature ou horodatage invalide, 409 doublon, 502 si CalDAV échoue.
• L'appli ne réessaie pas ; l'échec est visible dans « Recent runs » (statut + 200 premiers caractères de la réponse). Délai max 2 min.
• Vecteur de test : l'exemple « Worked Example » de la doc sert de test unitaire de la vérification.

Modèle
• Domaine = une collection CalDAV (ex. cnv, esa, perso, maison). Chaque collection contient journaux, notes et tâches et doit accepter VJOURNAL et VTODO.
• Type = composant iCalendar :
    ◦ note → VJOURNAL sans DTSTART (onglet Notes de jtx)
    ◦ journal → VJOURNAL avec DTSTART (onglet Journals)
    ◦ tâche → VTODO, STATUS:NEEDS-ACTION (onglet Tasks)

Classification par mots-clés
Le texte commence par un mot-clé de type, puis un mot-clé de domaine, tous deux optionnels :
  « Journal ESA, réunion avec… » · « À faire maison, changer le filtre » · « CNV, … »
• Type : « note » → note ; « journal » → journal ; « todo », « tâche », « à faire » → tâche ; aucun → note.
• Domaine : « cnv », « nvc » → cnv ; « travail », « esa » → esa ; « perso » → perso ; « maison » → maison ; aucun → perso.
• Comparaison insensible à la casse, aux accents et à la ponctuation qui suit (« À faire, » = « a faire »).
• Les mots-clés sont retirés du texte.
• Listes de mots-clés et correspondance domaine → collection dans le fichier de configuration.

Journal : ajout au journal du jour
• Un seul VJOURNAL par jour et par domaine. UID déterministe : pebble-journal-<domaine>-<AAAA-MM-JJ>, fichier <uid>.ics.
• GET de l'entrée :
    ◦ 404 → PUT avec If-None-Match: *, SUMMARY « Journal <AAAA-MM-JJ> », DTSTART;VALUE=DATE.
    ◦ 200 → ajout d'une ligne « HH:MM texte » à DESCRIPTION, PUT avec If-Match: <etag>.
    ◦ 412 (modifié entre-temps) → relire et réessayer (3 fois max).
• Le jour est calculé dans le fuseau TZ à partir de l'horodatage d'enregistrement.
• Configuration journal.mode : seule la valeur append est prise en charge pour l'instant (new = une entrée par dictée, plus tard).

Génération iCalendar
Note :
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//pebble-jtx//FR
BEGIN:VJOURNAL
UID:<uuid>
DTSTAMP:<maintenant UTC>
SUMMARY:<résumé>
DESCRIPTION:<texte>
CATEGORIES:Pebble
END:VJOURNAL
END:VCALENDAR
Tâche : VTODO avec STATUS:NEEDS-ACTION. Journal : voir plus haut.
Le résumé (SUMMARY) prend la première phrase ou les 60 premiers caractères, la description le texte complet.
Échappement RFC 5545 (virgules, points-virgules, retours ligne) et pliage des lignes à 75 octets.

Écriture CalDAV
• URL : <caldav.base_url><collection>/<uid>.ics
  (Nextcloud : https://<nextcloud>/remote.php/dav/calendars/<user>/<collection>/)
• PUT avec Content-Type: text/calendar; charset=utf-8 et If-None-Match: * (ou If-Match pour l'ajout au journal).
• Authentification : Basic avec un mot de passe d'application Nextcloud dédié.
• Succès : 201 Created ou 204 No Content.

Stack Rust
• axum + tokio : serveur HTTP et multipart
• hmac + sha2 + subtle : vérification de signature en temps constant
• reqwest (rustls) : client CalDAV
• uuid, chrono, chrono-tz : identifiants et dates
• serde + YAML : fichier de configuration
• tracing : logs
• ICS écrit à la main (format court), pas de crate icalendar.

Configuration
Fichier YAML (ConfigMap en Kubernetes), chemin dans CONFIG_PATH (défaut /config/config.yaml) :
  caldav:
    base_url: https://cloud.example.com/remote.php/dav/calendars/alice/
    user: alice
  timezone: America/Montreal
  journal:
    mode: append
  types:
    note: [note]
    journal: [journal]
    todo: [todo, tâche, à faire]
  default_domain: perso
  domains:
    cnv:    { collection: cnv,    keywords: [cnv, nvc] }
    esa:    { collection: esa,    keywords: [esa, travail] }
    perso:  { collection: perso,  keywords: [perso] }
    maison: { collection: maison, keywords: [maison] }
Secrets en variables d'environnement (Secret Kubernetes ou --env-file) :
• PEBBLE_WEBHOOK_SECRET
• CALDAV_APP_PASSWORD

Gestion des secrets (Makefile)
• make key : affiche un secret aléatoire (openssl rand), pour tout le monde.
• make secret : demande les valeurs (saisie masquée, vide = génération du secret webhook, affiché une fois pour l'appli Pebble) et écrit un Secret Kubernetes en clair sur la sortie standard.
• make sealed-secret : même chose, envoyé directement dans kubeseal → deploy/sealed-secret.yaml (contrôleur et namespace en variables).
• Sans Kubernetes : .env.example + docker run --env-file.

Déploiement
• Forge : GitHub. CI GitHub Actions (build, tests), image publiée sur ghcr.io.
• Image multi-stage (rust musl puis scratch ou distroless).
• deploy/ : base Kustomize générique (Deployment, Service, ConfigMap d'exemple, Ingress standard) ; chacun ajoute son overlay (IngressRoute Traefik, SealedSecret, FluxCD) dans son propre dépôt GitOps.
• Endpoint /healthz pour les probes.
• Ne pas exposer d'autre route que le webhook.

Étapes de test
1. Configurer le webhook dans l'appli Pebble et envoyer l'événement de test (200 attendu dans « Recent runs »).
2. Dicter une note, une tâche et deux journaux le même jour et vérifier les .ics sur Nextcloud (un seul journal, deux lignes).
3. Synchroniser DAVx⁵ et vérifier dans jtx Board.

Évolutions possibles
• journal.mode: new (une entrée par dictée).
• Serveur MCP CalDAV pour laisser l'agent choisir le type.
• Archivage de l'audio dans Nextcloud et lien en pièce jointe (ATTACH).
• Dates d'échéance extraites du texte pour les tâches.
